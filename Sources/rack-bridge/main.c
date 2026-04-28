/*
 * rack-bridge — unix socket <-> loopback TCP bridge
 *
 * Usage:
 *   rack-bridge --socket <path> --port <n> -- <command> [args...]
 *
 * What it does:
 *   1. Creates and listens on the unix socket
 *   2. execs the real dev server with PORT=<n> HOST=127.0.0.1 injected
 *   3. Proxies connections between the socket and 127.0.0.1:<n>
 *
 * The dev server never knows about the unix socket.
 * RackProxy never sees a TCP port.
 */

#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#define BUFSIZE 65536
#define BACKLOG 64

static const char *g_socket_path = NULL;

static void cleanup(void) {
    if (g_socket_path) unlink(g_socket_path);
}

static void on_signal(int sig) {
    cleanup();
    _exit(0);
}

/* Bidirectional splice between two fds until one closes. */
static void bridge(int a, int b) {
    char buf[BUFSIZE];
    fd_set fds;
    int maxfd = (a > b ? a : b) + 1;

    while (1) {
        FD_ZERO(&fds);
        FD_SET(a, &fds);
        FD_SET(b, &fds);

        if (select(maxfd, &fds, NULL, NULL, NULL) <= 0) break;

        if (FD_ISSET(a, &fds)) {
            ssize_t n = read(a, buf, sizeof(buf));
            if (n <= 0) break;
            if (write(b, buf, n) != n) break;
        }

        if (FD_ISSET(b, &fds)) {
            ssize_t n = read(b, buf, sizeof(buf));
            if (n <= 0) break;
            if (write(a, buf, n) != n) break;
        }
    }

    close(a);
    close(b);
}

/* Connect to 127.0.0.1:port, retry for up to 30s (server may still be starting). */
static int connect_to_server(int port) {
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);

    for (int attempt = 0; attempt < 60; attempt++) {
        int sock = socket(AF_INET, SOCK_STREAM, 0);
        if (sock < 0) return -1;

        if (connect(sock, (struct sockaddr *)&addr, sizeof(addr)) == 0) {
            return sock;
        }

        close(sock);
        usleep(500000); /* 500ms */
    }
    return -1;
}

int main(int argc, char *argv[]) {
    const char *socket_path = NULL;
    int port = 0;
    int cmd_start = -1;

    /* Parse: --socket <path> --port <n> -- <cmd> [args...] */
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--socket") == 0 && i + 1 < argc) {
            socket_path = argv[++i];
        } else if (strcmp(argv[i], "--port") == 0 && i + 1 < argc) {
            port = atoi(argv[++i]);
        } else if (strcmp(argv[i], "--") == 0 && i + 1 < argc) {
            cmd_start = i + 1;
            break;
        }
    }

    if (!socket_path || port <= 0 || cmd_start < 0) {
        fprintf(stderr, "Usage: rack-bridge --socket <path> --port <n> -- <command> [args...]\n");
        return 1;
    }

    /* Ignore SIGPIPE — write errors are handled by return values. */
    signal(SIGPIPE, SIG_IGN);
    signal(SIGTERM, on_signal);
    signal(SIGINT, on_signal);

    g_socket_path = socket_path;
    atexit(cleanup);

    /* Create the unix socket. */
    unlink(socket_path); /* remove stale socket */
    int listen_sock = socket(AF_UNIX, SOCK_STREAM, 0);
    if (listen_sock < 0) { perror("socket"); return 1; }

    struct sockaddr_un un;
    memset(&un, 0, sizeof(un));
    un.sun_family = AF_UNIX;
    strlcpy(un.sun_path, socket_path, sizeof(un.sun_path));

    if (bind(listen_sock, (struct sockaddr *)&un, sizeof(un)) < 0) {
        perror("bind"); return 1;
    }
    if (listen(listen_sock, BACKLOG) < 0) {
        perror("listen"); return 1;
    }

    /* Fork: parent bridges connections, child execs the dev server. */
    pid_t child = fork();
    if (child < 0) { perror("fork"); return 1; }

    if (child == 0) {
        /* Child: exec the real dev server with PORT injected. */
        char port_str[16];
        snprintf(port_str, sizeof(port_str), "%d", port);
        setenv("PORT", port_str, 1);
        setenv("HOST", "127.0.0.1", 1);

        execvp(argv[cmd_start], &argv[cmd_start]);
        perror("execvp");
        _exit(1);
    }

    /* Parent: accept connections and bridge them to the TCP server. */
    while (1) {
        int client = accept(listen_sock, NULL, NULL);
        if (client < 0) {
            if (errno == EINTR) continue;
            break;
        }

        pid_t handler = fork();
        if (handler < 0) { close(client); continue; }

        if (handler == 0) {
            /* Grandchild: handle one connection. */
            close(listen_sock);
            int server = connect_to_server(port);
            if (server < 0) {
                const char *err = "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 30\r\n\r\nrack-bridge: server not ready\n";
                write(client, err, strlen(err));
                close(client);
                _exit(1);
            }
            bridge(client, server);
            _exit(0);
        }

        /* Parent continues accepting. */
        close(client);
        /* Reap zombie grandchildren non-blockingly. */
        waitpid(-1, NULL, WNOHANG);
    }

    return 0;
}

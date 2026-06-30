# Rack rewrite

Rack is a macOS menu bar app for running local services and Rack-owned hooks during development.

This branch is the in-progress rewrite. Use `main` as the product-behaviour reference, but new runtime code is split under `packages/` and Rust owns runtime truth.

## Package layout

- `packages/core` — shared config schema, parsing, backfill, validation, and write helpers.
- `packages/services` — local service process supervision and readiness detection.
- `packages/proxy` — Host-header routing for services and Rack hooks.
- `packages/hooks` — hook deployment/runtime support.
- `packages/sdk` and `packages/sdk-macro` — hook authoring APIs and macros.
- `packages/cli` — command-line entry points.
- Swift package targets provide the macOS shell and UI.

## Build and test

```bash
cargo test
make app
```

Useful scoped validation while working on config/core code:

```bash
cargo test -p rack-core
```

`make app` writes the bundled app to `dist/rack.app`.

## Config

Rack rewrite uses TOML source config at:

```text
~/.config/rack/config.toml
```

If `XDG_CONFIG_HOME` is set, Rack prefers:

```text
$XDG_CONFIG_HOME/rack/config.toml
```

The source config is user-editable and starts with a schema header:

```toml
# RACK:V1

use_standard_ports = false
terminal = "Ghostty"

[[services]]
id = "A123C23D-DBCB-4689-8A7F-D888B8A47BAE"
name = "DEFAULT"
host = "default"
run = "echo hi"
working_dir = "~"
auto_start = true
```

Rack also writes a generated, fully backfilled cache for runtime consumers at:

```text
~/Library/Caches/Rack/config.full.toml
```

Do not edit the generated cache directly.

Rack does not migrate old JSON config. The rewrite uses TOML only. Legacy per-service environment variables, custom domains, explicit ports, command arguments, and port flags are intentionally not part of the rewrite TOML schema. The rewrite keeps one shell `run` command, detects the loopback port opened by the process, and routes to that detected port.


## Service model

Services are long-running local processes managed by Rust. Swift displays snapshots and sends start/stop/restart/edit commands through the bespoke FFI bridge.

Each service maps `host` to `<host>.localhost`. With the current non-privileged listener, URLs include the proxy port when needed, for example:

```text
http://jaf.localhost:1355
```

Services do not configure ports. Rack starts the process, watches its process group, detects listening loopback ports with `lsof`, and registers the first detected port with the proxy. `stop` terminates the process group.

Logs are written per service under:

```text
~/Library/Logs/Rack/services/<service-id>.log
```

The menu bar preview renders ANSI colours and the terminal action opens the full log in the configured terminal.

## Proxy model

The proxy is Pingora-based and focused on Host-header routing:

- `<service>.localhost` routes to a detected service backend.
- `rack.local` routes to Rack-owned hooks.
- nested localhost hosts such as `fix-auth.myapp.localhost` are rejected for now.

The proxy waits briefly for a service that is still starting, rejects loopback proxy loops with `508 Loop Detected`, and tunnels WebSocket upgrades/frames.

## Hooks

Hooks are WASM modules deployed under:

```text
~/.rack/hooks/<hook-name>
```

Hook metadata is embedded in the WASM custom section by the SDK macros. There is no required `manifest.toml` in the rewrite runtime.

Example HTTP hook:

```rust
#[rack::payload]
struct Message {
    text: String,
}

#[rack::route(POST, "echo")]
fn echo(request: rack::Request<Message>) -> rack::Response {
    rack::Response::json(request.payload()).unwrap()
}
```

Example cron hook:

```rust
#[rack::cron("weekdays at 9:30am")]
fn tick(event: rack::CronEvent) {
    rack::log(format!("tick: {}", event.schedule));
}
```

Supported SDK conveniences include `Request<T>`, `Payload`, `#[rack::payload]`, `String`/`()` payloads, `Result<Response>` handlers, and response helpers such as `ok`, `created`, `bad_request`, `teapot`, `server_error`, `text`, `html`, `json`, `csv`, and `bytes`.

## CLI

Service commands:

```bash
rack service list
rack service add <id> <name> <host> <run> <working_dir> [--auto-start]
rack service edit <id|name|host> [--name ...] [--host ...] [--run ...] [--working-dir ...] [--auto-start true|false]
rack service start <id|name|host>
rack service stop <id|name|host>
rack service restart <id|name|host>
rack service remove <id|name|host>
rack service log <id|name|host>
```

Hook commands:

```bash
rack hook init <path>
rack hook build [path]
rack hook deploy [path]
rack hook list
rack hook remove <name>
rack hook test [path] [--hook <hook>] [--route <route>]
```

`rack hook deploy` moves the hook source into `~/.rack/hooks/<name>` and symlinks it back to the original path.

## App shortcuts and URL scheme

The app registers the `rack://` scheme. Supported URLs include:

```text
rack://settings
rack://service/start?id=<service-id>
rack://service/stop?id=<service-id>
rack://service/restart?id=<service-id>
rack://service/stop-all
rack://hooks/reload
```

Legacy `rack://server/...` and `rack://functions/reload` aliases are accepted.

## Notes for release QA

- Confirm `cargo test -p rack-core` passes after config changes.
- Confirm a fresh launch without config uses defaults and only writes the generated cache.
- Confirm launch with an existing `config.toml` preserves that source file.

- Confirm `dist/rack.app` includes `Contents/Frameworks/librack_services.dylib` and `Contents/Resources/rack-cli`.
- Confirm `rack://settings`, service actions, and hook reload URLs work from a built app.
- Confirm Settings add/edit/remove service works and persists TOML.
- Confirm start/stop/restart works from menu, Settings, CLI, Shortcuts, and URL actions.
- Confirm launch-at-login works from the built app location.

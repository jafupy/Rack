# Rack Functions

Rack functions are local WebAssembly packages that run behind `rack.local`.
They are meant for small HTTP tools, local automation, and scheduled jobs that
belong next to your development environment.

The supported authoring path is Rust compiled to `wasm32-wasip1`.

## Requirements

`rack fn compile` will try to install the Rust WASI target for you. To install
it yourself:

```bash
rustup target add wasm32-wasip1
```

Install `wasmtime` somewhere on your `PATH`:

```bash
brew install wasmtime
```

Rack uses `wasmtime` to execute `functions.wasm`.

## Quickstart

Create a package:

```bash
rack fn init my-functions
cd my-functions
```

Compile and test it locally:

```bash
rack fn test
```

Install it into Rack:

```bash
rack fn add
```

Call the example route while Rack.app is running:

```bash
curl http://localhost:1355/hello
```

## Package Shape

A function package is a directory with two files:

```text
my-functions/
  Cargo.toml
  manifest.toml
  src/lib.rs
  functions.wasm
```

`Cargo.toml` must build a `cdylib`:

```toml
[package]
name = "my-rack-functions"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
rack = { path = "/Users/you/.rack/sdk" }

[profile.release]
lto = true
opt-level = "z"
strip = true
```

`manifest.toml` declares the package name, routes, and scheduled jobs:

```toml
name = "my-functions"
version = "0.1.0"

[route.hello]
path = "/hello"
method = "GET"
function = "hello"

[route.assets]
path = "/assets/**"
method = "GET"
function = "assets"

[cron.heartbeat]
schedule = "every 5 minutes"
function = "heartbeat"
```

Each `function` value must match an exported Rust symbol.

## HTTP Functions

The Rust SDK is installed at `~/.rack/sdk` by `rack fn init`, `rack fn compile`,
and `rack fn add`. Function packages depend on it as the `rack` crate.

Use `#[rack::route]` to export a Rack HTTP function. Rack writes a JSON request
to stdin, the SDK parses it into `rack::Request<T>`, and the generated wrapper
writes the response JSON to stdout.

```rust
#[rack::route]
fn hello(req: rack::Request) -> rack::Response {
    rack::log::info(format!("{} {}", req.method(), req.path()));

    rack::response::ok().text("hello from wasm")
}
```

Request bodies can be typed with `#[rack::payload]`. Payloads are JSON by
default.

```rust
#[rack::payload]
struct CsvUpdate {
    body: String,
}

#[rack::route]
fn update_csv(req: rack::Request<CsvUpdate>) -> rack::Response {
    std::fs::write("data.csv", &req.body().body)?;

    rack::response::ok().csv(req.body().body.as_str())
}
```

The macro lets `?` work inside the handler. Payload parse errors return `400`.
Handler errors return `500`.

Build and install it in one step:

```bash
rack fn add
```

Package files can be referenced from Rust with `rack::fs!`. The path is rooted
where `Cargo.toml` and `manifest.toml` live:

```rust
const DATA_PATH: &str = rack::fs!("./public/data.csv");
```

Then call it while Rack.app is running:

```bash
curl http://localhost:1355/hello
```

If standard port forwarding is enabled in Rack.app, this also works:

```bash
curl http://rack.local/hello
curl https://rack.local/hello
```

### Request Format

HTTP route functions receive this JSON on stdin:

```json
{
  "method": "GET",
  "path": "/hello",
  "uri": "/hello?debug=1",
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"name\":\"Ada\"}",
  "route": {
    "package": "my-functions",
    "id": "hello",
    "path": "/hello",
    "pattern": "/hello",
    "method": "GET",
    "function": "hello",
    "is_glob": false,
    "matched_path": "/hello"
  }
}
```

Headers are lowercased. Duplicate headers are joined with `, `.

The `route` object describes the manifest route that Rack selected. For glob
routes, `path` and `pattern` contain the manifest glob, `is_glob` is `true`, and
`matched_path` contains the normalized request path.

### Response Format

For HTTP routes, stdout can be either plain text or a JSON response object.

Plain text becomes a `200 text/plain` response:

```rust
println!("hello");
```

Structured responses give you status, headers, and body control:

```json
{
  "status": 201,
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"ok\":true}"
}
```

`status` must be between `100` and `599`. Header values must be strings. `body`
must be a string.

## Scheduled Functions

Cron functions use `#[rack::cron]`. Rack invokes them on a schedule instead of
through HTTP.

```rust
#[rack::cron]
fn heartbeat(event: rack::CronEvent) -> rack::Response {
    rack::log::info(format!("scheduled at {}", event.scheduled_at));
    rack::response::ok().text("heartbeat")
}
```

Cron functions receive this JSON on stdin:

```json
{
  "type": "schedule",
  "package": "my-functions",
  "id": "heartbeat",
  "schedule": "every 5 minutes",
  "scheduled_at": "2026-05-31T12:00:00+01:00"
}
```

Supported schedules:

```text
every 30 seconds
every 5 minutes
every 2 hours
every 1 day
9:30am
17:00
weekdays at 9am
monday at 5pm
```

## Logs

`rack::log::info`, `rack::log::warn`, and `rack::log::error` write structured
logs for both routes and crons. Rack stores them as daily JSONL files:

```text
~/.rack/logs/functions/<package>/routes/<route-id>/YYYY-MM-DD.jsonl
~/.rack/logs/functions/<package>/crons/<cron-id>/YYYY-MM-DD.jsonl
```

Raw stderr from functions is logged there too. Stdout is reserved for function
responses.

## CLI

Create a new Rust/WASI function package:

```bash
rack fn init my-functions
```

Build the current package without installing it:

```bash
rack fn compile
```

Build and install the current package:

```bash
rack fn add
```

Build and install another package:

```bash
rack fn add examples/hello-route
```

Compile and run the first route or cron locally through `wasmtime`:

```bash
rack fn test
```

Run a specific export:

```bash
rack fn test examples/hello-route hello
```

Install a built package into `~/.rack/functions/<name>`:

```bash
rack fn install
```

Install as a symlink to the source directory, so Rack runs the package in place:

```bash
rack fn install --link
```

Reinstall after rebuilding:

```bash
rack fn install --replace
```

If an installed copy already exists and you want to switch it to a symlink, use
both flags:

```bash
rack fn install --link --replace
```

List installed packages:

```bash
rack fn ls
```

Remove a package:

```bash
rack fn rm my-functions
```

## Routing Rules

- Functions are served from `rack.local` when standard port forwarding is enabled.
- Without standard port forwarding, use `localhost:<rack-port>`; the default is
  `localhost:1355`.
- `/` is reserved for Rack itself.
- Paths beginning with `/_` are reserved.
- Route paths are normalized with a leading slash and without trailing slashes.
- Route paths support glob syntax through `globset`: `*`, `?`, character
  classes like `[a-z]`, alternates like `{json,html}`, and recursive `**`.
- Exact routes are preferred over glob routes. More specific glob routes are
  preferred over broader ones. If multiple matching routes have the same
  specificity, Rack returns a conflict instead of choosing one.

## Runtime Model

Rack starts a fresh `wasmtime` process for each invocation. That keeps function
state simple: use files, environment variables, or external services when state
needs to survive between calls.

Rack runs `wasm32-wasip1` functions with the local WASI profile Rack functions
need: CLI APIs, stdio, clocks, random, inherited environment, and filesystem
access. Rack grants filesystem access to the local root with `--dir /::/`.

Rack also enables outbound network client calls through WASI TCP, UDP, and name
lookup. That means a function can fetch APIs, shell out to `curl` if it is
available in the guest environment, or call a local service.

Rack does not use functions as servers. Functions should do finite work and
return; they should not bind a port, start their own HTTP server, or become a
daemon. Rack is the server boundary.

Functions are local development tools, so do not install untrusted packages.

HTTP invocations time out after 30 seconds. Rack limits concurrent function
workers; the default is 4.

### Wasmtime Profile

Rack and `rack fn test` use the same profile:

```text
wasmtime run \
  --dir /::/ \
  -S cli=y \
  -S allow-ip-name-lookup=y \
  -S tcp=y \
  -S udp=y \
  -S inherit-env=y \
  --invoke <function> \
  functions.wasm
```

Rack also passes host environment variables explicitly with `--env KEY=VALUE`.

## Troubleshooting

`rack: wasmtime is required to run functions.wasm`
: Install `wasmtime` and make sure Rack.app can find it on `PATH`.

`missing functions.wasm`
: Run `rack fn add`, or run `rack fn compile` before using low-level
  `rack fn install`.

`route conflict`
: Run `rack fn ls`, then remove or change the package that owns the same
  method and path.

`function failed`
: Run the wasm directly with `wasmtime` or check stderr from the Rust function.

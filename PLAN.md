# Plan

Hello agent(s). Today we are rewriting Rack.

This is not a small refactor. We are boiling the ocean. The current repo has accumulated weird structure, mega-files, blurred responsibilities, rot and tech debt. The existing main branch should be functionality reference only.

The rewrite has already been started. Keep existing code as close to as is as possible unless things have to be changed for compat with the new. use main as a reference for features.

Goals

* Clean up the architecture.
* Make Rack faster.
* Make the code easier to read, test, and maintain.
* Preserve the product behaviour that works.
* Do not recreate the current mega-file / god-object structure.

## Product Renames

Functions → Hooks
Dev servers → Services

## Repository Layout

All new code lives under packages.
```
packages/
  blob       # Rust. Local object/blob storage for hooks.
  cli        # Rust. Command-line interface.
  core       # Rust. Shared config, types, errors, validation, and core utilities.
  hooks      # Rust. Hook runtime. Replaces what is currently functions-related.
  mac        # Swift. macOS app shell and OS integration: app lifecycle, intents, login items, bridge ownership, DNS/privileged integration later.
  proxy      # Rust. Reverse proxy and Host-based routing.
  sdk        # Rust. Library for making hooks easier to write.
  sdk-macro  # Rust. Proc macros for the SDK.
  services   # Rust. Long-running process supervision. Replaces current dev server runtime.
  ui         # Swift. SwiftUI views only.
```

## Architecture Rules

Rack should be one app/binary from the user’s perspective, but internally split into clear packages.

Do not put runtime ownership into SwiftUI views. SwiftUI should render state and call a thin model/API.

Do not recreate ServerStore as a god object. The old Swift side mixed UI, process lifecycle, config persistence, proxy coordination, logs, terminal integration, and app lifecycle. The rewrite must keep those responsibilities separate.

Rust owns the runtime truth. Swift owns the native macOS shell and UI.

## Package Responsibilities

### core

Owns shared foundations:

* Config schema.
* Config parsing and serialization.
* Defaults/backfill.
* Validation.
* Shared types.
* Shared errors.

Config should be TOML. Source config should be user-editable. Generated/full config can exist as a cache, but normal load should not rewrite the user’s source config.

### services

Owns long-running local processes.

Responsibilities:

* Register services from config.
* Start services.
* Stop services.
* Track lifecycle state.
* Track process IDs and process groups.
* Detect opened loopback ports.
* Report service status to the rest of Rack.

Services do not have explicit configured ports in the new model. Rack starts a process, watches it, detects the port it opens, and exposes it through the proxy.

Service lifecycle should roughly be:

Stopped
Starting { pid, pgid }
Running { pid, pgid, ports }

Registry state is the source of truth. If the registry and process handles disagree, treat that as an explicit desync/internal error. Do not silently paper over impossible states.

### proxy

Owns HTTP routing.

There should be one proxy listener that routes based on the Host header.

Planned host model:

*.localhost  -> services
rack.local   -> Rack-owned hooks/control surface

Examples:

jaf.localhost  -> service named/hosted as "jaf"
api.localhost  -> service named/hosted as "api"
rack.local     -> hooks / Rack-native surface

The proxy should not own service lifecycle. It should consume service targets from the runtime.

### hooks

Hooks replace the old “functions” concept.

Hooks are Rack-native event-triggered code. Under the hood they run as WASM, but the product name is Hooks.

### blob

Local object/blob storage for hooks. Think S3-like local storage.

### mac (swift)

Owns the macOS app shell and OS-specific integration.

Responsibilities include:

* RackApp.
* AppDelegate.
* App lifecycle.
* Swift/Rust bridge ownership.
* Launch at login.
* App Intents.
* Terminal opening.
* Future rack.local DNS/resolver/privileged integration.
* Future Finder drag/drop integration.

mac owns the executable target.

### ui (swift)

Owns SwiftUI views only.

Responsibilities include:

* Menu bar popover views.
* Service row views.
* Settings views.
* UI-only display utilities such as ANSI log rendering.

ui should not start processes, own config persistence, own proxy routes, or directly manage runtime state.

### cli

Command-line interface for Rack.

Possible commands:

rack service add
rack service start
rack service stop
rack service list
rack hook add

### sdk and sdk-macro

Current rack-macros folder but rewritten and cleaned up.

## Swift Split

The Swift side should be split like this:

packages/mac
  executable target
  app shell
  OS integration
  runtime bridge
packages/ui
  library target
  SwiftUI views
  UI-only helpers

Current UI Direction

The existing menu bar UI is good and should be mined/ported. The product shape is:

* Small native menu bar popover.
* Service status rows.
* Start/stop controls.
* Running count.
* Settings button.
* Quit button.
* Log preview eventually.

However, port the UI against new models. Do not port the old runtime ownership.

Old concepts should map roughly like this:

ServerStore              -> thin Rack view model / runtime bridge
ServerConfiguration      -> ServiceView
ServerStatus             -> ServiceState
startServer / stopServer -> start(id) / stop(id)

Config Direction

Services should look roughly like:

[[services]]
id = "..."
name = "Jafupy.com"
host = "jaf"
run = "bun dev"
working_dir = "/Users/jafu/Projects/jafupy.com"
auto_start = false

host = "jaf" maps to jaf.localhost.

Guiding Principle

The old app proved the product idea. The rewrite should preserve the good product behaviour while replacing the architecture.

Keep runtime truth in Rust. Keep SwiftUI dumb and native. Keep package boundaries real.

Write a new FFI translation layer that is highly performant. Avoid having swift and rust on separate threads. Multithread/async the app where it makes sense.

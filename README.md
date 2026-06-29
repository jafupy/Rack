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

`make app` writes the bundled app to `dist/Rack.app`. `make dmg` builds the drag-and-drop installer image at `dist/Rack.dmg`.

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

## Legacy JSON migration

On load, existing TOML always wins and is not rewritten by the backfill path.

If `config.toml` is missing, Rack looks for old JSON service config in this order:

1. `$XDG_CONFIG_HOME/rack/config.json`
2. `~/.config/rack/config.json`
3. `~/.config/server-bar/config.json`
4. `~/Library/Application Support/ServerBar/servers.json`

When a legacy JSON file is found, Rack writes a one-time TOML migration to the current source config path and then loads/caches that TOML.

Legacy server fields are mapped as follows:

| Legacy JSON | Rewrite TOML |
| --- | --- |
| `id` | `services[].id` |
| `name` | `services[].name` |
| `customDomain` or `name` | `services[].host`, lowercased, spaces changed to `-`, trailing `.localhost` removed |
| `command` + `arguments` | `services[].run` |
| `workingDirectory` | `services[].working_dir`, or `~` when blank |
| `autoStart` | `services[].auto_start` |

Legacy per-service environment variables, explicit ports, and port flags are not part of the rewrite TOML schema yet.

## Notes for release QA

- Confirm `cargo test -p rack-core` passes after config changes.
- Confirm a fresh launch without config uses defaults and only writes the generated cache.
- Confirm launch with an existing `config.toml` preserves that source file.
- Confirm launch with only legacy JSON writes `config.toml` once and preserves the legacy JSON file.
- Confirm `dist/Rack.app` includes the Rust dynamic library and bundled CLI before packaging a DMG.

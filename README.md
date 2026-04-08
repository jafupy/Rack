# Rack.

Rack. is a small macOS menu bar app for running and monitoring multiple dev servers at once.

## What it does

- Runs multiple commands concurrently from your menu bar
- Persists named server configs
- Supports working directories, args, and environment variables
- Lets you start, stop, and restart each server independently
- Stores recent process output so you can inspect logs quickly

## Run it

```bash
swift run
```

## Build a `.app`

```bash
./scripts/build-app.sh
```

That writes the bundled app to:

```text
dist/Rack.app
```

The first launch creates a config file at:

```text
~/.config/rack/config.json
```

Rack. will automatically migrate existing `ServerBar` config and terminal preferences on first launch.

## Notes

- Commands are executed in an interactive login `zsh` shell
- Arguments are currently split on whitespace, so quoted shell syntax is not supported yet
- This is intended for local dev servers, not long-lived production services

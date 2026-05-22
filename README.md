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

On launch, `Rack.app` installs the bundled `rack` CLI at:

```text
~/.local/bin/rack
```

and adds `~/.local/bin` to zsh's PATH through `~/.zprofile` if needed.

## Build a `.dmg`

```bash
./scripts/build-dmg.sh
```

That writes the installer disk image to:

```text
dist/Rack.dmg
```

The DMG includes `Rack.app` plus an `Applications` shortcut for drag-and-drop install.

The first launch creates a config file at:

```text
~/.config/rack/config.json
```

Rack. will automatically migrate existing config and terminal preferences on first launch.

## Unsigned Install

`Rack.` is currently distributed as an unsigned macOS app.

- Open the DMG and drag `Rack.app` into `Applications`
- If macOS blocks the first launch, right-click the app and choose `Open`
- If needed, allow it in System Settings > Privacy & Security

## Notes

- Commands are executed in an interactive login `zsh` shell
- Arguments are currently split on whitespace, so quoted shell syntax is not supported yet
- This is intended for local dev servers, not long-lived production services

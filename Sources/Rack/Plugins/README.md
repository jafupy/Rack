# Rack Detection Plugins

This directory contains compiled WASM plugins bundled with Rack.app.

Built-in detectors (Swift) take priority and cover common stacks.
These WASM plugins extend detection without recompiling Rack.

## Writing a plugin

Plugins are WASM/WASI binaries that:
- Read a JSON `ProjectManifest` from stdin
- Write a JSON `DevCommand` to stdout, or nothing/"null" for no match

### ProjectManifest (stdin)
```json
{
  "files": ["package.json", "vite.config.ts", ...],
  "contents": {
    "package.json": "{ \"name\": \"myapp\", ... }",
    ...
  }
}
```

### DevCommand (stdout)
```json
{
  "command": "npm run dev",
  "env": {},
  "name": null,
  "portFlag": null
}
```

Return nothing or `null` to pass to the next detector.

### Priority

Name your plugin `<priority>-<name>.wasm`, e.g. `200-deno.wasm`.
Higher numbers run first. Built-in Swift detectors run at 100-110.
User plugins default to 200.

### Install

Drop .wasm files into `~/.config/rack/plugins/`.
Rack picks them up on next launch.

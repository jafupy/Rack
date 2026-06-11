# Rack Function Examples

Each example is a standalone Rust crate that builds to Rack's expected function
package shape:

```text
manifest.toml
functions.wasm
```

Build and install one with:

```bash
cd examples/hello-route
rack fn add
```

See [../docs/rack-functions.md](../docs/rack-functions.md) for the full package
format, request/response contract, and schedule syntax.

The examples target `wasm32-wasip1`. If Rust does not have that target yet:

```bash
rustup target add wasm32-wasip1
```

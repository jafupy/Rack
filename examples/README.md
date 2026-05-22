# Rack Function Examples

Each example is a standalone Rust crate that builds to Rack's expected function
package shape:

```text
manifest.toml
functions.wasm
```

Build one with:

```bash
cd examples/hello-route
./build.sh
```

Then install it with:

```bash
rack function .
```

The examples target `wasm32-wasip1`. If Rust does not have that target yet:

```bash
rustup target add wasm32-wasip1
```


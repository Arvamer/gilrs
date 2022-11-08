# Wasm Example

These are instructions for running the GUI example in your web browser using Wasm.

Currently only the GUI example is set up to run with Wasm.

### Ubuntu requirements
```bash
sudo apt install build-essential
sudo apt-get install libssl-dev pkg-config
```

### Setup

```pwsh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo install basic-http-server
```


### Build and Run

Run these from the workspace root.

```pwsh
cargo build --release --example gui --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example --out-dir gilrs/examples/wasm/target --target web target/wasm32-unknown-unknown/release/examples/gui.wasm
basic-http-server gilrs/examples/wasm
```

Now open your web browser and navigate to http://127.0.0.1:4000

### This script can be used instead of the "Build and Run" step in `./gilrs/examples/wasm/README.md`.
### Useful for gilrs devs that want a single script to to point their IDE to for run configurations.
### Supports Powershell 5 and up on Windows or Linux
### Make sure to run the install steps from the readme first.

# Start at this script's path and go up three levels to the workspace root.
# Ensures a consistent path regardless of the working directory when you run the script.
$Path = $PSScriptRoot | Split-Path | Split-Path | Split-Path
$ProjectDir = Resolve-Path $Path
Set-Location $ProjectDir

cargo build --release --example gui --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example --out-dir gilrs/examples/wasm/target --target web target/wasm32-unknown-unknown/release/examples/gui.wasm
basic-http-server gilrs/examples/wasm

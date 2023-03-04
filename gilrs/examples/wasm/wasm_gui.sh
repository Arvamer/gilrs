#!/usr/bin/env bash

### This script can be used instead of the "Build and Run" step in `./gilrs/examples/wasm/README.md`.
### Useful for gilrs devs that want a single script to to point their IDE to for run configurations.
### Make sure to run the install steps from the readme first.

set -e

# Start at this script's path and go up three levels to the workspace root.
# Ensures a consistent path regardless of the working directory when you run the script.
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_DIR=$(dirname "$(dirname "$(dirname "$SCRIPT_DIR")")")
cd "$PROJECT_DIR" || exit

cargo build --release --example gui --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example --out-dir gilrs/examples/wasm/target --target web target/wasm32-unknown-unknown/release/examples/gui.wasm
basic-http-server gilrs/examples/wasm

#!/bin/sh
# Build and type-check the wasm package.
# Full integration tests require wasm-pack test --headless --chrome (browser env).
# cargo test panics with newer wasm-bindgen on non-wasm targets because
# #[wasm_bindgen] struct allocation calls __wbindgen_malloc (a wasm host import).
set -e
cargo build

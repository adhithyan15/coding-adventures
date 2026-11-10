# build-tool: deps=rust/mosaic-compile rust/spreadsheet-wasm
# tests/web_demos_are_current.rs builds the VisiCalc engine bundle for the
# browser, so the wasm target must be present (BUILD-001: it is not committed).
rustup target add wasm32-unknown-unknown
cargo test

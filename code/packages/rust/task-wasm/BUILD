# The ABI logic is unit-tested natively (the exports are plain pointer + JSON logic).
cargo test -p task-wasm -- --nocapture
# The .wasm is generated, not committed (BUILD-001): build it for the browser
# target and smoke-test that fresh build with node.
rustup target add wasm32-unknown-unknown
bash build-wasm.sh
node js/smoke.mjs

# needs-toolchain: typescript
# needs-toolchain: python
# needs-toolchain: rust
mkdir -p wasm-output
cargo run --manifest-path ../../../specs/fixtures/historical-unix-v1/test-adapters/wasm/Cargo.toml --bin wat-to-wasm --target-dir wasm-output/cargo-target --locked --offline -- src/unix_history.wat wasm-output/unix_history.wasm
node ../../../specs/fixtures/historical-unix-v1/test-adapters/wasm/component.test.mjs wasm-output/unix_history.wasm
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py node ../../../specs/fixtures/historical-unix-v1/test-adapters/wasm/fixture_cli.mjs wasm-output/unix_history.wasm

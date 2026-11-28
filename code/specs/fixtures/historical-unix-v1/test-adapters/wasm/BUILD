# Standalone smoke test for the fixture-side compiler. The WASM package BUILD
# additionally runs its component and full canonical trace replay checks.
cargo run --manifest-path Cargo.toml --bin wat-to-wasm --target-dir target --locked --offline -- ../../../../../packages/wasm/unix-history-simulator/src/unix_history.wat target/unix_history.wasm

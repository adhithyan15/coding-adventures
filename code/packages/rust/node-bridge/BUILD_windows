# node-bridge is a library with no tests of its own: every napi_* import it
# declares is exercised by the addons built on it (font-parser-node's smoke test
# loads a real cdylib into Node.js).  Here we prove it builds, and is lint-clean,
# on every OS.
cargo build --locked --release -p node-bridge
cargo clippy --locked -p node-bridge --all-targets -- -D warnings

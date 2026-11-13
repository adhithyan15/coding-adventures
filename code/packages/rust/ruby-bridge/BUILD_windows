# ruby-bridge is a library with no tests of its own: every rb_* import it
# declares is exercised by the extensions built on it (font-parser-ruby's smoke
# test requires a real cdylib into Ruby).  Here we prove it builds, and is
# lint-clean, on every OS.
cargo build --release -p ruby-bridge
cargo clippy -p ruby-bridge --all-targets -- -D warnings

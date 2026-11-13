# python-bridge is a library with no tests of its own: every Py* import it
# declares is exercised by the extension modules built on it (font-parser-python's
# smoke test imports a real cdylib into CPython).  Here we prove it builds, and
# is lint-clean, on every OS.
cargo build --release -p python-bridge
cargo clippy -p python-bridge --all-targets -- -D warnings

---
category: Rust
---

# Pin the installed full Rust toolchain version before parallel validation to avoid racing rustup alias installation

The installed CI-matching toolchain was `1.99.0`, but parallel validation mistakenly invoked `cargo +1.99`. Rustup treated that short alias as a separate installation and concurrent downloads raced over the same partial file; no test ran. Inspect `rustup toolchain list` and use the exact installed `cargo +1.99.0` name in every validation command. Keep compiler/package validation failures distinct from toolchain setup failures.

The validation helper also assumed Python 3.11 tomllib, relative repository-root lesson paths while running at the Rust workspace, and default Windows text decoding. Use the exact known workspace root, standard-library dependency/name matching compatible with Python 3.10, and explicit UTF-8 for repository files. Setup failures are not test failures.

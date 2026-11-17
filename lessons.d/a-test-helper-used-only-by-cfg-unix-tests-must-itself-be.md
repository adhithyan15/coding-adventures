---
category: Cross-platform & Windows BUILD_windows
---

# A test helper used only by #[cfg(unix)] tests must itself be #[cfg(unix)], or clippy -D warnings fails on Windows

PR #19577, Windows CI job 134310005032, ran `cargo clippy --all-targets -- -D warnings` and failed with `error: function 'within' is never used` at `code/packages/rust/warrant-corpus-gen/tests/check_cli.rs:445`.

What went wrong: `within` is a test helper that runs a closure on a thread with a timeout. Every caller sits inside a `#[cfg(unix)]` test or a `#[cfg(unix)]` block (the FIFO tests). On Linux and macOS the helper is used, so nothing warns. On Windows the callers are compiled out, the helper is never referenced, and the `dead_code` lint fires. Clippy runs with `-D warnings`, so the lint is fatal.

The fix: put `#[cfg(unix)]` on the helper too, between its doc comment and the `fn` line.

What to do differently: when a helper's only callers are behind a platform gate, gate the helper with the same attribute. Before pushing a cross-platform test change, grep every call site of the helper and confirm each one is inside an item or block with the same `cfg`. Clippy on Linux cannot catch this, so the Windows target (`rustup target add x86_64-pc-windows-gnu`, then `cargo clippy --all-targets --target x86_64-pc-windows-gnu -- -D warnings`) is the check that finds it locally.

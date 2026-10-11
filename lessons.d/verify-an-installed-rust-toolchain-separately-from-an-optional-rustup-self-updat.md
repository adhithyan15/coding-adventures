---
category: Rust
---

# Verify an installed Rust toolchain separately from an optional rustup self-update failure

Installing the additional Rust 1.99 toolchain completed its components, then
rustup's optional self-update failed because its updater executable was missing.
The overall installation command returned nonzero, although the requested
toolchain was usable. Treating that exit as proof of absent components would
have caused an unnecessary reinstall or blocked the real compiler validation.

Verify the exact installed toolchain with `rustup run 1.99.0 rustc -V` and the
actual strict build command. Those reported Rust 1.99 and passed all-target
Clippy. Keep the default toolchain unchanged and distinguish optional updater
failure from compiler/test failure; do not restart a still-live installation.

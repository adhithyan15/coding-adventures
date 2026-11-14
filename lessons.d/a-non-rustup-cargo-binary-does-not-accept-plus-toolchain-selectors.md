---
category: Rust
---

# A non-rustup Cargo binary does not accept plus-toolchain selectors

The macOS workstation can resolve `cargo` to Homebrew while `rustup` resolves
through an untrusted mise shim. In that state, `cargo +1.100.0 ...` fails before
Cargo runs because only the rustup proxy understands the `+toolchain` selector,
and invoking the shim fails on repository trust instead. For a reviewed pinned
toolchain already installed under `~/.rustup/toolchains`, prepend that exact
toolchain's `bin` directory to `PATH` for the command. This selects matching
`cargo`, `rustc`, and `rustdoc` binaries without changing repository trust or
silently falling back to the unrelated Homebrew toolchain.

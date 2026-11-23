---
category: Rust
---

# Verify the running clippy version before trusting local lint results

`cargo clippy` resolves the `cargo-clippy` subcommand through PATH, so
invoking an absolute toolchain cargo (e.g.
`~/.rustup/toolchains/1.100.0-*/bin/cargo clippy`) still runs the *default*
toolchain's clippy when the pinned toolchain lacks the component — and even
with the component installed, a rustup shim earlier on PATH wins. The failure
mode is silent: lint output carries the wrong toolchain's lints (one stale
`needless_borrows_for_generic_args` set failed `-D warnings` locally while CI
on the pinned toolchain was green, on byte-identical sources).

Before trusting a local clippy gate, run `cargo clippy --version` through the
same invocation and confirm it prints the pinned release (root `mise.toml`,
`rust` key; CI enforces the match in `toolchain_pins.py`). The reliable
invocation is `rustup run <pinned-toolchain> cargo clippy ...`, or export
`RUSTUP_TOOLCHAIN=<pinned-toolchain>` with the toolchain's `bin/` first on
PATH. Install a missing component with
`rustup component add clippy --toolchain <pinned>`.

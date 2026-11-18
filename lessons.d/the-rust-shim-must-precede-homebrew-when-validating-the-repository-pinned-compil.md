---
category: Rust
---

# The Rust shim must precede Homebrew when validating the repository-pinned compiler

In an isolated worktree whose `mise.toml` had not been trusted, the shell
resolved Homebrew's `/opt/homebrew/bin/cargo` and `rustc` before the installed
`~/.cargo/bin` rustup shims. The repository pins Rust 1.100.0, but the build
runner actually used Homebrew rustc 1.94. A selected, otherwise-unmodified VOS
package then failed on `isolate_lowest_one`, which is unavailable in that older
toolchain. This looked like a source regression until the executable and
version behind `cargo` were checked.

Before trusting a local build-tool result, read the exact repository pin and
CI pin, then inspect `command -v cargo`, `command -v rustc`, `cargo --version`,
and `rustc --version`. If the project mise file is untrusted, do not trust it
just to activate Rust. Put the installed rustup shims before system package
managers on `PATH` and set `RUSTUP_TOOLCHAIN` to the repository's exact pin;
verify the reported compiler version, then rerun the selected build. Keep the
original wrong-toolchain failure separate from the correctly pinned run. This
is distinct from a stale `stable` alias: a different executable can shadow
rustup entirely even when the pinned toolchain is already installed.

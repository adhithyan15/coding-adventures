---
category: Rust
---

# Run cargo clippy --all-targets -- -D warnings before pushing Rust, test code included

CI's build tool runs `cargo clippy --all-targets -- -D warnings` before a
Rust package's BUILD commands (`build-tool --clippy`); the BUILD file itself
only says `cargo test`. #16608 added a cross-host test in
`mosaic-app-bindings` that split a string with `.split(|c| c == '"' || c ==
'\'')`. Rust 1.99's `clippy::manual_pattern_char_comparison` rejects that,
so the package failed to compile under `-D warnings`. Every local run had
used `cargo test`, which never runs clippy, so it passed. Four dependent
packages were skipped behind it.

The fix was `.split(['"', '\''])`.

**Do instead:** before pushing any Rust change, run
`cargo clippy -p <crate> --all-targets -- -D warnings` on each crate it
touches. `--all-targets` covers `#[cfg(test)]` code, which is where new
helpers tend to land. Run it with the same stable toolchain CI uses, because
a new stable release adds lints.

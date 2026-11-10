---
category: Rust
---

# macOS CI runs a newer Rust than Linux CI and local, so run clippy with that toolchain before pushing Rust

WX-A (#19112) passed `cargo clippy --all-targets -- -D warnings` locally
(Rust 1.97, the same toolchain as the Linux job) but failed `build
(macos-latest)`. The macOS image ships Rust 1.99, whose clippy adds
`needless_borrows_for_generic_args` for `.map_err(&closure)`, where the
closure is `Copy`. That gave ten errors in warrant-grammar, and warrant-ir
failed too because it compiles warrant-grammar under `-D warnings`. The
other "newer clippy" lessons (`chunks_exact`) are the same failure class.

Do this: the clippy version in the log line
`rust-clippy/rust-X.Y.Z/index.html` is the runner's. Before pushing Rust
that touches new code, run
`rustup toolchain install X.Y.Z --profile minimal -c clippy` and then
`CARGO_TARGET_DIR=<separate dir> cargo +X.Y.Z clippy -p <crate> --all-targets -- -D warnings`
for each changed crate. A separate target dir avoids rebuilding the main
toolchain's cache. A clean run on the local toolchain alone does not show
the macOS job will pass.

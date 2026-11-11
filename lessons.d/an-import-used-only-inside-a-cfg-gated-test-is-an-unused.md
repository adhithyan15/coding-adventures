---
category: Rust
---

# An import used only inside a cfg-gated test is an unused-import error on every other platform; cross-check with --target before pushing

WX-H/E (#19121) passed clippy with `-D warnings` on Linux, both locally and
in CI, but failed `build (macos-latest)`. `tests/wxe_edges.rs` imported
`vectors::read_fixture` at the top of the file, and its only use was a
`#[cfg(target_os = "linux")]` test that reads `/proc/self`. On every other
platform the import is unused, and `-D warnings` makes that an error. The
Linux job could never see it.

The fix: drop the item from the shared `use` and call it by path
(`vectors::read_fixture`) inside the gated test, or gate the import with the
same `cfg`.

Do this: when a crate has cfg-gated tests or code, run clippy for a
non-host target before pushing, e.g.
`rustup target add aarch64-apple-darwin` and then
`cargo clippy -p <crate> --all-targets --target aarch64-apple-darwin -- -D warnings`.
Type-checking needs no linker, so this works on Linux. Install the target
for the toolchain you run: `cargo +1.99.0` with a target installed only for
stable fails with "can't find crate for core". This is the import-side twin
of `a-pre-main-hook-in-elf-init-array-does-not-run-on`.

---
category: Rust
---

# A Rust std deprecation on floating stable breaks -D warnings in crates nobody touched

CI installs Rust through `dtolnay/rust-toolchain` on the floating `stable`
channel and builds with `-D warnings`. Rust 1.99 deprecated
`AtomicUsize::fetch_update` in favour of `try_update`. From then on,
`generic-job-runtime` would not compile on any CI run that rebuilt it, even
though no one had touched it.

Diff-based CI hid the break. #16935 changed `vault-sealed-store`, the build
tool rebuilt its dependents and their dependencies, and `generic-job-runtime`
came along. A latent toolchain break then looked like that PR's fault, on
macOS and Linux at once. The local toolchain was 1.97, so `cargo clippy`
passed there.

What to do:

- When CI fails with `use of deprecated` in a crate the PR did not change,
  suspect toolchain drift before anything else. Reproduce it with
  `rustup toolchain install stable` and run `cargo +stable clippy` across the
  same affected set the build tool computes (`build-tool -dry-run`). Fix every
  hit in one push, not one per CI round. Re-run after the fix, because
  `--keep-going` cannot lint the dependents of a crate that failed to compile.
- Do not apply the compiler's suggested replacement (`try_update`) when it
  does not exist on older toolchains contributors still run. Write the
  operation so it compiles on both sides of the rename. For `fetch_update`
  that is an explicit `compare_exchange_weak` loop, which is what
  `closure-pass-pipeline` had already done.

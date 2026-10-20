---
category: Rust
---

# Run cargo clippy --all-targets -D warnings on every touched Rust crate before pushing, not just cargo test

**What went wrong.** PR #17796 added a `twig-aot` unit test that wrote
`compile_module_x86_64_to_text(&m, abi).err().expect("refused")`. `cargo test`
passed locally, along with the full `lang_matrix` and the security review.
Then `build (windows-2025)` failed: the build tool runs clippy over test
targets with `-D warnings`, and `clippy::err_expect` rejects
`.err().expect()`. It cost one CI cycle on a change that was otherwise green.

**The fix.** Use `let Err(err) = … else { panic!(…) };`. That also avoids
`expect_err`'s `Debug` bound on the `Ok` type.

**Next time.** Before pushing, run
`cargo clippy -q --all-targets -- -D warnings` in each Rust crate the diff
touches, as well as `cargo test`. Test-only code is linted too: `--all-targets`
is what reaches `#[cfg(test)]` modules and `tests/`.

**It happened again, for a different reason (2026-10-20).** #17800's
`aarch64-backend` test used `bytes.chunks_exact(4)`. Local clippy was clean,
but Windows CI failed with `clippy::chunks_exact_to_as_chunks`: the local
toolchain was Rust 1.97 and CI's is 1.99, which added that lint. Clean local
clippy only means something if it is CI's clippy. Check the version CI uses
(the log's lint-help URL shows it, e.g. `rust-clippy/rust-1.99.0`), install it
with `rustup toolchain install 1.99.0 --profile minimal -c clippy`, and run
`cargo +1.99.0 clippy --all-targets -- -D warnings`.

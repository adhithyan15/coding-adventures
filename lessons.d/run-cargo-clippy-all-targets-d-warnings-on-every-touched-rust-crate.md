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

---
category: CI & GitHub Actions
---

# Run cargo clippy --all-targets -D warnings before pushing new Rust tests

**What went wrong.** I added acceptance tests to three emitter crates from one
shared template. Each test looped `for needle in [...]` over a list of
expected strings. Two of the lists held a single element, so CI's
`build (ubuntu-latest)` job failed: the build tool runs clippy with
`-D warnings`, and `clippy::single_element_loop` became an error. Locally I
had run only `cargo test` and `cargo check --tests`, neither of which runs
clippy.

**Fix.** Give every needle list at least two real assertions. Both tests
gained a second check, the circle's size.

**Next time.** Before pushing Rust that adds or changes tests, run
`cargo clippy --all-targets -- -D warnings` in each changed crate. That is
what CI runs; `cargo test` alone does not catch lint-only failures. When
code is generated from a template, check the instances whose inputs differ.
Here the Qt instance passed only because its list happened to have two
entries.

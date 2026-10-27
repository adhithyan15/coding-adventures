---
category: Rust
---

# Rebuilding a crate while a long cargo test run is still going makes that run's later doctests fail with E0460

`cargo test -p twig-module-driver` runs the unit tests (about 11 minutes,
because of the TW05 self-compile suite) and only then compiles and runs the
doctests.  While it was in its unit-test phase, a second
`cargo test -p iir-linker` in the same target directory rebuilt
`iir_linker` with a new edit.  When the first run reached its doctests,
`rustdoc` linked them against the *new* `iir_linker` rlib while
`twig_module_driver` had been built against the old one, and all three
doctests failed with `error[E0460]: found possibly newer version of crate
iir_linker`.  Nothing was wrong with the code: rerunning
`cargo test -p twig-module-driver --doc` alone passed.

What to do differently:

- Do not rebuild any dependency of a crate whose long `cargo test` is still
  running in the same target directory; wait, or use `cargo clippy` /
  `cargo check`, which write to a separate metadata profile and do not
  replace the rlibs the test run links against.
- If E0460 shows up only in doctests after a concurrent build, rerun the
  doctests (`cargo test -p <crate> --doc`) before investigating further.

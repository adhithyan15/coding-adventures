---
category: Testing & coverage
---

# A test run that fills the disk can look clean when you only grep for FAILED

A verification run over 157 affected Rust packages ran
`cargo test ... | grep -E "FAILED|error: test failed"` in the background. It
printed nothing, which looked like a pass. In fact the session's disk
allowance had filled to 100% partway through. Test binaries stopped building,
and nothing that ran matched the grep. The `df` at the end of the run showed
177M free.

What to do instead:
- Write the full log to a file and grep that for `No space`,
  `could not compile` and `error[`, as well as `FAILED`.
- Count the `test result: ok` lines, so an empty grep is not read as a pass.
- Check `df` before a large run. In this container the writable allowance is
  about 40G, and a Rust `target/` grows past 24G across many builds.
- For broad runs, set `CARGO_INCREMENTAL=0` and
  `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`. Deleting `target/`
  (all regenerable) freed 25G here.

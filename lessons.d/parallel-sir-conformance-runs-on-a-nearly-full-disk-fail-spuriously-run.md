---
category: Testing & coverage
---

# Parallel sir-conformance runs on a nearly full disk fail spuriously; run them with one test thread

`cargo test -p sir-conformance` was run with about 450 MB free. It reported
real-looking failures in `tests/division.rs`: "go program exited 1",
"rustc failed", and "could not write temp .js". These were not regressions.
Each test compiles and runs dozens of programs through `go run`, `rustc`, `cc`,
`node` and `ruby`, in parallel. Those builds, plus the `target/` dir (about
300 MB without debug info), filled the disk to a few KB. The partly written
`/tmp/sir_conf_*` rustc objects were left behind.

What to do:

- Check `df` before running the suite.
- On a tight disk, build with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`
  and run the toolchain-driving test binaries with `-- --test-threads=1`.
  This turns the peak into a trickle.
- `cargo clean` between suites.
- Before treating a toolchain failure from this harness as a bug, rerun that
  single test in isolation. The disk-full runs all passed alone and
  serially.

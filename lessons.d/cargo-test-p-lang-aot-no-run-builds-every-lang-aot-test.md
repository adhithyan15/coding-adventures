---
category: Testing & coverage
---

# cargo test -p lang-aot --no-run builds every lang-aot test target, about 2.4 GB without debuginfo; build only --test lang_matrix (about 1 GB) on a tight disk

**What went wrong.** Fixing #18483 in a sparse worktree with about 2.5 GB
free (shared with another building agent), I had already run
`cargo test -p lang-aot --test lang_matrix --no-run` (about 1 GB of
`target/`). To also run `op_conformance`, I then ran
`cargo test -p lang-aot --no-run` with `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_INCREMENTAL=0`. That links a separate binary for each of
`lang-aot`'s ~50 test files. `target/` grew to 2.4 GB and the disk dropped
to 36 MB free. The other agent's build could have failed with ENOSPC. I
ran `cargo clean` and rebuilt only `lang_matrix`.

**Fix.** Build and run one test target at a time:
`cargo test -p lang-aot --test lang_matrix`, then `cargo clean -p lang-aot`
(or a full `cargo clean`) before building the next.

**Do differently.** Before `--no-run` on a crate with many integration
tests, count them (`ls tests/*.rs | wc -l`). Each one is a statically
linked binary of the whole dependency graph. Check `df` against that,
not against the size of one binary. On a shared disk, keep 800 MB free.

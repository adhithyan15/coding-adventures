---
category: CI & GitHub Actions
---

# Some ubuntu-latest runners have a 72G disk, so a PR build that passed on a 145G runner can run out of space on the next run

PR #18787 added a DRAT proof checker. Its `build (ubuntu-latest)` job (run
41443486324) failed with `No space left on device` while compiling the
unrelated `lang-aot` crate, followed by an `ld ... Bus error`. The macOS and
Windows legs passed. An earlier PR with the same affected set had passed on
Linux.

The code was not the cause. The runner was smaller:

- That job's root filesystem was `/dev/mapper/cow-root`, 72G (image
  20261018.96.1), with about 20G free after the "Reclaim Linux runner disk"
  step.
- The 25 other Linux jobs sampled ran on `/dev/root`, 145G.
- `lang-aot` was legitimately affected, through `cdcl-sat` →
  `constraint-engine` → `lang-refinement-checker` → `iir-refinement-pass` →
  `twig-aot` → `lang-aot`, plus prerequisite expansion: 189 packages.
- With full debuginfo (`debug = 2`), `lang-aot`'s ~50 statically linked test
  binaries need 20-26G of `target/` on their own.

Sharding and the `ci-full-build` label would not have helped. The shard that
contained `lang-aot` used 24G of `target/` by itself.

**Fix.** The `build` job sets `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.
Panics and backtraces still give file and line, and the debug binaries are
much smaller. Nothing in the repo reads a Rust binary's
own debuginfo: `native-debug-info` and `aot-debug` only write DWARF for
generated code. The PR build step now runs `disk-watch.sh` like main's full
build does. The reclaim step prints a `::warning::` when less than 40G is
free, so a small runner shows up in the run summary.

**Do differently.**

- When a Linux build fails with ENOSPC and the other operating systems pass,
  read `df -h /` from the reclaim step before suspecting the change. A
  `cow-root` device, or about 20G free, means the job got a small runner.
- Plan for the smallest runner GitHub may assign, not the usual one. A
  passing rerun on a large runner does not mean the build fits.
- Before adding a build-output setting such as `debug = 2` back to CI,
  check it against 20G of free disk.

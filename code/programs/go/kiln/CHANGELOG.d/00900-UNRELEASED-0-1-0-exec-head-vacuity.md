## [0.1.0] - UNRELEASED

### Added (KILN04 D48 step 6: a zero that nothing could have changed is not a result)

- **Vacuity counters** in the census `exec_head` block: `go_actions`, `go_built_ok` and
  `tmpfs_execs` (successful execs under the step tmpfs that reached the classifier's
  exec-head pairing, believed or refused). `tmpfs_execs` is counted in `loadChain`'s tmpfs
  branch, the same place the `exechead-*` sub-codes are raised, so it is at least their sum;
  it rides in `census.json` as the additive `exec_head_tmpfs_execs` (read even from an
  otherwise empty census, bounded to 1<<31 per file, summed saturating).
- **Rendering**: a sealed shard with `go_actions == 0` or `tmpfs_execs == 0` opens its Exec
  heads section with "exec heads ran but nothing exercised them; go_emittable is not a
  result"; the three counters are printed; the stdout census line gains `tmpfs_execs=N` for a
  sealed shard.
- **`ci/census_not_clean.py`**: per shard, `tmpfs_execs 0` or `go_actions 0` is a NOTE, never
  a failure (one shard without Go is legitimate). The new
  `--exec-head-run [--expected-shards N] <census.json>...` fails when ALL sampled sealed shards on
  this image have `tmpfs_execs 0` (each matrix leg checks only its own, at most 3), when none ran
  with exec heads, when a summary is unreadable (named), or when fewer than N were read (the
  workflow passes the sampled count). `readCensus` refuses a file whose `exec_head_tmpfs_execs` is
  below its own `exechead-*` occurrences; the counter is diagnostic and a forged value on a clean
  file is undetectable.
  `--strict-exec-head` requires the three new fields to be real ints (missing fails closed).
- **Workflow**: a last step, "Fail when no shard exercised exec heads", makes one
  `--exec-head-run` call over every shard's summary. A final invocation was chosen over an
  aggregate artifact or a per-shard counter file: the summaries are already on the runner in
  `$CENSUS_OUT`, so nothing new is written, uploaded or trusted. Only for `exec_head: true`.

Motivation: census run A sealed exec heads on Rust-only shards (fresh HOME, no cargo), every
action failed in 0.1 s, no step exec'd a binary under the tmpfs, and every criterion passed.
No `RecorderVersion` or step-schema change.

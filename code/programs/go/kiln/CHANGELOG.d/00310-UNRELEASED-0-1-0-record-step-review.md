## [0.1.0] - UNRELEASED

### Fixed

- M1: supervisors of steps that start together (the build tool defaults to
  `-jobs NumCPU`) raced to create `traces/`: one `mkdir` won and the others got
  `EEXIST` and exited 125, voiding the shard (895 of 1200 in the review's proof of
  concept). `prepare` now creates `traces/` (0700, outside the sealed `pre/`), and
  `ensureDir` tries `mkdir` first and on `EEXIST` checks the directory (real,
  owned by this user, no group or other bits); the per-action directory the same.
- L1: a rerun or out-of-order refusal did not always leave `<step>.rerun`
  (`ReadDir` sorts `10.json` before `2.json`, and an early return skipped the
  marker; two supervisors racing on one step left none). `checkOrder` examines
  every entry, compares step numbers as numbers, and the trace file's `O_EXCL`
  failure leaves the marker too.
- L2: the sandbox configuration was verified by content but used by path.
  `kiln sandbox-exec` takes an optional `--config-sha256 <hex>` and refuses a
  configuration of other content; `step` always passes the binding's digest.
- L3: disk exhaustion was found only through strace's stderr and a 250 ms poll
  could miss a fast `fallocate` and unlink. strace now writes the trace to a pipe
  (`-o /proc/self/fd/5`) that the supervisor reads and writes to the file, hashing
  as it goes: an exact cap (`trace-too-large`), a failed write
  (`trace-write-failed`: kill strace, keep draining), no polling, and
  `trace_sha256` from the streamed hash. Only the free-space floor before a step
  starts remains a measurement; it is now a field.
- L4: an orphaned supervisor could outlive the build tool and keep writing while
  `collect` read. `step` holds a shared `flock` on the record directory for its
  whole run (`record.LockDir`), and asks the kernel to kill it when the build tool
  dies (`PR_SET_PDEATHSIG`; Linux).
- Security LOW-1: a survivor holding strace's own stdout or stderr was noticed only
  when strace exited 0. All five pipes (the step's two, the trace, strace's two)
  are now files drained with one timeout, so `pipesHeld` is independent of the exit
  status.
- The validated `LD_LIBRARY_PATH` of `build_tool_env` reaches strace.
- strace's own stderr, when not empty, is saved as `<step>.strace-stderr`.
- `VerifyPre` cross-checks every action file against the sealed `steps.json`.

### Specification

- KILN04 4.8 P12 and 4.3, 4.7 amended for all of the above; `env -i` and
  `::stop-commands::` requirements for the workflow; four new 2b-2b obligations of
  `collect` (exclusive non-blocking `flock`; newline and re-hash of every trace;
  the init's `--sha256` and the root's `--config-sha256` against the binding;
  `.rerun`, `trace-write-failed` and `strace-pipes-held-open` void the shard).

### Tests

- `TestParallelFirstStepsDoNotRaceOnTheTracesDirectory` (the proof of concept as a
  regression test), `TestParallelRunStepsOfDifferentActions`, a Linux test across
  real processes, `TestRerunWithoutMarker`, `TestOutOfOrderLeavesTheMarkerToo`,
  `TestRacingSupervisorsOfOneStep`, locks, an exact cap, an injected `ENOSPC`, a
  survivor holding strace's pipes while strace fails, `LD_LIBRARY_PATH`, the saved
  stderr, the steps cross-check, `--config-sha256`, and `PR_SET_PDEATHSIG`.

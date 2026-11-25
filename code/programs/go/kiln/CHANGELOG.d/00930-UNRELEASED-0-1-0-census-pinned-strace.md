## [0.1.0] - UNRELEASED

### Added (census workflow: a pinned strace 6.19 experiment)

- `.github/workflows/kiln-census.yml` gains a `workflow_dispatch` input `strace`
  (`system`, the default, or `6.19`), validated like the other inputs. Scheduled runs are always
  `system`.
- `6.19` builds strace from the official release tarball (SHA-256 pinned in the workflow, the
  upstream signature checked when the pin was recorded), installs it under `$RUNNER_TEMP` before the
  job start is stamped, fails unless `strace -V` says 6.19, and passes it to every
  `kiln record prepare` as `--strace`. No Go code changes: `prepare` already takes `--strace`.
- Every shard's summary states the strace used (version and path) and, for the pinned run, says it is
  NOT PR8 evidence. Purpose: test whether strace 6.19 prints a thread killed at its syscall-entry stop
  in the "never ran" form the parser accepts, which would explain the `unresumed-call` refusals seen
  on ubuntu-24.04 (strace 6.8) and not on ubuntu-26.04 (strace 6.19).
- KILN04 spec: a note after D50 describes the experiment input.

## [0.1.0] - UNRELEASED

### Added (stage 4 PR 2b-2c-4)

- `survivor` run-void: `collect` scans `/proc` right after the exclusive lock and
  `VerifyPre` (S1 another mount namespace that mounts the checkout, S2 the sealed
  `pre/sandbox.json` in a command line, S3 an unreadable process of our user;
  no cwd test), waits up to 10 s (200 ms poll) and, if a survivor remains, adds
  `survivor` to the header `void` and marks every record unknown. Hooks
  `CollectOptions.Survivors`, `SurvivorWait`, `Now`, `Sleep`; Linux file plus a
  fail-closed stub elsewhere.
- `state-preloaded` shard-void: `RunStep` at sequence number 0 re-checks the state
  roots and leaves `traces/.state-preloaded` (O_EXCL, the path); `collect` adds
  the token and accepts the marker in `globalSeqCheck`.
- Tests: a fake-`/proc` suite for S1-S3, the wait, both tokens through `collect`,
  the marker only at seq 0; Linux e2e `TestSandboxRecordSurvivor` (required).
- Spec KILN04: M3/M4 say who writes each token; new decision P15.

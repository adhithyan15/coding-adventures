## [0.1.0] - UNRELEASED

### Fixed

- Defence in depth: `kiln sandbox-exec` marks every descriptor of 3 and up
  close-on-exec before it spawns the init (fail closed), so the init never holds a
  descriptor the caller left open, the recorder's trace pipe included.
- The drain after strace exits no longer blocks on the relays that write to the
  caller's stdout and stderr: the trace sink and strace's own streams are always
  waited for (the sidecar reads them), the two relays get a one-second grace after
  the drain timeout and are then abandoned.
- `step` takes the shared `flock` before `VerifyPre` (and before reading anything
  of the record directory).
- `kiln record step` pins itself to one OS thread before it asks for
  `PR_SET_PDEATHSIG`; the comment on thread retirement no longer overstates it.

### Specification

- KILN04 P12: the arming depends on the build tool's forking thread staying alive
  (a misfire fails closed; the `flock` protects `collect` on its own);
  `<step>.strace-stderr` is untrusted diagnostic text that `collect` treats as
  opaque and does not upload; the drain rule; the lock is taken first. 4.7: the
  root's close-on-exec marking.

### Tests

- `TestSandboxRecordStep` gains two steps: `ls /proc/self/fd` shows only 0 to 2 (and
  ls's own), and a write to descriptor 5 is refused and absent from every trace.
- `TestStepAnUnreadableCallerDoesNotHoldTheSupervisor`.

## [0.1.0] - UNRELEASED

### Changed (KILN04 D47, `kiln:recorder` 14 to 15)

- `dyingPolicy` now drops a dying call whose name is exactly strace's spelling of
  an unnameable number (`syscall_0x...`, `isUnknownNumberName`) and records no
  event, in both trace readers, when its number N is (a) above the sandbox
  filter's ceiling and at most 32 bits, where the filter answers ENOSYS, or
  (b) in the closed path-free set `{456}` (`futex_requeue`, cited from the kernel's
  `syscall_64.tbl`). Every other unknown number stays `unresumed-call`. It is
  not a spawn, so it accounts for no orphan pid.
- The ceiling moved into a new leaf package, `internal/syscallceil`, which both
  `sandbox.BuildFilter` (`MaxKnownSyscall`) and the trace rule read; a sandbox
  test pins the filter's boundary to it.
- `RecorderVersion` 14 to 15: the
  accept set grows, so every weak key changes.

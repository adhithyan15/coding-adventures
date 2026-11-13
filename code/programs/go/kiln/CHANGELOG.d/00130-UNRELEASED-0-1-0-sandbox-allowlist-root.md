## [0.1.0] - UNRELEASED

### Fixed

Second adversarial review of `kiln sandbox-exec` (KILN04 sections 4.3, 4.7, 13
and 14):

- **The new root is an allowlist** (`ro_roots`), not a copy of `/` with masks.
  A pathname Unix socket or FIFO belongs to the filesystem, so a read-only bind
  did not stop `connect()` (the runner's .NET diagnostic socket in `/tmp` makes
  `Runner.Worker` load a library). The root is now an empty tmpfs holding only
  the configured read-only roots (symlinks such as `/bin` are recreated), the
  checkout writable at its own path, the state roots, fresh tmpfs for `$HOME`,
  `TMPDIR`, `/tmp`, `/var/tmp` and `/run`, the minimal `/dev` and `/proc`, sealed
  read-only before the pivot. Missing parents are created in the tmpfs. The
  configuration gains `ro_roots`, refuses overlaps with the checkout, `home`,
  `tmpdir` and the state roots, and refuses an empty list.
- **Masks** are only the checkout's direct children; a mask anywhere else is
  refused with a clear message (it is already absent, and used to exit 125). The
  README example is fixed and now has a test that validates it.
- `RLIMIT_CORE` is 1, not 0: a limit of 0 does not stop a piped `core_pattern`
  handler, the special value 1 does.
- CI: the assertion runs in an `if: always()` step so the failing output is
  printed; the job runs on the scheduled and dispatched full builds too (detect
  forces every gate there); a probe before recording is required of stage 4.

### Added

- `TestSandboxNoReachableSocketsOrFifos`: walks every mount of a running
  sandbox for sockets and FIFOs, and checks that ones made in the host's `/tmp`
  and `/var/tmp` are invisible. A test of the core-file limit.

### Changed

- Supersedes the 00120 entries on masking `/run` and `/var/run` and on
  `RLIMIT_CORE` 0.

## [0.1.0] - UNRELEASED

### Added (KILN04 D50: two parse shapes that were refused and are provably safe; `kiln:recorder` 19 to 21)

- **Rule 2b: a `fork`, `vfork` or `clone` that ended `= ? ERESTARTNOINTR`** (whole-line or split) is
  dropped: no event, no spawn record, not a dropped spawn, and NOT paired with the later call (the
  kernel restarts the call itself; the restart is a fresh line). The result must be `?` with the
  errno name exactly `ERESTARTNOINTR`; a clone's flags must pass the existing flag check and be
  names strace prints. Still `result`: any other `ERESTART*` name, `ERESTARTNOINTR` on a non-spawn
  call, `clone3`, a bare `= ?`. Shared model (`internal/trace/model.go`, `restartedSpawn`), so both
  readers get it.
- **Rule 2c: a successful read-only `open`/`openat` with no descriptor annotation** is accepted with
  an empty `ReturnFDPath` (`bareReadOnlyOpen`): plain decimal descriptor, quoted path, flags inside
  the closed set `O_RDONLY O_CLOEXEC O_NONBLOCK O_NDELAY O_DIRECTORY O_NOFOLLOW O_LARGEFILE O_NOCTTY
  O_NOATIME O_PATH` with no numeric residue, and `O_RDONLY` or `O_PATH` present. The classifier is
  unchanged: `openCall` already walks the name under the strict stability rule when the return
  path is not absolute (pinned by `internal/traced/bareopen_test.go`). Still `fd-path`: any
  write-capable, creating, truncating or `O_TMPFILE` open, `openat2`, `creat`, a present but
  malformed annotation, and later un-annotated calls on the descriptor.
- **Census**: `Trace.RestartedSpawns` / `Trace.BareOpens` count the firings; they ride in each
  action's census file as `parse_restarted_spawns` / `parse_bare_opens` (additive, bounded to 1<<31),
  sum into the shard summary's `parse_rules`, a "Parse rules" markdown section and the stdout census
  line (`restarted_spawns=N bare_opens=N`).
- **Tests**: shape fixtures with neighbour refusals for both rules (`internal/trace/d50_test.go`),
  mutants that must be killed (`d50_mutant_test.go`: errno check dropped, a write flag added to the
  closed set, a hex path accepted), a biased twin differential over strace text
  (`d50_biased_test.go`), classifier tests and a biased read-set-oracle differential with floors
  (`internal/traced/bareopen_*_test.go`), census tests, and the real-kernel
  `TestSandboxStraceRestartedSpawn`, now REQUIRED in `ci/assert_sandbox_tests.py`.
- **CI**: the kiln-sandbox job repeats `TestSandboxStraceRestartedSpawn`, `TestSandboxStraceSignals`
  and `TestSandboxStraceDyingThreads` with `-count=20` (a refusal rule that passed once caused the
  D46 flake); the cross-check must stay at zero mismatches and zero allowed divergences.

### Changed

- `legacy.RecorderVersion` 19 to 21 (20 belongs to the open PR that took it; whichever lands second
  renumbers). Every weak key changes: the accept set grows.
- `TestStringErrors` now uses an `O_RDWR` open for its un-annotated refusal case (a read-only one is
  accepted by rule 2c).

### Observed on real kernels (PR CI)

- Rule 2b fires on strace 6.8 (ubuntu-24.04), about 130 times per restart-test run, and never
  on strace 6.19 (ubuntu-26.04); the repeat step warns rather than fails when it does not fire.
- strace 6.8 prints a thread killed mid-call as one line, `<unfinished ...>) = ?`; the parser
  refuses it (`unparsable-line`) and `TestSandboxStraceDyingThreads` now recognises it as a dying shape.

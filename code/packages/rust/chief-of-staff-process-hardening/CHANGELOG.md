# Changelog

## Unreleased

### Added

- `suppress_core_dumps()`, for the Chief daemon's `main` (D18S S-I5; #13980
  P2.6a). It applies these measures:
  - every Unix: `RLIMIT_CORE` set to zero, soft and hard, so nothing later
    can raise it;
  - Linux: `prctl(PR_SET_DUMPABLE, 0)`. This also makes `/proc/<pid>`
    root-owned, and refuses ptrace from any process of the same user;
  - macOS: `ptrace(PT_DENY_ATTACH)`.

  Each measure is read back after it is set, and a mismatch is an error.
  The returned `CoreDumpProtection` lists what was applied, and what this
  platform still lacks: the BSDs' attach denial, and Windows' process DACL
  (step 8).
- The `process-hardening-probe` test child, and tests that read the result
  from outside the hardened process:
  - the zero limit;
  - non-dumpable, against a dumpable control;
  - root-owned `/proc/<pid>/mem`, against a control when not run as root;
  - the process can still list its own fds;
  - an exec'd child is dumpable again but keeps the zero limit.

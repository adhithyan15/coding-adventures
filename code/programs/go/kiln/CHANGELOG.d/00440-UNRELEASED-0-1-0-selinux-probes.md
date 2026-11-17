## [0.1.0] - UNRELEASED

### Added

- KILN04 decision D27: `code/kiln/legacy/host-roots.json` allows the libselinux
  start-up probes of ubuntu-26.04 (coreutils, grep, sed). `/proc/filesystems`
  is `ephemeral`; `/selinux` and `/sys/fs/selinux` are `absent-ok`, allowed
  only while absent, so an SELinux-enabled host still records nothing (a present
  `/sys/fs/selinux` is a `host-read`, and `/proc/mounts` and
  `/proc/self/attr/current` stay `trace-incomplete:proc-path`). No
  `/sys/fs/selinux/**` entry exists. `kiln:recorder` is not bumped; the policy
  digest carries the change.
- `internal/traced` tests load the real `host-roots.json` and pin these
  behaviours and the list memberships.
- CI: the `kiln-sandbox` job is a matrix over `ubuntu-24.04` and
  `ubuntu-26.04` (job id unchanged, so `ci-gate` still aggregates it).
- KILN04: the record workflow and its consumers use one explicit
  `ubuntu-NN.04` label, never `ubuntu-latest`; `/proc/filesystems` is a named
  residual risk (section 14).
- Absent-ok child probes (D27 review): a failed probe of a child of an `absent-ok` entry (for example `/sys/fs/selinux/enforce`) is allowed only while the entry itself is absent (`lstat` gives ENOENT or ENOTDIR); otherwise `host-read`. Entries under the fresh `$HOME` are unaffected. `kiln-sandbox` is also selected by changes under `code/kiln/legacy/**`.

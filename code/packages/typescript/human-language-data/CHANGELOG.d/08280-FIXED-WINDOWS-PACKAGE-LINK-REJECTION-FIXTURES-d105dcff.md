### Fixed — Windows package link-rejection fixtures

- Exercise linked owner directories with junctions on Windows, preserving
  real-link rejection without requiring Developer Mode.
- Isolate file-symlink cases from other negative assertions. An unprivileged
  Windows account skips only those cases; Linux CI still requires genuine
  file-symlink rejection.
- Keep the production loader and write guards unchanged.

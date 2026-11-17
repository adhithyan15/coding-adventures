## [0.1.0] - UNRELEASED

### Fixed (tests, D29)

- Flaky `TestStepRefusesAResealedPreWhoseContentIsWrong` (`lstat
  .../.git/objects/maintenance.lock: no such file or directory`, ubuntu-26.04).
  Newer git runs `git maintenance run --auto` (and auto-gc) after a commit,
  creating and deleting a lock file in `.git/objects` while the tests copy and
  walk the template repository. Every git the `internal/record` test helpers
  (and the sandbox record step helper) start now runs with `maintenance.auto=false`,
  `gc.auto=0`, `gc.autoDetach=false` (via `GIT_CONFIG_COUNT`), the template
  repository persists the same config, and `copyTree` skips a `.git/**/*.lock`
  that vanishes mid-walk. Test-only; no production code changed.

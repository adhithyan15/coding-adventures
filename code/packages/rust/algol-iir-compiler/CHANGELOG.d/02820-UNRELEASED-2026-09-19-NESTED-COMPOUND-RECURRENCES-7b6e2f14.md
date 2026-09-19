## Unreleased - 2026-09-19 - Nested compound recurrence siblings

- Flatten unlabeled nested compound statements while proving bounded finite-step,
  while, and controlled-scalar recurrences, so nested grouping of the already
  supported recurrence, scalar-identity, and dummy statements retains exact
  snapshots. Labels and every existing effectful or dynamic barrier still fail
  closed.

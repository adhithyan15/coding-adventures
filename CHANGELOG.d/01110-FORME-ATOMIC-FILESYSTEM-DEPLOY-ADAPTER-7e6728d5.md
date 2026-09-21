### Forme atomic filesystem deploy adapter

- Added `forme-deploy-runner-fs-adapter` with complete-tree sibling staging,
  a same-parent backup, explicit commit/finalize/rollback transactions, and
  write-free detection of already exact targets.
- Added adversarial containment tests for linked roots and descendants,
  multiply linked files, cancellation cleanup, exclusive publication locks,
  filesystem-object substitution, stale-output pruning, retry-safe rollback,
  bounded streaming scans, nested-entry races, final-read cancellation,
  irreversible cleanup failure, rootless rollback, and primary-error
  preservation.

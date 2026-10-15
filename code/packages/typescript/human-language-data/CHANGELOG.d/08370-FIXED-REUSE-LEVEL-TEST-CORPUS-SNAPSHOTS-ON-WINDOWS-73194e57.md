### Fixed — reuse level-test corpus snapshots on Windows

- Reuse one freshly loaded, read-only real-corpus snapshot per level-test file
  instead of rebuilding every curriculum owner and lesson inside each timed
  assertion. The level-gate and levels checks retain their full-corpus
  assertions and unchanged timeout budgets.

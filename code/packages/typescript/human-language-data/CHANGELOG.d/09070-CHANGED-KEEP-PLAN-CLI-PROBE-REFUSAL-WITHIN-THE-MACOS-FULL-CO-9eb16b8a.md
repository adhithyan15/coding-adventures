### Changed — keep plan-CLI probe refusal within the macOS full-corpus budget

- Run the missing-probe negative control against a private inventory-only copy
  of the committed registry and French A1 inventory. The planner still parses
  the real corrupted inventory and must return a nonzero exit with a named
  French A1 diagnostic; an unrelated failure cannot satisfy the assertion.
- Avoid a redundant all-lesson parse in this schema-refusal case. The separate
  clean-corpus and pruned-versus-committed plan tests still exercise the full
  lesson path, and the private fixture is restored and removed after the case.
- Keep the 120-second case budget, package worker and heap settings, complete
  corpus coverage run, and 85% line-coverage threshold unchanged. The focused
  six-test plan-CLI file passed locally; an exact-final-source macOS full-suite
  result remains required for #18852 and #18947.

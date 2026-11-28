# Changelog

## Unreleased

- Export `ValidateGeometry` for whole-image admission and make `Qualify` use
  the shared geometry and erase-ceiling check before target-specific checks.
- Add a pure interrupted-erase role qualifier that matches the retained intent
  sequence, physical block and generation before returning detached Free/Stale
  roles. Bound geometry and erase budgets, validate markers, and test
  indistinguishable raw observations against an independent model.

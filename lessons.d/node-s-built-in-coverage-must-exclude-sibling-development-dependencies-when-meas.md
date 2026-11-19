---
category: Testing & coverage
---

# Node's built-in coverage must exclude sibling development dependencies when measuring a leaf package

The first `node --experimental-test-coverage` run reported 40.59% because Node included the local
`test-bootstrap` development dependency in the denominator, even though the game-clock package
sources were already above 98%. The command therefore measured a workspace dependency graph as if
it were one package. For a leaf-package threshold, exclude sibling packages and test files, then
set the threshold against the package's own compiled source; keep tests running normally. Verify the
report lists only the intended package before interpreting a coverage failure as missing tests.

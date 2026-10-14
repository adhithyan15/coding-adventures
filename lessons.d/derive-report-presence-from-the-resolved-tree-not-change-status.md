---
category: Testing & coverage
---

# Derive report presence from the resolved tree, not change status

The hot-file report inferred current presence from each path's newest counted
status. A path touched early in the window and later renamed never received a
destination-oriented delete row, so the absent source was reported as present.
Historical status streams answer how a path changed, not whether it exists at
the measured revision. Build a strict NUL-delimited membership set from the
resolved tree and test an add-then-rename sequence across multiple commits.

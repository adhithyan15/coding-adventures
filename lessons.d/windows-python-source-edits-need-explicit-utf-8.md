---
category: Cross-platform & Windows BUILD_windows
---

# Windows Python source edits need explicit UTF-8

A conflict-resolution script used pathlib default encoding on a Unicode changelog
and failed under Windows cp1252 before editing anything. Use explicit UTF-8 for
repository text and verify conflict markers are gone before running checks.
When upstream changes diagnostic reach, recompute combined counts rather than
keeping the original branch baseline; both-theme analysis found 77, not 76.

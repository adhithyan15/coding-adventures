---
category: Cross-platform & Windows BUILD_windows
---

# Validate Windows output spelling before absolute-path normalization can erase ambiguity

The publication test for out. unexpectedly succeeded even though filename validation rejected trailing dots. Windows absolute-path conversion had already erased the spelling before validation. Validate original normal components first, including missing parents, and reject a missing component followed by parent traversal before normalization can collapse it. Keep normalized path and filesystem-identity checks as separate later guards.

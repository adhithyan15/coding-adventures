---
category: Testing & coverage
---

# Discover package test files before assuming conformance filenames

The C frontend has `tests/three_way_conformance.rs`, not the guessed
`tests/conformance.rs`. A read failed because a conceptual test category was
treated as a filename. Use `rg --files` in the package test directory first;
then read the actual harness and its compiler/environment requirements.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.

---
category: BUILD files & dependency management
---

# Test-only sibling deps still need to be declared

If a TEST file imports a sibling for `isinstance()` checks, install the sibling in BUILD AND declare it in `pyproject.toml` dev extras — otherwise the validator's prerequisite check fails. Every package referenced by `-e ../pkg` in a BUILD must be directly declared in that package's metadata; declaring the *parent* dep is not transitively sufficient.

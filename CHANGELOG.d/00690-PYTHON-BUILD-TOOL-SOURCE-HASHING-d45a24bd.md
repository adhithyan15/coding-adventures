### Python build-tool source hashing

- Excluded the complete exact, case-sensitive generated-directory registry
  from both Python source-collection modes before extension or declared-source
  matching while preserving case variants and near names.
- Enforced the no-follow boundary for directory and file symlinks plus Windows
  junction/reparse attributes, with direct extension and Starlark regressions
  for all 26 excluded components and a real NTFS-junction check.


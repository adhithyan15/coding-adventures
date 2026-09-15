### Rust build-tool generated-directory hashing exclusion

- Excluded the complete exact, case-sensitive 26-component generated,
  dependency, VCS, cache, and temporary-directory registry from both
  extension and declared-source hashing before file selection.
- Projected both language-neutral source-collection fixtures into native Rust
  tests while preserving case variants and near names, and kept POSIX links
  plus Windows junction/reparse points outside both recursive collectors.


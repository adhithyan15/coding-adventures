### Ruby build-tool generated-directory hashing exclusion

- Excluded the complete exact, case-sensitive 26-component generated,
  dependency, VCS, cache, and temporary-directory registry from both
  extension and declared-source hashing before file selection.
- Projected both language-neutral source-collection fixtures into native Ruby
  tests while preserving case variants, near names, discovery-only `specs`
  directories, and the existing no-follow boundary for directory links.


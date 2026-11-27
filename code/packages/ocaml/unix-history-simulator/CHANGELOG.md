# Changelog

## Unreleased

- Check directory traversal before processing dot components, so `/file/.`
  returns `ENOTDIR`.
- Add pure in-memory filesystem, descriptor-table, and kernel components for
  the HOS00 historical Unix teaching model. Fixture loading stays in the
  shared test-adapter tree.

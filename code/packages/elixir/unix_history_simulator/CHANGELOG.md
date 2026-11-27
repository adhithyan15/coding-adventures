# Changelog

## 0.2.0

- Keep the reusable model library-only and move JSON argv/stdout handling to an
  external fixture adapter.
- Keep component tests independent of repository fixture files and declare an
  empty capability profile for the in-memory library.

## Unreleased

- Keep the fixture adapter external and the package library-only.

## 0.1.0

- Add independently reusable in-memory filesystem, descriptor table, kernel operations, shell adapter, and component tests.
- Replay all shared historical Unix traces from both platform build fronts.

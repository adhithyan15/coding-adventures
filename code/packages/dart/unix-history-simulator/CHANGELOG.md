# Changelog

## 0.2.0

- Keep the package runtime library-only and move the shared fixture replay to
  an external adapter that imports the reusable API.
- Keep component checks independent of repository fixture files and declare an
  empty capability profile for the in-memory library.

## 0.1.0

- Add reusable in-memory filesystem, descriptor table, kernel, JSON CLI, and
  tests for HOS00 shared traces.

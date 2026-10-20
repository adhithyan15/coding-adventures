# Changelog

## Unreleased

### Fixed

- `writeClassFile` now works on Windows. The embedded Python writer used to
  call `os.open()` on directories with `dir_fd=`, which Windows rejects with
  `PermissionError: [Errno 13]`, so every write failed there. It now uses the
  race-free directory-FD walk only when `os.supports_dir_fd` covers `open`
  and `mkdir`. Otherwise it falls back to per-component symlink and junction
  checks plus a temp file and an atomic `os.replace()`, matching the Python
  `ir-to-jvm-class-file` package.
- The relative class-file path passed to the writer is now always joined
  with `/`. Before, `node:path` `join` produced `demo\Example.class` on
  Windows, which the writer's `PurePosixPath` read as one component, so the
  per-directory symlink checks were skipped.
- Reject `classFilename` components containing `\`, `:` or NUL, which
  Windows would treat as separators, drive or stream markers, or truncation
  points.

### Added

- Internal `writeClassFileWithStrategy` test seam. It is not exported from
  the package entry point. The filesystem tests now run under both `auto`
  and the Windows `path-checks` strategy, so Linux and macOS CI cover the
  Windows code path. New cases cover symlinked output files, directories
  in the way of the target, missing nested roots and overwrites.

## 0.1.0

- Add the first TypeScript generic `compiler-ir` to JVM class-file backend.
- Include a classpath-aware write helper and malformed-output path validation.
- Remove accidentally committed transpiled JavaScript, declaration, and source
  map outputs so the package only tracks its TypeScript sources.

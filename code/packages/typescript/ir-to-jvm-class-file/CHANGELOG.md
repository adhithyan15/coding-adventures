# Changelog

## Unreleased

### Fixed

- `writeClassFile` works on Windows. The secure writer walks the output
  directories through directory file descriptors (`os.open` + `dir_fd`), which
  Windows Python does not support (`PermissionError` opening the root). Where
  `os.supports_dir_fd` lacks `open`/`mkdir`, it now uses the Python port's
  fallback: the same existing-ancestor check, a symlink check on every
  component, then a sibling temp file `os.replace()`d into place.

## 0.1.0

- Add the first TypeScript generic `compiler-ir` to JVM class-file backend.
- Include a classpath-aware write helper and malformed-output path validation.
- Remove accidentally committed transpiled JavaScript, declaration, and source
  map outputs so the package only tracks its TypeScript sources.

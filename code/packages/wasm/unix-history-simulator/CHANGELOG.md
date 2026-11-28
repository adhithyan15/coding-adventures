# Changelog

## Unreleased

- Compile the WAT guest with the repository's locked, local Rust parser and
  encoder crates instead of an unapproved npm WABT dependency.
- Use `wasm-output` on both platform fronts because `build` aliases the
  tracked `BUILD` file on case-insensitive filesystems such as macOS.

## 1.0.0

- Add a host-import-free WAT guest for the bounded HOS00 file-read image.
- Bound guest path reads and descriptor output writes to disjoint one-page
  scratch regions, with atomic rejection checks.
- Add external component and shared-fixture replay checks for guest state and
  the host adapter boundary.

# Changelog

## Unreleased

- Use a case-distinct Windows output directory so it does not alias the
  tracked `BUILD` file on case-insensitive filesystems.

## 1.0.0

- Add a host-import-free WAT guest for the bounded HOS00 file-read image.
- Bound guest path reads and descriptor output writes to disjoint one-page
  scratch regions, with atomic rejection checks.
- Add external component and shared-fixture replay checks for guest state and
  the host adapter boundary.

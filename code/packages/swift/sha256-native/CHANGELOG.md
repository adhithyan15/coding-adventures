# Changelog — Sha256Native (Swift)

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust SHA-256 for Swift — the first Swift
  `*-native` package in the campaign, establishing the "Rust staticlib + C ABI
  + SPM systemLibrary" Swift native pattern.
- Links the `sha256-c` static library (over `coding_adventures_sha256`) and
  calls its C ABI via the `CSha256` module: `digest` (caller-owned 32-byte
  buffer) and an opaque streaming `Hasher` (`update` / non-destructive `digest`
  / `hexDigest` / `copy`, freed in `deinit`).
- 9 XCTest cases: FIPS 180-4 vectors (incl. one-million-'a'), block boundaries,
  streaming parity, byte-at-a-time, non-destructive digest, and copy
  independence. `swift test` green against the linked Rust library.

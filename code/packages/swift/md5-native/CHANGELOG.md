# Changelog — Md5Native (Swift)

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust MD5 for Swift — reuses the Swift native
  pattern (Rust staticlib + C ABI + SPM systemLibrary) established by
  `swift/sha256-native`, specialised to MD5's 16-byte digest.
- Links `md5-c` (over `coding_adventures_md5`) via the `CMd5` module: `digest`
  (caller-owned 16-byte buffer) and an opaque streaming `Hasher` (`update` /
  non-destructive `digest` / `hexDigest` / `copy`, freed in `deinit`).
- 8 XCTest cases: RFC 1321 vectors (incl. the 0x00..0xFF known digest), digest
  size, streaming parity, byte-at-a-time, non-destructive digest, copy
  independence. `swift test` green against the linked Rust library.

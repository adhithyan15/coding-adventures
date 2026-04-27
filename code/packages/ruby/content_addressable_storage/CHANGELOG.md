# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-12

### Added

- `BlobStore` module — abstract interface for pluggable storage backends
  with four required methods: `put`, `get`, `exists?`, `keys_with_prefix`
- `ContentAddressableStore` — wraps any `BlobStore`, adds automatic SHA-1
  keying, integrity verification on every read, and hex prefix resolution
- `LocalDiskStore` — filesystem backend using the Git 2/38 fanout layout;
  atomic writes via PID+nanosecond-timestamp temp file + `File.rename`
- `CasError` exception hierarchy:
  - `CasNotFoundError` — key absent from store; carries the 20-byte key
  - `CasCorruptedError` — stored bytes don't hash to the key; integrity violation
  - `CasAmbiguousPrefixError` — hex prefix matches ≥2 objects
  - `CasPrefixNotFoundError` — hex prefix matches 0 objects
  - `CasInvalidPrefixError` — hex prefix is empty or contains non-hex characters
- Hex utility functions: `key_to_hex`, `hex_to_key`, `decode_hex_prefix`
- Minitest test suite with >95% coverage across all classes and edge cases
- Literate programming style: every design decision explained inline

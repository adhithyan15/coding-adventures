# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-09

### Added

- Initial implementation of LZ77 encoding/decoding (CMP00 specification)
- Streaming API: `encode()` and `decode()` for token stream manipulation
- One-shot API: `compress()` and `decompress()` for byte I/O
- Fixed-width 4-byte token serialisation format (teaching format, not optimised)
- Comprehensive test suite with 95%+ coverage including:
  - Specification test vectors from CMP00 spec
  - Round-trip invariant tests
  - Parameter constraint tests
  - Edge cases (boundaries, overlapping matches, binary data)
  - Serialisation/deserialisation tests
- Module-level docstring with Knuth-style literate programming
- Full type annotations (PEP 257 compliant)
- README with usage examples and API reference
- pyproject.toml with pytest, coverage, ruff, and mypy configuration

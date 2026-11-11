# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Security

- Hardening (CWE-789): _deserialise_tokens clamps the untrusted header token count to the number of complete tokens present instead of looping over the claimed count. Not previously exploitable (the loop broke at the first missing token) but no longer depends on that.
- Added test_hostile_headers.py: hostile headers (0xFFFFFFFF counts/lengths, counts beyond the data) checked for bounded time and memory, plus round trips over every byte string of length <= 2 and a seeded random sample.
- `decode()` now stops at the first back-reference whose offset is `< 1` or `> len(output)`, returning the output produced so far. Previously a negative offset silently copied bytes from the end of the buffer (Python negative indexing) and an oversized offset raised `IndexError`. This matches the other language lanes. Regression tests for offset 0, negative offset, offset beyond output, the exact boundary (offset == len(output), still valid), and `decompress()` on a hostile stream were added to test_hostile_headers.py.

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

# Changelog

## [Unreleased]

### Security

- No code change: the stream is strictly validated (token count must match the length exactly, declared length bounded by the max-output limit, dangling indexes rejected).
- Added HostileHeadersTest (JUnit 5 via kotlin.test): hostile headers under a preemptive timeout, plus round trips over every byte string of length <= 2 and a seeded random sample.

## 0.1.0 - 2026-08-26

- Add LZ78 tokenization, exact serialization, and strict decompression.
- Cover published vectors, binary inputs, bounded dictionaries, and malformed streams.

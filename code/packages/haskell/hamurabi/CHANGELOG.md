# Changelog

All notable changes to the `coding-adventures-hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `coding-adventures-hamurabi`: pure deterministic simulation core for the seminal 1968 Hamurabi game (GAME01).
- Strict record implementations for `HamurabiState` and `TurnOrder` preventing space leaks (CWE-400).
- Pure PCG32 XSH RR 64/32 implementation matching the canonical 1968 Sumerian economy loop.
- Full 8-variant `HamurabiError` refusal hierarchy.
- Authentic 1970s teletype report generator (`formatReport`).
- Zero third-party dependency conformance test suite parsing `code/specs/fixtures/hamurabi-v1/cases.txt`.

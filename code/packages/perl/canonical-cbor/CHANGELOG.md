# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added
- RFC 8949 deterministic canonical CBOR codec implementing §4.2.3 Length-First Map Key Ordering (CBR01 profile).
- Pure Perl implementation with zero CPAN runtime dependencies (`MIN_PERL_VERSION => '5.028000'`).
- Full 55/55 test vector conformance from shared `cases.json` specification corpus.
- Centralized `Messages.pm` containing all 14 closed payload-blind error diagnostics and `<redacted>` security placeholder (Standing Rule #20).
- Centralized `Constants.pm` declaring zero-magic-constant protocol parameters and wire byte markers.
- Inside-out unforgeable algebraic value models (`Types.pm`) with lexical registry encapsulation, tie/bless TOCTOU defenses, and strict UTF-8 validation.
- Atomic `encode_into_checked` buffer chunk appending with rollback on encoding errors.
- Spanned map entry decoding (`decode_map_spans`) with 0-based offset spans `[start, finish)`.
- 7-class Perl CVE Resilience Showcase defending against 64-bit integer wraparound / NV float precision traps, pre-allocation OOM bombs, recursion exhaustion, non-canonical map ordering, UTF-8 smuggling, trailing-byte smuggling, and secret reflection leakage.
- Deterministic PRNG round-trip and map-permutation property tests, plus 10,000-iteration byte-mutation fuzz loop.
- Built-in zero-dependency line-coverage harness asserting >= 95% line coverage via `t/lib/Devel/LineCov.pm` and `t/99-coverage.t`.

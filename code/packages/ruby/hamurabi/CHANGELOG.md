# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Pure, deterministic simulation core for Hamurabi (`GAME01`).
- Exact PCG32 pseudorandom number generator with Knuth LCG multiplier and stream id 54 (`rng_inc = 109`).
- Canonical two-phase state transition: validation and deterministic PRNG update.
- Immutable `HamurabiState` record with `.freeze` enforcing `VER04m` fresh value semantics.
- Closed refusal hierarchy with all 8 canonical error arms (`HamurabiError`).
- Full teletype annual report and terminal summary formatter (`format_report`).
- Strict Ruby CWE hardening (type validation, explicit bounds, rejection of floats/strings/booleans, zero eval/Marshal).
- Conformance test suite running 31 test vectors from `cases.json` with 100% bit-exact parity.
- Adversarial CWE test suite covering integer overflow, state invariant violations, and type confusion.
- Empty capability certificate (`capabilities: []`) guaranteeing zero ambient authority.

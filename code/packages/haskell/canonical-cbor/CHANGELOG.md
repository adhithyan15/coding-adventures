# Changelog: canonical-cbor

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial release of RFC 8949 deterministic canonical CBOR (`CodingAdventures.CanonicalCbor`) in Haskell.
- CBR01 conformance profile: RFC 8949 §4.2.3 length-first map key ordering, shortest integer encoding, depth limit 128, and 1 MiB budget cap.
- 7-class Haskell/GHC CVE resilience: Word64 -> signed Int wraparound protection, length bomb pre-allocation guards, structural-sharing fast bounds checking, depth limits, payload-blind redaction in `Show`, and fail-closed decode.
- 100% extracted error diagnostics in `Messages.hs` (Standing Rule #20).
- Pure computation with zero external Hackage dependencies (`base`, `bytestring`, `containers`, `text` only).

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-11-14

### Added
- Initial release of bounded IC18 PNG codec in Perl (`CodingAdventures::ImageCodecPNG`).
- Deterministic RGBA8 encoder with adaptive scanline filtering and zero duplicated compression primitives.
- Bounded PNG decoder with strict RFC 2083 chunk framing, untrusted input type guards, and single-copy IDAT accumulation.
- Modular architecture with clean extraction of constants (`Constants.pm`), payload-blind diagnostics (`Messages.pm`), and custom typed error (`Error.pm`).
- Comprehensive CVE-resilience suite covering 6/7 core vulnerability classes.
- Zero-dependency statement line coverage harness (`t/99-coverage.t`) asserting >= 95.0% coverage.

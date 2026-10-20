# Changelog

All notable changes to `x509-certificate-path-length-policy` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-20

### Added

- Added a consuming path-length state transition for caller-ordered
  intermediate issuing-CA role evidence.
- Added RFC 5280 non-self-issued accounting, self-issued rollover handling,
  constraint tightening, and stable payload-free failures.

### Security

- Callers retain path ordering and self-issued classification authority.
- The gate grants no path construction, trust, signature, name, time,
  revocation, TLS, HTTP, or network authority.

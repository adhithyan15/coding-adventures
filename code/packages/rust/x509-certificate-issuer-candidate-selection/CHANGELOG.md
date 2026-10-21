# Changelog

All notable changes to `x509-certificate-issuer-candidate-selection` will be
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-20

### Added

- Added bounded, allocation-free unique issuer-candidate selection over decoded
  certificates with caller-owned distinguished-name policy.
- Added non-copyable evidence binding the exact child, selected candidate, and
  caller-order index.
- Added stable payload-free limit, absence, and ambiguity failures.

### Security

- The complete accepted pool is scanned and the comparator is invoked exactly
  once per candidate, so an early match cannot hide a later ambiguity.
- Selection grants no CA role, signature, path, constraint, trust, time,
  revocation, TLS, HTTP, or network authority.

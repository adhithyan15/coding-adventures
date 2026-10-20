# Changelog

All notable changes to `x509-webpki-root-anchor-selection` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-19

### Added

- Added exact decoded-certificate subject and SPKI sequence-content selection
  against only the reviewed `webpki-roots` snapshot.
- Added complete-snapshot ambiguity detection and redacted stable failures.
- Added non-copyable evidence retaining the selected anchor's optional name
  constraints for later path policy.

### Security

- Selection grants no certificate role, path, signature, time, revocation,
  hostname, TLS, HTTP, OAuth transport, or network authority.

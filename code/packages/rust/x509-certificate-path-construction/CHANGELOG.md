# Changelog

All notable changes to `x509-certificate-path-construction` will be documented
in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-23

### Added

- Added bounded target-to-terminal chain construction over one caller-owned
  candidate pool and the existing unique issuer selector.
- Added non-copyable evidence retaining the exact target, candidate pool, and
  ordered selected indices.
- Added stable payload-free errors for invalid limits, issuer selection,
  certificate cycles, and path-depth exhaustion.

### Security

- Candidate and path bounds reject before policy callbacks, every accepted
  certificate receives one terminal-policy decision, and complete certificate
  DER equality rejects cycles through either repeated indices or re-decodes.
- Construction grants no trust, signature, certificate-role, constraint,
  validity, identity, revocation, TLS, HTTP, OAuth, or network authority.

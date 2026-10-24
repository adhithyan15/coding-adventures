# Changelog

All notable changes to `x509-certificate-path-reviewed-anchor-binding` will be
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-23

### Added

- Added exact-object binding between the final candidate selected into a
  constructed certificate path and reviewed Web PKI root-anchor evidence.
- Added non-copyable evidence retaining both complete inputs and stable,
  payload-free errors for target-only paths and evidence mismatches.

### Security

- Target-only paths, reviewed nonterminal candidates, and byte-equal
  independently decoded certificates fail closed.
- Binding grants no link authentication, certificate-role, self-signature,
  algorithm, validity, constraint, hostname, revocation, TLS, HTTP, OAuth, or
  network authority.

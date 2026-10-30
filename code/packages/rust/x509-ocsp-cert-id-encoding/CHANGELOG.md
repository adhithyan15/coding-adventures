# Changelog

All notable changes to `x509-ocsp-cert-id-encoding` will be documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added bounded validation of exact canonical hash-algorithm and serial-number
  DER inputs for one OCSP `CertID`.
- Added allocation-free encoding into caller-owned output after complete DER
  length and capacity preflight.
- Added non-copyable validation and encoding evidence retaining exact inputs.
- Added focused short-form, long-form, malformed-input, failure-atomicity,
  identity-binding, extra-capacity, and redaction coverage.

### Security

- Hash selection, authorization, computation, digest-length policy,
  certificate binding, issuer authentication, complete request framing,
  revocation, transport, and network effects remain separate boundaries.

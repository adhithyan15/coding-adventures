# Changelog

All notable changes to `x509-ocsp-cert-id-binding` will be documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-24

### Added

- Added exact target-serial and issuer-identity binding for one decoded OCSP
  `SingleResponse`.
- Added a caller-owned data-driven hash policy receiving the exact `CertID`
  algorithm, exact checked-certificate issuer Name, and candidate-issuer key
  inputs in stable order.
- Added complete fixed-size constant-time digest comparisons and non-copyable
  evidence retaining the exact response, target, and issuer certificates.
- Added stable payload-free failures and focused identity, ordering, rejection,
  length, serial, first-byte, and last-byte mismatch coverage.

### Security

- Hash algorithm selection and implementation remain explicit caller policy;
  this crate does not silently enable SHA-1 or any other algorithm.
- Issuer-link authentication, OCSP signature verification and responder
  authorization, path and trust policy, freshness, revocation, TLS, HTTP, and
  network authority remain separate boundaries.

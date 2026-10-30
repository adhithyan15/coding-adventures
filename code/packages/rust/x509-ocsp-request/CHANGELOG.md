# Changelog

## 0.1.0 - 2026-10-29

### Added

- Added exact structural validation for caller-supplied OCSP `CertID` DER.
- Added allocation-free encoding of the minimal unsigned, single-request RFC
  6960 `OCSPRequest` shape into caller-owned output.
- Added canonical short- and long-form nested-length coverage plus failure-
  atomic insufficient-output and malformed-identifier tests.

### Security

- Stable errors and debug output retain no identifier, hash, serial, request,
  or output bytes.
- The framing grants no certificate binding, nonce, responder, destination,
  revocation, TLS, HTTP, OAuth transport, or network authority.

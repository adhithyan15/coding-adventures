# Changelog

## 0.1.0 — 2026-09-19

- Added a zero-external-dependency RFC 9525 server-identity matcher for typed
  DNS-ID and IP-ID subjectAltName inputs.
- Added strict bounded ASCII DNS validation, case-insensitive exact matching,
  complete-left-most-label wildcard matching, and exact parsed-IP comparison.
- Excluded Common Name, URI-ID, SRV-ID, certificate parsing, certificate-path
  validation, trust roots, TLS, sockets, and IDNA U-label conversion.
- Added fail-closed collection limits, redacted errors, invalid-presented-name
  isolation, and exhaustive exact/wildcard/IP/cross-type/adversarial tests.

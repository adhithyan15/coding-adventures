# Changelog

## 0.1.0

- Add allocation-free structural RFC 6960 `SingleResponse` decoding.
- Compose repository-owned `CertID`, `CertStatus`, OCSP GeneralizedTime, and generic Extensions primitives under shared DER budgets.
- Reject missing, reordered, duplicated, malformed, and trailing fields with payload-free errors and whole-response offsets.
- Keep certificate binding, response authentication, responder authorization, extension semantics, freshness, revocation policy, HTTP, and TLS explicitly out of scope.

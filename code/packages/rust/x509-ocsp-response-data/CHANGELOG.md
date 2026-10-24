# Changelog

## 0.1.0

- Add allocation-free structural RFC 6960 `ResponseData` decoding.
- Compose repository-owned Name, OCSP GeneralizedTime, `SingleResponse`, and generic Extensions primitives under shared DER budgets.
- Reject explicit default or unsupported versions, missing or malformed responder identifiers, empty or oversized response sequences, malformed children, invalid optional extensions, and trailing fields with payload-free errors and whole-response offsets.
- Keep certificate binding, response authentication, responder authorization, extension and hash semantics, freshness, revocation policy, HTTP, and TLS explicitly out of scope.

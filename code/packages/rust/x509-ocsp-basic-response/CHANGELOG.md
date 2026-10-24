# Changelog

## 0.1.0

- Add allocation-free structural RFC 6960 `BasicOCSPResponse` decoding.
- Retain the exact complete response and signed `ResponseData` DER bytes
  without reserialization.
- Compose the generic signature algorithm, canonical BIT STRING, and optional
  single embedded RFC 5280 certificate under one shared caller-owned decoder.
- Reject missing, malformed, out-of-order, oversized, and trailing structures
  with payload-free categories and whole-response byte offsets.
- Keep algorithm enablement, signature verification, responder authorization,
  certificate selection and path policy, freshness, revocation decisions,
  HTTP, and TLS explicitly out of scope.

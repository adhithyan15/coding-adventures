# Changelog

## 0.1.0

- Add allocation-free structural RFC 6960 `OCSPResponse` and `ResponseBytes`
  decoding.
- Recognize only assigned response-status values and enforce the status/body
  presence relationship.
- Retain the generic response-type OID, borrowed response OCTET STRING, and
  exact complete outer encoding under shared DER budgets.
- Reject malformed, reordered, missing, unassigned, and trailing values with
  payload-free error categories and whole-response offsets.
- Keep response-type dispatch, inner decoding, authentication, responder
  authorization, certificate/path binding, freshness, revocation policy,
  HTTP, and TLS explicitly out of scope.

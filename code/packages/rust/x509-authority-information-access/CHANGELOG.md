# Changelog

## 0.1.0 — 2026-10-25

- Added bounded, allocation-free decoding for one non-critical RFC 5280
  Authority Information Access extension.
- Retained arbitrary canonical access-method OIDs with non-empty URI
  `GeneralName` locations in wire order.
- Added exact-method iteration plus public `id-ad-ocsp` and
  `id-ad-caIssuers` identifiers without granting locator authority.
- Added adversarial structure, capacity, shared-budget, offset, and redaction
  coverage.
- Preserved the enclosing Extension depth when decoding the embedded AIA
  document.

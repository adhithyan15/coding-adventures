# Changelog

## 0.3.0 — 2026-10-23

- Added bounded full-DER decoding for excluded dNSName subtrees, independently
  capped alongside optional permitted dNSName subtrees.
- Added allocation-free combined evaluation with complete constraint
  prevalidation, exclusion precedence, and any-expansion overlap checks for
  presented wildcards.
- Kept reviewed-root snapshot contents decoding explicitly permitted-only and
  rejected empty, duplicate, reversed, distance-bearing, and non-DNS forms.

## 0.2.0 — 2026-10-23

- Added bounded decoding for one exact complete DER NameConstraints SEQUENCE
  as carried by a certificate extension.
- Published the RFC 5280 `id-ce-nameConstraints` OID for data-driven extension
  lookup.
- Preserved the snapshot-contents decoder and fail-closed permitted-DNS-only
  subset.

## 0.1.0 — 2026-10-22

- Added bounded decoding of reviewed-anchor NameConstraints SEQUENCE contents.
- Added allocation-free retention for data-driven permitted DNS subtrees.
- Added fail-closed tag, cardinality, distance-field, shared-budget, offset,
  capacity, and diagnostic-redaction coverage.
- Added allocation-free permitted-DNS evaluation with strict ASCII syntax,
  leading-dot and wildcard semantics, full constraint prevalidation, bounded
  work, and payload-free indexed failures.

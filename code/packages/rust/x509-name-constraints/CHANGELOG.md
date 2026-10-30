# Changelog

## 0.5.0 — 2026-10-29

- Added bounded IPv4 and IPv6 permitted/excluded subtree evaluation with
  exact-family matching and exclusion precedence.
- Composed the repository-owned `x509-ip-name-constraint-policy` primitive so
  non-contiguous masks fail before any presented address is inspected and
  masked-out address bits do not affect subnet matching.
- Added a mixed DNS/IP entry point while keeping both single-form evaluators
  fail-closed when the other retained form remains unevaluated.

## 0.4.0 — 2026-10-29

- Added bounded complete-DER retention for exact 8-octet IPv4 and 32-octet
  IPv6 address-plus-mask GeneralSubtree values.
- Enforced one shared 256-entry ceiling per permitted or excluded field across
  DNS and IP forms, while retaining each form in wire order within its class.
- Kept the reviewed-root snapshot contents decoder restricted to its reviewed
  permitted-DNS-only input and made DNS-only evaluation fail closed while any
  retained IP subtree still requires separate policy.

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

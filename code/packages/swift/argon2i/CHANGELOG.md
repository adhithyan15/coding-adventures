# Changelog

## 0.1.0 — 2026-04-20

- Initial from-scratch Swift implementation of Argon2i (RFC 9106).
- Data-independent fill_segment: deterministic address stream via
  `double-G(0, compress(0, input_block))`, refreshed every 128 columns.
- Depends only on our sibling `Blake2b` Swift package.
- Verified against the RFC 9106 §5.2 gold-standard vector
  (`c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8`).
- Parameter validation matches RFC 9106 §3.1 bounds, raising
  typed `Argon2i.ValidationError` values.

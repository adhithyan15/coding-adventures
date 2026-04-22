# Changelog

## 0.1.0 — 2026-04-20

- Initial from-scratch Swift implementation of Argon2d (RFC 9106).
- G-mixer, permutation P, compression G, H' (`blake2bLong`),
  `indexAlpha`, `fillSegment`, and top-level `argon2d` / `argon2dHex`.
- Depends only on our sibling `Blake2b` Swift package.
- Verified against the RFC 9106 §5.1 gold-standard vector
  (`512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb`).
- Parameter validation matches RFC 9106 §3.1 bounds, raising
  typed `Argon2d.ValidationError` values.

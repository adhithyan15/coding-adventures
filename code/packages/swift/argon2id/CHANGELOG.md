# Changelog

## 0.1.0 — 2026-04-20

- Initial from-scratch Swift implementation of Argon2id (RFC 9106).
- Hybrid fill_segment: data-independent in the first two slices of the
  first pass, data-dependent thereafter.
- Depends only on our sibling `Blake2b` Swift package.
- Verified against the RFC 9106 §5.3 gold-standard vector
  (`0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659`).
- Parameter validation matches RFC 9106 §3.1 bounds, raising
  typed `Argon2id.ValidationError` values.

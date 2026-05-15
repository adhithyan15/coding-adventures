# Changelog — @coding-adventures/forme-identity

## 0.1.0 — 2026-05-15

Initial release. Implements the FM01 §7 identity layer.

### Added

- `canonicalJson(value)` — RFC 8785 (JSON Canonicalization Scheme)
  serialiser. Sorted object keys (UTF-16 code-unit order), no
  insignificant whitespace, IEEE-754 shortest-form numbers, lower-case
  `\uXXXX` for control chars, `-0` → `"0"`, NaN/Infinity throw,
  cycles throw `TypeError`.
- `computeRevisionId(payload)` — BLAKE2b-256 hash over the canonical
  JSON encoding, prefixed with `blake2b:` for forward compatibility.
- `isRevisionIdShape(value)` — format predicate; permissive about
  unknown algorithm prefixes (so future migrations don't break callers)
  but strict about hex chars and length when the algorithm matches.
- `REVISION_ALGORITHM = "blake2b"`, `REVISION_DIGEST_BYTES = 32` —
  exposed constants for callers that need them (e.g. cache-key
  derivation in `forme-cache`).
- `generateLogicalId()` — fresh UUIDv7 from `Date.now()` +
  `crypto.getRandomValues`. Throws clearly when crypto is unavailable.
- `buildLogicalIdFrom(unixMillis, randomTail)` — UUIDv7 from
  externally-supplied timestamp and 10 random bytes. Useful for
  deterministic / reproducible builds (FM03 §8).
- `isLogicalIdShape(value)` — UUIDv7 format predicate (lower-case hex,
  version nibble `7`, variant nibble `8|9|a|b`).

### Spec divergences from FM01

- **Hash algorithm.** FM01 §7 specifies BLAKE3; v0 uses BLAKE2b (the
  monorepo's existing from-scratch implementation). The `<algo>:`
  prefix in `RevisionId` keeps the format forward-compatible — a
  future BLAKE3 migration changes the prefix only, not the consumer
  contract.

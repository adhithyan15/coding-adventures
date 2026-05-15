# @coding-adventures/forme-identity

Forme kernel identity layer — `LogicalId` (UUIDv7) generation, `RevisionId` (BLAKE2b-over-canonical-JSON) computation, and the RFC 8785 canonical JSON serialiser they're built on.

See [code/specs/FM01-forme-kernel.md](../../../specs/FM01-forme-kernel.md) §7 for the design.

## Exports

| Function / Constant     | Purpose                                                                |
| ----------------------- | ---------------------------------------------------------------------- |
| `canonicalJson(value)`  | RFC 8785 canonical JSON serialisation; deterministic byte-for-byte.     |
| `computeRevisionId(v)`  | BLAKE2b-256 hash over `canonicalJson(v)`; format `blake2b:<hex>`.       |
| `isRevisionIdShape(s)`  | Predicate — does `s` match the RevisionId format?                       |
| `REVISION_ALGORITHM`    | The hash algorithm prefix (`"blake2b"`).                                 |
| `REVISION_DIGEST_BYTES` | Digest length in bytes (`32`).                                          |
| `generateLogicalId()`   | Fresh UUIDv7 from current time + crypto-RNG.                            |
| `buildLogicalIdFrom(t, r)` | UUIDv7 from caller-supplied timestamp + random tail (deterministic). |
| `isLogicalIdShape(s)`   | Predicate — does `s` match the UUIDv7 format?                           |

## Quick reference

```typescript
import {
  canonicalJson,
  computeRevisionId,
  generateLogicalId,
} from "@coding-adventures/forme-identity";

// Stable serialisation regardless of object key order.
canonicalJson({ b: 2, a: 1 });        // → '{"a":1,"b":2}'

// Content-addressed identity.
computeRevisionId({ title: "Hi" });   // → "blake2b:a1b2..."

// Time-ordered logical identity.
generateLogicalId();                  // → "01952c0d-7e63-7xxx-8xxx-..."
```

## Design highlights

- **Canonical JSON ≠ JSON.stringify.** Object keys are sorted in UTF-16 code-unit order, `-0` becomes `"0"`, NaN/Infinity throw, control characters use lower-case `\uXXXX` hex, and cycles throw `TypeError` instead of looping forever.
- **`RevisionId` format:** `blake2b:<64-hex-chars>`. The `<algo>:` prefix is forward-compatible — a future migration to BLAKE3 (when the monorepo gains a from-scratch implementation) just changes the prefix without breaking the consumer contract.
- **`LogicalId` format:** UUIDv7. 48-bit unix-millis timestamp prefix gives lexicographic-equals-chronological ordering; 74 random bits give cryptographically-safe collision resistance.
- **No ambient I/O.** Reads `Date.now()` and `globalThis.crypto.getRandomValues` only — both standard platform APIs available in Node 19+, browsers, Deno, Bun, and Workers. For deterministic builds, use `buildLogicalIdFrom` instead.

## Spec divergences from FM01 §7

- **Hash algorithm.** FM01 specifies BLAKE3; v0 uses BLAKE2b because the monorepo has a from-scratch BLAKE2b but no BLAKE3. The `<algo>:` prefix in `RevisionId` keeps the format forward-compatible.

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets 100% line + branch on every executable file.

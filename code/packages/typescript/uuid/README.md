# @ca/uuid

UUID v1/v3/v4/v5/v7 generation and parsing (RFC 4122 + RFC 9562) — implemented from scratch for educational purposes.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What is a UUID?

A UUID (Universally Unique Identifier) is a 128-bit label used for uniquely identifying information without a central authority. Two systems can each generate a UUID independently with a practically zero chance of collision.

Format: `550e8400-e29b-41d4-a716-446655440000` — 32 hex digits in 8-4-4-4-12 groups.

## Versions

| Version | Algorithm | Use case |
|---------|-----------|----------|
| v1 | Gregorian time + random node | Time-sortable, distributed systems |
| v3 | MD5 name-based | Deterministic from (namespace, name); legacy |
| v4 | Random | General purpose, most common |
| v5 | SHA-1 name-based | Deterministic from (namespace, name); preferred |
| v7 | Unix ms timestamp + random | Time-ordered, great for database primary keys |

## Installation

```bash
npm install @ca/uuid
# Or in development (within the monorepo):
npm install
```

## Usage

```typescript
import { v4, v5, v3, v1, v7, NAMESPACE_DNS, parse, isValid, UUID } from "@ca/uuid";

// Random UUID
const u = v4();
console.log(u.toString()); // e.g. 550e8400-e29b-41d4-a716-446655440000

// Name-based (deterministic)
const u2 = v5(NAMESPACE_DNS, "python.org");
console.log(u2.toString()); // 886313e1-3b8a-5372-9b90-0c9aee199e5d (always)

// Time-ordered (sortable)
const u3 = v7();
console.log(u3.version()); // 7

// Parse from string (multiple formats accepted)
const u4 = parse("urn:uuid:550e8400-e29b-41d4-a716-446655440000");
const u5 = parse("{550e8400-e29b-41d4-a716-446655440000}");
const u6 = parse("550e8400e29b41d4a716446655440000"); // compact

// Validate
isValid("not-a-uuid"); // false
```

## Types

- `UUID` — class wrapping a `Uint8Array` of 16 bytes; methods `version()`, `variant()`, `toString()`
- `UUIDError` — extends `Error`; thrown on invalid parse input

## Dependencies

- `@ca/sha1` — SHA-1 implementation (for v5)
- `@ca/md5` — MD5 implementation (for v3)

## Development

```bash
npm install
npm test
npm run test:coverage
```

## Design

All UUID implementations in this monorepo are written as literate programs: the source code explains the algorithm, the bit layouts, the Gregorian epoch offset derivation, and why the RFC 4122 variant bits are what they are. Reading the source should leave you understanding how UUIDs work at the byte level.

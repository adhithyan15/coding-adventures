# ca_uuid (Rust)

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

Add to your `Cargo.toml`:

```toml
[dependencies]
ca_uuid = { path = "../uuid" }
```

## Usage

```rust
use ca_uuid::{v4, v5, v3, v1, v7, NAMESPACE_DNS, parse, is_valid, UUID};

// Random UUID
let u = v4().unwrap();
println!("{}", u); // e.g. 550e8400-e29b-41d4-a716-446655440000

// Name-based (deterministic)
let u = v5(NAMESPACE_DNS, "python.org");
assert_eq!(u.to_string(), "886313e1-3b8a-5372-9b90-0c9aee199e5d"); // always

// Time-ordered (sortable)
let u = v7().unwrap();
assert_eq!(u.version(), 7);

// Parse from string (multiple formats accepted)
let u: UUID = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
let u = parse("550e8400e29b41d4a716446655440000").unwrap(); // compact

// Validate
assert!(!is_valid("not-a-uuid"));
```

## Types

- `UUID` — `[u8; 16]` newtype; implements `Display`, `FromStr`, `Debug`, `PartialEq`, `Eq`, `Hash`; methods `version()`, `variant()`, `as_bytes()`
- `UUIDError` — implements `std::error::Error`; returned from fallible functions

## Namespace Constants

```rust
ca_uuid::NAMESPACE_DNS   // 6ba7b810-9dad-11d1-80b4-00c04fd430c8
ca_uuid::NAMESPACE_URL   // 6ba7b811-9dad-11d1-80b4-00c04fd430c8
ca_uuid::NAMESPACE_OID   // 6ba7b812-9dad-11d1-80b4-00c04fd430c8
ca_uuid::NAMESPACE_X500  // 6ba7b814-9dad-11d1-80b4-00c04fd430c8
```

## Dependencies

- `ca_sha1` — SHA-1 implementation (for v5)
- `ca_md5` — MD5 implementation (for v3)
- `getrandom` — cryptographically secure random bytes (for v4, v7)

## Development

```bash
cargo test
cargo build --workspace
```

## Design

All UUID implementations in this monorepo are written as literate programs: the source code explains the algorithm, the bit layouts, the Gregorian epoch offset derivation, and why the RFC 4122 variant bits are what they are. Reading the source should leave you understanding how UUIDs work at the byte level.

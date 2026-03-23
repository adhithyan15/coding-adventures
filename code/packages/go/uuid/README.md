# ca_uuid (Go)

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
go get github.com/adhithyan15/coding-adventures/code/packages/go/uuid
```

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/uuid"

// Random UUID
u, err := ca_uuid.V4()
fmt.Println(u) // e.g. 550e8400-e29b-41d4-a716-446655440000

// Name-based (deterministic)
u = ca_uuid.V5(ca_uuid.NamespaceDNS, "python.org")
fmt.Println(u) // 886313e1-3b8a-5372-9b90-0c9aee199e5d (always)

// Time-ordered (sortable)
u, err = ca_uuid.V7()
fmt.Println(u.Version()) // 7

// Parse from string (multiple formats accepted)
u, err = ca_uuid.Parse("urn:uuid:550e8400-e29b-41d4-a716-446655440000")
u, err = ca_uuid.Parse("{550e8400-e29b-41d4-a716-446655440000}")
u, err = ca_uuid.Parse("550e8400e29b41d4a716446655440000") // compact

// Validate
ok := ca_uuid.IsValid("not-a-uuid") // false
```

## Types

- `UUID` — `[16]byte` value type; methods `Version()`, `Variant()`, `String()`
- `UUIDError` — dedicated error type for parse/generation failures

## Dependencies

- `ca_sha1` — SHA-1 implementation (for v5)
- `ca_md5` — MD5 implementation (for v3)

## Development

```bash
go test ./...
go vet ./...
```

## Design

All UUID implementations in this monorepo are written as literate programs: the source code explains the algorithm, the bit layouts, the Gregorian epoch offset derivation, and why the RFC 4122 variant bits are what they are. Reading the source should leave you understanding how UUIDs work at the byte level.

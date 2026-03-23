# ca_uuid (Elixir)

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

Add to your `mix.exs`:

```elixir
{:ca_uuid, path: "../uuid"}
```

Then:

```bash
mix deps.get
```

## Usage

```elixir
# Random UUID
uuid = Ca.Uuid.v4()
Ca.Uuid.to_string(uuid)
# => "550e8400-e29b-41d4-a716-446655440000"

# Name-based (deterministic)
uuid = Ca.Uuid.v5(Ca.Uuid.namespace_dns(), "python.org")
Ca.Uuid.to_string(uuid)
# => "886313e1-3b8a-5372-9b90-0c9aee199e5d"  (always)

# Time-ordered (sortable)
uuid = Ca.Uuid.v7()
Ca.Uuid.version(uuid)  # => 7

# Parse from string
{:ok, uuid} = Ca.Uuid.parse("550e8400-e29b-41d4-a716-446655440000")
{:ok, uuid} = Ca.Uuid.parse("550e8400e29b41d4a716446655440000")  # compact

# Validate
Ca.Uuid.valid?("not-a-uuid")  # => false
```

## API

All UUIDs are represented internally as 16-byte binaries (`<<...>>`). Public functions:

| Function | Description |
|----------|-------------|
| `Ca.Uuid.v1/0` | Time-based UUID |
| `Ca.Uuid.v3/2` | MD5 name-based UUID |
| `Ca.Uuid.v4/0` | Random UUID |
| `Ca.Uuid.v5/2` | SHA-1 name-based UUID |
| `Ca.Uuid.v7/0` | Unix timestamp UUID |
| `Ca.Uuid.parse/1` | `{:ok, binary}` or `{:error, reason}` |
| `Ca.Uuid.to_string/1` | Format binary as `xxxxxxxx-xxxx-...` |
| `Ca.Uuid.version/1` | Extract version integer |
| `Ca.Uuid.variant/1` | Extract variant string |
| `Ca.Uuid.valid?/1` | Boolean check on a string |
| `Ca.Uuid.namespace_dns/0` | Well-known DNS namespace binary |
| `Ca.Uuid.namespace_url/0` | Well-known URL namespace binary |
| `Ca.Uuid.namespace_oid/0` | Well-known OID namespace binary |
| `Ca.Uuid.namespace_x500/0` | Well-known X.500 namespace binary |

## Dependencies

- `:ca_sha1` — SHA-1 implementation (for v5)
- `:ca_md5` — MD5 implementation (for v3)

## Development

```bash
mix deps.get
mix test
mix test --cover
```

## Design

All UUID implementations in this monorepo are written as literate programs: the source code explains the algorithm, the bit layouts, the Gregorian epoch offset derivation, and why the RFC 4122 variant bits are what they are. Reading the source should leave you understanding how UUIDs work at the byte level.

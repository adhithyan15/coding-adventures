# coding_adventures_uuid (Elixir)

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
{:coding_adventures_uuid, path: "../uuid"}
```

Then:

```bash
mix deps.get
```

## Usage

```elixir
# Random UUID
uuid = CodingAdventures.Uuid.v4()
CodingAdventures.Uuid.to_string(uuid)
# => "550e8400-e29b-41d4-a716-446655440000"

# Name-based (deterministic)
uuid = CodingAdventures.Uuid.v5(CodingAdventures.Uuid.namespace_dns(), "python.org")
CodingAdventures.Uuid.to_string(uuid)
# => "886313e1-3b8a-5372-9b90-0c9aee199e5d"  (always)

# Time-ordered (sortable)
uuid = CodingAdventures.Uuid.v7()
CodingAdventures.Uuid.version(uuid)  # => 7

# Parse from string
{:ok, uuid} = CodingAdventures.Uuid.parse("550e8400-e29b-41d4-a716-446655440000")
{:ok, uuid} = CodingAdventures.Uuid.parse("550e8400e29b41d4a716446655440000")  # compact

# Validate
CodingAdventures.Uuid.valid?("not-a-uuid")  # => false
```

## API

All UUIDs are represented internally as 16-byte binaries (`<<...>>`). Public functions:

| Function | Description |
|----------|-------------|
| `CodingAdventures.Uuid.v1/0` | Time-based UUID |
| `CodingAdventures.Uuid.v3/2` | MD5 name-based UUID |
| `CodingAdventures.Uuid.v4/0` | Random UUID |
| `CodingAdventures.Uuid.v5/2` | SHA-1 name-based UUID |
| `CodingAdventures.Uuid.v7/0` | Unix timestamp UUID |
| `CodingAdventures.Uuid.parse/1` | `{:ok, binary}` or `{:error, reason}` |
| `CodingAdventures.Uuid.to_string/1` | Format binary as `xxxxxxxx-xxxx-...` |
| `CodingAdventures.Uuid.version/1` | Extract version integer |
| `CodingAdventures.Uuid.variant/1` | Extract variant string |
| `CodingAdventures.Uuid.valid?/1` | Boolean check on a string |
| `CodingAdventures.Uuid.namespace_dns/0` | Well-known DNS namespace binary |
| `CodingAdventures.Uuid.namespace_url/0` | Well-known URL namespace binary |
| `CodingAdventures.Uuid.namespace_oid/0` | Well-known OID namespace binary |
| `CodingAdventures.Uuid.namespace_x500/0` | Well-known X.500 namespace binary |

## Dependencies

- `:coding_adventures_sha1` — SHA-1 implementation (for v5)
- `:coding_adventures_md5` — MD5 implementation (for v3)

## Development

```bash
mix deps.get
mix test
mix test --cover
```

## Design

All UUID implementations in this monorepo are written as literate programs: the source code explains the algorithm, the bit layouts, the Gregorian epoch offset derivation, and why the RFC 4122 variant bits are what they are. Reading the source should leave you understanding how UUIDs work at the byte level.

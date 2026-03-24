# ca_uuid (Ruby)

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
gem install ca_uuid
# Or in development:
bundle install
```

## Usage

```ruby
require "ca_uuid"

# Random UUID
u = Ca::Uuid.v4
puts u.to_s  # e.g. 550e8400-e29b-41d4-a716-446655440000

# Name-based (deterministic)
u = Ca::Uuid.v5(Ca::Uuid::NAMESPACE_DNS, "python.org")
puts u.to_s  # 886313e1-3b8a-5372-9b90-0c9aee199e5d (always)

# Time-ordered (sortable)
u = Ca::Uuid.v7
puts u.version  # 7

# Parse from string (multiple formats accepted)
u = Ca::Uuid.parse("urn:uuid:550e8400-e29b-41d4-a716-446655440000")
u = Ca::Uuid.parse("{550e8400-e29b-41d4-a716-446655440000}")
u = Ca::Uuid.parse("550e8400e29b41d4a716446655440000")  # compact

# Validate
Ca::Uuid.valid?("not-a-uuid")  # false
```

## Types

- `Ca::Uuid::UUID` — wraps a 16-byte binary string; methods `version`, `variant`, `to_s`, `to_i`, `to_hex`
- `Ca::Uuid::UUIDError` — raised on invalid parse input (subclass of `ArgumentError`)

## Namespace Constants

```ruby
Ca::Uuid::NAMESPACE_DNS   # 6ba7b810-9dad-11d1-80b4-00c04fd430c8
Ca::Uuid::NAMESPACE_URL   # 6ba7b811-9dad-11d1-80b4-00c04fd430c8
Ca::Uuid::NAMESPACE_OID   # 6ba7b812-9dad-11d1-80b4-00c04fd430c8
Ca::Uuid::NAMESPACE_X500  # 6ba7b814-9dad-11d1-80b4-00c04fd430c8
```

## Dependencies

- `ca_sha1` — SHA-1 implementation (for v5)
- `ca_md5` — MD5 implementation (for v3)

## Development

```bash
bundle install
bundle exec rake test
```

## Design

All UUID implementations in this monorepo are written as literate programs: the source code explains the algorithm, the bit layouts, the Gregorian epoch offset derivation, and why the RFC 4122 variant bits are what they are. Reading the source should leave you understanding how UUIDs work at the byte level.

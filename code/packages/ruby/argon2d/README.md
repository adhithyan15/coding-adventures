# coding_adventures_argon2d

A pure-Ruby, from-scratch implementation of **Argon2d** (RFC 9106) —
data-dependent memory-hard password hashing.

## What is Argon2d?

Argon2d uses **data-dependent** addressing throughout every segment:
the reference block index for each new block is derived from the
first 64 bits of the previously computed block. This maximises
GPU/ASIC resistance at the cost of leaking a noisy channel through
memory-access timing. Use Argon2d only in contexts where
side-channel attacks are *not* in the threat model (e.g.
proof-of-work). For password hashing, prefer
[`argon2id`](../argon2id/).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```ruby
require "coding_adventures_argon2d"

tag = CodingAdventures::Argon2d.argon2d(
  "password", "somesalt",
  3,   # time_cost
  64,  # memory_cost (KiB)
  1,   # parallelism
  32   # tag_length (bytes)
)

hex = CodingAdventures::Argon2d.argon2d_hex("password", "somesalt", 3, 64, 1, 32)
```

### Keyed / authenticated data

```ruby
tag = CodingAdventures::Argon2d.argon2d(
  password, salt, 3, 64, 1, 32,
  key: server_secret,
  associated_data: "challenge-id"
)
```

## API

| Method | Returns |
| -- | -- |
| `Argon2d.argon2d(password, salt, t, m, p, T, **opts)` | `String` (binary, `T` bytes) |
| `Argon2d.argon2d_hex(password, salt, t, m, p, T, **opts)` | `String` (lowercase hex) |

Parameters follow RFC 9106 §3.1.

## Where this fits in the stack

- **Dependencies:** [`coding_adventures_blake2b`](../blake2b/) (H0 and H' extender).

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations.
  Clamp caller-controlled values at the application layer.
- **Verify in constant time.** When comparing a stored tag to a
  freshly computed one, use a constant-time comparison (e.g.
  `OpenSSL.fixed_length_secure_compare`) rather than `==` on binary
  strings.

## Running the tests

```bash
cd code/packages/ruby/argon2d
bundle install
bundle exec rake test
```

Tests include the canonical RFC 9106 §5.1 gold-standard vector, plus
18 parameter-edge tests (19 total) covering validation, determinism,
binding to key/AD, tag-length variants, and multi-lane / multi-pass
parameters.

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).

# coding_adventures_sha512

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in Ruby.

## What Is SHA-512?

SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces a 512-bit (64-byte) digest using 8 x 64-bit state words and 80 rounds. On 64-bit platforms it is often faster than SHA-256 because it processes 128-byte blocks.

## How It Fits

Part of the `coding-adventures` monorepo hash function collection, alongside MD5, SHA-1, and SHA-256. Implemented from scratch with literate programming style for learning.

## Usage

```ruby
require "coding_adventures_sha512"

SHA512 = CodingAdventures::Sha512

# One-shot
digest = SHA512.sha512("abc")          # 64-byte binary string
hex    = SHA512.sha512_hex("abc")      # 128-char hex string

# Streaming
h = SHA512::Digest.new
h.update("ab")
h.update("c")
h.hexdigest  # same as SHA512.sha512_hex("abc")

# Copy for prefix sharing
h = SHA512::Digest.new
h.update("common_prefix")
h1 = h.copy
h1.update("suffix_a")
h2 = h.copy
h2.update("suffix_b")
```

## API

| Method | Returns | Description |
|--------|---------|-------------|
| `Sha512.sha512(data)` | `String` | 64-byte binary digest |
| `Sha512.sha512_hex(data)` | `String` | 128-char lowercase hex |
| `Digest.new` | `Digest` | Streaming hasher |
| `#update(data)` | `self` | Feed bytes (chainable) |
| `#digest` | `String` | Get 64-byte result (non-destructive) |
| `#hexdigest` | `String` | Get 128-char hex (non-destructive) |
| `#copy` | `Digest` | Deep copy of hasher state |

## Development

```bash
bundle install
bundle exec rake test
```

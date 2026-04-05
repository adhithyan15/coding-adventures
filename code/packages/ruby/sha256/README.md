# coding_adventures_sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in Ruby.

## What Is SHA-256?

SHA-256 is a cryptographic hash function from the SHA-2 family that produces a 256-bit (32-byte) digest. It is the workhorse of modern cryptography, used in TLS, Bitcoin, git, code signing, and password hashing.

This gem implements SHA-256 from first principles with no dependencies on `Digest::SHA256` or any other cryptographic library.

## How It Fits in the Stack

This is package HF03 in the coding-adventures monorepo (Ruby variant). It builds on the same Merkle-Damgard construction used in the SHA-1 gem, but with a wider state (8 words), more complex message schedule, and stronger auxiliary functions.

## Installation

```ruby
gem "coding_adventures_sha256", path: "code/packages/ruby/sha256"
```

## Usage

### One-shot hashing

```ruby
require "coding_adventures_sha256"

digest = CodingAdventures::Sha256.sha256("hello world")      # 32 bytes
hex    = CodingAdventures::Sha256.sha256_hex("hello world")   # 64-char hex
```

### Streaming (chunked) hashing

```ruby
h = CodingAdventures::Sha256::Digest.new
h.update("hello ")
h.update("world")
puts h.hexdigest  # same as sha256_hex("hello world")
```

### Branching with copy

```ruby
h = CodingAdventures::Sha256::Digest.new
h.update("common prefix")
h1 = h.copy
h2 = h.copy
h1.update(" branch A")
h2.update(" branch B")
```

## API

| Method | Description |
|---|---|
| `Sha256.sha256(data)` | One-shot, returns 32 bytes |
| `Sha256.sha256_hex(data)` | One-shot, returns 64-char hex |
| `Digest.new` | Create streaming hasher |
| `#update(data)` | Feed bytes (chainable) |
| `#digest` | Get 32-byte result (non-destructive) |
| `#hexdigest` | Get 64-char hex result |
| `#copy` | Deep clone for branching |

## Testing

```bash
bundle exec rake test
```

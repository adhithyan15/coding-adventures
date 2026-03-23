# coding_adventures_sha1 (Ruby)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in Ruby.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest.
The same input always yields the same digest. Change one bit of input and the entire
digest changes — the avalanche effect. This package implements SHA-1 from scratch,
without using Ruby's `Digest::SHA1`, so every step of the algorithm is visible.

## How It Fits in the Stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo. SHA-1 is a prerequisite for the UUID v5 package.

## Usage

```ruby
require "coding_adventures_sha1"

# One-shot
digest = CodingAdventures::Sha1.sha1("abc")       # binary String, 20 bytes
hex = CodingAdventures::Sha1.sha1_hex("abc")      # "a9993e364706816aba3e25717850c26c9cd0d89d"

# Streaming
h = CodingAdventures::Sha1::Digest.new
h.update("ab")
h << "c"                                          # << is an alias for update
puts h.hexdigest                                  # "a9993e364706816aba3e25717850c26c9cd0d89d"

# Copy for prefix hashing
h2 = h.copy
h2.update(" world")
```

## FIPS 180-4 Test Vectors

```ruby
CodingAdventures::Sha1.sha1_hex("") == "da39a3ee5e6b4b0d3255bfef95601890afd80709"
CodingAdventures::Sha1.sha1_hex("abc") == "a9993e364706816aba3e25717850c26c9cd0d89d"
```

## Development

```bash
bundle install
bundle exec rake test
```

Tests: 37 tests, all passing.

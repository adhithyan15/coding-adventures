# coding_adventures_md5

MD5 message digest algorithm (RFC 1321) implemented from scratch in Ruby.

## What Is MD5?

MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size 16-byte (128-bit) "fingerprint" called a digest. The same input always produces the same digest. Change even one bit of input and the digest changes completely (avalanche effect).

**Security warning:** MD5 is cryptographically broken — collision attacks have been practical since 2004. Do NOT use MD5 for passwords, digital signatures, TLS certificates, or any security-sensitive purpose. Use SHA-256 or SHA-3 instead. MD5 remains appropriate for: non-security checksums, UUID v3 generation, and legacy system compatibility.

## Where This Fits in the Stack

This package implements the MD5 algorithm purely from arithmetic primitives — no external hash libraries. It sits at the same level as the SHA-1 package and is used by UUID v3 generation.

```
UUID v3
  └── md5 (this package)
        └── Ruby arithmetic + String#pack/unpack
```

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_md5", path: "../md5"
```

## Usage

### One-shot API

```ruby
require "coding_adventures_md5"

# Returns 16-byte binary String
CodingAdventures::Md5.md5("abc")
# => "\x90\x01P\x98<\xD2..."  (16 bytes)

# Returns 32-character lowercase hex string
CodingAdventures::Md5.md5_hex("abc")
# => "900150983cd24fb0d6963f7d28e17f72"

CodingAdventures::Md5.md5_hex("")
# => "d41d8cd98f00b204e9800998ecf8427e"
```

### Streaming API (Digest class)

```ruby
require "coding_adventures_md5"

d = CodingAdventures::Md5::Digest.new
d.update("Hello, ")
d << "world!"          # << is an alias for update
d.hexdigest            # => "e5a00d6eeab1a4e0901b0ef31f645a0a"
d.digest               # => 16-byte binary String

# Chaining
result = CodingAdventures::Md5::Digest.new
  .update("abc")
  .update("def")
  .hexdigest
# => "e80b5017098950fc58aad83c8c14978e"

# Branching: hash a common prefix, then fork
base = CodingAdventures::Md5::Digest.new.update("prefix ")
branch_a = base.copy.update("A").hexdigest
branch_b = base.copy.update("B").hexdigest
```

## Algorithm Details

### Little-endian — the key difference from SHA-1

MD5 uses **little-endian** byte order throughout. This is the most common source of MD5 implementation bugs. In Ruby's pack/unpack format strings:

| Format | Meaning                              | Used by |
|--------|--------------------------------------|---------|
| `"N"`  | Big-endian 32-bit unsigned           | SHA-1   |
| `"V"`  | **Little-endian 32-bit unsigned**    | **MD5** |

MD5 reads each 64-byte block as `block.unpack("V16")` and writes the final digest as `state.pack("V4")`.

### The T-Table (64 sine-derived constants)

Each of the 64 rounds uses a constant derived from the sine function:

```
T[i] = floor(abs(sin(i)) × 2^32)   for i = 1..64
```

Example: `sin(1) ≈ 0.8414709848`, so `T[1] = floor(0.8414709848 × 4294967296) = 3614090360 = 0xD76AA478`.

### Four-stage compression

64 rounds divided into 4 stages of 16, each with a different auxiliary function mixing the four state words A, B, C, D:

| Stage | Rounds | Function f(b,c,d)      | Message index g |
|-------|--------|------------------------|-----------------|
| 1 (F) | 0–15   | `(b & c) \| (~b & d)` | `i`             |
| 2 (G) | 16–31  | `(d & b) \| (~d & c)` | `(5i+1) % 16`  |
| 3 (H) | 32–47  | `b ^ c ^ d`            | `(3i+5) % 16`  |
| 4 (I) | 48–63  | `c ^ (b \| ~d)`        | `(7i) % 16`    |

### RFC 1321 test vectors

| Input                     | MD5 Hex Digest                     |
|---------------------------|------------------------------------|
| `""`                      | `d41d8cd98f00b204e9800998ecf8427e` |
| `"a"`                     | `0cc175b9c0f1b6a831c399e269772661` |
| `"abc"`                   | `900150983cd24fb0d6963f7d28e17f72` |
| `"message digest"`        | `f96b697d7cb7938d525a2f31aaf161d0` |
| `"abcdefghijklmnopqrstuvwxyz"` | `c3fcd3d76192e4007dfb496cca67e13b` |

## Development

```bash
# Run tests
bundle install --quiet
bundle exec rake test
```

## Ruby-specific implementation notes

- Ruby integers are arbitrary-precision. Every arithmetic operation on 32-bit words **must** be masked with `& 0xFFFFFFFF` to stay within 32 bits.
- Strings have encodings in Ruby. The implementation calls `.b` (equivalent to `.force_encoding(Encoding::ASCII_8BIT)`) to work with raw bytes without transcoding.
- `String#byteslice` is used instead of `[]` to avoid encoding-aware slicing.

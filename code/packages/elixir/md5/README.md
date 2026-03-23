# md5 (Elixir)

MD5 message digest algorithm (RFC 1321) implemented from scratch in Elixir.

Part of the coding-adventures monorepo — a ground-up implementation of the
computing stack from transistors to operating systems.

## What It Does

Produces a 128-bit (16-byte) cryptographic digest from any binary input.
The same input always produces the same digest; any change in input produces
a completely different digest (avalanche effect).

**Security note:** MD5 is cryptographically broken — collision attacks have
existed since 2004. Do NOT use for passwords, digital signatures, or TLS.
Valid uses: UUID v3, non-security checksums, legacy compatibility.

## Public API

```elixir
# Returns a 16-byte binary
CodingAdventures.Md5.md5("abc")
# => <<144, 1, 80, 152, 60, 210, 79, 176, 214, 150, 63, 125, 40, 225, 127, 114>>

# Returns a 32-character lowercase hex string
CodingAdventures.Md5.md5_hex("abc")
# => "900150983cd24fb0d6963f7d28e17f72"

CodingAdventures.Md5.md5_hex("")
# => "d41d8cd98f00b204e9800998ecf8427e"

CodingAdventures.Md5.md5_hex("message digest")
# => "f96b697d7cb7938d525a2f31aaf161d0"
```

## How It Fits In The Stack

```
RFC 1321 (MD5 spec)
     ↓
CodingAdventures.Md5   ← this package
     ↓
UUID v3 (uses MD5 as its hash function)
     ↓
Distributed systems, legacy protocols
```

## Algorithm Overview

1. **Pad** the message to a multiple of 64 bytes:
   append `0x80`, then zeros, then the 64-bit little-endian bit count.

2. **Compress** each 64-byte block through 64 rounds:
   - Parse block as 16 × 32-bit little-endian words M[0..15]
   - Four stages of 16 rounds each, using auxiliary functions F/G/H/I
   - Davies-Meyer feed-forward adds the compressed output to the input state

3. **Output** the four state words A/B/C/D as 16 bytes, little-endian.

## Key Implementation Detail: Little-Endian

MD5 is uniquely little-endian among hash functions — SHA-1 and SHA-256 are
big-endian. In Elixir, this is expressed directly in bitstring syntax:

```elixir
# Parse a little-endian 32-bit word from binary
<<word::little-32, rest::binary>>

# Produce little-endian output
<<a::little-32, b::little-32, c::little-32, d::little-32>>

# Pad with little-endian 64-bit length
<<bit_len::little-64>>
```

## Development

```bash
# Run tests
mix deps.get && mix test --cover

# Using the build system
bash BUILD
```

## Test Coverage

38 tests covering:
- All RFC 1321 official test vectors
- Output format (16 bytes, 32 hex chars, lowercase)
- Little-endian correctness
- Block boundary edge cases (55, 56, 63, 64, 65, 128 bytes)
- Avalanche effect
- Well-known hashes (hello, fox/dog, null bytes, all byte values)

Coverage: **100%**

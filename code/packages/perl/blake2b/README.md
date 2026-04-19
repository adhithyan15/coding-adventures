# BLAKE2b (Perl)

Pure Perl implementation of the **BLAKE2b** cryptographic hash function
(RFC 7693).  No XS, no C extensions, no external CPAN crypto deps.

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full walk-through.

## Requirements

- Perl 5.26+ on a 64-bit build (`ivsize == 8`)

## Usage

```perl
use CodingAdventures::Blake2b;

# One-shot
my $hex = CodingAdventures::Blake2b::blake2b_hex("hello");
my $raw = CodingAdventures::Blake2b::blake2b("hello", digest_size => 32);

# Keyed (MAC mode)
my $tag = CodingAdventures::Blake2b::blake2b_hex(
    $message, key => "shared-secret", digest_size => 32,
);

# Streaming
my $h = CodingAdventures::Blake2b->new(digest_size => 32);
$h->update("hello ");
$h->update("world");
my $out = $h->hex_digest;

# Salt + personal (each exactly 16 bytes, or absent)
my $salt     = "\x00" x 16;
my $personal = "\x00" x 16;
my $dsep = CodingAdventures::Blake2b::blake2b_hex(
    $data, salt => $salt, personal => $personal,
);
```

## API

| Function / method | Returns | Description |
|---|---|---|
| `blake2b($data, %opts)` | string | Raw digest bytes |
| `blake2b_hex($data, %opts)` | string | Lowercase hex digest |
| `->new(%opts)` | object | Streaming hasher |
| `$h->update($data)` | self | Absorb more bytes |
| `$h->digest` | string | Finalize (non-destructive) |
| `$h->hex_digest` | string | Finalize to lowercase hex |
| `$h->copy` | object | Independent deep copy |

`%opts` may contain any of `digest_size` (1..64, default 64), `key`
(0..64 bytes), `salt` (exactly 0 or 16 bytes), `personal` (exactly 0 or
16 bytes).

## Implementation notes

Perl on a 64-bit build has 64-bit integers, but naïve unsigned arithmetic
silently promotes large sums to floating-point NVs.  Every addition in
this module is wrapped in `use integer` (signed 64-bit wrap-on-overflow)
and then masked with `0xFFFFFFFFFFFFFFFF` -- yielding the unsigned
`mod 2^64` semantics the RFC specifies.

Bitwise ops (`& | ^ ~ >> <<`) behave correctly on 64-bit values without
`use integer`.  `pack("Q<", $w)` and `unpack("Q<8", $s)` give little-
endian 64-bit packing natively.

The 128-bit byte counter is represented as a single 64-bit Perl integer,
which is sufficient for any practical input (up to 2^64 - 1 bytes).  The
spec's reserved high 64 bits are always zero for realistic inputs.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.

## Running the tests

```bash
cpanm --with-test --installdeps .
prove -l -v t/
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation in the monorepo.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  BLAKE2b is a prerequisite for Argon2
(the memory-hard password hashing function).

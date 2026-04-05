# SHA-512 (Perl)

Pure Perl implementation of the SHA-512 cryptographic hash function (FIPS 180-4).

## Overview

SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces a 512-bit (64-byte) digest using 8 x 64-bit state words and 80 rounds of compression. On 64-bit platforms, SHA-512 is often faster than SHA-256 because it processes 128-byte blocks using native 64-bit arithmetic.

## Requirements

- Perl 5.26+ with 64-bit integer support

## Usage

```perl
use CodingAdventures::Sha512;

# Functional interface
my $hex   = CodingAdventures::Sha512::hex("hello");
my $bytes = CodingAdventures::Sha512::digest("hello");

# OO interface
my $sha512 = CodingAdventures::Sha512->new();
my $hex    = $sha512->hex("hello");
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `hex($message)` | string | 128-character lowercase hex digest |
| `digest($message)` | arrayref | 64-element arrayref of byte values (0-255) |

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers.

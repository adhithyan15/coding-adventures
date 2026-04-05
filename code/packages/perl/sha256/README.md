# CodingAdventures::SHA256 (Perl)

Pure Perl implementation of the SHA-256 cryptographic hash function (FIPS 180-4).

## Overview

SHA-256 is a member of the SHA-2 family that produces a 256-bit (32-byte) digest. It uses the Merkle-Damgard construction with 8 x 32-bit state words and 64 compression rounds per block. This implementation provides both one-shot and streaming APIs.

## Installation

```bash
cpanm --installdeps .
```

## Usage

### One-shot API

```perl
use CodingAdventures::SHA256;

# Hex digest (64-character lowercase string)
my $hex = CodingAdventures::SHA256::sha256_hex("hello");
# "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

# Raw digest (arrayref of 32 integers, each 0-255)
my $bytes = CodingAdventures::SHA256::sha256("hello");
```

### Streaming API

```perl
use CodingAdventures::SHA256;

my $hasher = CodingAdventures::SHA256->new();
$hasher->update("hello ");
$hasher->update("world");
print $hasher->hex_digest();  # same as sha256_hex("hello world")

# Non-destructive: can call digest multiple times
my $d1 = $hasher->hex_digest();
my $d2 = $hasher->hex_digest();
# $d1 eq $d2

# Copy for branching
my $branch = $hasher->copy();
$branch->update("!");
```

## API Reference

| Function | Returns | Description |
|----------|---------|-------------|
| `sha256($data)` | arrayref of 32 ints | Raw 32-byte digest |
| `sha256_hex($data)` | string | 64-char lowercase hex digest |
| `new()` | object | Create streaming hasher |
| `$h->update($data)` | $self | Feed bytes (chainable) |
| `$h->digest()` | arrayref of 32 ints | Get digest (non-destructive) |
| `$h->hex_digest()` | string | Get hex digest (non-destructive) |
| `$h->copy()` | object | Deep copy for branching |

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers. This package implements HF03 (SHA-256) from the hash functions layer.

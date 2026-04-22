# CodingAdventures::Argon2d

Pure-Perl implementation of **Argon2d** (RFC 9106) — the data-dependent
variant of the Argon2 memory-hard password hashing family.

## What is Argon2d?

Argon2d reads the reference-block index for every new block from the
first 64 bits of the previously computed block. The memory-access
pattern therefore depends on the password, which maximises resistance
to GPU/ASIC cracking at the cost of leaking a side channel through
memory-access timing.

Use Argon2d only when side-channel attacks are **not** in the threat
model (e.g. proof-of-work). For password hashing prefer
[`CodingAdventures::Argon2id`](../argon2id/).

## Installation

```
cpanm --installdeps .
perl Makefile.PL
make
make test
```

The package depends on [`CodingAdventures::Blake2b`](../blake2b/) as
the only runtime prerequisite.

## Usage

```perl
use CodingAdventures::Argon2d;

# Raw binary tag (32 bytes here).
my $tag = CodingAdventures::Argon2d::argon2d(
    $password, $salt,
    3,      # time_cost  (passes)
    65536,  # memory_cost (KiB)
    4,      # parallelism (lanes)
    32,     # tag_length  (bytes)
    key             => $optional_mac_key,
    associated_data => $optional_context,
);

# Or hex-encoded.
my $hex = CodingAdventures::Argon2d::argon2d_hex(
    $password, $salt, 3, 65536, 4, 32,
);
```

All string inputs are treated as raw byte strings.

## Parameter bounds (RFC 9106)

| Parameter      | Constraint            |
|----------------|-----------------------|
| `salt`         | ≥ 8 bytes             |
| `tag_length`   | ≥ 4 bytes             |
| `parallelism`  | 1 … 2²⁴ − 1           |
| `memory_cost`  | ≥ 8 × parallelism     |
| `time_cost`    | ≥ 1                   |
| `version`      | 0x13 only             |

Password, salt, key, associated-data, memory-cost, and tag-length are
capped at 2³² − 1 so that the RFC 9106 H₀ length fields stay valid.

## Security

Argon2 is **designed** to burn memory and CPU. Callers control that
DoS boundary by picking the `memory_cost`, `time_cost`, and
`parallelism` parameters. Do not expose them to untrusted input
without bounds of your own.

This module does **not** perform constant-time tag comparison. When
verifying a tag you MUST use a constant-time equality check — a naïve
`eq` leaks byte-level timing.

## Where this fits

Argon2d sits in the `KD03` key-derivation layer of this repo's
cryptographic stack. It depends only on `CodingAdventures::Blake2b`
(from `HF06`) and is used by higher-level key-schedule packages.

# CodingAdventures::Argon2id

Pure-Perl implementation of **Argon2id** (RFC 9106) — the hybrid
Argon2 variant and the RFC-recommended default for password hashing.

## What is Argon2id?

Argon2id runs the Argon2**i** (data-independent) fill for the first
pass over the first two slices, then switches to the Argon2**d**
(data-dependent) fill for the rest of the computation. You get
Argon2i's side-channel resistance over the memory region most
accessible to a timing attacker, plus Argon2d's GPU/ASIC hardening
everywhere else.

**This is the variant you want for password hashing.**

## Installation

```
cpanm --installdeps .
perl Makefile.PL
make
make test
```

Runtime dependency: [`CodingAdventures::Blake2b`](../blake2b/).

## Usage

```perl
use CodingAdventures::Argon2id;

# Sensible defaults (adjust memory/time for your hardware).
my $tag = CodingAdventures::Argon2id::argon2id(
    $password, $salt,
    3,       # time_cost
    65536,   # memory_cost (KiB)
    4,       # parallelism
    32,      # tag_length
);

my $hex = CodingAdventures::Argon2id::argon2id_hex(
    $password, $salt, 3, 65536, 4, 32,
);
```

Optional arguments (hash-style):
- `key             => $secret_mac_key`
- `associated_data => $context_bytes`

## Security

Argon2 is designed to burn memory and CPU. Callers control the DoS
boundary via `memory_cost`, `time_cost`, and `parallelism` — do not
forward untrusted values.

**Constant-time comparison**: this module does *not* constant-time-
compare tags for you. When verifying an Argon2id tag against a stored
value, use a constant-time equality helper to defeat byte-level timing
oracles.

## Where this fits

Argon2id sits in the `KD03` key-derivation layer. It depends only on
`CodingAdventures::Blake2b` (from `HF06`).

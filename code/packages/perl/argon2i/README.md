# CodingAdventures::Argon2i

Pure-Perl implementation of **Argon2i** (RFC 9106) — the
data-independent variant of the Argon2 memory-hard password hashing
family.

## What is Argon2i?

Argon2i derives every reference-block index from a deterministic
pseudo-random stream seeded purely from public parameters (pass, lane,
slice, total memory, total passes, type, counter). The memory-access
pattern is therefore constant across secrets, which defeats
memory-access side-channel observers at the cost of making the variant
the easiest for GPUs/ASICs to parallelise.

For general-purpose password hashing prefer
[`CodingAdventures::Argon2id`](../argon2id/). Use Argon2i only when
side-channel resistance is the dominant concern.

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
use CodingAdventures::Argon2i;

my $tag = CodingAdventures::Argon2i::argon2i(
    $password, $salt, 3, 65536, 4, 32,
    key             => $optional_key,
    associated_data => $optional_ad,
);

my $hex = CodingAdventures::Argon2i::argon2i_hex(
    $password, $salt, 3, 65536, 4, 32,
);
```

## Security

Argon2 is designed to burn memory and CPU. Callers control the DoS
boundary via `memory_cost`, `time_cost`, and `parallelism` — do not
forward untrusted values.

This module does **not** perform constant-time tag comparison. Use a
constant-time equality primitive of your own when verifying a tag.

## Where this fits

Argon2i sits in the `KD03` key-derivation layer. It depends only on
`CodingAdventures::Blake2b` (from `HF06`).

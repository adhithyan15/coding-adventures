# X25519 (Perl)

Pure Perl implementation of X25519 (RFC 7748) — elliptic curve Diffie-Hellman on Curve25519.

## What is X25519?

X25519 is a key agreement protocol that allows two parties to establish a shared secret over an insecure channel. It operates on Curve25519, a Montgomery-form elliptic curve designed by Daniel Bernstein.

## How It Works

1. **Scalar clamping** — Private key is modified for algebraic safety (divisible by 8, high bit set).
2. **Montgomery ladder** — Scalar multiplication using only the u-coordinate, scanning bits high to low.
3. **Field arithmetic** — All operations in GF(2^255-19), using Perl's core Math::BigInt.
4. **Final inversion** — Projective (X, Z) converted to affine (X/Z) via Fermat's little theorem.

## Usage

```perl
use CodingAdventures::X25519 qw(x25519 x25519_base generate_keypair);

# Generate a keypair
my $private_key = ...; # 32 random bytes
my $public_key = x25519_base($private_key);

# Diffie-Hellman
my $shared_secret = x25519($my_private, $their_public);
```

## Dependencies

None beyond Perl core. Math::BigInt ships with Perl.

## Running Tests

```bash
prove -l -v t/
```

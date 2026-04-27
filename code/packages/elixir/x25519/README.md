# X25519 (Elixir)

Pure Elixir implementation of X25519 (RFC 7748) — elliptic curve Diffie-Hellman on Curve25519.

## What is X25519?

X25519 is a key agreement protocol that allows two parties to establish a shared secret over an insecure channel. It operates on Curve25519, a Montgomery-form elliptic curve designed by Daniel Bernstein for high performance and resistance to side-channel attacks.

X25519 is used in TLS 1.3, Signal Protocol, WireGuard, SSH, and many other modern cryptographic systems.

## How It Works

1. **Scalar clamping** — The private key is modified to ensure it has the right algebraic properties (divisible by 8, high bit set for constant-time execution).
2. **Montgomery ladder** — Scalar multiplication is performed using only the x-coordinate (u-coordinate) of the curve point, scanning bits from high to low.
3. **Field arithmetic** — All operations are in GF(2^255-19), using Elixir's native arbitrary-precision integers.
4. **Final inversion** — The projective result (X, Z) is converted to affine (X/Z) via Fermat's little theorem.

## Usage

```elixir
alias CodingAdventures.X25519

# Generate a keypair
private_key = :crypto.strong_rand_bytes(32)
public_key = X25519.x25519_base(private_key)

# Or use generate_keypair/1
{priv, pub} = X25519.generate_keypair(private_key)

# Diffie-Hellman key exchange
shared_secret = X25519.x25519(my_private_key, their_public_key)
```

## API

- `x25519(scalar, u_point)` — Compute scalar * u_point. Both are 32-byte binaries.
- `x25519_base(scalar)` — Compute scalar * base_point (u=9).
- `generate_keypair(private_key)` — Returns `{private_key, public_key}`.

## Dependencies

None. All field arithmetic is implemented from scratch using native Elixir integers.

## Running Tests

```bash
mix test --cover
```

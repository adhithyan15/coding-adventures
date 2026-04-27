# Argon2i (Swift)

From-scratch Swift implementation of **Argon2i** (RFC 9106) — the
data-independent member of the Argon2 memory-hard KDF family.

Depends only on our sibling `Blake2b` package for the underlying hash.

See the spec at [../../specs/KD03-argon2.md](../../specs/KD03-argon2.md).

## Usage

```swift
import Argon2i

let tag = try Argon2i.argon2i(
    password: Array("secret".utf8),
    salt: Array("0123456789abcdef".utf8),
    timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32
)

let hex = try Argon2i.argon2iHex(
    password: Array("secret".utf8),
    salt: Array("0123456789abcdef".utf8),
    timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32
)
```

## When to pick Argon2i

Argon2i avoids password-dependent memory accesses entirely, giving
side-channel resistance. The price is weaker GPU/ASIC hardening than
Argon2d. RFC 9106 explicitly recommends **Argon2id** as the general
password-hashing default; use `Argon2i` only when side-channel
resistance is paramount and hardware-cracking resistance is not (e.g.
a constrained-channel embedded device).

## Implementation notes

The deterministic address stream is computed as `double-G(0,
compress(0, input_block))` — exactly the construction in the RFC. The
stream is refreshed every 128 columns, and `input_block[6]` is bumped
each time. Nothing about the stream depends on the password, so there
is no timing side channel on memory access.

## Trust boundary

Argon2 is designed to burn CPU and RAM on purpose. Callers control the
DoS boundary via `timeCost`, `memoryCost`, and `parallelism`. Don't
expose those to untrusted input without bounds. Tag comparison is NOT
provided here — use a constant-time helper when verifying passwords.

## Running the tests

```bash
swift test --enable-code-coverage
```

The RFC 9106 §5.2 gold-standard vector is verified along with validation,
determinism, key/AD binding, and tag-length boundary tests.

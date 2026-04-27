# Argon2id (Swift)

From-scratch Swift implementation of **Argon2id** (RFC 9106) — the
hybrid Argon2 variant and **RFC 9106's recommended password-hashing
default**.

Depends only on our sibling `Blake2b` package for the underlying hash.

See the spec at [../../specs/KD03-argon2.md](../../specs/KD03-argon2.md).

## Usage

```swift
import Argon2id

let tag = try Argon2id.argon2id(
    password: Array("secret".utf8),
    salt: Array("0123456789abcdef".utf8),
    timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32
)

let hex = try Argon2id.argon2idHex(
    password: Array("secret".utf8),
    salt: Array("0123456789abcdef".utf8),
    timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32
)
```

## Why Argon2id is the default

| Phase                   | Addressing      | What it buys you          |
|-------------------------|-----------------|---------------------------|
| First 2 slices of pass 1| **Data-indep.** | Side-channel resistance   |
| Everything afterwards   | **Data-dep.**   | GPU/ASIC cracking cost    |

The two regimes cover each other's weaknesses. Pick `Argon2d` only for
proof-of-work where side channels don't matter; pick `Argon2i` only
when side-channel resistance is the overriding concern.

## Implementation notes

The hybrid switch is purely local to `fillSegment`: when `r == 0 &&
sl < 2` we run Argon2i's deterministic address stream; otherwise we
run Argon2d's `prev_block[0]` lookup. All other machinery (H0, H',
index_alpha, compression G, final XOR) is shared.

## Trust boundary

Argon2 is designed to burn CPU and RAM on purpose. Callers control the
DoS boundary via `timeCost`, `memoryCost`, and `parallelism`. Don't
expose those to untrusted input without bounds. Tag comparison is NOT
provided here — use a constant-time helper when verifying passwords.

## Running the tests

```bash
swift test --enable-code-coverage
```

The RFC 9106 §5.3 gold-standard vector is verified along with validation,
determinism, key/AD binding, and tag-length boundary tests.

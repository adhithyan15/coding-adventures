# argon2d

A pure-Go, from-scratch implementation of **Argon2d** (RFC 9106) --
data-dependent memory-hard password hashing.

## What is Argon2d?

Argon2d uses **data-dependent** addressing throughout every segment: the
reference block for each new block is chosen from the first 64 bits of
the previously computed block. This maximises GPU/ASIC resistance at the
cost of leaking a noisy channel through memory-access timing.

Use Argon2d only in contexts where side-channel attacks are *not* in the
threat model -- e.g. proof-of-work. For password hashing, prefer
[`argon2id`](../argon2id/).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md)
for the full algorithm walkthrough.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/argon2d"

tag, err := argon2d.Sum(
    []byte("password"),
    []byte("somesalt"),
    3,    // timeCost
    64,   // memoryCost (KiB)
    1,    // parallelism
    32,   // tagLength (bytes)
    nil,
)

hexStr, _ := argon2d.SumHex(password, salt, 3, 64, 1, 32, nil)
```

### Keyed / authenticated data

```go
tag, _ := argon2d.Sum(password, salt, 3, 64, 1, 32, &argon2d.Options{
    Key:            secret,
    AssociatedData: []byte("challenge-id"),
})
```

## API

| Function | Returns |
| -- | -- |
| `Sum(password, salt, t, m, p, T, opts) ([]byte, error)` | raw tag bytes |
| `SumHex(password, salt, t, m, p, T, opts) (string, error)` | lowercase hex |

Parameters follow RFC 9106 §3.1: `t` = time cost (passes), `m` = memory
in KiB, `p` = parallelism (lanes), `T` = tag length in bytes.

## Where this fits in the stack

- **Dependencies:** [`blake2b`](../blake2b/) (H0 and the H' extender).

## Security notes

- **Trust boundary on `memoryCost` and `tagLength`.** RFC 9106 allows
  both up to `2^32 - 1`, which translates to multi-TiB allocations. If
  either value is caller-controlled from an untrusted source, clamp it
  at the application layer before calling `Sum`.
- **Verify in constant time.** When comparing a stored tag to a freshly
  computed one, use `crypto/subtle.ConstantTimeCompare`, never
  `bytes.Equal`.

## Running the tests

```bash
go test ./... -v -cover
```

Tests include the canonical RFC 9106 §5.1 gold-standard vector, plus
16 unit tests covering validation, determinism, binding to key/AD,
tag-length variants, and multi-lane / multi-pass parameters.

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).

# argon2id

A pure-Go, from-scratch implementation of **Argon2id** (RFC 9106) -- the
RFC-recommended memory-hard password hashing function.

## What is Argon2id?

Argon2id is the "hybrid" member of the Argon2 family:

- The first half of the first pass uses **data-independent** addressing
  (Argon2i) -- resistant to side-channel attacks on memory access
  patterns.
- Everything afterwards uses **data-dependent** addressing (Argon2d) --
  maximally resistant to GPU/ASIC attackers.

Pick this variant unless you have a specific reason to prefer
[`argon2d`](../argon2d/) (proof-of-work, no side-channel threat) or
[`argon2i`](../argon2i/) (strict side-channel requirements).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md)
for the full algorithm walkthrough.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/argon2id"

tag, err := argon2id.Sum(
    []byte("correct horse battery staple"),
    []byte("somesalt"),
    3,    // timeCost
    64,   // memoryCost (KiB)
    1,    // parallelism
    32,   // tagLength (bytes)
    nil,  // Options: Key, AssociatedData, Version
)

hexStr, _ := argon2id.SumHex(password, salt, 3, 64, 1, 32, nil)
```

### Keyed / authenticated data

```go
tag, _ := argon2id.Sum(password, salt, 3, 64, 1, 32, &argon2id.Options{
    Key:            serverSecret,
    AssociatedData: []byte("user:alice"),
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
- **Used by:** the rest of the repo's password-hashing surface.

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

Tests include the canonical RFC 9106 §5.3 gold-standard vector, plus
16 unit tests covering validation, determinism, binding to key/AD,
tag-length variants, and multi-lane / multi-pass parameters.

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).

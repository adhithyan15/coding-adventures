# argon2d (Python)

Pure-Python Argon2d (RFC 9106) — the data-dependent variant of the
Argon2 memory-hard password hashing function.

Argon2d uses the previously computed block's first 64 bits to pick the
next reference block.  That tight coupling between memory contents and
access pattern is what makes Argon2d the hardest variant to attack with
GPUs or ASICs, but it also means each access pattern leaks a function of
the secret.  Pick Argon2d for contexts where side-channel attacks are
out of scope (proof-of-work, key derivation behind a memory barrier).
For password hashing, prefer
[`argon2id`](../argon2id) (RFC-recommended default).

Depends on [`coding-adventures-blake2b`](../blake2b) for the outer
BLAKE2b calls (`H0` and the variable-length `H'`).  The compression
function's inner round is an Argon2-specific modification of BLAKE2b
(with an integer multiplication term added to each `G_B` addition), so
it is inlined in this package rather than imported.

See [`code/specs/KD03-argon2.md`](../../specs/KD03-argon2.md) for the
full algorithm walk-through (the spec covers all three variants).

## Usage

```python
from coding_adventures_argon2d import argon2d, argon2d_hex

tag = argon2d(
    password=b"...",
    salt=b"some-random-salt",
    time_cost=3,
    memory_cost=65536,   # 64 MiB
    parallelism=4,
    tag_length=32,
)
```

With optional secret key (`K`) and associated data (`X`):

```python
tag = argon2d(
    password=b"...",
    salt=b"...",
    time_cost=3,
    memory_cost=65536,
    parallelism=4,
    tag_length=32,
    key=b"server-secret",
    associated_data=b"context-tag",
)
```

## API

| Function | Returns | Description |
|---|---|---|
| `argon2d(...) -> bytes` | raw bytes | `tag_length` bytes |
| `argon2d_hex(...) -> str` | lowercase hex | same computation |

Both functions share the signature:

```
argon2d(password, salt, time_cost, memory_cost, parallelism, tag_length,
        *, key=b"", associated_data=b"", version=0x13)
```

## Running the tests

```bash
uv venv
uv pip install -e ../blake2b
uv pip install -e ".[dev]"
pytest
```

The suite includes the RFC 9106 §5.1 `argon2d` vector and a spread of
parameter-edge cases.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  Argon2 sits on top of BLAKE2b (HF06); the
sibling packages `argon2i` and `argon2id` expose the other two
variants.

# argon2id (Python)

Pure-Python Argon2id (RFC 9106) — the hybrid, side-channel-resistant
variant of the Argon2 memory-hard password hashing function.

Depends on [`coding-adventures-blake2b`](../blake2b) for the outer
BLAKE2b calls (`H0` and the variable-length `H'`).  The compression
function's inner round is an Argon2-specific modification of BLAKE2b
(with an integer multiplication term added to each `G_B` addition), so
it is inlined in this package rather than imported.

See [`code/specs/KD03-argon2.md`](../../specs/KD03-argon2.md) for the
full algorithm walk-through.

## Usage

```python
from coding_adventures_argon2id import argon2id, argon2id_hex

tag = argon2id(
    password=b"correct horse battery staple",
    salt=b"some-random-salt",
    time_cost=2,
    memory_cost=19456,   # 19 MiB (OWASP 2024 interactive-login floor)
    parallelism=1,
    tag_length=32,
)
```

With optional secret key (`K`) and associated data (`X`):

```python
tag = argon2id(
    password=b"...",
    salt=b"...",
    time_cost=3,
    memory_cost=65536,
    parallelism=4,
    tag_length=32,
    key=b"server-secret",
    associated_data=b"tenant-id=42",
)
```

## API

| Function | Returns | Description |
|---|---|---|
| `argon2id(...) -> bytes` | raw bytes | `tag_length` bytes |
| `argon2id_hex(...) -> str` | lowercase hex | same computation |

Both functions share the signature:

```
argon2id(password, salt, time_cost, memory_cost, parallelism, tag_length,
         *, key=b"", associated_data=b"", version=0x13)
```

## Parameter guidance (OWASP 2024)

| Use-case | `t` | `m` (KiB) | `p` | `T` |
|---|---|---|---|---|
| Interactive login | 2 | 19 456 (19 MiB) | 1 | 32 |
| Sensitive storage | 2 | 65 536 (64 MiB) | 1 | 32 |
| Offline / maximum | 3 | 1 048 576 (1 GiB) | 4 | 32 |

Pure Python is slow — treat these numbers as a test harness, not a
production parameter choice.  For production use a native-backed
implementation (e.g. `argon2-cffi`).

## Running the tests

```bash
uv venv
uv pip install -e ../blake2b
uv pip install -e ".[dev]"
pytest
```

The suite includes the RFC 9106 §5.3 `argon2id` vector and a spread of
parameter-edge cases.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  Argon2 sits on top of BLAKE2b (HF06); the
sibling packages `argon2d` and `argon2i` expose the other two
variants.

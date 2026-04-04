# gf256-c

A Rust crate that wraps the `gf256` crate in a stable C ABI, producing a
static library (`libgf256_c.a`) suitable for compile-time linkage from
Swift, C, and C++.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## Why This Crate?

The `gf256` crate panics on undefined operations (division by zero, inverse of
zero). Panics cannot unwind through C frames — that is undefined behaviour. This
wrapper:

1. Catches panics via `std::panic::catch_unwind`.
2. Returns a sentinel value (`0xFF`) and sets a per-thread error flag.
3. Exposes `gf256_c_had_error()` for callers to check the error state.

## Build

```bash
# Debug (faster compile):
cargo build

# Release (LTO enabled, recommended for linking into Swift):
cargo build --release

# Type-check only:
cargo check
```

Output: `target/release/libgf256_c.a`

## Usage from C

```c
#include "include/gf256_c.h"

uint8_t result = gf256_c_multiply(2, 128);  // → 29

// Division by zero:
result = gf256_c_divide(42, 0);
if (gf256_c_had_error()) {
    // handle error: result is 0xFF sentinel
}
```

## API

| Function | Description |
|----------|-------------|
| `gf256_c_add(a, b)` | XOR; no error cases |
| `gf256_c_subtract(a, b)` | XOR (= add); no error cases |
| `gf256_c_multiply(a, b)` | Log/antilog multiply; `0×anything=0` |
| `gf256_c_divide(a, b)` | Division; returns 0xFF + sets error if b=0 |
| `gf256_c_power(base, exp)` | Non-negative integer power |
| `gf256_c_inverse(a)` | Multiplicative inverse; 0xFF + error if a=0 |
| `gf256_c_had_error()` | Returns 1 if last call on this thread errored |
| `gf256_c_primitive_polynomial()` | Returns 285 (0x11D) |

## Error Flag Design

The error flag is thread-local (`thread_local!` in Rust). Each OS thread has
an independent flag, cleared at the start of each `gf256_c_*` call. This
matches Swift's structured concurrency model where tasks run on OS threads.

## License

Part of the coding-adventures project.

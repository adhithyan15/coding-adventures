# polynomial-c

A Rust crate that wraps the `polynomial` crate in a stable C ABI, producing a
static library (`libpolynomial_c.a`) suitable for compile-time linkage from
Swift, C, and C++.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack.

## Why This Crate?

The `polynomial` crate uses idiomatic Rust types (`&[f64]`, `Vec<f64>`,
panics for errors) that cannot cross a C ABI boundary. This wrapper:

1. Converts raw `*const f64` + `usize` length pairs into safe Rust slices.
2. Writes results into caller-provided output buffers (no cross-FFI heap allocation).
3. Catches Rust panics via `std::panic::catch_unwind` and returns error codes.

## Build

```bash
# Debug (faster compile, unoptimised):
cargo build

# Release (LTO enabled, recommended for linking into Swift):
cargo build --release

# Just type-check without producing output:
cargo check
```

Output: `target/release/libpolynomial_c.a`

## Usage from C

```c
#include "include/polynomial_c.h"

double a[] = {1.0, 2.0, 3.0};  // 1 + 2x + 3x²
double b[] = {4.0, 5.0};       // 4 + 5x
double out[4];

// Add: worst-case output is max(a_len, b_len)
size_t n = poly_c_add(a, 3, b, 2, out, 4);
// n == 3, out = {5.0, 7.0, 3.0}  =  5 + 7x + 3x²
```

## Memory Protocol

All memory is owned by the caller. Functions write results into caller-provided
buffers and return the number of elements written.

Worst-case output sizes:

| Function | Output size |
|----------|-------------|
| `poly_c_normalize` | `len` |
| `poly_c_add` | `max(a_len, b_len)` |
| `poly_c_subtract` | `max(a_len, b_len)` |
| `poly_c_multiply` | `a_len + b_len - 1` |
| `poly_c_divide` | `dividend_len` |
| `poly_c_modulo` | `divisor_len` |
| `poly_c_gcd` | `max(a_len, b_len)` |
| `poly_c_divmod` | Two buffers: `dividend_len` + `divisor_len` |

## Error Handling

- `poly_c_divmod` returns `-1` on zero divisor, `0` on success.
- Other division functions return `0` elements on zero divisor (ambiguous;
  use `poly_c_divmod` for reliable error detection).
- All functions use `std::panic::catch_unwind` to prevent Rust panics from
  unwinding through C frames (undefined behaviour).

## License

Part of the coding-adventures project.

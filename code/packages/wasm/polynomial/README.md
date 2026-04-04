# polynomial-wasm

WebAssembly build of the [`polynomial`](../../rust/polynomial) crate, exposing all polynomial arithmetic through a plain C ABI — zero wasm-bindgen, zero JavaScript glue, zero external dependencies.

## What This Package Does

The `polynomial` crate implements polynomial arithmetic over `f64` coefficients: add, subtract, multiply, divide (long division), evaluate at a point (Horner's method), and compute the greatest common divisor (Euclidean algorithm). See the [polynomial spec](../../../specs/) for the mathematical background.

This package compiles that Rust library to a `.wasm` binary with `#[no_mangle] pub extern "C"` exports. Any WASM runtime — Wasmtime, WAMR, Wasmer, or the future WASM runtime in this repo — can load the binary and call these functions without any JavaScript or language-specific bridge.

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

Output artifact:

```
target/wasm32-unknown-unknown/release/polynomial_wasm.wasm
```

If the `wasm32-unknown-unknown` target is not installed:

```bash
rustup target add wasm32-unknown-unknown
```

## Memory Protocol

WASM linear memory is a flat byte array shared between the host (runtime) and this module. Since WASM function signatures can only pass scalar types (`i32`, `i64`, `f32`, `f64`), variable-length polynomial arrays must be passed through pointers into this shared memory.

### Step-by-step: calling `poly_add`

```
1. ptr_a = poly_alloc(3)        // allocate 3 f64 slots = 24 bytes
2. write [1.0, 2.0, 3.0] into memory at ptr_a   // host writes into module memory
3. ptr_b = poly_alloc(2)        // allocate 2 f64 slots = 16 bytes
4. write [4.0, 5.0] into memory at ptr_b
5. ptr_result = poly_add(ptr_a, 3, ptr_b, 2)
6. len = poly_last_result_len() // → 3  (result: [5.0, 7.0, 3.0])
7. read len f64s from ptr_result
8. poly_dealloc(ptr_result, len) // host frees result
9. poly_dealloc(ptr_a, 3)        // host frees input a
10. poly_dealloc(ptr_b, 2)       // host frees input b
```

### Coefficient layout

Polynomials are stored **lowest degree first** (little-endian):

```
[3.0, 0.0, 2.0]  →  3 + 0·x + 2·x²  =  3 + 2x²
index 0 = degree-0 coefficient (constant term)
index 1 = degree-1 coefficient (x term)
index 2 = degree-2 coefficient (x² term)
```

### Ownership rules

- Every pointer returned by this module is **owned by the caller**.
- The caller MUST call `poly_dealloc(ptr, len)` after reading the result.
- Failing to dealloc causes a memory leak in WASM linear memory. (WASM linear
  memory does not shrink; it can only grow, up to the configured maximum.)

### The zero polynomial

The zero polynomial is represented as an empty array (length 0). Functions may
return a null pointer with `poly_last_result_len() == 0` to indicate the zero
polynomial. Hosts must handle null pointers gracefully (do not read from null).

## Exported Functions

### Memory management

| Function | Signature | Description |
|----------|-----------|-------------|
| `poly_alloc` | `(len: u32) → *mut f64` | Allocate `len` f64 slots in module memory |
| `poly_dealloc` | `(ptr: *mut f64, len: u32)` | Free previously allocated memory |
| `poly_last_result_len` | `() → u32` | Length of the most recent result array |
| `poly_had_error` | `() → u32` | 1 if the last operation panicked, else 0 |

### Polynomial operations

| Function | Signature | Description |
|----------|-----------|-------------|
| `poly_normalize` | `(ptr, len) → *mut f64` | Strip trailing near-zero coefficients |
| `poly_degree` | `(ptr, len) → u32` | Degree of the polynomial |
| `poly_add` | `(a_ptr, a_len, b_ptr, b_len) → *mut f64` | Add two polynomials |
| `poly_subtract` | `(a_ptr, a_len, b_ptr, b_len) → *mut f64` | Subtract b from a |
| `poly_multiply` | `(a_ptr, a_len, b_ptr, b_len) → *mut f64` | Multiply two polynomials |
| `poly_divide` | `(a_ptr, a_len, b_ptr, b_len) → *mut f64` | Quotient of a/b |
| `poly_modulo` | `(a_ptr, a_len, b_ptr, b_len) → *mut f64` | Remainder of a/b |
| `poly_evaluate` | `(ptr, len, x: f64) → f64` | Evaluate polynomial at x (Horner's) |
| `poly_gcd` | `(a_ptr, a_len, b_ptr, b_len) → *mut f64` | GCD of two polynomials |

### Divmod protocol

Because `divmod` returns two arrays, it uses a separate protocol:

| Function | Signature | Description |
|----------|-----------|-------------|
| `poly_divmod` | `(div_ptr, div_len, sor_ptr, sor_len)` | Compute and cache quotient+remainder |
| `poly_divmod_quotient_ptr` | `() → *mut f64` | Pointer to cached quotient |
| `poly_divmod_quotient_len` | `() → u32` | Length of cached quotient |
| `poly_divmod_remainder_ptr` | `() → *mut f64` | Pointer to cached remainder |
| `poly_divmod_remainder_len` | `() → u32` | Length of cached remainder |

After calling `poly_divmod`, free both arrays:

```
poly_divmod(div_ptr, div_len, sor_ptr, sor_len);
if poly_had_error() == 0 {
    q_ptr = poly_divmod_quotient_ptr()
    q_len = poly_divmod_quotient_len()
    r_ptr = poly_divmod_remainder_ptr()
    r_len = poly_divmod_remainder_len()
    // read q_len f64s from q_ptr
    // read r_len f64s from r_ptr
    poly_dealloc(q_ptr, q_len)
    poly_dealloc(r_ptr, r_len)
}
```

## Error Handling

Operations that can panic (divide, modulo, gcd, divmod — all of which assert the
divisor is non-zero) are wrapped in `std::panic::catch_unwind`. If they panic:

- `poly_had_error()` returns 1.
- The return value is null (for pointer-returning functions) or 0 (for scalars).

Always check `poly_had_error()` after any operation on potentially-zero polynomials.

## Where This Fits in the Stack

```
code/specs/          — mathematical specification
code/packages/rust/polynomial/   — pure Rust implementation (this is the dependency)
code/packages/wasm/polynomial/   — this package: WASM artifact
                                   (future) code/packages/*/wasm-runtime/ — runtimes that load this
```

Future WASM runtimes in any language (Python, Ruby, Go, TypeScript, Elixir) can
load `polynomial_wasm.wasm` and call the same arithmetic without reimplementing it.
This is the key architectural goal: one implementation, many consumers.

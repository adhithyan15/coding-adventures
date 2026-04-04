# gf256-wasm

WebAssembly build of the [`gf256`](../../rust/gf256) crate, exposing GF(2^8) arithmetic through a plain C ABI — zero wasm-bindgen, zero JavaScript glue, zero external dependencies.

## What This Package Does

The `gf256` crate implements Galois Field GF(2^8) arithmetic: add, subtract, multiply, divide, power, and multiplicative inverse. All elements are single bytes (0–255). See the [gf256 spec](../../../specs/) for the mathematical background.

This package compiles that Rust library to a `.wasm` binary with `#[no_mangle] pub extern "C"` exports. Any WASM runtime can load the binary and call these functions without any JavaScript or language-specific bridge.

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

Output artifact:

```
target/wasm32-unknown-unknown/release/gf256_wasm.wasm
```

If the `wasm32-unknown-unknown` target is not installed:

```bash
rustup target add wasm32-unknown-unknown
```

## No Memory Protocol

Unlike the polynomial WASM module, GF(256) operations are purely scalar. Every
function takes one or two bytes (passed as `u32`) and returns one byte (as `u32`).
WASM can pass these directly. No heap allocation, no `alloc`/`dealloc`, no pointers.

## Value Types

WASM has no native `u8` type — the smallest integer is `i32`. All GF(256) byte
values are passed and returned as `u32`. The module reads only the low 8 bits of
each argument; the caller should ensure values are in the range `[0, 255]`.

## Exported Functions

### Arithmetic

| Function | Signature | Description |
|----------|-----------|-------------|
| `gf256_add` | `(a: u32, b: u32) → u32` | Add: `a XOR b` |
| `gf256_subtract` | `(a: u32, b: u32) → u32` | Subtract: `a XOR b` (same as add in GF(2)) |
| `gf256_multiply` | `(a: u32, b: u32) → u32` | Multiply using log/antilog tables |
| `gf256_divide` | `(a: u32, b: u32) → u32` | Divide `a/b`; returns 0xFF on div-by-zero |
| `gf256_power` | `(base: u32, exp: u32) → u32` | `base^exp` in GF(256) |
| `gf256_inverse` | `(a: u32) → u32` | Multiplicative inverse; returns 0xFF if `a=0` |

### Constants

| Function | Signature | Returns |
|----------|-----------|---------|
| `gf256_zero` | `() → u32` | `0` — additive identity |
| `gf256_one` | `() → u32` | `1` — multiplicative identity |
| `gf256_primitive_polynomial` | `() → u32` | `0x11D` (285) — the irreducible polynomial |

### Error handling

| Function | Signature | Description |
|----------|-----------|-------------|
| `gf256_had_error` | `() → u32` | `1` if the last op panicked, else `0` |

## Error Handling

Two operations can panic on invalid input:

- `gf256_divide(a, 0)` — division by zero
- `gf256_inverse(0)` — zero has no multiplicative inverse

In both cases, the module catches the panic (using `catch_unwind`), returns `0xFF`
as a sentinel value, and sets the error flag. Always check `gf256_had_error()` after
these operations:

```
let inv = gf256_inverse(some_value);
if gf256_had_error() != 0 {
    // error: some_value was 0; inv is 0xFF (sentinel, not a valid result)
}
```

The error flag is cleared at the start of each operation, so it always reflects
only the most recent call.

## Why `0xFF` as the Error Sentinel?

`0xFF` (255) is a valid GF(256) element in general arithmetic, but it is chosen
as the sentinel because in practice — for the Reed-Solomon and AES use cases —
the caller always knows when it is requesting inverse or divide, and should always
check the error flag in those cases. The sentinel is distinct from `0`, which
could legitimately be a result of `gf256_multiply(0, x)`.

## Where This Fits in the Stack

```
code/specs/        — mathematical specification
code/packages/rust/gf256/   — pure Rust implementation (this is the dependency)
code/packages/wasm/gf256/   — this package: WASM artifact
                               (future) code/packages/*/wasm-runtime/ — runtimes that load this
```

Future WASM runtimes in any language (Python, Ruby, Go, TypeScript, Elixir) can
load `gf256_wasm.wasm` and call the same GF(256) arithmetic without reimplementing
it. This is the key architectural goal: one implementation, many consumers.

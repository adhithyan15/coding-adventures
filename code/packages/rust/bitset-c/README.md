# bitset-c

A Rust crate that wraps the `bitset` crate in a stable C ABI so C, C++, Swift,
and .NET callers can reuse the Rust implementation through a thin FFI layer.

## What This Crate Exports

- Opaque `bitset_c` handles for native callers
- Constructors from size, `u128` split into two `u64`s, and binary strings
- In-place single-bit operations: `set`, `clear`, `test`, `toggle`
- Bulk operations that return fresh handles: AND, OR, XOR, NOT, AND-NOT
- Query helpers: `len`, `capacity`, `popcount`, `any`, `all`, `none`, `is_empty`
- Conversion helpers: `to_u64`, equality, and thread-local error inspection

## Error Model

The ABI catches Rust panics before they can unwind through a C frame, because
that would be undefined behaviour. Functions that fail return a sentinel
(`NULL`, `0`, or `false`) and store per-thread error information retrievable via:

- `bitset_c_had_error()`
- `bitset_c_last_error_code()`
- `bitset_c_last_error_message()`

## Build

```bash
cargo build --release
```

This produces:

- `target/release/libbitset_c.dylib` on macOS
- `target/release/libbitset_c.so` on Linux
- `target/release/bitset_c.dll` on Windows

The package also emits a static library (`staticlib`) for compile-time linkage
scenarios.

## Header

The public C declarations live in `include/bitset_c.h`.

## Development

```bash
# Run tests
bash BUILD
```

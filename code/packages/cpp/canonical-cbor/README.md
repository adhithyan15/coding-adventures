# canonical-cbor (C++)

A deterministic CBOR (RFC 8949) codec, **header-only** in pure ISO C++17
(namespace `ca::canonical_cbor`). A faithful port of the Rust
[`canonical-cbor`](../../rust/canonical-cbor) crate.

## What it does

Encodes and decodes CBOR values in a **canonical** (deterministic) profile so
that `decode(encode(v))` round-trips and `encode(v)` is the same bytes on every
platform.

**Profile** (RFC 8949 §4.2.3, "length-first map key ordering"): definite length
only, smallest-form integers, map keys sorted length-first then bytewise, no
floats, opaque tags, no `undefined`.

## API

- `CborValue` — a fully value-semantic type (copyable, `==`-comparable) built
  from `std::vector` / `std::string`, with factories `unsigned_val`, `negative`,
  `boolean_val`, `null`, `byte_string`, `text_string`, `arr`, `mapping`, `tag`.
- `encode(v)` → `std::vector<std::uint8_t>` of canonical bytes.
- `decode(bytes)` → `CborValue`; throws `CborException` (carrying a `CborError`)
  on any violation of the canonical profile.

## Design notes

- **Value semantics, no pointers.** Recursion goes through `std::vector`; a Tag
  keeps its number in `u` and its single inner value as `array[0]`. The type is
  trivially copyable and comparable — no manual memory management.
- **Exceptions, not `Result`.** `decode` throws `CborException::error()` where
  Rust returns `Err`; `encode` returns a `std::vector` where Rust returns `Vec`.
- **Security-hardened decoder** (matching the Rust crate): recursion depth is
  capped (`MAX_DECODE_DEPTH`), declared lengths are bounded by the remaining
  input before allocating, and cursor arithmetic is overflow-checked.

## Usage

```cpp
#include "canonical_cbor.hpp"
using namespace ca::canonical_cbor;

auto m = CborValue::mapping({{CborValue::text_string("count"),
                             CborValue::unsigned_val(42)}});
std::vector<std::uint8_t> bytes = encode(m);
CborValue back = decode(bytes);   // throws CborException on bad input
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.

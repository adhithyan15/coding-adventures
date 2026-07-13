# garbage-collector (C++)

A language-agnostic **mark-and-sweep garbage collector**, **header-only** in
pure ISO C++17 (namespace `ca::garbage_collector`). A faithful port of the Rust
[`garbage-collector`](../../rust/garbage-collector) crate.

## What it does

A tracing GC finds and reclaims unreachable objects: **mark** from the roots,
**sweep** the unmarked, **reset** marks on survivors. Reference cycles are
handled correctly. Heap objects (`ConsCell`, `Symbol`, `LispClosure`) derive
from `HeapObject` and report the heap addresses they reference. Roots are
`Value`s; only address-like values are followed. Addresses increase
monotonically from `0x10000` and are never reused.

## API

- `HeapObject` base + `ConsCell` / `Symbol` / `LispClosure` (each overrides
  `references()` / `type_name()`).
- `MarkAndSweepGC` (implements the `GarbageCollector` interface): `allocate`
  (takes a `std::unique_ptr<HeapObject>` → address), `deref`, `collect(roots)`,
  `heap_size`, `is_valid_address`, `stats`.
- `Value` with factories `integer` / `address` / `str` / `boolean` / `nil` /
  `list`.
- `SymbolTable`: `intern`, `lookup` → `std::optional<size_t>`, `all_symbols`.

## Design notes

- **Virtual dispatch + `unique_ptr` ownership.** The Rust `HeapObject` /
  `GarbageCollector` traits become abstract base classes; the GC owns objects via
  `std::unique_ptr` in a `std::unordered_map<size_t, …>` keyed by address.
- **`std::variant` roots**, `std::optional` lookups — Rust `enum` / `Option`.
- **Header-only.** `#include "garbage_collector.hpp"` and go.

## Usage

```cpp
#include "garbage_collector.hpp"
using namespace ca::garbage_collector;

MarkAndSweepGC gc;
auto a1 = gc.allocate(std::make_unique<ConsCell>(42, -1));
gc.allocate(std::make_unique<Symbol>("unreachable"));

std::size_t freed = gc.collect({Value::address(a1)});  // 1
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.

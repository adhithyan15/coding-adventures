# static-vector

A fixed-capacity, **header-only** `ca::static_vector<T, N>` in pure ISO C++17. It
behaves like a small `std::vector` whose capacity is fixed at `N` and whose
storage lives *inside* the object — so it never touches the heap.

It is a sample package for the repo's C/C++ multi-compiler lane: it compiles and
runs under **GCC, Clang, and MSVC** with strict ISO-conformance flags
(`-pedantic-errors` / `/permissive-`, warnings-as-errors), via the shared
[`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "static_vector.hpp"

ca::static_vector<int, 3> v;   // capacity 3, no allocation
v.push_back(10);
v.push_back(20);

int first = v[0];              // 10
int sum = 0;
for (int x : v) sum += x;      // range-for over the live elements
```

### API

| Member | Purpose |
| --- | --- |
| `push_back(v)` | append; returns `false` (stores nothing) when full |
| `pop_back()` | drop the last element (no-op if empty) |
| `operator[](i)` | unchecked access |
| `at(i)` | checked access — throws `std::out_of_range` |
| `size()` / `capacity()` | live count / fixed `N` |
| `empty()` / `full()` | state predicates |
| `begin()` / `end()` | iterators over the live range (range-for works) |
| `clear()` | reset to empty |

`push_back` returns a bool rather than throwing, so it is usable in
no-exceptions builds; `at()` throws like `std::vector` for callers who want
checked access.

## Development

```bash
# Compile + run the tests under every C++ compiler present (g++, clang++; MSVC
# on Windows), each with strict ISO-conformance flags:
sh BUILD
```

## Where it fits

Part of the C/C++ multi-compiler lane — see
[`code/specs/CCPP01-c-cpp-iso-multicompiler-lane.md`](../../../specs/CCPP01-c-cpp-iso-multicompiler-lane.md)
and the shared [`iso-harness`](../../c/iso-harness/README.md).

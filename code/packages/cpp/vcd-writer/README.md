# vcd-writer (C++)

A streaming **Value Change Dump (VCD)** writer in pure ISO C++17, header-only, in
namespace `ca`. A faithful port of the Rust `vcd-writer` crate.

VCD (IEEE 1364-2005 §18) is the text format every waveform viewer reads.
`ca::VcdWriter` builds a complete VCD document in two phases:

1. **Header** — `open_scope` / `declare` / `close_scope` / `end_definitions`.
   Each `declare` returns a compact identifier (bijective base-94 over `!`..`~`).
2. **Body** — `time(t)` then `value_change(id, value)`.

Scalars emit a single bit, vectors a `b<binary>`, and `"real"` variables an
`r<n>`; an unchanged value is skipped.

## API

```cpp
#include "vcd_writer.hpp"

ca::VcdWriter w("1ps");
w.open_scope("adder");
std::string a   = w.declare("a",   4, "wire");
std::string sum = w.declare("sum", 5, "wire");
w.close_scope();
w.end_definitions();

w.value_change_at(0,  a, 0);
w.value_change_at(10, a, 3);

std::string text = w.finish();   // or w.text() to borrow
```

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```

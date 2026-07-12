# vcd-writer (C)

A streaming **Value Change Dump (VCD)** writer in pure ISO C17. A faithful port
of the Rust `vcd-writer` crate.

VCD (IEEE 1364-2005 §18) is the text format every waveform viewer (GTKWave,
Surfer, ModelSim, …) reads. The writer builds a complete VCD document in two
phases:

1. **Header** — `vcd_open_scope` / `vcd_declare` / `vcd_close_scope` /
   `vcd_end_definitions`. Each `declare` returns a compact printable-ASCII
   identifier (bijective base-94 over `!`..`~`).
2. **Body** — `vcd_time(t)` then `vcd_value_change(id, value)` pairs.

Scalars emit a single bit, vectors a `b<binary>` value, and `"real"` variables
an `r<n>` value; an unchanged value is skipped.

## API

```c
#include "vcd_writer.h"

VcdWriter *w = vcd_new("1ps");
vcd_open_scope(w, "adder");
char a[16], sum[16];
vcd_declare(w, "a",   4, "wire", a,   sizeof a);
vcd_declare(w, "sum", 5, "wire", sum, sizeof sum);
vcd_close_scope(w);
vcd_end_definitions(w);

vcd_value_change_at(w, 0,  a, 0);
vcd_value_change_at(w, 10, a, 3);

const char *text = vcd_text(w);   /* borrowed until vcd_free */
vcd_free(w);
```

`vcd_declare` writes the identifier into a caller buffer (16 bytes always
suffices). Output accumulates in an internal buffer; `vcd_ok` reports whether a
previous allocation failed.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```

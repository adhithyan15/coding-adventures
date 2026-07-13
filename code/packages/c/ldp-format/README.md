# ldp-format (C)

A **versioned binary codec for `.ldp` profile artefacts** in pure ISO C17. A
faithful port of the Rust [`ldp-format`](../../rust/ldp-format) crate — the
read/write of the LANG22 "profile artefact" format, a compact on-disk record of
a JIT/AOT profiler's observations (call counts, per-instruction type feedback,
promotion state, …).

## Format (version 1, little-endian)

A fixed 32-byte header (magic `LDP\0`, version, 16-byte NUL-padded ASCII
language, flags, record count, reserved), a **deduplicated string table** (every
record string is a `u32` index into it — a name used many times is stored once),
then module → function → instruction records.

## Determinism

`ldp_write` produces byte-identical output for equal input: the string table is
built in **first-occurrence order** during a pre-walk, so the same file always
serialises to the same bytes.

## Safety

`ldp_read` treats its input as **untrusted**. Every field goes through a
bounds-checked cursor that returns `LDP_ERR_UNEXPECTED_EOF` the moment a read
would run past the buffer, string indices are range-checked
(`LDP_ERR_BAD_STRING_INDEX`), and enum bytes are validated. Nested arrays grow
**incrementally** as elements are read, so a corrupt record/string count can
never drive a huge speculative allocation (an improvement over the Rust
original's `Vec::with_capacity(count)`). All growable buffers guard `size_t`
overflow. Verified clean under ASan + UBSan and the macOS `leaks` tool (0 leaks),
including a fuzz sweep over every truncation and single-byte corruption.

## API

```c
#include "ldp_format.h"

LdpFile file = { .version_major = 1, .language = "twig", /* … */ };
uint8_t *bytes; size_t len;
ldp_write(&file, &bytes, &len);          /* serialise */

LdpFile *restored;
ldp_read(bytes, len, &restored);         /* parse (owns its allocations) */
ldp_file_free(restored);
free(bytes);
```

- `ldp_write` / `ldp_read` — serialise / parse (status-returning).
- `ldp_file_free` — free an owned (read-produced) file.
- `ldp_file_equal` — deep structural equality (for round-trip checks).

Strings are NUL-terminated (the format's names/opcodes are text; embedded NULs
are not modelled).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.

# ldp-format (C++)

A **versioned binary codec for `.ldp` profile artefacts**, header-only, ISO
C++17. A faithful port of the Rust [`ldp-format`](../../rust/ldp-format) crate,
in namespace `ca::ldp_format` — read/write of the LANG22 "profile artefact"
binary format (a compact on-disk record of a JIT/AOT profiler's observations).

## Format (version 1, little-endian)

A fixed 32-byte header (magic `LDP\0`, version, 16-byte NUL-padded ASCII
language, flags, record count, reserved), a **deduplicated string table** (every
record string is a `u32` index into it), then module → function → instruction
records.

## Determinism & safety

`write` is deterministic — the string table is built in first-occurrence order,
so equal input always serialises to identical bytes. `read` treats its input as
untrusted: every field is bounds-checked (throwing `Error` on a truncated buffer
or an out-of-range string index / bad enum byte), and nested vectors grow
incrementally as elements are read, so a corrupt count cannot pre-allocate.
Verified clean under ASan + UBSan.

## API

```cpp
#include "ldp_format.hpp"
namespace ldp = ca::ldp_format;

ldp::LdpFile file;
file.header.language = "twig";
// … populate file.modules …

std::vector<std::uint8_t> bytes = ldp::write(file);   // throws ldp::Error
ldp::LdpFile restored = ldp::read(bytes);
// restored == file
```

The data model is plain value structs (`LdpFile`, `Header`, `ModuleRecord`,
`FunctionRecord`, `InstructionRecord`, `TypeSeen`) with `operator==`, so
round-trips compare directly.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.

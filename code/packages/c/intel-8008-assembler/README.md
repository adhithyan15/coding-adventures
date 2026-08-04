# intel-8008-assembler (C)

A **two-pass Intel 8008 assembler** in pure ISO C17. A faithful port of the Rust
[`intel-8008-assembler`](../../rust/intel-8008-assembler) crate: it turns Intel
8008 assembly *text* into raw machine-code bytes. A companion to the ported
[`intel4004-encoder`](../intel4004-encoder).

## Two passes

Assembling needs two passes because of **forward references** — `JMP loop_end`
can appear before `loop_end:` is defined:

- **Pass 1** walks every line, tracks a program counter, and records each
  label's address in a symbol table (advancing the PC by each instruction's
  encoded size, or setting it from `ORG`).
- **Pass 2** walks again and encodes every instruction, now that all label
  addresses are known. `ORG` pads forward with `0xFF` (erased-ROM state).

## ISA coverage

Fixed 1-byte ops (returns, rotations, `HLT`), `MOV`/`INR`/`DCR`/`IN`/`OUT`/`RST`,
ALU-register (`ADD`…`CMP`), ALU-immediate + `MVI` (2 bytes), and 3-byte
jumps/calls (`JMP`/`CAL` + all conditional variants). Operands may be decimal or
`0x` hex literals, `$` (the current PC), label references, or `hi(sym)`/`lo(sym)`
byte extractions of a 14-bit address.

## API

```c
#include "intel_8008_assembler.h"

char err[128];
uint8_t *bytes; size_t len;
if (intel8008_assemble("    ORG 0x0000\n_start:\n    MVI  B, 0\n    HLT\n",
                       &bytes, &len, err, sizeof err) == INTEL8008_OK) {
    /* bytes = {0x06, 0x00, 0xFF} */
    free(bytes);
}
```

- `intel8008_assemble` — the two-pass entry point (status + malloc'd bytes).
- `intel8008_instruction_size` / `intel8008_encode_instruction` — the lower-level
  pieces (useful for testing), plus an opaque `Intel8008Symbols` table
  (`_new`/`_free`/`_set`/`_get`).

Where the Rust crate returns `Result`, this port returns an `Intel8008Status`
and writes a diagnostic into the caller's `errbuf`.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.

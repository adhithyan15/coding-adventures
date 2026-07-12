# intel-4004-assembler (C)

A two-pass assembler for the **Intel 4004** (the first commercial
microprocessor, 1971), in pure ISO C17. A faithful port of the Rust
`intel-4004-assembler` crate.

It turns 4004 assembly text into a byte array of machine code:

- **Pass 1** walks the lines building a symbol table (`label -> program
  counter`), honouring `ORG` to set the origin.
- **Pass 2** encodes each instruction, padding with zeros for forward `ORG`s.

A line is `[label:] [mnemonic [operands]] [; comment]`. Mnemonics are
case-insensitive; operands are comma-separated. Registers are `Rn`, register
pairs `Pn`, numbers decimal or `0x`-hex, and any bare identifier is looked up as
a symbol.

## API

```c
#include "intel_4004_assembler.h"

uint8_t *code = NULL;
size_t   len  = 0;
char     err[128];

I4004Status st = i4004_assemble("ORG 0x000\nLDM 5\nXCH R2\nHLT\n",
                                &code, &len, err, sizeof err);
/* st == I4004_OK; code = {0xD5, 0xB2, 0x01}, len = 3 */
free(code);
```

On an assembly error `i4004_assemble` returns `I4004_ERROR` and writes a message
into `err`; on success it writes a malloc'd buffer to `*out` (caller frees) and
its length to `*out_len`.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```

# intel-4004-assembler (C++)

A two-pass assembler for the **Intel 4004** (the first commercial
microprocessor, 1971), in pure ISO C++17, header-only, in namespace
`ca::intel4004`. A faithful port of the Rust `intel-4004-assembler` crate.

It turns 4004 assembly text into machine code:

- **Pass 1** builds a symbol table (`label -> program counter`, honouring `ORG`).
- **Pass 2** encodes each instruction, padding with zeros for forward `ORG`s.

A line is `[label:] [mnemonic [operands]] [; comment]`. Mnemonics are
case-insensitive; operands comma-separated. Registers `Rn`, register pairs `Pn`,
numbers decimal or `0x`-hex, and bare identifiers are symbols.

## API

```cpp
#include "intel_4004_assembler.hpp"

std::vector<std::uint8_t> code =
    ca::intel4004::assemble("ORG 0x000\nLDM 5\nXCH R2\nHLT\n");
// code == {0xD5, 0xB2, 0x01}
```

`assemble` returns a `std::vector<std::uint8_t>` or throws
`ca::intel4004::AssemblerError` on any error (unknown mnemonic, bad operand,
unknown symbol, wrong operand count).

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```

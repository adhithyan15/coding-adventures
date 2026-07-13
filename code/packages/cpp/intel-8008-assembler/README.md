# intel-8008-assembler (C++)

A **two-pass Intel 8008 assembler**, header-only, ISO C++17. A faithful port of
the Rust [`intel-8008-assembler`](../../rust/intel-8008-assembler) crate, in
namespace `ca::intel8008_assembler`: it turns Intel 8008 assembly *text* into raw
machine-code bytes.

## Two passes

Assembling needs two passes because of **forward references** — `JMP loop_end`
can appear before `loop_end:` is defined:

- **Pass 1** builds the symbol table (label → address).
- **Pass 2** encodes every instruction, resolving all references; `ORG` pads
  forward with `0xFF`.

## ISA coverage

Fixed 1-byte ops (returns, rotations, `HLT`), `MOV`/`INR`/`DCR`/`IN`/`OUT`/`RST`,
ALU-register (`ADD`…`CMP`), ALU-immediate + `MVI`, and 3-byte jumps/calls.
Operands may be decimal / `0x` hex literals, `$` (the current PC), label
references, or `hi(sym)`/`lo(sym)` byte extractions of a 14-bit address.

## API

```cpp
#include "intel_8008_assembler.hpp"
namespace a8 = ca::intel8008_assembler;

std::vector<std::uint8_t> bytes =
    a8::assemble("    ORG 0x0000\n_start:\n    MVI  B, 0\n    HLT\n");
// bytes == {0x06, 0x00, 0xFF}
```

- `assemble(text)` — the two-pass entry point (throws `AssemblerError`).
- `instruction_size(mnemonic)` and `encode_instruction(mnemonic, operands,
  symbols, pc)` — the lower-level pieces; `Symbols` is a `std::map<std::string,
  std::size_t>`.

Where the Rust crate returns `Result`, this port throws
`ca::intel8008_assembler::AssemblerError` (a `std::runtime_error`).

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.

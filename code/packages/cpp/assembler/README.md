# assembler (C++)

An ARM assembly parser and binary encoder, **header-only** in pure ISO C++17
(namespace `ca::assembler`). A faithful port of the Rust
[`assembler`](../../rust/assembler) crate.

## What it does

Parses a subset of ARM assembly text into structured instructions, then encodes
each into its 32-bit ARM machine-code word. Supported mnemonics: `MOV(S)`,
`ADD(S)`, `SUB(S)`, `AND(S)`, `ORR(S)`, `EOR(S)`, `RSB(S)`, `CMP`, `LDR`, `STR`,
`NOP`, and labels (`name:`).

## API

- `Assembler::parse(source)` → `std::vector<ArmInstruction>` (a
  `std::variant<DataProcessing, Load, Store, Nop, Label>`); labels are recorded
  in the public `labels` map.
- `Assembler::encode(instrs)` → `std::vector<std::uint32_t>` (labels emit
  nothing).
- Both throw `AssemblerError` (a `std::runtime_error`) whose `what()` matches the
  Rust `Display` text on any error.

## Design notes

- **Exceptions + variant.** Rust's `Result`/`AssemblerError` becomes throwing
  `AssemblerError`; the Rust `ArmInstruction` enum becomes a `std::variant`, and
  `Option<u32>` becomes `std::optional`.
- **Header-only.** `#include "assembler.hpp"` and go.

## Usage

```cpp
#include "assembler.hpp"
using namespace ca::assembler;

Assembler a;
auto instrs = a.parse("MOV R0, #42\nADD R2, R0, R1");
auto binary = a.encode(instrs);        // binary[0] == 0xE3A0002A
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.

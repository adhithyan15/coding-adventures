# arm1-simulator (C++)

A **behavioral simulator for the ARM1** (1985) — the first ARM chip — header-only,
ISO C++17. A faithful port of the Rust [`arm1-simulator`](../../rust/arm1-simulator)
crate, in namespace `ca::arm1_simulator`.

## What it models

The complete ARMv1 instruction set: 16 data-processing ops through the inline
barrel shifter (LSL/LSR/ASR/ROR/RRX), load/store, block transfer (LDM/STM, four
stacking modes), branch (B/BL), SWI, conditional execution on every instruction
(16 codes), and 4 processor modes with banked registers plus ARMv1's shared
PC+status register (R15). Each executed instruction yields a `Trace`
(before/after registers and flags, memory reads/writes, disassembly).

## API

```cpp
#include "arm1_simulator.hpp"
namespace a1 = ca::arm1_simulator;

a1::ARM1 cpu(4096);
cpu.load_program_words({a1::encode_mov_imm(a1::COND_AL, 0, 42),
                        a1::encode_halt()}, 0);
auto traces = cpu.run(100);           // cpu.read_register(0) == 42
```

- `ARM1` (`reset`, register/flag/mode/memory accessors, `step`, `run` →
  `std::vector<Trace>`).
- Free functions `evaluate_condition`, `barrel_shift`, `decode_immediate`,
  `alu_execute`, `decode`, `DecodedInstruction::disassemble`, and the `encode_*`
  helpers. RAII throughout (`std::vector` memory, `std::string` mnemonics).

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan.

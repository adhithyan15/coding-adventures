# intel4004-simulator (C++)

A **behavioral simulator for the Intel 4004** (1971) — the world's first
commercial single-chip microprocessor — header-only, ISO C++17. A faithful port
of the Rust [`intel4004-simulator`](../../rust/intel4004-simulator) crate, in
namespace `ca::intel4004_simulator`. Pairs with the ported
[`intel4004-encoder`](../intel4004-encoder).

## What it models

The 4004 is natively **4-bit** and an **accumulator machine** — operations
funnel through a single accumulator. This port executes 4004 machine code
directly: 16 registers (8 pairs) and the carry flag, byte-addressable ROM, data
RAM (4 banks × 4 registers × 16 characters) with 4 status nibbles per register,
per-bank output ports, the ROM I/O port, and the 3-level hardware call stack
(nesting a 4th call wraps, losing the oldest return address). All 46
instructions are covered. Each executed instruction yields a `Trace` (address,
raw bytes, mnemonic, before/after accumulator + carry, `std::optional` second
byte).

## API

```cpp
#include "intel4004_simulator.hpp"
namespace i4 = ca::intel4004_simulator;

i4::Simulator s;
// LDM 1; XCH R0; LDM 2; ADD R0; XCH R1; HLT  ->  R1 = 3
auto traces = s.run({i4::encode_ldm(1), i4::encode_xch(0), i4::encode_ldm(2),
                     i4::encode_add(0), i4::encode_xch(1), i4::encode_hlt()},
                    100);
// s.register_at(1) == 3
```

- `run(program, max_steps)` returns the vector of `Trace`s; `step()` executes one
  instruction (throwing `std::runtime_error` if the CPU has already halted,
  mirroring the Rust `step()` precondition).
- State accessors (accumulator, carry, registers, RAM/status/output, banks,
  stack), `reset`, `load_program`.
- `encode_*` free functions build machine code; two-byte forms return a
  `std::pair<uint8_t, uint8_t>`.

Every ROM read is bounds-checked (a runaway program counter reads NOP rather
than out of bounds). Verified clean under ASan + UBSan.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.

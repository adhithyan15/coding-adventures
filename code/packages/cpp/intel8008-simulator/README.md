# intel8008-simulator (C++)

A **behavioral simulator for the Intel 8008** (1972) — the world's first 8-bit
microprocessor — header-only, ISO C++17. A faithful port of the Rust
[`intel8008-simulator`](../../rust/intel8008-simulator) crate, in namespace
`ca::intel8008_simulator`. Completes the repo's Intel 8008 trio alongside the
ported [`intel8008-encoder`](../intel8008-encoder).

## What it models

Executes 8008 machine code directly: registers A/B/C/D/E/H/L, the M
pseudo-register (memory at `[H:L]`), four condition flags (carry/zero/sign/
parity), a 16 KiB address space, and the 8008's 8-level push-down call stack
(`stack[0]` is the live program counter). The full instruction set is covered.
Each executed instruction yields a `Trace` (address, raw bytes, mnemonic,
before/after accumulator + flags, optional memory access).

## API

```cpp
#include "intel8008_simulator.hpp"
namespace i8 = ca::intel8008_simulator;

i8::Simulator s;
// MVI B,1; MVI A,2; ADD B; HLT
auto traces = s.run({0x06, 0x01, 0x3E, 0x02, 0x80, 0x76}, 100);
// s.a() == 3
```

- `run(program, max_steps)` returns the vector of `Trace`s; `step()` executes one
  instruction (throwing `std::runtime_error` on halt / unknown opcode).
- Register/flag accessors, `set_input_port` / `get_output_port`, `reset`,
  `load_program`.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.

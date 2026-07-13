# clr-simulator (C++)

A **CLR (.NET Common Language Runtime) bytecode simulator**, header-only, ISO
C++17. A faithful port of the Rust [`clr-simulator`](../../rust/clr-simulator)
crate, in namespace `ca::clr_simulator`: a type-inferring, stack-based virtual
machine for a subset of Microsoft's CIL (the CLR's bytecode).

## What makes the CLR interesting

Unlike the JVM's typed opcodes, the CLR **infers operand types from the stack** —
one `add` works for every numeric type. Values are 32-bit integers or object
references (`null`, or a heap index into an `object[]`), modelled here with
`std::optional`. Enough of the ISA is implemented to run boxing, object arrays,
method calls, and conditional branches.

## Design

- `Value` — an int or an object reference (`std::optional<std::size_t>` heap
  index). `as_int()` throws `Error(ExpectedInt)` on a reference; `as_cmp_int()`
  and `is_truthy()` mirror the Rust helpers.
- `Slot = std::optional<Value>` — an unset slot is distinct from a null value.
- `Simulator` — the machine: shared operand stack + heap, per-method locals/args
  saved across `call`/`ret` frames.
- `Error : std::runtime_error` with an `ErrorKind kind()` — where the Rust crate
  panics, this throws. Untrusted bytecode never reads out of bounds: every
  operand read and heap/array index is checked, and arithmetic wraps through
  `std::uint32_t` (no signed-overflow UB).

## API

```cpp
#include "clr_simulator.hpp"
namespace clr = ca::clr_simulator;

// ldc.i4 1; ldc.i4 2; add; stloc.0; ldloc.0; ret  ->  local 0 == 3
clr::Simulator sim;
sim.load({0x17, 0x18, 0x58, 0x0A, 0x06, 0x2A}, 16);
sim.run(100);                       // halted after 6 steps
auto loc = sim.local_at(0);         // loc->as_int() == 3
```

- `load` / `load_program` — one method, or a table starting at an entry index.
- `step` / `run` — execute one / many instructions (throwing `Error` on faults).
- `halted` / `pc` / `stack` / `locals` / `stack_top` / `local_at` — inspection.
- `encode_ldc_i4` / `encode_stloc` / `encode_ldloc` / `assemble` — static
  encoder helpers returning `std::vector<std::uint8_t>`.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.

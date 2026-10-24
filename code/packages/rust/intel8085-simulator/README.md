# Intel 8085 Functional Simulator

A checked Rust simulator for Intel's 8085A (1976), the single-supply successor
to the Intel 8080. It executes all 246 documented opcode encodings and models
the 8085-only vectored interrupt controller and serial pins.

The package composes the repository's completed `intel8080-simulator` for the
244 shared instruction encodings. It adds:

- `RIM` and `SIM` at opcodes `0x20` and `0x30`;
- TRAP, RST 7.5, RST 6.5, RST 5.5, and INTR priority and vectoring;
- per-source restart masks and the RST 7.5 edge latch;
- the one-instruction EI delay and immediate DI behavior;
- SID input and SOD output latches;
- exact installed-program bounds, owned snapshots, typed atomic faults, and
  transactional bounded runs.

The default checked contract rejects the ten undocumented bytes used by hidden
8085 instructions on some physical steppings. This keeps the simulator aligned
with Intel's published programming contract while leaving room for a future
opt-in compatibility mode.

## Quick start

```rust
use intel8085_simulator::{Intel8085Simulator, InterruptRequest};

let mut cpu = Intel8085Simulator::new(65_536);
cpu.load_program(&[
    0x3E, 0xC0, // MVI A,C0h: SOD=1, SOE=1
    0x30,       // SIM
    0x76,       // HLT
])?;

let result = cpu.run_loaded_with_limit(10)?;
assert!(result.halted);
assert!(cpu.serial_output);

// TRAP is non-maskable and can release HLT at the next boundary.
cpu.core.regs.sp = 0x4000;
cpu.request_interrupt(InterruptRequest::Trap);
let trace = cpu.step()?;
assert_eq!(trace.mnemonic, "INT TRAP");
assert_eq!(cpu.core.pc, 0x0024);
# Ok::<(), intel8085_simulator::Intel8085Error>(())
```

The shared registers, flags, memory, ports, PC, halt flag, and global IE latch
are available through `cpu.core`. The 8085-specific masks and serial pins are
top-level fields. `snapshot()` returns an owned `Intel8085State` containing all
of both groups plus pending requests and load lifecycle.

## RIM and SIM

RIM loads A with this bit layout:

```text
7    6      5      4     3    2      1      0
SID  I7.5   I6.5   I5.5  IE   M7.5   M6.5   M5.5
```

SIM interprets A as:

```text
7    6    5  4      3    2      1      0
SOD  SOE  X  R7.5   MSE  M7.5   M6.5   M5.5
```

MSE gates mask changes, R7.5 clears the latched edge request, and SOE gates the
serial output update. Disabled groups preserve their previous latches.

## Interrupt model

Call `request_interrupt` between steps. Acceptance order is TRAP, RST 7.5,
RST 6.5, RST 5.5, then INTR. INTR carries the instruction byte returned by an
external device during interrupt acknowledge; the checked API accepts the
conventional eight `RST n` bytes and rejects other bytes without mutation.

Acceptance pushes the current PC in normal 8085 stack order, clears halt and
global interrupt enable, and transfers to the vector. Interrupt acknowledge
traces have no raw program bytes because no byte was fetched from memory.

## Verification

The test suite includes:

- exhaustive classification and width checks over all 256 first bytes;
- RIM/SIM bit-layout and latch-preservation witnesses;
- EI delay, DI, priority, all four fixed vectors, INTR/RST, and HLT release;
- exact interrupt stack byte order;
- typed, atomic load/fetch/opcode/data/stack/INTR/run failures;
- reset, repeatable bounded runs, and owned full-state snapshots.

Release tests, strict Clippy, strict rustdoc, and formatting pass. Coverage is
98.89% of lines (357/361), 97.51% of regions, and 100% of functions.

See [`07af-intel-8085-simulator.md`](../../../specs/07af-intel-8085-simulator.md)
for the complete contract and primary Intel references.


# OpenRISC 1000 simulator (Rust)

A checked, deterministic functional simulator for the 32-bit OpenRISC 1000
ORBIS32 scalar integer architecture. It is the instruction-level reference for
the later educational gate-level OpenRISC cell.

The architectural source is the OpenRISC project's
[OpenRISC 1000 Architecture Manual 1.4](https://github.com/openrisc/doc/blob/master/openrisc-arch-1.4-rev0.pdf).
OpenRISC's separate [mor1kx](https://github.com/openrisc/mor1kx) RTL is not
copied into this crate; it is the primary implementation reference reserved for
the gate-level partner.

## What is modeled

- all 32 general-purpose registers, including writable `r0` as required by the
  architecture (software normally maintains it as zero);
- PC/PPC, supervision flags, exception state, MAC state, and atomic reservation;
- complete 64 KiB big-endian memory and explicit program lifecycle metadata;
- scalar integer Class I instructions plus the documented integer Class II
  shifts, extensions, multiply/divide, MAC, synchronization, and atomic forms;
- no-delay-slot branches and links; and
- checked complete-state traces and transactional bounded execution.

Vector, floating-point, custom, 64-bit, cache, MMU, timer, and device behavior
is explicitly outside this CPU-only package. Those encodings produce typed
errors instead of approximations.

## Example

```rust
use openrisc1000_simulator::OpenRisc1000Simulator;

// l.addi r1,r0,42 ; l.nop 1 (the repository's semihosting halt)
let program = [0x9c, 0x20, 0x00, 0x2a, 0x15, 0x00, 0x00, 0x01];
let mut cpu = OpenRisc1000Simulator::new();
let result = cpu.execute(&program, 0x100, 2)?;
assert_eq!(result.final_state.gpr[1], 42);
# Ok::<(), openrisc1000_simulator::OpenRisc1000Error>(())
```

Every successful step returns the exact big-endian instruction bytes, raw
word, conventional `l.*` mnemonic, PC transition, and complete resulting
state. Failed steps and failed bounded runs leave the machine unchanged.

## Verification

The completion contract sweeps every primary opcode, all implemented ALU,
shift, compare, MAC, system, and SPR subfamilies, malformed encodings, lifecycle
faults, rollback, atomics, exception entry/return, and exact transport. Release
tests, strict Clippy, rustdoc warnings, and 94.12% line coverage (752/799) pass.

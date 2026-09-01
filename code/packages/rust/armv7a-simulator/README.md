# ARMv7-A / Thumb-2 simulator

Pure Rust functional simulator for the educational ARMv7-A Thumb-2 machine in
[Spec 07x](../../../specs/07x-armv7a-thumb2-simulator.md). It owns an exact
64 KiB little-endian wrapping memory, sixteen 32-bit registers, an authoritative
program counter, CPSR, halt state, and checked program-installation metadata.

The public lifecycle fails closed: restore and loading validate before commit;
single-step faults preserve the complete prior state; bounded runs either commit
all transitions or restore their starting state. Successful traces include the
raw 16- or 32-bit encoding, width, mnemonic, and complete before/after snapshots.

```rust
use armv7a_simulator::{encoding, Armv7ASimulator};

let mut cpu = Armv7ASimulator::new();
let program = encoding::program(&[encoding::mov_imm8(0, 42), encoding::halt()]);
cpu.load_checked(&program).unwrap();
let result = cpu.run_checked(4).unwrap();
assert_eq!(result.final_state.registers[0], 42);
```

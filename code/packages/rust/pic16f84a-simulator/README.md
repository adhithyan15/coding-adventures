# PIC16F84A functional simulator

This crate models the classic 14-bit mid-range CPU in Microchip's PIC16F84A.
It owns the 1K-word Harvard program store, 68 physical GPR bytes, W, the
banked CPU SFR views, and the circular eight-entry return stack. All 35
documented instructions are decoded from their real 14-bit words.

Timer0, GPIO, EEPROM, watchdog timing, asynchronous interrupts, and electrical
behavior are deliberately typed boundaries for a later SoC composition.

The implementation classifies every possible 14-bit word, uses the documented
13-bit architectural PC with 1K-word physical aliasing, applies PCL/PCLATH and
STATUS write rules atomically, and models the hardware's wrapping eight-entry
return stack. Successful steps return the fetched word, stable uppercase
mnemonic, pipeline effects, effective file address, and complete post-state.

```rust
use pic16f84a_simulator::{Pic16f84aSimulator, encoding};

let mut cpu = Pic16f84aSimulator::new();
let result = cpu.execute(&[encoding::movlw(42), encoding::halt()], 0, 4)?;
assert!(result.halted);
assert_eq!(result.final_state.w, 42);
# Ok::<(), pic16f84a_simulator::Pic16f84aError>(())
```

The normative architecture contract is
[`../../../specs/07bb-pic16f84a-simulator.md`](../../../specs/07bb-pic16f84a-simulator.md).

## Development

```bash
bash BUILD
```

The independent test matrix exhausts all 16,384 instruction words, every
direct and indirect file address, all byte pairs for add/subtract, every
11-bit CALL/GOTO literal and PCLATH page combination, all PCL writers, skip
wrap boundaries, stack overwrite depths, and typed lifecycle rollback.
The standard all-target release coverage command reports 98.67% line coverage
(670/679 lines) for package source.

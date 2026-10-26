# Classic AVR simulator

This crate is a checked instruction-level model of the ATmega328P `avr5` CPU
profile. It owns the device's 32 KiB Flash transport and complete 0x900-byte
data map, including the register file, opaque I/O, SREG, stack pointer, and
SRAM. It implements the bounded integer, memory, bit, skip, branch, call, and
return surface in [Spec 07ay](../../../specs/07ay-avr-classic-simulator.md).

```rust
use avr_classic_simulator::AvrClassicSimulator;

let mut cpu = AvrClassicSimulator::new();
cpu.load(&[0x00, 0x00, 0x98, 0x95], 0)?; // NOP; BREAK
let result = cpu.run(2)?;
assert!(result.halted);
let final_state = result.final_state.clone();
cpu.restore(final_state.clone())?;
assert_eq!(cpu.state(), &final_state);
# Ok::<(), avr_classic_simulator::AvrError>(())
```

`BREAK` is the repository's deterministic halt sentinel. This is a CPU model,
not a complete microcontroller: peripherals, interrupts, EEPROM protocol,
timing, pins, watchdog, sleep, and self-programming remain excluded.

The release completion suite exercises 34 lifecycle, exhaustive flag, and
instruction-family tests. It includes terminal-state restore round trips and
whole-span call-stack preflight at the lower SRAM boundary, and reaches 97.43%
line coverage (949/974) in the package library.

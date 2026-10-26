# Renesas SuperH-2 functional simulator

A checked, bounded instruction-level model of the SH-2 integer core described
by [Spec 07av](../../../specs/07av-superh-2-simulator.md) and Renesas manual
REJ09B0171-0500O. It owns complete 64 KiB state, uses big-endian instruction
and data transport, and models delayed control flow, the integer/memory
families in the specification, and the SH-2 multiply/divide state.

The package is intentionally a processor teaching oracle rather than a claim
to model a complete microcontroller or later SuperH variant. Interrupt entry,
peripherals, caches, buses, `TAS.B`, privileged SR/VBR transfers, SH-DSP,
SH-2A, SH-3, and SH-4 remain explicit follow-up work.

```rust
use superh2_simulator::{SuperH2Simulator, DEFAULT_ORIGIN};

// MOV #42,R0 ; SLEEP
let mut cpu = SuperH2Simulator::new();
let result = cpu.execute(&[0xe0, 0x2a, 0x00, 0x1b], DEFAULT_ORIGIN, 2)?;
assert_eq!(result.final_state.registers[0], 42);
assert!(result.halted);
# Ok::<(), superh2_simulator::SuperH2Error>(())
```

Every typed failure is atomic. Successful traces retain the exact fetched
bytes, decoded word, conventional uppercase mnemonic, delay-slot metadata,
and a complete post-step snapshot.

## Verification

The completion suite classifies every admitted encoding family and reserved or
unsupported neighbors, and exercises arithmetic flags, addressing modes,
multiplication/division, MAC alias ordering, delayed control flow, PC-relative
delay-slot bases, complete-state lifecycle, and transactional rollback. It
also locks architectural illegal-slot precedence for admitted control
transfers, undefined code, and excluded RTE, TRAPA, and SR-load forms while
retaining ordinary outside-slot classifications and fetch-fault precedence.
All 23 tests, including an independent exhaustive `DIV1` Q/M/T truth table,
strict Clippy, rustdoc, and the package BUILD are green. Measured release line
coverage is 94.44% (1,036/1,097).

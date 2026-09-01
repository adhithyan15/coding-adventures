# Motorola 68000 Functional Simulator (Rust)

Complete Rust implementation of the Motorola 68000 surface defined by Spec
07n. It is the functional oracle for `motorola68k-gatelevel` and preserves the
repository convention that `TRAP #15` and `STOP` halt execution.

## Architecture

- Eight 32-bit data registers (`D0`–`D7`) and eight 32-bit address registers
  (`A0`–`A7`), with `A7` as the supervisor stack pointer.
- A 24-bit, 16 MiB, big-endian address space with checked word alignment.
- Complete X/N/Z/V/C condition-code behavior for the specified instruction
  surface.
- All twelve effective-address forms: register direct, the indirect family,
  displacement/indexed, absolute, PC-relative/indexed, and immediate.

The bit-field decoder implements MOVE/MOVEA/MOVEQ; immediate, quick, extended,
and ordinary arithmetic; AND/OR/EOR; compare; bit operations; multiply/divide;
register and memory shifts/rotates; all conditional branches/Scc/DBcc; stack,
frame, jump, call, return, SR/CCR, trap, and halt operations in Spec 07n.
Reserved line-A/line-F and undefined encodings fail closed.

## Checked API

`M68kSimulator::architectural()` constructs the exact machine: 16,777,216
memory bytes, PC `0x001000`, A7 `0x00F000`, and SR `0x2700`.
`load_checked`, `restore`, `step_checked`, `run_loaded_checked`, and
`run_checked` return typed `M68kError` values and are atomic on failure.
`M68kState` owns all registers, SR, halt state, and all memory; each
`StepTrace` includes complete before/after states.

```rust
use m68k_simulator::M68kSimulator;

let mut simulator = M68kSimulator::architectural();
let result = simulator.run_checked(&[
    0x70, 0x2a, // MOVEQ #42,D0
    0x4e, 0x4f, // TRAP #15
], 10)?;
assert_eq!(result.final_state.d[0], 42);
assert!(result.halted);
# Ok::<(), m68k_simulator::M68kError>(())
```

The legacy caller-sized `new`, zero-origin `load_program`, `step`, and `run`
methods remain for existing backend and encoder consumers. Checked lifecycle
methods deliberately require the architectural 16 MiB machine.

## Verification

The reproducible Python-oracle fixture contains 82 full-state vectors. It
covers every addressing form and decode line, all formerly deferred families,
flag/carry/overflow edges, stack/control behavior, division errors, and hashes
every byte in its deterministic memory window. Separate lifecycle tests pin
the exact 16 MiB state, typed bounds, atomic restore/execute failure, and full
traces.

See [`code/specs/07n-motorola-68000-simulator.md`](../../../specs/07n-motorola-68000-simulator.md).

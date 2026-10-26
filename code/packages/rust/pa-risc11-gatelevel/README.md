# PA-RISC 1.1 gate-level simulator

This package is an educational ISA-level gate topology for the bounded
problem-state integer core in `pa-risc11-simulator`.  It is not an HP silicon
netlist.  An independent one-hot decoder and explicit Boolean, ripple,
barrel-mux, memory-lane, condition, and queue networks settle each transition.
All 525,436 persistent bits are stable Q outputs clocked through the repository
D-flip-flop primitive.

The checked API mirrors the functional simulator.  Loads, restores, steps,
runs, and executions are transactional; complete snapshots include 64 KiB of
memory, both IAOQ offsets, 32 physical register slots, eight C/B bits, delay and
nullification state, halt, and installed-image metadata.

The functional simulator is used only as an independent test oracle.  See
Spec 07au2 for the exact admitted instruction surface and exclusions.

The implementation has 25 release tests. Measured all-target package line
coverage is 97.65% (1,040/1,065 lines). The matrices cover all 64 primary decode
lines, all 64 ALU extension values, every SHD count, every condition and sense,
register overlap/GR0, exhaustive nibble add/subtract relations, reserved-field
neighbors, memory widths and boundaries, delay/nullification, functional-oracle
lifecycle rollback, stable-Q divergence, and compiler-checked hostile rewrites.

```rust
use pa_risc11_gatelevel::PaRisc11GateSimulator;

let mut cpu = PaRisc11GateSimulator::new();
// LDO 7(0),r1 ; BREAK 0,0.
let image = [0x34, 0x01, 0x00, 0x0e, 0, 0, 0, 0];
let result = cpu.execute(&image, 0x100, 2)?;
assert_eq!(result.final_state.gpr[1], 7);
# Ok::<(), pa_risc11_gatelevel::PaRisc11Error>(())
```

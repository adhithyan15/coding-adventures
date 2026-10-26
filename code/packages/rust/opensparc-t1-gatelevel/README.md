# OpenSPARC T1 gate-level simulator (Rust)

This package is the DFF-backed architectural gate companion to
`opensparc-t1-simulator`. It models one scalar integer strand of Oracle's
OpenSPARC T1 boundary. The public UltraSPARC manuals define behavior and the
released OpenSPARC Verilog provides the structural implementation reference.

All 534,813 persistent architectural bits are stored in repository D flip-
flops. Gate networks derive format and subopcode decode, register-window
mapping, integer and condition-code results, delayed control flow, big-endian
memory operations, and typed fault selection. The checked functional simulator
is used only as a differential-test oracle.

```rust
use opensparc_t1_gatelevel::{encoding, OpenSparcT1GateLevel};

let mut cpu = OpenSparcT1GateLevel::new();
cpu.load(&encoding::encode_ta(0).to_be_bytes(), 0)?;
let trace = cpu.step()?;
assert_eq!(trace.mnemonic, "ta");
assert!(trace.state_after.halted);
# Ok::<(), opensparc_t1_gatelevel::OpenSparcT1Error>(())
```

The exact topology is 534,813 DFFs: 524,288 memory bits, 10,240 physical
register bits, and 285 bits for PC/nPC, condition codes, Y, ASI, PSTATE,
register-window control, halt/delay state, and installed-image metadata.
Bounded execution fallibly preallocates retained complete-state buffers before
mutation. The release suite checks all primary formats, every format-2 and
format-3 subopcode line, both register and immediate arithmetic forms, exact
traces, complete state, atomic faults, and the fixed topology against the
independent checked functional package. Combined line coverage is 94.80%
(1,623 of 1,712 lines).

The model intentionally stops at the instruction boundary. Pipeline timing,
strand scheduling, caches, translation, crossbar, memory controllers, and
other SoC structures are separate future cells.

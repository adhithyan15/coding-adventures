# OpenRISC 1000 gate-level simulator (Rust)

This package is the DFF-backed architectural gate companion to
`openrisc1000-simulator`. It implements the documented big-endian ORBIS32
integer boundary with a local transition engine and clocks all persistent
state through the repository's simulated D flip-flops.

The public OpenRISC architecture manual is the behavioral source and the
public mor1kx RTL is the structural implementation reference. This is an
educational architectural gate model, not a cycle-accurate mor1kx pipeline,
Wishbone bus model, FPGA placement, or transistor-level die simulation.

```rust
use openrisc1000_gatelevel::OpenRisc1000GateLevel;

let mut cpu = OpenRisc1000GateLevel::new();
// l.nop 1 is the repository's deterministic semihosting halt convention.
cpu.load(&0x1500_0001_u32.to_be_bytes(), 0)?;
let trace = cpu.step()?;
assert_eq!(trace.mnemonic, "l.nop");
assert!(trace.state_after.halted);
# Ok::<(), openrisc1000_gatelevel::OpenRisc1000Error>(())
```

The exact topology is 525,651 DFFs: 524,288 memory bits, 1,024 GPR bits,
256 core/control-register bits, 33 reservation bits, one halt bit, 32 installed
origin bits, and 17 installed-length bits. Release tests compare the complete
state and exact traces with the independent checked functional package.

# RCA CDP1802 gate-level simulator

This crate is the DFF-backed educational architectural gate companion to
`cdp1802-simulator`, governed by
[Spec 07bc2](../../../specs/07bc2-rca-cdp1802-gatelevel.md). It implements the
complete accepted CDP1802 functional boundary with an independent decoder and
transition engine. Production code does not depend on the functional oracle.

Every mutable bit is authoritative stable-Q storage clocked through repository
D flip-flops: 524,288 memory bits, 256 register-matrix bits, and 123 selector,
flag, input, image, interrupt-inhibit, and IDL bits, for exactly **524,667
DFFs**. Repository gates and ripple networks derive opcode-family legality,
branch/skip/interrupt predicates, ALU results, DF, selectors, address changes,
and write enables.

```rust
use cdp1802_gatelevel::Cdp1802GateSimulator;

let mut cpu = Cdp1802GateSimulator::new();
cpu.load(&[0xf8, 0x42, 0xb1, 0x00], 0x0100)?;
let result = cpu.run(3)?;
assert!(result.stopped);
assert_eq!(result.final_state.registers[1], 0x4200);
# Ok::<(), cdp1802_gatelevel::Cdp1802Error>(())
```

The model exposes checked reset/load/restore, direct state access, exact
instruction traces, bounded run/execute, sampled EF/port inputs, and explicit
DMA/interrupt transactions. It is not a transistor netlist, cycle-accurate
die, SoC, peripheral system, or timing model.

## Verification

```sh
cargo fmt -p cdp1802-gatelevel -- --check
cargo clippy -p cdp1802-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cdp1802-gatelevel --no-deps
cargo test -p cdp1802-gatelevel --release --no-fail-fast
cargo llvm-cov -p cdp1802-gatelevel --release --summary-only
```

The release suite compares exact full traces and states for every accepted
first byte plus focused alias, selector, wrap, self-modification, branch, skip,
immediate, port, lifecycle, DMA, interrupt, fault, run, execute, and allocation
matrices. Allocation can fail after an earlier private instruction succeeds,
and run/execute still restore their complete call-entry state. Exhaustive
eight-bit arithmetic, exact topology, and eleven compiler-checked hostile
full-crate anti-bypass mutations complete the evidence. The current result is
40 release tests plus one doctest and 95.62% combined line coverage (1,442 of
1,508 lines).

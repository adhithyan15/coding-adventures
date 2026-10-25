# OpenSPARC T1 simulator (Rust)

A checked, deterministic functional simulator for one UltraSPARC T1 hardware
thread (a *strand*). It is the instruction-boundary reference for the later
OpenSPARC gate-level and SoC models.

The primary sources are Oracle's
[OpenSPARC T1 page](https://www.oracle.com/servers/technologies/opensparc-t1-page.html),
[*UltraSPARC Architecture 2005*](https://www.oracle.com/docs/tech/systems/t1-07-091020-ua2005-d0-9-3b-hp-ext.pdf),
the [T1 supplement](https://www.oracle.com/docs/tech/systems/t1-09-ust1-uasuppl-draft-hp-ext.pdf),
and the [T1 microarchitecture specification](https://www.oracle.com/docs/tech/systems/t1-01-opensparct1-micro-arch.pdf).

## What is modeled

- one 64-bit SPARC V9 scalar integer strand;
- four global-register levels and eight overlapping register windows, totaling
  the T1's 160 physical integer registers per strand;
- separate 32-bit `icc` and 64-bit `xcc` condition codes;
- architectural PC/nPC delay slots, branch annul, calls, and indirect links;
- scalar arithmetic, logical, shift, multiply/divide, window, and big-endian
  memory operations; and
- complete 64 KiB memory, installed-program metadata, typed atomic faults, and
  transactional bounded execution.

The real chip's other 31 strands, shared pipelines, caches, TLBs, MMUs,
crossbar, L2 banks, memory controllers, JBus, SSI, floating-point, VIS, and
hyperprivileged facilities are intentionally outside this CPU-only package.
The gate and SoC cells will compose those structures from Oracle's released
RTL instead of approximating them here.

## Example

```rust
use opensparc_t1_simulator::{assemble, encoding::*, OpenSparcT1Simulator};

let program = assemble(&[
    encode_add_imm(1, 0, 42),
    encode_ta(0), // repository semihosting halt
]);
let mut strand = OpenSparcT1Simulator::new();
let result = strand.execute(&program, 0x100, 2)?;
assert_eq!(strand.read_register(1)?, 42);
assert!(result.halted);
# Ok::<(), opensparc_t1_simulator::OpenSparcT1Error>(())
```

Every successful step reports both PC pipeline values, exact raw bytes and
word, conventional mnemonic, and complete resulting state. Failed steps and
failed bounded runs preserve the complete prior machine.

## Verification

The completion contract covers decode families, both condition-code widths,
all scalar memory widths, register-window overlap and counters, delay slots and
annul, links, lifecycle validation, fault atomicity, and run rollback. Release
tests, strict Clippy, and rustdoc warnings pass. The completion baseline is
88.52% line coverage (933/1,054).

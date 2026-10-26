# Classic MSP430 functional simulator

This package is a checked teaching model of the original 16-bit MSP430 CPU,
pinned to the classic core in the MSP430F2274. It implements all 27 encoded
classic instruction families, every documented source and destination mode,
constant generation, little-endian byte/word memory, stack and control flow,
typed atomic lifecycle operations, and complete post-state traces.

The bounded contract is defined by
[`07ba-msp430-classic-simulator.md`](../../../specs/07ba-msp430-classic-simulator.md).
The package deliberately models a flat 64 KiB CPU address space rather than
inventing Flash, peripheral, interrupt, clock, or low-power behavior from the
MSP430F2274 SoC.

```rust
use msp430_simulator::{Msp430, HALT_WORD};

let mut cpu = Msp430::new();
let halt = HALT_WORD.to_le_bytes();
cpu.load(&halt, 0x0200).unwrap();
let traces = cpu.run(1).unwrap();
assert_eq!(traces[0].mnemonic, "HALT");
assert!(cpu.snapshot().halted);
```

Run checks from `code/packages/rust`:

```sh
cargo test -p msp430-simulator --release
cargo clippy -p msp430-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p msp430-simulator --no-deps
```

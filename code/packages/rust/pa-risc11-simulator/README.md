# PA-RISC 1.1 simulator (Rust)

A checked functional simulator for a bounded 32-bit HP PA-RISC 1.1
problem-state integer core. The normative source is HP's
[*PA-RISC 1.1 Architecture and Instruction Set Reference Manual*, third
edition](https://parisc.docs.kernel.org/en/latest/_downloads/f15883d2c095ac4e7606ac263ab8d883/Pa11_acd.pdf)
(09740-90039, February 1994); Spec 07au records the exact repository boundary.

The package models fixed 32-bit big-endian instructions, all 32 general
registers with hard-wired GR0, a 64 KiB big-endian teaching memory, both
instruction address queues, all eight PSW nibble carry/borrow outputs, delayed
branches, direction-sensitive
nullification, calls/returns, exact complete-state traces, and transactional
typed lifecycle operations. Its coherent instruction subset includes basic
register arithmetic/logic, PA-RISC immediate assembly, SHD, long-displacement
byte/half-word/word memory operations, compare/add branches, BL/BLR/BV, and a
deterministic `BREAK 0,0` halt sentinel.

BV follows the architecture's split between instruction-address and privilege
bits. Because this flat model omits privilege levels, it normalizes the low two
privilege bits instead of treating them as address misalignment. Architected
nested delayed transfers are intentionally deferred to a later bounded cell.

```rust
use pa_risc11_simulator::PaRisc11Simulator;

// LDO 7(0),r1 ; BREAK 0,0. The low-sign immediate puts its sign in bit zero.
let program = [0x34, 0x01, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00];
let mut cpu = PaRisc11Simulator::new();
let result = cpu.execute(&program, 0x100, 2)?;
assert_eq!(result.final_state.gpr[1], 7);
# Ok::<(), pa_risc11_simulator::PaRisc11Error>(())
```

Privileged/system instructions, spaces, MMU/TLB state, caches, FPU and
coprocessor facilities, exceptions, timing, and the remaining documented
problem-state instruction families are explicit follow-up work. Unknown and
unsupported encodings fail atomically; this package does not claim full ISA or
hardware coverage.

## Validation

```bash
cargo test -p pa-risc11-simulator --release --no-fail-fast
cargo clippy -p pa-risc11-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p pa-risc11-simulator --no-deps
```

The completion suite independently encodes all admitted formats, classifies all
64 primary opcodes, exhausts both senses of the eight compare/add conditions,
and covers immediate permutations, memory boundaries, delay/nullification,
calls/returns, state validation, halt, fetch, allocation-failure seams,
retention, and rollback. Standard all-target line coverage is 92.80% (580/625).

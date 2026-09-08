# DEC Alpha AXP 21064 simulator

Complete Rust functional simulator for the integer subset of the 1992 DEC
Alpha AXP 21064 defined by Layer 07s.

## Machine

- 32 unsigned 64-bit GPRs; `r31` is hardwired to zero
- 64-bit PC/nPC control flow over an exact 64 KiB educational address space
- little-endian fixed-width 32-bit instructions and memory transfers
- no condition codes or delay slots
- `call_pal 0` (`0x00000000`) as the halt sentinel

The implemented surface includes INTA arithmetic and comparisons, INTL logic
and conditional moves, INTS shifts and byte manipulation, INTM multiplication,
aligned byte/word/long/quad loads and stores, all integer branches, JMP/JSR/
RET/coroutine jumps, and the repository's scoped PALcode behavior.

## Checked lifecycle

```rust
use alpha_axp_simulator::encoding::{assemble, halt, mov_literal};
use alpha_axp_simulator::AlphaSimulator;

let program = assemble(&[mov_literal(1, 42), halt()]);
let mut cpu = AlphaSimulator::new();
let result = cpu.run_checked(&program, 8)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[1], 42);
# Ok::<(), alpha_axp_simulator::AlphaError>(())
```

`AlphaState` owns PC/nPC, all registers, every memory byte, halt, and the exact
installed-program range. `restore` validates before mutation. Checked load,
direct register/memory access, step, and bounded run methods return typed
errors; instruction faults and failed bounded runs roll the complete machine
back. Each `StepTrace` owns the raw word plus complete before/after states.

## Verification

The Rust suite includes seed unit tests, lifecycle and workload tests, and a
reproducible 624-vector differential generated from the Python oracle. The
corpus covers both register and literal forms of every implemented operate
function, every load/store, branch, and jump family, four seeded full states,
and seven fault families. It hashes PC/nPC, all GPRs, all 64 KiB of memory, and
halt state.

```sh
cargo test --package alpha-axp-simulator
cargo clippy --package alpha-axp-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --package alpha-axp-simulator --no-deps
```

# DEC VAX-11/780 simulator

Checked Rust functional simulator for the integer core of the 1977 DEC
VAX-11/780 defined by Layer 07ag.

## Machine

- sixteen 32-bit registers (`R12=AP`, `R13=FP`, `R14=SP`, `R15=PC`)
- PSL N/Z/V/C condition bits
- exact 64 KiB little-endian educational memory arena
- unaligned byte, word, and longword transfers
- owned installed-program origin/length and halt lifecycle

The implemented instruction surface includes HALT/NOP, byte/word branches,
all twelve integer conditions, BSB/JSB/JMP/RSB, B/W/L arithmetic and logical
families, move/compare/unary families, zero-extension, address moves/pushes,
PUSHL, and CASE.  All general operand modes are decoded, including short
literals, index, deferred, auto-update, displacement, immediate, absolute,
and PC-relative forms.

Floating-point, decimal, string, queue, MMU, process-context, and operating-
system trap delivery instructions are deliberately outside this cell.

## Checked lifecycle

```rust
use vax11_780_simulator::{VaxSimulator, encoding};

let program = encoding::assemble(&[
    encoding::movl(encoding::literal(42), encoding::register(0)),
    vec![encoding::HALT],
]);
let result = VaxSimulator::new().run_checked(&program, 8)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[0], 42);
# Ok::<(), vax11_780_simulator::VaxError>(())
```

`VaxState` owns every architectural and lifecycle field.  Restore, load,
memory/register access, stepping, and bounded execution are checked.  Faulting
instructions and failed bounded runs restore the exact prior state.  Traces
retain raw instruction bytes plus scalar before/after state without cloning
64 KiB of memory per step, and a fixed safe trace ceiling bounds retention.

## Verification

The release suite executes every documented arithmetic/data opcode family,
every branch condition, every addressing family, stack and control-flow
paths, complete-state restoration, trace behavior, and the typed atomic fault
surface.

```sh
cargo test --package vax11-780-simulator --release
cargo clippy --package vax11-780-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --package vax11-780-simulator --no-deps
cargo llvm-cov --package vax11-780-simulator --release --summary-only
```


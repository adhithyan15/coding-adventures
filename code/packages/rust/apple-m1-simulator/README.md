# Apple M1 Simulator (Rust)

Checked functional implementation of the Spec 07z Apple M1 teaching surface:
the complete Layer 07v AArch64 integer core plus scalar IEEE-754 and
NEON/AdvSIMD instructions.

The machine owns exactly 64 KiB of big-endian memory, 32 64-bit GPR slots with
XZR enforced as zero, separate SP and PC, four NZCV bits, 32 128-bit vector/FP
registers, halt state, and installed-program origin and length. Restore,
origin-aware load, direct access, fetch, data alignment/ranges, and reserved
encodings are checked. Failed steps preserve every bit; failed bounded runs
restore the complete pre-run state.

## Supported instructions

- The complete `aarch64-simulator` integer surface: branches, arithmetic,
  logical/shift, move/bit, conditional-select, integer load/store, NOP, SVC,
  and halt
- Scalar FP: FMOV, FABS, FNEG, FSQRT, FCVT, FMUL, FDIV, FADD, FSUB, FCMP,
  FCVTZS, SCVTF, and UCVTF in single and double precision
- FP memory: scaled unsigned-offset LDR/STR for S and D registers
- NEON integer: lane-wise ADD, SUB, MUL and GPR broadcast with DUP
- NEON FP: lane-wise FADD, FSUB, FMUL, and the teaching model's `Vd + Vn*Vm`
  FMLA behavior

Malformed fields, unsupported precision encodings, invalid vector shapes, and
unsupported instructions return a typed `UnknownInstruction` fault. Exception
levels, MMU, interrupts, FPCR/FPSR, half precision, SVE, crypto, atomics, and
unlisted memory addressing modes remain outside this functional cell.

## Example

```rust
use apple_m1_simulator::{encoding, AppleM1Simulator};

let program = encoding::program(&[
    encoding::integer_to_fp(1, 1, true, 0, 0),
    encoding::fp_two_source(1, 0, 2, 0, 1),
    encoding::halt(),
]);
let mut cpu = AppleM1Simulator::new();
cpu.load_checked(&program).unwrap();
cpu.write_register(0, 21).unwrap();
let result = cpu.run_checked(3).unwrap();
assert_eq!(result.final_state.vectors[1] as u64, 42.0_f64.to_bits());
```

The structured encoders emit the repository's big-endian teaching transport,
not native little-endian Apple Silicon instruction bytes.

## Verification

Ten Rust tests cover lifecycle, full-boundary traces, checked state/direct
access, all scalar-FP and NEON families, vector memory, reserved encodings, and
transactional faults. A reproducible 360-vector Python full-state differential
covers every Apple-specific decode family across four seeds; the delegated
AArch64 core separately retains its 836-vector differential. All 109 Python
Apple M1 tests and the Rust AArch64 functional/gate consumers pass. Package
line coverage is 88.37% (646/731).

Run from `code/packages/rust`:

```bash
cargo fmt -p apple-m1-simulator -- --check
cargo clippy -p apple-m1-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p apple-m1-simulator --no-deps
cargo test -p apple-m1-simulator
cargo llvm-cov -p apple-m1-simulator --summary-only
```

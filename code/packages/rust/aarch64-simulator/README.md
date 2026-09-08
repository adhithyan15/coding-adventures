# AArch64 Simulator (Rust)

Checked functional implementation of the complete integer surface documented
by Layer 07v for the AArch64 execution state introduced with ARMv8-A.

The machine owns exactly 64 KiB of big-endian memory, 32 64-bit GPR slots with
XZR enforced as zero, separate 64-bit SP and PC values, four NZCV bits, halt
state, and installed-program origin and length. Public operations validate
restore, origin-aware loading, register and byte access, instruction fetch,
data alignment and ranges, branch alignment, and reserved encodings. A failed
step preserves the complete pre-step state; a failed bounded run restores the
complete pre-run state.

## Supported instructions

- Control flow: B, BL, B.cond, CBZ, CBNZ, TBZ, TBNZ, BR, BLR, RET
- Arithmetic: ADD, ADDS, SUB, SUBS, MADD, MSUB, UDIV, SDIV, SMULH, UMULH
- Logical and shifts: AND, ORR, EOR, ANDS, BIC, ORN, EON, BICS, LSLV,
  LSRV, ASRV, RORV, and shifted-register variants
- Move and bit operations: MOVZ, MOVN, MOVK, RBIT, REV16, REV, REV32, CLZ
- Unsigned-offset integer loads/stores: byte, halfword, word, doubleword, and
  the documented sign-extending load forms
- Conditional select: CSEL, CSINC, CSINV, CSNEG
- NOP, SVC-as-no-op, and the repository's all-zero halt sentinel

Unsupported instruction classes and malformed or reserved fields return a
typed `UnknownInstruction` fault. SIMD/floating-point, atomics, exception
levels, MMU, interrupts, pre/post-indexed memory, register-offset memory,
HVC/SMC, and barrier instructions are outside this functional cell.

## Example

```rust
use aarch64_simulator::{encoding, AArch64Simulator};

let program = encoding::program(&[
    encoding::move_wide(1, 2, 0, 42, 0),
    encoding::halt(),
]);
let mut cpu = AArch64Simulator::new();
cpu.load_at_checked(&program, 0x100).unwrap();
let result = cpu.run_checked(2).unwrap();
assert_eq!(result.final_state.registers[0], 42);
```

The structured encoders return words; `encoding::program` deliberately emits
the big-endian teaching transport used by Spec 07v. This differs from the
existing `aarch64-encoder`, which emits native little-endian backend code and
has a separate production-code purpose.

## Verification

Fourteen Rust tests cover the exact lifecycle, installation metadata,
validated restore/direct access, complete traces, halt boundaries,
transactional runs, every valid decode family, and malformed alignment/range/
reserved-field failures. The implementation matches a reproducible 836-vector
Python one-step full-state corpus over every common decode family. The Python
oracle's 151 tests and the existing Rust AArch64 encoder/backend consumers also
pass. Package line coverage is 97.20% (938/965).

Run the package checks from `code/packages/rust`:

```bash
cargo fmt -p aarch64-simulator -- --check
cargo clippy -p aarch64-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p aarch64-simulator --no-deps
cargo test -p aarch64-simulator
cargo llvm-cov -p aarch64-simulator --summary-only --ignore-filename-regex 'tests/'
```

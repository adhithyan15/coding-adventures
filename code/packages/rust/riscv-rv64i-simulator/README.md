# RISC-V RV64I + M Simulator (Rust)

Checked functional implementation of the complete Spec 07y RV64I base integer
ISA and M multiply/divide extension.

The machine owns exact architectural state: 64 KiB of little-endian memory,
32 64-bit integer registers with x0 enforced as zero, a 64-bit PC, explicit
halt state, and installed-program origin and length. Its public API validates
restore, origin-aware loading, register and byte access, fetches, data
alignment, memory ranges, and instruction encodings. A failed step preserves
the complete pre-step state; a failed bounded run restores the complete
pre-run state.

## Supported instructions

- Upper immediate and control flow: LUI, AUIPC, JAL, JALR
- Branches: BEQ, BNE, BLT, BGE, BLTU, BGEU
- Loads: LB, LH, LW, LD, LBU, LHU, LWU
- Stores: SB, SH, SW, SD
- RV64 ALU: ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI,
  ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR, AND
- Word ALU: ADDIW, SLLIW, SRLIW, SRAIW, ADDW, SUBW, SLLW, SRLW, SRAW
- M extension: MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU, MULW,
  DIVW, DIVUW, REMW, REMUW
- Ordering and halt: FENCE, FENCE.I, ECALL, EBREAK, and the repository's
  distinct all-zero halt sentinel

Unsupported opcodes and reserved funct combinations fail with a typed
`UnknownInstruction` error. This model intentionally excludes compressed,
atomic, floating-point, privileged, CSR, MMU, interrupt, and device behavior.

## Example

```rust
use riscv_rv64i_simulator::{encoding, Rv64ISimulator};

let program = encoding::assemble(&[
    encoding::encode_addi(10, 0, 42),
    encoding::encode_ecall(),
]);
let mut cpu = Rv64ISimulator::new();
cpu.load_at_checked(&program, 0x100).unwrap();
let result = cpu.run_checked(2).unwrap();
assert_eq!(result.final_state.registers[10], 42);
```

## Verification

Nine lifecycle/fault and encoder suites cover exact reset and installation state,
validated restore and direct access, complete traces, halt boundaries, fetch,
decode, alignment, range, post-halt, and transactional step-limit behavior.
The checked implementation also matches a reproducible 364-vector Python
one-step full-state corpus spanning every RV64I+M decode family and arithmetic
edge seeds. All 14 Rust test functions pass, and package line coverage is
97.09% (700/721).

Run the package checks from `code/packages/rust`:

```bash
cargo fmt -p riscv-rv64i-simulator -- --check
cargo clippy -p riscv-rv64i-simulator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p riscv-rv64i-simulator --no-deps
cargo test -p riscv-rv64i-simulator
```

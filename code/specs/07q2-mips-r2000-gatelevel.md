# 07q2 — MIPS R2000 Gate-Level Simulator

## Status

This is the normative specification for the Rust package
`code/packages/rust/mips-r2000-gatelevel`. It is the gate-level partner of
Spec 07q and implements the same complete educational MIPS R2000 transition
surface.

The simulator operates at instruction boundaries. Persistent architectural
state is modeled as D flip-flops, while arithmetic, logic, comparison, shifts,
multiply, divide, decode, and register writes route through repository gate
primitives. It does not reproduce transistor timing, the physical five-stage
pipeline, delay slots, caches, MMU/TLB, package pins, or analog effects.

## Architectural contract

The visible machine is identical to the functional simulator:

- 32-bit, fixed-width, big-endian MIPS I instructions;
- 64 KiB wrapping unified byte-addressed memory;
- 32 GPRs with R0 hardwired to zero, plus HI, LO, and PC;
- R-, I-, and J-format arithmetic, logic, shifts, comparisons, branches,
  jumps, multiply/divide, moves, and immediate operations;
- byte, halfword, word, and big-endian LWL/LWR/SWL/SWR transfers;
- immediate branch/jump effects with no modeled delay slot;
- SYSCALL as the repository halt sentinel;
- distinct typed BREAK, divide-by-zero, signed-overflow, alignment, range,
  truncation, halt, and unknown-instruction failures.

Spec 07q remains authoritative for encoding and instruction semantics. Every
valid checked gate transition must produce the same complete state as its
functional transition.

## Gate networks

The ALU applies primitive gates per bit. Addition and subtraction use the
arithmetic package's ripple-carry network; subtraction supplies complemented B
and carry-in one. Signed overflow is the XOR of carry into and out of bit 31.
Signed and unsigned comparisons derive from subtraction flags. Fixed and
variable shifts use gate-backed bit-vector networks.

MULT/MULTU use 32 shift-and-add rounds with a 64-bit gate accumulator.
DIV/DIVU use fixed-round restoring division. Decode fields are extracted from
LSB-first instruction wires. PC increments and all GPR/HI/LO/PC writes are
clocked through the register bank.

## Exact persistent topology

The completion topology is exactly **525,409 D flip-flops**:

| State | Bits |
|---|---:|
| 64 KiB unified memory | 524,288 |
| 32 GPRs + HI + LO + PC (35 × 32) | 1,120 |
| HALT latch | 1 |
| **Total** | **525,409** |

`FLIP_FLOP_COUNT` publishes this value. A master/slave DFF's internal latch
nodes are determined by Q at a stable instruction boundary. Memory and
register storage therefore retain one packed stable-Q value per DFF;
simulator-owned writes reconstruct the latch state and clock both phases via
`logic_gates::sequential::register`. Installed-program origin and length are
lifecycle metadata and are not included in the DFF count.

## Normative Rust API

```rust
use coding_adventures_mips_r2000_gatelevel::CpuMipsR2000;

let program = [0x2408_002Au32.to_be_bytes(), 0x0000_000Cu32.to_be_bytes()]
    .concat();
let mut cpu = CpuMipsR2000::new();
let result = cpu.run_checked(&program, 8)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[8], 42);
# Ok::<(), coding_adventures_mips_r2000_gatelevel::MipsError>(())
```

The package re-exports the functional `MipsState`, `MipsError`, `StepTrace`,
and `ExecutionResult`. `MipsState` owns PC, all registers, all memory bytes,
halt state, and the installed-program range.

The checked lifecycle is:

- `load_checked` / `load_at_checked`: validate first, reset deterministically,
  and install an exact fetch range;
- `get_state` / `restore`: snapshot or atomically validate and reconstruct the
  complete machine;
- checked register and byte/word helpers: reject invalid indices, alignment,
  and bounds through typed errors;
- `step_checked`: preflight with the shared functional contract, run the gate
  transition, compare every state field, and roll back any failure;
- `run_loaded_checked` / `run_checked`: bound execution, retain complete traces,
  and roll the entire machine back on error.

The legacy `load`, `step`, `execute`, and public register/memory inspection
surface remain available for existing callers.

## Conformance and completion tests

The reproducible fixture in the functional package contains 218 deterministic
one-step full-state hashes generated from the Python oracle. It covers every
decoded instruction family and all Python fault paths across four seeded state
variants. The gate differential hashes PC, all 32 GPRs, HI, LO, every memory
byte, and halt state; expected failures must leave the complete machine intact.

Separate lifecycle tests cover exact topology, deterministic reset, complete
restore, typed direct boundaries, R0, halt clocking, full results, invalid
load/state atomicity, and transactional rollback.

Completion requires:

- all original gate unit tests and doctests green;
- the aggregate 218-vector Python/functional full-state differential green;
- lifecycle tests green;
- strict `cargo fmt`, Clippy `-D warnings`, and rustdoc `-D warnings`;
- total Rust line coverage at or above 80%.

# 07r2 — SPARC V8 Gate-Level Simulator

## Status

This is the normative specification for the Rust package
`code/packages/rust/sparc-v8-gatelevel`. It is the gate-level partner of Spec
07r and implements the same complete educational SPARC V8 transition surface.

The simulator operates at instruction boundaries. Persistent architectural
state is modeled as D flip-flops, while arithmetic, logic, comparison, shifts,
multiply, divide, decode, and register writes route through repository gate
primitives. It does not reproduce transistor timing, physical pipelines,
delayed branches, caches, MMU behavior, package pins, or analog effects.

## Architectural contract

The visible machine is identical to the functional simulator:

- 32-bit fixed-width, big-endian SPARC V8 instructions;
- 64 KiB wrapping unified byte-addressed memory;
- PC/nPC control flow, Y, four integer condition codes, and three windows over
  56 physical registers, with `%g0` hardwired to zero;
- Formats 1, 2, and 3 covering calls, all 16 Bicc predicates, integer ALU,
  shifts, multiply/divide, loads/stores, JMPL, SAVE/RESTORE, RD/WR `%y`, and
  traps;
- immediate branch effects in the repository's no-delay-slot model;
- `ta 0` as the halt sentinel;
- distinct typed division, window, trap, alignment, range, truncation, halt,
  unknown-instruction, step-limit, and invalid-state failures.

Spec 07r remains authoritative for encoding and instruction semantics. The
manual-correct six-bit `UMULcc`, `SMULcc`, `UDIVcc`, and `SDIVcc` op3 values
are `0x1A`, `0x1B`, `0x1E`, and `0x1F`. Every successful checked gate step
must produce the same complete state and trace as the functional transition.

## Gate networks

The ALU applies primitive gates per bit. Addition and subtraction use
ripple-carry networks. Logic and condition codes derive from gate wires, and
shifts operate on LSB-first bit vectors. UMUL/SMUL use fixed-round
shift-and-add networks. UDIV/SDIV use a fixed 64-step restoring divider;
signed division converts operand magnitudes through two's-complement gates and
then reapplies the sign. Divide-cc saturation derives V from quotient width,
sets N/Z from the saturated result, and clears C.

Instruction fields are extracted into gate-visible bit vectors. PC/nPC,
register, control, memory, and halt updates are clocked through DFF state.

## Exact persistent topology

The completion topology is exactly **526,185 D flip-flops**:

| State | Bits |
|---|---:|
| 64 KiB unified memory | 524,288 |
| 56 physical registers | 1,792 |
| PC and nPC | 64 |
| Y | 32 |
| PSR N/Z/V/C | 4 |
| CWP and save depth | 4 |
| HALT latch | 1 |
| **Total** | **526,185** |

`FLIP_FLOP_COUNT` publishes this value. A master/slave DFF's internal latch
nodes are determined by Q at a stable instruction boundary. Storage retains
one packed stable-Q value per DFF; simulator-owned writes reconstruct and
clock the sequential primitive. Installed-program origin and length are
lifecycle metadata and are not included in the DFF count.

## Normative Rust API

```rust
use coding_adventures_sparc_v8_gatelevel::SparcCpu;

let program = [0x9010_202Au32.to_be_bytes(), 0x91D0_2000u32.to_be_bytes()]
    .concat();
let mut cpu = SparcCpu::new();
let result = cpu.run_checked(&program, 8)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[8], 42);
# Ok::<(), coding_adventures_sparc_v8_gatelevel::SparcError>(())
```

The package re-exports the functional `SparcState`, `SparcError`,
`StepTrace`, and `ExecutionResult`. `SparcState` owns PC/nPC, every physical
register, CWP, window depth, PSR, Y, memory, halt state, and installed-program
range.

The checked lifecycle is:

- `load_checked` / `load_at_checked`: validate first, reset deterministically,
  and install an exact fetch range;
- `get_state` / `restore`: snapshot or atomically validate and reconstruct the
  complete machine;
- checked register and byte/word helpers: reject invalid indices, alignment,
  and bounds through typed errors;
- `step_checked`: preflight with the shared functional contract, run the gate
  transition, compare every state field and trace, and roll back any failure;
- `run_loaded_checked` / `run_checked`: bound execution, retain complete
  traces, and roll the entire machine back on error.

Legacy `load`, `step`, `execute`, and public register inspection remain for
existing callers.

## Conformance and completion tests

The reproducible functional fixture contains 248 deterministic one-step
full-state vectors derived from the Python oracle. It covers every Python
decode and fault path across seeded machine states. The gate differential
hashes PC/nPC, all 56 physical registers, CWP, depth, PSR, Y, every memory
byte, halt, and installed range; expected failures must leave the complete
machine intact.

Separate lifecycle suites cover exact topology, deterministic reset, complete
restore, typed direct boundaries, `%g0`, halt clocking, full results, invalid
load/state atomicity, transactional rollback, every Bicc predicate over every
condition-code combination, and divide-cc overflow edges.

Completion requires:

- all original gate unit tests green;
- the aggregate 248-vector Python/functional full-state differential green;
- lifecycle suites green;
- strict `cargo fmt`, Clippy `-D warnings`, and rustdoc `-D warnings`;
- total Rust line coverage at or above 80%.

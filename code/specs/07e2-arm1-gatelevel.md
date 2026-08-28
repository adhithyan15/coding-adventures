# 07e2 — ARM1 Gate-Level Simulator

## Status

This is the normative specification for the Rust package
`code/packages/rust/arm1-gatelevel`. It is the gate-level partner of Spec 07e
and implements the same complete ARM1 / ARMv1 architectural transition surface.

The simulator operates at instruction boundaries. It models persistent state as
D flip-flops and routes the condition evaluator, barrel shifter, and 32-bit ALU
through repository gate primitives. It does not reproduce transistor timing,
the physical three-stage pipeline, DRAM refresh, package pins, or analog effects.

## Architectural contract

The visible architecture is identical to the functional simulator:

- 32-bit fixed-width little-endian instructions;
- a 26-bit, 64 MiB unified byte-addressed memory;
- 16 logical registers and 27 physical registers across USR, FIQ, IRQ, and SVC;
- combined PC, NZCV flags, interrupt masks, and mode in R15;
- pipeline-visible PC+8 operand reads;
- all sixteen condition codes and data-processing operations;
- immediate and register barrel shifts, including RRX;
- single byte/word and block transfers, including force-user banking;
- B/BL, SWI, undefined/coprocessor traps, and the repository HALT SWI;
- externally triggered, mask-aware IRQ and FIQ entry.

Spec 07e remains authoritative for instruction encodings and flag/exception
semantics. The gate simulator must produce the same complete transition for
every valid checked step.

## Gate networks

### Condition evaluator

Condition predicates are built from `AND`, `OR`, `NOT`, `XOR`, and `XNOR`:

```text
HI = C AND NOT Z       LS = NOT C OR Z
GE = N XNOR V          LT = N XOR V
GT = NOT Z AND (N XNOR V)
LE = Z OR (N XOR V)
```

All sixteen ARM condition encodings, including AL and reserved NV, are handled.

### Barrel shifter

LSL, LSR, ASR, and ROR use five 32-bit mux levels controlled by shift-amount
bits 0 through 4. Each level selects either the current bit or the value shifted
by 1, 2, 4, 8, or 16 positions. The edge rules for zero, 32, larger register
amounts, rotated immediates, and RRX match Spec 07e.

### ALU and flags

Logical operations apply primitive gates to each bit. ADD, ADC, SUB, SBC, RSB,
RSC, CMP, and CMN use the arithmetic package's 32-stage ripple-carry adder;
subtraction supplies complemented input and the appropriate carry-in. N is the
most-significant result bit, Z is a gate reduction, C is selected from the adder
or shifter, and V is computed from sign transitions.

The shared decoder performs field extraction and instruction classification.
All arithmetic decision paths and state changes after decoding remain in the
gate simulator; checked execution compares the resulting complete transition
with the functional oracle.

## Exact persistent topology

The completion topology is exactly **536,871,777 D flip-flops**:

| State | Bits |
|---|---:|
| 64 MiB unified memory | 536,870,912 |
| 27 × 32-bit physical registers, including R15/PC/status | 864 |
| HALT latch | 1 |
| **Total** | **536,871,777** |

The constant `FLIP_FLOP_COUNT` publishes this value.

A master/slave DFF has four internal latch outputs, but at a stable instruction
boundary they are completely determined by Q: `(q, !q, q, !q)`. Memory and
register storage therefore retain one packed stable-Q bit for each DFF.
Simulator-owned writes reconstruct the latch state and clock both phases through
`logic_gates::sequential::register`. This preserves exact persistent identity
without allocating multiple host bytes for each internal latch node.

Installed-program origin/length and the diagnostic gate-operation counter are
simulator metadata, not architectural storage, and are not included in the DFF
count.

## Normative Rust API

```rust
use arm1_gatelevel::ARM1GateLevel;
use arm1_simulator::{encode_halt, encode_mov_imm, COND_AL};

let words = [encode_mov_imm(COND_AL, 0, 42), encode_halt()];
let code: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
let mut cpu = ARM1GateLevel::new(4096);
let result = cpu.run_checked(&code, 10)?;
assert!(result.halted);
assert_eq!(result.final_state.regs[0], 42);
# Ok::<(), arm1_simulator::Arm1Error>(())
```

`ARM1GateLevel::architectural()` constructs the exact 64 MiB machine.
`ARM1GateLevel::new(memory_size)` remains for bounded tests and compatibility.

The gate package shares `Arm1State`, `Arm1Error`, `StepTrace`, and
`ExecutionResult` with the functional package. `Arm1State` owns all 27 physical
registers, every memory byte, halt state, and installed-program range.

The checked lifecycle is:

- `load_checked` / `load_at_checked` / `load_words_checked`: validate first,
  reset and clear memory deterministically, then install an exact fetch range;
- `restore`: validate the complete state before changing the machine;
- checked logical-register and byte/word helpers: reject invalid indices and
  memory bounds with typed errors;
- `step_checked`: reject halt/truncation/data-bound failures atomically, run the
  gate transition, and compare complete state with the functional transition;
- `run_loaded_checked` / `run_checked`: bound execution and roll the complete
  machine back after any late failure.

Legacy `load_program`, `step`, and `run` remain available for existing callers.
A legacy step on a halted CPU is inert.

## Banking and interrupts

The physical-register layout is:

```text
0..15   base R0..R15
16..22  R8_fiq..R14_fiq
23..24  R13_irq..R14_irq
25..26  R13_svc..R14_svc
```

LDM/STM with the force-user bit selects base registers for non-PC transfers.
Loading R15 restores its combined PC/status/mode word. `raise_irq` saves R15 in
R14_irq, selects IRQ mode, masks IRQ, and vectors to 0x18. `raise_fiq` saves R15
in R14_fiq, selects FIQ mode, masks IRQ/FIQ, and vectors to 0x1C. A masked
request leaves state unchanged.

## Conformance and completion tests

The reproducible Python fixture from the functional package contains 599
deterministic one-step full-state hashes. It spans all sixteen conditions and
ALU operations in immediate/register forms, every single-transfer and
block-transfer control combination, forward/backward B and BL, SWI/HALT, and
coprocessor/undefined entry. The gate test hashes all 27 physical registers,
all bytes of its bounded memory, and halt state after each transition.

Separate lifecycle tests cover exact topology, the architectural constructor,
stable-Q clocked writes, invalid state/load/register/memory boundaries,
deterministic clearing, truncated fetches, inert halt, late-fault rollback,
complete results, force-user transfers, and IRQ/FIQ banking/masks.

Completion requires:

- all original gate unit/program differentials green;
- the aggregate 599-vector full-state differential green;
- lifecycle tests green;
- strict `cargo fmt`, Clippy `-D warnings`, and rustdoc `-D warnings`;
- total Rust line coverage at or above 80%.

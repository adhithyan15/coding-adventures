# Spec 07l2 — Manchester Baby (SSEM) Gate-Level Simulator

**Layer**: 07l2

**Architecture**: Manchester Small-Scale Experimental Machine (SSEM)

**Year**: 1948

**Rust package**: `manchester-baby-gatelevel`

**Behavioral oracle**: `manchester-baby-simulator`

**Gate dependencies**: `logic-gates`, `arithmetic`

## Purpose and scope

This is the gate-level companion to Spec 07l. It preserves the functional
simulator's public lifecycle and architectural state while routing stored state,
arithmetic, sign testing, and instruction selection through reusable digital
logic primitives.

The model is an educational ISA-level netlist, not a transistor-accurate
reconstruction of the 1948 Williams-tube electronics. Host-language control
flow may sequence fetch, decode, and clock edges. Host integers may format
traces and select a store line after its five address bits have been decoded.
Architecturally visible data must not use host arithmetic or bitwise operators
to implement an instruction result.

## Architectural state

All persistent machine state is represented by D flip-flops from
`logic_gates::sequential`:

| Component | Width | Flip-flops |
|---|---:|---:|
| Williams-tube store | 32 words × 32 bits | 1,024 |
| Accumulator | 32 bits | 32 |
| Control instruction counter | 5 bits | 5 |
| Halt latch | 1 bit | 1 |
| **Total** | | **1,062** |

Bit vectors are always least-significant-bit first, matching the Rust
`logic-gates` and `arithmetic` packages. A register write uses a low clock for
setup and a rising edge to capture. Reset clocks zero into the store and
accumulator, one into every CI bit (line 31), and zero into the halt latch.

The external snapshot remains the Spec 07l shape: 32 `u32` words, `u32`
accumulator, five-bit `u8` CI, and a Boolean halted value. Converting flip-flop
outputs into host integers is observation, not instruction execution.

### Topology metrics

`flip_flop_count()` is the exact instantiated storage total above.
`gate_count()` is a stable educational gate-equivalent estimate, calculated as
follows:

| Component | Assumption | Gates |
|---|---:|---:|
| Persistent state | 1,062 flip-flops × 6 gates | 6,372 |
| Five-bit CI ripple adder | 5 full adders × 5 gates | 25 |
| 32-bit arithmetic ripple adder | 32 full adders × 5 gates | 160 |
| Shared arithmetic inverter bank | 32 NOT gates | 32 |
| Opcode decoder and SUB combiner | 3 NOT + 16 AND + 1 OR | 20 |
| Five-to-32 address decoder | 5 NOT + 32 four-AND chains | 133 |
| 32-word, 32-bit read selection | 32 × (32 AND + 31 OR) | 2,016 |
| Clock sequencing and control | documented estimate | 100 |
| **Total** | | **8,858** |

The address decoder, read-selection mux, and sequencer rows describe the
equivalent circuit topology. The simulator is permitted to use host control
flow for those ideal wire-selection operations, so it does not dynamically
invoke every estimated gate on each read. The count must therefore never be
presented as a transistor count, timing model, or count of gate function calls.

## Combinational paths

### Incrementer and relative jump

The mandatory pre-fetch CI increment uses a five-bit ripple-carry adder:

```text
CI[4:0] + 00001 -> next CI[4:0]
```

JRP selects the low five bits of `Store[S]` and feeds them into the same-width
ripple-carry path with the already-incremented CI. Carry out is discarded, so
the result wraps modulo 32. CMP conditionally clocks one more ripple increment
when accumulator bit 31 is high.

### Negation and subtraction

LDN negates all 32 bits of `Store[S]` with 32 NOT gates followed by a
32-bit ripple-carry addition with carry-in one:

```text
0 + NOT(Store[S]) + 1 -> accumulator
```

Both SUB encodings subtract through the same two's-complement path:

```text
accumulator + NOT(Store[S]) + 1 -> accumulator
```

Carry out and signed overflow are not architectural Baby state and are ignored.
All results therefore wrap modulo 2^32 exactly like the functional simulator.

### Decoder and data routing

Instruction bits 13–15 feed NOT gates and three-input AND compositions to
produce eight mutually exclusive select lines. The two SUB select lines join
through an OR gate before enabling the subtraction path. The five operand bits
select one of 32 store words. CMP tests accumulator bit 31 through the decoded
CMP select line. STP clocks one into the halt latch.

Host `match` statements may sequence only the one-hot control lines. They must
not re-decode the numeric opcode or calculate architectural data results.

### JMP and STO

JMP clocks the low five bits of the selected store word into CI. STO clocks all
32 accumulator bits into the selected store word. Other registers and store
lines retain their prior flip-flop state.

## Fetch-decode-execute cycle

One `step()` performs exactly these phases:

1. Reject the operation if the halt latch is set.
2. Clock `CI + 1` into the CI register.
3. Read the 32-bit instruction word selected by CI.
4. Decode function bits 13–15 into one-hot gate outputs and read operand bits
   0–4.
5. Route the selected data path and clock its architectural destination.
6. Return an owned trace matching the functional simulator's before/after CI,
   raw instruction, mnemonic, and description.

The externally visible behavior for JMP, JRP, CMP, wraparound, and
self-modifying STO is exactly Spec 07l.

## Public Rust contract

`ManchesterBabyGateLevel` exposes the same lifecycle vocabulary as
`BabySimulator`:

- `new()` and `Default` construct the documented power-on state.
- `reset()` restores every state element through register writes.
- `load(program, origin)` clocks complete little-endian words into consecutive
  store lines, ignores an incomplete trailing word, and stops at line 31.
- `step()` executes one gate-level cycle and returns a trace or typed halted
  error.
- `run(max_steps)` executes an already-loaded image with a mandatory bound.
- `execute(program, max_steps)` resets, loads at line zero, and runs.
- `get_state()` returns an owned snapshot with no mutable alias to the machine.
- `gate_count()` and `flip_flop_count()` expose stable educational topology
  metrics; their documentation must distinguish exact instantiated storage
  counts from estimated combinational/control counts.

Invalid origins and exhausted step bounds fail closed with typed errors. Loading
and execution never allocate from guest-provided size fields; trace retention is
bounded by the caller-provided `max_steps`.

## Conformance tests

The Rust package must cover:

- reset, loading, boundary truncation, and immutable snapshots;
- all eight function encodings, including both SUB spellings;
- five-bit and 32-bit arithmetic wraparound;
- negative and non-negative CMP paths;
- absolute and relative jumps;
- self-modifying STO and bounded loops;
- flip-flop persistence and stable topology counts;
- differential single-step and complete-program comparisons with
  `manchester-baby-simulator` for deterministic programs and seeded generated
  state/program cases.

The package must pass tests, Rustfmt, Clippy with warnings denied, rustdoc with
warnings denied, its `BUILD` recipe, and the repository affected-package build.
Line coverage must be at least 80%, with a target of 95%.

## Backlog mapping

This specification is RCPU-002 in `RUST-CPU-SIMULATOR-BACKLOG.md`. Completion
unblocks the chronological move to RCPU-003, the IBM 704 functional simulator.

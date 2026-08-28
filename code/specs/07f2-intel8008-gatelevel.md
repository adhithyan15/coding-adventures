# 07f2 — Intel 8008 Gate-Level Simulator

## Status and scope

This specification defines the Rust `intel8008-gatelevel` package. It is an
instruction-level educational gate model of Intel's 1972 8008: it implements
the complete instruction and lifecycle surface specified by
`07f-intel8008-simulator.md`, but architectural datapaths use repository logic
gates, adders, and D flip-flops.

It is not a transistor-accurate die reconstruction or a timing-accurate model
of the physical two-phase clock. Host control flow may sequence one complete
instruction per `step`; host integers may index arrays and format traces.
Neither exception permits host arithmetic in an architectural datapath or host
storage for mutable architectural state.

## Architecture

The model has:

- seven physical 8-bit registers: A, B, C, D, E, H, and L;
- the M pseudo-register, which aliases memory at `(H & 0x3F) << 8 | L`;
- four flags: carry/borrow, zero, sign, and even parity;
- an eight-word, 14-bit push-down stack whose word zero is the live PC;
- 16,384 bytes of unified code/data memory;
- eight 8-bit input ports and twenty-four 8-bit output ports;
- one halt latch.

Instruction encodings, overlap rules, flag semantics, call-stack behavior, and
trace mnemonics are identical to the functional 07f oracle.

## Gate fidelity

### Decode

The opcode decoder derives group, register, ALU, control-flow, I/O, immediate,
and halt signals with AND, OR, and NOT gates. The unconditional JMP/CAL and HLT
patterns are gate expressions, not host opcode comparisons. Host branches may
route already-decoded control signals to components.

### Arithmetic and logic

- ADD/ADC and SUB/SBB/CMP use the arithmetic crate's ripple-carry ALU.
- INR and DCR add or subtract one through the same gate-backed arithmetic path.
- ANA, XRA, and ORA apply one gate per corresponding result bit.
- zero uses an OR reduction; parity uses XOR reduction followed by NOT.
- rotates are bit-vector rewiring with gate-backed stored inputs and outputs.
- the 14-bit PC increment is a half-adder chain using XOR and AND at every bit.

### Sequential state

Every mutable architectural bit is held in `FlipFlopState` and written through
the repository's two-phase `register` primitive. Reading samples the stored
slave-latch output without changing state.

| Persistent component | Width/count | D flip-flops |
|---|---:|---:|
| Unified memory | 16,384 × 8 | 131,072 |
| Physical registers | 7 × 8 | 56 |
| Push-down stack | 8 × 14 | 112 |
| Stack selector/depth | 3 | 3 |
| CY/Z/S/P | 4 | 4 |
| Halt | 1 | 1 |
| Input latches | 8 × 8 | 64 |
| Output latches | 24 × 8 | 192 |
| **Exact total** |  | **131,504** |

M has no physical register. The PC is stack word zero. Both facts are required
to avoid fictitious or double-counted storage. `FLIP_FLOP_COUNT` is normative
and must equal 131,504.

## Public Rust contract

The main type is `GateLevelCpu`.

### Construction and reset

`new()` initializes every DFF to zero. `reset()` clears registers, stack and
selector, flags, halt, and output latches. It preserves unified memory and
external input latches, matching the functional oracle's distinction between
CPU reset and external stimulus.

### Loading

```text
load_program(program: &[u8], start: usize) -> Result<(), Intel8008Error>
```

Loading overlays bytes without clearing other memory. The range is calculated
with checked arithmetic. An overflowing origin or any byte beyond address
`0x3FFF` returns `ProgramOutOfRange` before a DFF changes. An empty load is valid
only at origins zero through 16,384.

### Stepping

```text
step() -> Result<Trace, Intel8008Error>
```

Preflight classifies the opcode and exact encoded width before fetch. Undefined
bytes return `UnknownOpcode`; a 2- or 3-byte instruction crossing the end of
memory returns `TruncatedInstruction`; stepping after HLT returns `Halted`.
Every failure leaves the complete state unchanged.

On success, `Trace` must be exactly equal to the functional trace, including
address, raw bytes, mnemonic, accumulator/flags before and after, and indirect
memory metadata.

### Bounded execution

```text
run(program: &[u8], max_steps: usize)
    -> Result<Vec<Trace>, Intel8008Error>
```

Run rejects an oversized image before mutation, builds a zeroed candidate
machine with the caller's input latches, loads at address zero, and executes at
most `max_steps`. The candidate replaces the caller only after all requested
steps succeed or HLT is reached. Thus a load or execution error is transactional
for the whole run.

### I/O

```text
set_input_port(port: usize, value: u8) -> Result<(), Intel8008Error>
get_output_port(port: usize) -> Result<u8, Intel8008Error>
```

Input ports 0–7 and output ports 0–23 are valid. Other indices return the shared
typed range errors and never clamp, alias, or panic.

### Immutable inspection

`snapshot()` returns the functional crate's owned `Intel8008State`:

- `[u8; 8]` register encoding slots, with M's slot zero;
- a complete owned 16 KiB memory image;
- all eight 14-bit logical stack words and depth;
- flags and halt state;
- all input and output latches.

Mutating the machine after a snapshot cannot change the earlier value.

## Failure model

The package uses the shared `Intel8008Error` variants:

- `ProgramOutOfRange`;
- `TruncatedInstruction`;
- `UnknownOpcode`;
- `Halted`;
- `InputPortOutOfRange`;
- `OutputPortOutOfRange`.

There are no stringly execution errors, silent program truncation, port
clamping, swallowed run failures, or caller-triggerable panics in the public
lifecycle boundary.

## Required conformance

Completion requires all of the following:

1. Existing component and program tests remain green.
2. Every one of 256 first opcode bytes is independently classified.
3. Every defined encoding executes one step in both simulators with equal raw
   width, full trace, and complete post-state.
4. Every undefined encoding returns the same typed error and preserves state.
5. End-of-memory, halted, oversized, overflowing-origin, and invalid-port
   failures are typed and atomic.
6. Multi-instruction ALU, memory-indirect, branch, restart/call/return, and I/O
   workloads compare full traces and snapshots with the functional oracle.
7. The exact 131,504-DFF topology is pinned by a test.
8. Core line coverage is at least 80%, with 95% as the target.
9. Formatting, tests, Clippy with warnings denied, and rustdoc with warnings
   denied all pass.

## Build integration

The package is a member of the Rust workspace and has a repository `BUILD`
recipe. From `code/packages/rust`:

```text
cargo test -p coding-adventures-intel8008-gatelevel --no-fail-fast
cargo clippy -p coding-adventures-intel8008-gatelevel \
  --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc \
  -p coding-adventures-intel8008-gatelevel --no-deps
```

The gate package depends on the functional package for the shared state, trace,
error contract, and differential oracle. It does not duplicate those public
types.

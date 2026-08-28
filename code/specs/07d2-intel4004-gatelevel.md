# 07d2 — Intel 4004 Gate-Level Simulator

## Scope

The Rust `intel4004-gatelevel` package is the gate-backed partner of the
functional `intel4004-simulator` defined by Spec 07d. Both implement the same
46-instruction Intel 4004 surface plus the repository's `HLT` test opcode.

This is an educational ISA-level gate model, not a transistor-accurate die or
cycle-accurate reconstruction. Host values may control dispatch, ROM addressing,
and trace construction. Architectural arithmetic, logic, decode, PC increments,
selector formation, and persistent state must use the repository's Rust gate,
adder, and sequential primitives.

## Required ISA surface

The gate simulator implements every encoding accepted by the functional oracle:

- machine control: `NOP`, repository `HLT`;
- register and immediate: `LDM`, `LD`, `XCH`, `INC`;
- arithmetic and carry: `ADD`, `SUB`, `CLB`, `CLC`, `IAC`, `CMC`, `CMA`,
  `RAL`, `RAR`, `TCC`, `DAC`, `TCS`, `STC`, `DAA`, `KBP`;
- control: `JCN`, `JUN`, `JMS`, `ISZ`, `BBL`;
- register-pair and indirect: `FIM`, `SRC`, `FIN`, `JIN`;
- RAM and ports: `WRM`, `WMP`, `WRR`, `WPM`, `WR0`–`WR3`, `SBM`, `RDM`,
  `RDR`, `ADM`, `RD0`–`RD3`;
- bank selection: `DCL`.

The arithmetic, BCD, page-relative target, three-level circular stack, RAM,
status, port, and selector semantics are identical to Spec 07d.

## Gate provenance

### Combinational paths

- The 8-bit instruction decoder recognizes families with AND/OR/NOT networks.
- The 4-bit ALU uses the `arithmetic` crate's gate-backed ALU for add,
  complement, increment, decrement, AND, and OR.
- Subtraction is complement-add through the same ALU.
- PC increments traverse a 12-stage half-adder chain.
- JCN and ISZ predicates reduce accumulator/register bits with gate trees.
- DCL and SRC selector masks pass through the gate ALU.

### Persistent state

All mutable architectural state is stored in D flip-flops. Read-only program
ROM is external and excluded from the count.

| State | D flip-flops |
|---|---:|
| R0–R15 | 64 |
| Accumulator and carry | 5 |
| Program counter | 12 |
| Three return registers and stack pointer | 38 |
| RAM main characters | 1,024 |
| RAM status characters | 256 |
| RAM output ports | 16 |
| RAM bank/register/character selectors | 8 |
| ROM port | 4 |
| Halt state | 1 |
| **Exact total** | **1,428** |

`FLIP_FLOP_COUNT` is the public exact topology constant. `gate_count()` remains
an educational estimate of gates in storage and combinational paths and must be
larger than six gates per persistent flip-flop.

## Checked public contract

```rust
impl Intel4004GateLevel {
    pub fn new() -> Self;
    pub fn reset(&mut self);
    pub fn load_program(&mut self, program: &[u8])
        -> Result<(), Intel4004Error>;
    pub fn step(&mut self) -> Result<GateTrace, Intel4004Error>;
    pub fn run(&mut self, program: &[u8], max_steps: usize)
        -> Result<Vec<GateTrace>, Intel4004Error>;
    pub fn snapshot(&self) -> GateState;
}
```

The gate package shares `Intel4004Error` with the functional package. It rejects
oversized ROM images, illegal encodings, truncated two-byte instructions, and
steps after halt. Rejected loads and steps leave the complete snapshot unchanged.
`run` grows its trace only as instructions are accepted and must not allocate
from an untrusted `max_steps` value.

`GateState` owns the full ROM, registers, PC, stack and pointer, RAM and status,
ports, selectors, carry, accumulator, and halt state. Callers cannot mutate the
machine through a snapshot.

## Acceptance tests

Completion requires:

1. Component tests for gate conversion, ALU, decoder, registers, PC, stack, RAM,
   and ports.
2. Full-state and trace differential execution against `intel4004-simulator`
   for every specified first-byte encoding.
3. Differential workloads covering RAM/status/ports, BCD and arithmetic,
   taken/fall-through page branches, indirect fetch/jump, and circular stack.
4. Atomic oversized-load, illegal-opcode, truncated-fetch, halted-step, and
   unbounded-limit boundaries.
5. Reset/snapshot determinism and the exact 1,428-DFF topology.
6. A working BUILD recipe, workspace membership, format, warnings-denied Clippy
   and rustdoc, and at least 80% core line coverage with a 95% target.

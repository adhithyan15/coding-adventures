# Layer 07ac2 — IAS Educational Gate-Level Simulator

## Purpose

This package is the gate-level partner for Layer 07ac's checked IAS machine
simulator. It models the published programmer-visible 1951 IAS architecture as
a stable Boolean topology. It does **not** claim to reproduce undocumented
wire routing, individual vacuum-tube placement, Williams-tube timing, or the
complete physical main-control pulse network.

The checked functional simulator remains the lifecycle and fault oracle. Each
successful order is also evaluated by an independent gate transition engine.
Persistent state is clocked only after its complete trace and architectural
state agree with the oracle.

## Persistent topology

Every architecturally persistent bit is stored in a simulated master/slave D
flip-flop:

- 4,096 40-bit memory words: 163,840 DFFs;
- accumulator and arithmetic register: 80 DFFs;
- control counter including the one-past sentinel: 13 DFFs;
- buffered-right valid bit and 20-bit order: 21 DFFs;
- current control register: 20 DFFs;
- function table register: 8 DFFs;
- memory address register: 12 DFFs;
- Selectron / memory buffer register: 40 DFFs;
- halt: 1 DFF; and
- installed origin and word count: 12 and 13 DFFs.

The exact total is **164,060 persistent DFFs**. Packed stable-Q memory is
permitted, but reset, restore, direct writes, program loading, partial stores,
and instruction commits must clock every selected word through repository DFF
primitives. The package reports a separate finite combinational-gate estimate.

## Combinational transition engine

The independent transition engine covers the complete Layer 07ac order set,
both control halves, and buffered-right behavior. Repository Boolean and
arithmetic networks must drive:

- one-hot opcode reconstruction and control selection;
- 40-bit add, subtract, negate, absolute value, shifts, and sign tests;
- signed 40-by-40 multiplication into the 80-bit `AC:AR` pair;
- signed divide with zero and quotient-overflow detection;
- 12-bit address generation and conditional left/right control transfer;
- address-field masks for both partial-store orders; and
- packed-memory write enables.

Host orchestration may dispatch on values reconstructed from asserted gate
lines and may compare the settled gate transition with the functional oracle.
It may not call the functional implementation to produce a successful gate
transition.

## Checked lifecycle

The public machine reuses `IasState`, `Half`, `Opcode`, `StepTrace`,
`RunResult`, and `IasError`. It provides deterministic construction, reset,
snapshot, restore, five-byte big-endian and structured-word loading, direct
memory and register access, one-order execution, bounded loaded execution, and
transactional load-and-execute helpers.

Every fault is typed and atomic. A later fault or exhausted bounded-run budget
restores the exact pre-run DFF state. Fetch never wraps the installed image,
and trace allocation remains bounded by the functional package's public limit.

## Conformance

Completion requires:

1. full-state and trace agreement for all 22 order classifications, both
   halves, taken and untaken branches, arithmetic edges, and buffered
   self-modification behavior;
2. matching typed and atomic failures for transport, restore, bounds, decode,
   divide, halt, fetch, and run-budget errors;
3. tests for exact DFF accounting and the combinational networks;
4. release tests, strict Clippy, rustdoc warnings, formatting, package BUILD,
   and repository validation passing; and
5. at least 95 percent Rust line coverage for the gate package.

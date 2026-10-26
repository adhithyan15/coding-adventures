# Layer 07ah2 — Motorola MC6809 Educational Gate-Level Simulator

## Purpose and evidence boundary

This package is the gate-level partner for Layer 07ah's checked MC6809
functional simulator. It models the published programmer-visible Motorola
MC6809 architecture as a stable Boolean topology. It does **not** claim to
reconstruct Motorola's proprietary mask artwork, transistor placement,
propagation delays, or undocumented internal control wiring.

The checked functional simulator remains the lifecycle and fault oracle. Each
successful instruction is also evaluated by an independent gate transition
engine. Persistent state is clocked only after the exact trace and complete
architectural state agree with that oracle.

## Persistent topology

Every architecturally persistent bit is stored in a simulated master/slave D
flip-flop:

- 65,536 bytes of memory: 524,288 DFFs;
- accumulators A and B: 16 DFFs;
- X, Y, U, S, and PC: 80 DFFs;
- direct-page and condition-code registers: 16 DFFs;
- deterministic wait/halt latch: 1 DFF; and
- installed origin and byte count: 16 and 17 DFFs.

The exact total is **524,434 persistent DFFs**. Packed stable-Q memory is
permitted, but reset, restore, direct writes, program loading, stack traffic,
read-modify-write operations, and instruction commits must clock selected
bytes through repository DFF primitives. The package reports a separate
finite combinational-gate estimate.

## Combinational transition engine

The independent engine covers the complete Layer 07ah base, page-2, and page-3
opcode surfaces and every documented indexed postbyte class. Repository Boolean
and arithmetic networks drive:

- one-hot reconstruction of prefixes, opcodes, and indexed postbytes before
  host dispatch;
- 8- and 16-bit add/subtract, carry/borrow, half-carry, overflow, sign, zero,
  compare, negate, shift, rotate, decimal-adjust, and multiply networks;
- direct, extended, relative, PC-relative, accumulator-offset, auto-update,
  indirect, stack, vector, and wrapped 16-bit address generation;
- branch predicates and transfer/exchange width selection;
- condition-code updates, stack masks, and interrupt-frame control; and
- packed-memory write enables.

Host orchestration may dispatch only on values reconstructed from asserted
gate lines and may compare the settled transition with the functional oracle.
It may not call the functional implementation to produce a successful gate
transition.

## Checked lifecycle

The public machine reuses `Motorola6809State`, `StepTrace`, `ExecutionResult`,
and `Motorola6809Error`. It provides deterministic construction, reset,
restore, raw-byte loading, direct memory and register access, one-instruction
execution, bounded loaded execution, and transactional load-and-execute.

Every failure is typed and atomic. A failing instruction or exhausted bounded
run restores the exact pre-transition DFF state. Fetch never wraps outside the
installed image. Trace retention remains bounded by the functional package's
public 1,024-step limit.

## Conformance

Completion requires:

1. full-state and trace agreement for every documented base/page-2/page-3
   encoding, all 217 legal indexed postbytes, both outcomes of every branch,
   stacks, software interrupts, wait state, and malformed encodings;
2. matching typed and atomic transport, restore, bounds, fetch, decode,
   addressing, halted, retention, and run-budget faults;
3. exact DFF accounting and focused combinational-network tests;
4. release tests, strict Clippy, rustdoc warnings, formatting, package BUILD,
   repository validation, and security review; and
5. at least 95 percent Rust line coverage for the gate package.

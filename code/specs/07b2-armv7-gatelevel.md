# 07b2 — ARMv7 Educational Gate-Level Simulator

## Scope

This specification defines the gate-level partner for the completed Spec 07b
functional machine. It implements the same little-endian 32-bit instruction
surface: MOV, ADD, SUB, CMP, AND, ORR, word LDR/STR, B/BL, all sixteen ARM
condition predicates, the S bit and NZCV, rotated immediates, and custom HLT.

## Exact persistent topology

The gate machine contains exactly 524,805 persistent D flip-flops:

```
64 KiB memory              65,536 × 8 = 524,288
R0–R15                         16 × 32 =     512
NZCV                                           4
halt                                           1
                                             -------
                                             524,805
```

R15 is the only PC storage; snapshots expose a matching `pc` convenience
field. Installed origin and length are checked loader metadata and are not
counted as architectural flip-flops. Packed memory bytes represent stable Q
bits exactly and reconstruct the master/slave latch state when clocked.

## Combinational execution

- Condition evaluation implements all sixteen predicates from AND, OR, NOT,
  XOR, and XNOR gates.
- Instruction class and opcode equality are gate comparator networks.
- AND/OR and N/Z calculation are per-bit gate networks.
- ADD, SUB, CMP, effective addresses, sequential PC, and PC+8 branch targets
  use 32-bit ripple-carry adders.
- Rotated immediates pass through four 32-bit mux stages selecting rotations
  by 2, 4, 8, and 16 bits; its carry feeds logical S-bit flag updates.
- Architectural writes clock DFFs. A fault restores the entry stable-Q image,
  so no partial transition is observable.

## Lifecycle

`Armv7GateLevel` consumes and emits the functional `ArmState`, `ArmError`,
`StepTrace`, and `ExecutionResult` contract. State restore is fully validated;
loads reset deterministically and accept checked aligned origins; direct access
is typed; a checked step is atomic; and a bounded run rolls back on any late
fault or step-limit exhaustion.

## Verification

The gate machine must pass topology, DFF access, load/restore, fault atomicity,
trace, condition, data-processing, memory, control-flow, and HLT suites. It must
match every completed functional edge and all 388 reproducible Python
common-surface full-state vectors. Formatting, Clippy, and rustdoc are warnings-
as-errors gates. The completed implementation measures 93.40% package line
coverage (495/530 lines), above the 80% floor.

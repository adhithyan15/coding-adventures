# NN30: Backward and Optimizer Lowering Labs

## Status

Version 1 is implemented by the language-neutral fixture under
[`fixtures/backward-optimizer-lowering-v1`](./fixtures/backward-optimizer-lowering-v1/README.md).

## Purpose

NN29 lowered a forward neural graph to scalar NeuralIR and a batched matrix
plan. NN30 fixes the next contract: saved forward values lower to a backward
program, parameter gradients reduce into persistent buffers, and a separate
optimizer program consumes those buffers without clearing them.

The canonical graph is one trainable multiplication followed by half-squared
error. It is intentionally scalar so every value can be checked on paper.

## Required IR

The V1 corpus pins three normalized streams:

- `CANB` backward IR with six instructions;
- `CANO` optimizer IR with four instructions; and
- `CANM-TRAIN` matrix training IR with ten operations.

Identifiers, operation order, input/output names, attributes, and provenance
arrays are exact. Unknown fields are rejected.

Backward and optimizer functions remain separate. The optimizer receives an
explicit gradient divisor and does not implicitly zero the gradient buffer.

## Required Scenarios

The V1 roster and order are fixed:

1. `one_row_by_hand` uses `w=0.5`, `x=[2]`, `target=[0]`, and divisor `1`.
   It produces `grad_w=2`, `d_x=[0.5]`, and `w_next=0.3`.
2. `two_row_mean` uses `w=1`, `x=[2,-1]`, `target=[1,1]`, and divisor `2`.
   Row gradients `[2,2]` reduce to `4`, mean to `2`, and update `w` to `0.8`.
3. `persistent_buffer` enters with `grad_w=3`, adds the current row's
   contribution `2`, updates with the resulting buffer `5`, and leaves that
   buffer populated after the optimizer step.

Every scenario checks direct arithmetic, normalized backward execution,
normalized optimizer execution, and matrix training execution. A fresh central
finite-difference audit checks only the current batch contribution at epsilon
`1e-5`, so an older carried buffer cannot masquerade as a derivative of the
current batch loss.

## Determinism

- Backward instruction order is fixed by reverse dependency order.
- Optimizer instruction order is independent of backward execution.
- Batch columns retain input row order.
- Parameter-gradient reduction uses ascending row index.
- The canonical absolute tolerance is `1e-8` and cannot be loosened.

## Validation and Safety

- Reject duplicate JSON keys, non-finite constants, unknown fields, malformed
  identifiers, wrong schema IDs, and non-canonical rosters.
- Cap instruction streams at sixteen operations and scenarios at four.
- Cap batches at eight rows and text at 512 characters.
- Bound teaching inputs before arithmetic and every derived value afterward.
- Reject mismatched input/target lengths, non-positive divisors, and a divisor
  larger than the batch.
- Cap recursive expected-trace comparison depth.
- Recompute every expected IR and trace; checked-in expected objects are never
  trusted as execution inputs.

## Production Forward and Training Boundary

The TypeScript visualizer uses the production `neural-network` and
`neural-graph-vm` packages to compile and execute the forward multiplication.
V1 then normalizes the new training contract in the fixture. That contract is
the portable target for future training compilers; it does not pretend the
current NN00 forward-only bytecode already implements backward execution.

## Cross-Language and Rust Direction

Every language consumer should reproduce the normalized streams and traces
from JSON. A future Rust bridge may translate matrix operations to MX01 tensor
`Mul`, `Add`, and `ReduceSum` nodes, but the host must provide parameter IDs,
saved-value ownership, reduction/divisor policy, update scheduling, and explicit
gradient clearing.

A stable C ABI should use opaque handles or caller-owned typed slices with
explicit lengths, shapes, dtypes, and lifetimes. It must not retain raw
host-object pointers or make optimizer/zeroing choices implicitly.

## Versioning

V1 is append-only. Changing instruction semantics, operation order, scenario
roster, tolerance, reduction order, or optimizer-zeroing behavior requires a
new fixture version.

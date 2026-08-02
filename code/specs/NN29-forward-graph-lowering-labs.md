# NN29: Forward Graph Lowering Learning Labs

Status: active

## Purpose

NN29 defines a deterministic teaching corpus for translating one feed-forward
neural graph into NN00 NeuralIR and the NN01 matrix plan used as the neural VM's
MatrixIR-shaped execution bridge.

The corpus is intentionally smaller than the production compiler surface. It
pins one weighted-sum/ReLU/output graph, one hand-calculable row, one two-row
batch, exact normalized IR, fusion provenance, and three-path numerical parity.

## Fixture location

```text
code/specs/fixtures/forward-graph-lowering-v1/
```

The corpus contains one JSON lab and a strict Draft 2020-12 schema.

## Canonical graph

The graph has two runtime inputs, one constant bias, one weighted sum, one ReLU,
and one named output. Stable node tie-breaking and stable incoming-edge ordering
produce a twelve-instruction `CANN` forward function.

The NN01 lowering fuses the three edge-weight loads, three multiplies, and the
add into one `WEIGHTED_SUM_MATRIX` operation. The normalized `CANM` plan has six
operations and retains all contributing source instruction and edge IDs.

## Required execution paths

A conforming validator must execute:

1. direct graph semantics;
2. the normalized NeuralIR instruction stream row by row; and
3. the normalized MatrixIR plan columnwise.

All three paths must match each fixture expectation and each other at the
canonical absolute tolerance `1e-10`.

## Determinism

- Node scheduling uses topological order with lexical node-ID tie-breaking.
- Incoming weighted edges use lexical edge-ID order.
- Neural value IDs are allocated monotonically as `v0`, `v1`, ... .
- Neural instruction IDs are `i0`, `i1`, ... .
- Matrix operation IDs are `m0`, `m1`, ... .
- Reordering JSON node or edge records must not change either lowered IR.

## Validation boundary

Validators must fail closed on duplicate JSON keys, non-finite numbers, unknown
fields, duplicate IDs, unknown edge endpoints, cycles, unsupported operations,
unsupported activations, invalid source mappings, oversized inputs, dishonest
expected IR, or parity drift.

Teaching graphs are bounded to 16 nodes, 24 edges, 4 examples, and batches of 8
rows. Input, constant, and weight magnitudes are bounded before arithmetic, and
every derived value must remain finite and no larger than `1e12` in magnitude.

## Cross-language and Rust direction

Every language may implement this normalized compiler and compare its output
directly with the fixture. A high-performance bridge should translate the
validated NN01 plan into the Rust MX01 `matrix-ir` tensor DAG using explicit
tensor shapes, dtypes, constants, inputs, and outputs.

The Rust core may own typed tensor algebra, serialization, planning, and kernel
dispatch. It must not infer neural source meaning or discard graph provenance.
Backward, optimizer, placement, precision, and residency lowering are outside
NN29 and remain separate roadmap tranches.

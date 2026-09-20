# CLR12: strict scalar acyclic control flow

Status: historical companion contract, committed before the original PR #15763
implementation. PR #15762 landed equivalent forward-only behavior first.
`CLR12-strict-forward-scalar-control-flow.md` is authoritative. This document
preserves the independent original contract; reconciled #15763 adds validation
coverage only and retains the landed production implementation, including its
preflight destination bound. No second control-flow implementation is introduced.

## Baseline and scope

CLR08 established strict i32/i64/bool lowering, and CLR10 added simulator
execution for both short and promoted long branches. Extend only
`lower_typed_scalars_to_cil` with IIR `label`, `jmp`, `jmp_if_true`, and
`jmp_if_false`. Labels and branches have the canonical `void` hint, no
destination, and Var-shaped label operands. Conditional branches take exactly
`[Var(bool_condition), Var(label)]`; integer truthiness is not accepted at this
strict boundary.

This first control-flow slice is deliberately acyclic: every branch target
must be a unique, non-empty label later in the same function. The builder may
choose short or long CIL encodings. Back edges, indirect targets, duplicate or
missing labels, malformed control shapes, and non-boolean conditions must fail
closed before an artifact is returned.

## Reachability and definite assignment

Treat fallthrough and branch edges as a forward control-flow graph. Every
instruction must be reachable from function entry, and every reachable path
must terminate in a correctly typed `ret`; the final instruction therefore
remains a return. Multiple reachable returns are permitted.

Parameters are definitely assigned at entry. A value destination becomes
assigned only after its defining instruction. At a join, a variable is
definitely assigned only when it is assigned on every incoming reachable edge.
Every value operand, return value, call argument, comparison operand, and
branch condition must be definitely assigned at its use. Preserve the existing
single-assignment namespace, scalar-width checks, 256 argument/local limits,
method-token checks, CLR01 literal/input gates, and default source routing.

## Validation

Add focused backend refusal tests for malformed, backward, unreachable, and
path-dependent programs. Add exact short-branch byte assertions and
builder-promotion coverage. Execute true/false branches, an if/else join, and
early returns through `clr-simulator`, checking normalized bool transport and
i32/i64 results. Run the complete `iir-to-cil-bytecode` and `clr-simulator`
suites, the affected `lang-aot` strict-scalar suite, strict Clippy for the
changed backend, and repository diff checks.

## Exclusions

Loops and general cyclic CFG analysis, phi nodes, mutable locals, host input,
new scalar types, source-routing changes, and integer conditions remain future
work. This slice does not change the legacy dynamic CLR lowering path.

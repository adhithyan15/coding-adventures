## Unreleased — `realizes` bindings to the Warrant gate model (VER33 ADL-002b)

- `evidence.bindings`: each of the eleven scalar gates gains
  `realizes=logic_gates_model.<gate> embedding=int-by-value` and its declared `panic` templates
  (`a must be 0 or 1, got {}` and, where reachable, `b must be 0 or 1, got {}`), copied from the
  format string of `validate_bit`. Nothing consumes a realization yet; the generated evidence
  is unchanged.
- New `tests/realizes_panics.rs`: every real panic of every gate, for non-bits in every
  position, matches exactly one declared template, and every declared template is raised.
  Added to the debug and release test lines of `BUILD`, `BUILD_linux` and `BUILD_windows`.
- The bindings are validated (W0806/W0807 clean) against `logic-gates.warrant` and
  `logic-gates-model.warrant` by warrant-grammar's `tests/logic_gates_realizes.rs`.

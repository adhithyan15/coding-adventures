## Unreleased — validation of the logic-gates `realizes` bindings (VER33 ADL-002b)

- New `tests/logic_gates_realizes.rs`: parses `logic-gates/evidence.bindings`, runs
  `validate_realizations` against `logic-gates.warrant` and `logic-gates-model.warrant`
  (W0806/W0807 clean), requires exactly the eleven gates to realize their own-named model
  function by value, ties the declared templates to the `assert!` in `validate_bit`
  (primitives exactly, NAND-built gates as an upper bound; the exact set is
  `logic-gates/tests/realizes_panics.rs`), and checks that no `panic!`, `expect`, index or
  declared compiler-generated message is in a gate body besides the undeclared `.unwrap()`.

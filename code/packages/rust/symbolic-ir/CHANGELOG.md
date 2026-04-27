# Changelog — symbolic-ir (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `symbolic-ir` package.
- `IRNode` enum with six variants: `Symbol`, `Integer`, `Rational`, `Float`,
  `Str`, `Apply`.
- `IRApply` struct with `head: IRNode` and `args: Vec<IRNode>`.
- Manual `PartialEq`, `Eq`, `Hash` impls for `IRNode`; floats compared by
  `f64::to_bits()` for deterministic equality.
- `IRNode::rational(numer, denom)` constructor with GCD reduction and
  sign normalisation; collapses to `Integer` when denominator reduces to 1.
- Convenience constructors: `sym`, `int`, `rat`, `flt`, `str_node`, `apply`.
- `Display` impls for `IRNode` and `IRApply` matching the Python `__str__`
  format.
- Standard head-name constants: `ADD`, `SUB`, `MUL`, `DIV`, `POW`, `NEG`,
  `INV`, `EXP`, `LOG`, `SIN`, `COS`, `TAN`, `SQRT`, `ATAN`, `ASIN`, `ACOS`,
  `SINH`, `COSH`, `TANH`, `ASINH`, `ACOSH`, `ATANH`, `D`, `INTEGRATE`,
  `EQUAL`, `NOT_EQUAL`, `LESS`, `GREATER`, `LESS_EQUAL`, `GREATER_EQUAL`,
  `AND`, `OR`, `NOT`, `IF`, `LIST`, `ASSIGN`, `DEFINE`, `RULE`.
- 24 unit tests + 8 doc-tests; all passing.

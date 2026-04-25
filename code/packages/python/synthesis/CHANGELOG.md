# Changelog

## [0.1.0] — Unreleased

Initial implementation of HIR -> HNL synthesis.

### Added
- `synthesize(hir) -> Netlist`: top-level entrypoint. Walks every Module in the HIR and produces an HNL Module with generic-cell instances.
- Operator lowering:
  - Bitwise binary: `&`/AND, `|`/OR, `^`/XOR, NAND, NOR, XNOR -> N parallel cells.
  - Adder: `+` on N-bit operands -> ripple-carry chain of full-adders (1-bit FA = 2 XOR2 + 2 AND2 + 1 OR2).
  - Unary NOT -> N parallel NOT cells.
  - Reduction AND/OR/XOR -> balanced binary tree of cells.
- Literal -> CONST_0 / CONST_1 cells (one per bit).
- Slice -> BUF chain selecting source bits.
- Concat -> BUFs packing parts MSB-first into a fresh net.
- LValue assignment: NetRef / PortRef / Slice / Concat handled.
- Width inference from HIR types (TyLogic = 1 bit; TyVector reads `.width`).
- 4-bit adder smoke test produces correct gate count and structure.

### Out of scope (v0.2.0)
- Sequential always blocks / FF inference.
- FSM extraction.
- Subtraction, multiplication, division, shifts, comparison operators.
- Mux trees from `if`/`case`.
- Optimization passes.
- Latch inference + warnings.

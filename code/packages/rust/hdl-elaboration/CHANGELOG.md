# Changelog — hdl-elaboration

## [0.1.0] — 2026-06-13

### Added

- Three-pass Verilog elaborator: collect module declarations → bind ports and
  continuous assignments → determine top module.
- `elaborate_verilog(source)` — parse and elaborate, top = first module declared.
- `elaborate_verilog_with_top(source, top)` — explicit top module selection.
- `elaborate(ast, top_override)` — elaborate a pre-parsed `GrammarASTNode`.
- `ElaborationError` enum with variants `ParseError`, `TopModuleNotFound`,
  `InvalidModule`, `InvalidPort`, `InvalidExpr`.
- Port elaboration: direction (`input`/`output`/`inout`), ANSI direction
  inheritance, scalar (`Ty::Bit`) and vector (`Ty::vec(width)`) port types.
- Continuous assignment elaboration (`assign lhs = rhs;`).
- Full Verilog expression tower: binary chains, ternary, unary, primary (name,
  number literal, slice), concatenation, replication.
- Operator mappings: arithmetic (`+`, `-`, `*`, `/`, `%`, `**`), bitwise
  (`AND`, `OR`, `XOR`, `NAND`, `NOR`, `XNOR`), shift (`<<`, `>>`, `<<<`,
  `>>>`), comparison (`==`, `!=`, `===`, `!==`, `<`, `<=`, `>`, `>=`),
  logical (`&&`, `||`).
- Unary operators: `POS`, `NEG`, `LOGIC_NOT`, `NOT`, `AND_RED`, `OR_RED`,
  `XOR_RED`, `NAND_RED`, `NOR_RED`, `XNOR_RED`.
- 16 integration tests + 1 doctest; all pass.
- Reference-based AST helpers in `ast.rs` avoid allocations during elaboration.

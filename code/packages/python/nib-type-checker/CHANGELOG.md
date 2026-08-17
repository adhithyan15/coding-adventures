# Changelog — coding-adventures-nib-type-checker

All notable changes to this package are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). This project
uses [Semantic Versioning](https://semver.org/).

## [0.1.1] — 2026-08-17

### Investigated

- `nib.grammar` gained a new `shift_expr` precedence level between
  `add_expr` and `mul_expr` (task #11257, "lower shift expressions"):
  `add_expr = shift_expr { ... }` / `shift_expr = mul_expr { (SHL|SHR)
  mul_expr }`. The equivalent TypeScript packages (`nib-formatter`,
  `nib-type-checker`) needed an explicit `"shift_expr"` entry added to a
  rule-name allowlist Set to avoid choking on the new wrapper node, and it
  was suspected this Python package would need the same fix.
- Confirmed by direct testing that **no code change is needed here**.
  `NibTypeChecker` dispatches expression nodes through
  `GenericTypeChecker.dispatch()`/`_check_ast_expr()`, whose fallback path
  unwraps *any* unrecognised single-child rule node (`if len(node.children)
  == 1: return self._check_expr(node.children[0], scope)`) regardless of
  its `rule_name`. This is the same generic mechanism that has always kept
  the pre-existing `mul_expr` level (added in #5677, itself never
  registered as an explicit hook here) transparent, so the new
  `shift_expr` level — a transparent single-child wrapper whenever no
  `<<`/`>>` operator is present — is handled correctly with no additional
  code. The handful of hardcoded rule-name lists elsewhere in
  `checker.py` (`_check_let_stmt`, `_check_assign_stmt`,
  `_check_return_stmt`, `_check_for_stmt`, `_check_if_stmt`,
  `_check_call_expr`) only ever match the immediate child of a statement
  node, which is always the `expr` rule itself (already present in every
  one of those lists) — they are unaffected by rule names added deeper in
  the expression tree.
- Also confirmed (contrary to an initial assumption) that Python's
  `nib-lexer` *does* tokenize `SHL`/`SHR` (it loads the shared
  `nib.tokens` file at runtime, same as the Rust/TS frontends), so
  `shift_expr` nodes are not unconditionally single-operand — real `<<`/`>>`
  usage is parsed. Real shift/multiplication semantics (e.g. `a << b`,
  `a * b`) are not fully type-checked by this package (there is no
  registered hook for `mul_expr`/`shift_expr` operator nodes with 3+
  children, so `_check_ast_expr` falls through to `None` for those), but
  that is a pre-existing gap dating back to `mul_expr`'s introduction in
  #5677 and is out of scope for this fix — this change only concerns the
  transparent-wrapper regression pattern.

### Added

- Regression tests `test_plain_add_two_variables` and
  `test_plain_add_two_variables_type_mismatch` in
  `tests/test_nib_type_checker.py`, exercising a plain 2-operand `a + b`
  (both `NAME` operands, no shift operator) to lock in the transparent
  `shift_expr` pass-through behaviour above.

## [0.1.0] — 2026-04-12

### Added

- `NibType` enum: four types (`U4`, `U8`, `BCD`, `BOOL`) with `size_bytes`
  property and helpers (`parse_type_name`, `types_are_compatible`,
  `is_bcd_op_allowed`, `is_numeric`).
- `Symbol` dataclass: name, type, is_const, is_static, is_fn, fn_params,
  fn_return_type.
- `ScopeChain` class: push/pop/define/lookup/define_global implementing
  lexical (static) scoping with a stack of dictionaries.
- `NibTypeChecker` class implementing `TypeChecker[ASTNode, ASTNode]` from
  `type-checker-protocol`. Performs two-pass type checking:
  - Pass 1: collect const/static/fn signatures into global scope; build
    call graph for recursion detection.
  - Pass 2: walk each function body, checking all statements and expressions.
- Checks enforced:
  1. All names declared before use (variables, functions).
  2. Expression types correct bottom-up (arithmetic, logical, comparison).
  3. Assignment LHS type == RHS type (no implicit widening).
  4. Function call argument types match parameter types; argument count
     matches.
  5. BCD operator restriction: only `+%` and `-` are legal for `bcd`
     operands.
  6. For-loop bounds must be integer literals or `const`-declared names.
  7. No recursion (direct or mutual), detected via DFS cycle check on the
     static call graph.
  8. `if` and `for` conditions must be `bool`.
  9. Return statements must match the declared return type.
- Module-level `check(ast)` convenience function.
- 80+ test cases organised into 12 categories covering valid programs,
  undeclared names, type mismatches, BCD restrictions, for-loop bounds,
  recursion, if-conditions, function call errors, return type errors,
  scope tests, NibType unit tests, and ScopeChain unit tests.

### Design notes

This package enforces *language-level* invariants only. Hardware constraints
(call depth ≤ 2, total RAM ≤ 160 bytes, register count) belong in the
`intel-4004-ir-validator`, which runs after IR generation. This keeps
the type checker target-independent — the same checker works for any
compilation target.

PR 5 of the Nib compiler pipeline (lexer → parser → **type-checker** → IR
compiler → backend validator → code generator).

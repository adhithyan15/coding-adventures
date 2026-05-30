# Changelog

All notable changes to the `ruby-to-semantic-ir` crate will be documented in this file.

## [0.42.0] - 2026-05-30

### Added (Phase 14c (FC) — inheritance `class Foo < Bar`)

`class Foo < Bar` now lowers to `Stmt::ClassDef` with
`superclass: Some("Bar")` (semantic-ir 0.3.0's new field); a base class
`class Foo` keeps `superclass: None`.

- New `extract_superclass` helper scans the `class_statement` node's
  *direct* child tokens for the `<` separator (a `Name`-type token with
  value `"<"`) and returns the value of the next `Name` token — the
  superclass.  Only direct tokens are inspected, so a `<` comparison
  *inside* a body statement (`a < b`) is never mistaken for the
  superclass separator (body statements are `statement` nodes, not bare
  tokens).
- Inheritance composes with Phase 14b: a subclass body still hoists its
  `def`s to top-level Functions and preserves non-def statements in
  `ClassDef.body`.

New tests (+5): `class_with_superclass_records_parent_name`,
`base_class_has_no_superclass`,
`subclass_with_body_records_superclass_and_hoists_methods`,
`subclass_passes_sir_validator` (E2E lower → validate),
`comparison_in_class_body_is_not_mistaken_for_superclass`.
Test count: 193 → 198 (+5).

## [0.41.0] - 2026-05-30

### Changed (Phase 14b (FC) — class body with method defs + statements)

`class Foo … end` now lowers to `Stmt::ClassDef` with a **populated**
`body` (Phase 14a always emitted `body: vec![]`).  The class body's
*executable* statements — constant/expression assignments, bare
expressions, nested `class`/`module` declarations, loops, … — are
lowered in source order and preserved in `ClassDef.body`, instead of
being silently dropped.

- New `lower_class_body_statements` helper walks the class body's
  `statement` children once:
  - `def_statement` / `endless_def_statement` are **hoisted** to
    top-level `Function`s (unchanged — SIR v0 has no
    method-as-statement node, so a method can't live inside a
    `Vec<Stmt>`), contributing nothing to `body`.
  - every other statement is lowered via the shared
    `lower_statement_inner_multi` dispatch and pushed onto `body`.
- The `class_statement` arm no longer calls the recursive
  whole-body `collect_def_statements_from_body` pre-pass; hoisting is
  now per-direct-child.  A nested `class`/`module` is lowered via the
  normal dispatch (whose own arm hoists *its* direct `def`s), so every
  method is hoisted **exactly once** — no double-registration that
  would trip the validator's function-name-uniqueness check.
- A method-*only* class still produces an empty `body` (the methods
  hoist); the `module_statement` arm is unchanged (still a NilLit
  no-op + def hoist, pending Phase 14d's `ModuleDef`).

New tests (4): `class_body_preserves_executable_statement_and_hoists_method`,
`class_body_preserves_multiple_statements_in_source_order`,
`class_with_body_statements_passes_sir_validator` (E2E lower → validate),
`nested_class_methods_hoisted_exactly_once`.  The existing
`class_with_method_body_still_emits_class_def_and_hoists_method` is
retained (method-only → empty body) with an updated comment.

Test count: 189 → 193 (+4).

## [0.40.0] - 2026-05-29

### Added (Phase 14a (FC) — empty `class Foo; end`)

`class Foo; end` now lowers to a first-class
`Stmt::ClassDef { name: "Foo", body: vec![], span }` (semantic-ir
0.2.0's new SIR17 node), replacing the pre-14a behaviour where a
class declaration lowered to a no-op `ExprStmt(NilLit)`.

- New `extract_class_name` helper pulls the class name from the
  first `TokenType::Name` token of a `class_statement` node (the
  `class` keyword is `TokenType::Keyword`, so it is skipped).
- Emitting a `ClassDef` requests `Feature::Classes`, which is now
  materialised into the module manifest alongside the existing
  feature tally.
- **Empty-body only:** Phase 14a always lowers `body: vec![]`.  The
  pre-existing Phase 6f method-hoisting fallback is preserved — a
  non-empty class body still hoists its `def`s to top-level
  `Function`s, leaving the `ClassDef` body empty — so older fixtures
  with method bodies continue to validate.  Phase 14b will populate
  `body` directly and retire the hoist-as-fallback path.
- `module M; end` is unchanged: it continues to lower to the Phase
  6f `NilLit` no-op until a later phase introduces a module node.

### Tests

- `ruby-to-semantic-ir`: 184 → 189 (+5): empty-class → ClassDef,
  `Feature::Classes` request, validator E2E, verbatim-name
  preservation, class-with-method-body (ClassDef + hoist), and a
  pin that `module` still lowers to NilLit.

## [0.39.0] - 2026-05-28

### Added (Phase 9c (FC) — single-RHS tuple destructure)

`multi_assignment` now accepts the single-RHS shape that Phase 9b's
comments still flagged as deferred:

```ruby
a, b    = arr           # a == arr[0]; b == arr[1]
a, b, c = arr           # a == arr[0]; b == arr[1]; c == arr[2]
a, b    = make_pair()   # make_pair() evaluated once into a temp
```

The lowerer routes 1-RHS / ≥2-LHS / no-splat through a new helper
`lower_multi_assignment_single_rhs_destructure`.  The strategy:

1. Bind the single (already-lowered) RHS to a fresh
   `LetStarBinding(__multi_assign_t<N>_seq, rhs)` — `LetStarBinding`
   keeps the temp visible to the LHS-binding pass and side effects
   in the RHS fire exactly once.
2. For each LHS position `i`, emit
   `Stmt::LetBinding`/`Stmt::Assign` reading
   `Expr::SeqIndex { seq: VarRef(temp), index: IntLit(i) }`.

| Source                | SIR shape (Phase 9c)                                                                                              |
|-----------------------|-------------------------------------------------------------------------------------------------------------------|
| `a, b = arr`          | `LetStarBinding(t0_seq, arr); LetBinding(a, SeqIndex(t0_seq, 0)); LetBinding(b, SeqIndex(t0_seq, 1))`             |
| `a, b, c = arr`       | `LetStarBinding(t0_seq, arr); LetBinding(a, SeqIndex(t0_seq, 0)); LetBinding(b, SeqIndex(t0_seq, 1)); LetBinding(c, SeqIndex(t0_seq, 2))` |
| `a = 0; a, b = arr`   | re-bind path: stmt for `a` is `Assign`, requests `Feature::MutableBindings`                                       |

Out-of-bounds semantics are target-language-defined per
`Expr::SeqIndex`'s docs.  Ruby itself fills missing positions with
`nil`; matching that exactly is left to the backend or a future
phase.

### Arity check (Phase 9c)

The no-splat arity check relaxes from "LHS == RHS strict" to
"LHS == RHS *or* exactly 1 RHS with ≥2 LHS".  All other shapes still
error.  The splat path is unchanged (`a, *b = arr` still uses the
Phase 9b splat lowering and treats the single RHS as one of the
absorbable values — single-RHS-with-splat auto-unpack remains a
future phase).

### Tests

- `ruby-to-semantic-ir`: 177 → **184** (+7)
- `coding-adventures-ruby-parser`: 152 → **155** (+3 — grammar
  coverage tests for `a, b = arr` shape).

## [0.38.0] - 2026-05-28

### Added (Phase 9b (FC) — splat target in multi-assignment LHS)

`multi_assignment` now accepts an optional `*` prefix on each LHS
target via the new `mlhs_target` rule.  At most one splat per LHS is
allowed; the splat absorbs zero or more "extra" RHS values into an
`Expr::SeqLit` while non-splat targets bind to fixed-position RHS
values (counted from the start, or from the end if a splat sits to
the left).

| Source                      | SIR shape (after Phase 9b)                                                                                                                                                                                  |
|-----------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `a, *b = 1, 2, 3`           | `LetStarBinding(t0,1); LetStarBinding(t1,2); LetStarBinding(t2,3); LetBinding(a, VarRef(t0)); LetBinding(b, SeqLit([VarRef(t1), VarRef(t2)]))`                                                              |
| `*a, b = 1, 2, 3`           | `LetStarBinding(t0,1); LetStarBinding(t1,2); LetStarBinding(t2,3); LetBinding(a, SeqLit([VarRef(t0), VarRef(t1)])); LetBinding(b, VarRef(t2))`                                                              |
| `a, *b, c = 1, 2, 3, 4`     | 4 temps + `LetBinding(a, t0); LetBinding(b, SeqLit([t1, t2])); LetBinding(c, t3)`                                                                                                                            |
| `a, *b = 1`                 | 1 temp + `LetBinding(a, t0); LetBinding(b, SeqLit([]))` *(empty splat)*                                                                                                                                     |

The splat path always routes through the swap-safe temp pass (Phase
9a pattern) — every RHS value lands in a fresh
`LetStarBinding(__multi_assign_t<N>_<i>, rhs[i])` first, so the
splat's `SeqLit` and the surrounding non-splat bindings all read
captured values.  `Feature::Sequences` is required.

Arity check:

- No splat → LHS count must equal RHS count (Phase 6r semantics).
- Splat present → RHS count must be `≥ non_splat_count`.  Otherwise
  the lowerer rejects with a clear error.

### Tests

- `ruby-to-semantic-ir`: 170 → **177** (+7):
  - `splat_lhs_at_end_absorbs_trailing_rhs_into_seqlit`
  - `splat_lhs_at_start_absorbs_leading_rhs_into_seqlit`
  - `splat_lhs_in_middle_absorbs_middle_rhs_into_seqlit`
  - `splat_lhs_with_minimum_rhs_count_gives_empty_seqlit`
  - `splat_lhs_requests_sequences_feature`
  - `splat_lhs_module_passes_sir_validator` (E2E for all three splat positions)
  - `splat_lhs_too_few_rhs_is_a_lower_error`

## [0.37.0] - 2026-05-28

### Changed (Phase 9a (FC) — swap-safe parallel multi-assignment)

Phase 6r lowered `a, b = rhs0, rhs1` as a flat sequence of one SIR
statement per pair: `Stmt(a := rhs0); Stmt(b := rhs1)`.  That's
observably correct only when no LHS name appears in any RHS — the
common case.  For the swap `a, b = b, a`, the sequential form reads
the *post-assignment* value of `a` when evaluating the second pair,
producing `a = old_b; b = old_b` instead of the true swap.

Phase 9a introduces a "needs-temps" heuristic.  After lowering every
RHS to an `Expr`, the lowerer scans each one (structural recursion
over the SIR `Expr` tree) for any `VarRef` whose name appears in the
LHS list.  If found:

1. Each RHS value is bound to a fresh `LetStarBinding` temp named
   `__multi_assign_t<N>_<i>` (counter `multi_assign_counter` ensures
   uniqueness across multiple multi-assignments in the same scope).
2. Each LHS is then assigned from its temp via the usual
   first-sighting `LetBinding` / re-binding `Assign` decision.

`LetStarBinding` (sequential semantics) is used for the temps so each
temp's name is visible to the subsequent LHS-binding pass — `LetBinding`
would put them in the same parallel-let validator group and hide them.

If no LHS appears in any RHS, the lowerer keeps Phase 6r's sequential
shape (no temps, no `LetStarBinding`) so the simple case stays cheap.

| Source                  | SIR shape (after Phase 9a)                                                                                                         |
|-------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `a, b = 1, 2`           | `LetBinding(a, 1); LetBinding(b, 2)` *(no temps — fast path)*                                                                      |
| `a = 1; b = 2; a, b = b, a` | `LetBinding(a, 1); LetBinding(b, 2); LetStarBinding(__multi_assign_t0_0, VarRef(b)); LetStarBinding(__multi_assign_t0_1, VarRef(a)); Assign(a, VarRef(__multi_assign_t0_0)); Assign(b, VarRef(__multi_assign_t0_1))` |

### Tests

- `ruby-to-semantic-ir`: 165 → **170** (+5):
  - `multi_assignment_swap_introduces_temps_to_preserve_parallel_semantics`
  - `multi_assignment_simple_case_keeps_fast_path_with_no_temps`
  - `multi_assignment_partial_dependency_still_uses_temps_for_all_positions`
  - `multi_assignment_swap_module_passes_sir_validator` (validator E2E)
  - `multi_assignment_two_swaps_use_distinct_temp_counters`

## [0.36.0] - 2026-05-28

### Changed (Phase 8b (FC) — short-circuit `||=` / `&&=` lowering)

Phase 6p originally lowered `x ||= y` and `x &&= y` eagerly to
`Assign(x, BuiltinCall("or"/"and", [VarRef(x), y]))`.  That form
ALWAYS evaluates `y` and ALWAYS re-binds `x`, which silently breaks
Ruby's documented short-circuit semantics whenever `y` has side
effects.  Phase 8b replaces it with a gated `Expr::If` so the RHS and
the re-bind are skipped when the short-circuit branch fires.

| Source     | SIR shape                                                          |
|------------|--------------------------------------------------------------------|
| `x ||= y`  | `ExprStmt(If(VarRef(x), Block{[], VarRef(x)}, Block{[Assign(x,y)], VarRef(x)}))` |
| `x &&= y`  | `ExprStmt(If(VarRef(x), Block{[Assign(x,y)], VarRef(x)}, Block{[], VarRef(x)}))` |

`Feature::MutableBindings` is still required (the gated branch
re-binds `x`), and `x` is recorded as a declared local so any
subsequent `x = …` doesn't trip the rebinding-into-undeclared-name
error.  All other compound-assign forms (`+=`, `-=`, `*=`, `/=`, `%=`,
`**=`, `<<=`, `>>=`, `&=`, `|=`, `^=`) keep their eager
`Assign + BuiltinCall` lowering — they have no short-circuit
semantics, so the previous shape is correct for them.

### Tests

- `ruby-to-semantic-ir`: 162 → **165** (+3 net):
  - Replaced `logical_compound_assigns_lower_to_or_and_builtins`
    (asserted the old eager shape) with four new tests:
    - `or_assign_lowers_to_short_circuit_if_with_assign_in_else_branch`
    - `and_assign_lowers_to_short_circuit_if_with_assign_in_then_branch`
    - `short_circuit_op_assign_marks_mutable_bindings_feature`
    - `short_circuit_op_assign_module_passes_sir_validator` (validator E2E for both ops)

## [0.35.0] - 2026-05-26

### Added (Phase 8a-2 (FC) — `>>=` right-shift compound-assign lowering)

`lower_assignment` gains one more case arm — `">>="` maps to `BuiltinCall(">>", ...)` with the same `Stmt::Assign` + `Feature::MutableBindings` shape as the rest of the compound-assign family.

| Source     | SIR shape                                                       |
|------------|-----------------------------------------------------------------|
| `x >>= y`  | `Assign(x, BuiltinCall(">>", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |

Combined with Phase 8a, Ruby's complete compound-assignment family on local variables is now fully lowered to first-class SIR.

### Tests

- `ruby-to-semantic-ir`: 160 → **162** (+2):
  - `right_shift_assign_desugars_to_assign_with_rshift_builtin`
  - `right_shift_assign_module_passes_sir_validator` (E2E smoke)

## [0.34.0] - 2026-05-26

### Added (Phase 8a (FC) — additional compound-assignment lowering)

`lower_assignment` learns six more compound forms — `%=`, `**=`, `<<=`, `&=`, `|=`, `^=` — and desugars each identically to `x = x op rhs`:

| Source     | SIR shape                                                       |
|------------|-----------------------------------------------------------------|
| `x %= y`   | `Assign(x, BuiltinCall("%",  [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x **= y`  | `Assign(x, BuiltinCall("**", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x <<= y`  | `Assign(x, BuiltinCall("<<", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x &= y`   | `Assign(x, BuiltinCall("&",  [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x \|= y`  | `Assign(x, BuiltinCall("\|", [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |
| `x ^= y`   | `Assign(x, BuiltinCall("^",  [VarRef(x, Local), <y>]))` + `Feature::MutableBindings` |

Same convention as the pre-existing `+= -= *= /=` family: BuiltinCall name matches the underlying surface operator literally, so downstream emitters that target Ruby can pass the name through unchanged.

### Tests

- `ruby-to-semantic-ir`: 155 → **160** (+5):
  - `modulo_assign_desugars_to_assign_with_modulo_builtin`
  - `power_assign_desugars_to_assign_with_power_builtin`
  - `left_shift_assign_desugars_to_assign_with_lshift_builtin`
  - `bitwise_op_assigns_lower_to_assign_with_bitwise_builtins`
  - `compound_assigns_module_passes_sir_validator` (E2E smoke)

## [0.33.0] - 2026-05-26

### Added (Phase 7f — Ruby 3.1 hash value-omitted shorthand lowering)

`lower_hash_entry` learns a third dispatch arm: when a `hash_entry` node has a `NAME` token, a `COLON` token, and ZERO `expression` children, the entry's value is emitted as `VarRef(name, scope)` — a same-named local variable lookup.  Key remains `SymLit(name)` (matching the existing keyword-style shorthand).

The `scope` follows the same Param-vs-Local dispatch as bare-name factor lowering: if the binding exists in `current_params`, mark it `Param`; otherwise mark it `Local`.  This means `{x:}` inside `def f(x); …; end` correctly emits `VarRef("x", Param)`.

### Lowering dispatch summary

| Source            | Shape                                                                        |
|-------------------|------------------------------------------------------------------------------|
| `{x: 1}`          | `MapEntry { key: SymLit("x"), value: IntLit(1) }` (unchanged)               |
| `{x => 1}`        | `MapEntry { key: <lowered x>, value: IntLit(1) }` (unchanged)               |
| **`{x:}`**        | **`MapEntry { key: SymLit("x"), value: VarRef("x", Local/Param) }` (new)** |

The change is purely additive — no existing SIR shape changes.  Both `Feature::Symbols` (for the key) and (transitively) any feature for the value expression are still recorded as before.

### Tests

- `ruby-to-semantic-ir`: 150 → **155** (+5):
  - `hash_value_shorthand_emits_var_ref_value` — `{name:}` value is `VarRef("name", Local)`.
  - `hash_value_shorthand_inside_method_uses_param_scope` — `def f(x); {x:}; end` value is `VarRef("x", Param)`.
  - `hash_value_shorthand_mixed_with_explicit_form` — `{name:, age: 30}` first entry is VarRef, second is IntLit.
  - `hash_explicit_form_unchanged_after_phase_7f` — `{x: 1, y: 2}` regression (still IntLit values).
  - `hash_value_shorthand_module_passes_sir_validator` — end-to-end validator smoke.

## [0.32.0] - 2026-05-25

### Added (Phase 7e — Ruby 3.0 rightward assignment lowering)

A new helper `lower_rightward_assignment` mirrors the `lower_assignment` LetBinding-on-first-sight / Assign-on-rebind dispatch.  Rightward assignment is purely syntactic — `expr => var` and `var = expr` produce identical SIR.

### Lowering

| Source              | SIR shape                                                |
|---------------------|----------------------------------------------------------|
| `1 + 2 => sum`      | `LetBinding(sum, BuiltinCall("+", [IntLit 1, IntLit 2]))` |
| `42 => x`           | `LetBinding(x, IntLit 42)`                               |
| `[1, 2] => arr`     | `LetBinding(arr, SeqLit([IntLit 1, IntLit 2]))`          |
| (re-bind) `5 => x`  | `Assign(x, IntLit 5)` + `Feature::MutableBindings`       |

`lower_statement_inner` dispatches `rightward_assignment` to the new helper alongside `assignment`.

### Tests

- `ruby-to-semantic-ir`: 146 → **150** (+4):
  - `rightward_assignment_lowers_to_let_binding_on_first_sight` — `1 + 2 => sum`.
  - `rightward_assignment_with_literal_lowers_to_int_let_binding` — `42 => x`.
  - `rightward_assignment_rebind_emits_assign_with_mutable_bindings_feature` — `Assign` + manifest gating.
  - `rightward_assignment_module_passes_sir_validator` — end-to-end smoke.

## [0.31.0] - 2026-05-25

### Added (Phase 7d — Ruby 3.0 case/in pattern matching lowering)

`lower_case_statement` now collects both `when_clause` and `in_clause` subnodes in source order.  Two new helpers:

- `lower_when_clause_condition` — refactored out of the original Phase 6u lowerer for symmetry with `in_clause` dispatch (no behaviour change).
- `lower_in_clause_pattern` — dispatches on pattern kind, returning `(cond, prefix_stmts)`.  Binding-pattern body-prefix stmts are prepended to the clause body so the bound local is visible from the first statement.

### Pattern lowering

| Pattern        | cond                                            | body-prefix stmts        |
|----------------|-------------------------------------------------|--------------------------|
| `in 1`         | `BuiltinCall("==", [scrut, IntLit(1)])`         | `[]`                     |
| `in "s"`       | `BuiltinCall("==", [scrut, StrLit("s")])`       | `[]`                     |
| `in :foo`      | `BuiltinCall("==", [scrut, SymLit("foo")])`     | `[]`                     |
| `in nil`       | `BuiltinCall("==", [scrut, NilLit])`            | `[]`                     |
| `in y`         | `BoolLit(true)`                                 | `[LetBinding(y, scrut)]` |
| `in [1, 2]`    | `BuiltinCall("__pattern_match__", [scrut, StrLit(raw)])` | `[]`            |
| `in {name: y}` | `BuiltinCall("__pattern_match__", [scrut, StrLit(raw)])` | `[]`            |

The `__pattern_match__` marker carries the verbatim pattern text (joined Token values via depth-first walk) so downstream emitters can re-derive the structural matching at codegen time.  Same marker-builtin pattern as Phase 6v rescue/ensure, Phase 6y `__interp__`, Phase 7a `backtick`.

The synthetic `StrLit` triggers `Feature::Strings`.

A new helper `lower_pattern_literal` mirrors the factor-atom Token dispatch but narrowed to the patterns the `literal_pattern` rule admits (NUMBER, STRING, KEYWORD/`nil`/`true`/`false`, symbol_literal).  It reuses Phase 6z's `lower_numeric_literal` so every numeric shape (float/hex/bin/oct/dec) parses identically inside a pattern.

### v0 deferred limitations

- Array / hash patterns are kept as the `__pattern_match__` marker — no structural decomposition, no sub-bindings emitted.  A follow-up phase will walk the inner patterns and emit element comparisons + sub-`LetBinding`s.
- Hash pattern shorthand `{name:}` doesn't bind `name` at SIR level.
- Pin operators (`^x`), find patterns (`[…, *, …]`), and class patterns (`SomeClass(x)`) are not yet parsed.
- Inside an `in` body, a bare-name statement (`in y; y; end`) hits a pre-existing grammar quirk where `method_call_no_paren` greedily consumes the closing `end` keyword as an argument.  Workaround in tests: use `puts(y)` rather than bare `y`.

### Tests

- `ruby-to-semantic-ir`: 141 → **146** (+5):
  - `case_in_literal_pattern_lowers_to_equality_check` — `in 1`.
  - `case_in_binding_pattern_emits_letbinding_prefix` — `in y`.
  - `case_in_array_pattern_lowers_to_pattern_match_marker` — `in [1, 2]`.
  - `case_in_hash_pattern_lowers_to_pattern_match_marker` — `in {name: y}`.
  - `case_in_with_else_clause_emits_else_branch` — `else` fallback.

## [0.30.0] - 2026-05-25

### Added (Phase 7c — Ruby 3.0 endless method definitions)

A new helper `lower_endless_def_statement` lowers `def foo = expr` / `def foo(x, y) = expr` into a top-level `Function`.  The shape:

```
Function {
    name,
    params,
    body: Block { stmts: [], value: <lowered expression> },
    return_type: None,
    captures: [],
    effects: PURE,
    metadata: Metadata::new(),
}
```

The `lower_statement_inner` `def_statement` match now also matches `endless_def_statement` (both hoist to a top-level Function and emit a `NilLit` ExprStmt placeholder in the main body).  Both pre-passes — `collect_def_statements` (program-level) and `collect_def_statements_from_body` (class/module-level) — dispatch on rule name so either form gets hoisted.

### Lowering details

- Parameter extraction reuses the same `params` → `param` Node walk as `lower_def_statement`, with the same lossy-splat v0 limitation: `*args` / `**kw` params drop the splat prefix.
- A fresh `declared_locals` / `current_params` scope is opened for the body expression so a parameter reference inside the body resolves to `Scope::Param` (validator-correct).
- The body is the single `expression` Node child (PEG guarantees exactly one, after the EQUALS token).  `Block.stmts` is empty; `Block.value` is the lowered expression.
- Function `effects` default to `PURE`; if the lowered expression contains effectful calls (e.g. `puts`), the SIR's `effects_of` inference will pick them up at validation time.

### v0 deferred limitations

- Lossy splat (inherited from Phase 6s): `def foo(*args) = ...` loses the `*` prefix.
- Endless defs inside classes / modules are hoisted to top level (no class scoping in SIR v0 — same caveat as the block-bodied def).
- No method visibility markers (`private`, `protected`) — same as block-bodied defs.

### Tests

- `ruby-to-semantic-ir`: 137 → **141** (+4):
  - `endless_def_no_params_hoists_to_top_level_function` — happy path.
  - `endless_def_with_params_carries_param_scope` — asserts `Scope::Param` for body VarRefs.
  - `endless_def_does_not_emit_main_body_stmt` — confirms hoisting + NilLit placeholder.
  - `endless_def_module_passes_sir_validator` — end-to-end validator smoke.

## [0.29.0] - 2026-05-25

### Added (Phase 7b — heredoc literal lowering)

`lower_factor_atom`'s `String` case now dispatches in lexeme-prefix priority order:
- starts with `` ` `` → backtick command literal (Phase 7a)
- starts with `<<` → heredoc (Phase 7b — this phase)
- otherwise → string interpolation lowering (Phase 6y)

### Lowering

| Source                            | SIR shape                            |
|-----------------------------------|--------------------------------------|
| `` `<<EOF\nhello\nEOF` ``         | `StrLit("hello\n")`                  |
| `` `<<-EOF\nhello\n  EOF` ``      | `StrLit("hello\n")`                  |
| `` `<<~EOF\n  hello\n  EOF` ``    | `StrLit("hello\n")` (lexer pre-strips indent) |

The `<<~TAG` common-indent stripping is performed by the lexer's `finalize_heredoc` before the token reaches the lowerer; this routine just removes the opener prefix (`<<`, `<<-`, `<<~`) and the trailing closing-tag suffix.

The synthetic `StrLit` triggers `Feature::Strings`.

### v0 deferred limitations

- Interpolation inside the body (`#{name}`) is NOT split — the body lowers as a single `StrLit` with `#{...}` markers preserved verbatim.  Follow-up will reuse the Phase 6y interpolation splitter.
- Non-interpolating heredocs (`<<'TAG'`) and the `<<"TAG"` form are not yet distinguished from the unquoted form — the lexer doesn't carry the quote state through.
- Escape sequences inside the body are kept literal.

### Tests

- `ruby-to-semantic-ir`: 132 → **137** (+5):
  - `plain_heredoc_lowers_to_strlit_body_only` — happy path.
  - `dash_indent_heredoc_lowers_to_strlit_body_only` — `<<-EOF`.
  - `tilde_indent_heredoc_strips_common_leading_whitespace` — `<<~EOF`.
  - `heredoc_triggers_strings_feature` — manifest gating.
  - `heredoc_module_passes_sir_validator` — end-to-end smoke.

## [0.28.0] - 2026-05-25

### Added (Phase 7a — backtick command literal lowering)

`lower_factor_atom`'s `String` case now dispatches by lexeme prefix:
- starts with `` ` `` → new `lower_backtick_command_literal` helper (Phase 7a).
- otherwise → existing `lower_string_literal_with_interp` (Phase 6y).

### Lowering

| Source       | SIR shape                                                       |
|--------------|-----------------------------------------------------------------|
| `` `ls` ``   | `BuiltinCall("backtick", [StrLit("ls")])` + MayBlock\|MayPrint\|MayThrow |
| `` `` ``     | `BuiltinCall("backtick", [StrLit("")])` + same effects          |

The triple-effect set reflects that command execution may **block** on the child process, **print** stdout/stderr, and **throw** if the command can't be invoked.  Marker-builtin pattern reused from Phase 6v (`__rescue_marker__`), Phase 6w (`lambda`/`proc`), and Phase 6y (`__interp__`).

The synthetic `StrLit` body triggers `Feature::Strings`.

### v0 deferred limitations

- Interpolation inside the body (`` `echo #{name}` ``) is NOT split — the body lowers as a single `StrLit` with any `#{...}` markers preserved verbatim.  Follow-up will reuse the Phase 6y splitter.
- Escape sequences are already resolved by the lexer's `backtick_body` state.

### Tests

- `ruby-to-semantic-ir`: 127 → **132** (+5):
  - `backtick_command_literal_lowers_to_backtick_builtin_call` — happy path.
  - `backtick_command_literal_carries_effect_set` — asserts MayBlock + MayPrint + MayThrow.
  - `empty_backtick_command_literal_lowers_with_empty_body` — `` `` ``.
  - `backtick_command_literal_triggers_strings_feature` — manifest gating.
  - `backtick_command_literal_module_passes_sir_validator` — end-to-end smoke.

## [0.27.0] - 2026-05-25

### Added (Phase 6z — float / hex / bin / oct numeric literal lowering)

`lower_factor_atom` now hands every `TokenType::Number` token to a new helper, `lower_numeric_literal`, which dispatches on shape (radix-prefix → IntLit with chosen radix; float-shape → FloatLit; otherwise → decimal IntLit).

### Lowering

| Source       | SIR shape                                |
|--------------|------------------------------------------|
| `42`         | `IntLit { value: 42 }`                   |
| `1_000_000`  | `IntLit { value: 1000000 }`              |
| `0x1F`       | `IntLit { value: 31 }` (radix 16)        |
| `0xDEAD_BEEF`| `IntLit { value: 3735928559 }`           |
| `0b1010`     | `IntLit { value: 10 }` (radix 2)         |
| `0o17`       | `IntLit { value: 15 }` (radix 8)         |
| `0d42`       | `IntLit { value: 42 }` (radix 10 explicit) |
| `1.5`        | `FloatLit { value: 1.5 }`                |
| `1e10`       | `FloatLit { value: 1e10 }`               |
| `1.5e-3`     | `FloatLit { value: 0.0015 }`             |

Float literals additionally trigger `Feature::Floats` in the module manifest.  The manifest aggregator now propagates `Feature::Floats` alongside the prior set (`Strings`, `Closures`, `Symbols`, etc.).

Underscore separators are stripped before parsing.  Radix detection checks `bytes[1] ∈ {x,X,b,B,o,O,d,D}` after a leading `0`.  Float detection is a single scan for `.` or `e`/`E` — mutually exclusive with radix prefixes in the Ruby grammar.

### v0 deferred limitations

- Rational (`r`) / Complex (`i`) numeric suffixes (lexed by Phase 4f) are rejected by the integer-parse path — a future phase will route those into `BuiltinCall("rational", ...)` / `BuiltinCall("complex", ...)` markers.
- Negative literals continue to flow through the Phase 6k unary-minus path; this routine sees only the magnitude.
- Legacy octal (`017` without `0o` prefix) is not supported by either the lexer or this lowerer.

### Tests

- `ruby-to-semantic-ir`: 121 → **127** (+6):
  - `float_literal_lowers_to_floatlit_and_triggers_floats_feature` — `1.5`.
  - `float_literal_with_signed_exponent_lowers_correctly` — `1.5e-3`.
  - `hex_literal_lowers_to_intlit_with_correct_value` — `0xDEAD_BEEF` + asserts Floats feature NOT triggered.
  - `binary_literal_lowers_to_intlit` — `0b1010`.
  - `octal_literal_lowers_to_intlit` — `0o17`.
  - `float_literal_module_passes_sir_validator` — end-to-end validator smoke.

## [0.26.0] - 2026-05-25

### Added (Phase 6y — string interpolation lowering)

`lower_factor_atom` now hands every `TokenType::String` token to a new helper, `lower_string_literal_with_interp`, which scans the raw content for `#{...}` interpolation markers and emits the appropriate SIR shape.

### Lowering

| Source              | SIR shape                                                                                       |
|---------------------|-------------------------------------------------------------------------------------------------|
| `"plain"`           | `StrLit("plain")` — zero-cost fast path                                                         |
| `"#{x}"`            | `VarRef("x")` — single non-literal segment, no wrapper                                          |
| `"hi #{name}"`      | `BuiltinCall("string_concat", [StrLit("hi "), VarRef("name")])`                                 |
| `"#{a}#{b}"`        | `BuiltinCall("string_concat", [VarRef("a"), VarRef("b")])`                                      |
| `"sum=#{1+2}"`      | `BuiltinCall("string_concat", [StrLit("sum="), BuiltinCall("__interp__", [StrLit("1+2")])])`    |

Bare-identifier interp bodies route to `VarRef` with the same `Scope::Param` / `Scope::Local` dispatch as the regular factor-atom Name case.  Complex bodies emit a marker `BuiltinCall("__interp__", [StrLit(raw)])` — same marker pattern as Phase 6v's `__rescue_marker__` / `__ensure_marker__`.

Brace depth is tracked while scanning the interp body so nested `{...}` (inline hash, block) is balanced correctly, matching the lexer's `interp_brace_depth` state.

### v0 deferred limitations

- Complex interp bodies (arithmetic, method calls, nested strings, sigil vars) are kept as a `__interp__` marker rather than being recursively parsed.  A future phase will invoke the Ruby parser/lowerer on the body so the SIR carries proper semantic info.
- Escape sequences inside the string literal (`\n`, `\t`, `\\`, `\"`) pass through unchanged — the lexer hasn't unescaped them yet.
- Sigil-prefixed vars (`@x`, `$x`, `@@x`) inside an interp body intentionally fall through to the `__interp__` marker; Phase 6x's sigil routing only fires at lex time, not at interp-split time.

### Tests

- `ruby-to-semantic-ir`: 116 → **121** (+5):
  - `plain_string_with_no_interp_remains_a_strlit` — regression for the zero-cost path.
  - `interpolated_string_with_bare_name_lowers_to_string_concat` — happy path.
  - `interpolated_string_that_is_only_interp_unwraps_to_a_single_segment` — `"#{name}"`.
  - `interpolated_string_with_expression_uses_interp_marker` — `"sum=#{1+2}"`.
  - `interpolated_string_module_passes_sir_validator` — end-to-end validator smoke.

## [0.25.0] - 2026-05-25

### Added (Phase 6x — sigil variable refs `@x`, `@@x`, `$x`)

`lower_factor_atom` now documents Ruby's sigil-prefixed variable convention and explicitly routes all three sigil forms to `Scope::Local` with the sigil preserved in the bound name.

### Lowering

| Source | SIR shape |
|---|---|
| `@a` | `VarRef { name: "@a", scope: Local }` |
| `@@count` | `VarRef { name: "@@count", scope: Local }` |
| `$config` | `VarRef { name: "$config", scope: Local }` |

`Scope::Local` is the conservative v0 choice — the SIR validator enforces that `Scope::Global` references have a matching `Global` declaration in the module, and the Ruby lowerer doesn't yet auto-emit those declarations.  Downstream emitters can still recognise the sigil form via the leading `@` / `@@` / `$` in `name`.

### v0 deferred limitations

- No SIR `IVar` / `CVar` / `GVar` scope.  A future phase will add auto-`Global`-declaration for `$x` so the validator-true mapping `$x` → `Scope::Global` becomes usable.
- The sigil convention is purely a name-encoding hint for downstream emitters; the SIR scope machinery treats all three identically.

### Tests

- `ruby-to-semantic-ir`: 112 → **116** (+4):
  - `global_var_ref_preserves_sigil_in_name` — `$config` keeps the `$`.
  - `instance_var_ref_lowers_with_local_scope_and_sigil_preserved` — `@a`.
  - `class_var_ref_lowers_with_local_scope_and_double_at_preserved` — `@@count`.
  - `sigil_vars_module_passes_sir_validator` — end-to-end validator smoke across all three sigils.

## [0.24.0] - 2026-05-25

### Added (Phase 6w — arrow-lambda lowering)

New `lower_lambda_literal` helper handles `->(params){body}`:

- Extracts params from the leading parens-list (Phase 6s — splat names preserved bare).
- Hoists the block body to a top-level `Function` named `__block_<n>` (reusing Phase 6g's counter) via new `hoist_lambda_body` helper.
- Emits `Expr::BuiltinCall { name: "lambda", args: [MakeClosure { fn_name, captures: [] }], effects: PURE }`.
- Auto-sets `Feature::Closures` (and `Feature::DynamicTyping` if any params present).

`ruby_builtin_effects` extended to recognise `lambda` and `proc` so `lambda { ... }` and `proc { ... }` (keyword forms going through `method_with_block`) also emit `BuiltinCall("lambda"|"proc", ...)`.  Downstream emitters see a single closure-construction shape.

### v0 deferred limitations

- Captures from the enclosing scope are NOT computed for arrow lambda bodies (same limitation as Phase 6g blocks).
- `lambda { … }` / `proc { … }` work at statement position only — they can't be used as an expression RHS because `method_with_block` is not part of `factor`.

### Tests

- `ruby-to-semantic-ir`: 108 → **112** (+4):
  - `arrow_lambda_no_params_lowers_to_lambda_builtin` — bare `-> { 1 }`.
  - `arrow_lambda_with_params_hoists_body_with_params` — params propagate to hoisted Function.
  - `lambda_keyword_form_lowers_via_method_with_block` — `lambda { |x| x + 1 }` keyword form.
  - `arrow_lambda_module_passes_sir_validator` — end-to-end validator smoke.

## [0.23.0] - 2026-05-25

### Added (Phase 6v — `begin … rescue … ensure … end` lowering)

New `lower_begin_statement` helper fans the source `begin_statement` into multiple SIR statements (via the existing `lower_statement_inner_multi` Vec<Stmt> dispatch from Phase 6r):

- Body stmts inline.
- ExprStmt(BuiltinCall("__rescue_marker__", [StrLit(exc_types_csv), StrLit(var_name)])) per `rescue_clause`, followed by that clause's body stmts inline.
- ExprStmt(BuiltinCall("__ensure_marker__", [])) before the ensure body stmts inline (if `ensure_clause` present).

Markers carry the `Effect::MayThrow` tag.  Strings feature is auto-set (markers emit StrLits).

### v0 deferred limitations

- SIR has no try/catch primitive — markers only signal the form's presence to downstream emitters that target languages with real exceptions.
- Rescue body is *unreachable* in SIR's effect model; the marker is informational.
- `else` clause inside `begin` (Ruby's "no-exception" branch) is not supported by the grammar.
- Exception class hierarchy is not modelled — `rescue StandardError` (with `=>`) and bare `rescue` lower identically apart from the marker payload.

### Tests

- `ruby-to-semantic-ir`: 104 → **108** (+4):
  - `begin_without_rescue_lowers_body_inline` — no marker for plain `begin … end`.
  - `begin_with_rescue_emits_rescue_marker` — `__rescue_marker__("StandardError", "e")`.
  - `begin_with_ensure_emits_ensure_marker` — `__ensure_marker__()`.
  - `begin_with_rescue_and_ensure_emits_both_markers_in_order` — full sequence shape.

## [0.22.0] - 2026-05-25

### Added (Phase 6u — `case … when … else … end` lowering)

New `lower_case_statement` helper folds the source `case_statement` into a chained `Expr::If`:

```
case x
when v1, v2 then a
when v3     then b
else c
end
```

→

```
if ((x == v1) || (x == v2)) then a
else if (x == v3) then b
else c
```

Each when_clause becomes one nested `If`; multi-value `when 1, 2, 3` lists OR-fold left-to-right using `BuiltinCall("or", ...)`.  The else clause (or implicit `NilLit` block) caps the chain.  Comparisons use `BuiltinCall("==", [scrutinee, value])` — see v0 deferred caveats below.

The result is wrapped in `Stmt::ExprStmt`.

### v0 deferred limitations

- Comparisons use `==` not `===`.  Ruby's case-equality (class-aware: `Integer === 1`, range membership, regex match) is NOT modelled.  Phase 7d adds full `case/in` pattern matching.
- Range/Regex/Class values in `when` lists work syntactically but don't behave as Ruby would.

### Tests

- `ruby-to-semantic-ir`: 100 → **104** (+4):
  - `case_single_when_lowers_to_if_with_eq` — basic shape check.
  - `case_with_multi_value_when_lowers_to_or_chain` — `==` × 3 + `or` × 2.
  - `case_with_else_terminates_chain` — else body lands in chain tail.
  - `case_without_else_uses_nil_tail` — no else → NilLit tail.

## [0.21.0] - 2026-05-25

### Added (Phase 6t — `yield` lowering)

`yield ...` → `Stmt::ExprStmt(Expr::BuiltinCall("yield", lowered_args, EffectSet::PURE))`.

Lowering walks the optional `yield_args` wrapper (when present), extracts its `call_arg` children, and routes each through Phase 6s's `lower_call_arg` helper.  Bare `yield` (no `yield_args` wrapper) lowers to an empty-arg BuiltinCall.

Effects are PURE — `yield` invokes the caller-supplied block, whose effects are tracked at the *block construction* site (via `Expr::MakeClosure`'s captured effect set), not at the yield call site.  Modelling `yield` as PURE keeps the effect lattice from double-counting block effects.

### Tests

- `ruby-to-semantic-ir`: 96 → **100** (+4):
  - `bare_yield_lowers_to_yield_builtin_no_args`
  - `yield_with_one_arg_lowers_to_builtin_with_one_arg`
  - `yield_with_paren_args_lowers_to_two_arg_builtin`
  - `yield_with_splat_arg_lowers_with_splat_envelope` — exercises Phase 6t × Phase 6s composition.

## [0.20.0] - 2026-05-25

### Added (Phase 6s — splat / double-splat lowering)

#### Call args (preserved through SIR)

| Source | Lowered |
|---|---|
| `f(*arr)` | `DirectCall(f, [BuiltinCall("splat", [VarRef(arr)])])` |
| `f(**hsh)` | `DirectCall(f, [BuiltinCall("double_splat", [VarRef(hsh)])])` |
| `f(1, *arr, **hsh)` | three positional args: `IntLit(1)`, `splat(arr)`, `double_splat(hsh)` |

New helper `lower_call_arg` dispatches on the leading `*` / `**` token (if any) and wraps the inner expression in a `BuiltinCall` envelope.  No prefix → return the bare expression.  Downstream emitters can pattern-match the builtin name to convert back to splat syntax in target source.

Renamed `head_call_expression_children` → `head_call_args` (returns `call_arg` Nodes instead of `expression` Nodes).  `lower_method_call` dispatches on the rule name: `method_call` uses the new `call_arg` shape; `method_call_no_paren` keeps the legacy bare-`expression` shape (paren-less splat is a deferred limitation — see ruby-parser changelog).

`fold_one_dot_call` likewise routes through `lower_call_arg`.

#### Params (lossy at SIR level)

`*args` / `**kwargs` lower to regular `Param { name: "args" / "kwargs" }` — SIR's `Param` has no variadic flag, so the splat-ness is dropped.  The parameter-name extractor in `lower_def_statement` skips the splat-prefix tokens (`*` and `**`) when locating the identifier.

**Downstream impact**: target source emitted for variadic functions will treat the parameter as positional.  Calls passing splat args (via the `BuiltinCall("splat", ...)` envelope) still preserve the variadic shape — the asymmetry only matters for definitions of variadic functions.  Tracked as a deferred limitation for a future SIR phase that adds variadic-aware `Param`.

### Tests

- `ruby-to-semantic-ir`: 91 → **96** (+5):
  - `splat_call_arg_lowers_to_splat_builtin`
  - `double_splat_call_arg_lowers_to_double_splat_builtin`
  - `mixed_call_args_with_splats_lower_in_order`
  - `splat_param_lowers_to_bare_name_param` (asserts lossy v0 lowering)
  - `splat_call_arg_module_passes_sir_validator` — end-to-end validator smoke.

## [0.19.0] - 2026-05-25

### Added (Phase 6r — multiple assignment lowering)

`a, b = 1, 2` fans out into one SIR statement per (LHS, RHS) pair — each lowered identically to the single-LHS `assignment` rule (`LetBinding` on first sighting, `Assign` thereafter).

#### Architecture change

New dispatch wrapper `lower_statement_inner_multi(node) → Vec<Stmt>`:
- `multi_assignment` → delegates to `lower_multi_assignment` (returns `Vec<Stmt>`).
- All other statement forms → wraps the single `lower_statement_inner` result in `vec![stmt]`.

The four statement-list walkers (`lower_program`, `lower_clause_statements`, `lower_def_statement` body, `lower_method_with_block` body) updated from `.push(...)` to `.extend(...)`.  The modifier-statement LHS path keeps the single-stmt `lower_statement_inner` call because `multi_assignment` is not an eligible LHS form in `modifier_statement`.

#### v0 restrictions (rejected with `RubyLowerError`)

- LHS count must equal RHS count.
- Single-RHS auto-unpack (`a, b = arr`) — not supported.
- Splat targets (`a, *b = 1, 2, 3`) — Phase 6s.

#### Lowering rule

For each pair `(lhs[i], rhs[i])`:
- First sighting of `lhs[i]` in this scope → `Stmt::LetBinding { name, value: rhs[i], … }`.
- Subsequent sighting → `Stmt::Assign { name, scope: Local, value: rhs[i], … }` (and sets `Feature::MutableBindings`).

RHS expressions are lowered first (in source order), then the LHS bindings happen in source order.  The parallel-binding swap case (`a, b = b, a`) is NOT correctly v0-lowered (would silently mis-evaluate); this is documented as a deferred limitation.

### Tests

- `ruby-to-semantic-ir`: 86 → **91** (+5):
  - `multi_assignment_lowers_to_independent_let_bindings` — basic `a, b = 1, 2` → two `LetBinding`.
  - `multi_assignment_redeclaration_uses_assign` — `a = 1; b = 2; a, b = 3, 4` second multi-assign uses `Assign`.
  - `multi_assignment_three_names_emits_three_stmts` — three LHS / three RHS → three SIR stmts.
  - `multi_assignment_arity_mismatch_errors` — `a, b = 1, 2, 3` returns `RubyLowerError`.
  - `multi_assignment_module_passes_sir_validator` — end-to-end validator smoke.

## [0.18.0] - 2026-05-24

### Added (Phase 6q — modifier conditionals/loops lowering)

New `lower_modifier_statement` handler dispatches on the parser's `modifier_statement` node.

Lowering table:

| Source              | Lowered SIR                                              |
|---------------------|----------------------------------------------------------|
| `lhs if cond`       | `Stmt::ExprStmt(Expr::If(cond, [lhs], Nil))`             |
| `lhs unless cond`   | `Stmt::ExprStmt(Expr::If(not(cond), [lhs], Nil))`        |
| `lhs while cond`    | `Stmt::While(cond, [lhs])`                               |
| `lhs until cond`    | `Stmt::While(not(cond), [lhs])`                          |

The lowering produces the same canonical `Expr::If` / `Stmt::While` shapes as the leading-keyword `if_statement` / `while_statement` lowerings — every downstream emitter (semantic-ir-to-python, -rust, -typescript, -go) handles modifier forms transparently with no new code paths.

`while`/`until` modifier variants set `Feature::Loops` automatically, matching the leading-keyword loop behaviour.

The LHS statement is wrapped in a single-statement `Block` whose `value` is `NilLit` — the modifier form is never tail-promoted to an expression (it sits in statement position only).

### Tests

- `ruby-to-semantic-ir`: 81 → **86** (+5):
  - `if_modifier_lowers_to_expr_if_statement` — produces `ExprStmt(If)` with bare cond.
  - `unless_modifier_wraps_condition_in_not` — cond becomes `BuiltinCall(not, …)`.
  - `while_modifier_lowers_to_stmt_while` — `Stmt::While` with bare cond.
  - `until_modifier_negates_condition_in_while` — `Stmt::While` with `not(cond)`.
  - `modifier_module_passes_sir_validator` — end-to-end validator smoke test across all four forms.

## [0.17.0] - 2026-05-24

### Added (Phase 6p — compound assignment lowering)

SIR encoding (for each `x op= rhs`):
```
Stmt::Assign {
  name: "x",
  scope: Local,
  value: Expr::BuiltinCall {
    name: "<op>",   // "+", "-", "*", "/", "or", "and"
    args: [VarRef("x"), <rhs>],
  },
}
```

| Source | Lowered as |
|---|---|
| `x += y` | `x = x + y` |
| `x -= y` | `x = x - y` |
| `x *= y` | `x = x * y` |
| `x /= y` | `x = x / y` |
| `x \|\|= y` | `x = x or y` |
| `x &&= y` | `x = x and y` |

Lowering identically to `x = x op y` means downstream emitters (semantic-ir-to-python, -rust, -typescript, -go) need no new code path — the existing assignment + binary-op lowering handles both forms.

### Lowerer changes
- `lower_assignment` now reads the operator token (skipping the leading NAME) to dispatch on `EQUALS` vs the six compound forms.
- Compound forms always emit `Stmt::Assign` (never `LetBinding`) even on first sighting — the read of `x` before the write means the binding semantically pre-exists.  Sets `Feature::MutableBindings` automatically.

### Tests (+4 new, total 81)
- `plus_equals_lowers_to_assign_with_plus_builtin`
- `all_arithmetic_compound_assigns_lower_correctly` — `+=`, `-=`, `*=`, `/=`.
- `logical_compound_assigns_lower_to_or_and_builtins` — `||=` → `"or"`, `&&=` → `"and"`.
- `compound_assign_module_passes_sir_validator` — end-to-end validator smoke test.

## [0.16.0] - 2026-05-24

### Added (Phase 6o — ternary lowering)

SIR encoding:
```
cond ? a : b  →  Expr::If {
                   cond,
                   then_branch: Block { stmts: [], value: a },
                   else_branch: Block { stmts: [], value: b },
                 }
```

Lowering identically to `if cond then a else b end` means downstream emitters (semantic-ir-to-python, semantic-ir-to-rust, etc.) need no new code path — the existing if-lowering paths handle both syntactic forms transparently.

**Right-associativity** falls out of the grammar: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`, so the inner ternary nests inside the outer's else-branch as another `Expr::If`.

### Lowerer changes
- `lower_expression` gained a `"ternary"` dispatch arm.
- New helper `lower_ternary(node)` filters operand sub-nodes: one operand (pass-through) or three (cond/then/else → `Expr::If`).

### Tests (+3 new, total 77)
- `ternary_lowers_to_if_expr_with_branch_blocks` — `x = 1 ? 2 : 3` → `LetBinding { value: If { cond=1, then=2, else=3 } }`.
- `ternary_right_associative_nests_in_else_branch` — `x = 1 ? 2 : 3 ? 4 : 5` produces a nested If in the outer else.
- `ternary_module_passes_sir_validator` — end-to-end validator smoke test.

## [0.15.0] - 2026-05-24

### Added (Phase 6n — range expressions lowering)

SIR encoding:
- `a..b`  →  `BuiltinCall("range", [a, b, BoolLit(false)])` ; inclusive end
- `a...b` →  `BuiltinCall("range", [a, b, BoolLit(true)])`  ; exclusive end

A single builtin name (`range`) handles both forms; the third argument carries the inclusive/exclusive flag so downstream emitters can pattern-match once and read the flag.  Effects default to `PURE` — constructing a range observes nothing.

### Lowerer changes
- `lower_expression` gained a `"range"` dispatch arm.
- New helper `lower_range(node)` filters operand `logical_or` sub-nodes from the `..`/`...` operator token, then either passes through (1 operand, no op) or emits the three-arg `BuiltinCall`.

### Tests (+4 new, total 74)
- `inclusive_range_lowers_to_range_builtin_with_false_flag` — `1..5` → flag = false.
- `exclusive_range_lowers_to_range_builtin_with_true_flag` — `1...5` → flag = true.
- `range_with_variable_operands_uses_var_refs` — `(a..b)` over function params (parens dodge the lessons.md ambiguity).
- `range_module_passes_sir_validator` — end-to-end smoke test through the validator.

### Out of scope (deferred to follow-up phase)
- Endless ranges `(1..)`, `arr[2..]` (lexer 4e already flags these; parser support TBD).
- Beginless ranges `(..5)`.

## [0.14.0] - 2026-05-24

### Added (Phase 6m — logical operators lowering)

SIR encoding:
- `a || b`, `a or b`  →  `BuiltinCall("or",  [a, b])`
- `a && b`, `a and b` →  `BuiltinCall("and", [a, b])`
- `!x`, `not x`       →  `BuiltinCall("not", [x])`
- `!!x`               →  `BuiltinCall("not", [BuiltinCall("not", [x])])`

Both symbol form (`||`/`&&`/`!`) and keyword form (`or`/`and`/`not`) collapse to the same builtin name — v0 simplification.  All effects default to `PURE`.

### Lowerer changes
- `lower_expression` gained dispatch arms for `logical_or`, `logical_and`, `logical_not`, and `comparison` (renamed from the old `expression` arm).  The `expression` arm itself is now a pass-through to the inner `logical_or` node.
- New helpers `lower_logical_chain(node, op_lexemes, builtin_name)` and `lower_logical_not(node)`.
- `lower_logical_chain` matches operators by lexeme (covers both `||`/`&&` Name-classified tokens and `or`/`and` Keyword tokens uniformly).

### Tests (+6 new, total 70)
- `logical_or_symbol_lowers_to_or_builtin`
- `logical_and_symbol_lowers_to_and_builtin`
- `logical_keyword_form_lowers_same_as_symbol`
- `logical_not_symbol_lowers_to_not_builtin`
- `logical_chain_and_then_or_nests_correctly` — `a && b || c` parses & lowers as `(a && b) || c`.
- `logical_module_passes_sir_validator`

All six use the parens workaround (`(a || b)` instead of bare `a || b`) inside def bodies to dodge the `method_call_no_paren` ambiguity (logged in lessons.md, parser CHANGELOG).

## [0.13.0] - 2026-05-23

### Added (Phase 6l — method receiver chains lowering)

Each `.method[(...)]` step in a receiver chain lowers to:

```
BuiltinCall {
  name: "__method__",
  args: [receiver, StrLit(method_name), ...actual_args],
  effects: PURE,
}
```

The receiver stays as a first-class expression so arbitrary nesting works (`a.b.c.d`).  The method name lives as a `StrLit` so backends can dispatch by string.  This avoids growing the shared `semantic-ir::Expr` enum.

**Why BuiltinCall and not DirectCall?**  The validator checks `DirectCall.fn_name` against the module's function table; our synthetic `__method__` envelope is intentionally not a declared function — it's a wire-format tag for backends.  BuiltinCall has no such resolution check.

**Effect set**: defaults to `PURE`.  Receiver-dispatched calls are type-erased at this layer; a later receiver-type analysis pass can widen as needed.

**Feature side-effect**: any dot_call fires `Feature::Strings` (because of the synthesised StrLit).  This is auto-added to the manifest in `lower_program`'s feature-collection pass.

### Lowerer changes
- New helpers: `apply_dot_chain(atom, factor_node)`, `fold_one_dot_call(receiver, dot_node)`, `head_call_expression_children`.
- `lower_factor` split into `lower_factor` (atom extraction + dot-chain application) and `lower_factor_atom` (the pre-6l atom logic).
- `lower_method_call` collects head-call args via `head_call_expression_children` so args inside `dot_call` subtrees don't leak into the head call.
- `lower_expression` gains a dispatch arm for `method_call` (which can now appear in expression position because it's the first atom alternative in `factor`).
- `Feature::Strings` added to the manifest-population loop in `lower_program`.

### Tests (+5 new, total 64)
- `dot_chain_lowers_to_method_builtincall` — `foo.bar` produces `BuiltinCall("__method__", [VarRef(foo), StrLit("bar")])`.
- `dot_chain_two_steps_nests_outer_recv` — `foo.bar.baz` nests as `__method__(__method__(foo, "bar"), "baz")`.
- `dot_call_with_args_includes_them_after_method_name` — `obj.add(1, 2)` → `__method__(obj, "add", 1, 2)`.
- `dot_chain_on_method_call_head` — `puts(1).then_something` keeps the head BuiltinCall("puts") and wraps it in `__method__(_, "then_something")`.
- `dot_chain_module_passes_sir_validator` — full module with a chain inside a function body validates clean.

## [0.12.0] - 2026-05-22

### Added (Phase 6k — unary minus lowering)
- `lower_expression` dispatches `unary_minus` to a new arm emitting `Expr::BuiltinCall { name: "neg", args: [inner], effects: PURE }`.

### Tests (+5 new, total 59)
- `unary_minus_on_number_lowers_to_neg_builtin`, `unary_minus_on_name_carries_scope`, `double_unary_minus_nests_correctly`, `unary_minus_with_binary_plus_resolves_precedence_correctly`, `unary_minus_module_passes_sir_validator`.

## [0.11.0] - 2026-05-22

### Added (Phase 6j — `return` / `break` / `next` lowering)
- `lower_statement_inner` dispatches `return_statement` / `break_statement` / `next_statement` to a common arm that emits `Expr::BuiltinCall` with the keyword name, the optional trailing expression as the sole argument (or `NilLit` when absent), and `Effect::Divergent` declared.

### Tests (+5 new, total 54)
- `return_with_value_lowers_to_divergent_builtin_call`, `bare_return_lowers_with_nil_arg`, `break_and_next_lower_to_their_respective_builtins`, `return_inside_def_body`, `return_module_passes_sir_validator`.

## [0.10.0] - 2026-05-22

### Added (Phase 6i — comparison operator lowering)
- `lower_expression` now dispatches the renamed `sum` rule via the existing `lower_binary_chain(..., ["PLUS", "MINUS"])`.
- New `lower_comparison_chain` helper — left-associative reduce of comparison operators into `BuiltinCall("==", [lhs, rhs])` (and similarly for `!=`, `<`, `>`, `<=`, `>=`).

### Tests (+5 new, total 49)
- `equality_op_lowers_to_builtin_call`, `less_than_op_lowers_to_builtin_call`, `all_six_comparison_operators_lower_with_correct_names`, `comparison_has_lower_precedence_than_arithmetic`, `comparison_used_in_if_condition_passes_validator`.

## [0.9.0] - 2026-05-22

### Added (Phase 6h — no-paren method call lowering)
- `lower_statement_inner` dispatches `method_call_no_paren` to the existing `lower_method_call` helper.

### Tests (+5 new, total 44)
- `no_paren_call_with_single_arg_lowers_to_builtin_call`, `no_paren_call_with_multiple_args`, `no_paren_call_with_binary_expr_arg_groups_correctly`, `no_paren_call_module_passes_sir_validator`, `paren_form_still_lowers_unchanged`.

## [0.8.0] - 2026-05-22

### Added (Phase 6g — method-with-block lowering)
- `lower_method_with_block` lowers the `method_with_block` rule node into:
  1. A `BuiltinCall` / `DirectCall` for the method dispatch.
  2. A hoisted top-level `Function` named `__block_<n>` for the block body.
  3. An `Expr::MakeClosure { fn_name, captures: [] }` appended as the call's trailing argument.
- New `Lowerer.block_counter: usize` field, new `hoist_block_to_function` helper, `Feature::Closures` declared, expanded builtin iterator table.

### Tests (+5 new, total 39)
- `brace_block_hoists_to_synthetic_function_and_make_closure`, `do_block_with_pipe_params_lowers_to_function_with_params`, `multiple_blocks_get_distinct_synthetic_names`, `block_module_declares_closures_feature`, `block_lowering_passes_sir_validator`.

## [0.7.0] - 2026-05-22

### Added (Phase 6f — class/module lowering with nested-def hoisting)
- `lower_statement_inner` dispatches `class_statement` / `module_statement` rule nodes.  In v0, SIR has no native `class` / `namespace` node, so the declaration itself lowers to a no-op `Stmt::ExprStmt(NilLit)` — same shape used for already-hoisted `def_statement`s.
- New `collect_def_statements_from_body(node)` helper recursively walks a class/module body and hoists every nested `def_statement` to a top-level `Function` (same machinery as the program-level pre-pass).  Nested class/module declarations are recursed into so deeply-nested `def`s still hoist.
- Each `def` body is lowered with a fresh `declared_locals` + `current_params` scope (snapshot/restore in `lower_def_statement`), so locals from sibling methods or the surrounding class don't leak across method boundaries.

### Documented v0 caveat
The hoisted methods land at top-level, *not* nested under the class name.  In real Ruby, `class Foo; def bar` makes `bar` an instance method of `Foo`; v0 SIR collapses the namespace.  The validator still accepts the result because every function has a unique name across the lowered module, and `main` remains the only export.  Proper namespace handling lands when SIR grows a `class` / `namespace` node in a future phase.

### Tests (+4 new, total 34)
- `class_with_method_hoists_def_to_top_level` — `class Foo; def greet; end; end` exposes `greet` on `m.functions`.
- `empty_class_lowers_cleanly` — `class Foo; end` produces a module with only `main` plus a no-op `Stmt::ExprStmt(NilLit)` in the main body.
- `module_with_def_hoists_def_to_top_level` — `module M; def helper; end; end` exposes `helper`.
- `class_module_lowering_passes_sir_validator` — a combined class+module module passes `semantic_ir::validate`.

## [0.6.0] - 2026-05-20

### Added (Phase 6e — symbol-literal lowering)
- `lower_symbol_literal` — picks the first Name / Keyword / String token under the `symbol_literal` node and emits an `Expr::SymLit` with that lexeme as the symbol name.  Quoted symbols (`:"hello world"`) work transparently because the String token's value already has the surrounding quotes stripped.
- Declares `Feature::Symbols` on every symbol literal (same feature the hash-shorthand entries already used).

### Tests (+4 new, total 30)
- `:foo` → `SymLit("foo")`.
- `:"hello world"` → `SymLit("hello world")` (spaces preserved).
- `:def` (keyword-shaped name) → `SymLit("def")`.
- Symbol-containing module passes `semantic_ir::validate`.

## [0.5.0] - 2026-05-20

### Added (Phase 6d — array and hash literal lowering)
- `lower_array_literal` — `[a, b, c]` → `Expr::SeqLit` with all element expressions lowered recursively.
- `lower_hash_literal` — `{a: 1, b => 2}` → `Expr::MapLit { entries }`.
- `lower_hash_entry` handles both syntactic forms:
  - **Shorthand** (`NAME COLON expression`) — the Name becomes a `SymLit` key (sugar for `:name =>`).
  - **Hash-rocket** (`expression "=>" expression`) — both sides are lowered as ordinary expressions.
- `lower_expression` now dispatches `array_literal` and `hash_literal` rule nodes alongside `expression`/`term`/`factor`.
- Feature tracking extended: `Sequences` declared on SeqLit, `Maps` on MapLit, `Symbols` on the shorthand hash-entry key.

### Tests (+4 new, total 26)
- `[1, 2, 3]` → `SeqLit` with three items.
- `[]` → empty `SeqLit`.
- `{a: 1, b: 2}` → `MapLit` with two entries whose keys are `SymLit("a")` and `SymLit("b")`.
- Combined array+hash module passes `semantic_ir::validate` (feature manifests align exactly).

## [0.4.0] - 2026-05-20

### Added (Phase 6c — `while … end` / `until … end` lowering)
- New `lower_while_or_until` handler emits a `Stmt::While`.  `until cond` lowers to `while !cond` (condition wrapped in `BuiltinCall("not", ...)`).
- `Feature::Loops` is now added to the module manifest whenever a `Stmt::While` is emitted (the SIR validator requires it).
- Loop body uses the existing `lower_clause_statements` helper, so locals introduced inside the loop don't leak to the outer scope.

### Tests (+3 new, total 22)
- `while_lowers_to_stmt_while` — basic while produces `Stmt::While`.
- `until_negates_condition` — `until cond` wraps cond in `BuiltinCall("not", ...)`.
- `while_module_passes_sir_validator` — a while-loop module passes `semantic_ir::validate` (Feature::Loops gating works).

## [0.3.0] - 2026-05-20

### Added (Phase 6b — `if … else … end` / `unless` lowering)
- New `lower_if_or_unless` handler — both rules produce a single `Expr::If` because SIR treats conditionals as expressions (every branch yields a value).
- `unless cond` lowers to `if !cond` by wrapping the condition in `BuiltinCall("not", [cond])`.
- `elsif` chains lower with right-associative nesting: the outermost `If`'s `else_branch` is itself a `Block` whose `value` is another `If` for the first `elsif`, and so on.  The validator sees one well-formed expression tree.
- New `lower_clause_statements` helper saves/restores `declared_locals` around each branch so locals introduced in one `if`/`elsif`/`else` arm don't leak into siblings (which would have caused spurious `Stmt::Assign` emissions and validator errors).
- `Lowerer.features_used: HashSet<Feature>` — tracks which SIR features the lowering actually exercises.  `compile` now emits a manifest that lists *only* the features in use:
  - `DynamicTyping` whenever a function has at least one untyped param.
  - `MutableBindings` whenever a `Stmt::Assign` re-binds an existing local.
  This swaps the previous "always declare DynamicTyping" approach for an exact-match manifest, which is what the validator requires.

### Tests (+5 new, total 19)
- `if_lowers_to_expr_if` — basic if/end produces `Expr::If`.
- `if_else_lowers_with_else_branch` — explicit else branch is captured.
- `unless_negates_condition` — `unless cond` wraps the cond in `BuiltinCall("not", ...)`.
- `if_elsif_else_chain_nests_right` — elsif chain produces nested `Expr::If` in `else_branch.value`.
- `if_module_passes_sir_validator` — an `if … else … end` containing module passes `semantic_ir::validate`.

## [0.2.0] - 2026-05-20

### Added (Phase 6a — `def name(params) … end` method definitions)
- New `collect_def_statements` pre-pass hoists every `def_statement` from the program to a top-level `semantic_ir::Function` *before* the main-body lowerer runs.
- `lower_def_statement` translates the AST node into a `Function`:
  - The first `Name` token after the leading `def` keyword becomes the function name (the `def` keyword itself is skipped so the function isn't named `"def"`).
  - The optional `params` sub-rule's `Name` tokens become `Param`s.
  - The body is lowered using a *fresh* `declared_locals` set so the outer program's bindings don't leak in.  Params are pre-declared as locals (so `x = 2` inside `def f(x)` routes through `Stmt::Assign`) *and* tracked in a new `current_params` set so `VarRef` to them gets `Scope::Param` (the validator's expectation for parameters).
  - The tail expression (if any) is promoted to the body block's `value`; otherwise `value = NilLit`.
- `Module::manifest` now declares `Feature::DynamicTyping` — Ruby is dynamically typed, and the SIR validator requires this whenever a module produces untyped params or globals.
- `def_statement` nodes left behind in the program body lower to a no-op `Stmt::ExprStmt(NilLit)` so the SIR-level statement stream stays in sync with the source line count.

### Tests (+5 new, total 14)
- `def_lowers_to_top_level_function` — `def add(x, y); x + y; end` produces an `add` function with two params and a `+` builtin call body.
- `def_with_no_params_lowers_cleanly` — `def hello; end` produces a paramless function whose body value is `NilLit`.
- `def_does_not_leak_locals_to_outer_scope` — locals in a method body don't leak into the program-level scope (each gets its own first-occurrence `LetBinding`).
- `def_with_param_reassignment_routes_through_assign` — `def f(x); x = 2; end` re-binds via `Stmt::Assign` (the param is pre-declared as local).
- `module_with_def_passes_sir_validator` — a `def`-containing module passes `semantic_ir::validate`.

## [0.1.0] - 2026-05-20

### Added (Phase 5 — initial Ruby → SIR frontend)
- New crate `ruby-to-semantic-ir`.  Consumes `coding-adventures-ruby-parser`'s `GrammarASTNode` and emits a `semantic_ir::Module`.
- `compile_source(source, module_name) -> Result<Module, RubyLowerError>` — tokenize → parse → lower in one call.
- `compile(ast, module_name) -> Result<Module, RubyLowerError>` — lower an already-parsed AST.
- `RubyLowerError` — carries a human-readable message plus 1-based line/column drawn from the AST node's span.
- v0 lowering covers everything ruby-parser v0 parses:
  - Programs → synthesised `main` function whose body is the sequence of lowered statements.  Last bare-expression statement becomes the block's `value`; otherwise `value = NilLit`.
  - Assignments (`x = expr`) → `LetBinding` for first occurrence, `Assign` for subsequent re-bindings (scope is `Local`).
  - Method calls (`name(args...)`) → `BuiltinCall` for the known builtin set (`puts`, `print`, `p`, `gets`, `raise`); other names lower to `DirectCall` (placeholder; in v0 there are no user-defined functions, so backends will flag these as unresolved).
  - Expressions: integer literals, string literals, name references, binary `+ - * /` lowered to `BuiltinCall("+"...)` etc., parenthesised sub-expressions.
- Tests cover empty program, single assignment, multi-statement programs, arithmetic, method calls, re-assignment routing through `Stmt::Assign`, and tail-expression promotion to block `value`.

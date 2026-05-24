# Changelog

All notable changes to the `ruby-to-semantic-ir` crate will be documented in this file.

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

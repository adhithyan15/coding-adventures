# sir-nested-def-shadowing — Nested def Shadowing Parameter or Local Variable

## Status

Active design & implementation specification. Addresses item 7 of #18496 and #18482.

## Motivation

In Python, a function definition (`def_stmt`) is an executable statement that defines a function
or closure and binds it to its declared identifier in the current local scope:

```python
def f(n):
    def n():
        return 42
    return n()
```

When an outer function has a parameter or prior local variable of the same name as the nested
function (here, `n`), the nested `def` rebinds/shadows that name in the local scope. Subsequent
references to `n` (such as a call `n()` or returning `n` as a value) must evaluate to the newly
defined nested function, not the parameter or earlier local value.

## Problem Statement

In `python-to-semantic-ir`, nested functions are lifted to top-level synthesized functions with
explicit captures (`lower_def`). When a nested `def_stmt` was lowered in `lower_statement`:

```rust
"def_stmt" => {
    let mc = self.lower_def(inner, ctx, depth)?;
    self.observed.add(Feature::Closures);
    Ok(Lowered::Expr(mc))
}
```

The resulting `Expr::MakeClosure` was yielded as `Lowered::Expr(mc)`. In `lower_function_suite`,
intermediate statements are converted to statements via `statements_only`, which wraps any
bare expression into an `ExprStmt`:

```rust
Lowered::Expr(expr) => {
    let span = expr.span().clone();
    vec![Stmt::ExprStmt { expr, span }]
}
```

This caused the newly constructed `MakeClosure` to be evaluated and immediately discarded without
ever being stored into the shadowed parameter or local variable.

Subsequently, when `n()` or `n` was referenced:
1. `lower_call_by_name` checks:
   ```rust
   if self.function_names.contains(&name) && !ctx.is_enclosing_value(&name)
   ```
2. Because `n` was an enclosing parameter (copied to `__param_n_0` in `param_copy_prologue`) or
   a prior local variable, `ctx.is_enclosing_value("n")` was `true`.
3. Thus `!ctx.is_enclosing_value("n")` evaluated to `false`, falling through to:
   ```rust
   let target = self.resolve_var_in(ctx, &name, span.clone())?;
   Ok(Expr::IndirectCall { target: Box::new(target), ... })
   ```
4. `resolve_var_in` resolved `n` to the local alias `__param_n_0` (or `n`), which still held the
   original parameter/local value (e.g. an integer).
5. Calling `n()` performed an `IndirectCall` on the parameter value instead of invoking the
   nested function!

## Design & Semantic Invariants

When lowering a nested `def_stmt`:
1. Extract the function name: `let name = self.def_name(inner)?`.
2. Lower the definition: `let mc = self.lower_def(inner, ctx, depth)?`.
3. Check whether the declared name shadows an enclosing value:
   `if ctx.is_enclosing_value(&name)`.
4. If it **does** shadow an enclosing value:
   - Call `self.store_target(Target::Name(name), mc, ctx, span)?`.
   - If `name` was already in `ctx.locals` (such as a copied parameter `__param_n_0` or prior
     local `n`), `store_target` emits a `Stmt::Assign` targeting `ctx.ir_local(&name)` and
     registers `Feature::MutableBindings`.
   - If `name` was an enclosing capture, `store_target` adds `name` to `ctx.locals` and emits a
     `Stmt::LetStarBinding`, shadowing the capture locally.
   - Return `Ok(Lowered::Stmt(Box::new(store)))`.
5. If it **does not** shadow an enclosing value:
   - Retain `Ok(Lowered::Expr(mc))` to preserve zero-overhead direct calls (`DirectCall`) and
     inlined closure calls (`lower_closure_call`) for unshadowed nested functions without
     allocating unnecessary local variable slots.

## Test Plan

- Unit tests in `code/packages/rust/python-to-semantic-ir/tests/statement_forms.rs`:
  - `nested_def_shadowing_parameter_is_invoked`: `def f(n): def n(): return 42; return n()` returns 42.
  - `nested_def_shadowing_prior_local_is_invoked`: `def f(): n = 1; def n(): return 42; return n()` returns 42.
  - `nested_def_shadowing_parameter_returns_closure`: `def f(n): def n(): return 42; return n` returns closure.
  - `nested_def_shadowing_parameter_with_captures`: nested function captures an outer parameter while shadowing another.
- Cross-backend conformance tests in `code/packages/rust/sir-conformance/tests/python_statement_forms.rs`:
  - Verify identical evaluation across backends (JS, TS, C, Go, Rust, Python).

# sir-subscript-evaluation-order — Subscript Assignment Evaluation Order in Python-to-SIR

## Status

Active design & implementation specification. Addresses the subscript store evaluation order
defect identified during the triage and security review of #18496 / #18482.

## Motivation

In Python, assignment statements strictly evaluate the right-hand side (RHS) expression *before*
evaluating the target expression. According to the Python Language Reference §6.3:

> "An assignment statement evaluates the given expression list (remember that this can be a single
> expression or a comma-separated list, the latter yielding a tuple) and assigns the single
> resulting object to each of the target lists, from left to right."
>
> "If the target is a subscription: The primary expression in the reference is evaluated. It should
> yield either a mutable sequence object (such as a list) or a mapping object (such as a dictionary).
> Next, the subscript expression is evaluated."

Therefore, for a subscript assignment `target[index] = value`:
1. `value` (the right-hand side expression) is evaluated first.
2. `target` (the sequence or map container expression) is evaluated second.
3. `index` (the key/subscript expression) is evaluated third.
4. The assignment is performed (`target[index] = value`).

### Prior Defect

In `python-to-semantic-ir/src/lower.rs`, single-target assignments were lowered as:

```rust
if let [only] = target_lists {
    let target = self.classify_target(self.single_expr(only)?, ctx)?;
    let stmt = self.store_target(target, value, ctx, span)?;
    return Ok(Lowered::Stmt(Box::new(stmt)));
}
```

When `target` was a `Target::Subscript`, `store_target` lowered the container base and index, and
constructed `Stmt::SeqSet { seq, index, value, span }` (or `Stmt::MapSet { map, key, value, span }`)
with `value` inlined directly.

When SIR backends (such as JavaScript emitting `(seq)[index] = value;`, or C emitting
`_sir_array_set(seq, index, value)`) evaluate this statement:
- `seq` is evaluated first.
- `index` is evaluated second.
- `value` is evaluated third.

As a result, in programs where `value` and `index` perform observable side effects (such as
`xs[g()] = f()` where `f()` and `g()` append to a log or modify state), `g()` ran *before* `f()`.
In CPython, `f()` executes before `g()`.

Notice that chained assignments `a = b = value` in `python-to-semantic-ir` already evaluated `value`
into a temporary variable `let* chain_tmp = value` before storing into each target left to right.
Single subscript assignments lacked this temporary.

## Contract Specification

### 1. Right-hand Side Evaluation in `python-to-semantic-ir`

When lowering a single assignment target (`[only] = target_lists` in `Lowerer::lower_assignment`):

1. Classify the target via `self.classify_target(self.single_expr(only)?, ctx)?`.
2. **If `target` is `Target::Subscript`**:
   - Allocate a fresh temporary local name (e.g. `__subscript_val = self.fresh_temp_name("subval")`).
   - Register the temporary in `ctx.locals`.
   - Emit a `Stmt::LetStarBinding` that evaluates `value` into the temporary:
     ```rust
     Stmt::LetStarBinding {
         name: tmp.clone(),
         sir_type: None,
         value,
         span: span.clone(),
     }
     ```
   - Emit the store statement via `self.store_target(target, VarRef(tmp), ctx, span)`.
   - Return both statements as `Lowered::Stmts(vec![val_binding, store_stmt])`.
3. **If `target` is `Target::Name` or `Target::Instance`**:
   - Proceed as before: `self.store_target(target, value, ctx, span)` (scalar / instance stores
     do not evaluate a container or index expression, so RHS evaluation order is already preserved).

### 2. Multi-Dimensional & Chained Subscripts

For multi-dimensional or nested subscriptions such as `m[a()][b()] = f()`:
- `value` (`f()`) is evaluated into the temporary first.
- In `store_target`, the base `m[a()]` and last index `b()` are evaluated after `f()`.
- Therefore, evaluation order `f() -> m -> a() -> b()` is strictly maintained.

### 3. Conformance Verification

- Add tests in `python-to-semantic-ir` verifying that `xs[g()] = f()` emits a `LetStarBinding` for `f()`
  prior to the `SeqSet` (or `MapSet`).
- Add conformance tests in `sir-conformance` executing `xs[g()] = f()` against CPython and across all
  target backends, asserting 100% agreement in execution order.

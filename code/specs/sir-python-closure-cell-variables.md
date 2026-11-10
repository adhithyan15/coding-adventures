# sir-python-closure-cell-variables — Cell Variable Representation for Python Closures

## Status

Active design & implementation specification. Addresses item 5 of #18496.

## Motivation

In Python, closures capture variable bindings (cells), not snapshot values at closure creation time.
When an enclosing function defines a variable `x` (parameter or local), and an inner function (nested
`def` or `lambda`) captures `x`, any modification to `x` in the enclosing scope (before, during, or
after closure creation) must be observed when the closure runs:

```python
def f():
    x = 10
    g = lambda: x
    x = 20
    return g()
```
CPython returns `20`.

```python
def f(x):
    def g():
        return x
    x = 20
    return g()
```
CPython returns `20`.

```python
def f():
    def g():
        return x
    x = 42
    return g()
```
CPython returns `42`.

```python
def f():
    funcs = []
    for i in [1, 2, 3]:
        funcs.append(lambda: i)
    return [funcs[0](), funcs[1](), funcs[2]()]
```
CPython returns `[3, 3, 3]`.

## Problem Statement

In `python-to-semantic-ir`, nested callables capture enclosing variables by snapshotting their value
at `MakeClosure` evaluation time:
1. `capture_values` calls `resolve_var_in(enclosing, name)` which resolves to `Expr::VarRef { name, scope: Scope::Local | Scope::Param }`.
2. This captures whatever value `name` holds at that instant. Subsequent assignments to `name` in the
   enclosing function update the local binding or temporary copy, but the closure retains its captured
   snapshot value.
3. If a closure is defined *before* the variable is assigned, `plan_captures` fails with `"unsupported: g needs x to build a closure it uses, but x is not in scope where g is defined (deferred)"`.
4. In loops, every closure captures the value of the loop variable at iteration time, failing to reflect subsequent iterations.

## Design: Cell Boxing via 1-Element Sequences

SIR already supports sequences (`SeqLit`, `SeqIndex`, `SeqSet`) with heap-allocated in-place reference
semantics across all six backends (C, Ruby, Python, JS, TS, Go, Rust).

### 1. Cell Variable Identification

A variable `v` in a function `f` (or top-level module `__py_main`) is identified as a cell variable if:
1. `v` is declared/bound in `f` (either as a parameter or as an assignment target in `f`'s suite), AND
2. `v` is captured by at least one closure (nested `def` or `lambda`) defined within `f` (or transitively planned), AND
3. `v` is mutated/rebound in `f`:
   - `v` is a parameter of `f` and is assigned in `f`'s suite; OR
   - `v` is assigned inside a loop (`for_stmt` or `while_stmt`); OR
   - `v` is assigned more than once; OR
   - `v` is assigned after (or inside) any closure definition (`def_stmt` or expression containing `lambda_expr`) in `f`'s suite; OR
   - `v` is assigned inside a conditional branch (`if_stmt`).

Immutable single-assignment variables (assigned once at top-level before any closure definition, or
read-only parameters) are NOT boxed into cells, avoiding any overhead and maintaining exact AST parity
with existing tests.

### 2. Cell Prologue

In the function prologue:
For each cell variable `v` owned by `f`:
- Register `v` in `ctx.cells`.
- Allocate a fresh cell alias `__cell_<v>_<N>`.
- In prologue statements:
  - If `v` is a parameter: `Stmt::LetStarBinding { name: alias, value: SeqLit([VarRef(v, Scope::Param)]) }`.
  - If `v` is a local: `Stmt::LetStarBinding { name: alias, value: SeqLit([Nil]) }`.
- Register `Feature::Sequences`.
- In `param_copy_prologue`: skip any parameter that is already a cell variable.

### 3. Cell Reads and Writes in Enclosing Function

- Reads of `v` (`resolve_var_in`):
  Evaluates to `Expr::SeqIndex { base: VarRef(alias, Scope::Local), index: 0 }`.
- Writes to `v` (`store_target`):
  Emits `Stmt::SeqSet { seq: VarRef(alias, Scope::Local), index: 0, value }`.
- Augmented assignment (`x += ...`):
  Reads `SeqIndex(alias, 0)` as current value, writes `SeqSet(alias, 0, new_value)`.

### 4. Closure Capture Passing

- When `f` builds a closure that captures `v` (`capture_values` / `named_closure`):
  Passes `Expr::VarRef { name: alias, scope: Scope::Local }` (the cell reference itself, NOT `SeqIndex`).

### 5. Cell Variables in Inner Functions

- When lowering a closure that captures `v`:
  - `v` is marked in `inner.cells`.
  - Inside the closure:
    - Reads of `v`: `Expr::SeqIndex { base: VarRef(sir_var_name(v), Scope::Capture), index: 0 }`.
    - If the closure builds a deeper closure capturing `v`: passes `VarRef(sir_var_name(v), Scope::Capture)` as the cell reference.

## Test Plan

- Unit tests in `code/packages/rust/python-to-semantic-ir/tests/`:
  - `py_param_captured_then_assigned`: Parameter assigned after closure definition reflects updated value.
  - `py_local_captured_then_assigned`: Local variable assigned after closure definition reflects updated value.
  - `py_closure_defined_before_variable_assigned`: Closure defined before variable's initial assignment sees the value when called.
  - `py_loop_closure_captures_variable`: Closures created in a loop share the final value of the loop variable.
- Cross-backend conformance tests in `code/packages/rust/sir-conformance/tests/python_statement_forms.rs`:
  - Verify identical evaluation across backends (JS, TS, C, Go, Rust, Python).

# LANG55 — Higher-Order List Operations

**Status**: In progress  
**Branch**: `feat/lang55-higher-order-list-ops`  
**Depends on**: LANG52 (list stdlib), LANG34 (first-class closures / `call_closure`)

---

## Motivation

LANG52 added the foundational list builtins (`cons`, `car`, `cdr`, `list`, `length`,
`append`, `reverse`, `list-ref`, `assoc`).  One class of operations was explicitly
deferred:

> Higher-order list ops (`map`, `filter`, `fold-left`, `fold-right`) require calling
> back into the VM interpreter — defer to LANG52+ or implement in Twig once string
> literals exist.

These four operations are the backbone of any functional programming style.  The
self-hosted Twig compiler uses them throughout its passes (source→token list
transformation, token→AST, environment threading).  Without them every compiler pass
must be hand-unrolled with explicit recursion.

LANG55 closes this gap by implementing `map`, `filter`, `fold-left`, and `fold-right` as
**VM-level higher-order builtins** — special-cased in `exec_call_builtin` with direct
access to the recursive `dispatch` call stack.

---

## What changes

| File | Change |
|------|--------|
| `twig-vm/src/dispatch.rs` | Extract `invoke_closure_value` helper; add `map`, `filter`, `fold-left`, `fold-right` special cases in `exec_call_builtin` |
| `twig-ir-compiler/src/compiler.rs` | Add `map`, `filter`, `fold-left`, `fold-right` to `BUILTINS` |
| `twig-vm/Cargo.toml` | Bump version `0.12.0 → 0.13.0` |
| `twig-ir-compiler/Cargo.toml` | Bump version `0.8.0 → 0.9.0` |
| `twig-vm/CHANGELOG.md` | Prepend `## [0.13.0]` entry |
| `twig-ir-compiler/CHANGELOG.md` | Prepend `## [0.9.0]` entry |
| `code/specs/LANG55-higher-order-list-ops.md` | This document |

`lispy-runtime` is **not changed** — the HOF functions are intercepted in
`exec_call_builtin` before `resolve_builtin` is called.

---

## Design

### Why not implement in `lispy-runtime`?

`lispy-runtime`'s builtin functions have signature `fn(&[LispyValue]) -> LispyRuntimeResult`.
They hold no reference to the VM module, the depth counter, the budget, or the globals
table.  Any function that needs to *call back into the interpreter* cannot be a simple
builtin — it would need to invoke `dispatch` recursively, which requires `module`,
`depth`, `budget`, `globals`, `ic_table`, `profile`, and `debug`.

`exec_call_builtin` in `dispatch.rs` already has all of these in scope.  The pattern is
established by `apply_closure`, `global_set`, `global_get`, and the `host/` family.
LANG55 follows exactly the same pattern.

### Why not implement as Twig source (stdlib.tw)?

A stdlib file would require a module-loading pass that runs before the user program.
That mechanism (LANG54-multi-file driver) does not yet exist.  The VM-level approach
is self-contained and requires zero new infrastructure.

### `invoke_closure_value` helper

The logic for calling a closure value (`LispyValue` holding a heap closure) is already
inside `exec_call_closure` and `exec_apply_closure` but not extractable without
synthesising an `IIRInstr`.  LANG55 extracts a standalone helper:

```rust
/// Call a LispyValue that is a heap closure (user fn or builtin).
///
/// This is the core of `exec_call_closure` / `exec_apply_closure`, extracted
/// so that higher-order builtins (`map`, `filter`, `fold-*`) can invoke
/// closure values without constructing a synthetic IIRInstr.
fn invoke_closure_value(
    module: &IIRModule,
    handle: LispyValue,
    args: Vec<LispyValue>,
    depth: usize,
    budget: &mut ExecutionBudget,
    globals: &mut Globals,
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
    debug: &mut Option<&mut dyn crate::debug::DebugHooks>,
) -> Result<LispyValue, RunError>
```

`exec_call_closure` and `exec_apply_closure` are then refactored to call this helper,
eliminating duplicated code.

### Semantics

#### `map` — `(map fn lst)`

Apply `fn` to each element of `lst` (a proper list), collecting results into a new
proper list in the same order.

```
(map (lambda (x) (* x x)) (list 1 2 3)) → (1 4 9)
(map car (list (cons 1 2) (cons 3 4)))  → (1 3)
(map fn nil)                             → nil
```

Argument order: `srcs = [name, fn_val, list_val]`

#### `filter` — `(filter pred lst)`

Keep elements of `lst` for which `pred` returns a truthy value (anything except `#f`
and `nil`).  Order is preserved.

```
(filter odd? (list 1 2 3 4 5))  → (1 3 5)
(filter null? (list nil 1 nil)) → (nil nil)
(filter pred nil)               → nil
```

Argument order: `srcs = [name, pred_val, list_val]`

#### `fold-left` — `(fold-left fn init lst)`

Left fold (accumulate from left to right).  Each step: `acc = (fn acc elem)`.

```
(fold-left + 0 (list 1 2 3 4))  → 10
(fold-left cons nil (list 1 2)) → ((nil . 1) . 2)
(fold-left fn init nil)         → init
```

Argument order: `srcs = [name, fn_val, init_val, list_val]`

#### `fold-right` — `(fold-right fn init lst)`

Right fold (accumulate from right to left).  Each step: `acc = (fn elem acc)`.

```
(fold-right cons nil (list 1 2 3)) → (1 2 3)   ; identity for proper lists
(fold-right + 0 (list 1 2 3))     → 6
(fold-right fn init nil)          → init
```

Argument order: `srcs = [name, fn_val, init_val, list_val]`

Implementation note: collect the list into a `Vec<LispyValue>` first, then iterate
in reverse calling `fn` from the right.

### Closure contract

The `fn` / `pred` argument **must** be a heap-tagged closure (either a user-function
closure or a builtin closure).  Passing a non-closure raises `RunError::NotCallable`.
This is the same contract as `call_closure`.

Builtin closures (e.g. wrapping `+` or `car`) are supported via the builtin-closure
path in `invoke_closure_value`.

### Error handling

| Situation | Error |
|-----------|-------|
| Wrong arity (e.g. `(map fn)` missing list) | `RunError::MalformedInstruction` |
| `fn` arg is not a closure | `RunError::NotCallable` |
| `lst` arg is not a list (not nil or cons) | `RunError::HostArgType` |
| `fn` call itself raises an error | propagated as-is |
| Budget exhaustion during iteration | `RunError::BudgetExceeded` |
| Depth exceeded during callback | `RunError::DepthExceeded` |

---

## Tests (≥ 10)

Integration tests live in `twig-vm/src/dispatch.rs` (module-level `#[cfg(test)]`)
and use the `run_program!` / `run_source!` macros.

1. `map_squares` — `(map (lambda (x) (* x x)) (list 1 2 3))` → `(1 4 9)` ✓
2. `map_empty_list` — `(map fn nil)` → `nil` ✓
3. `map_builtin_fn` — `(map car (list (cons 1 2) (cons 3 4)))` → `(1 3)` ✓
4. `filter_keeps_odds` — `(filter (lambda (x) (= (modulo x 2) 1)) (list 1 2 3 4 5))` → `(1 3 5)` ✓
5. `filter_empty_input` — `(filter pred nil)` → `nil` ✓
6. `filter_all_drop` — `(filter null? (list 1 2 3))` → `nil` ✓
7. `fold_left_sum` — `(fold-left + 0 (list 1 2 3 4 5))` → `15` ✓
8. `fold_left_empty` — `(fold-left + 0 nil)` → `0` ✓
9. `fold_right_cons` — `(fold-right cons nil (list 1 2 3))` → `(1 2 3)` ✓ (proper list round-trip)
10. `fold_right_sum` — `(fold-right + 0 (list 1 2 3))` → `6` ✓
11. `map_then_fold` — compose: `(fold-left + 0 (map (lambda (x) (* x x)) (list 1 2 3)))` → `14` ✓
12. `filter_then_map` — `(map (lambda (x) (* x 2)) (filter odd? (list 1 2 3 4 5)))` → `(2 6 10)` ✓

Where `odd?` is defined as `(define (odd? x) (= (modulo x 2) 1))`.

---

## Version bumps

- `twig-vm`: `0.12.0 → 0.13.0` (minor — new HOF builtins)
- `twig-ir-compiler`: `0.8.0 → 0.9.0` (minor — BUILTINS expansion)

---

## Intentional deferrals

- **`for-each`** — side-effect-only map; deferred until output story is cleaner
- **`reduce`** — alias for `fold-left` without initial value; deferred (empty-list error semantics)
- **`flat-map` / `concat-map`** — deferred; requires `append` on results
- **`find`** — deferred; first match returning the value or `#f`
- **`any` / `every`** — deferred; short-circuit predicates over lists
- **Higher-order integration with type-checker** — LANG55 does not add HOF type signatures to
  `twig-type-checker`.  Type inference for `(map f lst)` returns `Any`; a future LANG56 pass
  could infer the element type from the closure's return kind.

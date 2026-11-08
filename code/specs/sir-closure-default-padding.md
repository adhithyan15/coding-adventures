# sir-closure-default-padding — Closure Call Default-Padding Sentinel Across Go, Rust, and C

## Status

Active design & implementation specification. Addresses issue #18490.

## Motivation

In Semantic IR (SIR), functions and closures may specify default parameter expressions.
Callee functions generate a parameter prologue that tests whether each optional parameter received
the backend's unique *missing* sentinel (`_sir_is_missing` in Go and C, `__sir::is_missing` in Rust),
and if so, evaluates and assigns the default expression:

```text
if (is_missing(param)) {
    param = <default_expression>;
}
```

For direct function calls where optional arguments are omitted, each emitter automatically pads
omitted slots with the missing sentinel (e.g. `_sir_missing` in Go, `_sir_missing()` in C, and
`__sir::Value::Missing` in Rust).

However, indirect closure invocations (`Expr::MakeClosure` and closure thunks/adapters) omitted
arguments inconsistently across compiled backends:

1. **Go (`semantic-ir-to-go`)**:
   The closure adapter indexed `__args[i]` for every capture and parameter:
   ```go
   _sir_make_closure(func(__args []Value) Value {
       return fname(__args[0], __args[1], ...)
   }, []Value{...})
   ```
   When a closure with default parameters was called with fewer arguments than parameters,
   `len(__args)` was strictly less than `captures.len() + params_arity`, causing an immediate
   runtime panic: `index out of range [1] with length 1`.

2. **Rust (`semantic-ir-to-rust`)**:
   The closure adapter drained arguments using `__it.next().unwrap_or(__sir::Value::Nil)`:
   ```rust
   let mut __it = __args.into_iter();
   fname(..., __it.next().unwrap_or(__sir::Value::Nil))
   ```
   Because `Value::Nil` is not `Value::Missing`, the callee's prologue:
   ```rust
   let b = if __sir::is_missing(&b) { 10 } else { b };
   ```
   evaluated to `false`, leaving `b` bound to `Value::Nil` rather than evaluating the default value.

3. **C (`semantic-ir-to-c`)**:
   The closure thunk padded missing argument positions with `_sir_nil()`:
   ```c
   return fname(..., (argc > i ? args[i] : _sir_nil()));
   ```
   Because `_sir_nil()` has tag `SIR_NIL` rather than `SIR_MISSING`, the callee's guard
   `if (_sir_is_missing(b))` failed to trigger, leaving `b` bound to `nil`.

In dynamic frontends (such as Ruby `->(a, b = 10) { a + b }.call(1)`), Ruby, Python, JS, and TS
evaluated this correctly to `11`, while Go panicked, and Rust and C failed or returned wrong results.

## Specification

### 1. Go (`semantic-ir-to-go`)

- In the inlined Go runtime (`runtime.rs`), define a bounds-checking argument extractor:
  ```go
  // _sir_closure_arg extracts argument `i` from `args`, returning the
  // `_sir_missing` sentinel if `i >= len(args)`. Used by closure adapter
  // functions so callers passing fewer arguments than the callee's parameter count
  // trigger default-parameter prologues rather than an index-out-of-bounds panic (#18490).
  func _sir_closure_arg(args []Value, i int) Value {
      if i < len(args) {
          return args[i]
      }
      return _sir_missing
  }
  ```
- In `emit.rs` under `Expr::MakeClosure`, emit `_sir_closure_arg(__args, {i})` instead of
  `__args[{i}]`.

### 2. Rust (`semantic-ir-to-rust`)

- In `emit.rs` under `Expr::MakeClosure`, change argument iterator draining from:
  ```rust
  out.push_str("__it.next().unwrap_or(__sir::Value::Nil)");
  ```
  to:
  ```rust
  out.push_str("__it.next().unwrap_or(__sir::Value::Missing)");
  ```
  ensuring any omitted parameter receives `Value::Missing`, triggering the callee's
  `__sir::is_missing` default-parameter substitution.

### 3. C (`semantic-ir-to-c`)

- In `emit.rs` under `emit_thunk`, change argument padding from:
  ```rust
  write!(out, "(argc > {i} ? args[{i}] : _sir_nil())", i = i);
  ```
  to:
  ```rust
  write!(out, "(argc > {i} ? args[{i}] : _sir_missing())", i = i);
  ```
  ensuring omitted parameters receive `_sir_missing()`, triggering the callee's
  `if (_sir_is_missing(...))` default-parameter substitution.

### 4. Conformance

- Add a conformance test in `sir-conformance` verifying:
  ```ruby
  f = ->(a, b = 10) { a + b }
  puts f.call(1)
  ```
  produces `11` across all supported backends (Ruby, Python, JS, TS, Go, Rust, C).

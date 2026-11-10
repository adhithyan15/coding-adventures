# Ruby Lambdas: `.call(...)` Dispatch and Outer-Local Captures

**Issue Reference:** [#18798](https://github.com/adhithyan15/coding-adventures/issues/18798) (Item 3)  
**Package Scope:** `code/packages/rust/ruby-to-semantic-ir`, `code/packages/rust/sir-conformance`

---

## 1. Problem Statement

In Issue #18798 Item 3, two primary deficiencies with Ruby lambdas were identified:
1. **Invocation via `.call(...)` fails across all backends**:
   In Ruby, procs and lambdas are invoked using the `.call(...)` method (e.g. `g.call(2)` or `g.call`).
   In `ruby-to-semantic-ir`, `.call` was treated as an ordinary dynamic method call dispatched through the `__method__` built-in envelope:
   ```scheme
   (builtin-call "__method__" [receiver "call" ...args])
   ```
   Because `__method__` routes through runtime OOP class-ancestry dispatch (`call_method`), the call fails at runtime on Python, JavaScript, Go, and Rust with an "undefined method 'call'" error, and is refused at compile time on C and Ruby because `"call"` is not in the registered/known method catalog.
2. **Lambda bodies cannot see outer locals or parameters**:
   In `ruby-to-semantic-ir`, `hoist_lambda_body` hoisted arrow lambdas (`->(x) { ... }`) to synthetic top-level functions with an empty capture list (`captures: Vec::new()`), and `lower_lambda_literal` created `Expr::MakeClosure` with `captures: Vec::new()`.
   Consequently, when a lambda body referenced a variable or parameter bound in an enclosing lexical scope (such as an outer local `x` or an enclosing method parameter `n`), the variable reference was not captured and either failed the SIR validator with an "unknown local" error or failed to link at runtime.

---

## 2. Root Cause Analysis

### 2.1 Invocation via `.call`
In `ruby-to-semantic-ir/src/lower.rs`:
```rust
fn fold_one_dot_call(...) -> Result<Expr, RubyLowerError>
```
processes chained and dotted method invocations (`recv.method(args)`). It contains special cases for:
- `.new` on constant receivers (object allocation `__new__`),
- non-`new` methods on constant receivers (class methods `__class_method__`),
- reflection predicates (`is_a?`, `kind_of?`, `instance_of?`).

However, for `recv.call(args)` on an expression value `recv`, it fell through to the default OOP `__method__` envelope:
```rust
full_args.push(receiver);
full_args.push(Expr::StrLit { value: method_name, span: name_span });
full_args.extend(args);
BuiltinCall { name: "__method__", args: full_args, ... }
```
In Semantic IR, callable values (closures and functions) are invoked via the dedicated AST node:
```rust
Expr::IndirectCall {
    target: Box<Expr>,
    args: Vec<Expr>,
    span: Span,
}
```
Every backend in the repository (C, Python, JavaScript, TypeScript, Go, Rust, Ruby) implements `Expr::IndirectCall`:
- **Ruby**: emits `sir_apply(target, *args)`, where `sir_apply(target, *args) = target.call(*args)`.
- **Python**: emits `_sir_apply(target, [*args])`, applying the callable with arguments.
- **JavaScript / TypeScript**: emits `target(...args)`.
- **Go / Rust**: emits native closure application `target(...args)`.
- **C**: emits `sir_call(target, ...)`.

Lowering `recv.call(args)` to `Expr::IndirectCall` cleanly routes execution directly to the callable without going through OOP method tables.

### 2.2 Outer-Local Captures in Lambdas
In `hoist_block_to_function` (which hoists iterator blocks like `[1].each { |x| ... }`), M4 capture extraction was previously implemented:
- It snapshots enclosing parameters and locals (`enclosing_params`, `enclosing_locals`).
- After lowering the body statements, it computes the block's locally-bound names (`block_bound`).
- Any free reference to an enclosing parameter or local is identified (`is_free`).
- Free reads are rewritten to `Scope::Capture` using `recapture_reads_in_block`.
- Matching `Capture` and `CaptureValue` structures are emitted, and the caller receives `(fn_name, capture_values)` to thread into `Expr::MakeClosure`.

In contrast, `hoist_lambda_body` had not received this M4 capture mechanism:
```rust
self.user_functions.push(Function {
    name: fn_name.clone(),
    params,
    return_type: None,
    captures: Vec::new(), // <--- Always empty!
    body: Block { ... },
    ...
});
```
And `lower_lambda_literal`:
```rust
Ok(Expr::BuiltinCall {
    name: "lambda".to_string(),
    args: vec![Expr::MakeClosure {
        fn_name,
        captures: Vec::new(), // <--- Always empty!
        span: span.clone(),
    }],
    ...
})
```
By incorporating the identical capture extraction and rewriting logic into `hoist_lambda_body`, lambdas now close over outer locals and parameters identically to iterator blocks.

---

## 3. Specification of Changes

### 3.1 `ruby-to-semantic-ir/src/lower.rs`
1. **`fold_one_dot_call`**:
   Before falling through to the `__method__` envelope, check `if method_name == "call"`.
   When matched:
   - Record `Feature::Closures` in `self.features_used`.
   - Return `Expr::IndirectCall { target: Box::new(receiver), args, span }`.
2. **`hoist_lambda_body`**:
   - Snapshot `enclosing_locals` and `enclosing_params` from the outer scope before resetting scope.
   - Assemble `body = Block { stmts: stmts_out, value, span: self.span_of(inner) }`.
   - Identify locally-bound names in the lambda: parameter names and any variables assigned inside the body (`Self::collect_bound_names_in_block`).
   - Identify free variable references matching `enclosing_params` or `enclosing_locals`.
   - Rewrite free references in `body` to `Scope::Capture` via `Self::recapture_reads_in_block`.
   - Populate `captures: Vec<Capture>` on the generated `Function`.
   - Return `Result<(String, Vec<CaptureValue>), RubyLowerError>`.
3. **`lower_lambda_literal`**:
   - Receive `(fn_name, capture_values)` from `hoist_lambda_body`.
   - Thread `capture_values` as the `captures` of `Expr::MakeClosure`.
   - Register `Feature::Closures`.

### 3.2 Testing & Conformance
1. **Unit tests in `ruby-to-semantic-ir/src/lib.rs`**:
   - `lambda_call_lowers_to_indirect_call`: Verifies `f.call(1, 2)` produces `Expr::IndirectCall`.
   - `lambda_captures_outer_local`: Verifies `x = 10; f = ->(y) { x + y }` populates `captures` on `MakeClosure` and the hoisted function.
   - `lambda_captures_enclosing_param`: Verifies a lambda inside a method captures the method's parameter.
   - `lambda_zero_args_call`: Verifies `f.call` with no arguments produces `IndirectCall` with empty args.
2. **Conformance tests in `sir-conformance/tests/ruby_sir_gaps.rs`**:
   - `rb_lambda_call_and_outer_captures`: Tests arrow lambdas, outer-local captures, parameter captures, and multi-argument invocation across all 7 backends (`Target::Ruby`, `Target::C`, `Target::Python`, `Target::JavaScript`, `Target::TypeScript`, `Target::Go`, `Target::Rust`).

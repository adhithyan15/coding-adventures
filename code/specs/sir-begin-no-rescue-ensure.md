# Specification: `begin … end` with No Rescue or Ensure Across Backends

**Issue**: #18798 (Item 22)  
**Packages**: `semantic-ir-to-python`, `semantic-ir-to-javascript`, `semantic-ir-to-typescript`, `semantic-ir-to-go`, `semantic-ir-to-rust`, `semantic-ir-to-c`, `sir-conformance`  
**Status**: In Implementation  

---

## 1. Problem Statement

In standard Ruby (MRI), a `begin … end` block can be used purely for grouping statements without specifying `rescue` or `ensure` clauses:

```ruby
begin
  x = 1
  puts x
end
```

The Ruby frontend (`ruby-to-semantic-ir`) lowers `begin_statement` to a `Stmt::TryCatch`:

```rust
Stmt::TryCatch {
    body,
    rescues: vec![],
    ensure_body: None,
    span,
}
```

The Semantic IR (SIR) specification and validator permit `Stmt::TryCatch` to carry empty `rescues` and a `None` `ensure_body`. However, several backend code generators assume that every `TryCatch` will have at least one `rescue` clause or an `ensure_body`:

1. **Python backend (`semantic-ir-to-python`)**:
   Emits `try:` followed by the indented body statements. It only emits `except Exception as __exc:` if `!rescues.is_empty()`, and only emits `finally:` if `ensure_body.is_some()`. When both are absent, the emitted code is:
   ```python
   try:
       x = 1
       print(x)
   ```
   This is an invalid Python compound statement and immediately raises `SyntaxError: expected 'except' or 'finally' block`.

2. **JavaScript backend (`semantic-ir-to-javascript`)**:
   Emits `try {` followed by the indented body statements and `}`. It only emits `catch (__exc) { ... }` if `!rescues.is_empty()`, and only emits `finally { ... }` if `ensure_body.is_some()`. When both are absent, the emitted code is:
   ```javascript
   try {
     let x = 1;
     console.log(x);
   }
   ```
   This is an invalid JavaScript statement and immediately throws `SyntaxError: Missing catch or finally after try`.

3. **TypeScript backend (`semantic-ir-to-typescript`)**:
   Like JavaScript, emits `try { ... }` with no `catch` or `finally`, failing compilation with `TS1005: 'catch' or 'finally' expected`.

4. **Go backend (`semantic-ir-to-go`)**:
   Wraps the body in an anonymous IIFE `func() { body }()`. If the body assigns new variables, wrapping in an inner function closure introduces unwanted lexical scoping rather than executing in the enclosing function scope.

5. **Rust backend (`semantic-ir-to-rust`)**:
   Wraps the body in `std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { ... }))` and re-raises on `Err`. This adds runtime unwinding overhead when no exception handling or cleanup is requested.

6. **C backend (`semantic-ir-to-c`)**:
   Pushes two `setjmp` handlers (`_sir_eh` and `_sir_bh`) on the runtime handler stack just to re-raise on catch.

---

## 2. Technical Architecture & Design

When a `Stmt::TryCatch` has both `rescues.is_empty()` and `ensure_body.is_none()`, there are no exception classes to match or rescue, and no cleanup code to execute on termination or unwind. Semantically, this construct is an identity wrapper around its `body` statement sequence.

### 2.1 Backend Lowering Strategy

Across all backends, when `rescues.is_empty() && ensure_body.is_none()`:
Directly emit the statements in `body` inline at the current indentation level.

1. **Python (`semantic-ir-to-python/src/emit.rs`)**:
   ```rust
   Stmt::TryCatch {
       body,
       rescues,
       ensure_body,
       ..
   } => {
       if rescues.is_empty() && ensure_body.is_none() {
           emit_stmt_list(out, body, indent, env);
           return;
       }
       // Existing try / except / finally emission
   }
   ```
   If `body` is empty, `emit_stmt_list` automatically emits `pass` at `indent` to prevent empty block syntax errors.

2. **JavaScript (`semantic-ir-to-javascript/src/emit.rs`)**:
   ```rust
   Stmt::TryCatch {
       body,
       rescues,
       ensure_body,
       ..
   } => {
       if rescues.is_empty() && ensure_body.is_none() {
           for st in body {
               emit_stmt(out, st, indent);
           }
           return;
       }
       // Existing try / catch / finally emission
   }
   ```

3. **TypeScript (`semantic-ir-to-typescript/src/emit.rs`)**:
   ```rust
   Stmt::TryCatch {
       body,
       rescues,
       ensure_body,
       ..
   } => {
       if rescues.is_empty() && ensure_body.is_none() {
           emit_stmt_block(out, body, indent);
           return;
       }
       // Existing try / catch / finally emission
   }
   ```

4. **Go (`semantic-ir-to-go/src/emit.rs`)**:
   ```rust
   Stmt::TryCatch {
       body,
       rescues,
       ensure_body,
       ..
   } => {
       if rescues.is_empty() && ensure_body.is_none() {
           emit_stmt_list(out, body, indent);
           return;
       }
       // Existing func() { ... }() defer recovery emission
   }
   ```

5. **Rust (`semantic-ir-to-rust/src/emit.rs`)**:
   ```rust
   Stmt::TryCatch {
       body,
       rescues,
       ensure_body,
       ..
   } => {
       if rescues.is_empty() && ensure_body.is_none() {
           emit_bare_stmts(out, body, indent);
           return;
       }
       // Existing catch_unwind emission
   }
   ```

6. **C (`semantic-ir-to-c/src/emit.rs`)**:
   ```rust
   Stmt::TryCatch {
       body,
       rescues,
       ensure_body,
       ..
   } => {
       if rescues.is_empty() && ensure_body.is_none() {
           for st in body {
               emit_stmt(out, st, indent);
           }
           return;
       }
       // Existing setjmp handler emission
   }
   ```

---

## 3. Conformance & Verification Plan

1. **Ruby SIR Conformance (`ruby_sir_gaps.rs`)**:
   Add test programs exercising `begin … end` without `rescue` or `ensure`:
   - Basic statement sequence inside `begin … end`.
   - Nested `begin … end` blocks.
   - `begin … end` inside method bodies.
   - Assignment inside `begin … end` visible in enclosing scope.
2. **Backend Unit Tests**:
   - Verify Python emission for empty and non-empty `begin ... end`.
   - Verify JS/TS emission compiles cleanly with `node` and `tsc`.
   - Verify Go, Rust, C compile and run cleanly.
3. **CI Matrix**:
   - `cargo test -p sir-conformance --test ruby_sir_gaps`
   - Full suite across Linux, macOS, and Windows runners.

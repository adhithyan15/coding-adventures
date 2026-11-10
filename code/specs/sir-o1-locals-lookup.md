# SIR Spec: O(1) Local Variable Lookup in Python Frontend

**Issue:** #18482 Item 6  
**Status:** Approved  
**Affected Crates:** `python-to-semantic-ir`

## Background

During expression, call, and statement lowering in `python-to-semantic-ir`, variable resolution queries (`is_enclosing_value` and `resolve_var_in`) determine whether a name is bound as a local variable, parameter, or captured variable.

Prior to this specification, `FunctionCtx` stored local variables as a flat `Vec<String>`. Every call to `is_enclosing_value` and `resolve_var_in` performed a linear scan:
```rust
self.locals.iter().any(|n| n == name)
```
For a function containing $V$ local variable definitions across $N$ expressions/statements, this produced $O(N \cdot V)$ lowering time. In large functions or generated code with many local variables, this linear search creates compilation bottlenecks and vulnerability to denial-of-service (DoS) via deeply nested or wide scopes.

## Specification

1. **Dual Representation in `FunctionCtx`**:
   `FunctionCtx` maintains both the ordered sequence of locals (to support LIFO block scoping with `mark`/`rewind`) and an active binding count map:
   - `locals: Vec<String>`: LIFO stack of local variable names mirroring the SIR validator's `LocalEnv`.
   - `locals_counts: HashMap<String, usize>`: Mapping each active local name to its multiplicity in the current scope stack.

2. **Scope Operations**:
   - `push_local(name: String)`: Appends `name` to `locals` and increments its entry in `locals_counts`.
   - `scope_rewind(mark: usize)`: Drains all locals above index `mark`, decrementing each drained variable's count in `locals_counts` and removing keys when their count reaches zero.
   - `is_local(name: &str) -> bool`: Queries `locals_counts.contains_key(name)` in $O(1)$ expected time.

3. **Constant-Time Name Resolution**:
   - `is_enclosing_value(&self, name: &str) -> bool`: Replaces the linear scan with `self.is_local(name) || self.params.contains(name) || self.captures.contains(name)`, executing in $O(1)$ time.
   - `resolve_var_in`: Uses `ctx.is_local(name)` in place of `ctx.locals.iter().any(...)`, resolving local variables in $O(1)$ time.

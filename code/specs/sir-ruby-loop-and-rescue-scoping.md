# Specification: Ruby while/until Loop Locals, Rescue Bindings, and Block Parameter Shadowing

**Issue**: #18798 (Comments 1 & 5, and review findings)  
**Package**: `ruby-to-semantic-ir`  
**Status**: Draft / In Implementation  

---

## 1. Problem Statement

In standard Ruby (MRI), local variable scoping has several distinctive characteristics that differ from block-scoped languages:

1. **`while` and `until` Loops Do Not Introduce a Scope**:
   In Ruby:
   ```ruby
   while (i ||= 0) < 1
     i += 1
     y = 5
   end
   puts y # => 5
   ```
   Locals first assigned in a `while` or `until` body (such as `y`) belong to the enclosing method or top-level script scope. If the loop does not execute (e.g. `while false; z = 1; end`), the local is still defined in the scope and initialized to `nil`.
   Currently, `ruby-to-semantic-ir` only performed pre-loop declarations (`Stmt::LetBinding { name, value: nil }`) for `for_statement` (via `collect_loop_assigned_names`), but omitted this for `while_statement` and `until_statement`. As a result, assignments in `while` loops produced bindings scoped strictly to the loop body, causing outer references to fail validator resolution.

2. **`rescue => e` Bindings Persist in the Enclosing Scope**:
   In Ruby:
   ```ruby
   begin
     raise "boom"
   rescue => e
   end
   puts e.class # => RuntimeError
   ```
   The exception variable `e` is an ordinary local variable belonging to the enclosing method or script scope. Furthermore:
   ```ruby
   e = 1
   begin
     raise "boom"
   rescue => e
   end
   puts e # => "boom"
   ```
   If `e` was already defined prior to the `begin` block, the rescue handler mutates that existing outer local.
   In Semantic IR (SIR), `RescueClause` contains a `binding: Option<String>`. Many backends (such as JavaScript, Go, Rust, and C) translate this into an exception parameter scoped strictly to the `catch` block (e.g. `catch (e)`), thereby shadowing any outer `e` and preventing mutations from escaping the catch block. Additionally, the lowerer previously removed `e` from `declared_locals` immediately upon exiting the clause (`self.declared_locals.remove(b)`).

3. **Block Parameters Shadowing Parameter Aliases and Outer Cells**:
   When an outer method parameter is mutated, the lowerer mints an alias (e.g., `__param_x_0`). Inside a block that declares its own parameter named `x` (`|x|`), reads of `x` must resolve to the block parameter `Scope::Param` (or its own local alias if mutated inside the block), rather than the outer method's alias.
   Additionally, block parameters assigned within the block (`xs.each { |x| x = x + 1 }`) must be copied into locals (or boxed) so that assignments do not mutate immutable `Scope::Param`.

---

## 2. Technical Specification

### 2.1 While and Until Loop Locals Pre-Declaration

In `ruby-to-semantic-ir/src/lower.rs`:
1. Route `while_statement` and `until_statement` through `lower_statement_inner_multi` (mirroring `for_statement`).
2. Before lowering the loop body, run `collect_loop_assigned_names` over the loop AST node.
3. For any assigned name not currently in `self.declared_locals` and not in `self.current_params`:
   - Register it in `self.declared_locals`.
   - Emit a prefix `Stmt::LetBinding { name, sir_type: None, value: Expr::NilLit, span }`.
4. Lower the loop statement (`Stmt::While`). Inside the loop body, assignments to those names will naturally lower as `Stmt::Assign` (or `Stmt::SeqSet` if boxed as cell variables), mutating the outer binding.

### 2.2 Rescue Bindings as Outer Locals

In `ruby-to-semantic-ir/src/lower.rs`:
1. In `lower_begin_statement` and `lower_rescued_method_body`:
   - Inspect all `rescue_clause` children for an exception binding (`=> e`).
   - If `e` is not already in `self.declared_locals` and not in `self.current_params`:
     - Register `e` in `self.declared_locals`.
     - Emit a prologue `Stmt::LetBinding { name: e, sir_type: None, value: Expr::NilLit, span }`.
2. In `rescue_ensure_clauses`:
   - For each clause with binding `e`:
     - Mint a fresh internal catch variable `fresh_err = self.fresh_name("__rescue_binding_")`.
     - Set the `RescueClause.binding` to `Some(fresh_err)`.
     - Prepend to the rescue clause body an assignment storing `fresh_err` into `e`:
       - If `e` is in `self.cell_vars`: `Stmt::SeqSet { target: __cell_e, index: 0, value: VarRef(fresh_err) }`.
       - If `e` is in `self.param_aliases`: `Stmt::Assign { target: alias, value: VarRef(fresh_err) }`.
       - Otherwise: `Stmt::Assign { target: e, value: VarRef(fresh_err) }`.
   - Keep `e` in `self.declared_locals` permanently rather than removing it.

### 2.3 Block Parameter Shadowing and Scoping

In `hoist_block_to_function` and `hoist_lambda_body`:
1. Save and restore `self.param_aliases`:
   ```rust
   let saved_param_aliases = self.param_aliases.clone();
   // remove any aliases shadowed by block parameters or block locals
   for p in &params { self.param_aliases.remove(&p.name); }
   for name in &block_locals { self.param_aliases.remove(name); }
   ```
2. For block parameters that are assigned within the block body, invoke `setup_param_aliases_and_prologue` to allocate fresh parameter copies and map `param_aliases`, preventing stores to `Scope::Param`.
3. Restore `self.param_aliases = saved_param_aliases` when exiting the block scope.

---

## 3. Conformance and Test Cases

1. `while_locals_persist_in_outer_scope`:
   `i = 0; while i < 1; y = 42; i += 1; end; puts y` -> validates and outputs 42.
2. `while_unexecuted_leaves_locals_as_nil`:
   `while false; z = 99; end; puts z` -> `z` starts as `nil` and validates.
3. `until_locals_persist_in_outer_scope`:
   `i = 0; until i >= 1; y = 100; i += 1; end; puts y` -> validates and outputs 100.
4. `rescue_binding_persists_in_outer_scope`:
   `begin; raise "err"; rescue => e; end; puts e` -> `e` is in outer scope and validates.
5. `rescue_binding_updates_preexisting_outer_local`:
   `e = 1; begin; raise "err"; rescue => e; end; puts e` -> `e` receives the caught error.
6. `block_param_shadows_outer_param_alias`:
   `def f(x); x = x + 1; [5].each { |x| puts x }; puts x; end` -> inside block `x` is the block param, outside it is the method param alias.
7. `block_param_assignment_copies_to_local`:
   `[1].each { |x| x = x + 10; puts x }` -> does not assign to `Scope::Param`.

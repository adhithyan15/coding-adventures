# SIR Spec: Parameter Assignment and Local Copying in Ruby

**Issue:** #18798 Item 4  
**Status:** Approved  
**Affected Crates:** `ruby-to-semantic-ir`

## Background

In Ruby, parameters are mutable local bindings. Methods and blocks may freely reassign their parameters:

```ruby
def increment(n)
  n = n + 1
  n
end
```

In Semantic IR (SIR), function parameters exist exclusively in `env.params` and are read via `Expr::VarRef { scope: Scope::Param, .. }`. They are immutable argument bindings across multiple backends (such as Rust `fn f(n: Value)` where arguments do not carry `mut`, or C and JavaScript where top-level parameter redeclaration via `let` causes duplicate identifier errors). Reassigning a parameter directly via `Stmt::Assign { name: "n", scope: Scope::Local, .. }` fails validation:

```text
ValidatorIssue { message: "var-ref scope=local references unknown name 'n'", severity: Error }
```

Prior to this specification, `ruby-to-semantic-ir` pre-inserted method parameters into `self.declared_locals`. When an assignment `n = ...` was encountered in the method body, `lower_assignment` observed `self.declared_locals.contains("n")` and emitted `Stmt::Assign { name: "n", scope: Scope::Local, .. }`. Because no `Stmt::LetBinding` had ever introduced `n` into `env.locals`, SIR validation failed.

## Specification

1. **Parameter Copy Prologue**:
   - For every callable scope with parameters (`def_statement`, `endless_def_statement`, `hoist_block_to_function`, `lower_lambda_expr`), the body statements and clauses are scanned prior to lowering to detect any assignments to parameter names.
   - For each parameter `p` that is assigned in the body:
     - Mint a unique alias local variable `__param_<name>_<counter>` claimed from `used_names`.
     - Register the mapping `p -> alias` in `self.param_aliases`.
     - Register `alias` in `self.declared_locals`.
     - Prepend a prologue binding statement:
       ```rust
       Stmt::LetBinding {
           name: alias.clone(),
           sir_type: None,
           value: Expr::VarRef {
               name: p.name.clone(),
               scope: Scope::Param,
               span: p.span.clone(),
           },
           span: p.span.clone(),
       }
       ```
   - Parameters that are never assigned in the body are left unaliased, with reads resolving directly to `Scope::Param`.

2. **Aliased Parameter Resolution**:
   - In `lower_assignment`, `lower_multi_assignment`, and `lower_rightward_assignment`, target variable names mapped in `self.param_aliases` resolve to their local alias. Because the alias is registered in `self.declared_locals`, assignments emit `Stmt::Assign { name: alias, scope: Scope::Local, .. }`.
   - In variable reference reads (identifier factor, string interpolation, value-omitted hash `{name:}` shorthand, and index assignment receivers), mapped parameters resolve to `Expr::VarRef { name: alias, scope: Scope::Local, .. }`.
   - In `bare_name_call`, `self.param_aliases.contains_key(name)` is treated as a local/parameter binding, preventing bare method call rewriting.

3. **Zero-Argument Super Forwarding**:
   - In `lower_super_expr`, when bare `super` forwards the enclosing method's parameters, any parameter that was aliased forwards the local alias with `Scope::Local` (carrying the current mutated value), while unaliased parameters forward with `Scope::Param`.

4. **Scope Hygiene**:
   - Callable scopes (`def`, endless `def`, block, lambda) save and restore `self.param_aliases` across their scope entry and exit.
   - In blocks and lambdas, inner parameters shadow any same-named parameter aliases from outer scopes.

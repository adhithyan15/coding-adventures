# sir-ruby-block-outer-local — Outer Local Variable Mutation Inside Ruby Blocks

## Status

Active design & implementation specification. Addresses item 5 of #18798.

## Motivation

In Ruby, blocks and closures (`do ... end`, `{ ... }`, `->`, `lambda`, `proc`) share the local variable
scope of their enclosing method or script. When a local variable or parameter `t` already exists in the
enclosing scope, assigning to `t` inside a block (`t = t + x` or `t += x`) mutates the outer variable
rather than creating a new block-scoped variable:

```ruby
t = 0
[1, 2, 3].each { |x| t = t + x }
puts t
```
MRI prints `6`.

Similarly with method parameters:
```ruby
def sum(xs, acc)
  xs.each { |x| acc += x }
  acc
end
```
`sum([1, 2, 3], 0)` returns `6`.

## Problem Statement

In `ruby-to-semantic-ir`:
1. `hoist_block_to_function` currently lowers block bodies with empty `declared_locals` and `current_params`
   (`std::mem::take(&mut self.declared_locals)`).
2. Any assignment `t = expr` inside the block is treated by `lower_assignment` as the first occurrence of
   `t` in that scope. It emits `Stmt::LetBinding { name: "t", value: expr }`.
3. If `expr` references `t` (as in `t = t + x`), `t` on the RHS is lowered before `LetBinding` establishes
   the binding, resulting in `VarRef("t", Scope::Local)` referencing an unknown local. The SIR validator
   refuses the program with `"var-ref scope=local references unknown name t"`.
4. Furthermore, `collect_bound_names_in_block` marks any name assigned via `LetBinding` as bound within the
   block (`block_bound.insert("t")`). Consequently, `is_free("t")` evaluates to `false`, preventing `t`
   from being captured by the closure.
5. Even if `expr` does not read `t` (e.g. `t = 42`), `t` is created as a purely block-local `LetBinding`,
   so the outer variable `t` is never updated.

## Design: Cell Boxing via 1-Element Sequences

SIR supports heap-allocated reference-typed sequences (`SeqLit`, `SeqIndex`, `SeqSet`) across all backends
(C, Ruby, Python, JavaScript, TypeScript, Go, Rust). Following the pattern established for Python closure
cell variables (PR #19052), variables in an enclosing scope that are mutated inside blocks/closures are
boxed into 1-element sequences ("cells").

### 1. Cell Variable Identification

A variable `v` in a function or top-level script `S` is identified as a cell variable if:
1. `v` is declared as a parameter or assigned as a local in `S` (outside blocks), AND
2. `v` is assigned or mutated (`=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `<<=`, `>>=`, `&=`, `|=`, `^=`,
   `||=`, `&&=`, or multi-assignment) inside a block or closure within `S`, AND
3. `v` is NOT shadowed in that block by an explicit block parameter (`|v|`) or block-local variable (`|...; v|`).

Variables that are strictly local to a block (never declared/assigned in `S`) remain normal block-local
`LetBinding`s. Variables that are only read in blocks and never mutated do not require cell boxing and
continue to use standard value captures.

### 2. Cell Prologue in Enclosing Scope

For each cell variable `v`:
- A unique cell alias `__cell_<v>_<N>` is allocated.
- In the function/script prologue:
  - If `v` is a parameter of `S`:
    `Stmt::LetStarBinding { name: alias, value: SeqLit([VarRef(v, Scope::Param)]) }`
  - If `v` is a local of `S`:
    `Stmt::LetStarBinding { name: alias, value: SeqLit([NilLit]) }`
- `Feature::Sequences` is registered in `features_used`.
- `v` is registered in `self.cell_vars` with its cell alias and scope.

### 3. Cell Reads and Writes in Enclosing Scope

- Reads of `v` resolve to:
  `Expr::SeqIndex { seq: VarRef(alias, Scope::Local), index: IntLit(0) }`
- Writes to `v` (`v = expr`) emit:
  `Stmt::SeqSet { seq: VarRef(alias, Scope::Local), index: IntLit(0), value: expr }`
- Compound assignments (`v += expr`, `v ||= expr`, etc.) read and update index 0 of `alias`.

### 4. Closure Captures and In-Block Mutation

When hoisting a block or lambda (`hoist_block_to_function` / `hoist_lambda_body`):
- Non-shadowed cell variables from `self.cell_vars` are propagated into the block's lowering context.
- Inside the block:
  - Reads of `v` resolve to:
    `Expr::SeqIndex { seq: VarRef(alias, Scope::Capture), index: IntLit(0) }`
  - Writes to `v` emit:
    `Stmt::SeqSet { seq: VarRef(alias, Scope::Capture), index: IntLit(0), value: expr }`
  - No `LetBinding` or local `Assign` is generated for `v`.
- In `MakeClosure`:
  - `alias` is added to the closure's capture list.
  - The capture value passed is `VarRef(alias, Scope::Local)` (or `Scope::Capture` if inside an enclosing block).
- Nested blocks transitively thread `alias` with `Scope::Capture`.

### 5. Shadowing Rules

In Ruby:
- `{ |t| t = t + 1 }`: `t` in `block_params` shadows outer `t`. The block treats `t` as a local parameter.
- `{ |x; t| t = 42 }`: `t` after semicolon in `block_params` is a block-local variable. It shadows outer `t`
  and starts as `nil`.

## Test Plan

- Unit tests in `ruby-to-semantic-ir`:
  - `ruby_block_updates_outer_local_accumulator`: `t = 0; [1, 2, 3].each { |x| t = t + x }; puts t` -> `6`.
  - `ruby_block_compound_assign_outer_local`: `t = 10; [1, 2].each { |x| t += x }`.
  - `ruby_block_updates_outer_parameter`: method parameter updated inside block.
  - `ruby_nested_blocks_update_outer_local`: multiple levels of block nesting updating outer accumulator.
  - `ruby_block_parameter_shadows_outer_local`: block param `|t|` does not mutate outer `t`.
  - `ruby_block_local_variable_shadows_outer_local`: block-local `; t` does not mutate outer `t`.
  - `ruby_block_local_does_not_escape`: variable assigned only in block is local to block.
- Conformance tests across backends (`sir-conformance`).

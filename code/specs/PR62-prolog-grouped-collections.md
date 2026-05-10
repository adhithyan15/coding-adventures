# PR62: Prolog Grouped Collections

## Goal

Move `bagof/3` and `setof/3` from simple collection helpers to Prolog-style
grouped collectors on the Logic VM path.

Real Prolog programs rely on the distinction between:

- collecting all answers with `findall/3`
- collecting one answer list per free-variable group with `bagof/3`
- collecting one sorted unique answer list per free-variable group with
  `setof/3`

## Scope

This batch updates:

```text
logic-builtins
prolog-loader
prolog-vm-compiler
```

## Semantics

`bagof(Template, Goal, Bag)` now:

- runs `Goal` as a nested proof search
- finds variables that occur in `Goal` but not in `Template`
- treats those free variables as grouping keys
- yields one outer solution per group
- binds each grouping variable to its group key
- preserves duplicate template values and proof order inside each bag

`setof(Template, Goal, Set)` uses the same grouping rules, then deduplicates
and sorts each group's template values by the runtime's deterministic term
ordering.

`Var^Goal` scopes inside `bagof/3` and `setof/3` mark variables existential.
Existential variables participate in the nested proof search but do not create
separate groups.

`findall/3` remains the all-solutions collector and does not group.

## Loader Contract

The loader keeps both views of a collection goal:

- the adapted executable goal, so source builtins like `member/2` still run
  through the existing builtin adapter
- the raw goal scope, so the collector can inspect source variables and `^/2`
  quantifiers

## Verification

- `logic-builtins` covers grouped `bagofo` and grouped sorted `setofo`.
- `prolog-loader` proves parsed `bagof/3` keeps grouping scope and honors
  `^/2` existential quantification.
- `prolog-vm-compiler` runs grouped `bagof/3`, grouped `setof/3`, and
  existential collection end to end through SWI-style source, loader
  adaptation, compilation, and VM execution.

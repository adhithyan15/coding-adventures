# cas-substitution (Rust)

Structural and pattern-aware substitution over `symbolic-ir` trees.
Rust port of the Python `cas-substitution` package.

## `subst` — structural substitution

Replace every occurrence of a node (symbol or compound expression) with a value:

```rust
use symbolic_ir::{apply, int, sym, POW};
use cas_substitution::subst;

// subst(2, x, x^2) → Pow(2, 2)  (un-simplified)
let expr = apply(sym(POW), vec![sym("x"), int(2)]);
let result = subst(int(2), &sym("x"), expr);
assert_eq!(result, apply(sym(POW), vec![int(2), int(2)]));
```

Multiple substitutions in sequence:

```rust
use symbolic_ir::{apply, int, sym, ADD};
use cas_substitution::subst_many;

let expr = apply(sym(ADD), vec![sym("x"), sym("y")]);
let rules = vec![(sym("x"), int(2)), (sym("y"), int(3))];
assert_eq!(subst_many(&rules, expr), apply(sym(ADD), vec![int(2), int(3)]));
```

## `replace_all` — pattern-aware substitution

Apply a `Rule(lhs, rhs)` pattern everywhere it matches (top-down, single-pass):

```rust
use symbolic_ir::{apply, int, sym, MUL, POW};
use cas_pattern_matching::{blank, named, rule};
use cas_substitution::replace_all;

// Rule: Pow(a_, 2) → Mul(a_, a_)
let r = rule(
    apply(sym(POW), vec![named("a", blank()), int(2)]),
    apply(sym(MUL), vec![named("a", blank()), named("a", blank())]),
);
let expr = apply(sym(POW), vec![sym("y"), int(2)]);
assert_eq!(replace_all(expr, &r), apply(sym(MUL), vec![sym("y"), sym("y")]));
```

## Stack position

```
symbolic-ir  ←  cas-pattern-matching  ←  cas-substitution
```

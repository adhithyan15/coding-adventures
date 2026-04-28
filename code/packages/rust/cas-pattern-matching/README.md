# cas-pattern-matching (Rust)

Structural pattern matching and term rewriting over `symbolic-ir` trees.
Rust port of the Python `cas-pattern-matching` package.

## Pattern nodes

Patterns are ordinary `IRNode` trees using three sentinel heads:

| Constructor | Head | Matches |
|-------------|------|---------|
| `blank()` | `Blank()` | Any expression |
| `blank_typed("T")` | `Blank(T)` | Expressions whose head is `T` |
| `named("x", inner)` | `Pattern(x, inner)` | Like `inner`, capturing `x → target` |

## Match

```rust
use symbolic_ir::{apply, int, sym, ADD};
use cas_pattern_matching::{blank, named, match_pattern, Bindings};

// Add(x_, y_)  matches  Add(2, 3)  and captures  x=2, y=3
let pat = apply(sym(ADD), vec![named("x", blank()), named("y", blank())]);
let target = apply(sym(ADD), vec![int(2), int(3)]);
let b = match_pattern(&pat, &target, Bindings::empty()).unwrap();
assert_eq!(b.get("x"), Some(&int(2)));
assert_eq!(b.get("y"), Some(&int(3)));
```

## Rewrite

```rust
use cas_pattern_matching::{blank, named, rule, rewrite};
use symbolic_ir::{apply, int, sym, ADD};

// Rule: x_ + 0  →  x_
let x = named("x", blank());
let r = rule(
    apply(sym(ADD), vec![x.clone(), int(0)]),
    x.clone(),
);
// Applied bottom-up to  Add(Add(z, 0), 0)  →  z
let expr = apply(sym(ADD), vec![apply(sym(ADD), vec![sym("z"), int(0)]), int(0)]);
assert_eq!(rewrite(expr, &[r], 100).unwrap(), sym("z"));
```

## Note on RHS patterns

RHS expressions reference captured variables via `Pattern(name, inner)` nodes
(e.g. `named("x", blank())`), **not** bare `Symbol("x")` nodes.  This mirrors
the Python implementation and Mathematica's `Rule[lhs, rhs]` semantics.

## Stack position

```
symbolic-ir  ←  cas-pattern-matching
                    ↓
              cas-simplify (planned)
```

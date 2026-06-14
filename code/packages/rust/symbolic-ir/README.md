# symbolic-ir (Rust)

The universal symbolic expression IR — the shared tree representation
that every computer-algebra-system frontend compiles to and every CAS
backend consumes.

This is the Rust port of the Python `symbolic-ir` package.  The API and
semantics are identical; only the language idioms differ.

## Node types

| Variant | Purpose | Example |
|---------|---------|---------|
| `Symbol(String)` | Named atom: variable, constant, or head | `sym("x")`, `sym("Pi")` |
| `Integer(i64)` | Integer literal | `int(42)`, `int(-7)` |
| `Rational(i64, i64)` | Exact fraction, always reduced | `rat(1, 2)` → `1/2` |
| `Float(f64)` | Double-precision float | `flt(3.14)` |
| `Str(String)` | String literal | `str_node("hello")` |
| `Apply(Box<IRApply>)` | Compound: `head(args...)` | `apply(sym("Add"), vec![…])` |

Every compound expression is an `Apply`.  The head is always a `Symbol`
naming an operation.  This Lisp-like uniformity keeps all tree-walking
code simple.

## Quick start

```rust
use symbolic_ir::{apply, int, sym, ADD, MUL, POW};

// Build  x^2 + 2*x + 1
let x = sym("x");
let x_sq = apply(sym(POW), vec![x.clone(), int(2)]);
let two_x = apply(sym(MUL), vec![int(2), x.clone()]);
let expr = apply(sym(ADD), vec![x_sq, two_x, int(1)]);

println!("{expr}");
// Add(Pow(x, 2), Mul(2, x), 1)
```

## Standard head constants

All standard operation names are exposed as `&'static str` constants:

```rust
use symbolic_ir::{ADD, SUB, MUL, DIV, POW, SIN, COS, EXP, LOG, DEFINE};
// use them to build symbol nodes without typos:
let add = sym(ADD);  // IRNode::Symbol("Add")
```

## Design notes

- `IRNode` implements `Clone`, `PartialEq`, `Eq`, and `Hash`.
- Float equality uses `f64::to_bits()` for a deterministic total order.
- `IRNode::rational(n, d)` normalises fractions (GCD reduction, sign in
  numerator) and collapses to `Integer` when the denominator becomes 1.
- All nodes are owned values; cloning a compound expression is O(depth).
  Future versions may add `Arc<IRApply>` for cheap shared sub-trees.

## Relationship to the Python package

This crate is a direct port.  The six node types map 1:1.  The only
differences are language-level:

| Python | Rust |
|--------|------|
| `IRSymbol("x")` | `sym("x")` or `IRNode::Symbol("x".into())` |
| `IRInteger(42)` | `int(42)` or `IRNode::Integer(42)` |
| `IRRational(1, 2)` | `rat(1, 2)` or `IRNode::rational(1, 2)` |
| `IRFloat(3.14)` | `flt(3.14)` or `IRNode::Float(3.14)` |
| `IRString("s")` | `str_node("s")` or `IRNode::Str("s".into())` |
| `IRApply(head, args)` | `apply(head, args_vec)` or direct `IRNode::Apply` |
| `ADD = IRSymbol("Add")` | `use symbolic_ir::ADD; sym(ADD)` |

## Stack position

`symbolic-ir` is the bottom layer of the CAS stack.  Nothing in this crate
depends on any other crate in the repository.  All higher-level packages
(`symbolic-vm`, `cas-simplify`, `macsyma-compiler`, …) depend on it.

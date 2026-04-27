# cas-pretty-printer (Rust)

Dialect-aware pretty-printer that renders symbolic IR back to source text.

Rust port of the Python `cas-pretty-printer` package.

## Operations

### `pretty(node, dialect) -> String`

Format `node` as source text in the given dialect.  Currently produces
single-line (linear) output.

### `format_lisp(node) -> String`

Format `node` as an always-prefix S-expression, bypassing the walker and
all registered head formatters.  Useful for debugging the raw IR shape.

## Dialects

| Struct | Language | Lists | Calls | Equality |
|---|---|---|---|---|
| `MacsymaDialect` | MACSYMA / Maxima | `[…]` | `f(…)` | `=` / `#` |
| `MathematicaDialect` | Mathematica | `{…}` | `f[…]` | `==` / `!=` |
| `MapleDialect` | Maple | `[…]` | `f(…)` | `=` / `<>` |
| `LispDialect` | Lisp S-expr | `(List …)` | `(f …)` | `(Equal a b)` |

All four dialects share the same arithmetic sugar:
- `Mul(-1, x)` → `-x`
- `Add(a, Neg(b))` → `a - b`
- `Mul(a, Inv(b))` → `a/b`

## Usage

```rust
use cas_pretty_printer::{pretty, format_lisp, MacsymaDialect, MathematicaDialect};
use symbolic_ir::{apply, int, sym, ADD, POW, SIN};

let x = sym("x");

// MACSYMA
let expr = apply(sym(ADD), vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)]);
assert_eq!(pretty(&expr, &MacsymaDialect), "x^2 + 1");

// Mathematica
let call = apply(sym(SIN), vec![x.clone()]);
assert_eq!(pretty(&call, &MathematicaDialect), "Sin[x]");

// Lisp debug form
assert_eq!(format_lisp(&expr), "(Add (Pow x 2) 1)");
```

## Extensibility

Downstream crates can register formatters for new IR heads:

```rust
use cas_pretty_printer::{register_head_formatter, unregister_head_formatter};
use symbolic_ir::IRNode;

register_head_formatter("Matrix", |node, _dialect, fmt| {
    let rows: Vec<String> = node.args.iter()
        .map(|row| {
            if let IRNode::Apply(a) = row {
                let cells: Vec<String> = a.args.iter().map(|c| fmt(c)).collect();
                format!("[{}]", cells.join(", "))
            } else { fmt(row) }
        })
        .collect();
    format!("matrix({})", rows.join(", "))
});
```

## Precedence constants

| Constant | Value | Binds |
|---|---|---|
| `PREC_OR` | 10 | `or` |
| `PREC_AND` | 20 | `and` |
| `PREC_NOT` | 25 | `not` |
| `PREC_CMP` | 30 | `=`, `<`, `>`, … |
| `PREC_ADD` | 40 | `+`, `-` |
| `PREC_MUL` | 50 | `*`, `/` |
| `PREC_NEG` | 55 | unary `-` |
| `PREC_POW` | 60 | `^` |
| `PREC_CALL` | 70 | function calls |
| `PREC_ATOM` | 100 | leaf atoms |

## Stack position

```
symbolic-ir  ←  cas-pretty-printer
```

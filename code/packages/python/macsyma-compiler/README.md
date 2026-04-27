# macsyma-compiler

Lowers a parsed MACSYMA AST to the universal symbolic IR defined in
`symbolic-ir`.

## What this package is

This is where the real translation from "MACSYMA surface syntax" to
"universal symbolic computation" happens. The `macsyma-parser` produces
a deeply nested AST that mirrors the precedence cascade; this compiler
flattens it into the uniform `IRApply(head, args)` shape.

Key transformations:

| MACSYMA source   | IR shape                                         |
|------------------|--------------------------------------------------|
| `x + 1`          | `Add(x, 1)`                                      |
| `a - b - c`      | `Sub(Sub(a, b), c)`  (left-associative)          |
| `a^b^c`          | `Pow(a, Pow(b, c))`  (right-associative)         |
| `-x`             | `Neg(x)`                                         |
| `f(x, y)`        | `Apply(Symbol('f'), (x, y))`                     |
| `diff(f, x)`     | `D(f, x)`  (standard function rewritten)         |
| `a : 5`          | `Assign(a, 5)`                                   |
| `f(x) := x^2`    | `Define(f, List(x), Pow(x, 2))`                  |
| `[1, 2, 3]`      | `List(1, 2, 3)`                                  |
| `a and b and c`  | `And(a, b, c)`  (variadic, not nested)           |

## Usage

```python
from macsyma_parser import parse_macsyma
from macsyma_compiler import compile_macsyma

ast = parse_macsyma("diff(x^2 + 1, x);")
[ir] = compile_macsyma(ast)
print(ir)
# D(Add(Pow(x, 2), 1), x)
```

## Standard function rewriting

The compiler recognizes a handful of MACSYMA names and rewrites them to
canonical IR heads so downstream backends can dispatch uniformly:

- `diff` → `D`
- `integrate` → `Integrate`
- `sin`, `cos`, `log`, `exp`, `sqrt` → their capitalized heads

Any other name stays as a user-defined `IRSymbol`, so `f(x)` keeps
`f` as the head when `f` isn't a known MACSYMA builtin.

## Dependencies

- `coding-adventures-symbolic-ir` — the IR types.
- `coding-adventures-macsyma-parser` — provides the AST.
- (transitive) the full grammar-tools/lexer/parser stack.

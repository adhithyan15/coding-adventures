# symbolic-vm

A tiny, pluggable virtual machine for evaluating `symbolic_ir` trees.

## What this package is

The VM is a generic tree walker. Every evaluation policy decision — how
to resolve names, what to do with unknown symbols, which rewrite rules
to try, how each head is evaluated — is delegated to a `Backend`. Two
reference backends ship in the box:

| Backend           | Unbound name          | Unknown head          | Arithmetic on symbols |
|-------------------|------------------------|-----------------------|-----------------------|
| `StrictBackend`   | raises `NameError`    | raises `NameError`    | raises `TypeError`    |
| `SymbolicBackend` | returns symbol as-is  | returns expr as-is    | applies identities, else leaves expression |

`StrictBackend` is the "calculator" — bind your variables, get numbers
back. `SymbolicBackend` is the CAS-oriented backend — free variables
survive, algebraic identities collapse, and calculus, algebra, summation,
and integration handlers layer on top of the same VM dispatch loop.

## How the VM decides

```
eval(node):
    atom        → look up / on_unresolved / return literal
    IRApply:
      1. evaluate args unless head is held (Assign, Define, If)
      2. try rewrite rules  → if one matches, eval(transform(expr))
      3. dispatch to handlers()[head_name]
      4. if head is bound to a Define record, substitute + eval
      5. fall through to on_unknown_head
```

The core handler table is shared between strict and symbolic backends.
CAS-only heads such as `D`, `Integrate`, `Sum`, and `Product` live on the
symbolic side.

## Usage

```python
from macsyma_parser import parse_macsyma
from macsyma_compiler import compile_macsyma
from symbolic_vm import VM, SymbolicBackend

src = "f(x) := x^2; diff(f(x), x);"
statements = compile_macsyma(parse_macsyma(src))
vm = VM(SymbolicBackend())
print(vm.eval_program(statements))
# Mul(2, x)
```

## What is included

- Arithmetic: `Add`, `Sub`, `Mul`, `Div`, `Pow`, `Neg`, `Inv` — exact
  when possible (Fraction internally), floats contaminate to floats.
- Elementary: `Sin`, `Cos`, `Exp`, `Log`, `Sqrt` — fold numerics,
  preserve a handful of exact identities (`Sin(0) = 0`, `Log(1) = 0`,
  etc.), leave symbolic args alone.
- Comparisons: `Equal`, `NotEqual`, `Less`, `Greater`, `LessEqual`,
  `GreaterEqual` — return the `True` / `False` symbols for concrete
  arguments.
- Logic: `And`, `Or`, `Not` — short-circuit over literal booleans.
- Calculus: `D` (symbolic backend only) — sum, difference, product,
  quotient, general power, and chain rules for elementary, inverse trig,
  hyperbolic, and selected special functions.
- Integration: `Integrate` (symbolic backend only) — elementary pattern
  rules, rational-function integration, trigonometric/hyperbolic families,
  u-substitution, definite integrals, special-function fallbacks, and
  tabular integration-by-parts fallback.
- Summation and products: `Sum` / `Product` handlers cover constant,
  geometric, power, telescoping, selected special-series, factorial, and
  finite-product shapes.
- CAS bridges: selected handlers delegate into companion CAS packages for
  simplification, factorization, substitution, equation solving, lists,
  matrices, limits/series, number theory, complex arithmetic, transforms,
  ODEs, algebraic numbers, multivariate polynomials, and summation helpers.
- Binding: `Assign` (eager), `Define` (delayed, for function bodies).
- `If` — held; only the chosen branch runs.

## What is not (yet) included

- No polynomial normalization. `x + x` stays `Add(x, x)`; we don't
  combine like terms. That's a separate simplification pass.
- No pattern-variable rewrite rules (`x_` in Mathematica). The
  `rules()` hook accepts predicate/transform pairs, so a real pattern
  matcher can be layered on without changing the VM.
- No full Risch integration algorithm. Integration is broad but intentionally
  rule/algorithm driven rather than a complete decision procedure.
- No complete expression canonicalization. Companion simplifier packages cover
  many shapes, but this VM still preserves unreduced structure when no handler
  owns a form.

## Extending with a new backend

A new language-specific backend is typically a subclass that adds or
replaces handlers and rules:

```python
class MapleBackend(SymbolicBackend):
    def handlers(self):
        base = dict(super().handlers())
        base["Range"] = _range_handler
        return base
```

That's the "80% reuse" promise: everything above — the walker, arithmetic
folding, identity rewrites, calculus handlers, and CAS bridge points — comes
for free.

## Dependencies

- `coding-adventures-symbolic-ir` — the IR node types.
- `coding-adventures-polynomial` and the `coding-adventures-cas-*`
  packages — algebra, calculus, simplification, and helper algorithms used by
  the symbolic backend.
- `coding-adventures-macsyma-parser` and `coding-adventures-macsyma-compiler`
  — end-to-end MACSYMA examples and tests.

Runtime has zero capabilities (`required_capabilities.json`): the VM
is a pure in-memory tree walker.

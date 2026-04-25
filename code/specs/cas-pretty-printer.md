# cas-pretty-printer — Symbolic IR Pretty-Printing

> **Status**: New spec. Defines a small standalone Python package
> `cas-pretty-printer` that turns symbolic IR back into source text.
> Part of the symbolic-VM-as-substrate program described in
> `symbolic-computation.md`.

## Why this package exists

A CAS is useless without a way to display results. The repo has a rich
symbolic IR (`symbolic_ir`) and a pluggable VM (`symbolic_vm`), but no
inverse — IR → string. The default `__repr__` of an `IRApply` is suitable
for debugging (`IRApply(IRSymbol('Add'), (IRInteger(2), IRSymbol('x')))`),
not for a REPL session where users expect `2 + x`.

This package is the **language-neutral pretty-printing substrate**. It
walks an IR tree and emits source text. The exact dialect — MACSYMA,
Mathematica, Maple, REDUCE, Matlab, plain Lisp — is supplied by a small
**Dialect** object that names the operators, function names, and number
format. The walker is shared across every dialect.

## Reuse story

`cas-pretty-printer` is intentionally not a MACSYMA-specific package.

- **MACSYMA / Maxima**: dialect uses `^` for power, `+` for `Add`, `*`
  for `Mul`, `[a, b]` for `List`, `f(x, y)` call syntax, lowercase
  `sin`/`cos`/`log`/`exp`. Default fraction display: `a/b`.
- **Mathematica**: `^` for power, `*` for `Mul`, `{a, b}` for `List`,
  `Sin[x]`/`Cos[x]` square-bracket calls.
- **Maple**: `^` or `**` for power, brackets for `List`, lowercase
  function names.
- **Matlab/Octave**: `^` for power, `[a, b]` (row vector), `;` for matrix
  row separator, `sin(x)` calls.
- **Plain Lisp** (debug helper): `(Add 2 x)` prefix form regardless of
  head. Useful when building a new dialect.

A future `matlab-pretty-printer` is just a `MatlabDialect` instance — no
new walker.

## Scope

In:

- IR → string for every IR node type (`IRSymbol`, `IRInteger`,
  `IRRational`, `IRFloat`, `IRString`, `IRApply`).
- Operator precedence and parenthesization (only insert parens when
  required by the dialect's precedence table — never gratuitous).
- Dialect-controlled spelling of the standard heads (`Add`, `Mul`,
  `Pow`, `Neg`, `Sub`, `Div`, `Inv`, `Sqrt`, elementary functions,
  comparison/logic, `List`, `Equal`, etc.).
- Two output styles per dialect:
  - **Linear**: single-line text suitable for REPLs and tests.
  - **2D ASCII** (optional, can ship in a later phase): stacked
    fractions, raised exponents, surd glyphs. Falls back to linear
    when 2D is not available.
- A `register_head(name, formatter)` hook so downstream packages
  (like `cas-matrix`) can teach the printer about new heads they
  introduce, without modifying this package.

Out:

- LaTeX rendering (separate package, future).
- HTML / MathML (separate package, future).
- Color or terminal control codes (separate package, future).
- Parsing in the reverse direction — that's `macsyma-parser` etc.

## Public API

```python
from cas_pretty_printer import (
    Dialect,
    MacsymaDialect,
    MathematicaDialect,
    MapleDialect,
    LispDialect,           # always-prefix debugging dialect
    pretty,                # main entry point
    register_head_formatter,
)

text = pretty(ir_node, dialect=MacsymaDialect())
text_2d = pretty(ir_node, dialect=MacsymaDialect(), style="2d")
```

### `Dialect` protocol

```python
class Dialect(Protocol):
    name: str

    # Numeric formatting
    def format_integer(self, value: int) -> str: ...
    def format_rational(self, numer: int, denom: int) -> str: ...
    def format_float(self, value: float) -> str: ...
    def format_string(self, value: str) -> str: ...

    # Operator spellings — return None to fall back to function-call form.
    def binary_op(self, head_name: str) -> str | None: ...
    def unary_op(self, head_name: str) -> str | None: ...

    # Function-call spellings (e.g. `Sin` → `"sin"` in MACSYMA).
    def function_name(self, head_name: str) -> str: ...

    # Container delimiters.
    def list_brackets(self) -> tuple[str, str]: ...     # ("[", "]")
    def call_brackets(self) -> tuple[str, str]: ...     # ("(", ")")

    # Precedence table — head_name → integer precedence (higher = binds tighter).
    # The walker uses this to decide where to insert parens.
    def precedence(self, head_name: str) -> int: ...
    def is_right_associative(self, head_name: str) -> bool: ...
```

A `BaseDialect` ABC provides sensible defaults for everything except
`name` and the operator/function tables, so a new dialect is a 30-line
subclass.

## Algorithm

A single `_format(node, parent_prec, parent_assoc) -> str` recursive
walker:

1. Leaf → `format_integer` / `format_rational` / `format_float` /
   `format_string` / bare symbol name.
2. `IRApply(head, args)`:
   - If the head is a unary operator (`Neg`), format as `op{arg}`.
   - If the head is a binary operator (`Add`, `Mul`, `Sub`, `Div`,
     `Pow`, comparison, logic), format `arg op arg op …` with
     parenthesization governed by precedence.
   - Otherwise, format as `function_name(arg, arg, …)` using
     `call_brackets`.
3. Inserted parens iff: child precedence < parent precedence, OR child
   precedence == parent precedence and the child is on the
   "wrong side" for the parent's associativity.

Special cases the dialect can override:
- `Add(x, Neg(y))` displays as `x - y` in math dialects.
- `Mul(x, Pow(y, -1))` displays as `x / y` in math dialects.
- `Mul(-1, x)` displays as `-x` in math dialects.
- `Pow(x, 1/2)` displays as `sqrt(x)` if the dialect prefers.

## Heads added

None. This package consumes heads but does not introduce any new ones.

## Test strategy

`tests/test_macsyma_dialect.py`:

- All node types round-trip through pretty → parse → IR → pretty
  (idempotence check using the existing `macsyma-{lexer,parser,compiler}`).
- Precedence: `Add(x, Mul(y, z))` → `x + y*z` (no parens around `y*z`);
  `Mul(Add(x, y), z)` → `(x + y)*z`.
- Power right-associativity: `Pow(a, Pow(b, c))` → `a^b^c`.
- Power left-side: `Pow(Pow(a, b), c)` → `(a^b)^c`.
- Negation: `Neg(Add(x, y))` → `-(x + y)`; `Neg(x)` → `-x`.
- Subtraction sugar: `Add(x, Neg(y))` → `x - y`.
- Division sugar: `Mul(x, Inv(y))` → `x/y`.
- Lists: `List(1, 2, 3)` → `[1, 2, 3]`.
- Function calls: `Sin(x)` → `sin(x)`; `Diff(x^2, x)` → `diff(x^2, x)`.

`tests/test_lisp_dialect.py`:

- Always-prefix output: `Add(x, y)` → `(Add x y)`; useful for debugging.

`tests/test_dialect_extensibility.py`:

- A custom dialect with a new head registered via
  `register_head_formatter` produces the expected output without
  modifying the package.

Coverage target: ≥95% (this is a library — pretty-printing has very
predictable code paths).

## Package layout

```
code/packages/python/cas-pretty-printer/
  pyproject.toml
  BUILD
  BUILD_windows
  README.md
  CHANGELOG.md
  required_capabilities.json
  src/cas_pretty_printer/
    __init__.py
    dialect.py          # Dialect protocol + BaseDialect
    walker.py           # _format recursive walker
    macsyma.py          # MacsymaDialect
    mathematica.py      # MathematicaDialect
    maple.py            # MapleDialect
    lisp.py             # LispDialect (debugging)
    py.typed
  tests/
    test_macsyma_dialect.py
    test_mathematica_dialect.py
    test_lisp_dialect.py
    test_dialect_extensibility.py
```

Dependencies: `coding-adventures-symbolic-ir` only.

## Future extensions

- `style="2d"` — stacked fractions, raised exponents in ASCII.
- `latex.py` — `LaTeXDialect` rendering to `\frac{a}{b}`, `x^{n}`.
- `mathml.py` — MathML 3 rendering for HTML embedding.
- Color / terminal styling via a separate `cas-pretty-printer-color`
  package that wraps this one.

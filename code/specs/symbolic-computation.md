# Symbolic Computation Pipeline

## Overview

This spec defines a Computer Algebra System (CAS) infrastructure built on the
grammar-driven lexer/parser machinery already present in the repo. The goal is
to support multiple CAS language frontends (MACSYMA, Mathematica, Maple, REDUCE,
SymPy expressions) that all compile down to a single universal **symbolic IR**
and execute on a single pluggable **symbolic VM** with language-specific
evaluation policies layered on top.

This spec covers the end-to-end MACSYMA pipeline as the reference
implementation. MACSYMA is chosen because it is the grandparent of every
modern CAS — Maxima, Mathematica, Maple, and REDUCE all inherited from it or
from its immediate contemporaries.

## Why a Universal Pipeline?

A CAS is, at its core, an expression tree walker with rules. The tree
structure — `head applied to args` — is the same in every CAS that has ever
existed. Only the surface syntax differs:

| Language    | Source                  | Tree after parsing           |
|-------------|-------------------------|------------------------------|
| MACSYMA     | `diff(x^2 + 1, x)`      | `D(Add(Pow(x,2), 1), x)`     |
| Mathematica | `D[x^2 + 1, x]`         | `D(Add(Pow(x,2), 1), x)`     |
| Maple       | `diff(x^2 + 1, x)`      | `D(Add(Pow(x,2), 1), x)`     |
| REDUCE      | `df(x^2 + 1, x)`        | `D(Add(Pow(x,2), 1), x)`     |

Once we parse into the universal IR, every downstream operation —
differentiation, simplification, integration, numeric evaluation, LaTeX
rendering, transpilation — runs on the same trees regardless of which
frontend produced them.

## Architecture

```
┌───────────────────┐
│ Source text       │   "f(x) := x^2; diff(f(x), x)"
└────────┬──────────┘
         │
         ▼
┌───────────────────┐    code/grammars/macsyma/macsyma.tokens
│ macsyma-lexer     │──▶ uses GrammarLexer
└────────┬──────────┘    produces Token stream
         │
         ▼
┌───────────────────┐    code/grammars/macsyma/macsyma.grammar
│ macsyma-parser    │──▶ uses GrammarParser
└────────┬──────────┘    produces ASTNode tree
         │
         ▼
┌───────────────────┐
│ macsyma-compiler  │──▶ walks ASTNode tree
└────────┬──────────┘    emits symbolic IR
         │
         ▼
┌───────────────────┐
│ symbolic-ir       │    IRNode = Symbol | Integer | Rational | Float
│                   │            | String | Apply(head, args)
└────────┬──────────┘
         │
         ▼
┌───────────────────┐
│ symbolic-vm       │    Backend interface + StrictVM + SymbolicVM
└───────────────────┘    evaluates IR, returns IR
```

## The Symbolic IR

The IR is a small set of immutable, hashable node types. Every compound
expression is an `Apply(head, args)` where `head` is an `IRSymbol` naming a
standard operation and `args` is a tuple of child `IRNode`s.

### Node Types

```python
IRNode (abstract)
├── IRSymbol(name: str)          # x, y, Pi, E
├── IRInteger(value: int)        # arbitrary precision
├── IRRational(numer: int, denom: int)  # exact fractions, always normalized
├── IRFloat(value: float)
├── IRString(value: str)
└── IRApply(head: IRNode, args: tuple[IRNode, ...])
```

All nodes are frozen dataclasses (immutable, hashable) so they can be used as
dict keys (essential for rule-matching caches) and compared structurally.

### Standard Heads

The head of an `IRApply` is always an `IRSymbol`. The following names are
reserved as "standard operations" that every backend understands:

```
Arithmetic:   Add  Mul  Pow  Neg  Inv  Sub  Div
Elementary:   Exp  Log  Sin  Cos  Tan  Sqrt  Abs
Calculus:     D  Integrate  Limit  Series  Sum  Product
Algebra:      Solve  Factor  Expand  Simplify
Relations:    Equal  NotEqual  Less  Greater  LessEqual  GreaterEqual
Logic:        And  Or  Not  If
Containers:   List  Matrix  Set
Assignment:   Assign   (non-delayed: evaluate rhs, bind to lhs)
              Define   (delayed: store rhs unevaluated for function defs)
Rules:        Rule       (x -> expr: rewrite pattern)
              RuleDelayed (x :> expr)
```

Any frontend that wants to add domain-specific operations can introduce new
heads; the VM's behavior when it encounters an unknown head depends on the
evaluation mode.

## The Pluggable VM

The VM is a generic tree walker. The evaluation strategy lives in a
`Backend` object that supplies three things: a name lookup policy, a set of
rewrite rules, and per-head evaluators.

### Backend Interface

```python
class Backend(Protocol):
    def lookup(self, name: str) -> IRNode | None: ...
    def on_unresolved(self, symbol: IRSymbol) -> IRNode: ...
    def rules(self) -> Iterable[tuple[IRNode, Callable[..., IRNode]]]: ...
    def handlers(self) -> Mapping[str, Callable[[VM, IRApply], IRNode]]: ...
```

- `lookup(name)` — returns the stored value for a symbol, or `None`.
- `on_unresolved(sym)` — what to do when a symbol has no binding:
  - **Strict backend:** raises `NameError`.
  - **Symbolic backend:** returns `sym` unchanged.
- `rules()` — yields `(pattern, transform)` pairs. The VM tries each rule on
  every `IRApply` before falling back to handlers.
- `handlers()` — maps head names to specialized evaluators. This is where
  `Add`, `Mul`, `D`, etc. get their behavior.

### Shared Core (~80% of the VM)

The VM's generic logic is reused across every backend:

1. Walk arguments first (applicative order) unless the head is in a "hold"
   list (`Define`, `RuleDelayed`, `If`, `Quote`).
2. After arguments are evaluated, attempt each rewrite rule in `rules()`.
   If one matches, recursively evaluate the rewritten expression.
3. If no rule matches, dispatch to the head-specific handler from
   `handlers()`.
4. If no handler exists, either leave the expression as-is (symbolic mode)
   or raise (strict mode).

### Language Quirks via Backend Subclassing

Language-specific quirks are small deltas on the base:

- **MACSYMA**: `:` is assignment (`Assign`), `:=` is definition (`Define`).
  Boolean `and`/`or` are short-circuited. `%pi`, `%e`, `%i` are constants.
- **Mathematica**: pattern variables `x_` match any expression;
  `Hold` attributes prevent argument evaluation; rules have much richer
  pattern matching.
- **Maple**: range syntax `a..b` becomes `Apply(Range, [a, b])`.

Each language ships a `Backend` subclass that overrides `handlers()` and
`rules()` for its quirks while reusing the generic core.

## The MACSYMA Frontend

### Tokens (summary)

- Numbers: integer (`42`), float (`3.14`, `1.5e10`)
- Names: `[a-zA-Z_][a-zA-Z0-9_]*` and `%pi`, `%e`, `%i`
- Strings: double-quoted
- Operators: `+ - * / ^ ** : := = # < > <= >= -> !`
- Delimiters: `( ) [ ] { } , ; $`
- Keywords: `and or not if then else elseif true false`
- Comments: `/* ... */`
- Skip: whitespace, newlines

### Grammar (summary)

```
program        = { statement } ;
statement      = expression ( SEMI | DOLLAR ) ;
expression     = assign_expr ;
assign_expr    = logical_or [ ( COLON | COLONEQ ) assign_expr ] ;
logical_or     = logical_and { OR logical_and } ;
logical_and    = logical_not { AND logical_not } ;
logical_not    = [ NOT ] comparison ;
comparison     = additive [ cmp_op additive ] ;
cmp_op         = EQ | HASH | LT | GT | LEQ | GEQ ;
additive       = multiplicative { ( PLUS | MINUS ) multiplicative } ;
multiplicative = unary { ( STAR | SLASH ) unary } ;
unary          = ( MINUS | PLUS ) unary | power ;
power          = postfix [ CARET unary ] ;
postfix        = atom { call_suffix } ;
call_suffix    = LPAREN [ arglist ] RPAREN ;
arglist        = expression { COMMA expression } ;
atom           = NUMBER | STRING | NAME | group | list ;
group          = LPAREN expression RPAREN ;
list           = LBRACKET [ arglist ] RBRACKET ;
```

Function definitions `f(x) := body` parse naturally: `f(x)` is a `postfix`
with a call suffix, and `:=` continues as the assign operator, so the whole
thing is `Assign(Call(f, x), body)` at the AST level. The compiler detects
`:=` and emits `Define(f, [x], body)` in the IR.

### AST → IR Compilation Rules

| AST shape                                | IR shape                              |
|------------------------------------------|---------------------------------------|
| `NUMBER`                                 | `IRInteger` or `IRFloat`              |
| `NAME`                                   | `IRSymbol(name)`                      |
| `Call(f, [args...])`                     | `IRApply(IRSymbol(f), [args...])`     |
| `BinaryOp("+", a, b)`                    | `IRApply(Add, [a, b])`                |
| `BinaryOp("*", a, b)`                    | `IRApply(Mul, [a, b])`                |
| `BinaryOp("^", a, b)`                    | `IRApply(Pow, [a, b])`                |
| `UnaryOp("-", a)`                        | `IRApply(Neg, [a])`                   |
| `BinaryOp(":", name, rhs)`               | `IRApply(Assign, [IRSymbol(name), rhs])` |
| `BinaryOp(":=", Call(f, params), body)`  | `IRApply(Define, [IRSymbol(f), List(params), body])` |
| `List([e1, e2, ...])`                    | `IRApply(List, [e1, e2, ...])`        |
| Well-known names (`diff`, `integrate`)   | Replace head with standard head (`D`, `Integrate`) |

## The VM Modes

### StrictVM

- `on_unresolved(x)` raises `NameError("symbol 'x' is not defined")`.
- Numeric handlers evaluate fully (e.g., `Add(2, 3) → 5`).
- No rewrite rules by default.
- Behaves like a numeric evaluator that happens to understand MACSYMA syntax.

### SymbolicVM

- `on_unresolved(x)` returns `x` unchanged.
- Numeric handlers evaluate constants but keep symbols: `Add(x, 2, 3) → Add(x, 5)`.
- Built-in rewrite rules for:
  - Identity elements: `Add(x, 0) → x`, `Mul(x, 1) → x`, `Pow(x, 1) → x`.
  - Zero laws: `Mul(x, 0) → 0`, `Pow(x, 0) → 1`.
  - Derivative rules: sum rule, product rule, chain rule, power rule.
- Behaves like a miniature Mathematica.

## Package Layout

```
code/grammars/
  macsyma/
    macsyma.tokens
    macsyma.grammar

code/packages/python/
  symbolic-ir/        # IR types, no dependencies
  symbolic-vm/        # Generic VM + StrictVM + SymbolicVM
  macsyma-lexer/      # Thin wrapper around GrammarLexer
  macsyma-parser/     # Thin wrapper around GrammarParser
  macsyma-compiler/   # ASTNode → IR
```

Each thin-wrapper package (`macsyma-lexer`, `macsyma-parser`) follows the
same structure as `json-lexer` and `json-parser` — a `tokenizer.py` or
`parser.py` module with a 20-line function that reads the grammar file and
calls the generic engine. The real work lives in the compiler and VM.

## Test Strategy

Every package has pytest tests with ≥80% coverage:

- `symbolic-ir`: constructor normalization (e.g., `IRRational(2, 4) → 1/2`),
  equality, hashing, structural comparison.
- `macsyma-lexer`: all token types round-trip on canonical MACSYMA snippets.
- `macsyma-parser`: grammar produces expected `ASTNode` shapes.
- `macsyma-compiler`: compiles every AST shape correctly to IR.
- `symbolic-vm`: both VMs on identity rules, arithmetic, assignment,
  function definition, and a simple `diff` rewrite.

The integration test (in `symbolic-vm/tests/test_end_to_end.py`) runs full
MACSYMA programs through the complete pipeline.

## Non-Goals (For This Spec)

- No symbolic integration (Risch algorithm is its own rabbit hole).
- No polynomial GCD or factoring.
- No simplification beyond local algebraic identities.
- No Mathematica, Maple, or REDUCE frontends yet — they reuse every
  downstream component. Adding them later means writing a new `.tokens`,
  `.grammar`, and compiler — nothing else.

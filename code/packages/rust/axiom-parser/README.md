# coding-adventures-axiom-parser

Axiom parser backed by `code/grammars/axiom/axiom.grammar`, compiled to Rust
and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

Axiom (Scratchpad II, IBM Research, 1977; commercialized as Axiom in 1992 by
Jenks & Sutor; today continued by OpenAxiom, FriCAS, and the independent
Axiom project) is the strongly-typed computer algebra system whose
category/domain type system is this repo's first symbolic-family (CAS)
language to need a per-value type tag at all
([`MA13-axiom-language.md`](../../../specs/MA13-axiom-language.md) §2). This
is the third crate of Axiom's frontend — **MA-13c** — following:

1. **MA-13a** — the design-only kickoff spec, fixing which Axiom this repo
   targets (§1), confirming `symbolic-ir`/`symbolic-vm`/`cas-*` need no
   changes for Axiom's arithmetic but carry no domain/category concept at
   all (§2), and scoping the category/domain type system to a small, fixed,
   **consumer-view-only** subset (§3) before any lexer/parser/runtime code
   landed.
2. **MA-13b** — [`axiom-lexer`](../axiom-lexer), the tokenizer.
3. **MA-13c** — `axiom-parser` (this crate), the subject of this README.

Next: **MA-13d** — `axiom-runtime` + `axiom-repl` (lowering this crate's CST
to `symbolic_ir::IRNode` and evaluating with `symbolic_vm::VM`, plus the new
`AxiomValue`/`AxiomDomain` layer MA13 §2/§3 fixes), then **MA-13e** —
`axiom-to-semantic-ir`.

## Scope

Covers the MA-13-scoped consumer-view subset fixed by
[MA13 §4](../../../specs/MA13-axiom-language.md#4-language-scope-the-historical-core):

- Literals: integers, floats (`123`, `1.5`), strings (`"hello"`), symbols
  (`x`, `foo`).
- Function calls: `f(a, b)` **and** the paren-optional single-argument form
  `f a` (`factorial 7`, `ff z`) — unified into one `postfix`/`call_args`
  production. **No dedicated rational-literal token** — `1/3` is ordinary
  `NUMBER SLASH NUMBER` division (inherited from `axiom-lexer`).
- Lists: `[a, b, c]` (square brackets, reusing the shared `List`
  symbolic-ir handler).
- Arithmetic: `+ - * /`, both power spellings `^`/`**` (the SAME operator,
  right-associative, tightest), left-associative `*`/`/`/`+`/`-`, unary `-`
  only (no unary `+`).
- Equality/comparison: `=` (lowers straight to Boolean `Equal` in this cut —
  MA13 §3's own disclosed divergence from real Axiom's default `Equation`),
  `~=` (Axiom's real not-equal spelling — **not** Maple's `<>` or Wolfram's
  `!=`), `< <= > >=`.
- `x := e` — immediate assignment (bare `NAME` left-hand side only).
- `f(x: T, ...): T == e` (declared, typed) / `f x == e` (undeclared,
  paren-optional single parameter, duck-typed) — held-body function
  definition. Two entirely separate grammar productions from `:=`, unlike
  Derive's/Reduce's own single shared assignment/definition operator — see
  `axiom.grammar`'s own `define` rule comment for why.
- `if p then e1 else e2` — `else` is **mandatory** in this cut (MA13 §4:
  "missing else — deferred").
- `( e1; e2; ...; eN )` — a parenthesised, `;`-separated block, value is the
  last expression's value. Shares ONE grammar rule (`group`) with plain
  `( ... )` grouping, distinguished by child count at a later lowering
  layer, mirroring `derive.grammar`'s own vector/matrix unification.
- `a : T`, `(a, b, c) : T` — declaration.
- `e :: T` — coercion, including the paren-optional type shorthand
  (`3 :: Fraction Integer`).
- `D has C` — category-membership query (e.g.
  `Polynomial(Integer) has Ring`).
- `( … )` — grouping.

Every one of these is `expr`-shaped and mutually nestable (an `if`-branch,
an assignment's right-hand side, and a function body are all just `expr`) —
see `axiom.grammar`'s own header comment for the full precedence cascade and
every grammar-design decision (especially the paren-optional call form's
disambiguation from a bare name followed by a binary operator, and why
`has`/`declaration` sit outside the ordinary arithmetic cascade while
`coercion` sits inside it).

**Out of scope** (MA13 §4's deferred list, unchanged from `axiom-lexer`):
user-defined categories/domains, `Join`, packages,
symbolic-domain-parameterized generic functions, `Record`/`Union`/`Any`,
`Matrix`/`SquareMatrix`/`Complex`, a genuine `Equation` domain, `macro`,
bare-variable delayed `==`, block early-exit `=>`, piecewise/multi-clause
function definitions, anonymous `+->` functions, list comprehensions,
`for`/`while` loops, package-calling `$`, target-type `@`.

## `program` parses exactly ONE top-level expression

Unlike `derive-parser`/`reduce-parser` (which parse a whole multi-statement
worksheet file, `program = { statement_line }`), `axiom.tokens` gives
top-level Axiom inputs **no separator at all** — no significant newline
(unlike Derive), no `;`/`$` (Axiom's `;` is reserved exclusively for the
parenthesised block). This is a direct, disclosed consequence of MA13
framing Axiom as a numbered, per-line interactive session (`(1) ->`,
incrementing per computation step, mirrored by a future `axiom-repl`'s own
numbered prompt, MA13 §5) rather than a batch worksheet — so `program = expr`
parses exactly what one interactive input is ever confirmed to be, leaving
"where does one REPL input end and the next begin" to that future
`axiom-repl`. See `axiom.grammar`'s own header comment for the full
reasoning.

## `:=` vs `==` — two operators where Derive/Reduce need only one

`derive-parser`'s/`reduce-parser`'s single `:=` production is shared,
unmodified, between plain variable assignment and function definition (a
later runtime disambiguates by inspecting the parsed left-hand side's
shape). That trick does not transfer here: Axiom's **declared**
function-definition form (`f(x: T, ...): T == e`) needs a parameter list of
*typed* declarations (`x: T`), a shape no ordinary call's `arglist` (a plain
comma-list of expressions) can express. So `assignment` (`:=`) and `define`
(`==`) are two entirely separate productions with two entirely separate,
narrowly-scoped confirmed left-hand-side shapes — this grammar does not
invent any combination MA13 §4 does not literally show (no untyped
parenthesised parameters, no optional return-type annotation).

## The paren-optional call form's disambiguation

`f a` (MA13's own confirmed `factorial 7`/`ff z` examples) is unified with
`f(a, b)` into one `postfix`/`call_args` production, whose paren-optional
branch matches a single bare `atom` — never a further operator-led
expression. This means `f -1`/`f +1` can **never** be misread as a call with
a negative/positive-literal argument: `atom` has no unary-minus (or `+`,
which does not exist in this cut) alternative, so `call_args` cannot start
on a `MINUS`/`PLUS` token at all, and `f - 1`/`f + 1` are unambiguously
ordinary subtraction/addition — the same resolution Haskell's own
juxtaposition-application gives its identically-overloaded `-`.

## Usage

```rust
use coding_adventures_axiom_parser::parse_axiom;

let ast = parse_axiom("power(x: Integer, n: NonNegativeInteger): Integer == x ** n");
assert_eq!(ast.rule_name, "program");
```

A parsed declared function definition's shape (rule names, not a bespoke AST
type — see below):

```text
program
└─ expr
   └─ define
      └─ declared_define
         ├─ NAME "power"
         ├─ typed_param_list
         │  ├─ typed_param (NAME "x", type_expr "Integer")
         │  └─ typed_param (NAME "n", type_expr "NonNegativeInteger")
         ├─ type_expr "Integer"                  <- the return type
         └─ expr (power's held body: x ** n)
```

`parse_axiom` panics on a malformed source string; use `try_parse_axiom` for
the `Result`-returning form, or `create_axiom_parser` directly if you need
the raw `GrammarParser`.

## No bespoke AST type — a generic `GrammarASTNode` CST, tagged by rule name

Like every grammar-driven parser in this repo, this crate does not define
its own AST enum. It returns the generic
[`GrammarASTNode`](../parser/src/grammar_parser.rs) CST, whose `rule_name`
field is what a future `axiom-runtime` (MA-13d) pattern-matches on. Type
positions (`declaration`'s target type, `coercion`'s target type, `has`'s
domain/category operands, a function header's parameter/return-type
annotations) all parse through their own dedicated `type_expr` rule —
structurally close to the ordinary `postfix`/`atom` call cascade (both are
"a NAME, optionally applied to further arguments" — Axiom's own domain
construction genuinely reuses ordinary call syntax, MA13 §3), but kept as
its own named rule so a later pass can tell "this subtree is a type
annotation" apart from "this subtree is an ordinary value-producing call"
by rule name alone, mirroring `idl-parser`'s own `index_suffix`/
`call_suffix` naming discipline for two structurally-similar-but-
semantically-distinct bracketed-list positions.

## Recursion-depth guard

`MAX_RULE_DEPTH = 140`, independently measured against this grammar's own
native-stack floor (not assumed from a sibling) across four structurally
distinct recursive shapes — see `src/lib.rs`'s own `MAX_RULE_DEPTH` doc
comment for the full measured-floor table and methodology.

## Where this fits (pipeline)

```text
Axiom source
   |
   v
axiom-lexer::tokenize_axiom        (MA-13b)
   |
   v
axiom-parser::parse_axiom          (MA-13c, this crate)
   |
   v
GrammarASTNode  <-  axiom-runtime + axiom-repl lower/evaluate this (MA-13d)
   |
   v
axiom-to-semantic-ir                (MA-13e)
```

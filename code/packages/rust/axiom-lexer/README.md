# coding-adventures-axiom-lexer

Axiom tokenizer backed by `code/grammars/axiom/axiom.tokens`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

Axiom (Scratchpad II, IBM Research, 1977; commercialized as Axiom in 1992
by Jenks & Sutor; today continued by OpenAxiom, FriCAS, and the
independent Axiom project) is the strongly-typed computer algebra system
whose category/domain type system is this repo's first symbolic-family
(CAS) language to need a per-value type tag at all
([`MA13-axiom-language.md`](../../../specs/MA13-axiom-language.md) §2).
This is the first crate of Axiom's frontend — **MA-13b** — following
MA-13a's design-only kickoff spec, which fixed which Axiom this repo
targets (§1), confirmed `symbolic-ir`/`symbolic-vm`/`cas-*` need no changes
for Axiom's arithmetic but carry no domain/category concept at all (§2),
and scoped the category/domain type system itself to a small, fixed,
**consumer-view-only** subset (§3) before any lexer/parser/runtime code
landed. The crate layout mirrors MA13 §6's rollout: `axiom-lexer` (this
crate, MA-13b) → `axiom-parser` (MA-13c) → `axiom-runtime`/`axiom-repl`
(MA-13d) → `axiom-to-semantic-ir` (MA-13e).

Axiom's surface is an ordinary infix expression grammar — closer in shape
to this repo's Reduce/Derive/Maple `head(args)`-style CAS grammars
(MA13 §5) than to any array-family grammar here — so `axiom.tokens` is
structured the same way `reduce.tokens`/`maple.tokens`/`derive.tokens`
already are, not forked from an array-family sibling.

## Scope

Covers the MA-13b-scoped lexical surface fixed by
[MA13 §4](../../../specs/MA13-axiom-language.md#4-language-scope-the-historical-core)/[§6](../../../specs/MA13-axiom-language.md#6-crate-layout-and-rollout-one-item--one-pr):

- Integer (`123`) and float (`1.5`) literals, one `NUMBER` pattern.
- **No dedicated rational-literal token.** `1/3` is ordinary integer
  division (`NUMBER SLASH NUMBER`, three tokens) — MA13 §4's own surface
  table confirms it lowers to the packed `IRNode::Rational`
  representation entirely at evaluation/lowering time, not via special
  lexer syntax.
- String literals (`"hello"`, one `STRING` token, no confirmed escape
  mechanism — `escapes: none`).
- Symbols/identifiers (`x`, `foo`, `f`) — **including every built-in
  domain/category name** (`Integer`, `Boolean`, `Ring`, `PositiveInteger`,
  ...), which are ordinary `NAME`s at this layer, not lexer-level
  keywords (see below).
- `(` `)` `[` `]` `,` — parens, list brackets, comma.
- Arithmetic: `+ - * /`, both power spellings `^` (`CARET`) and `**`
  (`POW`, kept as a distinct token type, mirroring `reduce.tokens`'s own
  `CARET`/`POW` split).
- Comparison: `=` (`EQ`), `~=` (`NE` — Axiom's real not-equal spelling,
  **not** Maple's `<>` or Wolfram's `!=`), `< <= > >=`.
- `:=` (`ASSIGN`, immediate assignment) and `==` (`DEFINE`, held-body
  function definition) — two distinct operators, unlike Reduce/Derive/
  Maple which only need one.
- `:` (`COLON`, declaration — also the type-annotation position inside a
  function header) and `::` (`COERCE`, coercion) — two of the three
  genuinely new tokens MA13 §3 introduces.
- `has` (the category-membership query infix keyword) — the third
  genuinely new token.
- `;` (`SEMI`, the separator inside a parenthesised block).
- `if`/`then`/`else` conditional keywords.
- `--` line comments (FriCAS/SPAD convention, confirmed reasonable by
  this repo's existing `--`-comment grammars — `haskell*.tokens`,
  `sql.tokens`, `vhdl*.tokens`).

This crate only tokenizes. There is no `axiom-parser`/`axiom.grammar` here
(that is a separate follow-on task, MA-13c) and no recursion-depth cap
(that is a parser-level concern for MA-13c — see the next section).

## No recursion-depth cap — the same split every sibling lexer/parser pair follows

`axiom-lexer` performs no recursive descent: [`GrammarLexer`] tokenizes
with a single left-to-right scan, one token at a time, with O(1) stack
depth regardless of how deeply nested the source is (nesting structure —
parenthesis depth, block depth — is invisible to a flat token scan; it
only becomes visible once something walks the token stream recursively,
which starts with `axiom-parser`, MA-13c). Every sibling `*-lexer` crate
in this repo that shares this shape documents the identical finding and
adds no depth cap of its own: `idl-lexer` ("no recursion-depth cap ... the
same split every sibling `*-lexer`/`*-parser` pair in this repo already
follows"), `q-lexer`, `scilab-lexer`, `apl-lexer`, `j-lexer`. A
`MAX_RULE_DEPTH`-style cap (this repo's own established convention for
*parser* recursion, per `lessons.md`) belongs to `axiom-parser`, not here.
This is verified directly, not just asserted: `tests/test_tokenizer.rs`
includes a regression tokenizing 50,000 levels of nested parens with no
stack growth (`deeply_nested_parens_do_not_overflow_the_lexer_stack`),
plus wide (non-nested) adversarial inputs — a 100,000-token flat stream and
a 500,000-character comment — to cover the other half of the DoS surface
(unbounded time/allocation, not stack depth).

## No pre/post-tokenize hooks — `axiom.tokens` is entirely declarative

Unlike `q-lexer` (whitespace-adjacency hooks for `-`-vs-negative-literal
and `/`-vs-comment) and `scilab-lexer` (a hook for `'`
transpose-vs-string), none of Axiom's MA-13b-scoped operators need one.
Every multi-character operator (`:=`, `::`, `==`, `~=`, `<=`, `>=`, `**`)
is resolved by ordinary longest-match-first declaration order in
`axiom.tokens`, and `--` comments never collide with `-` (`MINUS`) because
`GrammarLexer`'s skip-pattern pass always runs *before* ordinary token
matching at each position — the same declarative shape
`sql.tokens`/`vhdl*.tokens`/`haskell*.tokens` already rely on for their own
coexisting `MINUS`/`--`-comment pair. So `create_axiom_lexer` installs
nothing beyond the compiled grammar, mirroring `idl-lexer`'s equally
hook-free shape.

## Case-sensitivity

Axiom is case-sensitive (`case_sensitive` left at its default — no
`@case_insensitive` directive) — MA13 §4's own surface table spells
`if`/`then`/`else`/`has` lowercase, the same lowercase-keyword,
case-sensitive convention `reduce.tokens`/`maple.tokens` already use (the
mirror image of `derive.tokens`'s uppercase `AND`/`OR`/`NOT`). Built-in
domain/category names are conventionally capitalized in real Axiom
(`Integer`, `Ring`, ...) but are **not** reserved words here — see below.

## Built-in domain/category names are ordinary identifiers, not keywords

MA13 §3 fixes a small, non-extensible built-in domain table (`Boolean`,
`Integer`, `PositiveInteger`, `NonNegativeInteger`, `Float`, `String`,
`Fraction(Integer)`, `Polynomial(Integer)`, `List(T)`) and category table
(`Ring`, `OrderedSet`) as an `axiom-runtime`-internal lookup structure
(MA-13d) — entirely invisible to this crate. `axiom-lexer` tokenizes every
one of these names as an ordinary `NAME`, exactly as `idl-lexer` resolves
IDL's intrinsic procedure/function names (`PLOT`, `SIN`, `TOTAL`, ...) as
plain `NAME`s rather than lexer-level keywords. Only four words are real
keywords in this cut: `if`, `then`, `else`, `has`.

## What this crate deliberately does NOT tokenize (MA13 §4's deferred list)

No token exists anywhere in `axiom.tokens` for: `Record`/`Union`/`Any`,
`macro`, package-calling `$`, target-type `@`, the anonymous "maps-to"
function operator `+->`, block early-exit `=>`, piecewise/multi-clause
function definitions, list comprehensions, or `for`/`while` iteration —
all explicitly out of MA13's first-cut scope (§3/§4). A source construct
using `$` or `@` fails honestly (neither character has any token in this
grammar); `+->` and `=>` decompose into their individual already-tokenizable
characters (`PLUS`/`MINUS`/`GREATER`, `EQ`/`GREATER`) since this cut has no
*dedicated* multi-character token for either sequence — an honest
reflection of the absence of a production, left for a future
`axiom-parser` to reject at the grammar level, not a special-cased lexer
rejection.

## Usage

```rust
use coding_adventures_axiom_lexer::tokenize_axiom;

let tokens = tokenize_axiom("a : PositiveInteger\na := 3\nPolynomial(Integer) has Ring\n");
```

`tokenize_axiom` panics on a malformed source string; use
`create_axiom_lexer` directly (or `try_tokenize_axiom`) if you need the
`Result`-returning form instead.

## Where this fits (pipeline)

`axiom-lexer` is the first of Axiom's frontend crates
([MA-13b](../../../specs/MA13-axiom-language.md#6-crate-layout-and-rollout-one-item--one-pr)),
following MA-13a's design spec. The sibling `axiom-parser` crate (MA-13c)
will consume this crate's token stream against
`code/grammars/axiom/axiom.grammar` — including the declaration (`name :
T`), coercion (`e :: T`), and category-query (`D has C`) productions, the
genuinely new grammar work MA13 §3 scopes — to build the `GrammarASTNode`
CST that a future `axiom-runtime` + `axiom-repl` (MA-13d) will evaluate
over the reused `symbolic-vm` engine, alongside `axiom-to-semantic-ir`
(MA-13e), per
[HML00](../../../specs/HML00-historical-math-languages-roadmap.md) Wave 7.

[`GrammarLexer`]: ../lexer/src/grammar_lexer.rs

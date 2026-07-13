# coding-adventures-j-parser

J parser backed by `code/grammars/j/j.grammar`, compiled to Rust and
statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

Consumes the token stream from `j-lexer` (MA-6b) and drives it through
`j.grammar`'s two-nonterminal design (`noun_expr`/`verb_expr`, see
[MA06 §3](../../../specs/MA06-j-language.md)) to produce a `GrammarASTNode`
CST rooted at `program` — the second of the two frontend crates for J
(MA-6c); a future `j-runtime` crate (MA-6d) will walk this tree to evaluate.

## The two-nonterminal design, reused from APL

J's grammar reuses `apl.grammar`'s exact two-nonterminal shape (MA06 §3),
with terminology renamed to J's own vocabulary:

- **`noun_expr`** — arrays and scalars (APL's `value_expr`). Built from
  `term`s combined by `verb_expr`s, one precedence tier, right-to-left
  (`a F b G c` parses as `a F (b G c)`, unchanged from APL).
- **`verb_expr`** — a primitive glyph, a primitive glyph with an adverb
  applied (a "derived verb", e.g. `+/`), two verbs joined by the `@`
  conjunction (compose), or the one production with no APL precedent: a
  parenthesised **train**.

Exactly like `apl.grammar`, whether a `verb_expr` application is monadic or
dyadic is a runtime concern the grammar doesn't decide — it's read off
which of `noun_expr`'s two alternatives matched (a `term` before the
`verb_expr`, or none).

## Trains — the one genuinely new production

A **train** is a flat run of 2+ `train_tooth`s (a bare/adverbed primitive
verb, an `@`-composed verb, a parenthesised sub-verb or sub-train, or a bare
noun — only meaningful in a fork's leading position) with no application
between them:

- **Hook** (exactly 2 teeth): `(f g) x` means `x f (g x)`.
- **Fork** (3+ teeth, right-to-left): `(f g h) x` means `(f x) g (h x)`,
  and `(a b c d)` reduces to the fork `(a (b c d))`.

This grammar builds the flat *shape* only; deciding whether a given shape
means a hook or a fork (and rejecting a bare-noun tooth outside a fork's
leading position) is left to a later lowering pass, exactly the same
division of labour `apl.grammar`'s own numeric stranding already uses (the
grammar doesn't know three juxtaposed numbers form a vector any more than it
knows three juxtaposed verbs form a fork).

**Trains are parenthesised-only in this cut** (MA06 §3): an unparenthesised
top-level train would be ambiguous with an ordinary application chain
without deeper lookahead than this grammar needs, so `verb_train` only
appears inside `LPAREN verb_train RPAREN`. One consequence worth knowing: a
lone `@`-composed verb (e.g. `+@-`) does *not* need parens at all (it's
already a complete `verb_expr` wherever one is expected), and — because
`LPAREN verb_train RPAREN` requires **2 or more** teeth — wrapping a single
composed verb in parens, e.g. `(+@-)`, is **not** valid syntax under this
grammar: `+@-` greedily parses as one whole tooth via `verb_expr`'s own
`AT`-alternative, leaving nothing for the required second tooth.

## Usage

```rust
use coding_adventures_j_parser::try_parse_j;

let tree = try_parse_j("A=.i.5\nB=.+/A\n")?;
assert_eq!(tree.rule_name, "program");
```

`parse_j`/`create_j_parser` panic on a lexical or syntax error; `try_parse_j`
returns a `Result` instead.

## Recursion-depth guard

`create_j_parser` opts the shared `GrammarParser` into a recursion-depth cap
(`MAX_RULE_DEPTH = 70`), empirically derived via the same binary-search
methodology `apl-parser` used for its own (twice-corrected) cap — measured
fresh for this grammar, not copied from `apl-parser`.

`j.grammar` inherits `apl.grammar`'s two ways to recurse `noun_expr`
arbitrarily deep (parenthesised nesting, and a flat unparenthesised dyadic
chain), plus one genuinely new J-only way: a long parenthesised **train**
(`verb_train`'s own flat `train_tooth { train_tooth }` repetition). All
three were measured independently (binary search, on a `std::thread::spawn`
worker with the **default ~2 MiB stack**, in a **debug** build to match how
`cargo test` actually runs):

| Shape                              | Safe (default stack) | Crashes at |
|-------------------------------------|----------------------|------------|
| Parenthesised nesting `((((5))))`   | 100 levels           | 101        |
| Flat dyadic chain `1+1+1+…+1`       | 135 terms            | 136        |
| Long train `(+ + + … +) 5`          | 200 teeth            | 201        |

**Parenthesised nesting is the binding (lowest) floor here** — the opposite
ranking from `apl-parser`'s own finding, where the flat chain was binding.
This is exactly the outcome MA06 §6 warns about: a shape's floor from a
sibling crate (or even this grammar's own other shapes) does not predict
another shape's floor; each needs its own measurement. See `MAX_RULE_DEPTH`'s
doc comment in `src/lib.rs` for the full derivation, and `CHANGELOG.md` for
the measurement methodology.

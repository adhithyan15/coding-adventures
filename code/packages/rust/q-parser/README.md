# coding-adventures-q-parser

Q (kdb+'s scripting language) parser backed by `code/grammars/q/q.grammar`,
compiled to Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

Consumes the token stream from `q-lexer` (MA-11b) and drives it through
`q.grammar`'s two-nonterminal design (`noun_expr`/`verb_expr`, see
[MA11 §3](../../../specs/MA11-q-language.md)) to produce a `GrammarASTNode`
CST rooted at `program` — the second of Q's frontend crates (MA-11c). A
future `q-runtime` crate (MA-11d) will walk this tree to evaluate, alongside
`q-repl` (the `q` binary) and `q-to-semantic-ir` (MA-11e), per
[HML00](../../../specs/HML00-historical-math-languages-roadmap.md) Wave 6.

This crate only parses — it does not evaluate anything. There is no `QFn`
representation, no environment/scope model, and no lowering here; those are
`q-runtime`'s job (MA-11d).

## The two-nonterminal design, reused from APL/J

Q's grammar reuses `apl.grammar`/`j.grammar`'s exact two-nonterminal shape
for primitive-verb application (MA11 §3: "reused UNCHANGED... this is the
easy, mechanical part"):

- **`noun_expr`** — arrays and scalars. Built from `term`s combined by
  `verb_expr`s, one precedence tier, right-to-left (`a F b G c` parses as
  `a F (b G c)`, unchanged from APL/J).
- **`verb_expr`** — a primitive glyph, optionally with one adverb applied
  (a "derived verb", e.g. `+/`).

Q has no trains and no `@` compose in this cut (MA11 §4 defers `@`; trains
are a J/K-internal device Q's own real grammar never had), so `verb_expr`
here is flatter than `j.grammar`'s own (no `verb_train`, no
`AT`-continuation).

## Function literals — the headline novelty (MA11 §2 / §3 bullet 1)

`{[x;y] stmt; stmt; ...}`: an optional bracketed, semicolon-separated
parameter list (defaulting to the implicit `x`/`y`/`z` names when the
brackets are omitted entirely — a runtime/lowering convention, not
something this grammar encodes) followed by a semicolon-separated statement
sequence, the last statement's value being the result. A function literal
is itself an ordinary noun value: assignable (`f:{x+y}`) and passable,
without being applied.

### Calling a function value — the one genuinely hard grammar problem here

MA11 §3 bullet 1 is explicit that a function literal is "applied with the
same juxtaposition/`@` mechanism as a primitive verb — no new *application*
production, only a new way to *produce* a callable value." Taken literally,
this means `verb_expr` gains two new alternatives — a bare `NAME` (a
previously-assigned function value) and an inline `function_literal` — so
the *existing* `noun_expr` production already parses a call once something
callable can occupy the `verb_expr` slot.

The catch: `NAME` and `function_literal` are *also* valid `term` (noun-side)
alternatives — they have to be, so an un-applied function value can still
be assigned. APL/J never had this overlap (their `term`/`verb_expr` token
sets were always completely disjoint — NUMBER/NAME/LPAREN vs. a fixed set
of primitive-glyph tokens), so they never needed to solve it. Two
consequences, both deliberate and documented in `q.grammar`'s own header
comment:

- `noun_expr`'s optional dyadic continuation widens from `[ verb_expr
  noun_expr ]` (APL/J's shape, unchanged for the primitive case) to
  `[ verb_expr noun_expr | noun_expr ]` — one more inner alternative, not a
  new named production — so that `f 5` (nothing before `f`, calling it
  monadically) parses: `term` always wins the first alternative's leading
  element when the input starts with a NAME, but once `f` is consumed,
  checking whether an entire independent `noun_expr` immediately follows
  with nothing in between is exactly the shape that means "apply."
- **A disclosed, deliberate limitation:** real K/Q resolves whether a bare
  NAME plays a noun role or a verb (callable) role in a chain of three or
  more juxtaposed names (`f g h`) by tracking each name's already-defined
  arity during parsing — a symbol table this repo's shared,
  context-free `GrammarParser` has no mechanism for at all (every other
  frontend in this family resolves monadic/dyadic dispatch purely from
  grammar shape, never from what a name was previously bound to). This
  grammar's own resolution — the inner alternative `verb_expr noun_expr` is
  tried before the bare `noun_expr` fallback — means `x f y` (exactly one
  middle "verb" name) parses as the dyadic call `f(x, y)`, but a longer
  chain right of that point recurses through the same rule without any
  attempt at global lookahead. This is a best-effort, *not* a faithful
  reproduction of true K/Q valence resolution — the same class of disclosed
  simplification `j.grammar`'s own header comment accepts for a bare-noun
  train tooth outside a fork's leading position. `q-runtime` (MA-11d) needs
  to be aware of this when it walks the tree.

Verified directly (see this crate's own tests): `f 5` (monadic named call),
`2 f 3` (dyadic named call), `{x*2} 5` and `2 {x+y} 3` (inline-lambda calls,
both arities), and `f:{x+y}` (assigning a lambda without calling it) all
parse correctly and distinctly from each other.

## Dual list-literal syntax (MA11 §3 bullet 3)

Two syntaxes lower to the same list value, and this grammar has two
distinct `term` alternatives for them (mirroring how MA07 §3 gave Derive's
`[a, b, c]` vector literal its own production alongside ordinary
arithmetic):

- **Adjacent numeric stranding** (`1 2 3`), reused unchanged from APL/J's
  own `NUMBER { NUMBER }` — no new production needed.
- **Explicit `(a; b; c)`** (`list_literal`), for lists of arbitrary
  expressions — including function literals or mixed types — that
  stranding cannot express.

The explicit form is disambiguated from ordinary parenthesised grouping
purely by the presence of a top-level `;` — and needs **no explicit
lookahead of its own** to make that work. `term`'s plain
`LPAREN noun_expr RPAREN` alternative is tried first; for `(2;3)` it simply
**fails** (`noun_expr` can only consume up to the first element, then the
mandatory `RPAREN` check finds a `SEMICOLON` instead), and the packrat
parser's Alternation backtracks cleanly to `list_literal`, which matches.
For plain `(2+3)`, the first alternative already succeeds outright, so
`list_literal` is never even attempted.

## Usage

```rust
use coding_adventures_q_parser::{parse_q, try_parse_q};

let tree = try_parse_q("f:{[x;y] x+y}\nf 2 3\n")?;
assert_eq!(tree.rule_name, "program");
# Ok::<(), String>(())
```

`parse_q`/`create_q_parser` panic on a lexical or syntax error; `try_parse_q`
returns a `Result` instead.

## Recursion-depth guard

`create_q_parser` opts the shared `GrammarParser` into a recursion-depth cap
(`MAX_RULE_DEPTH = 32`), empirically derived via the same binary-search
methodology `apl-parser`/`j-parser` used for their own caps — measured fresh
for **three** distinct recursion shapes this grammar can drive deep, per
MA11 §6's own instruction (a throwaway subprocess per data point, on a
`std::thread::spawn` worker with the **default ~2 MiB stack**, an *uncapped*
`GrammarParser`, in a **debug** build to match how `cargo test` actually
runs):

| Shape                                       | Safe (default stack) | Crashes at |
|----------------------------------------------|-----------------------|------------|
| Parenthesised nesting `((((5))))`             | 101 levels            | 102        |
| Flat dyadic chain `1+1+1+…+1`                 | 115 terms             | 116        |
| Nested function-literal bodies `{{{5}}}`      | 45 levels             | 46         |

**Nested function-literal bodies are the binding (lowest) floor** — the
genuinely new shape MA11 §6 flagged as needing its own fresh measurement
("no sibling crate has measured this shape before"). It costs the most
native-stack per level of the three: each additional `{`/`}` layer recurses
through `function_literal -> stmt_seq -> statement -> assignment ->
noun_expr -> term -> function_literal`, six named-rule hops, versus
parenthesised nesting's two (`term -> noun_expr`) — a function literal's
body is a full statement sequence, not a bare `noun_expr` the way a
parenthesised group's contents are.

`MAX_RULE_DEPTH` is set to **32** — about 29% below the binding
nested-function-literal floor of 45 (comparable margin to `apl-parser`'s own
~26.5% and `j-parser`'s own ~30%), and therefore safely below the other two
floors (101, 115) as well. Measured real-input headroom at `32` (using the
*capped* parser, so no crash risk at all): parenthesised nesting parses
cleanly to 13 levels (14 trips), a flat chain to 26 terms (27 trips), and
nested function literals to 4 levels (5 trips). The function-literal
headroom is modest in absolute terms, but not a practical limitation: MA11
§4 puts nested function-literal *definitions* out of this cut's semantic
scope entirely (no closure/scoping model is specified for them), so no
legitimate program in this subset needs more than one level of `{...}`
nesting — the cap exists to reject a *pathologically crafted* deep input
cleanly, not to bound realistic programs. See `MAX_RULE_DEPTH`'s doc comment
in `src/lib.rs` for the full derivation, and `CHANGELOG.md` for the
measurement methodology.

## Known limitations (disclosed, not fixed here)

- **Three-or-more bare-name juxtaposition chains** (`f g h`) do not
  replicate real K/Q's arity-tracking valence resolution — see "Calling a
  function value" above. A future `q-runtime` needs to account for this.
- **`q-lexer` bug/gap discipline**: this crate consumes `q-lexer`'s token
  stream as-is and does not modify it. No genuine bug was found in
  `q-lexer` while building this crate; if one is found later, per this
  repo's "fix what's local, defer what's shared" convention it belongs in a
  separate change to `q-lexer` itself, not folded into this crate's own PR.

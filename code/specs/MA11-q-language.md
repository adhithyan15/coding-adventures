# MA11 — Q: kdb+'s scripting language, Arthur Whitney's array family made readable

## Status

Active spec. Wave 6 of the historical math-languages roadmap
([`HML00`](HML00-historical-math-languages-roadmap.md) §7) lists this item as
"K/Q" — the next array-family language after J ([`MA06`](MA06-j-language.md))
and Scilab ([`MA10`](MA10-scilab-language.md)). This is **MA-11a**, the design
item (mirroring MA05/MA06's own "-a" kickoff pattern): it fixes which of the
two related languages this repo targets first (§1), checks the substrate
(§2), and fixes the one genuinely new grammar/evaluator problem — user-defined
function literals — before any lexer/parser/runtime code lands (§3).

## §1 Why this spec picks Q, not raw K, as Wave 6's "K/Q" item

HML00 §7 names this wave's item "K/Q" as if it were one language, the same
shorthand it uses for "Octave" riding on MATLAB's grammar or "Maxima" riding
on Macsyma's. Closer inspection (the same kind of check MA06 §1 and MA07 §1
each did before accepting their wave's one-line description at face value)
shows that shorthand is misleading here in a way it wasn't for
Octave/Maxima:

- **K and Q share one execution model but not one surface grammar.** K
  (Arthur Whitney, 1993, Kx Systems) is a direct, terser descendant of APL/J
  in this repo's own family tree — single-character primitives, no reserved
  words, right-to-left evaluation, no operator precedence. Q (Kx Systems,
  2003) is a *second, deliberately more readable surface syntax* layered on
  the same underlying k-tree/vector engine that ships inside kdb+: it adds
  long-form keyword primitives (`til`, `where`, `count`, `select`, `update`)
  alongside the symbolic ones, an explicit function-literal syntax
  (`{[x;y] …}`), and (out of this cut's scope, see §4) a first-class table
  type with SQL-flavored query syntax. This is Wolfram-vs-Maxima's
  relationship, not Octave-vs-MATLAB's: Q is a **real second frontend**,
  not a thin reuse of K's grammar the way Octave's `octave-to-semantic-ir`
  reuses `matlab-to-semantic-ir` wholesale (per [`MA05`](MA05-apl-language.md)'s
  own "octave-to-semantic-ir thin wrapper" precedent for that *other*
  relationship).
- **K itself fragments across incompatible dialect versions (K2/K3/K4/K6/K7)
  with real, documented semantic drift in its primitive table** — the
  opposite of J, which has one stable canonical reference (the *J
  Dictionary*) MA06 could cite directly. Q, by contrast, has been
  syntactically stable since kdb+ 2.0 (c. 2004) and has a single canonical,
  freely available reference: Borror, *Q for Mortals* (kx.com), plus Kx's
  own `code.kx.com/q/ref/` reference card. Per this repo's "verified against
  the actual documented behavior of a specific named version" discipline
  (the same discipline MA07 §3/§4 used for Derive 6.1 and MA06 §4 used for
  J's *Dictionary*), Q is the version of this language family this repo can
  responsibly commit specific primitive-glyph semantics to; raw K is not,
  without picking one specific K version to pin to (a decision this spec
  defers rather than guesses at).
- **Q is also the actively-used member of the pair** — kdb+/q remains in
  production use (time-series/financial data) and is what "K" means in
  practice for the vast majority of people who encounter this language
  family today; raw K is largely an internal/legacy dialect inside Kx.

**Decision:** this spec, and the items under it, target **Q**. Raw K (one
specific pinned dialect, likely K3 or K4 as the best-documented public
versions) remains a possible later follow-on — structurally the same
"thin second frontend over an already-built engine" relationship Octave has
to MATLAB and Maxima has to Macsyma, once Q's own runtime exists to be
reused. This spec's filename and every item under it says `q`, not `kq`, to
avoid re-litigating this decision implicitly the way an ambiguous name
would.

## §2 Substrate check: `array-runtime` unchanged; the one genuinely new *evaluator* concern

Checked directly against the current `array-runtime` public API
(`value::Array`, `ops::{elementwise, reduce, scan, outer, matmul,
transpose}`) and `j-runtime`'s/`apl-runtime`'s own internal function
representations (`JFn`/`AplFn`, in `j-runtime/src/eval.rs` and
`apl-runtime/src/eval.rs`):

- **The value model needs nothing new.** Q's in-scope core (§4) is dense
  numeric vectors and scalars — precisely `array-runtime::Array`, the same
  value model APL/J/Scilab's array-family cuts already share. Q's
  elementwise/reduce/scan semantics (`+`, `-`, `*`, `%`, `&`, `|`, `,`, `#`,
  `_`, adverbs `'`/`/`/`\`) map directly onto the `BinOp`/`reduce`/`scan`
  kernels AR-2 already generalized for APL and J reused unchanged (MA06
  §2) — **no `array-runtime` substrate work is needed**, the same "zero new
  substrate" finding MA06 §2 reached for J.
- **The one genuine novelty is confined to `q-runtime`'s own evaluator, not
  a shared crate.** Every array-family runtime in this repo so far
  (`apl-runtime`'s `AplFn`, `j-runtime`'s `JFn`) represents a "function
  value" as a closed set of variants built *only* from existing primitives:
  a bare primitive, or a composition/hook/fork of other such values. None of
  them has ever needed to represent a genuine **user-defined function
  literal** — named parameters, a multi-statement body, lexical capture of
  enclosing local bindings — because APL's and J's in-scope grammars (this
  repo's own cuts of them) are expression-only: trains *recombine* existing
  primitives, they never introduce a brand-new parameter name a body can
  reference. Q's `{[x;y] …}` function literal (§3, §4) is exactly that new
  thing. This is real, novel *evaluator* design — the direct analogue of
  how MA06 §3 had to fix hook/fork's evaluation shape before `j-runtime`
  could be written — but it is `q-runtime`-internal work, parallel to how
  `JFn` already lives in `j-runtime` rather than in `array-runtime` itself.
  **No shared crate needs to change**; `q-runtime` simply needs its own
  `QFn` enum with a `Lambda { params: Vec<String>, body: Vec<GrammarASTNode>
  }`-shaped variant (or the post-parse equivalent) alongside `Atom`/
  `Reduce`/`Scan`, plus a small environment/scope-frame concept for
  evaluating a call (bind arguments to parameter names, evaluate the body
  statements in order, return the last statement's value) that
  `apl-runtime`/`j-runtime` have never needed.

## §3 Grammar design: reused shape, three genuine novelties

Per [`feedback_no_handwritten_lexers_parsers`], `q-lexer`/`q-parser` wrap the
shared `GrammarLexer`/`GrammarParser`, exactly as every other frontend in
this family. Q's grammar reuses APL/J's noun/verb split and right-to-left,
single-tier evaluation **unchanged** — Q inherits "no operator precedence,
apply right-to-left" from K exactly as J inherited it from APL, so
`noun_expr`'s right-recursive dyadic continuation (MA05 §3, reused unchanged
by MA06 §3) transfers a third time with no redesign. Three things are
genuinely new and must be fixed here before `q.tokens`/`q.grammar` land:

1. **Function literals — the headline novelty (§2).** `{[x;y] stmt; stmt;
   …}` is a new `noun_expr` production: an optional bracketed,
   semicolon-separated parameter list (defaulting to the implicit `x`/`y`/
   `z` names when omitted — a real, well-documented Q convenience this
   subset **does** include, since omitting it would make trivial one-line
   lambdas artificially verbose) followed by a semicolon-separated sequence
   of statements, the last one's value being the call's result. A function
   literal is itself an ordinary noun value (assignable, passable), applied
   with the same juxtaposition/`@` mechanism as a primitive verb — no new
   *application* production, only a new way to *produce* a callable value.
2. **Whitespace-sensitive tokenization for two unrelated ambiguities**,
   both real, both documented, and both needing the lexer's
   longest-match/lookahead discipline (the same discipline MA06 §3's final
   bullet already established for J's digraphs) applied to a *spacing*
   signal instead of a character signal:
   - **Negative-literal vs. subtraction.** Unlike J (which spells a negative
     literal with a leading underscore specifically to avoid this
     ambiguity, MA06 §4), Q spells a negative number with an ordinary
     leading `-` and disambiguates by whitespace: `2 -1` (space before the
     minus, none after) tokenizes as the two-element list `2, -1`; `2 - 1`
     (space on both sides) tokenizes as subtraction of `1` from `2`. This
     subset's lexer implements this the same way Q's own does: a `-`
     immediately followed by a digit with no intervening space, at a
     position where a new list-stranding element may start, is folded into
     a signed-numeric-literal token rather than emitted as the standalone
     subtract verb.
   - **`/` is a trailing/leading comment marker *or* the reduce/over
     adverb**, disambiguated purely by surrounding whitespace: a `/` at the
     start of a line, or preceded by whitespace with nothing but
     whitespace following it before the next token boundary, opens a
     comment to end of line; a `/` immediately following a verb/noun with
     no preceding space is the reduce adverb (`+/x`). This is a genuine,
     well-documented Q lexer wrinkle (parallel in spirit, though not in
     mechanism, to MA06 §4's `.`-as-decimal-point-vs-primitive-suffix
     callout for J) that `q.tokens` must encode explicitly rather than
     assume the grammar-tools default skip-pattern handles for free.
3. **List literals have two syntaxes, not one**, and this subset needs
   both: adjacent numeric-literal stranding (`1 2 3`, reused unchanged from
   APL/J's `noun_expr`) for simple numeric vectors, **and** an explicit
   parenthesised, semicolon-separated form (`(a; b; c)`) for lists of
   arbitrary expressions (including function literals or mixed types) that
   stranding cannot express. Both lower to the same list value; the parser
   needs two distinct `noun_expr` alternatives (mirroring how MA07 §3 gave
   Derive's `[a, b, c]` vector literal its own production alongside ordinary
   arithmetic, rather than trying to unify it with juxtaposition).

Everything else about application — monadic-vs-dyadic dispatch resolved by
which production matched, not by the lexer (MA05 §3 bullet 3, reused
unchanged by MA06 §3 and again here) — transfers with no changes.

## §4 Language scope (the historical core)

In scope for the first cut — a faithful "textbook q session" subset,
following the same honesty-about-subsets convention as every language here:

- **Arrays only, dense and numeric** — identical value model to APL/J/
  Scilab: a scalar is a rank-0 array, built on `array-runtime::Array`.
  Q's own boolean atom type (`0b`/`1b`) and rich temporal-literal suffix
  family (`2020.01.01`, `12:00:00.000`, `0Nj`, …) are **not** in this cut;
  comparisons and logic produce/accept plain `0`/`1` numerics, matching
  this repo's own APL/J convention, and this subset has no date/time
  literals at all.
- **Primitive verbs** (monadic / dyadic meaning, ASCII spelling, Q's actual
  names in comments below since — unlike APL/J — Q's own documentation
  itself calls these by name): `+` add / flip, `-` negate / subtract, `*`
  first / multiply, `%` reciprocal / divide (always true/float division,
  matching Q's own real behavior — no integer-division special case), `!`
  til (monadic index generator, **0-based**, matching J's own `i.`
  convention rather than APL's 1-based `⍳`) — **dyadic `!` (dict creation,
  and its other real overloads) is deferred**, see below, `,` enlist /
  join, `#` count / take, `_` floor / drop, `&` where (monadic — returns
  the indices of nonzero elements, one of Q's most idiomatic primitives) /
  min (dyadic), `|` reverse (monadic) / max (dyadic), `~` not (monadic) /
  match (dyadic, deep equality), `=` `<` `>` `<=` `>=` `<>` comparison
  (dyadic; Q spells not-equal `<>`, not `~=` or `#`).
- **Adverbs**: `'` (each), `/` (over/reduce), `\` (scan) — the same three
  AR-2 primitives APL/J already reuse, spelled identically (each-prior
  `':`, each-right `/:`, each-left `\:` are real Q adverbs, explicitly
  deferred to keep this first cut to the three already proven out for
  APL/J).
- **Function literals**: `{[x;y] …}` and the bracket-omitted implicit
  `x`/`y`/`z` form (§3 bullet 1). Recursion (a function referring to its
  own name, or Q's `.z.s`/anonymous-self-reference idiom) is **out** of
  this first cut — every function body in scope calls only primitives and
  other already-defined functions, not itself.
- **List literals**: adjacent-numeric stranding and explicit `(a; b; c)`
  (§3 bullet 3).
- **Assignment**: `name:expr` at the top level and inside a function body
  (local to that call) — Q's real global-vs-local scoping subtlety across
  *nested* function definitions (and the explicit `::` global-assignment
  operator used to reach outward past a local scope) is **out**, matching
  how this subset has no nested function literals to begin with.
- **Comments**: `/` at line-start or after a preceding-whitespace gap, to
  end of line (§3 bullet 2).
- **Parenthesised grouping** `( … )`, disambiguated from the list-literal
  form by the presence/absence of a top-level `;`.

**Deferred (post-MA-11a), each a follow-on item exactly as APL/J deferred
their own harder extras:**

- **Symbols** (`` `abc ``) and **strings** (`"abc"`) as atomic values — Q's
  actual mechanism for naming table columns and building the table/query
  half below; deferred alongside it since this cut's numeric-only value
  model has no use for them yet.
- **Dictionaries** (`key!value`) **and tables** (the flagship Q/kdb+
  feature — `flip`ped column-dictionaries, `select`/`update`/`delete`/
  `from`/`by`/`insert`/`upsert` q-SQL) — by far the largest deferred
  surface, matching how Wolfram's `cas-*` built-in surface grew across many
  W-6 through W-22 follow-on items rather than landing with the kickoff.
  This is genuinely Q's reason for existing over plain K, so it is real,
  substantial follow-on work, not a corner being permanently cut.
- **`?` and `.` (both heavily overloaded across distinct/random/find and
  apply/value/namespace-path meanings respectively)** — deferred whole,
  rather than committing to a partial reading of either.
- **`@` (apply-at/index/type)** — Q's `@` is a more general
  apply/index/type-introspection primitive than J's compose-only `@`
  (§1's own point about K/Q sharing an execution model but not always the
  same primitive meaning per glyph); deferred rather than assumed to
  transfer from J's narrower meaning unchanged.
- **Each-prior/each-right/each-left adverbs** (`':`/`/:`/`\:`), recursion
  inside function literals, nested nested-nested function definitions and
  the associated global/local scoping subtlety, boolean/temporal/symbol
  atom types and their literal suffixes.

## §5 Reuse strategy

- **Lexer/parser**: the `grammar-tools` frontend, exactly as APL/J/Scilab
  — `code/grammars/q/q.tokens` + `q.grammar` compile to committed
  `_grammar.rs` in `q-lexer`/`q-parser`. The noun/verb split and
  right-to-left `noun_expr` continuation reuse APL/J's shape verbatim
  (renamed nonterminals); the function-literal production and the two
  list-literal alternatives (§3) are the genuinely new grammar rules. The
  whitespace-sensitive negative-literal and comment/reduce disambiguations
  (§3 bullet 2) are lexer-level, resolved with lookahead over the
  surrounding characters at tokenization time, the same place J's own
  digraph lookahead already lives.
- **Runtime**: `q-runtime` walks the parse tree over `array-runtime::Array`
  values, lowering `+/`/`+\` through the same AR-2 kernels APL/J already
  call. Its own `QFn` enum (§2) generalizes `JFn`'s shape with one new
  variant for a user-defined lambda (captured parameter names, a body
  statement sequence, and the enclosing call's local-binding environment at
  the point of definition) — the one piece of genuinely new evaluator
  design this spec fixes (§2/§3).
- **REPL & binary**: `q-repl` + a `q` binary, mirroring `j-repl`/
  `apl-repl`'s continuation-scanning shape (paren/bracket-balance
  tracking, `/`-comment stripped at the lexer's skip-pattern level so the
  REPL scanner itself needs no special comment handling, mirroring how
  `apl-repl` already handles `⍝` and `j-repl` already handles `NB.`).
- Per [`HML01`](HML01-math-to-semantic-ir.md) §2's amended per-language
  pattern, `q-to-semantic-ir` is built **alongside** the runtime in this
  same wave, not bolted on afterward — mirroring APL's/J's/Scilab's own
  precedent. It lowers the numeric-array subset onto
  [`SIR22`](SIR22-array-matrix-semantic-ir.md)'s domain, reusing whatever
  `Expr` variants APL/J's own `-to-semantic-ir` crates already added for
  reduce/scan; a function literal lowers to a `Closure`-shaped SIR node if
  one already exists from another frontend's needs, or is tracked as its
  own small SIR addendum item if not — a decision for that item, not this
  one, since it depends on what the shared IR already has by the time
  `q-to-semantic-ir` starts.

  **Resolved by MA-11e itself**: `semantic-ir`'s core (not a SIR22
  addendum at all — this need predates Q, from the general-purpose-language
  frontends) already had everything required: an ordinary
  `semantic_ir::Function` with named parameters, `Expr::DirectCall` (a
  statically-known callee), `Expr::MakeClosure` (a bare reference to a
  named function used as a value), and `Expr::IndirectCall` (a call through
  a value whose identity is not statically known). No SIR addendum item was
  needed at all. A function literal does **not** lower to a single
  `Closure`-shaped node the way this section's own phrasing speculated —
  each one becomes its own genuine top-level `Function` (this is closer to
  Python's/Ruby's own lambda-lifting than to a single first-class-closure
  value node), simplified considerably by the fact that Q's own function
  values capture nothing at all (§2's own finding): no free-variable
  analysis, no capture list, ever. See `q-to-semantic-ir/src/lower.rs`'s
  module doc comment for the full design.

## §6 Crate layout and rollout (one item = one PR)

```
q-lexer/      src/{lib.rs, _grammar.rs}   ← MA-11b (+ code/grammars/q/q.tokens)
q-parser/     src/{lib.rs, _grammar.rs}   ← MA-11c (+ code/grammars/q/q.grammar)
q-runtime/    src/{lib.rs, eval.rs, value.rs, builtins.rs}   ← MA-11d
q-repl/       src/{lib.rs, main.rs}       ← MA-11d (the `q` binary)
```

- **MA-11a — this spec.** The K-vs-Q scope decision (§1), the substrate
  check finding no `array-runtime` change is needed (§2), and the three
  genuine grammar/evaluator novelties — function literals, two
  whitespace-sensitive lexer disambiguations, and dual list-literal syntax
  — fixed before any lexer/parser/runtime code lands (§3).
- **MA-11b — `q.tokens`/`q.grammar`**: the noun/verb grammar (§3), reusing
  APL/J's shape with renamed nonterminals, plus the function-literal and
  dual-list-literal productions. Should ship with a recursion-depth cap
  from day one, measuring the actual native-stack floor for every distinct
  way this grammar recurses deeply — parenthesised nesting, a flat
  right-recursive dyadic chain, **and**, new to this grammar, nested
  function-literal bodies — per `apl-parser`'s/`j-parser`'s own
  (twice-corrected, per their `CHANGELOG.md`s) measure-don't-assume
  methodology.
- **MA-11c — `q-parser`**: `create_q_parser`/`parse_q`/`try_parse_q`,
  mirroring `j-parser`'s shape, producing a `GrammarASTNode` CST rooted at
  `program`.
- **MA-11d — `q-runtime` + `q-repl` + the `q` binary.** A working REPL:
  right-to-left evaluation, the §4 primitive set, `!`/`#`/`_` array
  construction, reduce/scan/each lowered onto AR-2, and function-literal
  definition/call (§2/§3's `QFn::Lambda`).
- **MA-11e — `q-to-semantic-ir`**, per [`HML01`](HML01-math-to-semantic-ir.md)
  §2 — built in this same wave rather than as a later retrofit, per §5.
- **Next**: IDL, per [`HML00`](HML00-historical-math-languages-roadmap.md)
  Wave 6's remaining member, once Q's own kickoff has landed — and,
  independently, a possible later K item reusing `q-runtime`'s engine the
  way Octave reuses MATLAB's (§1), once there is an engine to reuse.

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (§5 survey,
§7 Wave 6), [`HML01`](HML01-math-to-semantic-ir.md) (the `-to-semantic-ir`
standing convention this spec adopts from the start), [`MA00`](MA00-array-runtime.md)
(the substrate — unchanged, per §2), [`MA05`](MA05-apl-language.md)/
[`MA06`](MA06-j-language.md) (the direct structural ancestors of this spec —
the noun/verb split, right-to-left evaluation, and the "hard grammar problem
gets its own spec item" precedent for function literals all descend from
them), [`MA07`](MA07-derive-language.md) (the closest precedent for this
spec's own §1 — a wave item that turns out to need its own real frontend
rather than a thin alias, decided by checking, not assuming).

External: Whitney, K (Kx Systems, 1993) — the family's origin, cited here
only for historical context per §1's own decision to target Q, not K,
first; Borror, *Q for Mortals* (Kx Systems; freely available at
kx.com/q-for-mortals) and Kx Systems' own Q reference card
(`code.kx.com/q/ref/`) — the two sources consulted directly to verify every
syntax claim in §3/§4, per this repo's standing "verified against the
actual documented behavior of a specific named version" discipline.

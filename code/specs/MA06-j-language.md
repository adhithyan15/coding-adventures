# MA06 — J: APL's ASCII-spelled, tacit-programming descendant

## Status

Active spec. Wave 6 of the historical math-languages roadmap
([`HML00`](HML00-historical-math-languages-roadmap.md) §7) — the first
kickoff item after APL (Wave 4). This is **MA-6a**, the design item
(mirroring how [`MA05`](MA05-apl-language.md) was APL's own MA-4a): it
fixes the language scope and — critically — the two places J's grammar
and substrate genuinely differ from APL, before any lexer/parser/runtime
code lands.

## §1 Why J, and why it is not "APL with different spelling"

J (Kenneth Iverson and Roger Hui, 1990) is APL's direct successor,
designed by the same person specifically to shed APL's two biggest
practical obstacles: a non-ASCII glyph set (`⍴ ⍳ ⌈ ⌊ ←` etc., needing a
special keyboard/input method) and a value model limited to dense
rectangular arrays. `+/` computes the same thing — reduce with `+` — in
both languages, needing no re-spelling at all: J is a direct lineal
descendant, sharing APL's core insight (functions are values operators
act on, no operator precedence) — but three of its defining properties
have **no precedent** in this repo's APL frontend and must be designed
up front:

1. **Every glyph is ASCII, so an overloaded character needs an explicit
   digraph, not a distinct code point.** APL's `⌈`/`⌊` are two unrelated
   Unicode characters; J spells them `>.`/`<.` — an ASCII character
   followed by `.` (or `:`) forms a *related but distinct* primitive.
   This has a real, easily-missed consequence: **`/` is not a verb (a
   function) in J at all** — J needed `/` for the reduce *adverb*
   (mirroring APL's `/`), so division, which APL spells `÷` (its own
   glyph, unambiguous), becomes J's `%` instead. A frontend that assumed
   "J's ASCII spellings are just APL's glyphs transliterated
   one-for-one" would get this specific case backwards — spelled
   correctly here, and worth calling out because it is the single most
   common APL→J transliteration mistake.
2. **Tacit function composition — "trains" — is a real grammatical
   novelty, not just new vocabulary.** In APL (this repo's MA05 subset),
   a `function_expr` is always a single primitive glyph, optionally with
   one reduce/scan/outer-product operator applied — an application
   always names its value operands explicitly. J additionally allows
   **two or more verbs (or a leading noun) written consecutively, with
   no operands between them**, to form a brand-new derived verb whose
   meaning depends on how many elements are in the sequence: a **hook**
   (exactly 2) or a **fork** (3, recursively reducing for more). No
   existing frontend in this repo's array or symbolic family has a
   "juxtaposed-verbs-form-a-new-verb" production — this is the hard
   grammar problem this spec fixes before any implementation lands (§3),
   exactly as APL's MA05 §3 fixed its own function/operator split before
   `apl-lexer` landed.
3. **J is 0-indexed by default; APL (as this repo's MA05 implemented
   it) is 1-indexed.** `i.5` produces `0 1 2 3 4`, not APL's `⍳5` →
   `1 2 3 4 5`. Per SIR10's (and this frontend family's) standing
   "disambiguation is the frontend's job" convention, this is not a bug
   to paper over — J's own index generator is honestly 0-based, and any
   later `j-to-semantic-ir` frontend must translate at lowering time
   exactly the way `matlab-to-semantic-ir` already translates MATLAB's
   1-based indexing, not silently assume APL's convention carries over.

Two further genuine J properties are **explicitly out of scope for this
first cut** (§4), each deferred for the same reason MA05 deferred its
own hardest extras:

- **Boxing** (`< x`, heterogeneous/ragged nested arrays) is a real new
  *value* substrate — array-runtime's dense rectangular `Array` has no
  representation for a box. Deferred, matching how MA05 §4 deferred
  APL's own nested/ragged arrays for the identical reason.
- **Explicit rank** (the `"` conjunction, `+/"1` — "reduce at rank 1")
  needs `array-runtime` to support an operand-rank parameter neither
  `AR-2`'s `reduce`/`scan`/`outer` nor this crate's own default-axis
  convention currently has. This is the *same* substrate gap MA05 §4
  already deferred as APL's axis-specific `⌿`/`⍀` — J's rank conjunction
  is a strict generalization of exactly that gap, not a new one, so
  deferring it here is consistent, not a new corner being cut.

Neither deferred property blocks a faithful "historical textbook
session" subset: dense rectangular arrays and default-axis reduce/scan/
outer-product are exactly what `array-runtime` (plus AR-2, already
shipped for APL) already provides.

## §2 Substrate gap: none — `array-runtime` + AR-2 already cover this cut's value model

Checked against the current `array-runtime` public API
(`ops::{reduce, scan, outer, matmul, transpose}`, all already shipped for
APL's MA-4e): this cut's scope (§4) needs dense rectangular numeric
arrays with default-axis reduce/scan/outer-product — precisely what
AR-2 already generalized `array-runtime` to provide, over the *same*
`BinOp` enumeration APL's own runtime already uses. Unlike APL's own
MA-4a (which needed AR-2 built first), **J's kickoff needs zero new
substrate work** — the entire value/operation model is inherited
unchanged. The two properties that *would* need new substrate (boxing,
explicit rank) are exactly the two deferred in §1/§4, so this cut never
touches them.

## §3 Grammar design: verb/noun split (reused from APL) plus trains (new)

Per [`feedback_no_handwritten_lexers_parsers`], J still wraps the shared
`GrammarLexer`/`GrammarParser` — the grammar-tools format already proved
expressive enough for APL's two novel properties (MA05 §3) and remains
so for J's:

- **Two nonterminals, not one — reused directly from APL's `value_expr`/
  `function_expr` split**, renamed to match J's own terminology: J calls
  a function a **verb** and an operator-that-acts-on-verbs an **adverb**
  (monadic, takes one verb — `/` reduce, `\` scan) or a **conjunction**
  (dyadic, takes two verbs, or a verb and a noun — `@` compose is the
  only one in this cut's scope, per §1's rank-conjunction deferral). A
  `noun_expr` (arrays/scalars) and `verb_expr` (primitive glyphs, derived
  verbs from adverbs/conjunctions, **and now trains**) mirror APL's
  `value_expr`/`function_expr` exactly in shape — this part of MA05's
  grammar design transfers with only a vocabulary rename, not a
  redesign.
- **Trains are the one genuinely new production.** A `verb_train` rule
  recognizes 2+ consecutive `verb_atom`s (a bare primitive verb, a
  parenthesised derived verb, or — for a fork's leading position only —
  a bare noun) with no noun between them, and folds them right-to-left
  into nested hook/fork structure:
  - **Hook** (exactly 2 verbs `(f g)`): monadic `(f g) y` = `y f (g y)`;
    dyadic `x (f g) y` = `x f (g y)` — `g` always applies monadically to
    `y` alone, regardless of the surrounding application's own arity.
  - **Fork** (3 verbs `(f g h)`, or a leading noun `(n g h)`): monadic
    `(f g h) y` = `(f y) g (h y)`; dyadic `x (f g h) y` =
    `(x f y) g (x h y)`; a leading noun `n` in `f`'s position is used as
    a literal constant instead of being applied — `(n g h) y` =
    `n g (h y)`.
  - **4 or more verbs** recursively reduce as a fork whose left tooth is
    itself the fork/hook of everything before the last two elements —
    i.e. `(a b c d)` parses as the fork `(a (b c d))`, recursing until a
    2- or 3-element base case remains. This right-to-left recursive
    reduction is the *same* shape as APL's own right-recursive
    `value_expr` (MA05 §3 bullet 2) — trains are not a fundamentally
    different recursion, just one more production folding the same way.
  - Trains are **only valid inside explicit parentheses** in this cut
    (`(f g)`, `(f g h)`, ...) — J's real grammar also permits *unparenthesised*
    trains at the top level of a tacit definition (`f =. g h`), but
    scoping trains to the parenthesised form avoids a genuine grammar
    ambiguity (an unparenthesised run of verbs and nouns is otherwise
    indistinguishable from an ordinary application chain without deeper
    lookahead than this cut's grammar needs) — an honest, disclosed
    subset restriction, the same convention APL's own MA05 §4 used
    throughout.
- **Right-to-left, one precedence tier — reused unchanged from APL.**
  `noun_expr`'s dyadic continuation stays the single right-recursive
  rule MA05 §3 bullet 2 already designed; nothing about trains changes
  this — a train is just one more *shape* a `verb_expr` can take before
  an ordinary application production combines it with its noun
  operand(s).
- **Monadic/dyadic dispatch is still a runtime concern, not a lexer
  concern** — unchanged from MA05 §3 bullet 3, and trains don't change
  this either: whether a hook/fork's *own* surrounding application is
  monadic or dyadic is resolved by which application production matched,
  exactly as it already is for a bare primitive verb.
- **Glyph tokenization needs digraph lookahead, unlike APL's single
  code points.** `<` alone is "less than"; `<.` is "floor/min" — a
  distinct primitive, not `<` followed by a separate `.` token. The
  lexer must greedily match the longest primitive spelling at each
  position (`<.` before falling back to bare `<`), the same
  longest-match-first discipline `apl.tokens` already uses for `∘.`
  (jot-dot, a two-code-point single token) — J needs it far more
  pervasively (roughly a dozen base characters each have a `.`-suffixed
  and/or `:`-suffixed sibling), not for one glyph.

## §4 Language scope (the historical core)

In scope for the first cut — a faithful, textbook-session subset,
following the same "honesty about subsets" convention as every other
language here ([`MA01`](MA01-matlab-language.md),
[`MA04`](MA04-wolfram-language.md), [`MA05`](MA05-apl-language.md)):

- **Arrays only, dense and rectangular** — identical value model to APL
  (§2): a scalar is a rank-0 array, built on `array-runtime::Array`.
- **Primitive verbs** (ASCII spelling, monadic / dyadic meaning):
  `+` (conjugate / add), `-` (negate / subtract), `*` (signum /
  multiply), `%` (reciprocal / divide — **not** `/`, see §1), `^`
  (exponential / power), `<.`/`>.` (floor·ceiling / min·max — note the
  digraph, and note the mapping is the *opposite* character from what
  APL's `⌊`/`⌈` might suggest: J's `<.` is floor because `<` already
  means "less than", and floor rounds *down toward* the lesser
  neighbor), `$` (shape / reshape), `i.` (index generator, **0-based** —
  see §1 — / index-of), `,` (ravel / catenate), `#` (tally — item count,
  new relative to this repo's APL cut, which never added a tally
  primitive — / copy/replicate), `=` `~:` `<` `>` `<:` `>:` (dyadic
  comparison, boolean `0`/`1` result, matching APL's convention).
- **Adverbs**: `/` (reduce), `\` (scan) — the same two AR-2 primitives
  APL already uses, spelled identically (adverbs, unlike verbs, mostly
  didn't need re-spelling — J kept `/` and `\` for exactly the same
  meanings APL gives them).
- **One conjunction**: `@` (compose — "atop": `(f@g) y` = `f (g y)`,
  the *tacit*, operator-form equivalent of function composition, not to
  be confused with a train). The rank conjunction `"` is explicitly
  deferred (§1).
- **Trains**: hooks and forks, parenthesised only (§3).
- **Assignment**: `=.` (local to the current tacit definition — not
  meaningfully different from a bare top-level `=:` in a script with no
  enclosing definition, since this cut has no user-defined verbs/tacit
  *definitions* yet, only tacit *expressions* built from trains) and
  `=:` (global) — both scoped, in this cut, to a bare top-level
  assignment statement (`name =. expr` / `name =: expr`); right-to-left
  evaluation, parenthesised grouping `( )`.
- **Comments** `NB.` to end of line.
- **Negative number literals** (addendum, fixed at MA-6b since this spec
  did not originally settle it): a leading underscore (`_5`, `1.5E_3`) per
  J's own real historical convention — not APL's high-minus `¯` (MA05 §4),
  which has no ASCII spelling. J's real language also overloads a bare `_`/
  `__` for (positive/negative) infinity; that reading is out of scope for
  this cut (never listed among this section's primitives) and is excluded
  structurally, not just by omission — `j.tokens`' `NUMBER` pattern requires
  at least one digit after the underscore, so a lone `_` matches no token
  in this grammar.

**Deferred (post-MA-6):** boxing and nested/ragged arrays (§1), the
`"` rank conjunction and axis-specific reduce/scan (§1 — the direct J
analogue of APL's own deferred `⌿`/`⍀`), user-defined explicit verbs
(`3 : 0`, multi-line definitions) and named tacit definitions (only
inline, parenthesised trains are in scope — see §3's own disclosed
restriction), the `¨` each-equivalent (`&.`/rank-based mapping), complex
numbers, and the wider J vocabulary (`u.`/`v.` locales, `.` as decimal
point overlapping with primitive-suffix `.` in numeric literals needs
lexer care but is not itself deferred — J's real lexer already
disambiguates this via numeric-literal-first matching, which
`grammar-tools`'s existing longest-match convention handles the same
way). Each is a follow-on item exactly as APL deferred its own harder
extras at its MA-4a stage.

## §5 Reuse strategy

- **Lexer/parser**: the `grammar-tools` frontend, exactly as APL —
  `code/grammars/j/j.tokens` + `j.grammar` compile to committed
  `_grammar.rs` in `j-lexer`/`j-parser` via the grammar-tools CLI. The
  verb/noun split and right-to-left `noun_expr` continuation are
  reused from `apl.grammar` nearly verbatim (renamed nonterminals); the
  train production (§3) is the one genuinely new grammar rule.
- **Runtime**: `j-runtime` walks the parse tree and computes over
  `array-runtime::Array`, lowering `+/`/`+\` through the same AR-2
  kernels APL's `apl-runtime` already calls, and evaluating hooks/forks
  by direct recursive application of their tooth verbs (no new
  substrate — a hook/fork *evaluates* to an ordinary value the moment
  it's applied, it never needs its own runtime representation beyond
  "a verb built from other verbs", which the same `AplFn`-style internal
  enum this repo's `apl-runtime` already uses generalizes to naturally:
  add `Hook`/`Fork` variants alongside `Atom`/`Reduce`/`Scan`/`Outer`).
- **REPL & binary**: `j-repl` + a `j` binary, mirroring `apl-repl`. J's
  continuation scanner needs the same paren-balance tracking
  `apl-repl` already has, plus tracking whether `NB.` has opened a
  line comment (mirroring how `apl-repl`'s own scanner already handles
  `⍝`-stripping happening entirely at the lexer's skip-pattern level,
  not in the REPL's own scanner — so in practice no REPL-level change is
  needed here either, `NB.` is just another lexer skip pattern).
- Per [`HML01`](HML01-math-to-semantic-ir.md) §2's amended per-language
  pattern, `j-to-semantic-ir` is built **alongside** the runtime in this
  same wave, not bolted on afterward — mirroring APL's own MA-4f. It
  lowers onto [`SIR22`](SIR22-array-matrix-semantic-ir.md)'s array/matrix
  domain, reusing whatever new `Expr` variants APL's own `apl-to-semantic-ir`
  (MA-4f) needs for reduce/scan/outer-product (that work is tracked
  against APL's own spec, not duplicated here) — a hook/fork train
  lowers to nested applications of those same variants, needing no
  train-specific SIR node of its own, since by the time lowering runs a
  train has already been resolved to "which verb applies to which
  operand," exactly the same shape an ordinary nested application
  already produces.

## §6 Crate layout and rollout (one item = one PR)

```
j-lexer/      src/{lib.rs, _grammar.rs}   ← MA-6b (+ code/grammars/j/j.tokens)
j-parser/     src/{lib.rs, _grammar.rs}   ← MA-6c (+ code/grammars/j/j.grammar)
j-runtime/    src/{lib.rs, eval.rs, value.rs, builtins.rs}   ← MA-6d
j-repl/       src/{lib.rs, main.rs}       ← MA-6d (the `j` binary)
```

- **MA-6a — this spec.** Language scope, the verb/adverb/conjunction
  terminology mapping from APL's function/operator split, and the one
  genuinely new grammar problem (trains) fixed before any lexer/parser/
  runtime code lands.
- **MA-6b — `j.tokens`/`j.grammar`**: the verb/noun grammar (§3),
  reusing APL's shape with renamed nonterminals, plus the new
  `verb_train` production. Digraph/trigraph tokenization (`<.`, `~:`,
  `=.`/`=:`, `NB.`) needs the same longest-match-first discipline
  `apl.tokens` already uses for `∘.`, applied far more pervasively.
  Should ship with a recursion-depth cap from day one, following
  `apl-parser`'s own (twice-corrected, see that crate's `CHANGELOG.md`)
  methodology: measure the *actual* native-stack crash floor for every
  distinct way this grammar can recurse deeply — parenthesised nesting
  **and** a flat right-recursive dyadic chain **and**, new to this
  grammar, a long train — rather than assuming one shape's measured
  floor bounds the others.
- **MA-6c — `j-parser`**: `create_j_parser`/`parse_j`/`try_parse_j`,
  mirroring `apl-parser`'s shape exactly, producing a `GrammarASTNode`
  CST rooted at `program`.
- **MA-6d — `j-runtime` + `j-repl` + the `j` binary.** A working REPL:
  right-to-left evaluation, the §4 primitive set, `$`/`i.` array
  construction, reduce/scan lowered onto AR-2, `@` compose, and hook/
  fork train evaluation.
- **MA-6e — `j-to-semantic-ir`**, per [`HML01`](HML01-math-to-semantic-ir.md)
  §2 — built in this same wave rather than as a later retrofit, per §5.
- **Next**: K/Q per [`HML00`](HML00-historical-math-languages-roadmap.md)
  Wave 6 (a further ASCII-spelled, terser descendant with its own
  additional novelties — e.g. K's own distinct primitive vocabulary and
  Q's SQL-flavored table type — each large enough to warrant its own
  MA-6a-style design pass rather than assuming J's kickoff spec
  transfers unchanged, the same lesson APL's own MA-4a→MA-6a transition
  already demonstrated: a shared grammar-shape *family* still needs a
  fresh substrate/grammar-gap analysis per member, not a rubber stamp).

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (§5
survey, §7 Wave 6), [`HML01`](HML01-math-to-semantic-ir.md) (the
`-to-semantic-ir` standing convention this spec adopts from the start),
[`MA00`](MA00-array-runtime.md) (the substrate — unchanged from APL's
own use of it), [`MA05`](MA05-apl-language.md) (the direct structural
ancestor of this spec — the verb/noun split, right-to-left evaluation,
and the "hard grammar problem gets its own spec item" precedent for
trains all descend from it).
External: Iverson & Hui, *J Introduction and Dictionary* (1990-2006,
Jsoftware) — the language's own canonical primitive-glyph and train
(hook/fork) semantics reference; Iverson, *A Programming Language*
(1962) — APL's original notation J descends from.

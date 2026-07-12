# MA05 — APL: the array-programming root, on `array-runtime`

## Status

Active spec. Wave 4 of the historical math-languages roadmap
([`HML00`](HML00-historical-math-languages-roadmap.md) §7) — the first
kickoff item after MATLAB/Octave (Wave 2) and Wolfram (Wave 3). This is
**MA-4a**, the design item (mirroring how [`MA01`](MA01-matlab-language.md)
was MATLAB's MA-3a): it fixes the language scope and — critically — the two
places APL's grammar and substrate genuinely differ from every array
frontend built so far, before any lexer/parser/runtime code lands.

## §1 Why APL, and why it is not "MATLAB with different spelling"

APL (Kenneth Iverson, IBM, notation 1957–62, implemented 1966) is the root
of the entire array-programming family MATLAB/Octave/Scilab/J/K/Q all
descend from. It fits [`array-runtime`](MA00-array-runtime.md) for the same
reason MATLAB does — arrays are the only data type — but two of its defining
properties have **no precedent** in this repo's existing array or symbolic
frontends and must be designed up front:

1. **Functions are values, and operators act on them.** In MATLAB/Wolfram, an
   operator (`+`, `/@`, …) combines *value* expressions. In APL, `/` (reduce),
   `\` (scan), `¨` (each), and `∘.` (outer product) are **operators**: they
   take a *function* — a primitive glyph like `+` or a user-defined one — and
   produce a **derived function**, which is then applied to values. `+/v`
   means "reduce `v` with `+`"; the `/` never sees `v` directly, it sees the
   function `+` as its left operand. No existing frontend's grammar has a
   "function-valued expression" nonterminal — MATLAB/Wolfram grammars only
   ever combine *value* expressions.
2. **No operator precedence: every expression is parsed right-to-left.**
   `2×3+4` is `2×(3+4) = 14`, not `(2×3)+4`. Every dyadic function has
   *equal, right-associative* precedence — there is no precedence table to
   climb. This is actually **simpler** to express in a grammar than MATLAB's
   multi-level precedence cascade (it is one right-recursive rule, not a
   dozen precedence tiers) — see §3.

Neither property requires a new value substrate: APL's core data model —
dense, rectangular, numeric arrays — is exactly
[`array-runtime::Array`](MA00-array-runtime.md). The gap is entirely in the
**grammar shape** (§3) and, as §2 found, in **array-runtime's operation set**,
which today has no generalized reduce/scan/outer-product.

## §2 Substrate gap: `array-runtime` needs generalized reduce/scan/outer-product first

Checked against the current `array-runtime` public API
(`accel::Kernel`, `exec::execute`/`execute_sum`, `ops::{matmul, transpose}`):
today's substrate has `Kernel::{Elementwise(BinOp), MatMul}` plus one
**fixed** whole-array `execute_sum`. APL's three defining primitives need
operations that do not exist yet:

- **Reduce** (`F/v`) — fold `v` along an axis with an *arbitrary* dyadic
  function `F` (not just `+`), producing a rank-reduced result. `execute_sum`
  is the `F = +`, whole-array special case of this.
- **Scan** (`F\v`) — the same fold, but keeping every intermediate result
  (a running-total analogue), same rank as `v`.
- **Outer product** (`v ∘.F w`) — apply `F` to every pair `(vᵢ, wⱼ)`,
  producing a result of rank `rank(v) + rank(w)`. `Kernel::MatMul` is the
  `F = ×` **and-then-sum-reduce** special case (matrix product *is*
  `+/¨ v∘.×w` in APL terms) — outer product alone (no summing) is new.

This is a genuine **substrate** extension, not an APL-specific hack, and
belongs in `array-runtime` (or `array-runtime` + `matrix-ir`/`matrix-runtime`
if the reduction should also be GPU-dispatchable), so J and K/Q — which have
the *same* three primitives — inherit it for free, exactly how MATLAB/Octave
share one frontend today. This is item **AR-2** in the rollout (§6),
sequenced *before* the APL frontend itself.

`BinOp` (the existing elementwise-op enum) already enumerates the algebraic
operations reduce/scan/outer-product need to be generic over
(`Add`/`Sub`/`Mul`/`Div`, extendable to `Max`/`Min`/comparison ops as APL
requires them) — `AR-2` parameterizes the new kernels over `BinOp`, it does
not invent a second operation-naming scheme.

## §3 Grammar design: two mutually-recursive nonterminals, one precedence tier

Per [`feedback_no_handwritten_lexers_parsers`], APL still wraps the shared
`GrammarLexer`/`GrammarParser` — the grammar-tools format is expressive
enough for both of APL's novel properties without a hand-written parser:

- **Two nonterminals, not one.** `value_expr` (arrays/scalars) and
  `function_expr` (primitive glyphs, derived functions, user-defined dfns).
  An **operator** production takes a `function_expr` and produces a
  `function_expr`. Reduce and scan are postfix on the function (`reduce =
  function_expr "/"`, `scan = function_expr "\"`); outer product is
  **prefix** on the function (`outer = "∘." function_expr`), not infix
  between two function_exprs — `∘.` (jot-dot) always takes exactly ONE
  function operand, immediately to its right (`∘.×` is "outer product with
  ×"); the value operands (`v ∘.F w`, §2) are supplied entirely by the
  surrounding application production below, never by `outer` itself. (This
  corrects an earlier draft of this bullet, which showed `outer` with a
  function_expr on both sides — that shape has no counterpart in §1.1/§2's
  semantic description and was never implemented; see MA-4b's
  `apl.grammar`.) An **application** production takes a `function_expr` and
  one or two `value_expr`s (monadic: `function_expr value_expr`; dyadic:
  `value_expr function_expr value_expr`) and produces a `value_expr`. This
  is an ordinary two-nonterminal CFG — no parser generator changes needed,
  just a grammar shaped differently from MATLAB/Wolfram's single-nonterminal
  expression grammars.
- **Right-to-left, one precedence tier.** A dyadic chain is right-recursive
  with **no** precedence levels between different glyphs: `value_expr =
  term | term function_expr value_expr`. This is simpler than MATLAB's
  cascade (§ in `MA01`), not harder — the entire "operator precedence"
  problem MATLAB/Wolfram both need is *absent* in APL by design.
- **Monadic/dyadic dispatch is a runtime concern, not a lexer concern.**
  Unlike MATLAB's `'` (transpose-vs-string, resolved by the *lexer* from the
  preceding token), whether `+` means "conjugate" (monadic) or "add"
  (dyadic) is resolved by the **parser production actually taken** (which of
  the two application rules matched) — no lexer-level context hook is
  needed, unlike [`MA01`](MA01-matlab-language.md) §3's `'` problem.
- **Glyph tokenization is easy.** Every APL primitive is a single dedicated
  Unicode code point (the APL block, e.g. `⍴ ⍳ ⌈ ⌊ ∘ ¨ ⌿ ⍀ ←`) — no ASCII
  character is overloaded the way MATLAB's `'` or `.` are, so each glyph is
  an unambiguous single-token regex rule (`RHO = "⍴"`, `IOTA = "⍳"`, …). This
  repo's frontend takes **native UTF-8 glyph source** (no keyboard-mapping
  layer — that concern is a historical editor/input-method problem, out of
  scope for a source-string-in frontend).

## §4 Language scope (the historical core)

In scope for the first cut — a faithful, textbook-session subset, following
the same "honesty about subsets" convention as every other language here
([`MA01`](MA01-matlab-language.md), [`MA04`](MA04-wolfram-language.md)):

- **Arrays only.** Dense, rectangular, numeric. A scalar is a rank-0 array.
  Built on `array-runtime::Array` — same value model as MATLAB, no new
  substrate needed for the *data*.
- **Primitive functions** (glyph, monadic / dyadic meaning):
  `+` (conjugate / add), `-` (negate / subtract), `×` (sign / multiply),
  `÷` (reciprocal / divide), `⌈`/`⌊` (ceiling·floor / max·min), `⍴` (shape /
  reshape), `⍳` (index generator / index-of), `,` (ravel / catenate),
  `=` `≠` `<` `≤` `≥` `>` (dyadic comparison, boolean 0/1 result).
- **Operators**: `/` (reduce), `\` (scan), `∘.` (outer product) — the three
  primitives motivating AR-2 (§2).
- **Assignment** `←`, right-to-left evaluation with no precedence (§3),
  parenthesized grouping `( )`.
- **Comments** `⍝` (line comment, to end of line).

**Deferred (post-MA-4):** nested/ragged arrays, mixed numeric+character
arrays in one array, user-defined functions/dfns (`∇`, `{…}`), axis-specific
reduce/scan (`⌿`/`⍀`), the `¨` (each) operator, `⍉` transpose with axis
permutation, complex numbers, and the wider IBM/Dyalog builtin library. Each
is a follow-on item exactly as MATLAB deferred cells/structs/`function`/
`switch` at its own MA-3 stage.

## §5 Reuse strategy

- **Lexer/parser**: the `grammar-tools` frontend, exactly as MATLAB/Wolfram —
  `code/grammars/apl/apl.tokens` + `apl.grammar` compile to committed
  `_grammar.rs` in `apl-lexer`/`apl-parser` via the grammar-tools CLI. Two
  nonterminals (§3) live entirely inside the `.grammar` file; no lexer or
  parser *generator* change is needed.
- **Runtime**: `apl-runtime` walks the parse tree and computes over
  `array-runtime::Array`, lowering `+/`/`+\`/`∘.×` through the new AR-2
  kernels and everything else through the existing `Elementwise`/`MatMul`
  kernels — the value model and GPU dispatch are shared, not reinvented,
  exactly as MATLAB's `matlab-runtime` today.
- **REPL & binary**: `apl-repl` + an `apl` binary, mirroring `matlab-repl`.
- Per [`HML01`](HML01-math-to-semantic-ir.md) §2's amended per-language
  pattern, `apl-to-semantic-ir` is built **alongside** the runtime in this
  same wave, not bolted on afterward — APL is the first language to follow
  that standing convention from day one, lowering onto
  [`SIR22`](SIR22-array-matrix-semantic-ir.md)'s array/matrix domain (reduce/
  scan/outer-product will need their own SIR22 `Expr` variants, added when
  `apl-to-semantic-ir` lands — tracked as a SIR22 follow-up, not scope creep
  into this spec).

## §6 Crate layout and rollout (one item = one PR)

```
array-runtime/    (existing)         ← AR-2 adds generalized reduce/scan/
                                        outer-product kernels
apl-lexer/    src/{lib.rs, _grammar.rs}   ← MA-4c (+ code/grammars/apl/apl.tokens)
apl-parser/   src/{lib.rs, _grammar.rs}   ← MA-4d (+ code/grammars/apl/apl.grammar)
apl-runtime/  src/{lib.rs, eval.rs, value.rs, builtins.rs}   ← MA-4e
apl-repl/     src/{lib.rs, main.rs}       ← MA-4e (the `apl` binary)
```

- **MA-4a — this spec.** Language scope, the function/operator grammar
  design (§3), and the substrate gap (§2).
- **AR-2 — `array-runtime`: generalized reduce/scan/outer-product kernels**
  (✅ done), parameterized over the existing `BinOp` enum. Prerequisite for
  MA-4e; benefits every future array-family language (J, K/Q) for free.
  Landed as `ops::reduce`/`ops::scan`/`ops::outer`, CPU-reference only
  (rank ≤ 2, matching this crate's existing ceiling) — GPU-dispatch wiring
  through `accel`/`exec` is a follow-up, not required to unblock MA-4e.
- **MA-4b — `apl.tokens`/`apl.grammar`** (✅ done): the two-nonterminal
  grammar (§3), validated with `grammar-tools validate` (24 tokens, 2 skip
  patterns, 8 rules, zero cross-validation warnings — every declared token
  is referenced). Files: `code/grammars/apl/apl.tokens`,
  `code/grammars/apl/apl.grammar`.
- **MA-4c — `apl-lexer`** (✅ done): a thin wrapper crate over the shared
  `GrammarLexer`, mirroring `macsyma-lexer`'s shape exactly (no pre/post-
  tokenize hooks needed — every glyph in this subset is single-codepoint and
  unambiguous, per §3 bullet 4). Statically compiles
  `code/grammars/apl/apl.tokens` at build time via `grammar-tools
  generate-rust-compiled-grammars`. 10 tests cover every primitive function
  glyph, the reduce/scan/outer-product operators, assignment/grouping,
  vector-stranded numeric literals, high-minus (`¯`) negative literals kept
  distinct from the `MINUS` function token, and `⍝` line comments.
- **MA-4d — `apl-parser`** (✅ done): the `value_expr`/`function_expr`
  grammar, monadic/dyadic application, reduce/scan/outer-product operator
  productions — `create_apl_parser`/`parse_apl`/`try_parse_apl`, mirroring
  every sibling parser crate's shape exactly. **Shipped with its recursion-
  depth cap from day one** (the shared `GrammarParser`'s guard is per-caller
  opt-in, not a single global fix — a lone constant can't be safe for every
  grammar at once, since rule-chain depth per source-nesting level varies
  wildly; see `macsyma-parser`/`matlab-parser`/`wolfram-parser`'s own
  retrofitted caps, task #12/PR #7928). `apl-parser`'s own empirically-
  derived `MAX_RULE_DEPTH = 150` (72 real nesting levels) produced a
  genuinely counter-intuitive finding: despite APL's much shallower one-
  precedence-tier grammar (no cascade to climb, ~3 rule calls per nesting
  level versus MACSYMA/MATLAB/Wolfram's 13-20), its raw native-stack crash
  floor (209 frames) measured *lower* than theirs (~275-280) — the opposite
  of the natural "fewer calls per level → higher floor" prediction. See
  `apl-parser/src/lib.rs`'s `MAX_RULE_DEPTH` doc comment for the full
  derivation.
- **MA-4e — `apl-runtime` + `apl-repl` + the `apl` binary** (✅ done): a
  working REPL — right-to-left evaluation (falls straight out of walking the
  grammar's right-recursive `value_expr`, no precedence climbing anywhere in
  the evaluator), every §4 primitive (monadic + dyadic), `⍴`/`⍳`/`,` array
  construction and reshaping, and reduce/scan/outer-product lowered onto
  AR-2's `ops::{reduce, scan, outer}`. 64 unit tests in `apl-runtime` + 6 in
  `apl-repl`, covering every primitive (monadic and dyadic), all three
  operators over 2+ distinct `BinOp`s each, right-to-left/grouping/stranding,
  chained assignment, session persistence, comments/blank-line no-ops, and
  the full error surface (undefined variable, non-conformable shapes, empty-
  vector reduce, out-of-range `⍳`, over-rank reshape target, mismatched-row
  `,`, monadic comparison). Design notes and real findings from building this
  layer:
  - **`AplFn`, the runtime's derived-function representation** (`eval.rs`):
    the 12 atoms that map onto `array_runtime::ops::BinOp` (`+ - × ÷ ⌈ ⌊
    = ≠ < ≤ ≥ >`) carry *just* the `BinOp` — there is exactly one glyph per
    `BinOp` variant (only `⌈` produces `Max`, only `⌊` produces `Min`, etc.),
    so `BinOp` alone is enough to recover the glyph for monadic dispatch,
    with no separate tag needed. `⍴`/`⍳`/`,` get their own `NonScalar`
    variant instead of being forced through `BinOp`, exactly as this spec's
    own §4 already anticipated ("Keep RHO/IOTA/RAVEL's monadic+dyadic
    bespoke logic as direct match arms").
  - **Reduce/scan are inherently monadic derived functions; outer product is
    inherently dyadic** — this is a real semantic distinction the grammar
    itself does *not* enforce (`value_expr`'s "function_expr value_expr"
    monadic alternative grammatically accepts *any* `function_expr`,
    including one with `∘.` applied, and the dyadic alternative similarly
    accepts one with `/`/`\` applied). The evaluator rejects the "wrong
    arity" combination — `∘.` applied monadically, or `/`/`\` applied
    dyadically — with a clean, explicit error rather than a silent
    misinterpretation, since the grammar alone can't rule those shapes out.
  - **Row-major vs. column-major is the one place a bug actually appeared
    during implementation**: `array_runtime::Array` stores data
    **column**-major, but APL's `,` (ravel) and `⍴`'s cyclic-fill semantics
    are defined in **row**-major terms. An initial `reshape` cut filled the
    raw column-major buffer directly from the row-major cycle sequence
    (`data[i] = source[i % len]`), which is silently wrong for any non-square
    fill — caught by a matrix-reshape unit test whose expected element order
    was worked out independently from `array_runtime::value::Array`'s own
    documented row-major/column-major example, not copied from the (buggy)
    implementation. Fixed by filling a row-major staging buffer first, then
    transposing it into column-major storage before handing it to
    `Array::from_shape`.
  - **APL-style display is genuinely different from `Array`'s own `Display`**
    (`value.rs`): high-minus `¯` instead of ASCII `-` (the same glyph
    `apl.tokens`'s `NUMBER` rule uses for negative literals, so a printed
    value round-trips as valid input), no `name =`/`ans =` prefix (real APL
    auto-print is bare), and a single-space cell separator instead of
    `Array`'s own 2-space gutter.
  - **The REPL's continuation scanner reduces to plain paren-balance
    tracking** (`apl-repl`), much simpler than `matlab-repl`'s (which also
    tracks block keywords and `"`-strings) — this language cut has neither.
    One real wrinkle: `apl.tokens` does not drop newlines inside `(...)` in
    this first cut, so a naive line-buffer join (MATLAB's approach) would
    hand the parser a real `NEWLINE` token mid-expression and fail; `apl-repl`
    joins continuation lines with a space instead of a literal `\n` so the
    accumulated source stays syntactically one logical line.
  - No discrepancies found against this spec's grammar/CST description —
    `apl-parser`'s tree shapes matched the design exactly. While starting
    this crate, an adversarial probe of `apl-parser` itself (a flat,
    unparenthesised dyadic chain) surfaced a real, separate DoS gap in that
    already-merged crate's own recursion-depth cap — shipped independently
    as its own fix (`apl-parser` 0.1.1), not part of this PR.
  - **Two rounds of `/security-review` found 4 real DoS gaps before this
    crate's first push**: dyadic `,` (catenate) and `∘.` (outer product) can
    each produce a result *larger* than either input, so capping only the
    operands (as `⍳`/dyadic `⍴` already did) wasn't enough — `A←A,A` could
    double a variable's size every line with no ceiling, and `∘.` inherited
    `array_runtime::ops::outer`'s own `checked_mul` (which only guards
    `usize` overflow, not an excessive-but-representable product) with no
    cap of its own. Dyadic `⍳` (index-of) is O(len(a)×len(b)) with no
    complexity bound. A follow-up re-review round then found a fourth,
    more fundamental gap: stranded numeric literals (`1 1 1 …`) never go
    through `builtins.rs` at all — `term`'s repetition is flat, not
    recursive, so `apl-parser`'s own depth cap never bounds the *count* of
    stranded numbers either — bypassing every one of the first three fixes
    via plain literal syntax. All four fixed by capping the actual output
    size / work product at its construction site (not just each operand's
    own length), verified adversarially (guard disabled → regression test
    fails → restored → passes) per this repo's standing DoS-guard-
    verification discipline. See `apl-runtime/CHANGELOG.md` for the full
    writeup.
- **MA-4f — `apl-to-semantic-ir`**, per [`HML01`](HML01-math-to-semantic-ir.md)
  §2 — built in this same wave rather than as a later retrofit.
- **Next**: J (shares APL's function/operator grammar shape almost
  wholesale, ASCII-spelled instead of glyph-spelled — the R/S-style "second
  frontend, same shared grammar shape" reuse), then K/Q per
  [`HML00`](HML00-historical-math-languages-roadmap.md) Wave 6.

## §7 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (§5 survey,
§7 Wave 4), [`HML01`](HML01-math-to-semantic-ir.md) (the `-to-semantic-ir`
standing convention this spec adopts from the start),
[`MA00`](MA00-array-runtime.md) (the substrate; §2's gap analysis is against
this spec's current API), [`MA01`](MA01-matlab-language.md) and
[`MA04`](MA04-wolfram-language.md) (the frontend-on-shared-substrate
playbook this mirrors, and the "hard problem gets its own spec item"
precedent for §3).
External: Iverson, *A Programming Language* (1962) — the notation; IBM APL\360
(1966) — the first implementation; Dyalog APL documentation — the modern
reference for glyph semantics used to check this spec's scope against a
living implementation.

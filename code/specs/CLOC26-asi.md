# CLOC26 — Automatic Semicolon Insertion (ASI)

> **Status:** **Phases 1 and 2 shipped** (the `}` / EOF rule and the
> line-terminator rule, in `javascript-parser/src/asi.rs`); Phase 3 (restricted
> productions) pending. This document was committed *before* any implementation
> code, per the repo standard.
>
> **Implementation refinement (Phase 1):** the shipped implementation uses the
> **retry-on-parse-error** strategy, not the lookahead table this spec first
> sketched under "What counts as an offending token". The two reach the same
> insertion points for the `}`/EOF rule, but retry-on-error has a decisive
> safety advantage: it inserts a `;` *only when parsing genuinely failed for
> lack of one*, so it is **byte-identical-by-construction** on any input that
> already parses — it cannot over-insert. (The "parser-feedback" option this
> spec had rejected was about mutating the shared `GrammarParser` mid-parse;
> the shipped approach instead re-parses from scratch with a fresh parser after
> inserting a token, so it stays entirely within `javascript-parser` and the
> packrat-memo concern does not apply.) The full closurec fixture suite stays
> byte-for-byte unchanged, confirming the no-regression property. Phases 2–3
> (which depend on line-terminator info) will reuse the same retry harness,
> extending the "is this failure ASI-recoverable?" predicate.

## The problem this solves

closurec's grammar parser requires **explicit** statement terminators. Real JS
routinely omits them — automatic semicolon insertion (ASI) is part of the
ECMAScript spec (§12.10). Today, an input as ordinary as

```js
function f(x) { if (x) { return 1; } else { g() } }
//                                          ^^^ no `;` before `}`
```

fails to parse (`Expected … or SEMICOLON, got "}"`), so closurec **silently
degrades the whole program to `WHITESPACE_ONLY`** — every optimization pass is
skipped. This is the single largest reason real-world code gets no SIMPLE/ADVANCED
optimization from closurec: the constant-fold / inline / DCE / rename / control-flow
passes are all reachable, but a single missing semicolon anywhere in the file
turns them all off.

ASI closes that gap. It is the **highest-leverage frontend item** because it
unblocks the entire existing optimizer for the common case of
semicolon-light source.

> **Discovered while shipping CLOC25** (the `else`-hoist): hand-written test
> inputs without semicolons looked "broken" at SIMPLE because they were silently
> falling back to WHITESPACE_ONLY. Fixture inputs currently work around this with
> explicit semicolons; ASI removes the workaround.

## Why this is feasible *and localized* (no shared-crate risk)

The parsing framework was built ASI-ready:

* The lexer emits a dedicated **`TokenKind::Newline`** variant, distinct from
  `Whitespace`, with an explicit note in
  [`javascript-tokens/src/lib.rs`](../packages/rust/javascript-tokens/src/lib.rs)
  (~line 302): *"Newline is sometimes not skipped — ASI implementations need to
  observe newlines to decide whether to insert a semicolon."* Line terminators
  are therefore observable.
* The `GrammarParser` exposes a **pre-parse hook** —
  `add_pre_parse_hook(Fn(Vec<Token>) -> Vec<Token>)`
  ([`parser/src/grammar_parser.rs`](../packages/rust/parser/src/grammar_parser.rs):285,
  applied in `parse()` ~line 322) — the official integration point for
  token-stream rewriting before parsing.
* The parser is PEG + packrat; a synthesized `SEMICOLON` token is just another
  token, so memoization is unaffected (the hook runs *before* any parsing).

**Scope:** ASI is implemented as a token-stream transform **inside the
`javascript-parser` crate only**. There are **no changes** to the shared
`grammar-tools` / `parser` crates (which every other language frontend uses) and
**no changes** to `es2025.grammar` (semicolons stay mandatory in the grammar;
ASI supplies them in the token stream). This is the load-bearing reason ASI is
safe to add: it cannot regress any other language.

### Open design question — RESOLVED in Phase 1

**Answer:** newlines do **not** reach the parser as tokens. The `.tokens`
grammar lists `WHITESPACE` (whose regex `/[ \t\r\n\v\f]+/` includes `\n`) under
`skip:`, so the lexer discards all trivia — the parser's stream is
significant-tokens-only plus one trailing `EOF`. The `}`/EOF rule (Phase 1)
needs no newline info, so it ships now. For Phases 2–3, line-terminator
information is still available **per token** via the lexer's
`TOKEN_PRECEDED_BY_NEWLINE` flag (`lexer::token::Token.flags`), so those phases
do not need newline *tokens* either — they read the flag on the offending token.

<details><summary>Original open question (for the record)</summary>

The hook receives whatever token stream `tokenize_javascript_typed` produces.
`Newline` is classified as trivia (`is_trivia()` groups
`Comment | Whitespace | Newline`,
[`javascript-tokens/src/lib.rs`](../packages/rust/javascript-tokens/src/lib.rs):308),
so it may already be filtered out before `GrammarParser::new`. Two sub-cases:

* **Newlines survive into the parser stream** → ASI is a `add_pre_parse_hook`
  closure, full stop.
* **Newlines are stripped before the parser** → the line-terminator-dependent
  rules can't run in a pre-parse hook. ASI must instead run as a token-stream
  step that still has the newline information — i.e. on the raw lexer output
  inside `tokenize_javascript_typed`, *before* trivia is dropped, emitting a
  stream where the parser-visible tokens already include the synthesized
  semicolons. Still entirely within `javascript-parser`.

Phase 1 begins by confirming which case holds (a 5-line probe test) and pins the
hook location accordingly. The `}`/EOF rules (Phase 1) do **not** need newlines,
so Phase 1 can proceed regardless; only Phases 2–3 depend on the answer.

</details>

## The three ASI rules (ECMAScript §12.10)

1. **Offending token after a line terminator.** When a token that cannot
   continue the current production is encountered and it is **preceded by at
   least one line terminator**, a `;` is inserted before it.
   *(e.g. `a = 1\n b = 2` → `a = 1; b = 2`.)*
2. **Offending token `}` / end of input.** A `;` is inserted before a `}` that
   would otherwise be a syntax error, and at end of input. **This rule does NOT
   require a line terminator** — `{ a() }` on one line still gets a `;`.
   *(This is the rule that fixes the CLOC25 pain point.)*
3. **Restricted productions.** A line terminator is **not allowed** in certain
   positions, and ASI is *forced* there even mid-line:
   `return`/`throw`/`break`/`continue`/`yield` followed by a newline before
   their argument; postfix `++`/`--` preceded by a newline; arrow `=>` — e.g.
   `return\n a` parses as `return; a` (NOT `return a`). Getting this wrong is a
   **miscompile**, so Rule 3 is gated behind its own phase and tests.

### What counts as an "offending token" / "can end a statement"

ASI inserts only where parsing would otherwise fail. The conservative, correct
predicate is **"the parser cannot consume the next token here"**. Two viable
implementations:

* **Lookahead table (preferred for a pure token transform):** a `;` is a
  candidate before token *T* when the **preceding** significant token can end an
  expression/statement (`NAME`, `RPAREN`, `RBRACKET`, `RBRACE`, a literal,
  `this`, `super`, `++`/`--` postfix, …) **and** *T* cannot legally follow it as
  a continuation (`}`, EOF, or — for Rule 1 — any token after a newline that
  isn't an infix continuation like `.`, `(`, `[`, a binary operator, `,`, `?`,
  `:`). The set of "continuation" tokens is small and enumerable.
* **Parser-feedback (rejected):** insert on parse error / retry. Powerful but
  requires modifying the shared `GrammarParser` error path and interacts badly
  with packrat memo invalidation. **Not chosen** — keeps the change out of the
  shared crate.

The lookahead-table approach keeps ASI a pure `Vec<Token> -> Vec<Token>`
function, fully unit-testable in isolation, with zero parser coupling.

## Soundness: never change a valid parse

The transform must satisfy: **for any input that already parses, ASI is a
no-op** (it never inserts a semicolon where the next token is a legal
continuation). This is the central correctness property and is enforced by:

* the continuation-token denylist (never insert before `.`/`(`/`[`/operators/
  `,`/etc.), and
* a regression guard: **all existing closurec fixtures must produce byte-identical
  output** after ASI lands (the 700+ `minify_*` + `simple-*` fixtures already use
  explicit semicolons, so a correct ASI changes none of them).

The dangerous direction is *over-insertion* (changing a valid multi-line
expression's parse). Rule 3 (restricted productions) is the classic trap and is
therefore isolated to Phase 3 with dedicated adversarial tests
(`return\n a`, `a\n ++b` vs `a++\n b`, etc.).

## Phased implementation plan (one PR per phase)

**Phase 1 — `}` and EOF insertion (the high-value, newline-free slice). ✅ SHIPPED.**
New `javascript-parser/src/asi.rs` with
`fn insert_automatic_semicolons(tokens: Vec<Token>) -> Vec<Token>` implementing
Rule 2 only: insert `;` before a `}` / EOF when the preceding significant token
can end a statement and there isn't already a `;`. Wire it into
`tokenize_javascript_typed` / the parser construction. Confirm the
newline-survival question. This alone fixes `{ a() }`, `function f(){return 1}`,
etc. Unit tests for the transform + closurec e2e fixtures (`simple-asi-block`)
that previously degraded now optimize. **Regression guard: every existing
fixture is byte-identical.**

**Phase 2 — line-terminator rule (Rule 1). ✅ SHIPPED.** The retry harness now
also inserts before an offending token preceded by a line terminator. Because
the lexer discards newlines as trivia and does **not** set
`TOKEN_PRECEDED_BY_NEWLINE`, "preceded by a line terminator" is derived from the
per-token `line` field: the offending token starts on a higher line than its
*single-line* predecessor (a multi-line predecessor is declined as ambiguous).
The retry-on-error design means no continuation-token denylist is needed — a
legal multi-line expression (`var c = a` ⏎ `+ b`) simply parses on the first
try, so ASI never fires. Tests cover `a=1`⏎`b=2` (recovered), one-line
`a=1 b=2` (a real error, NOT recovered), and the continued-expression no-op.

**Line-terminator detection — now flag-based (limitation removed).** Phase 2
originally detected Rule 1 with start-line arithmetic
(`off.line > prev.line`), which forced a conservative `token_may_span_lines`
guard (a multi-line string/template predecessor whose *cooked* `value` hid the
newline made the comparison unreliable), and a documented limitation: a
statement ending in a string/template/regex literal before a newline was *not*
recovered. The **lexer** now sets `TOKEN_PRECEDED_BY_NEWLINE` directly on a
token when a line terminator was consumed *as trivia* before it (a newline
*inside* a string/template is consumed by token matching, not trivia, so it
never trips the flag). `asi_applies_at`'s Rule 1 reads that flag, which is
robust regardless of the predecessor's lexeme — so the `token_may_span_lines`
workaround and the string-predecessor limitation are both gone. (Shipped with
`lexer` 0.6.0 / `javascript-parser` 0.17.0; fixture
`closurec/tests/diff/simple-asi-string-newline`.)

**Phase 3 — restricted productions (Rule 3).** Force ASI after
`return`/`throw`/`break`/`continue`/`yield` + newline, and around postfix
`++`/`--`. Adversarial miscompile tests. This is the highest-risk phase and is
deliberately last.

**Phase 4 (optional) — remove the semicolon workaround** from any fixture inputs
that only had explicit semicolons to avoid the fallback, and add a CHANGELOG note
that closurec now optimizes semicolon-light source.

## Test strategy

* **Pure-function unit tests** on `insert_automatic_semicolons` (the transform is
  `Vec<Token> -> Vec<Token>`, trivially testable): each rule, each offending
  token, each continuation-token negative case, idempotence (running twice =
  running once), and the no-op-on-already-valid property.
* **closurec end-to-end fixtures**: inputs that omit semicolons and now optimize
  at SIMPLE, each with the whitespace-fallback guard (the output is NOT the
  whitespace fallback — an optimization that can only come from the typed
  pipeline is present).
* **Regression**: the full existing fixture suite must stay byte-identical
  (proves no over-insertion).

## Non-goals

* No grammar-file changes (semicolons stay mandatory in `es2025.grammar`).
* No shared `grammar-tools` / `parser` crate changes.
* Not changing the bridge, AST, emitter, or any optimization pass — ASI is purely
  a frontend token-stream concern. Once the program parses, everything downstream
  already works.

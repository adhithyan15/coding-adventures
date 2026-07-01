# Changelog — asciimath

All notable changes to the AsciiMath pluggable frontend.

## [0.13.0] — 2026-07-01

### Added — delimiter-wrapping functions (`abs`/`norm`/`floor`/`ceil` → `Fenced`)

The frontend now reads AsciiMath's delimiter-wrapping functions and lowers each to the neutral
`MathExpr::Fenced { open, body, close }` node (the same node the `latex` and `mathml` frontends emit
for their fenced forms), carrying the specific bracket pair as data:

- `abs(x)` → `Fenced { open: "|", body: x, close: "|" }`  (|x|)
- `norm(v)` → `Fenced { … "‖" … "‖" }`  (‖v‖, U+2016)
- `floor(x)` → `Fenced { … "⌊" … "⌋" }`  (⌊x⌋, U+230A/U+230B)
- `ceil(x)` → `Fenced { … "⌈" … "⌉" }`  (⌈x⌉, U+2308/U+2309)

Each takes the next single atom as its body (the `sqrt x` convention — `abs(x)` reads the `(x)`
group, whose parens normalise away), so `abs(x+y)` fences the whole group. Because the bracket kind
is preserved, a bar `|x|` is no longer indistinguishable from a paren `(x)`. Recognised via the
parser's own keyword tables (a single `fenced_delim_of`, shared with the tokenizer's longest-match
scan through `is_keyword`, so lexer and parser can't drift). `capabilities()` now declares
`fenced_delimiters`, closing that gap vs the `latex`/`mathml` frontends; the conformance harness
enforces it. No consumer change — the `Fenced` node already existed and is lowered by the `adj-lang`
adapter; verified across `latex`/`mathml`/`asciimath`/`unicode-math`/`adj-lang`/`adj-lang-cli`.

## [0.12.0] — 2026-07-01

### Added — plus-or-minus (`+-` → ±, `-+` → ∓)

The frontend now reads the AsciiMath plus-or-minus spellings and lowers them to the same neutral
`BinOp::PlusMinus` / `BinOp::MinusPlus` the `latex` (`\pm`/`\mp`) and `unicode-math` (`±`/`∓`)
frontends emit — so a consumer that already lowers those binops gets the AsciiMath spelling for free.

- The tokenizer emits `TokenKind::PlusMinus` for a **contiguous** `+-` and `TokenKind::MinusPlus` for
  `-+`; the parser handles them in the additive loop at the same precedence as `+`/`-`
  (`a +- b + c` ⇒ `(a ± b) + c`).
- A bare `+`/`-` is unchanged, and `a + -b` (add-then-unary-minus, with a space) still parses as
  before — only the contiguous two-character sequences are the ± / ∓ operators.
- `Capabilities::plusminus` is now declared (`with_plusminus()`); the shared `check_frontend` harness
  enforces the honesty. Verified across the full frontend + adj-lang + adj-lang-cli consumer suite.

## [0.11.0] — 2026-06-30

### Added — semicolon-separated fences lower to rows (third Sequence frontend to get rows)

- A fence whose body contains a top-level semicolon — `(a, b; c, d)` — is now read as **rows**:
  semicolons are the row separator and commas the within-row (column) separator, the classic
  fenced-matrix notation. `(a, b; c, d)` lowers to `Sequence([Sequence([a, b]), Sequence([c, d])])`.
  This mirrors the `latex` and `mathml` crates' semicolon-rows exactly, so **all three
  Sequence-emitting frontends now agree on the neutral rows shape** — the same write-once-use-many
  the flat `Sequence` list already demonstrated.
- Degenerate shapes are predictable, identical to LaTeX/MathML: a **semicolon-only** fence `(a; b; c)`
  — a column vector with no second column in any row — collapses to the same flat `Sequence([a, b, c])`
  as a comma list; a **ragged** fence `(a; b, c)` stays faithful as `Sequence([a, Sequence([b, c])])`.
  Each cell is still folded to a full relation. A comma-only fence is unchanged (flat `Sequence`),
  and a separator-free fence is still plain grouping.
- Adds a `Semicolon` token (`;`, previously an "unknown byte" error). `parse_group` records the
  separator after each parsed relation in a bounded loop (never recursion) and groups rows at each
  `;`; a leading/trailing/doubled `;` or `,` leaves a non-atom before the next `parse_relation` → a
  clean spanned error, never a dropped row. No new AST node, no shared-crate change (reuses
  `MathExpr::Sequence`, which already nests). 5 new tests (rows, semicolon-only flat, ragged, folded
  cells, trailing-`;` error). `asciimath` 0.10.0 → 0.11.0.

## [0.10.0] — 2026-06-30

### Added — comma-separated fences lower to `Sequence`

- **`(a, b, c)` → `MathExpr::Sequence([a, b, c])`.** A single fence containing top-level
  commas is now a LIST (a coordinate tuple, an argument list) instead of a parse error: each
  item is parsed as a full relation and collected, mirroring the matrix row's cell loop (a
  bounded loop, never recursion, so a wide list cannot overflow the stack). This is the
  second frontend to emit the neutral `Sequence` node added in `math-frontend` 0.6.0 (after
  `mathml`) — write-once-use-many across notations.
- **A comma-free fence is unchanged** — `(x+1)` / `(a)` still lower to plain grouping (the
  delimiters dropped, the inner expression returned). The matrix shape `((a,b),(c,d))`
  (an outer bracket immediately followed by another opening bracket) is unaffected — it is
  still handled by `parse_matrix` before this path. A trailing/doubled comma leaves a
  non-atom before the next item and yields a clean spanned error (no panic).
- **Capabilities** gain `sequences`, kept honest by the shared `check_frontend` harness (the
  conformance corpus now includes `(a,b,c)`). Requires `math-frontend` >= 0.6.0.
- 3 new tests (comma list, compound items, retained comma-free grouping) + a conformance
  sample; the capability-honesty test asserts `sequences`.

## [0.9.0] — 2026-06-30

### Added — ASM01 PR-3c (the last remainder): greedy longest-match identifier scan

- The tokenizer now carves a maximal letter run into tokens by **greedy longest-match** against
  the known-keyword set — AsciiMath's actual rule — so a glued run reads the way a human does:
  **`sinx` ⇒ `sin x`**, **`pir` ⇒ `pi · r`**, **`inta` ⇒ `int · a`** (longest wins: `int` over
  `in`), **`2pir` ⇒ `2 · pi · r`**, **`sinx^2` ⇒ `(sin x)^2`**. This lifts the PR-1 limitation
  ("function names need a token boundary: `sin x`, not `sinx`" — ASM01 §3.1).
- A stretch with **no** keyword stays a single identifier, so the existing letter-product rule is
  unchanged: **`xy` ⇒ `x · y`**, `text` ⇒ one `Ident`. Sub-tokens carry correct source sub-spans.
- The keyword set is the parser's **own** lookup tables behind a single `parser::is_keyword`, so the
  lexer and parser **cannot drift** — adding a function/symbol is automatic. The operator words
  `xx`/`cdot`/`div` are part of that set so the scan takes them whole; without `cdot` there, the
  run `cdot` would peel its `c` and mis-split the trailing `dot` as the accent. As in real AsciiMath,
  a multi-letter *variable* that collides with a keyword (e.g. `pi`) must be spaced or quoted.
- The prefix search is capped at `MAX_KEYWORD_LEN` (14, the longest keyword `leftrightarrow`), so the
  scan is **linear** in run length — without the cap a long keyword-free run (`aaa…a`) would be O(R²).
  A guard test (`keyword_cap_covers_the_longest_keyword`) pins the constant to the real maximum so it
  can't silently truncate a keyword if a longer one is ever added.
- **No new node, capability, or consumer change** — purely a tokenizer-boundary refinement; every
  existing golden, the conformance corpus, and round-trips are unaffected. Tests: tokenizer
  `longest_match_splits_glued_keywords` / `keyword_free_run_stays_one_identifier` /
  `keyword_cap_covers_the_longest_keyword`, parser `glued_function_name_splits_longest_match`. 41 unit
  + 1 doc test pass; clippy `-D warnings` clean; `/security-review` passed (linear-time cap added).
- This completes the ASM01 PR-3c remainder list; the AsciiMath frontend now matches AsciiMath's
  greedy tokenization in full.

## [0.8.0] — 2026-06-29

### Added — ASM01 PR-3c (part 3): over/under-set emission

- AsciiMath now **emits** the neutral `MathExpr::Overset` / `MathExpr::Underset` nodes
  (math-frontend 0.5.0): **`overset(a)(b)`** and its LaTeX synonym **`stackrel(a)(b)`** →
  `Overset { over: a, base: b }`; **`underset(a)(b)`** → `Underset { under: a, base: b }`. Each
  keyword takes **two atoms** (annotation then base, the same convention as `root(n)(x)`), so the
  paren-free `stackrel a b` form works too and the annotation may be a full group expression
  (`overset(a+c)(R)`). This realizes the stacked-annotation half of the PR-3c remainder, now that
  the neutral node exists.
- Semantics: a centered mark **over/under** the base, **distinct from `Pow`/`Subscript`** (raised /
  lowered) — a faithful renderer must stack it. `capabilities()` adds **`oversets`**, enforced by the
  conformance harness; the corpus gains `overset(a)(b)` and `underset(a)(b)`.
- Tests: parser `oversets_lower_to_neutral_overset_underset_nodes` (overset/underset/stackrel,
  paren-free form, group annotation, distinct-from-Pow). 36 unit + doc tests pass; clippy
  `-D warnings` clean. Mirrors how accent emission (0.3.0) followed the `Accent` node.
- Still **PR-3c remainder** (done in 0.9.0): longest-match identifier scan (`sinx` → `sin·x`).

## [0.7.0] — 2026-06-29

### Added — ASM01 PR-3c (part 2): the `text(…)` keyword form

- The tokenizer now recognizes **`text(…)`** — the parenthesised twin of the `"…"` literal — and
  emits the *same* `TokenKind::Text` the quote form does. So `text(kg)` and `"kg"` lower to an
  **identical** `MathExpr::Text("kg")`, and `5 text(kg)` == `5 "kg"`. **Zero ripple beyond the lexer**:
  no new `TokenKind`, no parser change, no `Capabilities`/conformance change (the `text` capability
  was already declared in PR-1).
- Behaviour: the open paren must *immediately* follow `text` (no space). Inner parens **nest**, so
  `text(f(x))` keeps its inner parens; an unterminated `text(` is a clean spanned error, never a panic.
  `text` not immediately followed by `(` stays an ordinary identifier (a variable named `text`, or
  `text (x)` with a space, is unchanged), and a longer word like `textual` is untouched. Byte-scanning
  for the matching paren is UTF-8-safe (`(`/`)` never occur inside a multi-byte sequence), so non-ASCII
  content slices without panicking.
- Tests: tokenizer `text_keyword_form` (quote-equivalence, empty, nested parens, raw spaces/operators),
  `text_without_immediate_paren_is_an_identifier`, `unterminated_text_keyword_is_an_error`; parser
  `text_keyword_form_equals_quote_literal`. Conformance corpus gains `text(kg)`. 35 unit + doc tests
  pass; clippy `-D warnings` clean.
- Still **PR-3c remainder** (each its own follow-up): **longest-match** identifier scan (`sinx` →
  `sin·x`, a deeper lexer change) and `stackrel`/`overset`/`underset` (need a neutral-AST node in
  `math-frontend` — no `Overset`/`Underset` in `MathExpr` yet).

## [0.6.0] — 2026-06-29

### Added — ASM01 PR-3c (part 1): punctuation arrows `->` and `=>`

- The tokenizer now recognizes the punctuation arrows **`->`** and **`=>`** and emits them as the
  *existing* symbol-table identifiers `rightarrow` / `implies` — so they flow through the PR-3a
  symbol table and lower to `MathExpr::Symbol("rightarrow")` / `Symbol("implies")`, agreeing exactly
  with the word forms `rarr` / `implies`. **Zero ripple**: only `token.rs` changes (two extra
  byte-lookahead branches, mirroring the existing `-:`/`-=`/`<=`/`!=` multi-char operators); no new
  `TokenKind`, no parser change, no `Capabilities`/conformance change.
- The single-character forms are untouched: `a - b` is still `Bin(Sub)`, `a = b` is still `Rel(Eq)`.
  A right-arrow inside a limit bound parses cleanly (`lim_(x -> 0) f` is a `BigOp`) — `x -> 0` is the
  juxtaposition `x · → · 0`, not a `-`/`>` mishmash.
- Tests: tokenizer `multi_char_operators` gains `->`/`=>` (and asserts `-`/`=` unaffected); new parser
  `punctuation_arrows_lower_to_symbols` (arrow lowering, punctuation==word-form agreement, sub/rel
  still intact, limit-bound parse). Conformance corpus gains `a -> b`, `x => y`. 31 unit + doc tests
  pass; clippy `-D warnings` clean.
- Still **PR-3c remainder** (each its own follow-up): **longest-match** identifier scan (`sinx` →
  `sin·x`, a deeper lexer change), `stackrel`/`overset`/`underset` (need a neutral-AST node in
  `math-frontend` — no `Overset`/`Underset` in `MathExpr` yet), and the `text(…)` keyword form (needs
  lexer raw-capture between the parens).

## [0.5.0] — 2026-06-29

### Added — ASM01 PR-3b: bare-keyword spellings + two-letter short forms

The second slice of PR-3, finishing the **symbol-table** surface (the structural items —
longest-match, `stackrel`/`overset`/`underset`, `text(…)` — remain PR-3c). All additions are more
`constant_of` entries lowering to `MathExpr::Symbol(canonical)`, so still **purely additive** (symbol
emission is not a `Capabilities` flag; no consumer/conformance change).

- **Bare English keywords** now lower to symbols: `in`, `and`, `or`, `not`. (PR-3a deliberately
  deferred these because `in` also appears inside big-operator bounds like `sum_(i in S)`; verified
  that mapping it to a symbol leaves such bounds parsing cleanly — `i in S` is the harmless
  juxtaposition `i · ∈ · S`, the same shape the previous `i·n·…` product had.) There is no
  `In`/`And` relation in the neutral `RelOp`, so a `Symbol` standing for the glyph is the faithful
  representation.
- **Two-letter short forms** fold onto the same canonical names as their PR-3a long forms:
  `sub`→`subset`, `sube`→`subseteq`, `sup`→`supset`, `supe`→`supseteq`, `uu`→`union` (= `cup`),
  `nn`→`intersection` (= `cap`), `AA`→`forall`, `EE`→`exists`.
- Behaviour change worth noting: `p("in")` was `i · n` in 0.4.0 and is now `Symbol("in")`; the PR-3a
  test that asserted the product form is updated accordingly. `x in RR` now parses as the
  three-symbol juxtaposition `x · ∈ · ℝ`.
- Tests: new `symbol_table_pr3b_bare_keywords_and_short_forms` (each keyword + short form, short/long
  agreement, `x in RR` juxtaposition, and that `sum_(i in S) i` still parses as a `BigOp`). 30 unit
  tests + doc-test pass; clippy `-D warnings` clean. Spec ASM01 §PR-3 updated (3a + 3b shipped; 3c =
  longest-match + `stackrel`/`overset`/`underset` + `text(…)`).

## [0.4.0] — 2026-06-29

### Added — ASM01 PR-3a: the symbol table (greek, blackboard sets, arrows, set/logic ops)

The first slice of PR-3. `constant_of` grows from a 22-entry Greek-plus-infinity table into a
proper **AsciiMath symbol table** — the fixed dictionary of multi-letter words that name *one*
mathematical glyph rather than a product of single-letter variables (without it, `Sigma` would
parse as `S·i·g·m·a` under the implicit-product rule). Every entry lowers to a
`MathExpr::Symbol(canonical_name)`. Added:

- **Greek** — completed the lowercase alphabet (`omicron`, `upsilon`) and the variant glyphs
  (`varepsilon`, `vartheta`, `varpi`, `varrho`, `varsigma`, `varphi`); added the eleven
  visually-distinct **uppercase** letters AsciiMath capitalizes (`Gamma Delta Theta Lambda Xi Pi
  Sigma Upsilon Phi Psi Omega`).
- **Blackboard number sets** — `NN ZZ QQ RR CC` → `naturals integers rationals reals complexes`.
- **Arrows** (word forms) — `rarr`/`rightarrow`, `larr`/`leftarrow`, `harr`/`leftrightarrow`,
  `uarr`/`uparrow`, `darr`/`downarrow`, plus `implies`, `iff`, `mapsto`.
- **Set / logic** — `notin`, `subset`, `subseteq`, `supset`, `supseteq`, `cup`→`union`,
  `cap`→`intersection`, `emptyset`, `forall`, `exists`, `aleph`.
- **Misc. operators / decoration** — `partial`, `nabla`/`grad`, `propto`/`prop`, `perp`, `angle`,
  `deg`→`degree`, and the dots `ldots cdots vdots ddots`.

Naming convention is documented inline: lowercase Greek verbatim, uppercase Greek capitalized,
number sets as words, arrows/set-ops as familiar TeX-ish long names — so two notations for the same
glyph can later agree on a single `Symbol` string. **Purely additive**: symbol emission is not a
declared `Capabilities` flag, so no consumer or conformance change; latex/adj-lang are unaffected.

**Deferred to PR-3b** (documented in `constant_of` and the ASM01 spec): the bare English keyword
spellings `in`/`and`/`or`/`not` (they need care — `in` also appears inside big-operator bounds like
`sum_(i in S)`), AsciiMath's two-letter short forms (`sub`, `sup`, `uu`, `nn`, `AA`, `EE`),
punctuation arrows (`->`, `=>`, tokenizer concern), longest-match identifier scan (`sinx` → `sin·x`),
`stackrel`/`overset`/`underset`, and the `text(…)` keyword form.

- Tests: `symbol_table_covers_greek_sets_arrows_and_operators` (greek lower/variant/upper, sets,
  arrow short+long agreement, set/logic ops, alias folding, composition in a larger expression, and
  that a deferred bare keyword like `in` still parses as a product without panic). Conformance corpus
  gains `alpha + Omega`, `x in RR`, `a cup b`. 29 unit tests + doc-test pass; clippy `-D warnings`
  clean. Spec ASM01 §PR-3 split into PR-3a (this) + PR-3b.

## [0.3.0] — 2026-06-29

### Added — accents (PR-2b), now that `math-frontend` 0.4.0 has a neutral `Accent` node

AsciiMath accents were deferred at PR-2 because the neutral AST had no way to represent them.
`math-frontend` 0.4.0 added `MathExpr::Accent { accent, body }`, so this release emits it.

- **`hat x`, `bar y` / `overline y`, `vec v`, `dot x`, `ddot x`, `tilde a`, `ul x` /
  `underline x`** now lower to `MathExpr::Accent` — a mark *over* the body, distinct from a
  named-function `Call`. Each accent keyword takes the next single atom as its body (the same
  "one atom argument" convention as `sqrt`/functions), so `hat(x+y)` accents the whole group.
  Synonyms normalise to one canonical name (`bar`/`overline`→`"bar"`, `ul`/`underline`→
  `"underline"`), so two spellings of the same mark lower equal.
- New `accent_of()` keyword table + a parse branch in `parse_ident_atom` (after function
  application). `capabilities()` gains `.with_accents()` — now honest, and the shared
  conformance harness enforces it.
- Tests: `accents_lower_to_neutral_accent_node` (per-accent + synonym + group-body + distinct);
  `name_and_capabilities_are_honest` asserts `accents`; conformance corpus gains `hat x + vec v`
  and `bar(x+y)`. 28 tests pass; clippy `-D warnings` clean.
- This completes the accents-unblock across **both** frontends (latex emitted `Accent` in
  latex 0.13.0; AsciiMath now matches).

## [0.2.0] — 2026-06-29

### Added — ASM01 PR-2: breadth (matrices + big operators)

- **Matrices** `[[a,b],[c,d]]` (rows may use `[…]` or `(…)`) → `MathExpr::Matrix(rows)`, cells
  parsed as full expressions in source order. Disambiguated from nested grouping: `((a))` and
  `[[a]]` remain *grouping* (a 1×1 single-cell shape is not a matrix); a real matrix has ≥2 rows
  or a row with ≥2 cells (i.e. it genuinely uses commas). Ragged rows are rejected (clean spanned
  error, no panic). `det[[a,b],[c,d]]` binds the matrix as the function argument.
- **Big operators** `sum`, `prod`, `int`, `oint`, `coprod`, `lim` → `MathExpr::BigOp{op,lower,upper,body}`.
  Optional `_`/`^` bounds attach to the operator (either order, each at most once); the body is the
  next atom (`sum_(i=1)^n i` ⇒ BigOp over `i`), the same one-atom convention used by `sqrt`/functions.
- New `Comma` token (cell/row separator); outside a matrix it yields a clean error, never a panic.
- `capabilities()` now declares `matrices` + `big_operators` (verified by the shared conformance
  harness against new matrix/big-op examples). **Accents stay out** — the neutral `MathExpr` has no
  `Accent` node yet; adding one to `math-frontend` is a prerequisite for an accents PR.
- Safety: matrix nesting charges the parser's `MAX_DEPTH` (a 3000-deep nested matrix returns a
  spanned error, not a stack overflow); matrix-vs-group is decided by a two-token lookahead —
  single-pass and committed, **no backtracking** — so deep `(((…` input stays linear (no
  exponential re-parse) and fails cleanly at the depth cap.
- Tests: 2×2 / row-vector / paren-row matrices, `det` of a matrix, grouping-not-matrix cases,
  ragged-row error, `sum`/`int`/`prod` with/without bounds and either bound order, comma token,
  deep-matrix overflow guard; conformance examples extended.

## [0.1.0] — 2026-06-28

### Added — ASM01 PR-1: the AsciiMath frontend (core subset)

- New standalone crate `asciimath` (added to the Rust workspace members), the **second**
  pluggable frontend after `latex` (see ASM01 / PFE01). Depends only on `math-frontend`.
- **`AsciiMath`** implements `math_frontend::MathFrontend`: `name() == "asciimath"`,
  `parse(src) -> Result<MathExpr, FrontendError>`, and an honest `capabilities()`. Free
  functions `parse` and `tokenize` are also public.
- **Tokenizer** (`token.rs`): numbers (exact), identifiers (maximal letter runs), the
  operators `+ - * / ^ _ = < >` and multi-char `<= >= != ~~ -= -:`, brackets `( ) [ ] { }`,
  and `"…"` text literals; whitespace skipped. Spanned, total, panic-free.
- **Parser** (`parser.rs`): precedence-climbing relation < add/sub < mul (incl. juxtaposition
  and `xx`/`cdot`/`-:`) < frac (`/`) < unary < scripts (`^`/`_`), over atoms: numbers,
  symbols, function application (`sin`, `ln`, …), `sqrt`/`root(n)(x)`, groups, and `"text"`.
  Identifiers classify as function / known constant (`pi`, `theta`, … , `oo`→`infinity`) /
  operator word / else a product of single-letter symbols (`xy` ⇒ `x·y`). Lowers directly to
  the neutral `MathExpr`; `1/2` ≡ LaTeX's `\frac{1}{2}`.
- **Total / panic-free / bounded:** every input yields `Ok` or a spanned `FrontendError`;
  recursion is depth-guarded (`MAX_DEPTH`) and left-associative chains are built with loops,
  so deep nesting and long chains can't overflow the parser stack.
- **Capabilities** advertised: `fractions, roots, powers, functions, relations, implicit_mul,
  text` (matrices / big-operators are off — PR-2). Conforms to the shared `check_frontend`
  harness (no panics, valid spans, no capability over-claim) + notation-specific goldens.
- Spec `code/specs/ASM01-asciimath-frontend.md`; documented PR-1 limitations + roadmap (§5).
- No `unsafe`; `cargo clippy -- -D warnings` clean.

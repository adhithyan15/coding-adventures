# Changelog — latex

All notable changes to the full-fidelity LaTeX parser crate.

## [0.16.0] — 2026-06-30

### Added — extensible / labelled arrows (`\xrightarrow` & friends)

- The parser now accepts the amsmath **extensible arrows** that carry a stretching label:
  `\xrightarrow`, `\xleftarrow`, `\xleftrightarrow`, `\xRightarrow`, `\xLeftarrow`, `\xmapsto`,
  `\xhookrightarrow`, `\xhookleftarrow`. Each takes a **mandatory `{above}` group** (the label set
  over the arrow) and an **optional `[below]` group** (a second label set under it) — the same
  argument shape as in real LaTeX, e.g. `\xrightarrow[\text{below}]{f}`.
- **No new AST node.** A labelled arrow is *exactly* an annotation stacked on the plain arrow
  symbol, so it lowers onto the existing `Overset`/`Underset` nodes: `\xrightarrow{f}` →
  `Overset { over: f, base: \rightarrow }`, and `\xrightarrow[g]{f}` →
  `Underset { under: g, base: Overset { over: f, base: \rightarrow } }`. Because nothing new is
  introduced, the neutral **`MathFrontend` lowering needs zero change** — `\xrightarrow{f}` lowers
  to the identical `MathExpr::Overset` as the explicit `\overset{f}{\rightarrow}`.
- The surface normalises through `to_latex()` to the equivalent `\overset`/`\underset` form, which
  re-parses to the identical tree (round-trip is a fixed point). A missing mandatory `{above}`
  label, or an unterminated optional `[below]` label, is a **spanned `ParseError`, never a panic**.
- 8 new parser tests (`math.rs`) + 1 neutral-lowering test (`frontend.rs`).

## [0.15.0] — 2026-06-30

### Added — the `array` / `subarray` grids (mandatory column-spec)

- The parser now accepts the general **`\begin{array}{cols} … \end{array}`** grid and its
  big-operator-limit cousin **`\begin{subarray}{c} … \end{subarray}`** — the math environments that
  carry a **mandatory column-alignment argument**. This was the documented gap in the matrix family
  (`pmatrix`/`bmatrix`/`cases`/`aligned`/…, all of which take *no* argument): the comment on
  `is_math_env` explicitly parked `array` for "a later layer" because it "need[s] an extra field on
  the node." That field is now here.
- New optional field **`MathNode::Matrix { col_spec: Option<String>, … }`**: the column spec is
  captured **verbatim** (`"cc"`, `"l|cr"`, `"p{3cm}"`, `*{3}{c}`, `>{…}`/`<{…}`/`@{…}` inserts), so
  the node **round-trips** exactly — `to_latex` re-emits `\begin{array}{<spec>}`. It is `None` for
  every environment that takes no argument (a `pmatrix` is unchanged).
- The neutral **lowering drops `col_spec`** (alignment is presentation, PFE01 §2.2): an `array` and
  the equivalent `pmatrix` lower to the **same** `MathExpr::Matrix`. So consumers gain `array` for
  free — no consumer change, no new neutral node, no capability change.
- `read_col_spec` reads the `{…}` argument with a **flat `depth` counter, not recursion**, so it is
  brace-nesting-aware (captures `p{3cm}` whole) yet an adversarial `{{{{…` cannot overflow the
  stack (bounded by input length, like the rest of the tokenizer output). A missing or unterminated
  col-spec is a clean **spanned error**, never a panic.
- Tests (9 new): `array_captures_column_spec_and_cells`, `…_keeps_rules_and_alignment_letters`,
  `…_handles_braced_groups`, `subarray_takes_a_column_spec_too`, `array_round_trips_through_to_latex`,
  `matrix_family_still_has_no_column_spec`, `array_without_column_spec_is_a_spanned_error`,
  `deep_column_spec_braces_do_not_overflow`, and frontend `array_lowers_dropping_column_spec`
  (array ≡ pmatrix after lowering). 150 unit + doc tests pass; clippy `-D warnings` clean;
  downstream `adj-lang` (the only consumer) 86 green.

## [0.14.0] — 2026-06-30

### Added — `\overset` / `\underset` / `\stackrel` → neutral `Overset` / `Underset`

- The parser recognizes **`\overset{over}{base}`**, **`\stackrel{over}{base}`** (amsmath's name
  for the over-set form), and **`\underset{under}{base}`** — two mandatory args (annotation then
  base, the `\frac`/`\binom` shape) — as new `MathNode::Overset` / `MathNode::Underset`, and the L6
  lower emits the neutral **`MathExpr::Overset` / `MathExpr::Underset`** (math-frontend 0.5.0). A
  centered annotation **over/under** the base, **distinct from `Pow`/`Subscript`** (`b^a` / `b_a`).
  This is the LaTeX-side twin of the asciimath over/under-set emitter.
- `to_latex` round-trips both nodes (`parse_math(node.to_latex()) == node`); the iterative
  `take_children` (deep-Drop guard) handles their two children; both stay atoms in the precedence
  table. The lower runs in the existing work-stack trampoline (no new recursion). `capabilities()`
  already advertises `oversets` via `all()`, so emission is conforming with no capability change.
- Tests: parser `overset_underset_parse_and_round_trip`, lower `overset_underset_lower_to_neutral_nodes`
  (overset/underset/stackrel, distinct-from-Pow). 141 unit + doc tests pass; clippy `-D warnings` clean.

## [0.13.0] — 2026-06-29

### Changed — accents lower to the neutral `MathExpr::Accent` (no longer faked as a function call)

The L6 adapter previously lowered a diacritical accent (`\hat{x}`, `\bar{y}`, `\vec{v}`,
`\tilde{a}`, `\dot{x}`, …) to `MathExpr::Call { func: Func::Other(kind), arg }` — a *function
application*, which is the wrong meaning: `\hat{x}` is a mark *over* `x`, not `hat(x)`. Now that
`math-frontend` 0.4.0 provides a neutral **`MathExpr::Accent { accent, body }`** node, the adapter
emits it faithfully.

- `lower()`'s `MathNode::Accent { kind, body }` arm now pushes a new `Build::Accent(kind)` work step
  (instead of `Build::Call(Func::Other(kind))`); its assembler pops the lowered body and produces
  `MathExpr::Accent { accent: kind, body }`. Still inside the iterative trampoline — O(1) call-frame
  depth, stack-safe on deep accent nests.
- `capabilities()` stays `Capabilities::all()` — now **honestly** includes `accents`, which the
  shared conformance harness (math-frontend 0.4.0) enforces (emitting an Accent requires declaring it).
- Test `accent_is_a_named_unary` becomes `accent_lowers_to_neutral_accent_node` (asserts `\hat{x}` →
  `Accent{accent:"hat", x}` and `\vec{v}` → `Accent{accent:"vec", v}`). Tests pass; downstream
  `adj-lang` green (its adapter's catch-all routes an Accent to "unsupported ADJ arithmetic", since a
  diacritic is not computable — behavior unchanged). clippy `-D warnings` clean.

## [0.12.0] — 2026-06-29

### Fixed — deep `MathNode` trees now drop without overflowing the stack

`MathNode` is a recursive `Box`-owning enum, so the compiler-generated destructor recurses
once per level. The parser builds **left-nested** chains via loops with no per-term depth
charge (`parse_add`/`parse_mul`/`parse_relation`), so input like `1+1+1+…` (or a long
juxtaposition `aaa…`) yields an O(n)-deep tree even though `MAX_DEPTH` bounds *nesting*.
Dropping a deep-enough such tree overflowed the stack — an **uncatchable abort**, even
though `parse_math` itself survives (it builds iteratively). This was the pre-existing
latent hazard flagged in the L6 security review.

- **`impl Drop for MathNode`** now dismantles the tree with an explicit **heap worklist**
  instead of the call stack: each node's boxed children are moved onto a `Vec` (replaced
  in place by a cheap `Sym("")` leaf), popped, and repeated, so the generated destructor
  recurses at most one trivial level. O(1) stack depth regardless of tree depth. This
  mirrors `math_frontend::MathExpr`'s `Drop` (added in `math-frontend` 0.3.0).
- **`take_children` helper** does the per-variant child extraction (leaves contribute
  nothing; `Matrix` rows are drained via `mem::take`).
- Because `MathNode` now implements `Drop`, fields can no longer be moved out of an owned
  value in a by-value `match` (E0509). The `frontend.rs` `lower()` trampoline and the
  by-value test matches were rewritten to `match &node`/`match &n` and extract children via
  `mem::replace`/`Option::take`/`mem::take`. No behavior change — purely how ownership is
  threaded.
- New regression tests: `deep_left_nested_tree_drops_without_overflow` (200k-deep `Bin`
  spine), `deep_parsed_chain_drops_without_overflow` (50k-term `+` chain through the real
  parser), `deep_unary_chain_drops_without_overflow` (200k-deep `Unary` spine). 139 tests
  pass; both the default build and `--no-default-features` (zero-dep L0–L5) stay green.

## [0.11.0] — 2026-06-28

### Changed — L6 closes its two honest neutral-AST gaps (± / ∓ and binomials)

The L6 capstone (0.10.0) had to return a spanned `FrontendError` for `\pm`, `\mp`, and
`\binom`, because the `math-frontend` neutral AST could not represent them. `math-frontend`
0.2.0 added `BinOp::PlusMinus`/`MinusPlus` and `MathExpr::Binom`; this release wires the
`latex` adapter to **emit** them, so every LaTeX math construct the L2/L3a grammar parses now
lowers to a faithful neutral node — no faking, no honest-error islands.

- **`\pm` → `BinOp::PlusMinus`, `\mp` → `BinOp::MinusPlus`** (the ± / ∓ pair operators).
  `lower_binop` is now total (infallible) — the two error arms are gone.
- **`\binom{n}{k}` → `MathExpr::Binom(n, k)`** (binomial coefficient, args in source order;
  distinct from `Frac` — no division bar). Lowered via a new `Build::Binom` work-stack step,
  so it stays inside the iterative (stack-safe) trampoline like every other node.
- `capabilities()` stays `Capabilities::all()` — now **honest**, since the adapter genuinely
  emits ± / ∓ and binomials (the shared conformance harness polices this).
- Tests: the former `neutral_gaps_error_honestly` becomes `plusminus_minusplus_and_binom_lower`
  (asserts the three now lower to the right shapes); the conformance corpus keeps `a \pm b`
  and `\binom{n}{k}` (now exercising emission, not error). 136 tests pass; both the default
  build and `--no-default-features` (zero-dep L0–L5) stay green; clippy `-D warnings` clean.
- Crate 0.10.0 → 0.11.0. **LTX01 L6 now has no honest gaps.**

## [0.10.0] — 2026-06-27

### Added — LTX01 L6: `math-frontend` adapter (the ladder's capstone)

- **`LatexMath`** implements `math_frontend::MathFrontend`: `parse(src)` runs the L2/L3a math
  grammar (`parse_math`) and **lowers** the LaTeX-shaped `MathNode` into the notation-agnostic
  `math_frontend::MathExpr`. LaTeX is now the first **pluggable frontend** of the PFE01 framework
  — a consumer (rule engine, CAS, renderer) lowers one neutral tree and gets LaTeX for free.
- **Registration:** `latex::registry()` returns a `FrontendRegistry` with LaTeX installed, and
  `latex::register_latex(&mut reg)` installs it into an existing one. (`math-frontend`'s own
  `with_builtins()` stays empty by design — it cannot depend on this crate without a cycle; the
  wiring lives here.)
- **Neutral lowering** drops presentation, keeps meaning: `\times`/`\cdot`/juxtaposition →
  `Mul`; `\frac`/`\dfrac`/`\tfrac` → `Frac`; every fence style → `Group`; every matrix
  delimiter → `Matrix`; `a^n` → `Pow`, `a_i` → `Subscript`, `a_i^n` → `Pow(Subscript(..),..)`;
  accents (`\hat{x}`) → `Call{Other(kind), arg}`. Numbers stay **exact** (`MathExpr::Number`,
  never `f64`). Declares `Capabilities::all()`; conforms to the shared `check_frontend` harness.
- **Honest gaps:** `\pm`/`\mp` and `\binom` have **no** neutral representation, so they lower to
  a well-formed spanned `FrontendError` rather than being faked. Extending the neutral AST to
  cover them is a future `math-frontend`-crate change, not a hack here.
- **Feature-gated:** the adapter (and the only dependency, `math-frontend`) sit behind the
  default-on **`frontend`** cargo feature. `--no-default-features` builds the zero-dependency
  L0–L5 document/math parser alone (verified: core tests pass under `--no-default-features`).
- Total / panic-free: the lowering walks the tree with an **explicit work stack** (not native
  recursion), so its call-frame usage is O(1) in tree depth. This matters because a LaTeX math
  tree can be arbitrarily deep along a left-associative spine (`a+a+a+…`, juxtaposition `aaa…`,
  a chained relation) that `parse_math`'s nesting `MAX_DEPTH` does **not** bound — a recursive
  lowering would overflow the stack (an uncatchable abort) on such adversarial input. No `unsafe`.
- +16 tests (frac/pow/subscript/root/func/bigop/rel/implicit-mul/normalization/symbol/text/
  group/accent/matrix/exact-number/gap-errors/parse-error-span/registry/conformance, plus a
  deep-chain no-overflow regression at depth 4000). **136 unit + 1 doc test** green with default
  features; core green under `--no-default-features`; clippy `-D warnings` clean both ways. Crate
  0.9.0 → 0.10.0. **This completes the LTX01 ladder (L0–L6).**

## [0.9.0] — 2026-06-27

### Added — LTX01 L5d: document-structure recognition

- **`recognize_structure(Vec<Node>) -> Vec<Node>`** — a new, opt-in recognition pass (a sibling
  of `expand` / `recognize_accents`) that classifies the *generic* `Node::Command`s produced by
  `parse` (L1) into **semantic** structure nodes. `parse` (L1) is unchanged, so its round-trip
  is preserved; run the pass, or don't.
- Four new `Node` variants (+ a `SectionLevel` enum):
  - **`Node::Section { level, starred, short, title }`** — `\part`/`\chapter`/`\section`/
    `\subsection`/`\subsubsection`/`\paragraph`/`\subparagraph`, including the starred form
    `\section*{T}` (the intervening `Text("*")` sibling is folded) and the optional short TOC
    title `\section[Short]{Title}`.
  - **`Node::CrossRef { command, note, target }`** — `\label`/`\ref`/`\eqref`/`\pageref`/
    `\autoref`/`\nameref`/`\cite`/`\citep`/`\citet`, keeping the `\cite[note]{key}` optional.
  - **`Node::Preamble { command, options, name }`** — `\documentclass`/`\usepackage`/
    `\RequirePackage` with their `[options]`.
  - **`Node::Styled { command, content }`** — argument-form text font commands (`\textbf`,
    `\textit`, `\texttt`, `\emph`, `\underline`, …). Font *declarations* (`\bfseries`,
    `\itshape`, …) stay plain `Command`s — their effect is positional, not a wrapped argument.
- **`to_latex`** renders each recognized node back to the exact shape the pass folds, so
  `recognize_structure(parse(&n.to_latex())) == [n]`. The pass recurses into groups, command
  arguments, environment bodies, and the parts of already-recognized nodes, so it is idempotent
  and composes with itself.
- Total / panic-free: a command that does not match its expected shape (a sectioning command
  with no title, a cross-ref with no key, a styled command with the wrong argument count) is
  left as a plain `Command`, never dropped or mis-folded. Recursion is bounded by the L1 tree
  depth (`MAX_DEPTH`). No `unsafe`; no `MAX_DEPTH` change (Node size still dominated by
  `Environment`).
- +14 tests (plain/starred/short sectioning, all seven levels, title-less command left alone,
  cross-refs, citation note, preamble, styled, declaration-stays-command, recursion into
  titles, surrounding text preserved, idempotence, round-trip corpus). **117 unit + 1 doc
  test** green; clippy `-D warnings` clean. Crate 0.8.0 → 0.9.0.

## [0.8.0] — 2026-06-27

### Added — LTX01 L5c: text accents

- **`recognize_accents(Vec<Node>) -> Vec<Node>`** — a new, opt-in recognition pass (a sibling
  of `expand`) that folds an accent control sequence and the character it accents into a new
  `Node::Accent { accent, arg }`. `parse` (L1) is unchanged, so its round-trip is preserved.
- Recognizes both spellings: control-symbol accents `\'  \`  \^  \"  \~  \=  \.` (which take no
  L1 argument, so they pair with the next node) and control-word accents `\u \v \H \c \d \b
  \r \t` (captured as `\c{e}`, or `\c e` where the lexer absorbed the space). The argument is
  a single following character (`\'e` → é over `e`, the rest of the run kept as text) or a
  braced group. Recurses into groups, command arguments, and environment bodies.
- **`Node::Accent::to_latex`** renders the braced form `\'{e}`, so
  `recognize_accents(parse(&n.to_latex())) == [n]` whether the source wrote `\'e` or `\'{e}`.
- Total / panic-free: a dangling accent (nothing accent-able after it) is left as a plain
  command, never dropped or mis-folded. No `unsafe`.
- +9 tests (control-symbol over next char, first-char-only with remainder kept, braced arg,
  control-word braced + bare, accent-in-group, non-accent untouched, dangling, round-trip
  corpus). **103 unit + 1 doc test** green; clippy `-D warnings` clean. Crate 0.7.0 → 0.8.0.

## [0.7.0] — 2026-06-27

### Added — LTX01 L5b: `verbatim` environment

- **`\begin{verbatim}…\end{verbatim}`** (and `verbatim*`) read their whole body **raw** —
  catcodes suspended, newlines included — up to the matching `\end{<env>}`. New
  `TokenKind::VerbatimEnv { env, content }` → `Node::VerbatimEnv { env, content }`, with a
  round-tripping `to_latex`. The lexer peeks after `\begin` (consuming nothing) and only
  diverts to raw scanning for `verbatim`/`verbatim*`; every other `\begin{…}` is still parsed
  structurally. `\end{…}` for a different name inside the body stays literal.
- Total / panic-free / spanned: an unterminated `verbatim` environment (or one closed with the
  wrong name) is a spanned `LexError`; the raw scan advances one char per step and terminates
  at EOF. No `unsafe`.

### Fixed

- Lowered the structural-parser nesting cap `MAX_DEPTH` 512 → 256. The new owned-`String`
  token/AST variants enlarged recursive-descent frames enough that 512-deep pathological input
  could overflow a small (2 MB) test-thread stack; 256 trips the spanned "nesting too deep"
  guard well within it. Real documents never nest this deep.

### Tests

- +9 (lexer: verbatim-env raw body incl. `{}$#`+newline, `verbatim*`, non-verbatim `\begin`
  left to the parser, inner wrong-`\end` stays literal, unterminated/wrong-close errors;
  parser: `VerbatimEnv` node + 2 round-trips). **94 unit + 1 doc test** green; clippy
  `-D warnings` clean; no `unsafe`. Crate 0.6.0 → 0.7.0.

## [0.6.0] — 2026-06-26

### Added — LTX01 L5a: inline `\verb` verbatim

- **Inline verbatim** `\verb<delim>…<delim>` and the `\verb*` visible-space variant. The
  tokenizer now intercepts `\verb` and reads its body **raw** — catcodes are suspended, so
  `{ } $ # \` etc. are literal characters (previously L1 mis-tokenized them). New
  `TokenKind::Verb { star, delim, content }` → `Node::Verb { star, delim, content }`, with a
  round-tripping `to_latex`.
- Total / panic-free / spanned: an unterminated `\verb`, a body that runs past the end of the
  line, `\verb` at end of input, or a `*`/space delimiter each return a spanned `LexError` —
  never a panic or a mis-parse. The body is bounded by the input (single line).
- +7 tests (lexer: raw body with `{}$#\`, `\verb*`, the error family; parser: `Verb` node,
  surrounding text undisturbed; +2 round-trip-corpus entries). **87 unit + 1 doc test** green;
  clippy `-D warnings` clean; no `unsafe`. Crate 0.5.0 → 0.6.0.
- Scope: this is L5a. The `verbatim` environment, text accents (`\'e`…), sectioning/font
  recognition, and cross-refs are later L5 sub-rungs (spec §5).

## [0.5.0] — 2026-06-26

### Added — LTX01 L4a: macro expansion

- **`expand(nodes: Vec<Node>) -> Result<Vec<Node>, ParseError>`** — a new, opt-in pass over
  the structural document tree (`parse` stays purely structural, so its round-trip is
  preserved). It registers user macros and replaces their uses by substituted, recursively
  expanded bodies; definitions vanish from the output (as in LaTeX).
- **Definitions**: `\newcommand`/`\renewcommand`/`\providecommand` with positional arity
  `[n]` and bodies referencing `#1`..`#9`. Handles L1's argument-capture quirk (it stops the
  greedy `{…}` run at the `[n]` arity bracket) by re-scanning the definition's sibling nodes.
- **Substitution** walks the tree (groups, command arguments, environment bodies) so `#n`
  inside `\bar{#1}` works; `##` is a literal `#`; arguments are expanded call-by-value.
- **Bounded & safe**: total, panic-free, spanned errors. Two guards stop runaway expansion —
  a recursion-depth cap (`MAX_EXPANSION_DEPTH`) and a work-budget cap (`MAX_EXPANSION_STEPS`)
  — so a self-recursive macro or an expansion bomb errors instead of hanging/overflowing.
  Bad calls (too few args, parameter out of range, malformed definition, or an unsupported
  `[n][default]` optional-with-default) are spanned errors.
- **Honest scope (L4a)**: positional args only. Deferred: optional arguments with a default,
  TeX-style `\def`, a built-in starter set, and `#n` substitution inside math islands.
- +16 macro tests (zero/one/two-arg, reordering, macro-calls-macro, param-in-group,
  redefinition, unknown-command pass-through, extra-group retention, `##`, recursion &
  too-few-args & out-of-range & default-arg & malformed-definition errors). **80 unit + 1 doc
  test** green; clippy `-D warnings` clean; no `unsafe`. Crate 0.4.0 → 0.5.0.

## [0.4.0] — 2026-06-26

### Added — LTX01 L3: math environments

- **`MathNode::Matrix { env, rows: Vec<Vec<MathNode>> }`** — math environments with
  row/column structure. `parse_math` now handles `\begin{env} … \end{env}` inside a math
  island: `&` separates columns, `\\` separates rows, and each cell is a full math
  expression. Supported environments (case-sensitive — `bmatrix` ≠ `Bmatrix`): `matrix`,
  `pmatrix`, `bmatrix`, `Bmatrix`, `vmatrix`, `Vmatrix`, `smallmatrix`, `cases`, `dcases`,
  `aligned`, `gathered`, `align`, `align*`, `split`.
- Environments **nest** (a cell may itself be an environment — depth-guarded via the
  enclosing atom), and a `Matrix` is an **atom**, so postfix scripts attach
  (`\begin{pmatrix}…\end{pmatrix}^2`).
- **`MathNode::to_latex`** renders the grid back and **round-trips**
  (`parse_math(&m.to_latex()) == m`); a trailing `\\` before `\end` is tolerated and does
  not create an empty final row.
- Total / panic-free / spanned: `\begin`/`\end` name mismatch, unterminated environment,
  unknown environment, a missing `{` after `\begin`, and a stray `\end` each return a
  spanned `ParseError`. Empty cells (`a & & b`) are a documented limitation (spanned error,
  never a silent empty node), as are `array`/`tabular` column-specs and document-mode list
  environments — those arrive in a later layer.
- +9 environment tests + 5 round-trip-corpus entries; **64 unit + 1 doc test** green;
  clippy `-D warnings` clean; no `unsafe`.

## [0.3.0] — 2026-06-26

### Added — LTX01 L2: math grammar

- **`parse_math(&str) -> Result<MathNode, ParseError>`** — a precedence-climbing parser
  over a math island's raw inner source (the string L1 keeps in `Node::Math`). Re-uses the
  L0 `tokenize` and filters space/par/comment tokens, then climbs:
  relations (`= ≠ < ≤ > ≥ \approx \equiv`) < add/sub (`+ - \pm \mp`) < mul/div
  (`\times \cdot \div /` **and implicit multiplication** via adjacency — `2x`, `\pi r`,
  `(a)(b)`) < unary `± ∓` < scripts (`^`/`_`, right-assoc) < atoms.
- **`MathNode` AST** (`math.rs`): `Num`, `Sym`, `Bin`, `Unary`, `Frac`/`\dfrac`/`\tfrac`,
  `Binom`, `Root { degree, radicand }` (`\sqrt[n]{}`), `Script { base, sub, sup }`,
  `Call { func, arg }` (named functions `\sin \log …`), `BigOp { op, lower, upper, body }`
  (`\sum \prod \int \lim` with bound scripts), `Accent` (`\hat \bar \vec …`),
  `Fenced { left, body, right }` (`\left( … \right)`, `\langle`, `|`, …), `Text`, and
  `Rel`. `{…}` groups are **transparent** (grouping only — they do not appear as nodes).
- **`MathNode::to_latex`** — precedence-aware round-trip: `parse_math(&m.to_latex()) == m`.
  Children below the parent's precedence are wrapped in invisible `{…}` so the re-parse
  re-associates identically.
- **`Node::parsed_math`** — parses a `Node::Math` island on demand; the L1 structural tree
  is unchanged (its round-trip stays intact).
- Total / panic-free / spanned; recursion is **depth-guarded** (`MAX_DEPTH`) so adversarial
  nesting (e.g. thousands of `(`) returns a spanned error instead of overflowing the stack.
- +15 math tests incl. the worked corpus (`\frac{12 \times 15}{3}`, `2^{10}`,
  `\sqrt[3]{27}`, `\sum_{i=1}^{n} i`, `\left(\frac{a}{b}\right)^2`), a round-trip corpus,
  relations/functions, and the deep-nesting bound; clippy `-D warnings` clean, no `unsafe`.

## [0.2.0] — 2026-06-26

### Added — LTX01 L1: structural document parser

- **`parse(&str) -> Result<Vec<Node>, ParseError>`** — a recursive-descent parser that
  turns the L0 token stream into a document tree:
  - ordinary characters coalesced into `Text`; `Space`/`Par`;
  - `{ … }` → `Group`;
  - `\cmd[opt]{arg}…` → `Command` (one optional `[…]` if it immediately follows, then a
    greedy run of mandatory `{…}` groups — generic capture; per-command arity is a later
    layer, so `\textbf{a}{b}` captures two args, and a space breaks the run);
  - control symbols (`\,`, `\\`, `\{`) → argless `Command`;
  - `\begin{env}[opt]{arg}… body \end{env}` → `Environment` with a **matched** close
    (a `\begin{a}…\end{b}` mismatch is a spanned error); environments nest;
  - math islands (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) → `Math { display, content }` keeping
    the **raw inner source** for L2;
  - comments, active `~`; in text mode `& # ^ _` are literal characters.
- **`Node` AST** (`ast.rs`) with **`Node::to_latex`** / `document_to_latex` — round-trips:
  `parse(&render(ast)) == ast` (AST-equality; surface spacing and `$`/`\(` delimiter
  choice are normalized). Reserves an `Unsupported { construct, span }` variant for the
  TeX-programmability asymptote (not produced at L1).
- **`ParseError`** — spanned; structural errors (unbalanced braces, env mismatch,
  unterminated env/math) never panic.
- +19 tests (39 unit + 1 doc total), incl. a round-trip corpus; clippy `-D warnings` clean.

## [0.1.0] — 2026-06-26

### Added — LTX01 L0: crate scaffold + catcode tokenizer

- New standalone, **zero-dependency** crate `latex` (added to the Rust workspace members).
  A full-fidelity LaTeX parser for documents *and* math; first frontend of the
  `math-frontend` framework.
- **`catcode(c)`** — TeX category codes (default plain-LaTeX assignments): Escape,
  BeginGroup, EndGroup, MathShift, AlignTab, EndLine, Parameter, Superscript, Subscript,
  Space, Letter, Other, Active, Comment.
- **`tokenize(&str) -> Result<Vec<Token>, LexError>`** — a catcode-driven, **text-mode-
  primary** state machine:
  - mode stack: Text (primary) ↔ Math (pushed by `$`/`\(`/`\[`, display via `$$`/`\[`,
    popped by the matching close); whitespace is significant in text, ignored in math;
  - control words (`\`+letters, with TeX space-absorption) and control symbols
    (`\`+non-letter, incl. `\\` line break, `\{`, `\,`, …);
  - groups `{ }`, math on/off (`MathOn`/`MathOff` with inline/display flag), `&`, `#`,
    `^`, `_`, active `~`, comments (`%` to end of line, eating the newline);
  - whitespace: a run collapses to one `Space`; a blank line (≥2 newlines) is `Par`;
  - ordinary characters emitted one-per-`Char` (faithful to TeX; the parser coalesces).
- **`Token` / `TokenKind` / `Span`** — every token carries a half-open byte span.
- **`LexError`** — spanned; the scanner **never panics** (trailing `\` → spanned error;
  a stray `\)`/`$` in text mode does not underflow the mode stack).
- 20 unit + 1 doc test; `cargo clippy -- -D warnings` clean; no `unsafe`.

### Notes

- Scope is full LaTeX surface; the Turing-complete TeX tail is the documented asymptote
  (see LTX01). The structural parser (L1), math AST (L2), environments (L3), macros (L4),
  text breadth (L5), and the `MathFrontend` adapter (L6) arrive in subsequent layers.

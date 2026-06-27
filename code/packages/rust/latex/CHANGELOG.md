# Changelog — latex

All notable changes to the full-fidelity LaTeX parser crate.

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

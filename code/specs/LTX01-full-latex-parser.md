# LTX01 — Full LaTeX Parser (`latex` crate)

**Status:** Specs-first. The first frontend for [PFE01](PFE01-pluggable-parser-frontends.md).
**Author:** architecture pass, 2026-06-26.
**Goal:** **Full LaTeX compatibility** — parse real-world LaTeX (documents *and* math),
not a math subset — into a faithful AST. Standalone and reusable; also implements
`math-frontend::MathFrontend`.

## 1. What "full LaTeX" means here (and its honest limit)

LaTeX rests on TeX, whose macro layer is **Turing-complete** (token-list manipulation,
runtime category-code reassignment, `\csname`, conditionals, `\expandafter`). A parser that
*executes arbitrary TeX* is a TeX engine, not a parser. So we draw a principled line:

- **In scope — "full LaTeX surface + standard semantics":** parse the complete surface
  syntax (catcodes, control sequences, groups, both modes, environments, scripts,
  delimiters, accents, special characters, the preamble) into a faithful AST, **and**
  support the macro mechanisms authors actually use — `\newcommand`/`\renewcommand`/`\def`
  with positional (`#1`..`#9`) and optional arguments, plus expansion of *user-defined* and
  *common built-in* macros.
- **Asymptote — documented, best-effort or rejected-with-a-clear-error:** runtime catcode
  reassignment (`\catcode`), full `\expandafter`/`\noexpand`/`\csname` gymnastics,
  arbitrary `\if…` programming, `\input`/`\include` of external files. These are reported as
  `Unsupported { construct, span }` nodes/errors, never silently mis-parsed.

This gives genuine full-document fidelity for human- and model-written LaTeX while being
explicit about the TeX-programmability tail. The conformance ladder (§5) makes the covered
surface precise and testable.

## 2. The model — catcodes, modes, groups

LaTeX meaning is **stateful**; the parser mirrors TeX's own machine:

- **Category codes (catcodes).** Each character has a category: escape `\`, group-begin
  `{`, group-end `}`, math-shift `$`, alignment `&`, end-of-line, parameter `#`,
  superscript `^`, subscript `_`, ignored, space, letter, other, active `~`, comment `%`,
  invalid. The tokenizer is catcode-driven (defaults per the LaTeX standard). Runtime
  `\catcode` changes are in the asymptote (§1).
- **Modes.** **Text mode is primary** (LaTeX starts in paragraph/horizontal mode). Math
  mode is entered by `$ … $` / `\( … \)` (inline) and `$$ … $$` / `\[ … \]` /
  `\begin{equation}` (display), and exited at the matching shift. This is the inverse of a
  math-only parser and is the central state in the machine.
- **Groups.** `{ … }`, `\begingroup … \endgroup`, and environments open/close scopes.
  Brace depth + a mode/scope stack are tracked so closes match the right open.
- **Control sequences.** *Control words* (`\` + letters, e.g. `\frac`, `\section`) and
  *control symbols* (`\` + one non-letter, e.g. `\,`, `\\`, `\{`). Active characters (e.g.
  `~`) behave like single-token macros.

## 3. The AST (document tree)

```rust
pub enum Node {
    Text(String),                         // a run of ordinary text
    Space,                                // significant inter-word space
    Group(Vec<Node>),                     // { … } scope
    Command { name: String, opt: Vec<Vec<Node>>, args: Vec<Vec<Node>> },  // \cmd[opt]{arg}…
    Environment { name: String, opt: Vec<Vec<Node>>, args: Vec<Vec<Node>>, body: Vec<Node> },
    Math { display: bool, body: Vec<MathNode> },   // $…$ / \[…\] — math subtree (§4)
    MacroDef { /* \newcommand/\def: name, params, optional-default, body */ },
    Comment(String),
    Special(SpecialChar),                 // ~ (nbsp), -- (en), --- (em), \& \% \$ \_ …
    Unsupported { construct: String, span: (usize, usize) },  // the asymptote (§1)
}
```

Math is its own subtree (`MathNode`) so the math grammar (precedence, scripts, big
operators) is clean and so the `MathFrontend` adapter can lift it directly to the neutral
`MathExpr` of [PFE01](PFE01-pluggable-parser-frontends.md).

```rust
pub enum MathNode {
    Num(String), Sym(String),
    Bin(BinOp, Box<MathNode>, Box<MathNode>),       // + - * / \times \cdot \div \pm \mp …
    Unary(UnaryOp, Box<MathNode>),
    Frac(Box<MathNode>, Box<MathNode>), Binom(Box<MathNode>, Box<MathNode>),
    Root { degree: Option<Box<MathNode>>, radicand: Box<MathNode> },
    Script { base: Box<MathNode>, sup: Option<Box<MathNode>>, sub: Option<Box<MathNode>> },
    BigOp { op: String, lower: Option<Box<MathNode>>, upper: Option<Box<MathNode>>,
            body: Box<MathNode> },                  // \sum \prod \int \lim …
    Call { func: String, arg: Box<MathNode> },      // \sin \ln \exp …
    Accent { kind: String, body: Box<MathNode> },   // \hat \bar \vec …
    Fenced { left: Delim, body: Box<MathNode>, right: Delim },   // \left( … \right)
    Group(Box<MathNode>), Text(String),
    Rel(RelOp, Box<MathNode>, Box<MathNode>),
    Matrix { env: String, col_spec: Option<String>, rows: Vec<Vec<MathNode>> },
}
```

Every node records a byte span. The parser is **total and panic-free**: malformed input
yields a spanned `ParseError`, never a panic.

**Deep-tree drop safety.** The math AST (`MathNode`) is a recursive `Box`-owning tree.
`MAX_DEPTH` bounds *nesting*, but left-associative operator/relation chains are built in
loops with no per-term depth charge, so well-formed input like `1+1+1+…` produces an
O(n)-deep tree that `parse_math` returns successfully (it builds iteratively). A
compiler-derived destructor would recurse one frame per level and overflow the stack on a
deep-enough tree — an uncatchable abort that the spanned-error contract cannot cover. So
`MathNode` (and the neutral `math_frontend::MathExpr`) implement `Drop` explicitly,
dismantling the tree with a heap worklist (O(1) stack depth). Panic-freedom therefore holds
through teardown as well as parse. (Implemented in `latex` 0.12.0 / `math-frontend` 0.3.0.)

## 4. Public API (`latex` crate)

```rust
pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError>;   // catcode-driven scanner
pub fn parse(src: &str) -> Result<Vec<Node>, ParseError>;     // full document
pub fn parse_math(src: &str) -> Result<MathNode, ParseError>; // a bare math expression
impl Node { pub fn to_latex(&self) -> String }                // round-trip pretty-printer
// math-frontend integration (behind a feature or thin adapter module):
impl math_frontend::MathFrontend for Latex { /* name="latex"; parse → MathExpr */ }
```

## 5. Conformance ladder = PR staging

Each rung is a small, reviewable, fully-tested PR; each builds on the prior. The **final
state is full LaTeX compat** per §1. (The earlier math-only tokenizer experiment on the
parked branch `feat/latex-math-parser` is a reference for L0/L2 — reuse what fits, but L0
is text-primary, which is a different mode model.)

- **L0 — catcode tokenizer.** Scaffold the `latex` crate; the catcode-driven state machine:
  text/math mode stack, group/brace depth, control words + control symbols, comments,
  active chars, escaped specials, math-shift detection (`$`, `\(`, `\[`, `$$`). Spanned
  `LexError`; exhaustive tokenizer tests.
- **L1 — structural document parser.** Groups; generic `\cmd[opt]{arg}…` with argument
  capture (mandatory `{}` + optional `[]`); generic `\begin{env}…\end{env}` with matched
  close; text runs, spaces, comments, special chars (`~`, `--`, `---`, `\&` …); `Math`
  nodes delimited but body left as raw tokens for L2. `to_latex()` round-trip.
- **L2 — math mode (full fidelity).** Parse `MathNode`: precedence climbing, atoms, groups,
  `\left\right` fences, infix ops, unary, implicit multiplication, `\frac`/`\binom`,
  `\sqrt[n]{}`, scripts `^`/`_`, functions, big operators with bounds, accents,
  greek/constants/symbols, relations.
- **L3 — environment semantics.** Split into two sub-rungs:
  - **L3a (shipped) — math environment family.** The matrix family
    (`matrix`/`pmatrix`/`bmatrix`/`Bmatrix`/`vmatrix`/`Vmatrix`/`smallmatrix`), `cases`,
    the alignment environments (`aligned`/`gathered`/`align`/`align*`/`split`), and the general
    **`array`/`subarray` grids** parsed inside math islands via `&` (columns) and `\\` (rows) →
    `MathNode::Matrix { env, col_spec, rows }`, with `to_latex` round-trip, nesting, and postfix
    scripts. `array`/`subarray` carry a **mandatory `{column-spec}` argument** (`\begin{array}{l|cr}`)
    captured verbatim on `col_spec` (`None` for every other environment); the neutral lowering drops
    it (alignment is presentation, PFE01 §2.2), so `array` ≡ `pmatrix` as `MathExpr::Matrix`. The
    col-spec reader is brace-nesting-aware yet iterative (no stack-overflow on adversarial `{{{…`).
    Empty cells, a missing col-spec, and an unterminated env are documented spanned errors.
    Implemented in the `latex` crate (math.rs).
  - **L3b (later) — document-mode tables & lists.** Row/column structure for the text-mode
    `tabular` family and list environments (`itemize`/`enumerate`/`description`) with `\item`,
    operating on the document `Node` tree. (Math-mode `array`/`subarray` shipped in L3a.) An unknown
    `\begin{…}` is rejected with a spanned error in the meantime, never mis-parsed.
- **L4 — macros.** Split into sub-rungs:
  - **L4a (shipped) — positional macros.** `\newcommand`/`\renewcommand`/`\providecommand`
    with positional arity `[n]` and `#1`..`#9` bodies; opt-in `expand(Vec<Node>)` pass that
    registers definitions (which then vanish) and replaces uses by substituted, recursively
    expanded bodies. Recursion-depth + work-budget guards → spanned error, never hang.
    Implemented in the `latex` crate (macros.rs).
  - **L4b (later) — optional-arg defaults + `\def`.** `\newcommand{\x}[n][default]{…}` and
    TeX-style `\def\x#1#2{…}` with arbitrary parameter text; `#n` inside math islands.
  - **L4c (later) — built-in starter set** of common macros pre-registered.
- **L5 — text-mode breadth.** Split into sub-rungs:
  - **L5a (shipped) — inline `\verb`.** `\verb<delim>…<delim>` and `\verb*` read raw at the
    tokenizer (catcodes suspended) → `Node::Verb { star, delim, content }`, round-tripping;
    spanned errors on unterminated / line-spanning / bad-delimiter. Implemented in
    lexer.rs + parser.rs + ast.rs.
  - **L5b (shipped) — `verbatim` environment** (`\begin{verbatim}…\end{verbatim}`, `verbatim*`)
    read raw to the matching `\end` (catcodes suspended, newlines kept) → `Node::VerbatimEnv`,
    round-tripping; lexer peeks after `\begin` and only diverts for these env names; spanned
    error on unterminated/wrong close. Implemented in lexer.rs + parser.rs + ast.rs.
  - **L5c (shipped) — text accents** (`\'e`, `\"o`, `\~n`, `\^o`, `\=`, `\.`, `\u`, `\v`, `\H`,
    `\c`, `\d`, `\b`, `\r`, `\t`) recognized over the next char/group → `Node::Accent`, via the
    opt-in `recognize_accents` pass (mirrors `expand`; L1 round-trip preserved); `to_latex`
    re-recognizes either spelling. Implemented in text.rs + ast.rs.
  - **L5d (shipped) — sectioning/font recognition + cross-refs + preamble.** The opt-in
    `recognize_structure` pass (mirrors `recognize_accents`/`expand`; L1 round-trip preserved)
    classifies the generic `Command` nodes L1 already produces into semantic nodes:
    - `Node::Section { level, starred, short, title }` — `\part`/`\chapter`/`\section`/
      `\subsection`/`\subsubsection`/`\paragraph`/`\subparagraph`, the starred form
      (`\section*{T}` — the intervening `Text("*")` sibling is folded), and the optional short
      TOC title (`\section[short]{Title}`);
    - `Node::CrossRef { command, note, target }` — `\label`/`\ref`/`\eqref`/`\pageref`/
      `\autoref`/`\nameref`/`\cite`/`\citep`/`\citet` (the `\cite[note]{key}` optional kept);
    - `Node::Preamble { command, options, name }` — `\documentclass`/`\usepackage`/
      `\RequirePackage` with their `[options]`;
    - `Node::Styled { command, content }` — the argument-form text font commands
      (`\textbf`/`\textit`/`\texttt`/`\emph`/`\underline`/…).

    A command that does not match its expected shape (a sectioning command with no title, a
    cross-ref with no key, …) is left as a plain `Command` — never dropped or mis-folded. Font
    *declarations* (`\bfseries`, `\itshape`, `\large`, …) stay plain commands: their effect is
    positional (until end of group), so wrapping them in an argument node would misrepresent
    them. `to_latex` re-renders each recognized node to a form that re-recognizes to the same
    node, so `recognize_structure(parse(&n.to_latex())) == [n]`. Implemented in structure.rs +
    ast.rs.
- **L6 (shipped) — `math-frontend` adapter.** `LatexMath` implements `MathFrontend` for `latex`:
  `parse` runs the L2/L3a grammar and lowers `MathNode` → neutral `MathExpr` (presentation
  dropped, meaning kept — `\times`/`\cdot`/juxtaposition → `Mul`, fence style → `Group`, matrix
  delimiter → `Matrix`, accents (`\hat{x}`/`\vec{v}`) → `Accent{accent, body}` (a diacritic over
  its body, distinct from a function `Call`; needs `math-frontend` ≥ 0.4.0), exact numbers
  preserved). Declares
  `Capabilities::all()` and conforms to the shared `check_frontend` harness. LaTeX is registered
  as plugin #1 via `latex::registry()` / `register_latex` — `math-frontend`'s `with_builtins()`
  stays empty because that crate cannot depend on `latex` (cycle), so the wiring lives in the
  `latex` crate. Gated behind the default-on `frontend` cargo feature; `--no-default-features`
  keeps L0–L5 dependency-free. Implemented in frontend.rs + Cargo.toml.
  **Neutral-AST gaps (closed in latex 0.11.0):** the L6 capstone initially had to lower `\pm`,
  `\mp`, and `\binom` to a spanned `FrontendError`, because the neutral `MathExpr` could not
  represent them. `math-frontend` 0.2.0 added `BinOp::PlusMinus`/`MinusPlus` and `MathExpr::Binom`
  (plus the matching `Capabilities` flags + conformance policing), and `latex` 0.11.0 wires the
  adapter to **emit** them (`\pm`→`PlusMinus`, `\mp`→`MinusPlus`, `\binom{n}{k}`→`Binom(n,k)`).
  Every LaTeX math construct the grammar parses now has a faithful neutral counterpart — no
  honest-error islands remain. This rung **completes the LTX01 ladder (L0–L6).**

  **Over/under-set emission (latex 0.14.0):** `\overset{a}{b}`, `\stackrel{a}{b}` (amsmath's
  over-set synonym), and `\underset{a}{b}` parse to new `MathNode::Overset`/`Underset` (two
  mandatory args, annotation then base) and the L6 lower emits the neutral
  `MathExpr::Overset`/`Underset` (added in `math-frontend` 0.5.0) — a centered annotation over/under
  the base, distinct from `Pow`/`Subscript`. `to_latex` round-trips both; `capabilities()` already
  advertises `oversets` via `all()`. The LaTeX-side twin of the asciimath over/under-set emitter.

  **Extensible / labelled arrows (latex 0.16.0):** the amsmath stretching-arrow family —
  `\xrightarrow`, `\xleftarrow`, `\xleftrightarrow`, `\xRightarrow`, `\xLeftarrow`, `\xmapsto`,
  `\xhookrightarrow`, `\xhookleftarrow` — parses with a **mandatory `{above}` group** and an
  **optional `[below]` group** (`\xrightarrow[g]{f}`). No new node: a labelled arrow IS an
  annotation stacked on the plain arrow symbol, so it desugars onto the existing `Overset`/`Underset`
  nodes — `\xrightarrow{f}` → `Overset { over: f, base: \rightarrow }`, `\xrightarrow[g]{f}` →
  `Underset { under: g, base: Overset { … } }`. The L6 lowering therefore needs **zero change**
  (`\xrightarrow{f}` lowers to the identical `MathExpr::Overset` as `\overset{f}{\rightarrow}`).
  `to_latex` normalises to the `\overset`/`\underset` form (a round-trip fixed point); a missing
  mandatory label is a spanned error, never a panic.

**Asymptote (documented in README, not built):** runtime `\catcode`, `\expandafter`/
`\noexpand`/`\csname`, arbitrary `\if…` programming, external `\input`/`\include`. Hit →
`Unsupported { construct, span }`.

## 6. Crates & files

- New crate `code/packages/rust/latex/` (scaffold like `bitset/`: Cargo.toml edition 2021 /
  MIT / minimal deps, `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`,
  `required_capabilities.json`, `src/{lib.rs,catcode.rs,token.rs,lexer.rs,ast.rs,parser.rs,
  math.rs,macros.rs,error.rs}`); add to workspace `members`.
- Depends on `math-frontend` only for the L6 adapter (gate so L0–L5 stay dependency-free).

## 7. Verification

- `cargo test -p latex` and `cargo clippy -p latex -- -D warnings` green at every rung;
  no `unsafe`.
- **Round-trip corpus:** real LaTeX snippets (a paragraph with inline `$…$`, a `pmatrix`,
  an `align`, a `\newcommand` use, a sectioned document) where `parse(s).to_latex()`
  re-parses to an equal AST.
- **Error spans:** unbalanced braces, `\begin{a}…\end{b}` mismatch, `\frac{1}`, unknown
  environment, unterminated math → spanned `ParseError`, never a panic.
- **Asymptote:** `\catcode`, `\csname…\endcsname` → `Unsupported` node with a correct span.
- **Frontend conformance (L6):** passes PFE01's shared harness; `latex"\frac{a}{b}"`-style
  math lifts to the expected `MathExpr`.

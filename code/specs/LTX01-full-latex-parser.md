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
    Matrix { env: String, rows: Vec<Vec<MathNode>> },
}
```

Every node records a byte span. The parser is **total and panic-free**: malformed input
yields a spanned `ParseError`, never a panic.

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
    and the alignment environments (`aligned`/`gathered`/`align`/`align*`/`split`) parsed
    inside math islands via `&` (columns) and `\\` (rows) → `MathNode::Matrix { env, rows }`,
    with `to_latex` round-trip, nesting, and postfix scripts. Empty cells are a documented
    limitation (spanned error). Implemented in the `latex` crate (math.rs).
  - **L3b (later) — document-mode tables & lists.** Row/column structure for `tabular`/`array`
    (which take a mandatory column-spec argument) and list environments
    (`itemize`/`enumerate`/`description`) with `\item`, operating on the document `Node` tree.
    Deferred because they need an extra column-spec field on the node; an unknown
    `\begin{…}` is rejected with a spanned error in the meantime, never mis-parsed.
- **L4 — macros.** `\newcommand`/`\renewcommand`/`\def` with `#1`..`#9` and one optional
  arg with default; expansion of user-defined and a built-in starter set. Recursion/loop
  guard (bounded expansion depth → `Unsupported`/error, never hang).
- **L5 — text-mode breadth.** Sectioning (`\section` …), font/style commands, text accents
  (`\'e`, `\"o`, `\~n`), `\verb`/`verbatim`, cross-refs (`\label`/`\ref`/`\cite` as nodes),
  `\documentclass`/`\usepackage` parsed structurally.
- **L6 — `math-frontend` adapter.** Implement `MathFrontend` for `latex`: lift `Math`/`MathNode`
  subtrees to the neutral `MathExpr`; declare capabilities; register in the builtin registry.
  (This is where LaTeX becomes a *plugin*; [PFE01](PFE01-pluggable-parser-frontends.md)'s
  `math-frontend` crate can land independently before this.)

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

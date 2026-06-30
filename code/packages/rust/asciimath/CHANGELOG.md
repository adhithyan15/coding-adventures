# Changelog — asciimath

All notable changes to the AsciiMath pluggable frontend.

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

# Changelog — math-frontend

All notable changes to the pluggable parser-frontend framework.

## [0.7.0] — 2026-06-30

### Added — fence delimiters as data (`MathExpr::Fenced`)

- New neutral node **`MathExpr::Fenced { open: String, body, close: String }`** — a delimited
  group that carries *which* delimiters bracketed it, so `|x|` (absolute value / norm) is no
  longer indistinguishable from `(x)`. Where `MathExpr::Group` records only *that* a subexpression
  was parenthesised (style dropped), `Fenced` preserves the surface open/close strings (`"("`,
  `"["`, `"\\{"`, `"\\langle"`, `"|"`, `"."` for an omitted `\left.`). Kept distinct from
  `Sequence` (a comma list) and `Matrix` (rows): `Fenced` brackets a single inner expression.
- New capability flag **`Capabilities::fenced_delimiters`** (+ `with_fenced_delimiters()`), wired
  into `all()`, the conformance `collect_used` walker, and the honesty check (`over_emitted`), so a
  frontend emitting `Fenced` without declaring it is flagged — the gate polices the new node.
- The iterative `Drop` for `MathExpr` handles `Fenced` (deep fence nesting frees without overflow).

This is the first slice of the fence-delimiters arc; individual frontends adopt `Fenced` one at a
time (latex first, in this release). Carrying delimiters on comma-list `Sequence` bodies is a later
slice.

## [0.6.0] — 2026-06-30

### Added — neutral `Sequence` node (comma-separated lists in fences)

Comma-separated sequences in fence contexts (`(a, b, c)` — MathML `<mfenced>` with
separators, coordinate tuples, argument lists) previously had **no faithful neutral
representation**: a frontend had to drop the commas or model the list as implicit
multiplication. This release adds the node so any frontend can lower a list honestly.
**Additive**; no existing variant or API changed — the same shape as the `Overset` node in
0.5.0.

- **`MathExpr::Sequence(Vec<MathExpr>)`** — an ordered list of expressions separated by
  commas. The items are preserved in source order; a faithful renderer shows them as a
  delimited list, not a product. Kept **distinct from nested `Bin(Mul, …)`**: juxtaposition
  is implicit multiplication, while a sequence is a deliberate list structure. An empty
  sequence is not representable (a fence with no items is not a list).
- `Capabilities` gains **`sequences`** (set by `all()`, with a `with_sequences()` builder)
  and the conformance harness now polices it: a frontend that emits a `Sequence` but does
  not declare `sequences` is flagged, exactly as for every other node-bearing capability.
- The iterative `impl Drop for MathExpr` (heap worklist) gains an arm for the new node, so a
  deep `Sequence` spine frees in O(1) stack depth — a 300 000-deep nesting drops in a test
  without overflow.
- Downstream unaffected: `latex`, `asciimath`, and the `adj-lang` adapter all build unchanged
  (frontends only *construct* `MathExpr`). The `mathml` crate consumes the node in the same
  release to emit comma-separated `<mfenced>` lists.

## [0.5.0] — 2026-06-29

### Added — neutral `Overset` / `Underset` nodes (unblock stacked annotations on every frontend)

Over/under-set annotations (`\overset{a}{b}`, `\stackrel{a}{R}`, `\underset{a}{b}`; AsciiMath
`overset`/`underset`/`stackrel`) previously had **no faithful neutral representation** — a frontend
had to drop them or misuse `Pow`/`Subscript`. This release adds the nodes so any frontend (LaTeX,
AsciiMath, future MathML/Unicode-math) can lower a stacked annotation honestly. **Additive**; no
existing variant or API changed — the same shape as the `Accent` node in 0.4.0.

- **`MathExpr::Overset { over: Box<MathExpr>, base: Box<MathExpr> }`** and
  **`MathExpr::Underset { under: Box<MathExpr>, base: Box<MathExpr> }`** — generalise `Accent`: where
  an accent's mark is a fixed named diacritic, an `Overset`'s `over` is a *full sub-expression* (an
  arrow label, a limit annotation, a reaction condition, …). Kept **distinct from `Pow`/`Subscript`**:
  the annotation is *centered* above/below the base, not raised/lowered, and a faithful renderer must
  stack it.
- `Capabilities` gains **`oversets`** (set by `all()`, with a `with_oversets()` builder) and the
  conformance harness now polices it: a frontend that emits an `Overset`/`Underset` but does not
  declare `oversets` is flagged, exactly as for every other node-bearing capability.
- The iterative `impl Drop for MathExpr` (heap worklist) gains arms for both new nodes, so a deep
  `Overset`/`Underset` spine frees in O(1) stack depth — a 300 000-deep alternating spine is dropped
  in a test without overflow.
- Downstream unaffected: `latex` (declares `all()`), `asciimath`, and the `adj-lang` adapter all build
  unchanged — frontends only *construct* `MathExpr`, and the adapter routes unknown nodes through its
  catch-all. No emitter ships in this PR; the node is the prerequisite (same staging as `Accent`),
  consumed by a later `asciimath`/`latex` `overset`/`underset`/`stackrel` emission PR.

## [0.4.0] — 2026-06-29

### Added — neutral `Accent` node (unblocks accents on every frontend)

Diacritical accents (`\hat{x}`, `\bar{y}`, `\vec{v}`, `\tilde{a}`, `\dot{x}`, …) previously had
**no faithful neutral representation** — frontends had to drop them or fake them as a function
call. This release adds the node so any frontend (LaTeX, AsciiMath, future MathML/Unicode-math)
can lower accents honestly. Additive; no existing variant or API changed.

- **`MathExpr::Accent { accent: String, body: Box<MathExpr> }`** — `accent` is the canonical
  accent name (`"hat"`, `"bar"`, `"vec"`, …) as a `String`, so the open-ended LaTeX/AsciiMath
  accent set needs no enum churn. Kept **distinct from `Call`**: `Accent{accent:"hat", x}` is a
  diacritic *over* `x`, not the named function `hat(x)` — a faithful renderer must reproduce the
  mark. The iterative `Drop` (heap worklist) gained an `Accent` arm, so deep accent spines free
  without overflowing.
- **`Capabilities::accents`** flag + `with_accents()` builder + included in `all()`. The shared
  conformance harness now polices it: a frontend that emits an `Accent` but doesn't declare
  `accents` is flagged (`over_emitted` extended to 12 capabilities), exactly like ± / binomials.
- Tests: `Accent` constructible / distinct-from-Call / deep-drop (300k spine); conformance
  `emitting_accent_without_declaring_is_flagged` + `declaring_accents_admits_accent`.
- Downstream `latex` (declares `Capabilities::all()`) and `asciimath` recompile unchanged — they
  only *construct* `MathExpr`; `adj-lang`'s adapter has a catch-all arm so an `Accent` (not
  computable arithmetic) correctly reports "unsupported ADJ arithmetic subset". This is the
  prerequisite for the deferred latex/asciimath accent-emission PRs.

## [0.3.0] — 2026-06-28

### Fixed — iterative `Drop` for `MathExpr` (stack-overflow / abort on deep trees)

- **`MathExpr` now drops iteratively.** A frontend can produce a very deeply-nested tree
  from small input — a left-associative chain `a + a + a + …` (or juxtaposition, or
  `1/1/1/…`) parses, by design, into `Bin(Add, Bin(Add, …))` nested N deep. The compiler's
  default recursive destructor would then overflow the stack and **abort the process** when
  that tree is dropped — an uncatchable failure reachable from every frontend's panic-free
  `parse` on adversarial-but-tiny input (≈100 KB). The new `impl Drop for MathExpr`
  dismantles the tree with an explicit heap worklist (moving each node's boxed children out
  in place), so freeing is O(1) in stack depth at any tree depth.
- Fixes the issue for **all** frontends at the source (surfaced by the `asciimath` PR-1
  security review; `latex` and any future frontend benefit too).
- +2 regression tests drop 300k-deep `Bin` and 100k-deep `Root`/`Matrix` trees without
  overflow. Behaviour-only change (no API change); additive. Crate 0.2.0 → 0.3.0.

## [0.2.0] — 2026-06-27

### Added — neutral-AST coverage for ± / ∓ and binomials

Closes the two honest gaps the LaTeX frontend (LTX01 L6) had to error on because the neutral
AST could not represent them:

- **`BinOp::PlusMinus` / `BinOp::MinusPlus`** — the `±` / `∓` operators (`a ± b` denotes the
  pair {a+b, a−b}; `∓` the opposite pairing). Meaning-bearing binary operators, not
  presentation.
- **`MathExpr::Binom(n, k)`** — a binomial coefficient "n choose k", distinct from `Frac`
  (no division bar).
- **`Capabilities`** gains `plusminus` and `binomials` flags (set by `all()` and the new
  `with_plusminus()` / `with_binomials()` builders); the conformance harness's
  `collect_used`/`over_emitted` now detect and police both, so a frontend emitting ± or a
  binomial without declaring it is flagged (verified by new over-claimer tests).
- Backward-compatible additive enum/struct changes (no removals). Downstream `latex` builds and
  its 136 tests pass unchanged; the `latex` frontend will start *emitting* these in a follow-up.
- +3 tests; **23 unit + 1 doc test** green; clippy `-D warnings` clean. Crate 0.1.0 → 0.2.0.

## [0.1.0] — 2026-06-26

### Added — PFE01 implementation: the framework

- New standalone, **zero-dependency** crate `math-frontend` (added to the Rust workspace
  members). The shared substrate for plugging in math-notation parsers.
- **`MathExpr`** — the notation-agnostic neutral AST: `Number`, `Symbol`, `Bin` (`Add Sub
  Mul Div Pow`), `Unary`, `Frac`, `Root`, `Call` (named `Func`), `BigOp` (with bounds),
  `Subscript`, `Rel`, `Group`, `Text`, `Matrix`. Presentation-only distinctions normalize
  away (`\times`/`\cdot`/juxtaposition → `Mul`).
- **`Number`** — **exact-preserving** numeric literal: parses decimal numerals (sign,
  integer/fraction, `e`-exponent) into a normalized `±digits×10^exp` triple, so `1`/`1.0`/
  `01`/`1e0` compare equal and `0.1` is never silently rounded. Keeps the written form;
  `to_f64()` is explicit and lossy. Zero is canonical (never `-0`). No big-int dependency.
- **`MathFrontend`** trait (total, panic-free, pure) + **`FrontendError`** (spanned, names
  the frontend) + **`Capabilities`** (builder over the constructs a frontend can emit).
- **`FrontendRegistry`** — name-keyed install/lookup/parse; unknown frontend yields a
  spanned error listing the installed ones (never a panic); `with_builtins()` is empty by
  design (LaTeX, the first frontend, registers here once its crate lands).
- **`check_frontend`** — shared conformance harness enforcing the contract: parsing never
  panics (`catch_unwind`), errors are well-formed (correct frontend name + in-range,
  non-inverted span), and capabilities are honest (a frontend may not emit a construct it
  didn't advertise).
- 19 unit tests + 1 doc test; `cargo clippy -- -D warnings` clean; no `unsafe`.

### Notes

- Parsing only — evaluation/lowering is a consumer concern. The LaTeX frontend (full
  LaTeX, per LTX01) and any consumer wiring (e.g. an ADJ `latex"…"` literal) are separate
  efforts that depend on this crate.

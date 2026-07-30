# HML01 — Math Languages → Semantic IR → any backend

## Status

Active roadmap spec. Extends
[`HML00`](HML00-historical-math-languages-roadmap.md) (the historical
math-languages effort — Macsyma/Maxima, MATLAB/Octave, Wolfram, and future
APL/J/Reduce/Derive/Maple/…) with a second compilation target: the
narrow-waist [Semantic IR](SIR10-narrow-waist-semantic-ir.md) already used to
translate Twig, Python, Ruby, and JavaScript to each other and to
TypeScript/Rust/Go.

This spec does not replace anything HML00 built. Every math language keeps
its existing native pipeline — `<lang>-lexer` → `<lang>-parser` →
`<lang>-runtime` (+ REPL) — for interactive/REPL use exactly as today. This
spec adds a **second, independent** lowering off the same parse tree:
`<lang>-to-semantic-ir`, so a math-language program can *also* be compiled to
any SIR backend (JavaScript today; TypeScript/Rust/Go/Python for free,
subject to each backend declaring the new features — see §4).

```
                              ┌─→ <lang>-runtime (+ repl)      [existing, unchanged]
<lang> source → GrammarASTNode┤
                              └─→ <lang>-to-semantic-ir → semantic_ir::Module → any SIR backend   [NEW]
```

## §1 Why this is additive, not a rewrite

Every math-language parser in this repo (`matlab-parser`, `wolfram-parser`,
`macsyma-parser`, and future ones) already emits the same generic
`GrammarASTNode` concrete syntax tree that `python-to-semantic-ir` walks
today (rule-name-tagged nodes with ordered children — see
`parser::grammar_parser::GrammarASTNode`). So `<lang>-to-semantic-ir` is a
**second consumer** of an artifact that already exists; it requires no
changes to any `<lang>-lexer` or `<lang>-parser` crate.

What *is* new: the Semantic IR itself has no math vocabulary. It models
closures, calls, and blocks for general-purpose languages — no matrices, no
N-D arrays, no symbolic expression trees, no pattern-matching/rewrite-rule
semantics. Two sibling specs add that vocabulary, additively, exactly the way
[`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md) added loops and
sequences for Python/JS without breaking Twig:

- [`SIR22`](SIR22-array-matrix-semantic-ir.md) — the numeric/array domain
  (N-D arrays, matrix ops, ranges, indexing), for MATLAB/Octave and every
  future array-family language (APL, J, Scilab, IDL, …).
- [`SIR23`](SIR23-symbolic-pattern-semantic-ir.md) — the symbolic/CAS domain
  (symbolic expression application, patterns, rewrite rules), for
  Wolfram/Macsyma/Maxima and every future symbolic-CAS-family language
  (Reduce, Derive, Maple, …).

These two domains are independent — different `Expr`/`SirType`/`Feature`
additions, different frontends, different backend runtime packages — and are
designed and shipped in parallel streams (§5).

## §2 The per-language pattern, amended

[`HML00` §6](HML00-historical-math-languages-roadmap.md#6-the-per-language-pattern)
described:

```
spec → grammar/lexer/parser → <lang>-runtime (+ repl) → <lang> binary
```

Every **future** math-language item (APL, J, Reduce, Derive, Maple, …) adds
one more stage to that pattern, built alongside the runtime rather than
bolted on afterward:

```
spec → grammar/lexer/parser → <lang>-runtime (+ repl) → <lang> binary
                              → <lang>-to-semantic-ir
```

`<lang>-to-semantic-ir` is a thin, separate crate (mirroring
`python-to-semantic-ir`'s shape — see §3) that walks the same
`GrammarASTNode` the runtime already parses and emits a `semantic_ir::Module`
instead of evaluating. It shares no code with `<lang>-runtime` beyond the
parser; the two are independent consumers of one parse tree, exactly as
`python-to-semantic-ir` and CPython's own interpreter both consume Python
source without depending on each other.

## §3 Frontend recipe (retrofit for MATLAB/Octave/Wolfram/Macsyma/Maxima)

Each new frontend crate follows the established
`compile`/`compile_source` shape (`python-to-semantic-ir`'s public API):

```rust
pub fn compile(tree: &GrammarASTNode, module_name: &str)
    -> Result<semantic_ir::Module, <Lang>LowerError>;
pub fn compile_source(source: &str, module_name: &str)
    -> Result<semantic_ir::Module, <Lang>LowerError>;
```

- **`matlab-to-semantic-ir`** (✅ v0.1.0 shipped) — walks the `matlab-parser`
  CST, emits SIR22 (array/matrix domain) nodes. 1-based-to-0-based index
  translation happens here, at lowering time — per SIR10's "disambiguation
  is the frontend's job," the IR never carries a language's indexing
  convention implicitly. A well-scoped first cut (literals, assignment,
  arithmetic/comparison/logical operators, ranges, transpose, indexing,
  `if`/`while`/`for`, single-output functions, `disp`) rather than full
  MATLAB — each excluded construct (stepped/matrix-valued `for`,
  `end`-relative indexing, matrix division, multi-output functions, nested
  functions, `break`/`continue`/`return`, `switch`/`try`/`global`, cell
  arrays, lambdas) returns an explicit error rather than being silently
  mis-lowered. Scalar-vs-array disambiguation for `+ - * / \ ^` is a
  syntactic heuristic on literal operands only (real shape inference is a
  follow-up), which means only *purely-literal* MATLAB arithmetic currently
  round-trips through the JS backend (SIR22 codegen for the array-domain
  nodes is separate, not-yet-shipped follow-on work — the frontend/backend
  split described in this spec).
- **`octave-to-semantic-ir`** (✅ v0.1.0 shipped) — a thin wrapper: run
  `octave-runtime`'s existing `octavify` source-rewrite shim, then delegate
  to `matlab-to-semantic-ir::compile_source`. Mirrors how `octave-runtime`
  itself reuses `matlab-runtime` wholesale today (no new grammar, no new
  SIR node kinds). Unlike this section's own sketched `compile`/
  `compile_source` shape above, this crate exposes **only**
  `compile_source` — there is no Octave-specific CST to hand a `compile`
  entry point, since the shim rewrites source *text* before anything is
  parsed; the only tree ever built is the MATLAB one
  `matlab-to-semantic-ir::compile_source` constructs internally. 9 tests
  cover every normalized construct (comments, all six `endX` terminators,
  `!=`/`!`), a string-awareness regression (the shim must not rewrite `#`/
  `!` inside a string literal), a plain-MATLAB-passthrough sanity check,
  and error propagation for both an out-of-scope MATLAB construct and an
  Octave-only construct `octavify` does not normalize (`do...until`).
- **`wolfram-to-semantic-ir`** (✅ v0.1.0 shipped) — walks the `wolfram-parser`
  CST, emits SIR23 (symbolic/pattern domain) nodes. Reuses the existing
  surface-to-head desugaring table already in `wolfram-runtime/src/lower.rs`
  (`+`→`Add`, `*`→`Mul`, etc.) but targets `semantic_ir::Expr` instead of
  `symbolic_ir::IRNode`. Adopts one design decision beyond what this section
  originally sketched: **every** Wolfram construct — not just patterns and
  rules — lowers to symbolic *data* (`SymApply`/`SymSymbol`), including
  ordinary arithmetic and `=`/`:=` assignment; there is no host-language
  variable binding at all in this frontend's output (see `src/lower.rs`'s
  module doc comment for why this is necessary, not just convenient, to
  compile an *uncomputed* function body like `f[x_] := x + 1`). One
  consequence: because every Wolfram program, even bare literal arithmetic,
  therefore emits at least one SIR23 node, no lowered module executed
  end-to-end through any backend until `sir-runtime-symbolic` and its JS/TS
  codegen shipped (Stream B rollout items 6-7, below) — unlike
  `matlab-to-semantic-ir`'s purely-literal subset, which could from the
  start. Both now round-trip through `node`; see `tests/e2e_node.rs`.
  Covers the full grammar `wolfram-parser` accepts (the W-6/W-11/W-21
  operator sugar included), since nothing here forces a MATLAB-style
  narrower cut.
- **`macsyma-to-semantic-ir`** (✅ v0.1.0 shipped) — walks the
  `macsyma-parser` CST using the same rule-name dispatch already proven in
  `macsyma-compiler` (`"assign"`, `"additive"`, `"postfix"`, …), emits SIR23
  nodes. Same "everything is symbolic data" design as
  `wolfram-to-semantic-ir` (this grammar has no pattern-matching syntax at
  all, so only the arithmetic/assignment/control-flow/function-call SIR23
  shapes are exercised); also round-trips through `node`.
- **`maxima-to-semantic-ir`** (✅ v0.1.0 shipped) — a thin alias reusing
  `macsyma-to-semantic-ir` wholesale, mirroring Maxima's existing reuse of
  `macsyma-runtime`.
- **`derive-to-semantic-ir`** (✅ v0.1.0 shipped) — walks the
  `derive-parser` CST using the same rule-name dispatch already proven in
  `derive-runtime` (`"assignment"`, `"additive"`, `"postfix"`, `"vector"`,
  …), emits SIR23 nodes. Same "everything is symbolic data" design as
  `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir` (this grammar has no
  pattern-matching syntax at all either, verified directly against
  `derive.grammar`/`derive.tokens`, not just trusted from
  `derive-runtime`'s own doc comment). Much thinner than either sibling:
  no `f[x]`-universal-application syntax and no control-flow grammar
  productions at all (`IF(…)` is an ordinary UPPERCASE builtin call, not a
  special `if_expr` rule the way Macsyma has one), but needs a BIGGER
  surface→canonical head-bridge table than Wolfram's, since Derive's
  built-ins are conventionally UPPERCASE and `SymSymbol` equality is
  case-sensitive. Also the first SIR23 frontend with a vector/matrix
  literal (`[a,b,c]`/`[a,b;c,d]`, structural `List` data only); also
  round-trips through `node` from v0.1.0 (unlike `macsyma-to-semantic-ir`,
  which shipped its `tests/e2e_node.rs` in a follow-up once the JS backend
  gained SIR23 codegen).
- **`reduce-to-semantic-ir`** (✅ v0.1.0 shipped) — walks the
  `reduce-parser` CST using the same rule-name dispatch already proven in
  `reduce-runtime` (`"assignment"`, `"additive"`, `"postfix"`, `"cons"`,
  `"if_expr"`, `"group_expr"`, …), emits SIR23 nodes. Same "everything is
  symbolic data" design as the other three (this grammar has no
  pattern-matching syntax in this subset either, verified directly
  against `reduce.grammar`/`reduce.tokens`, not just trusted from
  `reduce-runtime`'s own doc comment). Much of the lowering is a direct
  copy of `derive-to-semantic-ir`'s shape (`reduce-runtime`'s own doc
  comment says so), but Reduce's grammar has three constructs no Derive
  analogue exists for: an expression-shaped `if`, a `<< s1; s2; ... >>`
  group statement (`CompoundExpression`), and `.` cons — plus flat
  curly-brace lists (`{a,b,c}`, unlike Derive's row-counting
  `[a,b;c,d]`). Confirms, and reuses, a REAL divergence MA08 §3's own
  prose has from the actual shared IR: arithmetic lowers to the same
  `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg` heads Derive/Macsyma use, not the
  literal (and non-existent in `symbolic-ir`) `Plus`/`Subtract`/`Times`/
  `Power` MA08 §3's table spells out — a disclosed, deliberate divergence,
  not new-head invention. Also reuses `reduce-runtime`'s disclosed gap
  that `CompoundExpression`/`First`/`Second`/`Third`/`Rest`/`Part`/
  `Append`/`Reverse`/non-folding `Cons` have no evaluation handler in the
  shared `symbolic-vm` — moot for this frontend (it never evaluates
  anything), confirmed structurally accepted by the JS backend's
  head-name-agnostic `SymApply` codegen regardless. Also round-trips
  through `node` from v0.1.0.
- **`maple-to-semantic-ir`** (✅ v0.1.0 shipped) — walks the
  `maple-parser` CST using the same rule-name dispatch already proven in
  `maple-runtime` (`"assignment"`, `"arrow_def"`, `"if_expr"`,
  `"postfix"`, `"set_literal"`, …), emits SIR23 nodes. Same "everything is
  symbolic data" design as the other four (this grammar has no
  pattern-matching syntax in this subset either, verified directly
  against `maple.grammar`/`maple.tokens` — the `ARROW` token exists but
  only for `arrow_def`, never a pattern-rule arrow — not just trusted
  from `maple-runtime`'s own doc comment). Much of the lowering is a
  direct copy of `reduce-to-semantic-ir`'s shape (both languages are
  "surface operators + `head(args)` calls" with no pattern vocabulary),
  but `maple.grammar` draws a REAL structural line Reduce's grammar does
  not: `statement = if_expr | assignment` sits in its own nonterminal,
  never reachable from `expr` at all, so `if`/`:=` can never nest inside
  an arithmetic/comparison/logical operand the way Reduce's can
  (`x := if a then 1 end if;` and `a := b := c;` are both syntax errors).
  Assignment's left-hand side is a bare `NAME` token only — Maple's
  `f(x) := expr` means a narrower remember-table patch in real Maple
  (MA09 §1/§4), so the grammar rejects it outright rather than pushing an
  interpretation question onto this frontend — general function
  definition instead uses a dedicated `arrow_def`/`arrow_params`
  production (`f := (x, y) -> x + y`) lowering to the same `Define` shape
  Derive/Reduce use. `if`/`elif`/`else` right-folds like Macsyma's own
  elif chain (unlike Reduce's simpler 2-or-3-child `if`), guarded by a
  new `check_elif_chain_length`. Introduces `Set` (`{a,b,c}`, unordered)
  as a canonical head genuinely new to this repo — Maple is the first
  language here with two distinct bracketed aggregate literals (`[a,b,c]`
  → shared `List`, `{a,b,c}` → new local `Set`) — plus the first literal
  `true`/`false` boolean TOKENS in this CAS family (bridged to the shared
  backend's pre-bound `True`/`False` symbols). `postfix` is deliberately
  NOT chainable (`f(x)(y)` fails to parse), so this is the first SIR23
  frontend needing no `check_postfix_chain_length`-equivalent guard at
  all. Also round-trips through `node` from v0.1.0. **This closes Stream
  B's previously-tracked language list** — every math-CAS language this
  spec names (Wolfram, Macsyma, Maxima, Derive, Reduce, Maple) now has a
  shipped SIR23 frontend.
- **`apl-to-semantic-ir`** (✅ v0.1.0 shipped, MA-4f) — the first Stream A
  frontend beyond MATLAB/Octave, confirming the array/matrix domain
  generalizes past the language it was designed against; emits SIR22 nodes
  plus the APL-primitive `Expr`/`ElementwiseOpKind` additions (SIR22
  addendum).
- **`j-to-semantic-ir`** — the second array-family frontend beyond
  MATLAB/Octave/APL, sharing the same SIR22 vocabulary.

## §4 Backend recipe: extend the existing JS/TS backends, don't fork

`semantic-ir-to-javascript` and `semantic-ir-to-typescript` gain new `match`
arms for the SIR22/SIR23 node kinds and declare the corresponding `Feature`s
in `accepts_features()`. New runtime behavior ships as new npm packages,
following the existing import-header precedent (`sir-runtime-core`,
`sir-runtime-oop` — pasted as an `import` line, not inlined):

- **`sir-runtime-array`** — a thin `Float64Array` wrapper with explicit
  column-major indexing, matmul, elementwise ops, broadcasting, and range
  materialization. Backs SIR22.
- **`sir-runtime-symbolic`** — a small term-rewriting engine (term-tree
  representation, structural equality, the Blank/Pattern matcher,
  substitution, and the `ReplaceAll`/`ReplaceRepeated` fixed-point loop) —
  a direct port of `cas-pattern-matching`'s existing algorithm to
  JS/TS, not a reimplementation from scratch. Backs SIR23.

Emit logic gates each import on the module's manifest exactly like the
existing `uses_oop`/`uses_exceptions` checks: a pure-numeric MATLAB module
never imports the symbolic runtime and vice versa.

Other backends (Rust/Go/Python) are **not required** to support SIR22/SIR23
in this first wave — per SIR10's backend-capability model, a backend that
doesn't declare a feature in `accepts_features()` cleanly rejects a module
that needs it, rather than emitting wrong code. JS/TS is the first target
because it's what motivated this work; other backends can add support later
without any change to the IR or the frontends.

## §5 Rollout — two parallel streams, one shared serialization point

**Stream A (array/matrix, backs MATLAB/Octave/APL/J/Scilab/Q/IDL):**
`SIR22` spec → `semantic-ir` core additions → `matlab-to-semantic-ir` →
`octave-to-semantic-ir` → `sir-runtime-array` → JS/TS backend codegen →
`apl-to-semantic-ir` (SIR22 addendum for APL primitives) →
`j-to-semantic-ir` → `scilab-to-semantic-ir` → `q-to-semantic-ir` →
`idl-to-semantic-ir` → golden/oracle tests
(MATLAB's, Octave's, APL's, J's, Scilab's, Q's, and IDL's own now shipped.
MATLAB — see
`matlab-to-semantic-ir/tests/oracle.rs`, the first
true oracle diff anywhere in this track: the same computation run through
`matlab-runtime` and through this frontend's compiled-JS-via-`node` path,
asserted equal, for a 7-case corpus spanning literal arithmetic,
comparisons, `if`/`elseif` branching, a `for`-loop accumulator, and two real
SIR22 array/matrix cases (matrix multiplication, elementwise scalar
broadcast). Building it also surfaced several confirmed bugs/gaps — an
integer-literal-division bug (still open), a unary-minus-on-power bug (now
fixed), a missing `Feature::ShortCircuit` declaration (now fixed), a severe
`while`-loop-accumulator correctness bug (now fixed), and two
`matlab-runtime` gaps (no function-definition support, no
indexed-assignment support) — see that crate's `CHANGELOG.md` for the full
write-up. Octave — see `octave-to-semantic-ir/tests/oracle.rs`, the direct
sibling of MATLAB's oracle file (same `Case`/`ground_truth`/`compiled`
shape), with a 6-case corpus deliberately restricted to Octave-only syntax
`octavify` actually rewrites rather than re-testing plain MATLAB arithmetic:
a `#` comment, `!=`, `!` (applied to a parenthesized comparison, not a bare
numeric variable — see below), and the `endif`/`endfor`/`endwhile` block
terminators, including a direct regression check (via `endwhile`) that the
now-fixed while-loop-accumulator bug stays fixed through Octave syntax too.
All 6 cases pass; building it also confirmed one more gap, inherited from
`matlab-to-semantic-ir`: negating a bare numeric variable via `!`/`~`
disagrees with Octave's "logicals are doubles, 0 is false" semantics,
because SIR's shared `truthy()` runtime helper treats only `false`/`nil` as
falsy (a Ruby/Lisp convention) — `CORPUS` sidesteps this by negating a
comparison instead, and the gap is recorded in that crate's `CHANGELOG.md`
for a follow-up. APL — see `apl-to-semantic-ir/tests/oracle.rs`, its own
distinct 17-case corpus (not just MATLAB's reused), which also surfaced 3
new bugs in monadic `- × ÷ ⌈ ⌊` (a wrong display glyph, a wrong value on
array operands, and a hard crash on `× ÷ ⌈ ⌊`) — all three now fixed, in
`semantic-ir-to-javascript` 0.43.0, see that crate's `CHANGELOG.md`. J —
see `j-to-semantic-ir/tests/oracle.rs`, its own 36-case corpus, which
fixed two bugs genuinely local to that frontend's own lowering (stranded
literals missing the same rank-1 `Ravel`-wrap fix APL's own lowering
already had, and monadic/dyadic `i.` silently inheriting APL's 1-based
`IndexGenerator`/`IndexOf` convention instead of J's own 0-based one with
a plain-tally not-found sentinel) directly in this PR, and found two more
bugs in the shared `semantic-ir-to-javascript` crate itself, deliberately
left open for a follow-up PR (no J-specific display convention for
negative numbers/infinity at all — only APL's high-minus glyph is wired
up — and three of J's own builtin names, `tally`/`replicate`/`exp`, never
registered in that crate's dispatch table) — see that crate's
`CHANGELOG.md` for the full write-up. Scilab — see
`scilab-to-semantic-ir/tests/oracle.rs`, its own 33-case corpus (`setup`
+ `final_expr` `Case` shape, structurally closer to MATLAB's/Octave's own
oracle files than to the CAS-family ones, since `scilab-runtime`'s `disp`
is a no-op and the only working ground-truth display convention is the
unsuppressed `name = value` echo — but also carrying the CAS-family
files' `known_bug` field, since this crate turned out to have six
genuinely worth-tracking findings). 27 of 33 cases agree end-to-end with
no marker at all. Three findings are already-open, shared-crate
display-convention gaps confirmed to also affect Scilab (not
independently reintroduced): the whole-valued-float-literal trailing
`.0` bug, `Inf`/`eps` number-formatting divergence (Rust `Display` vs.
JS `Number.prototype.toString`), and the still-open integer-literal-
division-floors bug MATLAB's own oracle file already documents. One
finding is a genuine, newly-discovered BUG in the shared
`semantic-ir-to-javascript` crate, NOT fixed in the Scilab PR: `matmul`
reads `a.shape`/`b.shape` unconditionally with no `toArrayValue`
normalization step first (unlike its sibling `elementwise`, which
normalizes both operands first), so any `x * y` between two non-literal
operands that turn out to be plain scalars — not array literals —
crashes at runtime (`TypeError: Cannot read properties of undefined
(reading 'length')` at `nrows`). This was only reachable via Scilab's
oracle harness because `scilab-runtime`, unlike `matlab-runtime`,
supports user-defined functions, giving a function-parameter
self-multiplication repro shape MATLAB's own oracle file could never
have exercised even if it had tried — see that crate's `CHANGELOG.md`
for the full write-up and the dedicated follow-up task filed against
`semantic-ir-to-javascript`'s `matmul`. Q — see
`q-to-semantic-ir/tests/oracle.rs`, its own 50-case corpus (no
`setup`/`final_expr` split needed — like J's and APL's own oracle files,
Q's silent-assignment/auto-print convention lets every case be one whole
`source` string). Building it reconfirmed the exact same shared-crate
display gap `j-to-semantic-ir`'s own oracle file first found —
`semantic-ir-to-javascript` printing APL's high-minus `¯` glyph for any
negative `NDArray` result regardless of source language, with no
per-language flag gating it to Q's own plain-ASCII-minus convention —
except this time it was fixed as a direct follow-up (task #109 added a
third, mutually exclusive `SIR_DISPLAY_Q_ASCII_MINUS` display flag
alongside the existing APL/J ones), so all 50 cases now agree end-to-end
with `known_bug: None` throughout. IDL — see
`idl-to-semantic-ir/tests/oracle.rs`, its own 32-case corpus (the
`setup`/`final_expr`/`expected`/`known_bug` `Case` shape borrowed
directly from Scilab's own oracle file, adapted to `idl-runtime`'s
Implied-Print convention). All 32 cases agree end-to-end with
`known_bug: None` throughout — the cleanest pass of any Stream A oracle
file so far — modulo one explicitly-disclosed testing-scope limitation,
not a bug: this frontend has no in-scope way to construct a genuine
rank-2 array literal, so every `#`/`##` (matmul) case necessarily uses
commuting rank-1 operands, meaning the file can confirm the VALUE agrees
but cannot independently prove `matmul`'s operand-order fix through
end-to-end values alone (that proof lives in `tests/test_lower.rs`'s own
`hash_is_matmul_with_operands_swapped` structural assertion instead). All
items through JS/TS backend codegen are shipped.

**Stream B (symbolic/CAS, backs Wolfram/Macsyma/Maxima/Derive/Reduce/
Maple):** `SIR23` spec → `semantic-ir` core additions →
`wolfram-to-semantic-ir` → `macsyma-to-semantic-ir` → `maxima-to-semantic-ir`
→ `derive-to-semantic-ir` → `reduce-to-semantic-ir` → `maple-to-semantic-ir`
→ `sir-runtime-symbolic` → JS/TS backend codegen → golden/oracle tests (all
five SIR23 frontends now have both proofs: a real `node`-execution proof
(`tests/e2e_node.rs`, shipped first for `wolfram-to-semantic-ir`,
`macsyma-to-semantic-ir`, `derive-to-semantic-ir`, `reduce-to-semantic-ir`,
and `maple-to-semantic-ir` — Wolfram's and Macsyma's predate the
oracle-testing convention itself, per each of their own `tests/oracle.rs`
module docs), and a true oracle diff against each language's own native
runtime (`tests/oracle.rs`). Derive's, Reduce's, and Maple's oracle diffs
shipped first — see `derive-to-semantic-ir/tests/oracle.rs` (a 38-case corpus
cross-checking `derive-runtime`), `reduce-to-semantic-ir/tests/oracle.rs`
(its own 38-case corpus cross-checking `reduce-runtime`), and
`maple-to-semantic-ir/tests/oracle.rs` (its own 43-case corpus
cross-checking `maple-runtime`), all three against the
compiled-JS-via-`node` path. Building the first two found that comparing
*evaluated* values is currently blocked, for every Stream B frontend alike
(not a `derive-to-semantic-ir`-or-`reduce-to-semantic-ir`-specific gap),
by two gaps in the shared `semantic-ir-to-javascript` crate: its SIR23
codegen constructs a symbolic term tree but never evaluates/simplifies one
(no arithmetic/comparison/calculus folding, no execution of the held
`Assign`/`Define`/`If` forms), and its sole SIR23 stringifier has no
per-source-language display convention (generic `head(args)` only, unlike
the SIR22 array domain's already-per-language-aware `ArrayRt.fmtNum`/
`display`) — now written up once, generally, in
`SIR23-symbolic-pattern-semantic-ir.md`'s own "Addendum — SIR23 symbolic
evaluator + per-language display convention" section (which confirms
`derive-runtime`, `reduce-runtime`, and `maple-runtime` all construct
`SymbolicBackend::new()` completely unchanged, so all three hit this exact
gap for the exact same reason), plus each oracle file's own module doc and
each frontend's `CHANGELOG.md` `0.1.1` entry for the case-by-case
accounting. `reduce-to-semantic-ir/tests/oracle.rs` additionally found a
THIRD, genuinely Reduce-specific gap one layer further back in the
pipeline (already disclosed in MA08 §5 and `reduce-runtime`'s own module
doc, not newly discovered by this oracle PR): `symbolic-vm`'s shared
handler table has no evaluation handler at all for `CompoundExpression`/
the list accessors (`First`/`Rest`/`Append`/…)/a non-folding `Cons`, so
`reduce-runtime` itself already leaves those constructs unevaluated —
for those specific cases the only actual disagreement against the
compiled path is the display-convention gap, not a missing compiled-side
evaluation. `maple-to-semantic-ir/tests/oracle.rs` was built after
`semantic-ir-to-javascript` 0.49.0 landed real arithmetic/comparison/logic
folding (`Symbolic.evalTerm`, found by the `derive-to-semantic-ir`/
`reduce-to-semantic-ir` oracle PRs above and fixed as a follow-up, not by
this Maple PR itself) — so 12 of its 43 cases (bare atoms and pure
arithmetic/identity-law folding) already agree end-to-end, and it
additionally found a FOURTH gap, genuinely Maple-specific and layered on
top of the still-open held-form/calculus/display gaps: a `True`/`False`
CASE mismatch. Every comparison/logic handler in the shared JS backend
folds to the capitalized symbol `True`/`False` with no per-language
case-bridging, which `derive-to-semantic-ir`'s and `reduce-to-semantic-ir`'s
own oracle corpora never noticed because those two languages' own native
printers *also* render `True`/`False` capitalized — but Maple's own
native printer (`maple-runtime::printer`) bridges the shared symbol back
to real Maple's own lowercase `true`/`false` surface (MA09 §3), so even a
comparison/logic case that folds identically on both sides still
disagrees on letter case. All four gaps are recorded via `known_bug`, not
patched in any frontend's own oracle PR, and remain a follow-up item for
`semantic-ir-to-javascript`/`symbolic-vm` themselves.

Wolfram's and Macsyma's own oracle diffs shipped afterward, closing what
was this stream's last open item — see
`wolfram-to-semantic-ir/tests/oracle.rs` (its own 32-case corpus
cross-checking `wolfram-runtime`, 27 of 32 cases `known_bug: None`) and
`macsyma-to-semantic-ir/tests/oracle.rs` (its own 34-case corpus
cross-checking `macsyma-runtime`, 31 of 34 cases `known_bug: None`), both
against the compiled-JS-via-`node` path. Both reconfirm finding one (the
SIR23 JS backend's arithmetic/comparison/logic/held-form folding, by then
shipped in full — `runtime.rs`'s own "all four addendum items now
shipped" comment) and finding five (the still-open, shared,
no-per-language-display-convention gap that needs `known_bug` for any
non-atomic result) rather than re-deriving either. Two gaps are genuinely
new, and both are left open rather than patched in either oracle PR:
Wolfram's own `SetDelayed`/`:=` lowering emits a 2-argument
`Define(f[x_], body)` call (full-fidelity, no lowering-time LHS
destructuring — deliberate, per `lower.rs`'s own "Everything is data"
design), but the shared JS `defineHandler` requires exactly 3 args, so a
Wolfram-defined function is never actually registered on the compiled
side — unlike every other SIR23 frontend's `:=`/`COLONEQ` lowering.
Macsyma's own `:=` lowering already emits the matching 3-argument
`Define(f, List(x), body)` shape (confirmed directly against
`macsyma-to-semantic-ir/tests/test_lower.rs`), so Macsyma needs no
equivalent `known_bug` here — the exact opposite finding. Separately,
Wolfram's `SymReplaceAll` (`/.`/`//.`) performs substitution only on the
compiled side and never re-folds the substituted result through
`evalTerm`, so a rule whose right-hand side needs further arithmetic
reduction after substitution stays unevaluated compiled-side while
`wolfram-runtime` folds it correctly natively — invisible whenever the
substituted result is already atomic (true of most of
`tests/sir23_symbolic.rs`'s own hand-built cases), and only surfaced
while building this corpus. Macsyma's own grammar (v0.1.0) has no
pattern-matching/rewrite-rule surface syntax at all (no `_`/blank, no
`->`/`:>` rule arrow), so there is no `SymReplaceAll` analogue for its
corpus to hit in the first place.

`maxima-to-semantic-ir` deliberately has no oracle corpus of its own, and
this is a documented finding, not a coverage gap —
`macsyma-to-semantic-ir/tests/oracle.rs`'s own "Maxima coverage" section
spells it out: `maxima-to-semantic-ir::src/lib.rs` is a pure re-export of
`macsyma-to-semantic-ir::{compile, compile_source}` (no Maxima-specific
CST is ever built), and `maxima-runtime::MaximaSession` is a thin façade
over `macsyma-runtime::MacsymaSession` whose `feed` forwards straight to
`eval_source` and echoes the exact same `output_text` (rendered by the
same `cas_pretty_printer::MacsymaDialect`) behind only a `(%oN) `
REPL-echo prefix the SIR/JS pipeline never sees either way — unlike
Octave, which needs a genuine `octavify` source-rewrite shim over
`matlab-runtime` for real surface departures. A separate Maxima corpus
would therefore re-run the identical evaluator against the identical
lowering and assert the identical strings — pure duplication, not
additional coverage — so `macsyma-to-semantic-ir/tests/oracle.rs`'s
corpus stands in for both languages; a future Maxima-specific surface
departure would be the trigger to split it, not anything currently
observed.

**All five frontends this stream's own language list names are now
shipped, and so is a true oracle diff for all five** — Wolfram's and
Macsyma's own `tests/oracle.rs` were the last open item and are done as
of those two crates landing. All items through JS/TS backend codegen are
shipped.

**A SIXTH Stream B frontend, Axiom (MA13, HML00 §7 Wave 7), landed after
this stream's original five and closes out with its own oracle diff too**
— see `axiom-to-semantic-ir/tests/oracle.rs`, its own 29-case corpus
cross-checking `axiom-runtime`, 28 of 29 cases `known_bug: None`, against
the compiled-JS-via-`node` path. Axiom is the one CAS-family language in
this repo whose own claim to fame — a fixed, non-extensible domain/category
type system (`:` declares, `::` coerces, `has` queries category
membership, MA13 §2/§3) — has no analogue in Wolfram/Macsyma/Derive/
Reduce/Maple, none of which carry any domain/type concept at all
(`symbolic_ir::IRNode` confirmed to have no such field anywhere, MA13 §2's
own central finding). `axiom-to-semantic-ir` (MA-13e, #9181) lowers `:`/
`::`/`has` as ordinary `SymApply` nodes under three new, locally-defined
reserved head names (`__axiom_declare`/`__axiom_coerce`/`__axiom_has`,
never added to shared `semantic-ir`/`symbolic-ir`) — the same "new
construct, no shared-crate change" pattern `reduce-to-semantic-ir`'s
`CompoundExpression` and `maple-to-semantic-ir`'s `Set` already
established — but, unlike those two (which shipped with no runtime
evaluator and remain that way), this stream's own close-out task wired a
REAL evaluator for all three heads directly into
`semantic-ir-to-javascript`'s shared `Symbolic.evalTerm` dispatch
(`axiomDeclareHandler`/`axiomCoerceHandler`/`axiomHasHandler`, a JS-side
port of `axiom-runtime::domains`'s fixed `AxiomDomain`/`AxiomCategory`
table, plus a sixth, narrow `SIR_DISPLAY_AXIOM_BOOLEAN` display flag —
see that crate's own `CHANGELOG.md` `0.51.6` entry). Confirmed
end-to-end: a passing AND a failing `:` declaration, a passing AND a
failing `::` coercion (byte-for-byte identical error text on both paths
for every atomic operand tested), and the book's own two confirmed `has`
examples (`Polynomial(Integer) has Ring` → `true`,
`List(Integer) has Ring` → `false`). The one `known_bug` case reconfirms
finding five/three's own already-documented class of gap (no
`SIR_DISPLAY_AXIOM` infix/bracket convention for a COMPOUND, non-boolean
value) rather than discovering a new one. Because `axiom.grammar`'s own
`program = expr` design (MA13 §5) means every compiled program is exactly
ONE top-level statement — even a `;`-block, which lowers to a single
`SymApply(CompoundExpression, [...])` node with no evaluator of its own
(the same pre-existing, shared gap `reduce-to-semantic-ir/tests/
oracle.rs`'s own "finding three" already disclosed) — this oracle file
also introduces its own harness-only `wrap_axiom_top_level_for_observation`
helper (test-local, no change to `semantic-ir-to-javascript` itself) that
unrolls a top-level `CompoundExpression` into N statements, printing only
the last, so block-based corpus entries (declare-then-assign,
define-then-call) still get a real value comparison instead of needing
`known_bug` for an unrelated, already-documented gap.

The streams touch disjoint crates except `semantic-ir` core and the two JS
backends — a short serialization point, not a merge of the whole effort.
Each item is one PR, following the repo's established one-PR-per-item
discipline; `/security-review` before every push, `/babysit-pr` after.

This track does not block, and is not blocked by, the existing HML00 backlog
(Wolfram's `cas-*` function-surface item, Macsyma's remaining Frobenius /
general-factoring / hypergeometric-summation gaps, or the not-yet-started
languages) — those continue independently, and per repo direction this
track deliberately **alternates** with that backlog rather than running to
completion in isolation (see §6).

## §6 Sequencing discipline: alternate across three fronts

This track is one of three fronts the math-languages effort runs
simultaneously, rotating rather than exhausting one before starting another:

1. **Existing-language feature completeness** — Wolfram's `cas-*` function
   surface, Macsyma's remaining gaps (general multivariate factoring,
   Frobenius ODEs, hypergeometric summation, TS/Rust `Apart` port).
2. **New math-language additions** — HML00 Wave 4+ has now shipped APL, J,
   K/Q (as `q-to-semantic-ir`), Scilab, IDL, Reduce, Derive, and Maple, and
   HML00 §7 Wave 7 has now shipped **Axiom** too (MA13 — kickoff spec
   #8990, `axiom-lexer` #8997, `axiom-parser` #9022, `axiom-runtime`/
   `axiom-repl` #9055, `axiom-to-semantic-ir` #9181, oracle tests +
   `:`/`::`/`has` JS-runtime wiring closing out Wave 7 — see §5's own new
   "A SIXTH Stream B frontend" paragraph). **Julia remains the one
   genuinely not-yet-started item** on this front (confirmed: no
   `julia-*` crate under `code/packages/rust/` and no `julia` spec under
   `code/specs/`).
3. **This track (Semantic IR compilation).**

A reasonable cadence: land one PR-sized unit here, then pick up one item from
front 1 or front 2, before returning. No front should go quiet for long.

## §7 Verification

- **Oracle/golden testing per language**: run the same source through (a) the
  existing `<lang>-runtime` (ground truth) and (b) `<lang>-to-semantic-ir` →
  JS backend → `node`; diff results. Mirrors the Python/TS/Rust parity
  testing already used across the Macsyma port.
- Extend the existing `sir-conformance` crate's corpus-vs-reference model
  (already built for cross-backend SIR conformance — see its lesson in
  `lessons.md` about builtins silently missing from one backend runtime)
  to cover the new node kinds. Any new builtin name a frontend emits must be
  checked against **every** backend runtime it targets before being assumed
  supported.
- `cargo test --workspace` after any `semantic-ir` core change — the crate is
  shared by every existing frontend/backend; `--lib` alone will not catch
  ripples into `python-to-semantic-ir`, `ruby-to-semantic-ir`, etc.

## §8 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`SIR10`](SIR10-narrow-waist-semantic-ir.md),
[`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md) (the precedent
this spec's extension mechanics follow),
[`SIR22`](SIR22-array-matrix-semantic-ir.md),
[`SIR23`](SIR23-symbolic-pattern-semantic-ir.md).

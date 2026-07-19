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

**Stream A (array/matrix, backs MATLAB/Octave/APL and future J/Scilab/IDL):**
`SIR22` spec → `semantic-ir` core additions → `matlab-to-semantic-ir` →
`octave-to-semantic-ir` → `sir-runtime-array` → JS/TS backend codegen →
`apl-to-semantic-ir` (SIR22 addendum for APL primitives) →
`j-to-semantic-ir` → golden/oracle tests (MATLAB's own and Octave's own now
shipped. MATLAB — see `matlab-to-semantic-ir/tests/oracle.rs`, the first
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
for a follow-up. APL/J's own oracle tests remain open follow-on items —
APL's distinct primitive vocabulary needs its own corpus, not just MATLAB's
reused).
All items through JS/TS backend codegen are shipped.

**Stream B (symbolic/CAS, backs Wolfram/Macsyma/Maxima and future
Reduce/Derive/Maple):** `SIR23` spec → `semantic-ir` core additions →
`wolfram-to-semantic-ir` → `macsyma-to-semantic-ir` → `maxima-to-semantic-ir`
→ `sir-runtime-symbolic` → JS/TS backend codegen → golden/oracle tests
(in progress — real `node`-execution proof shipped for `wolfram-to-semantic-ir`
and `macsyma-to-semantic-ir` via each crate's `tests/e2e_node.rs`; a true
oracle diff against each language's native runtime remains open).
All items through JS/TS backend codegen are shipped.

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
2. **New math-language additions** — HML00 Wave 4+ (APL, J, K/Q, Scilab,
   IDL, Reduce, Derive, Maple, Axiom, Julia), none of which exist yet.
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

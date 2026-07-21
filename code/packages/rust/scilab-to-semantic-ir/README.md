# scilab-to-semantic-ir

Scilab CST → narrow-waist Semantic IR. Targets
[SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md), the array/matrix
domain extension of the SIR10 narrow-waist IR (see
[HML01](../../../specs/HML01-math-to-semantic-ir.md)) — the same domain
`matlab-to-semantic-ir`/`apl-to-semantic-ir`/`j-to-semantic-ir` already
target. This is **MA-10e**, the last item in the Scilab frontend rollout
(see [MA10](../../../specs/MA10-scilab-language.md) §6), built alongside
`scilab-runtime`/`scilab-repl` per `HML01` §2's "every math language gets a
`-to-semantic-ir` frontend built alongside the runtime" convention.

## Where this fits

```
Scilab source
   │
   ▼  coding_adventures_scilab_parser::try_parse_scilab(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  scilab_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR16 + SIR22)
```

The lowered `Module` can then be validated (`semantic_ir::validate`) and
handed to any SIR backend (e.g. `semantic-ir-to-javascript`) — "write the
Scilab frontend once, target every SIR backend" is the whole point of this
narrow-waist design.

This crate is a wholly separate, ahead-of-time lowering pass over the same
CST shape `scilab-parser` produces — it does **not** depend on
`coding-adventures-scilab-runtime` (the tree-walking evaluator) at all,
mirroring `apl-to-semantic-ir`/`j-to-semantic-ir`'s identical choice.

## Usage

```rust
use scilab_to_semantic_ir::compile_source;

let module = compile_source("x = 1 + 2;\n", "demo")?;
```

## Scope (v0.1.0)

Scilab is close to MATLAB in *grammar shape* but not in *language* (MA10
§1); this first cut covers MA10 §4's own in-scope surface and returns a
clean `ScilabLowerError` for anything outside it, rather than silently
mis-lowering. See `src/lower.rs`'s module doc comment for the exact
supported-construct list and every deliberate, disclosed scope limit.

### What's genuinely new relative to `matlab-to-semantic-ir` (this crate's
primary template)

- **The `stmt_sep` linker** (`then`/`do`/comma/newline before a
  control-flow body) needs no SIR representation at all — by lowering
  time it has already collapsed to "which statements are in this
  branch/body." It does, however, shift every control-flow lowering
  function's child-node indexing by one slot relative to the MATLAB
  template, since `stmt_sep` is a real child node in this grammar.
- **`select`/`case`/`else`** (Scilab's own multi-way conditional, no
  MATLAB `switch`/`otherwise` analogue) desugars into a nested `if`-chain
  at lowering time — no new SIR node — mirroring how
  `scilab-runtime::eval::eval_select` evaluates it at runtime. The
  selector is hoisted into a fresh, uniquely-numbered temporary
  (`__select_N`) and evaluated exactly once, since re-lowering it once per
  `case` would re-evaluate a possibly side-effecting expression multiple
  times.
- **The eight `%`-prefixed special constants** (`%pi %e %i %inf %nan %eps
  %t %f`) are constant-folded directly into a plain `Expr::IntLit`/
  `Expr::FloatLit` at lowering time — their values are fixed and already
  known, so no dedicated SIR node is needed. `%i` (complex numbers) is a
  clean, honest `ScilabLowerError`, mirroring
  `scilab-runtime::builtins::percent_const`'s own identical choice.
- **`$` (last-index)** mirrors `matlab-to-semantic-ir`'s own
  `end`-relative-indexing exclusion: a disclosed v0.1.0 scope limit, not
  a silently-wrong lowering.
- **Multi-output functions** (`[a, b] = f(...)`) are out of scope,
  mirroring the MATLAB template's identical exclusion — no sibling
  frontend in this repo has since solved multi-output lowering either.
  Single- and zero-output functions (including the explicit `[] = f(...)`
  bracket spelling) are supported, following
  `scilab-runtime::eval::Interpreter::register_function`'s own more
  complete reading of `func_returns` (a strict superset of the MATLAB
  template's coarser handling, which cannot distinguish `[y] = f(x)` from
  a genuine multi-name bracket list).
- **`\`/`.\ ` (left division)** are handled *uniformly* as a broadcast
  reciprocal division, regardless of operand scalar-ness — a deliberate
  divergence from the MATLAB template's asymmetric treatment (which
  rejects bare `\` between non-scalars), because `scilab-runtime`'s own
  ground-truth interpreter already makes this exact simplification for
  both spellings. See `src/lower.rs`'s module doc comment for the full
  rationale.
- **No arithmetic or ordering over a directly-written string literal.**
  MA10 §1 finding 1 — the decisive finding motivating Scilab's existence
  as its own frontend — is that `+` means concatenation on Scilab strings
  where it means ASCII-numeric addition on MATLAB ones. This frontend
  proactively guards every additive/multiplicative/power/ordering-
  comparison operator against a bare `Expr::StrLit` operand, closing a gap
  the MATLAB template's own `expr_is_known_scalar` heuristic does not
  address. String *equality* (`==`/`~=`/`<>`) remains in scope, per MA10
  §4.

### Testing

- `tests/test_lower.rs` — unit tests over every supported construct
  (literals, assignment, arithmetic scalar/array disambiguation, matrix
  literals, ranges, 1-D/2-D indexing, `if`/`elseif`/`else` including both
  linker spellings, `while`/`for` including both linker spellings,
  `select`/`case` desugaring, the eight `%`-constants, and every
  documented scope-limit rejection), plus the same DoS-guard regression
  pair (`a_pathologically_long_flat_{additive,multiplicative}_chain_is_
  cleanly_rejected`) `matlab-to-semantic-ir` carries for the identical
  flat-chain hazard (Scilab's grammar collapses a flat operator run into
  one many-child CST node the same way MATLAB's does).
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate`, and modules using SIR22 array/matrix features,
  strings, `%`-constants, and desugared `select`/`case` are all correctly
  *accepted* by `semantic-ir-to-javascript`'s capability check.
- `tests/e2e_node.rs` — lowers Scilab source, emits JavaScript, and
  **actually executes it with `node`** (gated on availability), covering
  scalar arithmetic, forward-referenced function calls, matrix
  multiplication, elementwise scalar broadcast, indexed assignment,
  range+transpose, `%pi`, a `for`-loop accumulator, and both branches of a
  desugared `select`/`case`.

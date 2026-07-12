# wolfram-to-semantic-ir

Wolfram CST → narrow-waist Semantic IR. The first frontend to target
[SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
symbolic-expression/pattern-matching domain extension of the SIR10
narrow-waist IR (Stream B of
[HML01](../../../specs/HML01-math-to-semantic-ir.md)).

## Where this fits

```
Wolfram source
   │
   ▼  coding_adventures_wolfram_parser::try_parse_wolfram(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  wolfram_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR23)
```

## Usage

```rust
use wolfram_to_semantic_ir::compile_source;

let module = compile_source("1 + 2\n", "demo")?;
```

`compile_source` is the hardened entry point: it parses and lowers on a
worker thread with an enlarged stack, so pathologically deep (but
syntactically valid) input fails cleanly instead of overflowing the native
stack — see `src/lib.rs`'s `PARSE_STACK_SIZE` doc comment for why. `compile`
(taking an already-parsed `GrammarASTNode`) is pure lowering with no
thread-spawning of its own, mirroring `matlab-to-semantic-ir::compile`'s
identical division of responsibility.

## Design: "everything is data"

Wolfram's defining idea (MA04 §1) is that *every* fragment is one expression
tree `head[arg, …]`, manipulable as data even before being evaluated. This
frontend lowers **every** construct — arithmetic, comparisons, lists,
function application, even `=`/`:=` assignment — into the SIR23 symbolic
vocabulary (`SymSymbol`/`SymApply`/pattern/rule nodes), never into a
host-language `Expr::VarRef`/`Stmt::Assign`. There is no environment, no
binding, no evaluation at lowering time — see `src/lower.rs`'s module doc
comment for the full reasoning and why this is the only choice that lets an
*uncomputed* Wolfram function body (`f[x_] := x + 1`) compile at all.

## Scope (v0.1.0)

Because everything reduces to the same small SIR23 vocabulary, this crate
covers the **full** grammar `wolfram-parser` accepts (arithmetic,
comparisons, logic, lists, application, patterns, rules, replacement, the
W-6/W-11/W-21 operator sugar) — unlike `matlab-to-semantic-ir`, there is no
scalar/array-style ambiguity here forcing a narrower cut. See
`src/lower.rs`'s module doc comment for the exact node-by-node mapping and
the small, disclosed set of things left out (sequence patterns and other
constructs still open even in the native `wolfram-runtime`'s own W-20
deferred list; `SymRational`, part of the SIR23 vocabulary but unreachable
from this grammar's surface syntax since there is no rational literal
token).

### Recursion-depth hardening

Every flat, same-precedence operator chain (`+`, `*`, `&&`, `||`, `|`,
`/@`/`@@`, `/.`/`//.`, `?`, and — after a security-review finding — chained
postfix application/part groups and `&`-run/pure-function-apply suffixes)
is capped at `MAX_EXPR_DEPTH` operands before any tree is built, because
the Wolfram grammar — like MATLAB's — collapses a long unparenthesized
chain into one CST node with many children rather than nesting through
parens, so it never trips the ordinary grammar-nesting depth guard.

That per-construct capping is necessary but, on its own, not sufficient: a
second security-review finding showed that per-node-scoped guards don't
compose across nested `(...)` boundaries — chaining several
independently-in-bounds constructs through parentheses can still build a
tree far deeper than any single guard's own limit. The authoritative fix
is [`measure_depth_iterative`](src/lower.rs) — an iterative (never
recursive) post-construction depth check, safe to call on a tree of any
size because building a deeply-nested `Box`-based tree only costs heap,
not stack; only *walking* it recursively is dangerous. It runs before this
crate's own unguarded recursive helpers (`collect_pattern_names`/
`bind_pattern_refs`) touch a tree, and once per top-level statement before
anything reaches the returned `Module`, closing the gap regardless of how
the tree was composed.

Detecting an oversized tree isn't the same as safely disposing of it,
either — a fourth review round found that simply letting a rejected tree
fall out of scope invoked `Expr`'s ordinary *recursive* `Drop` glue on the
very tree just found to be too deep, relocating the same crash from
"walking forward" to "walking backward" through it. [`drop_iterative`](src/lower.rs)
tears a rejected tree down the same way `measure_depth_iterative` measures
it — an explicit work stack, no native recursion — before either rejection
site returns its error.

See `CHANGELOG.md`'s "Fixed" entries for the full four-round history —
every guard here (per-construct caps, the authoritative depth check, and
the teardown fix) was verified adversarially: temporarily disabling it and
confirming a real `SIGABRT` native stack overflow (or, for the composition
gap, a silent wrongful *acceptance* of oversized input) reproduces, then
restoring it and confirming the regression test — or, for the `Drop` fix,
an isolated-subprocess repro — passes cleanly.

`compile_source` additionally parses on an enlarged-stack worker thread
(see "Usage" above), reusing `wolfram-runtime`'s own validated-safe
deployment pattern rather than inventing a new one — see that function's
doc comment and `wolfram-parser`'s own `MAX_RULE_DEPTH` doc comment for why
no single depth cap can be both bare-stack-safe and support realistic
nesting for this particular grammar.

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for
  every grammar production, plus DoS-guard regression tests proving each
  guard rejects a chain comfortably past `MAX_EXPR_DEPTH` (covering the
  flat operator chains, the postfix/amp chained-application gap, and the
  cross-`(...)`-boundary composition gap the review found), and
  exact-boundary tests (`MAX_EXPR_DEPTH` operands/groups parse, one more is
  rejected). These run at a scale well past the cap but much smaller than
  the incidents that originally motivated them, since `wolfram-parser`
  must parse the input before this crate's guards ever run, and parsing
  a very large flat chain is itself slow — see `CHANGELOG.md`'s "Fixed
  (CI)" entries for the full story (this was the real cause of an early
  CI failure, not a stack-size issue).
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate` (manifest declares exactly the SIR23 features
  used) and is correctly *rejected* by `semantic-ir-to-javascript`'s
  capability check.

There is **no** e2e `node`-execution test in this crate, unlike
`matlab-to-semantic-ir`'s purely-literal case: under the "everything is
data" design, even bare literal arithmetic (`1 + 2`) emits at least one
SIR23 node, and no backend implements SIR23 codegen yet
(`sir-runtime-symbolic`, the JS/TS runtime library it would depend on, is
separate, not-yet-shipped follow-on work — HML01 Stream B rollout item 6).

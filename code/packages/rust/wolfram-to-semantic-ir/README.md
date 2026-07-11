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
`/@`/`@@`, `/.`/`//.`, `?`) — and, after a security-review finding fixed
before first push, every chained postfix application/part group
(`f[…][…]…`) and `&`-run/pure-function-apply suffix run — is capped at
`MAX_EXPR_DEPTH` operands *before* any tree is built, because the Wolfram
grammar — like MATLAB's — collapses a long unparenthesized chain into one
CST node with many children rather than nesting through parens, so it
never trips the ordinary grammar-nesting depth guard. The first eight
productions were covered from day one; the postfix/amp chains were an
initial gap the security review caught (see `CHANGELOG.md`'s "Fixed"
entry) — both classes are now verified adversarially: temporarily removing
each guard and re-running its 60,000-term regression test reproduces a
real `SIGABRT` native stack overflow, confirming the guards are
load-bearing, not decorative.

`compile_source` additionally parses on an enlarged-stack worker thread
(see "Usage" above), reusing `wolfram-runtime`'s own validated-safe
deployment pattern rather than inventing a new one — see that function's
doc comment and `wolfram-parser`'s own `MAX_RULE_DEPTH` doc comment for why
no single depth cap can be both bare-stack-safe and support realistic
nesting for this particular grammar.

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for
  every grammar production, plus DoS-guard regression tests at the same
  60,000-term scale `matlab-to-semantic-ir`'s own security review
  established (covering both the flat operator chains and the
  postfix/amp chained-application gap the review found here), and
  exact-boundary tests (`MAX_EXPR_DEPTH` operands/groups parse, one more is
  rejected).
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

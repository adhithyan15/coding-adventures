# q-to-semantic-ir

Q (kdb+'s scripting language) CST → narrow-waist Semantic IR. Item **MA-11e**
of the Q frontend (spec [`MA11`](../../../specs/MA11-q-language.md)), built
alongside the runtime (`q-runtime`) in this same wave rather than as a later
retrofit, per [HML01](../../../specs/HML01-math-to-semantic-ir.md) §2's
amended per-language pattern — mirroring APL's/J's/Scilab's own precedent.

## Where this fits

```
Q source
   │
   ▼  coding_adventures_q_parser::try_parse_q(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  q_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR22 + SIR22 addendum)
```

## Usage

```rust
use q_to_semantic_ir::compile_source;

let module = compile_source("f:{x+y}\n2 f 3\n", "demo")?;
assert!(module.functions.iter().any(|f| f.name == "main"));
// `f` lowers to its OWN top-level `semantic_ir::Function` (params x, y),
// not a bare closure value stored in a variable -- see below for why.
assert!(module.functions.len() >= 2);
```

## Design: built directly on `j-to-semantic-ir`

Q is APL/J's second-generation descendant (MA11 §1) and reuses their exact
two-nonterminal (`noun_expr`/`term`/`verb_expr`), right-to-left,
no-precedence grammar shape (MA11 §3) — so this crate is built directly on
`j-to-semantic-ir`'s design, the most structurally similar prior frontend
(same shape, same primitive-verb dispatch pattern), reusing SIR22 + the
"APL/J addendum" (`Reduce`/`Scan`/`Ravel`/`Catenate`/`IndexGenerator`) those
two crates already established. See `src/lower.rs`'s module doc comment for
the full node-by-node lowering table and design rationale.

### All 17 primitive verbs (MA11 §4)

| Glyph | Monadic | Dyadic |
|---|---|---|
| `+` | flip → identity (no primitive in this cut can ever construct rank 2) | add → `ElementwiseOp` |
| `-` | negate → `neg` builtin (reused from APL) | subtract → `ElementwiseOp` |
| `*` | first → `q_first` (new) | multiply → `ElementwiseOp` |
| `%` | reciprocal → `recip` builtin (reused) | divide → `ElementwiseOp` |
| `!` | til (0-based) → `IndexGenerator` + a `-1` correction (same fix J's own `i.` needs) | dyadic (dict creation) — **deferred**, clean error |
| `,` | enlist → `Ravel` (provably identical on every reachable value in this cut) | join → `Catenate` (case-by-case identical to APL/J's) |
| `#` | tally → `tally` builtin (reused **as-is** from J) | take → `q_take` (new — genuinely different from J's *replicate*) |
| `_` | floor → `floor` builtin (reused) | drop → `q_drop` (new) |
| `&` | where → `q_where` (new) | min → `ElementwiseOp` |
| `\|` | reverse → `q_reverse` (new) | max → `ElementwiseOp` |
| `~` | not → `q_not` (new, elementwise) | match → `q_match` (new — deep equality, the one non-elementwise dyadic primitive) |
| `=` `<>` `<` `<=` `>=` `>` | none (error) | `ElementwiseOp` |

Only **5 genuinely new** `BuiltinCall` names were needed (`q_first`,
`q_where`, `q_reverse`, `q_not`, plus the dyadic `q_take`/`q_drop`/`q_match`)
— two would-be-new primitives turned out to coincide exactly with existing
SIR22-addendum nodes once Q's actual reachable value space (rank 0/1 only,
no reshape in scope) was checked directly, and one (`tally`) reuses J's
existing builtin verbatim.

### Function literals — the one lowering surface with no APL/J precedent

`{[x;y] stmt; stmt; ...}` (MA11 §2/§3 bullet 1) is the one genuinely new
*lowering* problem this crate's model (`j-to-semantic-ir`) never had to
solve — APL's/J's in-scope grammars are expression-only, so neither ever
needed to represent a real user-defined function with named parameters.
Since Q's own function values capture **nothing at all** (MA11 §2;
`q_runtime::eval::Lambda`'s own doc comment: every non-parameter name
resolves against the *global* frame at call time, not a snapshot), this
crate's design is considerably simpler than a general-purpose language
frontend's (Python's/Ruby's) own lambda-lifting machinery — no free-variable
analysis, no capture list ever populated.

Every function literal becomes its own genuine `semantic_ir::Function`
(`captures: vec![]`), dispatched three ways, one shared decision:

1. A NAME resolving to a function directly assigned at the top level, or an
   inline literal applied the moment it's written → `Expr::DirectCall`
   (statically known callee).
2. Anything else (a parameter, an unresolved global, a parenthesised term)
   → `Expr::IndirectCall` through whatever `Expr` it evaluated to.

This single rule handles the genuinely dynamic, higher-order case with
**no special-casing**: `apply:{[g] g 5}` then `apply inc` passes a function
value as an argument, and `g 5` inside `apply`'s own body dispatches
dynamically with no static knowledge of what `g` holds — exactly mirroring
`q-runtime`'s own real test coverage for this exact pattern.

### Top-level scope is `Global`, not `Local` — a real divergence from J/APL

J/APL lower every top-level name to a `main`-local binding, since their
entire program lives inside one function and nothing else ever reads it.
Q genuinely breaks this: a function literal's body can read a plain array
variable assigned at the top level, from inside a **separate**,
independently-compiled `Function` — so every top-level Q variable becomes a
genuine `semantic_ir::Global` instead, visible to every compiled JS function
in the same file. See `src/lower.rs`'s module doc comment for the full
rationale.

### Testing

```sh
cargo test -p q-to-semantic-ir -- --nocapture
```

- `tests/test_lower.rs` — 57 unit tests over exact `Expr`/`Function` shapes
  for every grammar production.
- `tests/test_validator.rs` — 10 capability-acceptance tests against the
  shared SIR validator and `semantic-ir-to-javascript`.
- `tests/e2e_node.rs` — 12 tests actually running compiled programs through
  `node`, weighted toward the function-literal machinery.
- `tests/oracle.rs` (HML01 §7) — 51-case oracle/golden corpus cross-checking
  `q-runtime` (ground truth) against this crate → `semantic-ir-to-javascript`
  → real `node`: every one of the 17 primitive verbs (monadic and dyadic),
  all three whitespace-sensitive strand-vs-subtraction spellings (`2 -1`,
  `2 - 1`, and fully-glued `2-1`), reduce/scan/each, dual list-literal
  syntax, chained/global assignment, and the full function-literal surface
  (named, inline, implicit/explicit params, multi-statement bodies, and the
  higher-order `MakeClosure`/`IndirectCall` case). All 51 cases pass
  end-to-end today — task #109 fixed the one shared-crate display gap this
  corpus originally found (`SIR_DISPLAY_Q_ASCII_MINUS` in
  `semantic-ir-to-javascript`; see `CHANGELOG.md`'s `[0.1.1]` entry). The
  `known_bug` field stays on `Case` for a future genuine bug to reuse, unused
  by any entry today.

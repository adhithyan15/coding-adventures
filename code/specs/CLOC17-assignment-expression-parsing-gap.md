# CLOC17 — assignment-expression parsing gap (the whitespace-only-fallback bug)

**Status:** IMPLEMENTED for the Rust parser (`javascript-parser` 0.9.0). The
14 `es*.grammar` `assignment_expression` rules were reordered and
`javascript-parser/src/_grammar.rs` regenerated; CLOC17 regression tests added;
closurec now optimizes assignment-containing programs. The 13 sibling-language
`javascript-parser` packages still carry the old ordering — regenerating them
is a tracked follow-up (no CI parity gate enforces it). Original spec +
root-cause analysis preserved below.

## TL;DR

**Any** JavaScript program containing an **assignment expression statement**
(`a = 1;`, `g = f(5);`, `obj.k = v;`, `count += 1;`) currently makes `closurec`
emit **whitespace-only** output — *no inlining, no folding, no renaming, no DCE,
nothing*. Since real-world JS is saturated with assignments, closurec today
effectively optimizes almost nothing on real input. This is the single
highest-impact correctness/coverage gap in the minifier, dwarfing any
individual pass improvement.

## Empirical proof

```js
// No assignment anywhere → the typed pipeline runs (folds, DCE, local rename):
function f(p){ log(p); } f(1);
// SIMPLE ⇒ function f(p){log(p)};f(1);   (typed pipeline; f KEPT — SIMPLE is
//                                         open-world, it never inlines/removes
//                                         a top-level name)

// Add ONE unrelated assignment → the WHOLE program degrades to whitespace-only:
function f(p){ log(p); } f(1); a = 2;
// SIMPLE ⇒ function f(p){log(p)};f(1);a=2;   (only spaces removed)
```

The two outputs happen to coincide on `f` here (open-world SIMPLE never inlines
`f` either way); the load-bearing difference is elsewhere — e.g. arithmetic
folds and dead code drop in the typed pipeline but survive verbatim under the
whitespace fallback. See CLOC24's `simple-debugger` oracle for a case where the
two diverge visibly (`debugger;` stripped vs. kept).

The single `a = 2;` forces fallback for the entire program. Verified on
`closurec --compilation_level SIMPLE` (2026-06-19).

## Why it happens (root cause)

`closurec` parses source into a grammar AST with `parse_javascript_typed`
([`javascript-parser/src/lib.rs`](../packages/rust/javascript-parser/src/lib.rs)),
then converts it to the typed AST with `grammar_to_program`
([`javascript-parser/src/bridge.rs`](../packages/rust/javascript-parser/src/bridge.rs)).
If **either** step errors, `closurec` silently falls back to whitespace-only
minification
([`closurec/src/run.rs`](../programs/rust/closurec/src/run.rs) ~L419–451,
`simple_bridge_status = parse_error:…` → `whitespace_only`).

The failure is in the **parser**, not the bridge. For `a = 1;`:

```
JavaScript parse failed: Parse error at 1:3: Expected COLON or DOT or LBRACKET …
or COMMA or SEMICOLON, got "="
```

The "Expected" set contains every binary/conditional/comma/semicolon follow-token
but **no assignment operator** — the grammar never even offers `=` at that
position. The PEG grammar rule (e.g. `code/grammars/ecmascript/es2025.grammar`
L463–467, and analogously in **all 14** `es*.grammar` files) is:

```
assignment_expression = conditional_expression
                      | arrow_function | async_arrow_function | yield_expression
                      | left_hand_side_expression assignment_operator assignment_expression ;
```

`GrammarParser`'s `Alternation` is an **ordered-choice PEG** (first match wins,
no longest-match / follow-set check —
`code/packages/rust/parser/src/grammar_parser.rs` ~L554–563). `conditional_expression`
is listed **before** the assignment-target alternative, and a bare identifier
`a` is itself a valid `conditional_expression`. So PEG commits to
`conditional_expression`, consumes only `a`, and returns success; the trailing
`=` is left for `expression_statement = expression SEMICOLON`, which expects
`;`, sees `=`, and fails. The assignment-target alternative (which *would*
consume the `=`) is never reached.

It is **not** a left-recursion problem (the assignment alternative begins with
`left_hand_side_expression`, not `assignment_expression`), and **not** a bridge
problem (`bridge.rs::convert_assignment_expression` already handles the 3-node
`lhs assignment_operator rhs` shape, and `expr_to_assignment_target` already
accepts `Identifier` and `MemberExpression` left-hand sides — both verified).

## The fix

**Reorder the alternatives** so `left_hand_side_expression assignment_operator
assignment_expression` is tried **before** `conditional_expression`, with the
function-like alternatives (`arrow_function`, `async_arrow_function`,
`yield_expression` / `"yield" [STAR] assignment_expression`) kept ahead and
`conditional_expression` moved **last**. For es2025:

```
assignment_expression = arrow_function | async_arrow_function | yield_expression
                      | left_hand_side_expression assignment_operator assignment_expression
                      | conditional_expression ;
```

PEG then tries the assign-target sequence, **fails fast** when no
`assignment_operator` follows the left-hand side (a bare `a`, a member `a.b`, a
call `f()`, a binary `a + b`, a ternary `a ? b : c`), and falls through to
`conditional_expression` exactly as before — while `a = 1`, `a += 1`,
`a.b = 1` now parse to a 3-node `assignment_expression` and bridge to `Ok`.
This was **empirically validated** for es2025 (assignment, compound assignment,
member-target assignment all parse + bridge; `var`/arrow/conditional still
parse) via an in-memory grammar patch.

### Per-grammar shape (all 14 share the bug)

| grammar | current order | reorder to |
| --- | --- | --- |
| es1, es3, es5 | `conditional \| lhs_assign` | `lhs_assign \| conditional` |
| es2015 | `arrow \| "yield"[STAR]assign \| conditional \| lhs_assign` | `arrow \| "yield"[STAR]assign \| lhs_assign \| conditional` |
| es2016 | `conditional \| arrow \| yield_expression \| lhs_assign` | `arrow \| yield_expression \| lhs_assign \| conditional` |
| es2017–es2025 | `conditional \| arrow \| async_arrow \| yield_expression \| lhs_assign` | `arrow \| async_arrow \| yield_expression \| lhs_assign \| conditional` |

(`lhs_assign` = `left_hand_side_expression assignment_operator assignment_expression`.)

### Scope & mechanics

- Edit the `assignment_expression` rule in all **14**
  `code/grammars/ecmascript/es*.grammar` files (one-line reorder each).
- **Regenerate** `code/packages/rust/javascript-parser/src/_grammar.rs` via
  `grammar-tools generate-rust-compiled-grammars javascript` (the binary at
  `code/programs/rust/grammar-tools`). **Never hand-edit `_grammar.rs`** — it is
  auto-generated; keep `.grammar` + `_grammar.rs` in the same commit, and do
  **not** `cargo fmt` the generated file.
- Other languages embed their own generated grammar artifacts from the same
  `.grammar` sources; regenerate those too if the parity check requires, or scope
  this PR to the Rust `javascript-parser` (closurec's parser) and file the
  cross-language regen as a follow-up. Decide based on the CI parity gate.
- **No `bridge.rs` changes.** No `unsafe`, no algorithmic risk — only PEG
  alternative ordering.

### Soundness caveats / required regression tests

PEG ordering is subtle. Before merge, add parser tests (per ES version, since
es1/es3/es5 have no arrow/yield while es2015+ do) asserting **all** still parse:
- bare identifier / member / call / binary expression statements (`a;`, `a.b;`,
  `f();`, `a + b;`);
- ternary (`a ? b : c;`);
- arrow (`x => x;`) and `yield` (in a generator) where the version supports them;
- `var`/`let`/`const` initializers (`var x = 1;` — these go through
  `convert_variable_declarator`, a different path, but must remain unaffected);
- and the newly-fixed forms: `a = 1;`, `a += 1;`, `a >>>= 2;`, `a.b = 1;`,
  `a = b = c;` (right-assoc chain), `a = b ? c : d;`.

Then verify end-to-end that closurec now **optimizes** assignment-containing
programs (the `function f(p){log(p)} f(1); a=2;` example should inline `f`).

## Downstream unlock

Fixing this is a prerequisite for an entire family of already-designed-but-
unreachable optimizations:

- **Assignment-target value capture** (`g = f(x)` → `body…; g = E`) — the third
  member of the value-capture family after PR-3 (const-init) and PR-5 (return),
  resolving CLOC15 Open Question 2's assignment-target case. The
  `closure-pass-inline` logic for it is straightforward (a `CaptureTail::IntoAssignment`
  variant + an `ExpressionStatement(AssignmentExpression{Eq, Identifier, <call>})`
  match), but it cannot fire — and cannot even be unit-tested via the
  bridge-based `inline_source` harness — until assignment statements parse. It
  was prototyped and reverted pending this fix.
- **`var`-local helper inlining** — the bridge desugars `var t = E` into
  `var t; t = E` (an assignment expression), so multi-statement helpers with a
  `var` local also hit the same fallback.

Both become reachable the moment assignment expressions parse.

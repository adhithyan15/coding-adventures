# Changelog

All notable changes to the `coding-adventures-javascript-parser` crate will be documented in this file.

## [0.19.9] - 2026-06-30

### Fixed — destructuring declarations aborted the compile instead of declining

A `var` / `let` / `const` declaration with a destructuring binding pattern
made the bridge raise an `Internal` error, which the CLI treats as a hard
failure (`exit 2`, error text on stdout, no JS output):

```
var [a, b] = c;   →  bridge internal error: variable declarator: missing name
let {p, q} = o;   →  bridge internal error: lexical_binding: ... missing name
```

Destructuring is a Phase 2 feature the typed bridge doesn't represent yet —
but, like spread / optional chaining / `new`, it should DECLINE gracefully so
the CLI falls back to WHITESPACE_ONLY and still emits valid (if less
optimized) JavaScript, never abort.

**Cause.** `convert_variable_declarator` searched the declarator's direct
children for a NAME token and unwrapped it with
`ok_or_else(|| internal(node, "missing name"))` BEFORE checking for a
`binding_pattern` node. A destructuring target is a `binding_pattern` node
with no NAME token at that level, so the unwrap fired the `Internal` error
first and the later binding-pattern→`UnsupportedSyntax` check was dead code.

**Fix.** The `binding_pattern` → `UnsupportedSyntax` decline now runs first,
so `var [a,b]=c;` / `let {p,q}=o;` / `const [x]=y;` round-trip through the
WHITESPACE_ONLY fallback (`exit 0`) instead of aborting. Plain (identifier)
declarations are unaffected.

Regression test: `destructuring_declarations_decline_gracefully_not_hard_error`.

## [0.19.8] - 2026-06-30

### Fixed — assignment expression as a call argument / array element was dropped (miscompile)

An assignment used as a call argument or array element lost its operator and
right-hand side, leaving only the assignment target:

```
f(x = 1)      →  f(x)        (assignment vanished; arg is now `x`, not `1`)
g(a, b = 2, c)→  g(a, b, c)
f(x += 1)     →  f(x)        (compound assignment vanished)
f(x = y = 1)  →  f(x)        (chained assignment vanished)
[x = 1]       →  [x]
[a = 1, b]    →  [a, b]
```

These are real miscompiles: the assignment's side effect is erased and the
expression's value changes (`f(x=1)` passes `1`; `f(x)` passes whatever `x`
already held).

**Cause.** The parser collapses the single-alternative `argument` /
element production, so the node reaching `convert_argument` (and the array
element loop in `convert_array_literal`) IS the `assignment_expression`
itself, whose children for `x = 1` are
`[left_hand_side_expression(x), assignment_operator(=), assignment_expression(1)]`.
Both call sites unwrapped to `node_children(node).next()` — the FIRST child —
grabbing only the LHS and discarding `= rhs`. (`convert_assignment_expression`
itself was already correct; it simply was never reached.)

**Fix.** Both sites now convert the WHOLE node via `convert_expression`, which
dispatches `assignment_expression` to `convert_assignment_expression`,
preserving the assignment. `convert_argument` still unwraps an explicit
`argument` wrapper node if a future grammar revision produces one, and the
spread (`...x`) guard is unchanged. Plain (non-assignment) arguments and
elements, and array holes (`[1,,3]`), are unaffected.

Regression tests: `assignment_expression_as_call_argument_is_not_dropped`,
`compound_and_chained_assignment_arguments_survive`,
`assignment_expression_as_array_element_is_not_dropped`.

## [0.19.7] - 2026-06-30

### Fixed — member access on a call result was silently dropped (miscompile)

A member access applied to a call result lost part of the expression:

```
f().x     →  f()       (the `.x` property read vanished)
f()[k]    →  f[k]      (the call `()` vanished — wrong object entirely)
g(f().x)  →  g(f())    (same drop, nested in an argument)
```

Both are real miscompiles: the emitted program reads a different value (or
calls nothing at all) compared to the source.

**Cause.** The grammar parses a `call_expression` as a FLAT suffix chain — a
base (`member_expression` / `primary_expression`) followed by any mix of
`arguments` (a call), `. NAME` (dot member) and `[ expr ]` (computed member)
suffixes, in source order. For example `f().x` parses to
`[member_expression(f), arguments(()), Token("."), Token("x")]`. The bridge,
however, inspected only the LAST child and dispatched the whole node to a
single handler:

- when the last child was `arguments` it built the call and ignored any
  trailing `.NAME` / `[expr]` tokens (`f().x` → `f()`);
- when the last child was a member suffix it delegated to
  `convert_member_expression`, which took the FIRST child as the base and
  skipped the intervening `arguments` node (`f()[k]` → `f[k]`).

**Fix.** `convert_call_expression` now folds EVERY suffix left-to-right onto
the growing base — `arguments` → `CallExpression`, `.NAME` → non-computed
`MemberExpression`, `[expr]` → computed `MemberExpression` — mirroring the
member-suffix walk in `convert_member_expression`. This also subsumes the
chained-call `f()()` fold added in 0.19.6. Optional chaining (`?.`) and any
unrecognised suffix token are rejected (fail-closed: a bridge error feeds the
CLI's WHITESPACE_ONLY fallback, never a wrong program).

Regression tests: `dot_member_on_call_result`, `computed_member_on_call_result`,
`call_member_call_mixed_chain` (plus the existing `chained_call_expression` /
`triple_chained_call_with_args`, which still pass).

## [0.19.6] - 2026-06-30

### Fixed — chained calls `f()()` raised a bridge internal error

A chained call such as `f()()` or `f(1)(2)(3)` raised
`bridge internal error: arguments: unknown expression rule 'arguments'`,
so any program containing one failed to compile.

The grammar models calls with left recursion
(`call_expression = call_expression arguments`), and the parser flattens a
chain of call sites into a **single** `call_expression` node whose children
are the base followed by one `arguments` node per call site:

```
f()()   →  call_expression[ member_expression(f), arguments(()), arguments(()) ]
```

`convert_call_expression` derived the callee of the outer call by converting
the *second-to-last* child directly — for a 3-child chain that child is the
inner `arguments` node, not an expression, so `convert_expression` fell through
to its catch-all and reported the rule name `arguments`.

The callee is now rebuilt by folding the leading `arguments` nodes
left-to-right into nested `CallExpression`s
(`f` → `f()` → `f()()`), with the final `arguments` node forming the outer
call. A guard keeps this sound: because `node_children` strips `Token`
children, a `.`/`[` member access appearing between calls would be invisible
to the fold, so when such a token is present at this level we fall through to
the existing unsupported-syntax path (an error) rather than risk silently
turning `f().x()` into `f()()`. Pure call chains carry no such tokens and now
round-trip correctly; interleaved member/call forms continue to nest into
their own sub-nodes and are unaffected.

Regression tests: `chained_call_expression`, `triple_chained_call_with_args`.

## [0.19.5] - 2026-06-30

### Fixed — prefix `++` / `--` silently dropped (miscompile)

`convert_unary_expression` recognises prefix operators by mapping the operator
token through `unary_operator_from_str`, which intentionally returns `None` for
anything that is not a real unary operator (`- + ! ~ typeof void delete`). A
prefix `++` / `--` token also maps to `None`, so it fell into the
`postfix_expression` pass-through arm and the bridge returned the bare operand —
`++a` became `a`, dropping the increment. That is a **miscompile** at
SIMPLE/ADVANCED (`++a` and `a` are different programs: the former increments `a`
and evaluates to `a+1`).

A prefix `++`/`--` is now REJECTED with `UnsupportedSyntax("UpdateExpression")`,
exactly as the postfix `a++` form already is in `convert_postfix_expression`.
closurec then falls back to identity passthrough, emitting `++a` verbatim —
unminified but correct. (Full `UpdateExpression` support, prefix and postfix, is
a separate Phase-2 item; this change only closes the soundness hole.) New test
`prefix_update_operators_are_rejected_not_dropped`.

## [0.19.4] - 2026-06-30

### Fixed — array elisions (holes) silently dropped (miscompile)

`convert_array_literal` iterated `node_children(element_list)`, but
`node_children` strips Token children — so the COMMA tokens that delimit array
holes were invisible and every elision was dropped. `[1,,3]` (a length-3 array
with a hole at index 1) became the length-2 dense array `[1,3]`. That is
observable: `[1,,3].length === 3` and `1 in [1,,3] === false`, versus
`[1,3].length === 2` and `1 in [1,3] === true`.

The function now walks the RAW children of `element_list` and applies the
standard elision rule: a comma seen while still "expecting an element" (at the
start, or right after another comma) pushes a `None` hole; a single trailing
comma after an element is not a hole (`[1,2,]` stays length 2). Spread elements
(`[...x]`) still return `UnsupportedSyntax`. New test
`array_elisions_become_holes_not_dropped` covers internal / leading / trailing /
multiple / single-hole and trailing-comma shapes.

## [0.19.3] - 2026-06-29

### Fixed — object property keys parsed as bare identifiers (miscompile)

`convert_property_key` matched on `t.type_name` to recognise STRING and NUMBER
keys, but ordinary terminals carry their kind in the `t.type_` discriminant —
`type_name` is `None` for them (only special tokens like BIGINT set it). So
every STRING/NUMBER key fell through to the NAME fallback and became a bare
`PropertyKey::Identifier` built from the **un-decoded** token text. Downstream
that emitted invalid or wrong code:

| source            | was            | now (correct)     |
|-------------------|----------------|-------------------|
| `{"a-b": 1}`      | `{a-b:1}` ✗ SyntaxError | `{"a-b":1}` |
| `{"a b": 1}`      | `{a b:1}` ✗ SyntaxError | `{"a b":1}` |
| `{"x\ty": 1}`     | `{x\ty:1}` ✗ stray escape | `{"x\ty":1}` |
| `{"__proto__":1}` | `{__proto__:1}` ✗ **proto setter** | `{"__proto__":1}` |
| `{"abc": 1}`      | `{abc:1}`      | `{abc:1}` (unchanged) |

The function now switches on `t.type_`, mirroring `convert_primary_token`, and
decodes string keys via `unquote_string` so a key's `value` holds the real
(decoded) property name. The quote-vs-bare emission choice is made soundly in
the emitter. New bridge tests assert the key node kinds (StringLiteral /
NumericLiteral / Identifier) for each shape, including `__proto__`.

## [0.19.2] - 2026-06-29

### Added — propagate per-token CvIds to the bridge (CLOC27 P2 + P3)

Closes the gap where constant-fold provenance dead-ended at the bridge
boundary: leaf literals in the typed AST carried `cv: None`, so a folded `3`
from `"abc".length` derived from *nothing* and the sidecar never tied it back
to the `"abc".length` source span. The CvIds already existed (minted per token
by `tokenize_javascript_with_cv`) — they were simply discarded before the
parser. This release stops discarding them and stamps them onto the leaves.

- **D2 — stop stripping the CvId before the parser.** `parse_javascript_with_cv`
  previously did `cv_tokens.into_iter().map(|t| t.token)`, dropping each token's
  CvId; it now sets `cv: Some(t.cv)` on the token via struct-update, so the id
  rides through the parser into the `GrammarASTNode` the bridge walks. The
  parser does not inspect `cv`, so this is transparent to it.
- **D3 — `parse_javascript_typed_with_cv`.** New CV-carrying twin of
  `parse_javascript_typed`: routes through the CV tokenizer (D2) and runs the
  identical Phase-1 ASI parse, returning a `GrammarASTNode` whose tokens carry
  CvIds. This is the typed-AST feeder the SIMPLE `--correlation_vector` path
  will use (CLOC27 D5/P4). The plain `parse_javascript_typed` stays the
  zero-overhead default.
- **D4 — stamp the leaf in `convert_primary_token`.** The bridge's sole
  leaf-literal factory replaces its nine `cv: None` returns
  (`NullLiteral`, `UndefinedLiteral`, `BooleanLiteral`×2, `BigIntLiteral`,
  `NumericLiteral`, `StringLiteral`, `Identifier`×2) with `cv: t.cv.clone()`.
  When the token carries no id (the non-CV path), this is `None` —
  **byte-identical to today**, so every existing test passes unchanged. When CV
  is on, the leaf now carries its source token's CvId, whose `Origin` is the
  source span.

No emitter change and no minting in the bridge: CvIds never appear in emitted
JS, and the bridge stays a pure `GrammarASTNode → Program` transform that only
*copies* an id that already exists. The disabled (non-CV) path is unchanged.

## [0.19.1] - 2026-06-29

### Changed — adapt to `lexer::Token` gaining a `cv` field (CLOC27 P1)

The synthetic ASI semicolon (`asi::synthetic_semicolon`) now sets `cv: None` on
the `Token` it builds — correct, since an ASI-inserted token corresponds to no
source bytes and so carries no correlation-vector id. Mechanical adaptation to
`lexer` 0.7.0; no behaviour change (all 82 tests pass unchanged).

## [0.19.0] - 2026-06-22

### Added — ASI Phase 3: restricted productions (Rule 3)

A new proactive pre-pass, `force_restricted_semicolons`, run *before* the
retry-on-error loop in `parse_with_asi`. It forces an automatic semicolon
immediately after a restricted keyword (`return`/`throw`/`break`/`continue`/
`yield`) whose argument is pushed onto the next line — the ECMAScript §12.10.1
"no LineTerminator here" rule.

This is the first ASI rule that must change a parse the grammar *already
accepts*: because the grammar is newline-blind, `return ⏎ a + b` would otherwise
parse as `return a + b` and closurec would re-emit that — a silent **miscompile**
(JS semantics are `return; a + b`). The retry-on-error harness (Rules 1/2) can
never see this, since the bad parse *succeeds*, so Rule 3 needs its own
forward-scanning pass.

Safety is preserved by the same lever as Rules 1/2: an insertion is made **only
when a line terminator actually follows the keyword** (`TOKEN_PRECEDED_BY_NEWLINE`
on the next token), so every valid single-line `return x;` is byte-identical.
Context guards keep a `return` that is really a *property name* from being
mis-split:

- **member access** — a `.`/`?.` before the keyword (`a.return`, `a?.return`)
  demotes it to a property; declined.
- **property key / label** — a `:` after the keyword (`{return: 1}`) marks it as
  an object key; declined.
- **already terminated** — a `;`/`}` after the keyword needs no extra `;`
  (Rule 2 covers the `}`); declined.

The pre-pass is idempotent and allocation-free on any stream containing no
restricted keyword. `yield` only triggers where the lexer classifies it as a
genuine keyword (inside a generator); as an ordinary identifier it is left to
Rule 1, which already splits it correctly. Postfix `++`/`--` restricted
productions remain a documented follow-up.

8 new unit tests cover each keyword, the same-line no-op, every guard, the
double-insert guard at `}`, idempotence, and the allocation-free fast path.

## [0.18.0] - 2026-06-21

### Changed — ASI Rule 1 reads the lexer's newline flag (limitation removed)

`asi_applies_at`'s line-terminator rule now reads `TOKEN_PRECEDED_BY_NEWLINE`
off the offending token (the `lexer` crate, 0.6.0, now sets it) instead of
comparing start lines and guarding against multi-line predecessors. This:

- **removes the `token_may_span_lines` workaround** and the cooked-`value`
  reasoning it depended on, and
- **removes the documented Phase-2 limitation** — a statement ending in a
  string/template/regex literal immediately before a newline now ASI-recovers
  correctly (the flag is set from *trivia*, so it is robust regardless of the
  predecessor's lexeme). The corresponding unit test flips from "declined" to
  "recovered".

Soundness is unchanged: insertion still happens only on a genuine parse failure
(byte-identical on already-valid input), and Rule 1 still requires an actual
line terminator, so one-line `a=1 b=2` remains a real error.

## [0.17.0] - 2026-06-21

### Fixed — prefix unary operators were silently dropped by the bridge

`convert_unary_expression` discriminated the two `unary_expression` grammar
alternatives —

```text
unary_expression = postfix_expression
                 | ("delete"|"void"|"typeof"|PLUS|MINUS|TILDE|BANG) unary_expression
```

— by counting AST **child nodes** (`if node_children(node).len() == 1 { …
pass-through … }`). But the prefix operator is a **token** child, and
`node_children` deliberately returns only `ASTNodeOrToken::Node`s, so *both*
alternatives expose exactly one AST child node. Every prefix-operator form was
therefore mis-classified as a pass-through and the bridge returned the bare
operand:

| source | bridged AST (before) | bridged AST (after) |
|--------|----------------------|---------------------|
| `!a`   | `a`                  | `!a`                |
| `-b`   | `b`                  | `-b`                |
| `~c`   | `c`                  | `~c`                |
| `typeof x` | `x`              | `typeof x`          |

This was a **miscompile** at SIMPLE/ADVANCED (the levels that run the bridge),
not a missed optimization — WHITESPACE_ONLY kept the operators because it never
builds the typed AST.

The discriminator is now the **presence of a recognized prefix-operator token**
(new `unary_operator_from_str` helper), independent of the child-node count.
Added bridge regression tests for each operator, double-negation nesting, and
the pass-through (no-operator) case.

## [0.16.0] - 2026-06-21

### Added — CLOC26 Phase 2: ASI line-terminator rule (Rule 1)

`asi` now also inserts a `;` before an offending token that is **preceded by a
line terminator** (ECMAScript §12.10 Rule 1), not just before a `}`/EOF
(Rule 2). So `a = 1` ⏎ `b = 2` parses as two statements.

The lexer discards newlines as trivia and does **not** populate the
`TOKEN_PRECEDED_BY_NEWLINE` flag, so detection is derived from the `line` field
the lexer records on every token: a line terminator sits between `tokens[idx-1]`
and `tokens[idx]` exactly when the offending token starts on a *higher line*
than its predecessor **and** that predecessor is single-line (its own text
contains no newline — a multi-line predecessor such as a template literal makes
the comparison ambiguous, so we conservatively decline). This needs **no change
to the shared lexer/parser crates**.

Soundness is unchanged from Phase 1: insertion happens only on a genuine parse
*failure*, so any program that already parses is byte-for-byte untouched.
Requiring an actual line terminator for the non-`}`/EOF case is what keeps a
true one-line error (`a = 1 b = 2`) from being silently "recovered" — it still
fails and the caller degrades exactly as before.

`is_asi_recoverable` is replaced by `asi_applies_at(tokens, idx)`, which the
retry loop consults after locating the offending token's index (Rule 1 needs the
predecessor).

- 5 new tests: newline-separated statements recovered; one-line two-statements
  NOT recovered; a multi-statement no-semicolon program; a valid multi-line
  program is a no-op; a binary expression continued on the next line is not split.

## [0.15.0] - 2026-06-21

### Added — CLOC26 Phase 1: Automatic Semicolon Insertion (`}` / EOF rule)

New `asi` module implementing ECMAScript ASI **Rule 2** — a `;` is inserted
before a `}` (or at end of input) that would otherwise be a syntax error. The
grammar spells `SEMICOLON` out as a required terminal in every statement, so
semicolon-light source (`function f(){return 1}`, `{ g() }`) previously failed
to parse — and closurec degraded the whole program to WHITESPACE_ONLY.

`asi::parse_with_asi(tokens, version)` drives insertion **from the parser**:
parse the stream; only if it fails *specifically because a `SEMICOLON` was
expected before a `}`/EOF* (`GrammarParseError` carries both the message and the
offending token), synthesize a `;` at that position and re-parse; bounded loop
with a same-position guard against non-progress. Any non-ASI error is returned
unchanged (caller degrades as before).

**The load-bearing property: a `;` is inserted only when parsing genuinely
failed for lack of one, so ASI is a no-op on any input that already parses** —
it can never change a valid program's parse. (This *retry-on-error* design was
chosen over the lookahead-table the design spec first sketched, precisely
because it guarantees byte-identical output on already-valid input — verified
by the full closurec fixture suite staying byte-for-byte unchanged.)

Wired into `parse_javascript_typed` (the entry closurec uses); other entry
points are unchanged for now. Implemented entirely within this crate — **no
changes to the shared `grammar-tools`/`parser` crates or to any `.grammar`
file** (semicolons stay mandatory in the grammar; ASI supplies them in the
token stream).

Phases 2 (line-terminator rule, via the lexer's existing
`TOKEN_PRECEDED_BY_NEWLINE` flag) and 3 (restricted productions) are follow-ups
per `code/specs/CLOC26-asi.md`.

- 7 unit tests: `}`/EOF recovery, no-op on already-valid input, idempotence on
  recovered input, a genuine syntax error is not papered over, and an empty
  block is not given a semicolon.

## [0.14.0] - 2026-06-20

### Added — CLOC23: bridge `for_of_statement` → `ForOfStatement`

`for_of_statement` no longer lands in the unsupported arm. New
`convert_for_of_statement` mirrors `convert_for_in_statement` but phase-splits on
the `of` token; it detects `var`/`let`/`const` for the binding kind and
**declines** the `using` binding form (scans for a `using` token →
`UnsupportedSyntax`). Destructuring and other unrepresentable lefts decline
gracefully (whitespace-only fallback). `for await (… of …)` is a distinct
grammar production and remains unsupported.

## [0.13.0] - 2026-06-20

### Added — CLOC22: bridge `for_in_statement` → `ForInStatement`

`for_in_statement` no longer lands in the unsupported arm. New
`convert_for_in_statement` walks the children using the `in` and `)` tokens as
phase delimiters (left / right-expression / body) and detects the
`var`/`let`/`const` keyword to set the binding kind. The left binding reuses
`convert_variable_declarator` (which already declines destructuring); any
binding shape it can't represent is mapped to a graceful `UnsupportedSyntax`
decline rather than a hard error, so an unrepresentable for-in left never aborts
compilation. All four left forms (`var`/`let`/`const` and a left-hand-side
expression) are covered; destructuring declines to WHITESPACE_ONLY.

## [0.12.0] - 2026-06-20

### Added — CLOC21: bridge `debugger_statement` → `DebuggerStatement`

`debugger_statement` no longer lands in the unsupported arm (which raised
`UnsupportedSyntax` and forced a WHITESPACE_ONLY fallback at the CLI). The
grammar production is `"debugger" SEMICOLON` — no node children — so the bridge
emits a bare `DebuggerStatement` marker. Added a `debugger_bridge_shape` test.

## [0.11.0] - 2026-06-20

### Added — CLOC20: bridge `do_while_statement` → `DoWhileStatement`

`do_while_statement` no longer lands in the unsupported arm (which raised
`UnsupportedSyntax` and forced a WHITESPACE_ONLY fallback at the CLI). New
`convert_do_while_statement` reads the grammar production
`do statement while ( expression )` — whose Node children are
`[statement, expression]` (body first, test second) — into the ESTree-shaped
`DoWhileStatement`. The prior `do_while_is_unsupported` test is replaced by
`do_while_bridge_shape`, which pins the structural conversion.

## [0.10.0] - 2026-06-20

### Added — CLOC19: bridge `try_statement` → `TryStatement`

`try_statement` no longer lands in the unsupported arm (which raised
`UnsupportedSyntax` and forced a WHITESPACE_ONLY fallback at the CLI). New
`convert_try_statement` reads the first `block` child and walks the remaining
children for a `catch_clause` / `finally_clause`; `convert_catch_clause` extracts
the single `NAME` token as the catch binding (or `None` for the ES2019
optional-catch-binding form). The grammar restricts the catch binding to a simple
`NAME`, so a destructuring catch param fails to parse or bridge and declines
cleanly — it is never lowered to a fabricated simple identifier.

Added structural bridge tests for the full `try/catch/finally`,
optional-catch-binding, and `try/finally` (no catch) forms, plus a guard test
that a destructuring catch param never mis-binds.

## [0.9.0] - 2026-06-19

### Fixed — assignment-expression statements failed to parse (CLOC17)

**Any** JavaScript program containing an assignment-expression statement
(`a = 1;`, `g = f(5);`, `obj.k = v;`, `count += 1;`) failed to parse, which
forced closurec into whitespace-only fallback for the *whole* program — no
inlining, folding, renaming, or DCE. Since real-world JS is saturated with
assignments, this was closurec's single highest-impact coverage gap.

The cause was PEG alternative **ordering**, not the typed bridge (which already
handled the 3-node `lhs assignment_operator rhs` shape). The
`assignment_expression` rule listed `conditional_expression` *before* the
`left_hand_side_expression assignment_operator assignment_expression`
alternative. `GrammarParser`'s `Alternation` is ordered-choice (first match
wins): a bare identifier `a` is itself a valid `conditional_expression`, so the
parser committed to it, consumed only `a`, and left the `=` unconsumed — the
assign-target alternative was never reached.

The fix reorders the `assignment_expression` rule in all 14
`code/grammars/ecmascript/es*.grammar` files so the assign-target alternative
is tried first (the function-like alternatives `arrow_function`,
`async_arrow_function`, `yield_expression` stay ahead of it, and
`conditional_expression` moves last), then regenerates this crate's
`src/_grammar.rs` via `grammar-tools generate-rust-compiled-grammars
javascript`. When no assignment operator follows the left-hand side, the
sequence fails fast and falls through to `conditional_expression` exactly as
before — so the change is purely additive: every non-assignment form (bare
identifier, member, call, binary, ternary, arrow, yield, `var` initializer)
still parses unchanged.

Added CLOC17 regression tests sweeping `EsVersion::ALL`: assignment / compound
/ member-target / right-associative-chain / ternary-RHS forms parse on every
version; every non-assignment form still parses; arrow/yield are unaffected on
es2015+; and `a = 1;` bridges to a typed `AssignmentExpression` (proving the
downstream optimization pipeline is unblocked, not merely the parser).

**Scope:** this PR regenerates the Rust parser only (closurec's parser). The
13 sibling-language `javascript-parser` packages embed their own generated
artifacts from the same `es*.grammar` sources and still carry the old ordering;
regenerating them is a tracked follow-up (no CI parity gate enforces it today).

## [0.8.0] - 2026-06-15

### Fixed — member-expression suffix chains were silently truncated

`grammar_to_program`'s `convert_member_expression` dropped every property
suffix past the first Node child. The early-return guard counted only the
*Node* children (`nodes.len() == 1`), but the grammar rule

```text
member_expression = primary_expression { DOT NAME | LBRACKET expr RBRACKET | … }
```

emits a **flat** child list: one primary Node followed by suffix *tokens*
(`.`, `NAME`) and Nodes (`[expr]`). With one Node child (`a`) but two suffix
tokens (`.`, `b`), `a.b` was misclassified as a bare primary and collapsed to
`a`; `a.b.c` collapsed to `a.c`; and `a.b(c)` produced the callee `a` — so a
method call like `console.log(x)` bridged (and emitted) as `console(x)`,
silently changing program meaning.

The conversion now walks the full suffix repetition left-to-right, folding each
`.NAME` and `[expr]` onto the growing base (mirroring the already-correct
`convert_optional_chain_expression`). A tagged-template suffix on a member base
is reported as `UnsupportedSyntax` (Phase 2) rather than mis-bridged.

- The bare-primary fast path now checks `node.children.len() == 1` (total
  children) instead of `nodes.len() == 1` (Node children only).
- **5 new bridge unit tests**: `member_dot_single` (`a.b`), `member_dot_chain`
  (`a.b.c`), `member_computed_then_dot` (`a[0].b`), `member_dot_then_computed`
  (`a.b[c]`), and `member_method_call_keeps_property` (`a.b(c)` keeps the
  `a.b` callee).

This bug was latent until now because the only consumer (closurec's SIMPLE
level) discarded the bridged `Program` and emitted via whitespace-only; wiring
the typed emitter exposed it.

## [0.7.0] - 2026-06-15

### Changed
- Transitive upgrade: `coding-adventures-javascript-lexer` 0.8.0 (via `lexer`
  0.5.0) fixes gap-044b — template literal substitutions with non-identifier
  expressions no longer produce a LexerError.  No API changes in this crate.

## [0.6.0] - 2026-06-14

### Added
- New dependency on `coding-adventures-javascript-ast` for the typed ESTree AST.
- `pub mod bridge` — `GrammarASTNode → javascript_ast::Program` bridge module (CLOC12.136). Converts the generic grammar tree produced by `GrammarParser` into the fully typed AST consumed by all downstream optimization passes.
- `pub fn parse_javascript_program(source, EsVersion) -> Result<Program, String>` — convenience entry point that parses AND bridges in one call.
- `bridge::grammar_to_program(&GrammarASTNode, EsVersion) -> Result<Program, BridgeError>` — the core converter.
- `bridge::BridgeError` — typed error with two variants:
  - `UnsupportedSyntax { rule, location }` — Phase 2+ syntax not yet in the typed AST (async, generators, classes, for-in/of, try-catch, destructuring, template literals, optional chaining, `new` expressions, update expressions, sequence expressions, computed property keys, spread elements). Callers should degrade gracefully to WHITESPACE_ONLY / identity output.
  - `InternalError { msg, rule }` — bug in the bridge (node shape mismatch). Should not occur on valid input.

### Bridge coverage (Phase 1 subset)
**Statements** (12 variants): `block`, `if/else`, `while`, `for`, `continue`, `break`, `return`, `throw`, `switch`/`case`/`default`, `labeled`, `empty`, `expression_statement`, `variable_statement` (`var`), `lexical_declaration` (`let`/`const`), `function_declaration`.

**Expressions** (15 variants): `Identifier`, `NumericLiteral`, `StringLiteral`, `BooleanLiteral` (true/false), `NullLiteral`, `UndefinedLiteral`, `BigIntLiteral`, `BinaryExpression` (all 21 operators), `LogicalExpression` (`&&`/`||`/`??`), `UnaryExpression` (7 prefix operators), `AssignmentExpression` (13 operators), `ConditionalExpression` (ternary), `CallExpression`, `MemberExpression` (dot and computed), `ArrayExpression`, `ObjectExpression` (init properties, shorthand).

**Grammar routing**: handles the `optional_chain_expression` intermediate rule (the grammar's general suffix-chain node for dot access, bracket access, and call expressions — not just `?.` chains), the `new_expression` pass-through, and binary expression left-fold for precedence chains (`additive`, `multiplicative`, `shift`, etc.).

### Notes
- v1: all produced nodes carry `cv: None`. Per-node CV threading (source-byte → IR → engine-clause provenance) is CLOC12.137.
- Standalone assignment expressions (`x = y;`) are not yet parseable by the underlying grammar parser (ordered alternation matches `conditional_expression` first). This is a grammar-level gap, not a bridge limitation.
- Phase 1 unsupported constructs return `Err(UnsupportedSyntax)` rather than panicking, allowing `closurec` to degrade to identity output for files containing them.

### Tests
30 tests total (20 bridge + 10 existing parser tests):
- Literals: `empty_program`, `numeric_literal`, `string_literal`, `boolean_literal_true`, `null_literal`
- Declarations: `var_declaration`, `let_declaration`, `const_declaration`
- Expressions: `binary_add`, `logical_and`, `call_expression_roundtrip`
- Statements: `if_statement_no_else`, `if_statement_with_else`, `while_statement_bridge`, `switch_statement_bridge`
- Functions: `function_declaration`, `return_with_value`
- Error paths: `do_while_is_unsupported`

## [0.5.0] - 2026-05-21

### Added
- New dependencies on `coding_adventures_correlation_vector` (for `CVLog`, `Origin`) and `serde_json` (for contribution `meta` JSON values).
- `pub struct ProgramWithCv { pub ast: GrammarASTNode, pub cv: String }` — packages a parsed AST with its program-root CV identifier.
- `parse_javascript_with_cv(source, source_file, EsVersion, &mut CVLog) -> Result<ProgramWithCv, String>` — full CV-plumbed parse per CLOC03 §"Stage 2 — Parser" (v1: root-only). Behavior:
  - Tokenizes via `tokenize_javascript_with_cv` so every token gets its own CV ID.
  - Runs the underlying `GrammarParser` on the unwrapped tokens.
  - Mints the program-root CV via `cv.merge(all_token_cv_ids, Origin{source: source_file, location: "0:0", …})` so the program CV has every token as an ancestor.
  - Appends `Contribution { source: "parser", tag: "constructed", meta: { rule: <root rule name>, version: <es version> } }` per CLOC03.
- Module docs added a "Correlation-vector plumbing" section linking to CLOC03 and noting that v1 is root-only.
- 5 new tests:
  - `parse_with_cv_assigns_a_program_id`
  - `parse_with_cv_program_id_resolves_in_log` — `cv.get(id)` returns an entry whose `Origin.source = source_file` and `Origin.location = "0:0"`.
  - `parse_with_cv_appends_constructed_contribution` — `cv.history(id)` contains a `(source="parser", tag="constructed")` entry whose meta carries the correct `rule` and `version`.
  - `parse_with_cv_program_has_token_ancestors` — `cv.ancestors(id)` is non-empty (the merge step worked).
  - `parse_with_cv_disabled_log_still_returns_ast` — `CVLog::new(false)` keeps the API shape; the parser does not panic and still returns a valid AST.

### Notes
- All existing APIs (string-based, typed, no-CV) are untouched. This PR is purely additive.
- v1 is **root-only**: per-AST-node CV propagation requires deeper plumbing into `GrammarParser` (which today produces a generic `GrammarASTNode` tree, not the typed `javascript-ast::Program`). That work happens in a follow-up PR alongside the AST-typed parser output.
- The merge approach (program CV inherits from all tokens) gives source-map generators a reasonable starting point even with root-only plumbing: every output byte that comes from the program node resolves to the leftmost token's `Origin`.

## [0.4.0] - 2026-05-21

### Added
- New dependency on `coding-adventures-javascript-tokens` for the shared `EsVersion` enum.
- `create_javascript_parser_typed(source, EsVersion) -> Result<GrammarParser, String>` — typed constructor; no unknown-version error path.
- `parse_javascript_typed(source, EsVersion) -> Result<GrammarASTNode, String>` — typed parser.
- `pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;` — typed default.
- New tests covering the typed APIs: `parse_typed_es2015`, `default_es_version_constant_is_es2025`, `all_typed_versions_load`, `create_parser_typed`.

### Notes
- The existing `&str`-based APIs are kept for backwards compatibility. Typed APIs are the preferred surface going forward.
- The typed parser delegates to `javascript-lexer`'s `tokenize_javascript_typed`, so token/grammar versions are guaranteed to come from the same ECMAScript edition.

## [0.3.0] - 2026-05-20

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/javascript.grammar`. The full ES1 through ES2025 grammars under `code/grammars/ecmascript/` supersede it.
- Removed the embedded `mod generic` block (~103 lines) from `_grammar.rs`.

### Changed
- Crate docstring no longer mentions the "generic" grammar.

### Migration
- Replace `parse_javascript(source, "")` with `parse_javascript(source, "es2025")` (or another explicit ES version).

### Notes
- Rust-only first step of CLOC01 Phase 1 stub retirement. Other language ports (Go, Python, TypeScript, Ruby) get equivalent follow-up PRs; the stub `.grammar` source file is preserved until all ports migrate.

## [0.2.0] - 2026-04-05

### Changed
- `create_javascript_parser(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarParser, String>` instead of panicking.
- `parse_javascript(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarASTNode, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` for versioned ECMAScript grammars stored in `grammars/ecmascript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same ECMAScript edition.
- New tests: `test_versioned_es2015`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_parser_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_javascript_parser(source)` — factory function that loads `javascript.grammar` and returns a configured `GrammarParser`.
- `parse_javascript(source)` — convenience function that parses JavaScript source and returns a `GrammarASTNode`.
- Loads grammar from `javascript.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, expressions, function declarations, if/else, while loops, for loops, multiple statements, empty programs, function calls, and the factory function.

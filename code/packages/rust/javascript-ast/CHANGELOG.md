# Changelog

All notable changes to the `coding-adventures-javascript-ast` crate will be documented in this file.

## [0.14.0] - 2026-07-01

### Added — CLOC12.149: `FunctionExpression` (function in value position)

Adds `FunctionExpression { cv, id: Option<Identifier>, params, body, generator,
is_async }` and the `Expression::FunctionExpression` variant — the expression
sibling of `FunctionDeclaration`, covering `var f = function () {}`, IIFEs
(`(function () {})()`), and function-valued properties/arguments. The one
structural difference from the declaration is `id: Option<Identifier>`: an
anonymous `function () {}` has no name, and a *named* function expression's name
is body-local (visible only for self-recursion inside its own body), never bound
in the enclosing scope. `params` / `body` / `generator` / `is_async` reuse
`FunctionParam` and `BlockStatement`, so a pass walking a function body does not
care which form produced it. `is_async` serializes as JSON `"async"` per ESTree,
and `id`/`cv` are omitted from the wire format when `None`.

This is the **AST-node slice** of a bottom-up `FunctionExpression` rollout: the
parser→typed-AST bridge (`javascript-parser` currently declines
`function_expression` as `UnsupportedSyntax`), emitter printing, and pass
traversal land in follow-on slices. Arrow functions, methods, getters/setters,
and class expressions remain Phase 3.

## [0.13.0] - 2026-06-20

### Changed — CLOC24: `DebuggerStatement` doc reflects stripping

Doc-comment-only change: the `DebuggerStatement` node doc no longer says
"stripping it … is a future enhancement". `closure-pass-dce` now strips
`debugger` statements at SIMPLE/ADVANCED (CLOC24), so the comment is updated to
describe the live behaviour. No structural or behavioural change to the node.

## [0.12.0] - 2026-06-20

### Added — CLOC23: `ForOfStatement` (`for (left of right) body`)

Adds `ForOfStatement { cv, left: ForInit, right: Expression, body: Box<Statement> }`,
a new `TaggedStatement::ForOfStatement` variant, and a
`Statement::for_of_statement` constructor. Structurally identical to
`ForInStatement` (only the `of` vs `in` keyword and the iteration protocol
differ), so the `left` reuses [`ForInit`] the same way. Making it representable
lets the closurec CLI optimize for-of loops; previously any such program fell
back to WHITESPACE_ONLY. (Destructuring left-hand sides and `using` bindings are
not represented — the bridge declines them.)

## [0.11.0] - 2026-06-20

### Added — CLOC22: `ForInStatement` (`for (left in right) body`)

Adds `ForInStatement { cv, left: ForInit, right: Expression, body: Box<Statement> }`,
a new `TaggedStatement::ForInStatement` variant, and a
`Statement::for_in_statement` constructor. The `left` reuses [`ForInit`]:
`VariableDeclaration` for `for (var/let/const k in o)` (a single-declarator
binding, no initializer) and `Expression` for `for (k in o)` (an existing
assignment target). ESTree wire format:
`{ "type": "ForInStatement", "left": …, "right": <Expression>, "body": <Statement> }`.
Making it representable lets the closurec CLI optimize for-in loops;
previously any such program fell back to WHITESPACE_ONLY. (Destructuring
left-hand sides are not represented — the bridge declines them.)

## [0.10.0] - 2026-06-20

### Added — CLOC21: `DebuggerStatement` (`debugger;`)

Adds `DebuggerStatement { cv }`, a new `TaggedStatement::DebuggerStatement`
variant, and a `Statement::debugger_statement` constructor. Like
`EmptyStatement` it carries no children. ESTree wire format:
`{ "type": "DebuggerStatement" }`. Making it representable lets the closurec
CLI optimize the rest of a program containing a `debugger` statement;
previously any such program fell back to WHITESPACE_ONLY. v1 preserves the
statement (stripping it, as upstream Closure does, is future work).

## [0.9.0] - 2026-06-20

### Added — CLOC20: `DoWhileStatement` (the test-after-body loop)

Adds `DoWhileStatement { cv, body: Box<Statement>, test: Expression }`, a new
`TaggedStatement::DoWhileStatement` variant, and a `Statement::do_while_statement`
constructor. This is the mirror of `WhileStatement` with the field order
following execution order (`body` before `test`) — a `do`-`while` runs its body
at least once *before* the test is first evaluated. ESTree wire format:

```json
{ "type": "DoWhileStatement", "body": <Statement>, "test": <Expression> }
```

Making it representable lets the closurec CLI optimize programs that use a
do-while loop; previously any such program fell back to WHITESPACE_ONLY.

## [0.8.0] - 2026-06-20

### Added — CLOC19: `TryStatement` + `CatchClause` (closes the try/catch coverage gap)

Adds `TryStatement { cv, block, handler: Option<CatchClause>, finalizer: Option<BlockStatement> }`
and `CatchClause { cv, param: Option<Identifier>, body: BlockStatement }`, a new
`TaggedStatement::TryStatement` variant, and a `Statement::try_statement`
convenience constructor. This makes `try`/`catch`/`finally` representable in the
typed AST for the first time — previously any program containing `try` could not
be lowered and the closurec CLI fell back to WHITESPACE_ONLY (zero optimization).

ESTree wire format:

```json
{
  "type": "TryStatement",
  "block": <BlockStatement>,
  "handler": { "type": "CatchClause", "param": <Identifier> | (absent), "body": <BlockStatement> } | (absent),
  "finalizer": <BlockStatement> | (absent)
}
```

`param` is absent for the ES2019 optional-catch-binding form (`catch { … }`).
Destructuring catch params are intentionally not modelled — the bridge declines
them rather than mis-binding (sound WHITESPACE_ONLY fallback at the CLI).

### Fixed — serde double-tagging on `TryStatement`

The initial `TryStatement` struct carried its own `#[serde(tag = "type")]` *and*
was a variant of the internally-tagged `TaggedStatement` enum, which injects the
tag from the variant name. The two tags collided and a serialized `TryStatement`
failed to deserialize back into the untagged outer `Statement` enum. Removed the
struct-level tag so it matches every sibling statement struct (only `rename_all`);
added round-trip tests covering the full, optional-binding, and no-catch forms.

## [0.7.0] - 2026-06-04

### Added — CLOC12.33: `SwitchStatement` + `SwitchCase` (Phase 1.x, closes gap-014)

Adds `SwitchStatement { cv, discriminant, cases: Vec<SwitchCase> }`
and `SwitchCase { cv, test: Option<Expression>, consequent: Vec<Statement> }`.
A new `TaggedStatement::SwitchStatement` variant and a
`Statement::switch_statement` convenience constructor round out
the surface.

ESTree wire format:

```json
{
  "type": "SwitchStatement",
  "discriminant": <Expression>,
  "cases": [
    { "type": "SwitchCase", "test": <Expression> | null, "consequent": [<Statement>...] },
    ...
  ]
}
```

`SwitchCase.test` is `None` (serialised as `null`) for the
`default:` clause; the parser is responsible for the at-most-one-
default invariant per ECMAScript §13.12, the AST doesn't enforce
it.

Three new roundtrip tests cover empty / case+default / untraced
cases plus inner `"type": "SwitchCase"` tag verification.

This unblocks the DCE port's `testRemoveSwitch*` cases and the
fold-control-flow port's `testRemoveEmptySwitch` case. The
peephole optimisations that consume this node — empty-switch
elimination, constant-discriminant collapse — are gap-014
follow-ups.

## [0.6.0] - 2026-06-01

### Added — CLOC12.16: `UndefinedLiteral` Expression variant (Phase 1.x, closes gap-001)

Adds `UndefinedLiteral { cv: Option<CvId> }` and its
`Expression::UndefinedLiteral` arm. No value field — there is
exactly one `undefined`.

Wire format:

```json
{ "type": "UndefinedLiteral" }
```

**Note: `undefined` is technically an identifier in ECMAScript,
not a reserved word.** `var undefined = 1;` is legal in non-strict
mode and shadows the global. ESTree historically modelled it as
`Identifier { name: "undefined" }`; we follow the modern typed
variant approach so passes can pattern-match on the leaf without
first checking the identifier name. The emitter renders it as
`void 0` (shadow-safe — see closure-emitter CHANGELOG).

Two new roundtrip tests cover traced and untraced cases.

This closes the final hole in CLOC12.09's typeof-literal fold
table: constant-fold now folds `typeof <UndefinedLiteral>` to
`"undefined"`.

## [0.5.0] - 2026-06-01

### Added — CLOC12.15: `BigIntLiteral` Expression variant (Phase 1.x, closes gap-021)

Adds `BigIntLiteral { cv: Option<CvId>, value: String, raw: String }`
and its `Expression::BigIntLiteral` arm.

Wire format:

```json
{ "type": "BigIntLiteral", "value": "123", "raw": "123n" }
```

`value` is the **decimal expansion** of the bigint as a JSON string
(per ESTree's bigint-as-string convention — bigints can exceed the
double-precision range that JSON `number` can faithfully represent).
`raw` preserves the original source representation including the
trailing `n` suffix and the source radix, so a literal written as
`0x1fn` round-trips as `0x1fn` rather than `31n`.

Matches ESTree exactly. The `-` in `-123n` is a `UnaryExpression`
over a `BigIntLiteral`, never part of the literal itself.

Two new roundtrip tests cover decimal-source (`123n`) and
hex-source (`0x1fn`) cases.

This unblocks gap-021 modelling. No optimisation rides on it yet —
passes treat a `BigIntLiteral` as already-folded the same way they
treat `NumericLiteral`. The downstream `typeof <BigIntLiteral>`
fold (constant-fold pass) does evaluate to `"bigint"` per
ECMAScript §UnaryTypeofExpression.

## [0.4.0] - 2026-06-01

### Added — CLOC12.14: `ThrowStatement` (Phase 1.x, closes gap-020)

Adds `ThrowStatement { cv: Option<CvId>, argument: Expression }` and
its `TaggedStatement::ThrowStatement` arm, plus the
`Statement::throw_statement(...)` convenience constructor.

Wire format:

```json
{ "type": "ThrowStatement", "argument": { "type": "NumericLiteral", "value": 1.0, ... } }
```

Per ECMAScript §13.14 the `argument` field is non-optional — `throw;`
with no value is a SyntaxError. Matches ESTree exactly.

Two new roundtrip tests cover `throw 1;` (traced) and `throw e;`
(untraced; asserts the `cv` key is omitted from the JSON wire
format).

This unblocks the upstream Closure-Compiler fold-control-flow port's
`testMinimizeIfWithThrow` test case
(`if (x) foo(); else throw e;` → `if (!x) throw e; foo();`). The
actual *rewriting* is a separate optimisation tracked under the
gap-020 follow-up; modelling the AST node is its prerequisite.

## [0.3.0] - 2026-06-01

### Added — CLOC12.13: `LabeledStatement` (Phase 1.x, closes gap-009)

Adds the `LabeledStatement { cv: Option<CvId>, label: Identifier,
body: Box<Statement> }` variant and its `TaggedStatement::LabeledStatement`
arm, plus the `Statement::labeled_statement(...)` convenience
constructor. The `BreakStatement` already existed in the original
Phase 1 implementation (the gap title was misleading — only the
label-wrapping node was missing).

This unblocks the upstream Closure-Compiler DCE port's
`testRemoveNoOpLabelledStatement` test case (`a: break a;`). The
actual *collapse* of a useless labelled-self-break to empty is a
separate optimisation tracked under the gap-009 follow-up; modelling
the AST node is its prerequisite.

Wire format:

```json
{
  "type": "LabeledStatement",
  "label": { "type": "Identifier", "name": "a" },
  "body": { "type": "BreakStatement", "label": { "type": "Identifier", "name": "a" } }
}
```

Matches ESTree exactly. CV id is optional (untraced ASTs omit the
`cv` key, per the CLOC09 amendment).

Two new roundtrip tests cover `a: break a;` and `outer: { ; }`.

### Note — Version bump reconciliation

The pre-existing `[0.2.0] - 2026-05-24` entry below documents the
Phase 1 ESTree-compat scaffolding work, but Cargo.toml never moved
off `0.1.0` at the time. CLOC12.13 bumps Cargo straight to `0.3.0`
to (a) flag the new Phase 1.x variant addition and (b) bring the
two source-of-truth values back in sync. Future minor bumps
follow the manifest from here on.

## [0.2.0] - 2026-05-24

### Added (CLOC09 Phase 1 — full ESTree-compat node taxonomy)
- 25 new node variants across three module files (`statement.rs`, `expression.rs`, `declaration.rs`) implementing CLOC09 Phase 1:
  - **Statements (10)**: `ExpressionStatement`, `BlockStatement`, `IfStatement`, `WhileStatement`, `ForStatement` (with `ForInit` enum), `ReturnStatement`, `BreakStatement`, `ContinueStatement`, `EmptyStatement`, and a `Statement::Declaration(Declaration)` untagged wrap arm (matches ESTree's flatter wire shape where declarations appear as statements).
  - **Expressions (14)**: `Identifier`, `NumericLiteral`, `StringLiteral`, `BooleanLiteral`, `NullLiteral`, `BinaryExpression` (+ `BinaryOperator` enum), `LogicalExpression` (+ `LogicalOperator`), `UnaryExpression` (+ `UnaryOperator`), `AssignmentExpression` (+ `AssignmentOperator` + `AssignmentTarget`), `ConditionalExpression`, `CallExpression`, `MemberExpression`, `ArrayExpression`, `ObjectExpression` (+ `Property` + `PropertyKind` + `PropertyKey`).
  - **Declarations (2)**: `VariableDeclaration` (+ `VarKind`, `VariableDeclarator`, `BindingTarget`), `FunctionDeclaration` (+ `FunctionParam`).
- `Program` extended with `body: Vec<ProgramItem>` where `ProgramItem` is an untagged union of `Statement | Declaration`. Defaults to empty so existing callers don't churn.
- `ProgramItem::with_body(...)` builder helper.
- `Program::new_untraced(version, source_type)` constructor for the tracing-disabled mode per the CLOC09 amendment.
- Full ESTree wire format compatibility: `#[serde(tag = "type")]` on each node enum, `#[serde(rename_all = "camelCase")]` on every struct, operator enums serialize to ESTree-canonical strings (`"+"`, `"==="`, `"&&"`, `"typeof"`, etc.), `VarKind` and `PropertyKind` serialize lowercase (`"var"`/`"let"`/`"const"`, `"init"`/`"get"`/`"set"`).
- Per-node `cv` field is `Option<CvId>` with `#[serde(skip_serializing_if = "Option::is_none", default)]` per the CLOC09 amendment — untraced ASTs match ESTree's wire format byte-for-byte (no `cv` key in output).
- `is_async` field on `FunctionDeclaration` serializes as JSON `"async"` to match ESTree (Rust's `async` is a reserved keyword).
- 50 tests covering: per-variant round-trip in both traced and untraced modes, JSON shape pinning each variant's `"type"` tag matches ESTree-spec name, every operator value round-trips with the canonical source-text spelling, `is_async` ↔ `"async"` JSON rename, declaration untagged-wrap collapses to inner type tag, optional fields are omitted from JSON when `None`, traced + untraced `Program` round-trip end-to-end.

### Changed
- `Program.cv: CvId` → `Program.cv: Option<CvId>` per CLOC09 amendment. `Program::new(cv, version, source_type)` keeps its existing signature but wraps the cv in `Some` internally, so all 11+ downstream call sites (every pass crate, the emitter, the typechecker) keep working without changes.
- `Program::new` body defaults to empty `Vec` — additive change.
- `Eq` derive removed from `Program` (NumericLiteral.value is `f64` which has no `Eq`); `PartialEq` is preserved.
- Cargo.toml gains `serde` (with `derive`) as a dependency and `serde_json` as a dev-dependency for round-trip tests.
- `closure-pass-pipeline`: the CV contribution loop and the FixedPoint diagnostic emission both gracefully handle `Program.cv == None` (skip the contribute calls; use empty-string cv on the diagnostic; tracked as Phase 1.x followup to migrate `Diagnostic.cv` to `Option<String>`).
- `closure-typechecker::check`: gracefully handles `Program.cv == None` by skipping the root-node judgment.

### Notes
- `MemberExpression.property` is `Box<Expression>` directly (the earlier `MemberProperty` enum was removed because untagged-enum disambiguation on identical JSON shapes is fundamentally ambiguous). When `computed: false`, the parser is required to emit an `Identifier` as the property; tools that walk the tree can assert this if they care. Matches ESTree's wire format precisely.
- A deprecated `MemberProperty = Box<Expression>` type alias is kept as a stub for the brief window between spec write-up and implementation.
- Phase 2 will add `BigIntLiteral`, `RegExpLiteral`, `TemplateLiteral`, control-flow gaps (`SwitchStatement`, `TryStatement`, etc.), `SequenceExpression`, `UpdateExpression`, `NewExpression`, `SpreadElement`, `ThisExpression`, `SuperExpression`.
- Phase 3 adds patterns (destructuring), arrow functions, classes.
- Phase 4 adds modules.
- Phase 5 adds async / generators / nullish / optional chaining / `&&=` `||=` `??=`.

## [0.1.0] - 2026-05-21

### Added
- New crate scaffolded per CLOC02 Phase 1.
- `Program` struct: AST root carrying `cv: CvId`, `version: EsVersion`, and `source_type: SourceType`. Per CLOC02, the version tag lives only on `Program` — never on individual nodes.
- `SourceType` enum (`Script` / `Module`) with derives `Debug, Clone, Copy, PartialEq, Eq, Hash`.
- `CvId` type alias for `String` matching the current `correlation-vector` representation (see module-level docs for the migration plan to a true newtype).
- `Program::new(cv, version, source_type)` constructor.
- Module-level docs enumerate the six backend-agnostic invariants from CLOC02 and the dependency whitelist.
- Test suite covering: synthetic construction with each `SourceType`, `Clone` + `PartialEq`, compile-time `Copy` assertions on `SourceType` and `EsVersion`.

### Notes
- Dependencies are exactly `coding-adventures-correlation-vector` and `coding-adventures-javascript-tokens`. No serde for v1 — round-trippable JSON ships in a follow-up once consumers actually need it.
- The `Statement` / `Expression` / `Declaration` / class / module / literal variants from CLOC02 are deferred to follow-up PRs to keep this scaffolding small.

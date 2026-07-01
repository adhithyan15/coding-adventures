# CLOC09 — JavaScript AST Taxonomy

## What this spec locks down

CLOC09 defines the full node taxonomy of `javascript-ast`. After
the Stage 1–4 scaffold loop (CLOC01–CLOC08), `javascript-ast`
shipped just `Program` and `SourceType`. Every optimization
pass, the type-checker, and the emitter were therefore identity.
This spec catalogues the variants the AST grows to so those
bodies can fill in.

The spec is binding for the implementation PRs that follow it,
in order:

- Phase 1 (this round): ~25 variants — enough to express
  `function f(x){ return x + 1; } const a = f(7);` and to give
  `constant-fold` and `dce` real work.
- Phase 2: leaf coverage gaps (`BigIntLiteral`, `RegExpLiteral`,
  `TemplateLiteral`), update/unary/spread, `SwitchStatement`,
  `TryStatement`.
- Phase 3: patterns (destructuring), arrow functions, classes.
- Phase 4: modules (`import` / `export` declarations).
- Phase 5: async / generators, optional chaining, nullish
  coalescing, `??=` `||=` `&&=` assignment operators.

This document focuses on Phase 1; subsequent phases extend the
same enums additively.

## Three load-bearing design decisions

### 1. Full ESTree compatibility

The AST mirrors the [ESTree
spec](https://github.com/estree/estree) — the de facto
interchange format used by every major JS tool (Babel, ESLint,
Acorn, Espree, Prettier, Rollup, esbuild, swc, TypeScript). Node
type names, field names, and per-variant shapes match ESTree
exactly.

**Why**: ESTree is the result of 10+ years of work by the JS
tooling community to converge on a lossless representation of
ECMAScript programs. Reusing it gets us:

- a known-rich representation that handles every ES2025
  construct (the underlying motivation is that *any* JS engine
  has to handle them all),
- interop with the broader ecosystem — a debugger, viz, or
  linter that already understands ESTree can read our AST,
- no design churn from arguments we'd otherwise have to
  resolve internally (where should `optional` go on
  `CallExpression`? what's the shape of a spread element? ESTree
  has answers).

We deviate from ESTree only in **two** ways:

1. **No source ranges or location objects.** ESTree nodes carry
   `loc: SourceLocation | null` and `range: [number, number] |
   null`. Per [CLOC02](./CLOC02-javascript-ast-design.md) our
   nodes carry only a `CvId` and look up source positions
   through the correlation-vector graph. This is the structural
   invariant that lets every optimization pass rewrite the AST
   without invalidating source maps.
2. **Rust idioms** for naming and field types. `Box<Expression>`
   instead of `Expression | null` for nullable children; Rust
   `enum` with `#[serde(tag = "type")]` to match ESTree's
   `{ "type": "IfStatement", ... }` JSON wire format on
   serialize. ESTree's `null`-able pieces (e.g. the `alternate`
   of `IfStatement`) become `Option<Box<...>>`.

### 2. AST is V8-frontend-ready

The user's project plan includes a future V8-on-LANG-VM
implementation that consumes this same AST. The V8 frontend
will lower the AST into LANG VM bytecode rather than into JS
source text, but it consumes the same node taxonomy the
Closure-style passes do.

This places one hard constraint on the AST: **it must be
lossless enough that an interpreter / JIT can implement exact
ECMAScript semantics from it.** A few examples of what
"lossless" demands:

- `VariableDeclaration` must distinguish `var` / `let` /
  `const` (scope semantics differ across all three).
- `FunctionDeclaration` / `FunctionExpression` /
  `ArrowFunctionExpression` are three different variants
  (`this` binding semantics differ).
- `BlockStatement` must be a distinct variant from a bare
  sequence of statements (block scoping for `let`/`const`).
- `ObjectExpression` preserves property order (insertion order
  is observable via `Object.keys` per ES2015+).
- `MemberExpression` distinguishes `computed: true` (`a[b]`)
  from `computed: false` (`a.b`) — meaning differs when the key
  is a `Symbol`.
- `BinaryExpression` and `LogicalExpression` are distinct
  variants — short-circuit semantics of `&&` / `||` / `??` matter.
- `AssignmentExpression` is distinct from `BinaryExpression`
  (assignment evaluates RHS then LHS, with side effects).
- `Literal` is split into typed variants (`NumericLiteral`,
  `StringLiteral`, `BooleanLiteral`, `NullLiteral`, …) so the
  type carries through statically without re-parsing the raw
  text. (ESTree uses a single `Literal` with `value: any`; this
  is one of the few places our Rust idioms make the typed split
  natural — and it's lossless: the underlying value's type is
  what matters.)

ESTree was designed to be lossless across the full ECMAScript
spec; it satisfies the V8-readiness constraint by construction.
This spec calls out the constraint explicitly so future PRs
don't accidentally optimize toward "what Closure-style passes
need" at the cost of "what an interpreter needs."

### 3. Optional per-node `CvId` is the only identity

Every node carries one optional `CvId` field, named `cv`:

```rust
pub struct IfStatement {
    pub cv: Option<CvId>,
    pub test: Expression,
    pub consequent: Box<Statement>,
    pub alternate: Option<Box<Statement>>,
}
```

No spans, no parent pointers, no node IDs. When `cv` is
populated, the correlation-vector graph stores the actual
`(file, byte_start, byte_end)` triple keyed by that `CvId`.

#### Optionality is first-class — both modes work

The user opts into CV tracing per-program, not per-pass and not
per-build. Two equally-supported modes:

1. **Tracing enabled** — every node is constructed with
   `cv: Some(id)`. The frontend assigns ids during lex/parse;
   passes fork new ids when they rewrite a node; the emitter
   reads `cv.expect(...)` on each token and writes a source-map
   mapping. Full source-map support, full provenance queries
   through the CV graph.

2. **Tracing disabled** — every node is constructed with
   `cv: None`. The frontend skips id assignment, passes don't
   fork (there's nothing to fork from), the emitter writes no
   source map. Useful for:
   - synthetic test programs constructed in Rust (no real source
     to map back to),
   - quick code-transform tools that just want fold/rename/emit
     without source-map overhead,
   - downstream consumers like a future V8-on-LANG-VM that
     produces its own debugger metadata and doesn't need
     CV-graph queries.

Modes are **per-program**, not mixed. Mixing within one program
is supported (an isolated `None` node in an otherwise-traced
tree is fine — the pipeline behavior below defines how passes
treat it), but the common case is "the whole tree is traced or
none of it is."

#### How passes behave in each mode

A pass's `Pass::run` body, when handling a node, follows this
rule:

```rust
let new_cv = match node.cv {
    Some(parent_id) => Some(ctx.cv.fork(parent_id)),  // tracing on
    None => None,                                      // tracing off
};
let new_node = SomeStatement { cv: new_cv, /* … */ };
```

In the `Some` arm the pass also emits a `Contribution` to
`ctx.cv` per CLOC03, marking what kind of rewrite happened. In
the `None` arm the pass *skips* contribution emission for that
node (there's no id to attribute it to).

Concretely: if every node in the input has `cv: None`, the pass
runs to completion and emits zero contributions and zero CV
entries. The CV log is empty at the end. The emitter then sees
`cv: None` everywhere and writes `source_map: Some(empty_v3_blob)`
or `source_map: None` (per `EmitOptions::source_map`) — the blob
has no mappings inside because there's nothing to map.

If the input is fully traced, the pass forks ids, emits
contributions, and the emitter writes a full source map.

The pipeline thus has **one behavior** for both modes — it
doesn't branch on a "tracing enabled?" flag. The `Option`
discriminator on each node is the only switch.

#### Why optional, not mandatory

Earlier drafts of this spec mandated `cv: CvId` (non-optional) on every node.
Three reasons that turned out to be the wrong call:

1. **Synthetic tests are painful.** Passes have unit tests that
   construct nodes directly. Forcing every test fixture to fork
   a fresh `CvId` adds boilerplate to every test for no win.
2. **Downstream consumers vary.** A V8-on-LANG-VM frontend that
   only cares about lowering to bytecode shouldn't have to
   participate in CV bookkeeping. Making it optional lets that
   consumer use the AST without dragging in the correlation
   vector graph.
3. **Future codegen / macro-expansion.** AST nodes constructed
   by macro expansion (Phase 3+) or by a `@JsxFactory`-style
   transform don't have a meaningful source position. They
   should emit nodes with `cv: None` rather than fabricate fake
   ids — the latter creates source maps that point to nonsense.

The cost is one byte per node (the `Option` discriminant) plus
slightly more boilerplate inside pass bodies. The win is that
both modes are honest about what they're doing.

#### Why one identity (still), not many

Even though `cv` is optional, it remains the **only** identity
field. We still don't carry spans / parent pointers / node IDs:

- **Source-map preservation across passes (when tracing).**
  Every pass forks ids; the old CvId stays in the log marked
  `deleted`; the chain `original_source_byte → original_ast →
  folded_ast → renamed_ast → emitted_token` is queryable through
  the CV graph long after the AST itself has moved on.
- **No parent pointers**. ESTree allows them (some tools attach
  them) but they make every mutation a graph-fixup. Our passes
  produce *new* trees rather than mutating in place; if a pass
  needs the parent it threads it through the recursion.
- **No span on the node**. The span lives in the CV graph (when
  tracing) or in the parser's input buffer (when not — the
  parser may surface span info via diagnostics out-of-band, but
  the AST itself stays span-free).

## Wire format

Every variant serializes to JSON with a `"type": "..."` tag
matching ESTree exactly. Internal field names are camelCase to
match ESTree, applied via `#[serde(rename_all = "camelCase")]`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Statement {
    ExpressionStatement(ExpressionStatement),
    BlockStatement(BlockStatement),
    IfStatement(IfStatement),
    // ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IfStatement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub test: Expression,
    pub consequent: Box<Statement>,
    pub alternate: Option<Box<Statement>>,
}
```

A node serialized to JSON is exactly the ESTree shape modulo
the `cv` field replacing `loc` / `range`. The `skip_serializing_if`
attribute means a `None` cv is omitted from JSON entirely (rather
than serialized as `"cv": null`) — so traced AST output carries
the `cv` key, untraced output omits it. JSON-emitting tools
(debug dumps, `--print_tree_json`) match what Babel / Acorn
output for the same source.

## Top-level structure

```rust
pub struct Program {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub version: EsVersion,         // already present
    pub source_type: SourceType,    // already present
    pub body: Vec<ProgramItem>,     // NEW in Phase 1
}

pub enum ProgramItem {
    Statement(Statement),
    Declaration(Declaration),
    // Phase 4: ModuleDeclaration(ModuleDeclaration),
}
```

`Program.body` is a `Vec` of either statements or declarations.
ESTree models top-level as `Statement | ModuleDeclaration` and
lifts declarations into the statement enum; we keep them as a
separate enum because most passes care about declarations
specifically (renaming, treeshaking, remove-unused-vars).

## Phase 1 — Statement variants

| Variant               | Fields                                                                                                                                              |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ExpressionStatement` | `cv: Option<CvId>`, `expression: Expression`                                                                                                                |
| `BlockStatement`      | `cv: Option<CvId>`, `body: Vec<Statement>` (Declarations land via Phase 1's `Statement::Declaration` lift — see below)                                      |
| `IfStatement`         | `cv: Option<CvId>`, `test: Expression`, `consequent: Box<Statement>`, `alternate: Option<Box<Statement>>`                                                   |
| `WhileStatement`      | `cv: Option<CvId>`, `test: Expression`, `body: Box<Statement>`                                                                                              |
| `ForStatement`        | `cv: Option<CvId>`, `init: Option<ForInit>`, `test: Option<Expression>`, `update: Option<Expression>`, `body: Box<Statement>`                               |
| `ReturnStatement`     | `cv: Option<CvId>`, `argument: Option<Expression>`                                                                                                          |
| `BreakStatement`      | `cv: Option<CvId>`, `label: Option<Identifier>`                                                                                                             |
| `ContinueStatement`   | `cv: Option<CvId>`, `label: Option<Identifier>`                                                                                                             |
| `LabeledStatement`    | `cv: Option<CvId>`, `label: Identifier`, `body: Box<Statement>` — Phase 1.x (CLOC12.13)                                                                     |
| `ThrowStatement`      | `cv: Option<CvId>`, `argument: Expression` — Phase 1.x (CLOC12.14)                                                                                          |
| `EmptyStatement`      | `cv: Option<CvId>`                                                                                                                                          |
| `Declaration`         | wraps `Declaration` so a top-level or block-scoped declaration is also a `Statement` (matches ESTree's lift of `VariableDeclaration` etc. into `Statement`) |

```rust
pub enum ForInit {
    VariableDeclaration(VariableDeclaration),
    Expression(Expression),
}
```

(Phase 2 adds `SwitchStatement`, `TryStatement`,
`DoWhileStatement`, `ForInStatement`, `ForOfStatement`,
`DebuggerStatement`, `WithStatement`.)

### Phase 1.x amendments (post-CLOC09 ratification)

- **CLOC12.13 — `LabeledStatement`.** The upstream Closure-Compiler
  DCE test suite includes `testRemoveNoOpLabelledStatement` which
  folds `a: break a;` to empty. The collapse optimisation itself is
  a separate gap, but modelling the AST node is its blocking
  prerequisite — so `LabeledStatement` is lifted from Phase 2 into
  Phase 1.x to unblock the test port. Adding it is binary-compatible:
  existing tagged-statement consumers had to gain one match arm in
  constant-fold, fold-control-flow, and DCE, all of which recurse
  into the labelled body and leave the label untouched.
- **CLOC12.14 — `ThrowStatement`.** The upstream Closure-Compiler
  fold-control-flow test `testMinimizeIfWithThrow` rewrites
  `if (x) foo(); else throw e;` into `if (!x) throw e; foo();`.
  The rewrite is its own follow-up gap, but the structural
  prerequisite — modelling `throw expr;` as an AST node — is
  lifted from Phase 2 into Phase 1.x here. Per ECMAScript §13.14
  the `argument` field is non-optional: `throw;` is a SyntaxError.
  Existing pass crates gained one match arm each that folds the
  argument expression and preserves the throw semantics.
- **CLOC12.15 — `BigIntLiteral`.** Adds the BigInt primitive
  literal as a sixth leaf in the Expression variants list. Per
  ESTree's bigint-as-JSON-string convention, the `value` field is
  the **decimal expansion of the bigint as a string** rather than
  a `f64` (bigints can exceed the double-precision range that JSON
  `number` can faithfully represent). `raw` keeps the original
  source spelling, so a literal written as `0x1fn` round-trips as
  `0x1fn` rather than `31n`. The emitter writes `raw` verbatim —
  no normalisation, no shortest-form rewriting (there is no
  exponential bigint syntax). Bigint arithmetic folding is **not**
  implemented in CLOC12.15 (would require a bigint runtime in
  constant-fold); however the `typeof <BigIntLiteral>` → `"bigint"`
  fold IS implemented since it requires no arithmetic.
- **CLOC12.16 — `UndefinedLiteral`.** Adds a typed `undefined`
  leaf node so passes can pattern-match on it without first
  checking an `Identifier`'s name. Note that in ECMAScript
  `undefined` is technically an *identifier*, not a reserved word
  — `var undefined = 1;` is legal in non-strict mode and shadows
  the global. ESTree historically modelled it as
  `Identifier { name: "undefined" }`; we follow the modern typed-
  variant approach. The emitter renders it as `void 0` — shadow-
  safe, since `void <expr>` always produces the genuine undefined
  value regardless of any name in scope. The `typeof
  <UndefinedLiteral>` → `"undefined"` fold closes the last hole
  in CLOC12.09's typeof-literal fold table. Partially closes
  gap-001 (the `NaN` and `Infinity` cases remain).

## Phase 1 — Expression variants

| Variant                 | Fields                                                                                                                                              |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Identifier`            | `cv: Option<CvId>`, `name: String`                                                                                                                          |
| `NumericLiteral`        | `cv: Option<CvId>`, `value: f64`, `raw: String`                                                                                                             |
| `StringLiteral`         | `cv: Option<CvId>`, `value: String`, `raw: String`                                                                                                          |
| `BooleanLiteral`        | `cv: Option<CvId>`, `value: bool`                                                                                                                           |
| `NullLiteral`           | `cv: Option<CvId>`                                                                                                                                          |
| `BigIntLiteral`         | `cv: Option<CvId>`, `value: String`, `raw: String` — Phase 1.x (CLOC12.15)                                                                                  |
| `UndefinedLiteral`      | `cv: Option<CvId>` — Phase 1.x (CLOC12.16)                                                                                                                  |
| `BinaryExpression`      | `cv: Option<CvId>`, `operator: BinaryOperator`, `left: Box<Expression>`, `right: Box<Expression>`                                                           |
| `LogicalExpression`     | `cv: Option<CvId>`, `operator: LogicalOperator`, `left: Box<Expression>`, `right: Box<Expression>`                                                          |
| `UnaryExpression`       | `cv: Option<CvId>`, `operator: UnaryOperator`, `prefix: bool`, `argument: Box<Expression>`                                                                  |
| `AssignmentExpression`  | `cv: Option<CvId>`, `operator: AssignmentOperator`, `left: AssignmentTarget`, `right: Box<Expression>`                                                      |
| `ConditionalExpression` | `cv: Option<CvId>`, `test: Box<Expression>`, `consequent: Box<Expression>`, `alternate: Box<Expression>`                                                    |
| `CallExpression`        | `cv: Option<CvId>`, `callee: Box<Expression>`, `arguments: Vec<Expression>`                                                                                 |
| `MemberExpression`      | `cv: Option<CvId>`, `object: Box<Expression>`, `property: MemberProperty`, `computed: bool`                                                                 |
| `ArrayExpression`       | `cv: Option<CvId>`, `elements: Vec<Option<Expression>>` (`None` represents an elision: `[1, , 3]`)                                                          |
| `ObjectExpression`      | `cv: Option<CvId>`, `properties: Vec<Property>` (insertion order preserved per ES2015+)                                                                     |

```rust
pub enum BinaryOperator {
    Eq, NotEq, StrictEq, StrictNotEq,
    Lt, LtEq, Gt, GtEq,
    LeftShift, RightShift, UnsignedRightShift,
    Add, Sub, Mul, Div, Mod, Exp,
    BitOr, BitXor, BitAnd,
    In, InstanceOf,
}

pub enum LogicalOperator { And, Or, NullishCoalescing }

pub enum UnaryOperator { Negate, Plus, Not, BitNot, TypeOf, Void, Delete }

pub enum AssignmentOperator {
    Eq,                                       // =
    AddEq, SubEq, MulEq, DivEq, ModEq, ExpEq,
    LeftShiftEq, RightShiftEq, UnsignedRightShiftEq,
    BitOrEq, BitXorEq, BitAndEq,
    // Phase 5 adds: LogicalAndEq (&&=), LogicalOrEq (||=), NullishCoalescingEq (??=)
}

pub enum MemberProperty {
    Identifier(Identifier),         // `obj.foo`            (computed = false)
    Expression(Expression),         // `obj[expr]`          (computed = true)
}

pub enum AssignmentTarget {
    Identifier(Identifier),
    MemberExpression(Box<MemberExpression>),
    // Phase 3: Pattern (destructuring)
}

pub struct Property {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub kind: PropertyKind,        // init | get | set
    pub key: PropertyKey,
    pub value: Box<Expression>,
    pub computed: bool,
    pub shorthand: bool,
    pub method: bool,
}

pub enum PropertyKind { Init, Get, Set }
pub enum PropertyKey {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
    NumericLiteral(NumericLiteral),
    Expression(Expression),         // when `computed: true`
}
```

(Phase 2 adds `BigIntLiteral`, `RegExpLiteral`,
`TemplateLiteral`, `TaggedTemplateExpression`,
`SequenceExpression`, `UpdateExpression`, `NewExpression`,
`SpreadElement`, `ThisExpression`, `SuperExpression`.)

## Phase 1 — Declaration variants

| Variant               | Fields                                                                                                                                              |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VariableDeclaration` | `cv: Option<CvId>`, `kind: VarKind`, `declarations: Vec<VariableDeclarator>`                                                                                |
| `FunctionDeclaration` | `cv: Option<CvId>`, `id: Identifier`, `params: Vec<FunctionParam>`, `body: BlockStatement`, `generator: bool`, `is_async: bool`                             |

```rust
pub enum VarKind { Var, Let, Const }

pub struct VariableDeclarator {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub id: BindingTarget,
    pub init: Option<Expression>,
}

pub enum BindingTarget {
    Identifier(Identifier),
    // Phase 3: ArrayPattern, ObjectPattern (destructuring)
}

pub enum FunctionParam {
    Identifier(Identifier),
    // Phase 3: AssignmentPattern (default value), RestElement, ArrayPattern, ObjectPattern
}
```

(Phase 3 adds `ClassDeclaration`; Phase 4 adds `ImportDeclaration`,
`ExportNamedDeclaration`, `ExportDefaultDeclaration`,
`ExportAllDeclaration`.)

### `is_async` vs ESTree's `async`

ESTree names the boolean `async`. Rust's `async` is a reserved
keyword (raw identifiers `r#async` work but are awkward at call
sites). We name the field `is_async` and tag it
`#[serde(rename = "async")]` so the JSON wire format still
matches ESTree exactly.

This is the only field-name divergence Phase 1 introduces. It
applies anywhere a function-like node has the async flag
(`FunctionDeclaration`, Phase 1.x `FunctionExpression` — landed in
CLOC12.149, Phase 3 `ArrowFunctionExpression`, `Method`, `ClassMethod`,
etc.).

## Why split `Statement` and `Declaration` enums

ESTree puts declarations inside the `Statement` enum (so
`VariableDeclaration` IS a `Statement`). We keep them in a
separate `Declaration` enum AND wrap them as
`Statement::Declaration(Declaration)` to support both views.

Reason: passes that care specifically about declarations
(`closure-pass-rename`, `closure-pass-treeshake`,
`closure-pass-remove-unused-vars`) traverse `Vec<Declaration>`
directly. Passes that traverse statement bodies
(`closure-pass-constant-fold`, `closure-pass-dce`) pattern-match
on `Statement` and reach declarations via the
`Statement::Declaration` wrapping.

The JSON wire format collapses this: on serialize,
`Statement::Declaration(Declaration::VariableDeclaration(d))`
serializes to `{"type": "VariableDeclaration", ...}` directly
(via `#[serde(untagged)]` on `Statement::Declaration`'s wrapping
arm). The Rust enum hierarchy is for pass ergonomics; the
output JSON matches ESTree's flatter shape.

## Visitor pattern (deferred to Phase 1.5)

ESTree-walking tools typically expose a visitor pattern.
`closure-pass-pipeline`'s `Pass::run` already provides a
context-shaped entry point, and Phase 1 passes will do their
walks inline. A first-class `Visitor` / `VisitorMut` trait in
`javascript-ast` is **deferred to Phase 1.5** once we have
real pass bodies and can identify the right shape from concrete
needs rather than guessing.

## Crate layout (Phase 1)

```
code/packages/rust/javascript-ast/
├── Cargo.toml          (unchanged: serde, correlation-vector, javascript-tokens)
├── src/
│   ├── lib.rs          (re-exports + Program / SourceType / EsVersion)
│   ├── statement.rs    (Statement enum + per-variant structs)
│   ├── expression.rs   (Expression enum + per-variant structs + operators)
│   └── declaration.rs  (Declaration enum + per-variant structs)
├── README.md, CHANGELOG.md, BUILD, BUILD_windows, required_capabilities.json
```

Per-variant structs land in module files (one file per kind:
`statement.rs`, `expression.rs`, `declaration.rs`). `lib.rs`
re-exports the enums and the leaves so downstream code does
`use coding_adventures_javascript_ast::{Program, IfStatement,
BinaryOperator};` without knowing the file layout.

## Test surface (Phase 1)

Per CLOC02 the AST has no behavior to unit-test directly. The
test surface is **round-trip property tests on the JSON wire
format**:

- For each variant, construct a representative node, serialize
  to JSON, deserialize back, assert structural equality.
- Round-trip the example program from Phase 1's intro
  (`function f(x){ return x + 1; } const a = f(7);`) end-to-end.
- Validate that the JSON `"type"` tag matches the ESTree-spec
  expected value for each variant (`{"type":"IfStatement"}`,
  not `{"type":"if_statement"}` etc.).

Coverage target: every Phase 1 variant has at least one
round-trip test. Aim for 95%+ line coverage on the new modules.

## Migration impact on existing crates

Adding variants to the AST is **additive** for every existing
crate:

- `closure-typechecker`: still receives `&Program`. Real
  checking implementation lands in a follow-up PR; the API
  doesn't change.
- `closure-pass-*`: every `Pass::run` is identity today and stays
  identity through this PR. Pass *bodies* fill in over
  subsequent PRs, one pass at a time, as each pass gains the
  ability to do real work on the now-rich AST.
- `closure-emitter`: same. Identity through this PR; real
  emission lands as a follow-up.
- `closurec`: no change. The CLI parses Closure-Compiler flags;
  the binary's body is still v1 identity.

No pass dependency edges shift. No public function signatures
change. The point of the Phase 1 PR is to grow the AST and
ship that as a self-contained, mergeable unit; pass
implementations are independent follow-ups.

## What this PR locks down

1. The full Phase 1 node taxonomy listed above (25 variants).
2. Full ESTree-compat wire format: JSON `"type"` tags, camelCase
   field names, structural shape per ESTree-spec.
3. The `cv: Option<CvId>` convention as the **only** identity
   field on every node. Optionality is first-class — both
   tracing-enabled and tracing-disabled modes are equally
   supported, switched per-program by whether the user
   constructs nodes with `Some(id)` or `None`. The pipeline has
   one behavior across both modes; the `Option` discriminator
   is the only switch. `#[serde(skip_serializing_if = "Option::is_none", default)]`
   omits the field from JSON entirely when `None`, so untraced
   ASTs match ESTree's wire format byte-for-byte.
4. The `is_async` rename to JSON `"async"` (only field-name
   divergence Phase 1 introduces).
5. The split `Statement` / `Declaration` enum design with the
   `Statement::Declaration` wrapping arm (untagged on serialize).
6. The deferred decisions (visitor pattern, BigInt/RegExp
   literals, async/generators) explicitly called out as Phase
   1.5+/Phase 2+/Phase 5 so a reviewer doesn't think they were
   forgotten.

## What's coming

- Phase 1 implementation PR (immediately after this spec
  merges): all 25 variants, round-trip tests, no pass changes.
- Phase 1.5: `Visitor` / `VisitorMut` traits with a default
  walk implementation; revise after seeing the first real pass
  body.
- Phase 1.x incremental variant additions (landed as the passes
  and emitter grew a need for them, each behind its own CLOC12.NN
  slice + gap entry): `BigIntLiteral` (CLOC12.15), `UndefinedLiteral`
  (CLOC12.16), and **`FunctionExpression` (CLOC12.149)** — the
  expression sibling of `FunctionDeclaration` (`var f = function () {}`,
  IIFEs, function-valued properties). `id` is `Option<Identifier>`
  (anonymous vs. named-but-body-local); `params` / `body` /
  `generator` / `is_async` reuse the declaration's types. Landed
  bottom-up: AST node first (this slice), then the parser→typed-AST
  bridge, emitter printing, and pass traversal in follow-on slices.
- Phase 2: leaf coverage gaps + control-flow variants
  (`SwitchStatement`, `TryStatement`, etc.).
- Phase 3: patterns, arrow functions, classes.
- Phase 4: modules.
- Phase 5: async / generators / nullish / optional chaining.
- Per-pass implementation PRs: once the AST has the variants a
  given pass needs, that pass's `run` body fills in. Order is
  flexible — passes don't depend on each other for variant
  coverage, only for ordering at pipeline runtime.

# CLOC02 — `javascript-ast`: Backend-Agnostic, CV-Bearing JavaScript AST

## Why this crate is the most important one to get right

Every other crate in the CLOC series consumes or produces `javascript-ast`.
Closure passes consume it. The typechecker consumes it. The emitter consumes it.
The JSDoc types extractor walks it to find comment anchors. The future V8 clone
will lower it to bytecode.

Because so many consumers depend on it, the AST has to be:

1. **Small.** Node structs hold the syntactic shape and nothing else.
2. **Backend-agnostic.** No type info, no optimization metadata, no IR opcodes,
   no spans — those live in adjacent stores keyed by CV.
3. **Immutable on the public surface.** Passes return new trees, not mutated
   trees. CV reasoning depends on this.
4. **Version-aware at the root only.** Per-node version tags would bloat every
   node; the `Program` root records `EsVersion` and that's enough.
5. **Stable.** Once a node variant lands, it does not change shape without a CLOC
   amendment spec. Downstream crates can rely on the schema.

This spec defines the crate's public surface. Implementation details (how nodes
are stored in memory, how interning works, whether `Box` vs `Arc`) are
implementation notes at the end.

## Crate location

```text
code/packages/rust/javascript-ast/
  BUILD
  BUILD_windows
  CHANGELOG.md
  Cargo.toml
  README.md
  required_capabilities.json
  src/
    lib.rs
    program.rs
    statement.rs
    expression.rs
    declaration.rs
    pattern.rs
    class.rs
    module.rs
    literal.rs
    version.rs
```

Crate name: `coding-adventures-javascript-ast` (matching the existing repo
naming convention for `coding-adventures-javascript-lexer` etc.).

## Dependencies (the entire whitelist)

The AST crate may depend only on:

- `coding-adventures-javascript-tokens` — for `TokenKind` constants used in
  binary operator enums and similar.
- `coding-adventures-correlation-vector` — for `CvId`.
- `serde` + `serde_json` — only for `Serialize` / `Deserialize` derives on every
  node. (Sidecar format and IDE tooling both need round-trippable JSON.)

It must **not** depend on:
- Any `closure-*` crate.
- The `type-sidecar` crate.
- Any IR or bytecode crate.
- The `lexer` or `parser` crates (those depend on it, not the other way).
- Any string-interning crate (we use plain `String` for v1; if interning becomes
  necessary later, it lives behind an opaque `Symbol` type without leaking the
  intern table).

Dependency cycles in the package graph will get caught by `cargo build
--workspace` per the repo's standard checks; this whitelist is a design
constraint above and beyond what the compiler can enforce.

## The `CvId` field

Every node — every single one, no exceptions — has a `cv: CvId` field. `CvId`
is a copy-cheap newtype defined in `correlation-vector`. The convention:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    pub cv: CvId,
    pub name: String,
}
```

The `cv` field is conventionally first so that any pass walking nodes via
reflection or pattern matching can pluck the CV off uniformly. Nodes wrapping
other nodes (e.g., `BinaryExpression`) get their own `CvId` plus their children
keep theirs.

The CV log itself lives outside the AST. Nodes carry only the ID. Spans, parent
chains, and contributions are queried from the log by ID. This is what keeps
nodes small.

## The root: `Program`

```rust
pub struct Program {
    pub cv: CvId,
    pub version: EsVersion,
    pub source_type: SourceType,   // Script or Module
    pub body: Vec<ProgramElement>,
    pub hashbang: Option<Hashbang>, // ES2023+
}

pub enum SourceType {
    Script,
    Module,
}

pub enum ProgramElement {
    Statement(Statement),
    ImportDeclaration(ImportDeclaration),     // module-only
    ExportDeclaration(ExportDeclaration),     // module-only
    FunctionDeclaration(FunctionDeclaration),
    ClassDeclaration(ClassDeclaration),
}

pub struct Hashbang {
    pub cv: CvId,
    pub content: String,  // text after #! up to (but excluding) newline
}
```

`SourceType` is filled by the parser based on caller hint or shebang/file
extension. The Closure Compiler scheduler needs this for cross-module passes;
the V8 clone needs it because module and script have different top-level scoping
rules.

## Version

```rust
pub enum EsVersion {
    Es1,
    Es3,
    Es5,
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    Es2023,
    Es2024,
    Es2025,
}
```

`EsVersion` is on `Program`, **not** on individual nodes. The parser refuses to
emit nodes whose variants are not legal at the requested version (e.g.,
`Decorator` cannot appear in an `Es5` program). Downstream consumers should
generally not branch on `version`; they should assume "every variant that
appears is legal." The version tag exists for:

- Emitters that may need to target a specific version (downleveling is out of
  scope for the MVP, but the field lets future passes do it).
- Telemetry and error messages.
- Round-tripping (a tree that round-trips through serialization knows where it
  came from).

## Statements

```rust
pub enum Statement {
    Block(BlockStatement),
    Empty(EmptyStatement),
    Expression(ExpressionStatement),
    If(IfStatement),
    For(ForStatement),
    ForIn(ForInStatement),
    ForOf(ForOfStatement),              // ES2015+
    ForAwaitOf(ForAwaitOfStatement),    // ES2018+
    While(WhileStatement),
    DoWhile(DoWhileStatement),
    Switch(SwitchStatement),
    Try(TryStatement),                  // ES3+
    Throw(ThrowStatement),              // ES3+
    Return(ReturnStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Labeled(LabeledStatement),
    With(WithStatement),
    Debugger(DebuggerStatement),        // ES5+
    Variable(VariableDeclaration),      // var
    Lexical(LexicalDeclaration),        // let / const, ES2015+
    Using(UsingDeclaration),            // ES2025+
    AwaitUsing(AwaitUsingDeclaration),  // ES2025+
}
```

Each variant struct follows the same shape: `cv: CvId` plus fields for the
construct's children. Example:

```rust
pub struct IfStatement {
    pub cv: CvId,
    pub test: Box<Expression>,
    pub consequent: Box<Statement>,
    pub alternate: Option<Box<Statement>>,
}
```

`Box` because statements are recursive; we don't want unbounded enum size.

## Expressions

The expression tree mirrors the precedence cascade documented in
`versioned-ecmascript-typescript-grammars.md` Section 7. A flat enum keeps the
shape simple even though precedence is encoded in the parser's choice of which
variant to produce.

```rust
pub enum Expression {
    // Primary
    This(ThisExpression),
    Super(Super),                        // ES2015+ (ESTree node type is bare `Super`; CLOC12.166)
    Identifier(Identifier),
    PrivateName(PrivateName),            // ES2022+ (only valid inside class)
    Literal(Literal),
    Template(TemplateLiteral),           // ES2015+
    TaggedTemplate(TaggedTemplate),      // ES2015+
    ArrayLiteral(ArrayLiteral),
    ObjectLiteral(ObjectLiteral),
    Function(FunctionExpression),
    Arrow(ArrowFunction),                // ES2015+
    Class(ClassExpression),              // ES2015+
    Regex(RegexLiteral),                 // ES3+
    NewTarget(NewTarget),                // ES2015+
    ImportMeta(ImportMeta),              // ES2020+
    Parenthesized(ParenthesizedExpression),

    // Operators
    Unary(UnaryExpression),
    Update(UpdateExpression),         // ++/-- prefix or postfix
    Binary(BinaryExpression),
    Logical(LogicalExpression),       // &&, ||, ??
    Assignment(AssignmentExpression),
    Conditional(ConditionalExpression),
    Sequence(SequenceExpression),     // a, b, c

    // Access
    Member(MemberExpression),
    OptionalChain(OptionalChainExpression),  // ES2020+

    // Calls
    Call(CallExpression),
    New(NewExpression),
    Import(ImportCallExpression),   // dynamic import, ES2020+

    // Iteration / control
    Yield(YieldExpression),         // ES2015+
    Await(AwaitExpression),         // ES2017+
    Spread(SpreadElement),
}
```

Selected variant structs (others follow the same pattern):

```rust
pub struct BinaryExpression {
    pub cv: CvId,
    pub op: BinaryOperator,    // a typed enum: Plus, Minus, Equals, ...
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

pub struct LogicalExpression {
    pub cv: CvId,
    pub op: LogicalOperator,   // And, Or, NullishCoalescing
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

pub struct CallExpression {
    pub cv: CvId,
    pub callee: Box<Expression>,
    pub arguments: Vec<Argument>,  // Argument may be a SpreadElement
    pub optional: bool,            // true for `f?.(...)` chains, ES2020+
}

pub struct MemberExpression {
    pub cv: CvId,
    pub object: Box<Expression>,
    pub property: MemberProperty,
    pub optional: bool,            // true for `a?.b`, ES2020+
}

pub enum MemberProperty {
    Identifier(Identifier),     // a.b
    Private(PrivateName),       // a.#b, ES2022+
    Computed(Box<Expression>),  // a[expr]
}
```

The `BinaryOperator` and `LogicalOperator` enums are exhaustive over every
operator from ES1 through ES2025. They live in `expression.rs`.

## Declarations and bindings

```rust
pub struct VariableDeclaration {
    pub cv: CvId,
    pub declarations: Vec<VariableDeclarator>,
}

pub struct VariableDeclarator {
    pub cv: CvId,
    pub id: BindingPattern,
    pub init: Option<Expression>,
}

pub struct LexicalDeclaration {
    pub cv: CvId,
    pub kind: LexicalKind,          // Let or Const
    pub declarations: Vec<VariableDeclarator>,
}

pub enum BindingPattern {
    Identifier(Identifier),
    Object(ObjectPattern),     // ES2015+
    Array(ArrayPattern),       // ES2015+
}
```

`UsingDeclaration` and `AwaitUsingDeclaration` mirror `LexicalDeclaration` but
only accept `BindingPattern::Identifier` (the spec disallows destructuring there).

## Classes and modules

These are large enough to live in their own files (`class.rs`, `module.rs`).
The variants are exhaustive over the union of ES2015 through ES2025 class and
module syntax: heritage, static methods, accessors, fields, private members,
static blocks, decorators, namespace imports, default exports, re-exports, and
import attributes.

The exact shape is mechanical given the grammar files under
`code/grammars/ecmascript/`. We omit the full listing here; the rule is "one
variant per grammar production, every variant carries `cv`."

## Literals

```rust
pub enum Literal {
    Null(NullLiteral),
    Boolean(BooleanLiteral),
    Number(NumberLiteral),
    BigInt(BigIntLiteral),         // ES2020+
    String(StringLiteral),
}

pub struct NumberLiteral {
    pub cv: CvId,
    pub value: f64,
    pub raw: String,               // preserves "0xFF" vs "255", "1_000" vs "1000"
}
```

`raw` is preserved so the emitter can round-trip literals byte-for-byte when no
pass has touched them. Constant-folding passes that *do* touch literals will
discard `raw` and re-render from `value`.

## Side stores keyed by CV

Things that look like they should be on nodes, but live elsewhere:

| Concept | Where it lives | Key |
| --- | --- | --- |
| Source span (file, line, col, byte offset) | `correlation-vector` `Origin` entries | `CvId` |
| Parent / lineage | `correlation-vector` log | `CvId` |
| Pass contributions ("renamed", "inlined", ...) | `correlation-vector` log | `CvId` |
| Resolved type | `type-sidecar::Sidecar` | `CvId` |
| Reaching definitions, dataflow facts | Pass-internal state, keyed by `CvId` | `CvId` |
| Comment attachments (JSDoc) | `type-sidecar` after JSDoc extraction | `CvId` |
| Symbol table entries | `closure-typechecker` state | `CvId` |

Every consumer that needs to attach data to nodes does so by keying on `CvId`
in its own store. The AST stays clean.

## Immutability and builder helpers

The public types are all `pub struct` with `pub` fields, but the convention is
that **consumers do not mutate them in place**. Passes return new trees by
calling builder functions. Each variant gets a builder:

```rust
impl BinaryExpression {
    pub fn new(cv: CvId, op: BinaryOperator, left: Expression, right: Expression) -> Self {
        Self { cv, op, left: Box::new(left), right: Box::new(right) }
    }
}
```

Builders compute nothing; they just box children. They exist for clarity and to
make pass code read top-down.

For larger trees, a `walk` module exposes a visitor pattern (visit + visit_mut)
that recurses through every node. Passes use this to find nodes of interest.
The walker is the canonical way to traverse — passes never hand-roll recursion.

## Construction by the parser

The parser builds the AST as it reduces grammar productions. For each
production, it:

1. Collects the token spans involved.
2. Calls `cv.create(Origin{...})` or `cv.merge(parent_ids)` to get a fresh
   `CvId` for the new node.
3. Constructs the node struct with that `cv` plus child nodes (whose `CvId`s
   are already set).

The parser depends on `javascript-ast` and `correlation-vector`. The AST does
not depend on the parser.

## Serialization

Every node derives `Serialize` and `Deserialize`. This is required for:

- IDE tooling (LSP) to ship trees to other processes.
- Snapshot tests in passes (golden ASTs as JSON files).
- Sidecar test fixtures.
- Debugging via `closurec --dump-ast`.

Round-tripping is byte-identical for any tree built from valid input.

## Testing strategy

The crate ships with three test layers:

1. **Construction tests** — for every variant, build it programmatically and
   assert field values. Pure unit, no parser involved.
2. **Serde round-trip tests** — for every variant, serialize and deserialize,
   assert equality.
3. **Visitor coverage tests** — walk a small synthetic tree and assert that
   every node is visited exactly once.

Per the repo's >80% coverage rule (`feedback_repo_standards`), AST coverage
should land at 95%+ since it's a library crate.

## What this spec does **not** cover

- The exact set of grammar-production-to-node mappings — that's parser internal.
- The visitor trait's full method list — that's an implementation detail of
  `walk.rs`, locked once the variants stabilize.
- The TypeScript AST. TS will have its own crate (`typescript-ast`) with its own
  spec, sharing only the type-sidecar format. JS and TS ASTs are separate types.
- The `javascript-tokens` crate. That's a one-paragraph spec eventually (token
  kind enum + version tag + span type); it lands as part of the Stage 1 fix-up.
- Implementation choices around `Box` vs `Arc` vs arena allocation. The default
  is `Box` for v1. If profiling shows allocation pressure, we'll revisit, but
  the public API does not depend on the choice.

## Open questions

These are flagged for follow-up specs, not blockers for landing the AST crate:

1. **Trivia.** Comments are not currently AST nodes; they live in the source via
   spans. JSDoc extraction reads them directly from source by CV. Is that
   enough? Or do we need an explicit `Comment` node attached to nearby
   statements? (Closure Compiler's JSDoc consumer needs the comments anchored
   to the *next* declaration. CV resolution can do that.)
2. **Regex internals.** Today `RegexLiteral` stores `pattern: String` and
   `flags: String`. A future spec may add a parsed regex AST. Not blocking.
3. **JSX.** Out of scope for the MVP. Will get its own variants under a feature
   flag if added later.

# Changelog

All notable changes to the `coding-adventures-javascript-ast` crate will be documented in this file.

## [0.38.0] - 2026-07-11

### Added — CLOC12.187 PR1: `with (obj) { … }` statement

New `WithStatement { cv, object, body }` struct and a
`TaggedStatement::WithStatement(WithStatement)` variant (ESTree `WithStatement`),
plus the `Statement::with_statement` convenience constructor. Models the legacy
`with` scope-injection statement — the last unmodelled self-contained statement
node. The node is **not** yet produced by the parser bridge (a follow-up PR
wires it, together with the scope-analyzer renaming-soundness bailout that `with`
demands — renaming a binding inside a `with` body is unsound); this PR only adds
the type so the whole pass pipeline can traverse it exhaustively. New
`with_statement_roundtrips` test.

## [0.37.0] - 2026-07-11

### Added — CLOC12.183: ES2021 logical assignment operators

Added three variants to `AssignmentOperator`, retiring the "Phase 5" TODO:
`LogicalAndEq` (`&&=`), `LogicalOrEq` (`||=`), and `NullishCoalescingEq`
(`??=`), each with its ESTree serde `rename`. These short-circuiting operators
were the last unmodelled compound-assignment forms; the parser bridge and
emitter now recognise them (see those crates' 0.47.0 / 0.42.0 entries).

## [0.36.0] - 2026-07-11

### Added — CLOC12.177 PR1: private class-member names (`#x` / `#m()`)

New `PrivateName { cv, name }` struct and a `PropertyKey::PrivateName(PrivateName)`
variant — the key of a private class field (`#x = 1`) or method (`#m(){}`),
ESTree's `PrivateIdentifier`. The `name` holds the bare name **without** the
leading `#` (mirroring `Identifier`); the emitter prepends it.

Because `PropertyKey` is `#[serde(untagged)]`, a bare `{ cv, name }` `PrivateName`
would be structurally identical to an `Identifier` and the deserializer would
silently pick whichever variant comes first (a `#x` key round-tripping back as a
plain `x` — data loss). The payload therefore serializes under the distinct key
`private_name` (`#[serde(rename)]`), giving the variant a unique required field;
the Rust field stays `name` for API symmetry. Two round-trip tests pin this.

MINOR. Member *access* (`this.#x`) is a distinct later node.

## [0.35.0] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` (static initialization blocks)

`ClassMember` grows a third variant beside `Method` and `Field`:

- `ClassMember::StaticBlock(BlockStatement)` models a `static { … }` block — a
  statement list that runs once at class-definition time. It has no name, key,
  param list, or initializer, so the variant wraps the existing `BlockStatement`
  (its body is exactly a `Vec<Statement>`) rather than re-modelling it.

3 roundtrip tests (empty block / block with a statement / static block interleaved
with a field).

## [0.34.0] - 2026-07-11

### Added — CLOC12.175 PR1: `ClassMember::Field` (class fields / `PropertyDefinition`)

The class-body member enum `ClassMember` grows a second variant beside `Method`:

- `ClassMember::Field(PropertyDefinition)` models a class field
  `[static] key [= initializer];`.
- `PropertyDefinition { cv, key: PropertyKey, value: Option<Expression>, computed, is_static }`
  — a bare field (`x;`) has `value: None`; an initialized field (`x = 1;`) carries
  the initializer. The `key` reuses the existing `PropertyKey` (so a computed
  `[expr]` field is `PropertyKey::Expression`), and `is_static` serializes as the
  JSON key `static`.

Reuses `PropertyKey` from the method form — no new key modelling. Four roundtrip
tests cover initialized / bare / static / computed-key fields.

## [0.33.0] - 2026-07-10

### Added — CLOC12.174 PR1: `ClassDeclaration` node (the class *statement*)

The declaration half of the class arc whose *expression* form shipped in 0.32.0
(CLOC12.173). New `Declaration` variant `ClassDeclaration(ClassDeclaration)`:

- `ClassDeclaration { cv, id: Identifier, super_class: Option<Box<Expression>>, body: Vec<ClassMember> }`
  — `class C [extends S] { members }` in statement position.
- **Reuses** `ClassMember` / `MethodDefinition` / `MethodKind` from the expression
  form — no new member modelling.
- The one structural difference from `ClassExpression`: `id` is `Identifier`
  (required), not `Option<Identifier>` — a class declaration must bind a name
  (`class {}` in statement position is a syntax error), exactly as
  `FunctionDeclaration.id` is required where `FunctionExpression.id` is optional.

Round-trip tests cover the `ClassDeclaration` type tag and the ESTree
`superClass` camelCase field (present with heritage, omitted without).

## [0.32.0] - 2026-07-08

### Added — CLOC12.173 PR1: `ClassExpression` node + class member sub-AST

The first class modelling in the typed AST (previously classes existed nowhere;
a class literal dropped its whole file to WHITESPACE_ONLY). New `Expression`
variant `ClassExpression(ClassExpression)` plus its member sub-AST:

- `ClassExpression { cv, id: Option<Identifier>, super_class: Option<Box<Expression>>, body: Vec<ClassMember> }`
  — `class [id] [extends S] { members }` in value position.
- `ClassMember::Method(MethodDefinition)` — the only member kind today; fields
  (`PropertyDefinition`) and `static { … }` blocks are additive follow-ups.
- `MethodDefinition { cv, key: PropertyKey, kind: MethodKind, value: FunctionExpression, computed, is_static }`
  — reuses `PropertyKey` (a method key has the same four shapes as a property
  key). A dedicated type rather than reusing object-literal `Property`, which
  cannot express `static` or a `constructor` kind and whose value is not always
  a function; matches Closure's Rhino `MEMBER_FUNCTION_DEF`/`GETTER_DEF`/
  `SETTER_DEF`/`COMPUTED_PROP` kinds.
- `MethodKind { Constructor, Method, Get, Set }`.

Re-exported from the crate root. Scoped to the class *expression* form; the
class *declaration* (`ProgramItem`) form is a separate future arc sharing this
sub-AST. Emitter + constant-fold support land in the same PR1 commit; the bridge
is PR2, the CodePrinter conformance port PR3 (see `code/specs/CLOC12-gaps.md`
§CLOC12.173).

## [0.31.0] - 2026-07-08

### Added — CLOC12.172 PR1: `RegExpLiteral` leaf node (`/pattern/flags`)

New `Expression::RegExpLiteral(RegExpLiteral { cv, pattern, flags })` — a regex
literal modelled as its own leaf, alongside the other literal leaves. Previously
the bridge fell back to an `Identifier` whose name was the regex source, which
round-tripped only by accident (the text is not a valid identifier, so the
rename passes skipped it) and left a latent hazard. The two halves are stored
split (`pattern` = the body between the slashes, `flags` = the trailing flag
set) so passes can reason about the flags without re-parsing; the printer
reconstructs `/{pattern}/{flags}`. Mirrors ESTree's `RegExpLiteral` minus the
live-`RegExp` `value` a source-to-source minifier never needs.

Purely additive — no existing node changed. This is the atomic node PR of the
CLOC12.172 arc; the bridge that *builds* it (replacing the identifier fallback)
is PR2, and the CodePrinter conformance port is PR3.

## [0.30.0] - 2026-07-08

### Added — CLOC12.171 PR1: optional chaining `a?.b` / `a?.[k]` / `a?.()` (ES2020)

Three new `Expression` variants model **optional chaining**:

- `OptionalMemberExpression { cv, object, property, computed }` — `a?.b` /
  `a?.[k]`. Structurally identical to `MemberExpression`; the distinct type is
  what records that the link was written with `?.`.
- `OptionalCallExpression { cv, callee, arguments }` — `a?.()`. Structurally a
  `CallExpression` written with `?.(`.
- `ChainExpression { cv, expression }` — the transparent chain-boundary wrapper
  (ESTree `ChainExpression`) that marks where the `undefined` short-circuit
  resolves. It carries no syntax of its own.

**Modelling note (diverges from the earlier flag-based plan).** The scouted
plan was the ESTree-7 shape: an `optional: bool` flag on `MemberExpression` /
`CallExpression` plus a `ChainExpression` wrapper. We instead give the optional
links their **own node types**, for two reasons: (1) it matches the conformance
target — Google's Closure Compiler represents optional access with dedicated
Rhino node kinds (`OPTCHAIN_GETPROP` / `OPTCHAIN_GETELEM` / `OPTCHAIN_CALL`),
not a flag on the ordinary access nodes; and (2) it is purely additive —
`MemberExpression` / `CallExpression` have ~150 construction sites across the
workspace, and a new required field would touch every one, whereas a new
variant only adds `match` arms where a pass actually cares. `ChainExpression`
is retained as the short-circuit boundary marker.

Purely additive — no existing node changed. This is the atomic node PR of the
CLOC12.171 arc; the grammar→typed-AST bridge that *builds* these nodes is PR2,
and the CodePrinter conformance port is PR3.

## [0.29.0] - 2026-07-07

### Added — CLOC12.170: object spread `{...o}` via `ObjectMember` (gap-SpreadProperty)

Object literals now model **object spread** `{...o}` (ES2018). This is the
first *structural* change to an existing node: `ObjectExpression.properties`
changes from `Vec<Property>` to `Vec<ObjectMember>`, where

```rust
pub enum ObjectMember { Property(Property), Spread(SpreadElement) }
```

reusing the existing `SpreadElement { cv, argument }` for the spread arm (the
same node the call/array spread uses — ESTree names both `SpreadElement`). The
enum keeps `Property` and `Spread` members in ONE ordered vector because the
order is observable (`{a: 1, ...o, b: 2}` — a later member overrides an earlier
key, and a spread may sit before, between, or after plain properties); a side
channel would lose the interleaving and miscompile override semantics.
Serialised `#[serde(untagged)]`: the `Property` arm carries its own
`"type": "Property"` tag and the `Spread` arm carries `SpreadElement`'s
`argument`, so a member round-trips unambiguously (new
`object_member_spread_round_trips` test). This is the **node-only** PR1 — the
`javascript-parser` bridge still declines `{...o}` (gap-SpreadProperty), and the
emitter + all nine downstream passes gain their `ObjectMember` match arms in the
same atomic change so the workspace never has a broken `match`.

## [0.28.0] - 2026-07-07

### Added — CLOC12.169: `Expression::ImportExpression` (dynamic `import(x)`)

Added `Expression::ImportExpression` variant + `ImportExpression { cv, source }` struct — a **dynamic `import(specifier)`**, the runtime module-loading form (distinct from a static `import` declaration and from the `ImportMeta` meta-property). A single-operand node (`source: Box<Expression>`, the module specifier), the same shape as `AwaitExpression`; `import()` with no argument is a syntax error, so `source` is non-optional. It is a call-like primary (`import` keyword + parenthesised argument), tagging at `PREC_PRIMARY`. Re-exported from the crate root; 2 roundtrip/serde tests. Atomic node PR (PR1): the node + `closure-emitter` emit + all nine downstream pass arms (which recurse into `source`) land together so the workspace never breaks. (CLOC12.169)


## [0.27.0] - 2026-07-07

### Added — CLOC12.168: `Expression::ImportMeta` (`import.meta`)

Added `Expression::ImportMeta` variant + `ImportMeta { cv }` struct — the `import.meta` module meta-property, the `MetaProperty` **leaf** sibling of `NewTarget` (same shape, no operand). Spelled with three tokens (`import` `.` `meta`) in source but modelled as an atomic node rather than a member access — `import` is a reserved word with no accessible identifier — so the renaming passes never touch it. Re-exported from the crate root. Atomic node PR (PR1): the node + `closure-emitter` emit + all nine downstream pass match-arms land together so the workspace never breaks. (CLOC12.168)


## [0.26.0] - 2026-07-04

### Added — CLOC12.167: `Expression::NewTarget` (`new.target`)

Added `Expression::NewTarget` variant + `NewTarget { cv }` struct — the `new.target` meta-property, a reserved-word **leaf** primary (same shape as `Super` / `ThisExpression`, no operand). Spelled with two tokens (`new` `.` `target`) in source but modelled as an atomic node rather than a member access, so the renaming passes never touch it. Re-exported from the crate root. Atomic node PR (PR1): the node + `closure-emitter` emit + all nine downstream pass match-arms land together so the workspace never breaks. (CLOC12.167)


## [0.25.0] - 2026-07-04

### Added — CLOC12.166: `Expression::Super` (`super`)

Added `Expression::Super` variant + `Super { cv }` struct — the `super` keyword, a reserved-word **leaf** primary (same shape as `ThisExpression`, no operand). Like `this`, modelled as its own node rather than `Identifier { name: "super" }` so renaming passes never touch it; `super` is syntactically restricted to member-object / call-callee position inside a method or derived constructor, but that is the parser's concern — the AST treats it as a plain leaf primary. Re-exported from the crate root. Atomic node PR (PR1): the node + `closure-emitter` emit + all nine downstream pass match-arms land together so the workspace never breaks. (CLOC12.166)


## [0.24.0] - 2026-07-04

### Added — CLOC12.165: `Expression::ThisExpression` (`this`)

Added `Expression::ThisExpression` variant + `ThisExpression { cv }` struct — the `this` keyword, a reserved-word **leaf** primary (same shape as `NullLiteral` / `UndefinedLiteral`, no operand). Modelled as its own node rather than `Identifier { name: "this" }` so renaming passes never touch it. Re-exported from the crate root. Atomic node PR (PR1): the node + `closure-emitter` emit + all nine downstream pass match-arms land together so the workspace never breaks. (CLOC12.165)


## [0.23.0] - 2026-07-04

### Added — CLOC12.164: `Expression::AwaitExpression` (`await x`)

Added `Expression::AwaitExpression` variant + `AwaitExpression { cv, argument:
Box<Expression> }` struct — the `await x` async-suspend operator (always has an
operand; no optional/delegate axis, unlike YieldExpression). Re-exported from
the crate root. (CLOC12.164)


## [0.22.0] - 2026-07-03

### Added — CLOC12.163: `Expression::YieldExpression` (`yield` / `yield x` / `yield* xs`)

New `YieldExpression { cv, delegate: bool, argument: Option<Box<Expression>> }`
variant of `Expression` modelling a generator `yield` — a bare `yield`
(`argument: None`, yields `undefined`), a value yield `yield x`
(`argument: Some(x)`), and a delegating `yield* xs` (`delegate: true`). The
argument is optional because a bare `yield` has no operand, and `delegate`
distinguishes `yield` from `yield*`. Re-exported from the crate root. Atomic
node PR (PR1): the node + `closure-emitter` emit + all nine downstream pass
match-arms land in one commit so the workspace never breaks.


## [0.21.0] - 2026-07-03

### Added — CLOC12.162: `Expression::SpreadElement` (`...arg`)

New `SpreadElement { cv, argument: Box<Expression> }` variant of `Expression`
modelling a spread `...arg` — the `...` prefix that unpacks an iterable into a
call/`new` argument list (`f(...a)`) or an array-literal element (`[...a]`). It
is not a free-standing expression (a bare `...x` is a syntax error); it is
modelled as an `Expression` variant so it slots into the existing
`Vec<Expression>` argument/element lists without a parallel enum, and every
existing AST walker that recurses those `Vec`s reaches `argument` for free.
Re-exported from the crate root. Atomic node PR (PR1): the node +
`closure-emitter` emit + all nine downstream pass match-arms land in one commit
so the workspace never breaks. The bridge conversion (grammar-AST → typed-AST)
and the conformance port follow in PR2/PR3.


## [0.20.0] - 2026-07-02

### Added — CLOC12.161: `Expression::TaggedTemplateExpression` (`` tag`...` ``)

New `TaggedTemplateExpression { cv, tag: Box<Expression>, quasi: TemplateLiteral }`
variant of `Expression` for a tagged template `` tag`abc${x}` `` — the `tag`
callee is applied to the template literal that directly follows it (no call
parentheses). The `quasi` field reuses the existing `TemplateLiteral` node
(CLOC12.154), so no new template shape is introduced. Re-exported from the
crate root. Atomic node PR (PR1) — the node + `closure-emitter` emit +
match-arms in all nine downstream passes land in one commit so the workspace
never breaks. The bridge conversion (PR2) and the CodePrinter conformance
port (PR3) follow.


## [0.19.0] - 2026-07-02

### Added — CLOC12.160: `Expression::SequenceExpression` (the comma operator)

New `SequenceExpression { cv, expressions: Vec<Expression> }` variant of
`Expression` for the comma operator `a, b, c` — evaluate each operand left to
right, yield the last. It is the **loosest** expression (below assignment),
so a sequence sub-operand almost always needs parentheses. Re-exported from
the crate root. Atomic node PR (PR1); the bridge conversion (PR2) and the
CodePrinter conformance port (PR3) follow.


## [0.18.0] - 2026-07-02

### Added — CLOC12.159: `Expression::NewExpression` (`new X(args)`)

New `NewExpression { cv, callee, arguments }` variant of `Expression` for the
`new` operator. Structurally a `CallExpression` (callee + arguments) but a
distinct node because its evaluation semantics (object construction) and its
grammar precedence differ — a pass must never rewrite one into the other.
Re-exported from the crate root. This is the atomic node PR (PR1); the bridge
conversion (PR2) and the CodePrinter conformance port (PR3) follow.


## [0.17.0] - 2026-07-02

### Added — CLOC12.158: `UpdateExpression` (`++` / `--`)

New `Expression::UpdateExpression` variant (`UpdateExpression { operator,
prefix, argument }`) plus the `UpdateOperator` enum (`Increment` `++` /
`Decrement` `--`), closing the long-standing Phase 2 gap noted in the
`UnaryExpression` docs. Kept **distinct** from `UnaryExpression` because
`++`/`--` are a read-modify-write: they carry a side effect (so DCE/purity
passes must not drop them) and require a *writable reference* operand (an
identifier or member), unlike the pure prefix unary operators. `prefix`
distinguishes `++x` (yield the new value) from `x++` (yield the old value).
This is the atomic node addition — the emitter and every exhaustive
`Expression` match across the pass crates are updated in the same change so
the workspace stays green; the parser bridge enable (grammar
`postfix_expression` / prefix `++`/`--` → this node) is the follow-up PR2.

## [0.16.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` (backtick template strings)

Adds `TemplateLiteral { cv, quasis: Vec<TemplateElement>, expressions: Vec<Expression> }`
and `TemplateElement { cv, raw: String, cooked: Option<String>, tail: bool }`, plus
the `Expression::TemplateLiteral` variant — covering `` `abc` ``, `` `a${x}b` ``,
`` `${x}${y}` ``. A template interleaves fixed string parts (`quasis`) with embedded
`${…}` expressions and satisfies the ESTree invariant
`quasis.len() == expressions.len() + 1` (a template both begins and ends with a
string part). `TemplateElement` keeps `raw` (verbatim source, escapes intact) and
`cooked` (escape-processed value, `None` for an illegal escape only valid in a
tagged template — omitted from the wire format when `None`); `tail` marks the final
quasi.

This is the **AST-node slice** of a bottom-up rollout — the emitter and every pass
traversal land in the same atomic PR (adding an `Expression` variant makes every
exhaustive `match` across the workspace non-exhaustive, and CI builds the
workspace). The parser→typed-AST bridge enable + an upstream conformance port
follow in later slices. *Tagged* templates (`` tag`…` ``) are a separate node
(Phase 3).

## [0.15.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` (the `=>` form)

Adds `ArrowFunctionExpression { cv, params, body: ArrowBody, is_async }` and the
`Expression::ArrowFunctionExpression` variant, covering `x => x + 1`,
`(a, b) => a + b`, `() => {}`, and `async x => f(x)`. Three structural
simplifications from `FunctionExpression` mirror the real syntax of arrows:

- **No `id`** — arrows are always anonymous, so there is no body-local name to
  carry (and nothing for renaming passes to protect beyond the params).
- **No `generator`** — `x =>*` is not valid syntax.
- **A dual-shape `body`** — modelled by the new `ArrowBody` enum: either a
  brace-delimited `Block(BlockStatement)` (`x => { return x; }`) or a concise
  `Expression(Box<Expression>)` (`x => x`). `ArrowBody` is `#[serde(untagged)]`
  with the `Expression` arm first: every `Expression` serializes with a `"type"`
  discriminant while a `BlockStatement` has none, so concise/block bodies
  round-trip without collapsing.

`params` and `is_async` reuse `FunctionParam`; `is_async` serializes as JSON
`"async"` per ESTree; `cv` is omitted when `None`. This is the **AST-node slice**
of a bottom-up rollout — emitter printing lands in the same atomic PR (the
`Expression` enum grows a variant, so every exhaustive `match` across the
workspace must compile together), and the parser→typed-AST bridge enable +
upstream conformance port follow in later slices. Object-literal concise bodies
(`() => ({ a: 1 })`), destructuring params, and default params are deferred with
the wider Phase 3 pattern work.

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

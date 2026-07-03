# coding-adventures-javascript-ast

Backend-agnostic JavaScript AST. The contract between the JS frontend and every
consumer downstream: the Closure-Compiler-clone (typechecker, passes, emitter),
the future V8-in-Rust clone (bytecode lowering), JSDoc and TypeScript
type-extractors, IDE tooling.

Per [CLOC02](../../../specs/CLOC02-javascript-ast.md), the AST holds **only the
syntactic shape**. Spans, type information, optimization metadata, and parent
chains all live in adjacent stores keyed by `CvId` — never on AST nodes
themselves. That's what makes the same AST safe to share across multiple
backends.

## Dependency whitelist

- `coding-adventures-correlation-vector` — for the `CvId` type.
- `coding-adventures-javascript-tokens` — for `EsVersion`.

Nothing else. No `closure-*`, no `type-sidecar`, no IR or bytecode crates. The
whitelist is what keeps the AST reusable.

## What's here in v1 (this crate's first PR)

- [`Program`] — the AST root.
- [`SourceType`] — `Script` or `Module`.
- A `CvId` type alias for `String` (matching the current `correlation-vector`
  representation; see the module docs for the migration plan).

## Phase 1.x variant additions

Beyond the Phase 1 core, variants are added incrementally as the passes and
emitter grow a need for them — each behind its own `CLOC12.NN` slice:

- `BigIntLiteral` (CLOC12.15), `UndefinedLiteral` (CLOC12.16).
- `FunctionExpression` (CLOC12.149) — a function used in **value** position
  (`var f = function () {}`, IIFEs `(function () {})()`, function-valued
  properties/arguments). The expression sibling of `FunctionDeclaration`; the
  only structural difference is `id: Option<Identifier>` (anonymous, or a
  body-local name). Rolled out bottom-up: this crate adds the node; the
  `javascript-parser` bridge, `closure-emitter`, and the passes wire it up in
  follow-on slices.
- `ArrowFunctionExpression` (CLOC12.151) — the `=>` form (`x => x + 1`,
  `() => {}`, `async x => f(x)`). No `id` (always anonymous) and no `generator`;
  the body is a new `ArrowBody` enum — a `Block(BlockStatement)` or a concise
  `Expression(Box<Expression>)`. The node, `closure-emitter`, and all pass
  traversals land in one atomic PR (adding an `Expression` variant makes every
  exhaustive `match` non-exhaustive); the bridge-enable + conformance port
  follow. (Methods, getters/setters, and classes remain Phase 3.)
- `TemplateLiteral` (CLOC12.154) — backtick template strings (`` `abc` ``,
  `` `a${x}b` ``). Parallel `quasis: Vec<TemplateElement>` (fixed string parts)
  and `expressions: Vec<Expression>` (`${…}` inserts) with the ESTree invariant
  `quasis.len() == expressions.len() + 1`. `TemplateElement` keeps `raw` /
  `cooked` / `tail`. Node + `closure-emitter` + all pass traversals land in one
  atomic PR; the bridge-enable + conformance port follow. (Tagged templates
  `` tag`…` `` remain Phase 3.)
- `UpdateExpression` (CLOC12.158) — the `++` / `--` read-modify-write forms
  (`++x`, `x++`, `--x`, `x--`). `UpdateExpression { operator: UpdateOperator,
  prefix: bool, argument: Box<Expression> }`. Distinct from `UnaryExpression`
  because `++`/`--` carry a side effect (passes must not drop them) and require
  a writable-reference operand; `prefix` splits `++x` (yield the new value)
  from `x++` (yield the old value). Node + `closure-emitter` + all pass
  traversals land in one atomic PR; the bridge-enable + conformance port
  follow (gap-159).
- `NewExpression` (CLOC12.159) — the `new` operator: `new Ctor(a, b)`.
  `NewExpression { callee: Box<Expression>, arguments: Vec<Expression> }`.
  Structurally a `CallExpression` but a distinct node: its semantics are object
  *construction* (a pass must never rewrite `new f()` into `f()`), and its
  callee excludes a trailing call (`new (f())()` needs the parens). Node +
  `closure-emitter` + all pass traversals land in one atomic PR; the
  bridge-enable + conformance port follow (gap-160).
- `SequenceExpression` (CLOC12.160) — the comma operator: `a, b, c`. Evaluates
  each operand left to right and yields the last.
  `SequenceExpression { expressions: Vec<Expression> }`. It is the **loosest**
  expression (below assignment), so a sequence sub-operand almost always needs
  parentheses (`f((a,b),c)`, `x=(a,b)`) — the exceptions are statement position
  (`a,b,c;`) and a computed-member key (`obj[a,b]`). Node + `closure-emitter` +
  all pass traversals land in one atomic PR; the bridge-enable + conformance
  port follow (gap-161).

## What's coming (follow-up PRs)

Per CLOC02:

- `Statement`, `Expression`, `Declaration`, `Pattern`, `Class`, `Module`,
  `Literal` variants — landing in their own files (`statement.rs`,
  `expression.rs`, …) so each diff stays reviewable.
- A `walk` visitor pattern over the whole tree.
- Builder helpers.

## Why this is a separate crate

If the AST lived in `javascript-parser`, every consumer would depend on the
parser. Splitting it out keeps the layering clean and lets the future V8 clone
reuse the parser's output without taking on the parser's dependencies.

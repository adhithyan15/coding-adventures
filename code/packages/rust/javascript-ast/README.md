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
  `` tag`…` `` are the separate `TaggedTemplateExpression` node, CLOC12.161.)
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
- `TaggedTemplateExpression` (CLOC12.161) — a tagged template `` tag`abc${x}` ``:
  the `tag` callee is applied to the template that directly follows it (no call
  parentheses). `TaggedTemplateExpression { tag: Box<Expression>, quasi:
  TemplateLiteral }` — the `quasi` reuses the existing `TemplateLiteral` node
  (CLOC12.154). It is primary (`PREC_PRIMARY`), so a member access on it is
  paren-free (`` a`x`.length ``) while a looser tag wraps (`` (a,b)`x` ``). Node
  + `closure-emitter` + all pass traversals land in one atomic PR; the
  bridge-enable + conformance port follow (gap-162).
- `SpreadElement` (CLOC12.162) — a spread `...arg`: the `...` prefix that
  unpacks an iterable into a call/`new` argument list (`f(...a)`) or an
  array-literal element (`[...a]`). `SpreadElement { argument: Box<Expression> }`.
  Not a free-standing expression (a bare `...x` is a syntax error), but modelled
  as an `Expression` variant so it slots into the existing `Vec<Expression>`
  argument/element lists without a parallel enum, and every walker that recurses
  those `Vec`s reaches `argument` for free. It tags at `PREC_ASSIGNMENT`,
  matching the assignment-position list slots it lives in. Node +
  `closure-emitter` + all pass traversals land in one atomic PR; the
  bridge-enable + conformance port follow (gap-163).
- `YieldExpression` (CLOC12.163) — a generator `yield`: a bare `yield`, a value
  `yield x`, or a delegating `yield* xs`.
  `YieldExpression { delegate: bool, argument: Option<Box<Expression>> }` — the
  `argument` is optional (a bare `yield` has no operand) and `delegate` splits
  `yield` from `yield*`. It tags at `PREC_ASSIGNMENT` (a yield is loose, so a
  tighter parent wraps the whole yield: `(yield a)+1`). Node + `closure-emitter`
  + all pass traversals land in one atomic PR; the bridge-enable + conformance
  port follow.
- `AwaitExpression` (CLOC12.164) — an `await x` async-suspend operator: waits on
  a promise operand and resumes with its settled value.
  `AwaitExpression { argument: Box<Expression> }` — the `argument` is mandatory
  (a bare `await` has no meaning), and there is no optional/delegate axis, unlike
  `YieldExpression`. It tags at `PREC_UNARY`, printed like the word-unaries
  typeof/void/delete: it binds tighter than binary parents (`await a+b`) but
  member/call/new parents wrap the whole await (`(await p).x`, `(await f)()`).
  Node + `closure-emitter` + all pass traversals land in one atomic PR; the
  bridge-enable + conformance port follow.
- `ThisExpression` (CLOC12.165) — the `this` keyword: a reserved-word **leaf**
  primary that reads the current execution context's `this` binding.
  `ThisExpression { cv }` — no operand, the same shape as `NullLiteral` /
  `UndefinedLiteral`. Modelled as its own node rather than
  `Identifier { name: "this" }` so the renaming passes never touch it (a
  reserved word can never be a variable name). It tags at `PREC_PRIMARY` and
  prints as the bare keyword — it never needs wrapping (`this.x`, `this()`,
  `f(this)` all print bare) and never forces a paren around an operand. Node +
  `closure-emitter` + all pass traversals land in one atomic PR; the
  bridge-enable + conformance port follow.
- `Super` (CLOC12.166) — the `super` keyword: the reserved-word **leaf**
  sibling of `this`, naming the home object's prototype (`super.m()`,
  `super[k]`) or the superclass constructor (`super(a, b)`). `Super { cv }` —
  no operand, the same shape as `ThisExpression`. Named `Super` to match
  ESTree's node type exactly (ESTree uses bare `Super`, asymmetric with
  `ThisExpression`). Modelled as its own node so the renaming passes never
  touch it. `super` is syntactically restricted to member-object / call-callee
  position inside a method or derived constructor, but that is the parser's
  concern — the AST treats it as a plain leaf primary. It tags at
  `PREC_PRIMARY` and prints as the bare keyword. Node + `closure-emitter` +
  all pass traversals land in one atomic PR; the bridge-enable + conformance
  port follow.
- `NewTarget` (CLOC12.167) — the `new.target` meta-property: the reserved-word
  **leaf** sibling of `this` / `super`, reading the constructor a function was
  invoked with (`undefined` for a plain call). `NewTarget { cv }` — no operand,
  the same shape as `Super`. Spelled with two tokens plus a dot in source, but
  the `.` is part of the spelling (not a member access), so it is modelled as
  an atomic leaf node rather than a `MemberExpression`; the renaming passes
  never touch it. It tags at `PREC_PRIMARY` and the emitter prints the literal
  spelling `new.target`. Node + `closure-emitter` + all pass traversals land in
  one atomic PR; the bridge-enable + conformance port follow.
- `ImportMeta` (CLOC12.168) — the `import.meta` module meta-property (host
  metadata about the current module, e.g. `import.meta.url`): the
  `MetaProperty` sibling of `NewTarget`. `ImportMeta { cv }` — no operand, the
  same leaf shape as `NewTarget`. Spelled with three tokens (`import` `.`
  `meta`) in source, but the `.meta` is part of the fixed spelling (not a member
  access — `import` is a reserved word with no accessible identifier), so it is
  modelled as an atomic leaf rather than a `MemberExpression`; the renaming
  passes never touch it. It tags at `PREC_PRIMARY` and the emitter prints the
  literal spelling `import.meta`. Node + `closure-emitter` + all pass traversals
  land in one atomic PR; the bridge-enable + conformance port follow.

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

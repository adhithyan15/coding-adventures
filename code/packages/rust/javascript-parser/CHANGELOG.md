# Changelog

All notable changes to the `coding-adventures-javascript-parser` crate will be documented in this file.

## [0.50.0] - 2026-07-12

### Fixed — CLOC12.186: bridge `let`/`const` init in a C-style `for` header

`convert_for_statement` only modelled a `var` init (`variable_declaration_list`)
in the classic `for (init; test; update)` header; a `let`/`const` init
(`for (let i = 0; …)`) parsed to a bare `binding_list` node that fell through to
`convert_expression`, raising an `InternalError`
("unknown expression rule 'binding_list'") that declined the WHOLE file to
WHITESPACE_ONLY — a wide-probe find, and the extremely common counted-loop idiom.

The grammar inlines the lexical declaration into the header: the `let`/`const`
keyword is a direct token child of the `for_statement`, and the bindings are a
bare `binding_list` node whose children are `lexical_binding` nodes. The init
phase now recognises `binding_list`, reads the kind via `has_token`, and reuses
`convert_variable_declarator` (the same shape `convert_lexical_declaration` reads)
to build a `ForInit::VariableDeclaration`. The AST already carried the var/let/
const kind, so this is a pure bridge fix.

New tests `for_lexical_init_bridges`, `for_lexical_init_multi_binding_bridges`,
`for_var_init_still_bridges`.

## [0.49.0] - 2026-07-12

### Added — CLOC12.185: bridge parenthesised object-body arrows `() => ({…})`

Follow-on to CLOC12.184. `convert_arrow_function` still declined a
**parenthesised object expression body** `() => ({…})` — even though the emitter
already re-wraps an `ObjectExpression` arrow body in parens so it is never
misread as a block. The guard now, when the body parses as an `ObjectExpression`,
branches on the concise_body's leftmost token:

- leads with `{` (a bare block body per the ES spec): empty → `ArrowBody::Block`
  (CLOC12.184), non-empty → DECLINE (contents would need re-parsing as
  statements);
- leads with `(` (a genuine parenthesised object expression body): **keep** it as
  `ArrowBody::Expression(ObjectExpression)` (CLOC12.185).

Pure bridge change — the emitter was already correct. Covers `() => ({})` and
`() => ({a:1})`. Flipped the former `arrow_paren_object_body_still_declines` /
`arrow_object_concise_body_is_declined` tests to `arrow_paren_object_body_bridges`
/ `arrow_paren_object_concise_body_bridges`.

## [0.48.0] - 2026-07-12

### Fixed — CLOC12.184: bridge the empty-block arrow `() => {}`

`convert_arrow_function` declined every arrow whose concise body parsed as an
`ObjectExpression`, dropping the whole file to WHITESPACE_ONLY. That over-broadly
caught the extremely common `() => {}` idiom: the grammar buckets the bare `{}`
after `=>` as an *empty object literal*, but per the ES spec a `{` immediately
after `=>` ALWAYS opens a **block** body (an object body must be parenthesised,
`=> ({})`).

The bridge now disambiguates by the concise_body's leftmost token (new
`leftmost_token` helper): a bare block body leads with `{`, a parenthesised
object body leads with `(`. A bare **empty** object-literal body (`=> {}`) is
reinterpreted as an `ArrowBody::Block` with no statements. `() => ({})` (leads
with `(`) and a non-empty `=> {…}` the grammar mis-bucketed (its contents would
need re-parsing as statements) both still DECLINE — never a mis-emit.

New tests `arrow_empty_block_body_bridges`, `arrow_paren_object_body_still_declines`,
`arrow_nonempty_brace_body_still_declines`.

## [0.47.0] - 2026-07-11

### Added — CLOC12.183: bridge ES2021 logical assignment operators

`parse_assignment_op` now recognises `&&=`, `||=`, and `??=`, mapping them to
the new `AssignmentOperator::LogicalAndEq` / `LogicalOrEq` / `NullishCoalescingEq`
variants (javascript-ast 0.37.0). These three operators parsed fine but
previously fell through to `None`, producing an `InternalError`
("unknown assignment operator") that dropped the whole file to WHITESPACE_ONLY.
New `logical_assignment_operators_bridge` test confirms all three bridge and a
neighbouring bitwise `&=` still maps to its own distinct variant.

## [0.46.0] - 2026-07-11

### Added — CLOC12.182: bridge private generator methods (`*#m(){}`)

`convert_private_method_definition` detected the leading `*` token but bundled it
into a blanket `"*" | "async" => decline` arm, dropping the file to
WHITESPACE_ONLY. The `*` is now split out into its own `saw_star` flag and set on
the method value's `FunctionExpression.generator`, exactly like a public
generator method (CLOC12.181) — `yield` is a modelled `YieldExpression`, and the
emitter's `emit_class_member` already reprints the `*` before the private-name
key. Covers `*#g(){}` and `static *#g(){}`.

Only the private **async** form (`async #m(){}`) still DECLINES — `await` is not
yet modelled (grammar-blocked, gap-165). A private name can never be the
`constructor`, so the generator's kind stays [`MethodKind::Method`]; private
get/set accessors (CLOC12.179) are unaffected.

The former `class_private_generator_still_declines` test flipped to
`class_private_generator` (asserting the generator flag); added
`class_static_private_generator` and `class_private_async_method_declines` to
lock in the static form and the remaining async decline.

## [0.45.0] - 2026-07-11

### Added — CLOC12.181: bridge generator methods (`*m(){}`)

`convert_method_definition` already detected the leading `*` token (`saw_star`)
but then DECLINED the method (dropping the file to WHITESPACE_ONLY) as a
conservative measure. That decline is now stale: `yield` is a modelled
`YieldExpression` (CLOC12.163), a top-level generator *function* already bridges,
and the emitter's `emit_class_member` already reprints the `*` from the value's
`generator` flag. So a generator method now bridges by setting
`generator: saw_star` on the method's `FunctionExpression` value — a
generator's `FunctionExpression` flows through every optimization pass exactly
like a `function*`.

Covers both `class C { *gen(){} }` (declaration) and `x = class { *gen(){} }`
(expression), plus `static *gen(){}`. The `constructor` classification is guarded
against a stray `*` (`*constructor(){}` is a SyntaxError — a generator is never a
constructor). Accessor generators (`get`/`set` + `*`) are grammatically
impossible; private generator methods (`*#m(){}`) remain declined in
`convert_private_method_definition` (a later slice).

2 former decline tests flipped to success
(`class_generator_method_bridges`, `class_decl_generator_method_bridges`); a new
`class_static_generator_method_bridges` test covers the static form. Async
methods (`async_method`) still DECLINE one level up (grammar-blocked).

## [0.44.0] - 2026-07-11

### Added — CLOC12.180: bridge computed member keys (`[expr]`)

`convert_property_key` declined every computed `[expr]` key (dropping the file to
WHITESPACE_ONLY); it now lowers the inner key expression to
`PropertyKey::Expression` (the typed-AST variant the emitter already brackets).
Because `convert_property_key` is the shared key converter, one change enables
computed keys in **all** three positions:

- a class computed **field** (`class C { [k] = v }`),
- a class computed **method** (`class C { [k](){} }`), and
- an object-literal computed key (`{ [k]: v }`).

Each construction site (`convert_class_field`, `convert_method_definition`, the
object `Property` builder) now sets its `computed` flag to
`matches!(key, PropertyKey::Expression(_))` instead of hard-coding `false`. The
inner key is routed through the shared `convert_expression`, so an unmodelled key
expression DECLINES (safe WHITESPACE_ONLY) rather than mis-emit.

2 former decline tests flipped to success (`class_computed_method_key`,
`class_computed_field_key`); a new `object_computed_key` test covers the object
position. Full parser suite (216) green. MINOR.

## [0.43.0] - 2026-07-11

### Added — CLOC12.179: bridge private accessors (`get #x()` / `set #x()`)

`convert_private_method_definition` (CLOC12.178) declined the private *accessor*
forms; it now lowers them. A private getter (`get #x(){}`) becomes a
`ClassMember::Method` with `MethodKind::Get` and a `PropertyKey::PrivateName`
key; a private setter (`set #x(v){}`) becomes `MethodKind::Set` with its single
parameter. The `get` / `set` keyword precedes the `PRIVATE_NAME` token as a
direct token child (read alongside `static`), so `static get #x(){}` also works.

Still declined: the private **generator** (`*#m(){}`) and **async** forms — like
a public generator method, they DECLINE (safe WHITESPACE_ONLY), never a mis-emit.

4 new bridge tests (private getter, private setter, static private getter; the
former `class_private_getter_still_declines` replaced by a
`class_private_generator_still_declines` guard). MINOR.

## [0.42.0] - 2026-07-11

### Added — CLOC12.178 PR1: bridge private methods → `ClassMember::Method` with a private key

A private class method (`class C { #m(){} }`) parses as its own
`private_method_definition` grammar node (distinct from `method_definition`),
which `convert_class_element` previously declined (dropping the file to
WHITESPACE_ONLY). It now dispatches that node to the new
`convert_private_method_definition`, producing a `ClassMember::Method` whose key
is a `PropertyKey::PrivateName` (javascript-ast 0.36.0):

- The key is the node's leading `PRIVATE_NAME` token (`#m`), lowered by the
  shared `private_name_key` helper (the `#` stripped, re-added by the emitter) —
  exactly as a private *field* key.
- The `static` modifier lives *inside* the `private_method_definition` node (the
  grammar's `[ "static" ]`), unlike a public method's `static` (on the
  `class_element`), so it is read here. `static #m(){}` works.
- Params and body reuse the shared `convert_formal_parameters` /
  `convert_formal_parameter` / `convert_function_body`, mirroring
  `convert_method_definition`. The kind is always `Method` (a private name can
  never be the `constructor`).
- The private **getter / setter / generator** forms (`get #x(){}`, `set #x(v){}`,
  `*#m(){}`) carry accessor / evaluation semantics not yet modelled, so — like a
  public generator method — they DECLINE (safe WHITESPACE_ONLY), never a
  mis-emit. A later slice.

5 new bridge tests (the former `class_private_method_still_declines` flipped to
success; a `class_private_getter_still_declines` guards the decline path); full
parser suite (212 tests) green. MINOR.

## [0.41.0] - 2026-07-11

### Added — CLOC12.177 PR2: bridge private class fields → `PropertyKey::PrivateName`

A private class field (`class C { #x = 1; }`) parses as a `class_field_declaration`
whose key is a bare `PRIVATE_NAME` token (`#x`) rather than a `property_name`
node, so `convert_class_field` previously declined it (dropping the file to
WHITESPACE_ONLY). It now detects that token via the new `private_name_key` helper
and lowers it to `PropertyKey::PrivateName` (javascript-ast 0.36.0):

- The `PRIVATE_NAME` token's `value` **includes** the leading `#` (e.g. `"#x"`);
  the stored `PrivateName.name` omits it (mirroring `Identifier`), so the helper
  strips the `#`. The emitter re-adds it.
- Works for a bare field (`#x;` → `value: None`), an initialized field
  (`#x = 1;`), and a `static` private field (`static #x = 1;`) — the `static`
  token precedes the `PRIVATE_NAME` token and still sets `is_static`.
- A private **method** (`#m(){}`) is a *separate* `private_method_definition`
  grammar node, not yet bridged — it still DECLINES (safe WHITESPACE_ONLY), never
  a mis-emit. A later slice.

5 new bridge tests (the former `class_private_field_declines` flipped to success).
MINOR.

## [0.40.0] - 2026-07-11

### Added — CLOC12.176 PR2: bridge static-init blocks → `ClassMember::StaticBlock`

The grammar already parses a static initialization block (`static { … }`) as a
`static_block` node inside `class_element`, but the bridge declined it (dropping
the file to WHITESPACE_ONLY). `convert_class_element` now dispatches a
`static_block` member to the new `convert_static_block`, producing
`ClassMember::StaticBlock(BlockStatement)` (javascript-ast 0.35.0):

- The block **body** reuses the shared statement converter (`convert_statement`),
  so the full statement surface is reachable — expression statements
  (`x = 1;`), lexical declarations (`let z = 2;` → `Statement::Declaration`),
  and multiple statements in source order. Identical shape to
  `convert_block_statement`.
- The leading `static` keyword lives *inside* the `static_block` node (not
  hoisted onto `class_element` like a method/field modifier), so the existing
  modifier loop never sees it — `is_static` stays false and the static-block arm
  ignores it. No special modifier handling is needed.
- An empty block (`static {}`) maps to an empty `body`.

Works in both class-expression and class-declaration bodies (shared conversion);
a static block and a field/method coexist in one body in source order.
5 new bridge tests. MINOR.

## [0.39.0] - 2026-07-11

### Added — CLOC12.175 PR2: bridge class fields → `ClassMember::Field`

The grammar already parses a class field (`class_field_declaration`) but the
bridge declined it (dropping the file to WHITESPACE_ONLY). `convert_class_element`
now dispatches a `class_field_declaration` member to the new `convert_class_field`,
producing `ClassMember::Field(PropertyDefinition)` (javascript-ast 0.34.0):

- **key** reuses `convert_property_key` — the same `property_name` node a method
  key uses (identifier / string / numeric). A computed `[expr]` key DECLINES (a
  later slice) → WHITESPACE_ONLY, sound, mirroring the method surface.
- **initializer** is the optional `assignment_expression`; a bare field (`y;`)
  maps to `value: None`.
- **`static`** is read from the field's own leading `static` token (inside
  `class_field_declaration`, not hoisted to `class_element` like a method's).
- A **private** field (`#x`) — a bare `PRIVATE_NAME` token with no `property_name`
  node — DECLINES as an unmodelled shape (safe fallback), never a mis-emit.

Works in both class-expression and class-declaration bodies (shared conversion).
10 new bridge tests. MINOR.

## [0.38.1] - 2026-07-11

### Fixed — CLOC12.175 PR1: `ClassMember` test bindings

`javascript-ast` 0.34.0 added `ClassMember::Field`, making `ClassMember` a
two-variant enum. The bridge's class-conversion tests bound a member with an
irrefutable `let ClassMember::Method(m) = &c.body[0];`, which the new variant
makes refutable. Converted each of the 13 test bindings to
`let ClassMember::Method(m) = … else { panic!("expected a method member") };`.
Bridge production code is unchanged — it still emits only `ClassMember::Method`
(field bridging is CLOC12.175 PR2).

## [0.38.0] - 2026-07-11

### Added — CLOC12.174 PR2: bridge `class_declaration` → `Declaration::ClassDeclaration`

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` node (PR1). The
bridge's `convert_source_element` now converts a top-level class **declaration**
(`class C { … }`) into `Declaration::ClassDeclaration` via the new
`convert_class_declaration`, instead of declining (`UnsupportedSyntax`, which
dropped the whole file to WHITESPACE_ONLY).

The grammar wraps a class declaration in `decorated_class_declaration` →
`class_declaration` at the source-element level (both the decorated wrapper and a
bare `class_declaration` are handled; a *decorated* form carrying actual
`@decorator`s declines — a later slice). The `class_declaration` node's flat
child shape is **identical to `class_expression`** (`class` / NAME / optional
`class_heritage` / `class_body`) except the name is **required** — so
`convert_class_declaration` reuses `convert_class_heritage` and
`convert_class_element` unchanged, and DECLINES a nameless class rather than
fabricate an empty id. Generator / async / computed / multi-member methods decline
to WHITESPACE_ONLY exactly as for the expression form (never a miscompile).

10 bridge unit tests (`class_decl_*`) cover the accepted forms (empty, `extends`
identifier / member, method, static, constructor, get/set, full shape) and the
generator / async declines.

## [0.37.0] - 2026-07-08

### Added — CLOC12.173 PR2: bridge class expressions `class { … }` → `ClassExpression` (closes gap-167)

`convert_expression` now converts a `class_expression` grammar node into the
typed `Expression::ClassExpression` (added to `javascript-ast` in PR1) instead
of declining it as `UnsupportedSyntax`. A greenfield class expression therefore
flows through the typed AST and every downstream pass.

The parse-tree shape was established by dumping the grammar parser's output (a
throwaway probe, removed with this commit):

- `class_expression = [ "class", NAME?, class_heritage?, class_body ]`. The lone
  direct-child token other than `class` is the class name (`None` for
  `class {}` / `class extends B {}`).
- `class_heritage = [ "extends", <operand> ]`. The operand is a bare `NAME`
  token (`extends B` → `Identifier`) or a `left_hand_side_expression` node
  (`extends ns.B` → `convert_expression`).
- `class_body`'s `class_element` children each wrap one `method_definition`. A
  leading bare `static` token marks a static member.
- `method_definition = [ ("get"|"set")?, property_name, "(", params, ")",
  "{", function_body?, "}" ]`. A single param parses as a direct
  `formal_parameter`; two or more under a `formal_parameters` wrapper — both are
  collected. A `constructor` key (non-static, non-accessor) becomes
  `MethodKind::Constructor`.

New converters: `convert_class_expression`, `convert_class_heritage`,
`convert_class_element`, `convert_method_definition`.

**Declined sub-forms (safe — a decline drops the whole file to WHITESPACE_ONLY,
never a miscompile).** The typed slice does not yet model, so the bridge
DECLINES via `UnsupportedSyntax`:

- **Computed keys** `[k]() {}` — `convert_property_key` already declines a
  computed property key.
- **Generator methods** `*m() {}` — the `*` sits inside `method_definition`; a
  generator carries semantics the slice does not model, and dropping the `*`
  would be a miscompile, so it declines.
- **`async` methods** `async m() {}` — the grammar attaches `async` as a
  *distinct* `async_method` node under `class_element` (not a `method_definition`
  token), so `convert_class_element` declines any member node that is not a plain
  `method_definition`. This node-kind check (not just a token scan) is what stops
  the `async` from being silently dropped.
- **`extends <call>`** e.g. `extends mix(B)` — the grammar flattens the call
  into several ambiguous `NAME` tokens with no clean operand node, so the
  heritage converter declines rather than mis-read the super-class.

**Grammar gap (not a bridge decision):** the grammar requires an explicit `;`
*between* class members (`class { m(){}; n(){} }`); an un-separated multi-member
class is a parse error and falls back to WHITESPACE_ONLY. Single-member classes
parse and bridge cleanly.

16 new bridge unit tests (`class_*`) cover empty / named / `extends`
identifier+member / method / single-param / static / getter / setter / a method
literally named `get` / constructor / static-`constructor`-is-plain-method, and
the computed / generator / `async` declines.

## [0.36.0] - 2026-07-08

### Added — CLOC12.172 PR2: bridge regex literals `/pat/flags` → `RegExpLiteral` (closes gap-RegExpAsIdentifier)

`convert_primary_token` now recognises the lexer's **REGEX** token and builds a
real `RegExpLiteral` (added to `javascript-ast` 0.31.0) instead of letting the
catch-all mis-encode the literal as an `Identifier` whose `name` is the raw
`/pat/flags` text. A parse-tree dump confirmed the whole literal arrives as one
token, discriminated exactly like BIGINT — `type_ = Name`, `type_name =
Some("REGEX")`, `value = "/pat/flags"`:

```text
  /ab+c/gi  → Token{ type_name: REGEX, value: "/ab+c/gi" }  → pattern "ab+c", flags "gi"
  /a\/b/    → Token{ type_name: REGEX, value: "/a\/b/"   }  → pattern "a\/b", flags ""
```

A new `split_regex_literal` helper splits `value` into `(pattern, flags)` around
the **closing** delimiter, scanning from index 1 while honouring `\`-escapes
(`\/` is literal) and character classes (`[...]`, inside which `/` is literal) —
so the delimiter it picks is the true terminating `/`. Debug-build asserts guard
that the pattern carries no raw line terminator and the flags are a subset of
`[dgimsuy]` (defence-in-depth against future lexer drift). 4 bridge unit tests +
a splitter unit test covering escaped-slash and char-class edges.

**Scope note (discovered lexer gap):** the splitter correctly handles a `/`
*inside* a character class (`/[/]/` → pattern `[/]`), but the current lexer does
not yet tokenise that shape — it stops the literal at the inner `/`. Enabling
`/[/]/` end-to-end is a lexer change tracked separately; this bridge PR is
already correct for when the lexer produces the right token.

## [0.35.0] - 2026-07-08

### Added — CLOC12.171 PR2: bridge optional chaining `a?.b` / `a?.[k]` / `a?.()` (closes gap-OptionalChain)

`convert_optional_chain_expression` now **builds** the optional-chain nodes that
`javascript-ast` 0.30.0 added, instead of declining every `?.` to
`UnsupportedSyntax` (which dragged the whole file to WHITESPACE_ONLY). A
parse-tree dump confirmed the grammar spells `?.` as its own token directly
followed by the suffix:

```text
  a?.b    → member_expr  Token("?.")  Token("b")            → OptionalMember (dot)
  a?.[k]  → member_expr  Token("?.")  Token("[") expr "]"   → OptionalMember (computed)
  a?.()   → member_expr  Token("?.")  Node(arguments)       → OptionalCall
  a?.b.c  → member_expr  Token("?.")  Token("b") "." "c"    → only `b` optional
```

The suffix walker gained a `?.` arm that emits an `OptionalMemberExpression`
(dot or computed) / `OptionalCallExpression` for the marked link; a non-optional
suffix that follows (the `.c` in `a?.b.c`) stays an ordinary member/call whose
object is the optional node. When any optional link appeared, the whole spine is
wrapped once in a `ChainExpression` before returning; a chain with no optional
link (`a.b.c`) is returned bare, exactly as before. 5 bridge unit tests.

**Scope:** the primary optional-chain path — every `?.` whose base is a
`member_expression` — routes through this function and is now supported. The two
remaining `?.` decline arms (in `convert_call_expression` /
`convert_member_expression`, reached only when the base is itself a *call*, e.g.
`f()?.x`) still decline gracefully to WHITESPACE_ONLY — a safe follow-up, never
a miscompile.

## [0.34.0] - 2026-07-07

### Added — CLOC12.170 PR2: bridge object spread `{...o}` → `ObjectMember::Spread` (closes gap-SpreadProperty)

`convert_object_literal` now converts an object spread `{...o}` (ES2018) into an
`ObjectMember::Spread`. Previously `convert_property_definition` declined the
spread with `UnsupportedSyntax { rule: "SpreadProperty" }`, dragging any file
containing `{...o}` to WHITESPACE_ONLY.

Dumping the parse tree showed the spread form nests one level deeper than the
call/array spread (CLOC12.162): a `property_definition` holds a single
`object_spread_property` Node child whose own children are
`[ Token("..."), Node(assignment_expression) ]` (the call/array spread's ELLIPSIS
sits directly under `spread_element`, a different rule). So the spread is
detected by that inner rule name — `convert_object_literal` finds an
`object_spread_property` child, extracts its `assignment_expression` (via
`node_children`, which strips the ELLIPSIS), converts it, and wraps it in
`ObjectMember::Spread(SpreadElement { .. })` — reusing the same `SpreadElement`
the call/array spread uses, so it prints through `emit_object_spread`. Member
order is preserved (`{a: 1, ...o}` keeps the plain property then the spread),
which is observable since a later member overrides an earlier key. The dead
`SpreadProperty` decline arm is removed from `convert_property_definition`
(spreads never reach it now). 3 bridge unit tests (`{...o}` → sole `Spread`
member; `{a: 1, ...o}` → `[Property, Spread]` in order; `f({...o})` bridges in
call-argument position).

## [0.33.0] - 2026-07-07

### Added — CLOC12.169 PR2: bridge `import(x)` → `Expression::ImportExpression` (closes gap-170)

The bridge now converts the grammar's `dynamic_import` node into
`Expression::ImportExpression` (the dynamic-`import()` call expression). Previously
the `dynamic_import` rule — whose children are `[Token("import"), Token("("),
Node(source_expr), Token(")")]` — fell through the expression dispatch to the
`other =>` internal-error arm, dragging any file containing `import(x)` to
WHITESPACE_ONLY. A new `convert_dynamic_import` extracts the sole Node child (the
module-specifier expression, via `node_children`), converts it with
`convert_expression`, and wraps it in `ImportExpression { cv, source }`. Unlike
the atomic `import.meta` leaf (v0.32.0), this is a **compound** single-operand
node: the `source` is recursively converted, so a fold inside the specifier
(e.g. `import("a" + "b")` → `import("ab")`) propagates. The `import` token's `cv`
becomes the node's provenance, mirroring `convert_import_meta`. 3 new bridge unit
tests (`import("m")` → `ImportExpression` with a `StringLiteral` source;
`import(x)` → `Identifier` source; `f(import("m"))` bridges in argument position).

## [0.32.0] - 2026-07-07

### Added — CLOC12.168 PR2: bridge `import.meta` → `Expression::ImportMeta` (closes gap-169)

The bridge now converts the grammar's `import_meta` leaf into
`Expression::ImportMeta` (the module meta-property, sibling of `new.target`).
Previously the `import_meta` rule — whose children are the three bare tokens
`[Token("import"), Token("."), Token("meta")]` with no Node child — fell through
the expression dispatch to the `other =>` internal-error arm, dragging any file
containing `import.meta` to WHITESPACE_ONLY. A new `convert_import_meta` lowers
it to the atomic `ImportMeta` leaf (the `.meta` is part of the fixed spelling,
not a member access; the `import` token's `cv` becomes the node's provenance),
mirroring `new.target`. The dead `import_meta_expression` decline arm (a rule
name the grammar never emits for this construct) is removed. 3 new bridge unit
tests (`import.meta;` → `ImportMeta`; `import.meta.url;` → member access whose
object is `ImportMeta`; `f(import.meta);` bridges in argument position). The
closurec end-to-end diff fixture exercises the full SIMPLE pipeline. (gap-169)


## [0.31.0] - 2026-07-07

### Added — CLOC12.167 PR2: bridge `new.target` → `Expression::NewTarget` (closes gap-168)

The bridge now converts the `new.target` meta-property to
`Expression::NewTarget { cv }` instead of declining it (dragging the whole file
to WHITESPACE_ONLY). The grammar emits `new.target` as three bare tokens
`[Token("new"), Token("."), Token("target")]` inside a `member_expression` with
**no Node child** — probed and confirmed, not routed through the dedicated
`new_target_expression` rule — so `convert_member_expression` distinguishes it
from the argumented `new X(args)` constructor (which always has a Node callee)
on `nodes.is_empty()`. A `new_target` flag (`nodes.is_empty() &&
has_token("new") && has_token("target")`) relaxes the empty-nodes guard
(mirroring the `super`-token base of gap-167) and the meta-property returns the
atomic `NewTarget` leaf, taking the `new` token's `cv` as provenance — the `.`
is part of the fixed spelling, not a member access, so there is nothing to
fold. `new.target` was already parseable (a bare `new.target;` parses
standalone), so this is a pure bridge slice — **no grammar work**. 3 new bridge
tests (`new_target_meta_property`, `new_target_in_function_return`,
`new_target_as_member_object`); 147 pass.

## [0.30.0] - 2026-07-04

### Added — CLOC12.166 PR2: bridge `super` → `Expression::Super` (closes gap-167)

The bridge now converts the `super` primary to `Expression::Super { cv }`
instead of declining it to `UnsupportedSyntax` (dragging the whole file to
WHITESPACE_ONLY). Unlike `this` (gap-166, a bare keyword handled in
`convert_primary_token`), `super` is emitted by the grammar as a bare *token*
directly among the `member_expression` children (not wrapped in a
`primary_expression` Node), so `convert_member_expression` gains a `super`
base-branch parallel to the `new` branch: the `super` token becomes the base
`Expression::Super` and the existing suffix-fold loop composes `.NAME` / `[expr]`
/ call arguments onto it (`super.m`, `super[k]`, `super.m(a)`). The
`nodes.is_empty()` guard is relaxed for the `super`-token base (it has no Node
child), and a lone `super` returns `Super` directly. Like `this`, `super` was
already parseable, so this is a pure bridge slice — **no grammar work**. 4 new
bridge unit tests (`super.x`, `super[k]`, `super.m(1 + 2)`). (CLOC12.166 PR2)


## [0.29.0] - 2026-07-04

### Added — CLOC12.165 PR2: bridge `this` → `Expression::ThisExpression` (closes gap-166)

The bridge now converts the `this` primary token to
`Expression::ThisExpression { cv }` (mirroring the `null` / `undefined` /
`true` / `false` keyword arms in `convert_primary_token`) instead of declining
it to `UnsupportedSyntax` and dragging the whole file to WHITESPACE_ONLY.
Unlike `await` (gap-165, blocked on grammar), `this` was already parseable —
the bridge reached and explicitly declined it — so this is a pure bridge slice
with no grammar work. 2 new bridge unit tests (`this;` → `ThisExpression`;
`this.x;` → a member access whose object is a `ThisExpression`). The `this`
node + emit + all nine downstream passes landed in CLOC12.165 PR1
(javascript-ast 0.24.0, closure-emitter 0.29.0).

## [0.28.0] - 2026-07-04

### Added — CLOC12.163 PR2: bridge generator functions and `yield` (closes gap-164)

The bridge now converts `generator_declaration` / `generator_expression`
(sharing the function converter; a `*` token sets the `generator` flag and is
skipped during name extraction) and `yield_expression` →
`Expression::YieldExpression` (delegate = the node carries a `*`; the operand
is the sole child node), instead of declining these to `UnsupportedSyntax` and
dragging the whole file to WHITESPACE_ONLY. 5 new bridge unit tests (generator
declaration with `yield`, delegating `yield*`, binary yield operand, generator
expression in value position, and a plain-function-is-not-a-generator guard).
Known grammar limitation: a bare operand-less `yield` (`function*g(){yield;}`)
does not parse — the grammar's `yield_expression` production requires an
operand — so the bridge only ever produces `Some(argument)`; tracked as a
separate grammar gap.

## [0.27.0] - 2026-07-03

### Added — CLOC12.162 PR2: bridge spread `...arg` → `SpreadElement` (closes gap-163)

`convert_argument` (call / `new` argument lists) and `convert_array_literal`
(array elements) now recognise the grammar's `spread_element` node — whose
children are `[ Token("..."), Node(assignment_expression) ]` — and wrap the
converted inner expression as `Expression::SpreadElement` instead of returning
`UnsupportedSyntax`. Previously any `f(...a)`, `new X(...a)`, or `[...a]`
dragged the whole file to WHITESPACE_ONLY. The parse shape was confirmed by
dumping the tree: the `...` sits directly under `spread_element`, so
`has_token(node, "...")` gates exactly the spread case and `node_children`
(which strips the token) yields the single inner expression. 6 new bridge unit
tests (call spread, interleaved-call arity, `new` spread, array spread,
interleaved-array count, and a guard that a plain non-spread argument is *not*
wrapped). The AST node + emit landed in CLOC12.162 PR1 (#7515); the CodePrinter
conformance port follows in PR3.

## [0.26.0] - 2026-07-03

### Added — CLOC12.161 PR2: bridge tagged templates → `TaggedTemplateExpression` (closes gap-162)

`convert_member_expression`'s suffix walk now converts a `template_literal`
node that follows a base expression into an
`Expression::TaggedTemplateExpression` (the accumulated base becomes the `tag`,
the template the `quasi`) instead of declining to `UnsupportedSyntax` — which
had dragged the whole file to WHITESPACE_ONLY (gap-162, now closed). The quasi
reuses the existing `convert_template_literal` (CLOC12.155), so no new parsing
is introduced; the wrap continues the suffix walk, so `` a`x`.length `` and
`` a`x`() `` chain naturally. 4 new bridge tests (identifier tag, member-chain
tag, member-access-on-tagged chaining, and the guard that an *untagged*
template still bridges to a bare `TemplateLiteral`). Scope note: substitution
templates `` `a${x}b` `` still do not parse in the grammar (`convert_template_literal`
handles no-substitution only), so the tagged form is enabled no-substitution —
matching the template bridge's scope. Handles the `javascript-ast` 0.20.0
`TaggedTemplateExpression` variant (PR1, #7495).


## [0.25.0] - 2026-07-02

### Added — CLOC12.160 PR2: bridge the comma operator → `SequenceExpression` (closes gap-161)

`convert_expression_rule` now converts a multi-operand `expression` (the comma
operator `a, b, c`) into an `Expression::SequenceExpression` instead of
declining it to `UnsupportedSyntax` (which dragged the whole file to
WHITESPACE_ONLY). The grammar rule is `expression = assignment_expression {
COMMA assignment_expression }`; `node_children` already drops the `COMMA`
tokens, so the operand list converts directly into the sequence's
`expressions`, in source order. The single-operand path is unchanged (it still
passes the operand through, never wrapping it in a one-element sequence). A
failed operand propagates its error, dropping the file to WHITESPACE_ONLY.

Wherever the grammar's `expression` rule appears — statement position
(`a, b, c;`), a parenthesised group (`x = (a, b)`), a computed-member key
(`obj[a, b]`) — the comma operator now flows through the full SIMPLE/ADVANCED
pipeline end-to-end. 4 new bridge tests + a closurec e2e diff fixture
(`tests/diff/simple-sequence-expression/`) proving `log((a, 1 + 2))` round-trips
the sequence parenthesised while the operand `1 + 2` folds to `3`.

## [0.24.0] - 2026-07-02

### Added — CLOC12.159 PR2: bridge `new X(args)` → `NewExpression` (closes gap-160)

The typed-AST bridge now converts the `new` operator instead of declining it to
`UnsupportedSyntax` (which dragged the whole file to WHITESPACE_ONLY). `new`
appears in **two** grammar productions, and both are handled:

- **argumented** `new X(args)` parses as `member_expression = "new"
  member_expression arguments` — `convert_member_expression` now builds a
  `NewExpression { callee, arguments }` as the base and folds any trailing
  `.NAME` / `[expr]` suffix onto it (so `new X().y` and `new X()[k]` convert
  correctly);
- **bare** `new X` parses as `new_expression = "new" new_expression` —
  `convert_new_expression` builds a `NewExpression` with an **empty** argument
  list (semantically identical to `new X()`).

Arguments reuse `convert_arguments`, so a spread argument (`new X(...a)`) still
declines gracefully (`SpreadElement` is a later slice). `new.target` still
declines (`NewTarget`, Phase 3). 7 new bridge tests (identifier / args / member
callee / bare / member-access-on-new / nested `new new X()` / spread-declines)
plus a closurec e2e diff fixture (`tests/diff/simple-new-expression/`) proving
`log(new Widget(1 + 2))` round-trips the construction while the argument folds
to `3`.

## [0.23.0] - 2026-07-02

### Added — CLOC12.158 PR2: bridge `++` / `--` → `UpdateExpression` (closes gap-159)

The typed-AST bridge now converts update operators instead of declining them
as `UnsupportedSyntax`:
  - `convert_postfix_expression` builds `UpdateExpression { prefix: false }`
    for `a++` / `a--` over the converted operand,
  - the prefix `++`/`--` arm of `convert_unary_expression` builds
    `UpdateExpression { prefix: true }` for `++a` / `--a`,
  - shared helper `update_operator_from_node` maps the `++`/`--` token child.

Both declines predated the `UpdateExpression` node (added in `javascript-ast`
0.17.0, CLOC12.158 PR1); with the node in place the whole SIMPLE/ADVANCED
pipeline now runs on files containing `++`/`--` end-to-end instead of dropping
the whole file to WHITESPACE_ONLY on the bridge decline. **Critical invariant
preserved:** additive-with-unary-sign (`a + +b`, `a - -b`) are separate
`+`/`-` tokens, never a single `++`/`--`, so the `has_token(node, "++")` check
does not false-positive them into an update — pinned by a bridge test. 6 new
bridge tests + a closurec e2e diff fixture (`tests/diff/simple-update-expression/`)
proving `i++` round-trips (never dropped to `i`) while the adjacent `1 + 2`
folds to `3`.

## [0.22.0] - 2026-07-02

### Added — CLOC12.155: bridge `template_literal` → `Expression::TemplateLiteral` (no-substitution only)

The typed-AST bridge now converts **no-substitution template literals**
(`` `abc` ``, `` `` ``) instead of declining them as `UnsupportedSyntax`.
`convert_template_literal` reads the single `TEMPLATE_NO_SUB` token (the grammar
tokenises a substitution-free backtick template as one token whose value is the
whole literal, backticks included), strips the leading and trailing `` ` ``, and
produces a `TemplateLiteral` with one tail `TemplateElement { raw, cooked, tail:
true }` and no `expressions`. This unblocks the full SIMPLE/ADVANCED pipeline for
files containing plain templates end-to-end, rather than dragging the whole file
to WHITESPACE_ONLY.

Conservative scope guard: the converter **declines** (`UnsupportedSyntax` →
WHITESPACE_ONLY, always correct) any `template_literal` node that is not exactly
one `TEMPLATE_NO_SUB` token — i.e. anything with a `${…}` substitution. Those do
not parse in the current grammar anyway (see `CLOC12-gaps.md` §CLOC12.155); when
the grammar learns them, the converter grows a multi-part branch and the AST node
(`quasis` / `expressions`) already models it. *Tagged* templates
(`` tag`…` ``) remain declined (Phase 3). 3 new bridge tests.

## [0.21.0] - 2026-07-02

### Added — CLOC12.152 / gap-155: bridge `arrow_function` → `Expression::ArrowFunctionExpression`

The typed-AST bridge now converts **concise-body arrow functions** (`x => x + 1`,
`(a, b) => a + b`, `() => 1`, `arr.map(x => x)`) instead of declining them as
`UnsupportedSyntax`. `convert_arrow_function` reads `arrow_parameters`
(`NAME` → one identifier param; `( formal_parameters )` → the list; `()` → none)
and the `concise_body`'s expression, producing an `ArrowFunctionExpression` with
an `ArrowBody::Expression`. This unblocks the full SIMPLE pipeline *inside* arrow
bodies end-to-end (e.g. `var f = x => 1 + 2` → `var f = x => 3`) rather than
falling back to WHITESPACE_ONLY.

Two deliberate conservative guards, both to avoid a **miscompile** given current
grammar limitations (see `CLOC12-gaps.md`):

- **Block-bodied arrows aren't reachable (gap-156).** The ECMAScript grammar
  currently rejects a statement block body — `x => { return x; }` fails to parse
  — so the bridge only ever sees a concise `concise_body`. The `ArrowBody::Block`
  path is written and ready for when the grammar is fixed.
- **`() => {}` / object-body arrows DECLINE.** Because the block alternative
  isn't taken, the grammar reads the braces of `() => {}` as an empty *object
  literal* concise body — indistinguishable from a genuine `() => ({})`. Since we
  cannot tell an empty-block arrow (returns `undefined`) from an object-returning
  one, the bridge declines any arrow whose concise body is an `ObjectExpression`,
  falling back to whitespace-only (which re-emits the source unchanged — always
  correct). Only an optimisation is forgone, never correctness.

Async arrows (`async x => x`) parse under the separate `async_arrow_function`
rule and remain declined for now. 5 new bridge tests; a closurec e2e diff
fixture (`tests/diff/simple-arrow-function/`) proves the fold-inside-body win.

## [0.20.0] - 2026-07-01

### Added — CLOC12.149 / gap-153: bridge `function_expression` → `Expression::FunctionExpression`

The typed-AST bridge now converts a `function_expression` grammar node to the
`Expression::FunctionExpression` node (landed in `javascript-ast` 0.14) instead
of declining it as `UnsupportedSyntax`. A function in **value** position — an
IIFE `(function(){})()`, an assigned function `x = function(){}`, a named
recursive `function f(){…f()…}`, or a callback `arr.map(function(x){…})` — now
flows through the full typed pipeline, so closurec optimises *inside* the body
rather than falling back to WHITESPACE_ONLY.

`convert_function_expression` mirrors `convert_function_declaration` with the one
grammatical difference that the **name is optional** (`id: Option<Identifier>`):
`function () {}` is anonymous; a named function expression's name is body-local
(self-reference for recursion), never bound in the enclosing scope. Generators,
async functions, arrow functions, classes, and template literals remain declined
(separate future slices). +4 bridge tests (IIFE callee shape, named body-local
name, anonymous no-id, all four value positions convert).

## [0.19.11] - 2026-06-30

### Changed — opt into the parser's recursion-depth guard (DoS backstop)

`parser` 0.4.1 made `GrammarParser`'s depth guard opt-in (default unlimited),
after a global default cap regressed richer grammars (Wolfram) and preempted
self-guarding frontends (python-to-semantic-ir). closurec, however, feeds
*untrusted* JavaScript to `parse_with_asi` on an ordinary ~2 MiB stack, so
pathologically deep grouping (`((((…))))`, deep unary chains) would otherwise
overflow the native stack — an uncatchable process abort. Both `GrammarParser`
construction sites in `asi::parse_with_asi` (the retry loop and the
budget-exhausted final parse) now opt in with
`.with_max_depth(DEFAULT_MAX_RULE_DEPTH)`. Deep grouping now returns a clean,
recoverable parse error (which closurec degrades to WHITESPACE_ONLY — still
valid output) instead of crashing. Real JS never nests grouping this deep, so
no legitimate program is affected, and all 97 crate tests are unchanged.

(Deep *flat* expressions like `1+1+…+1` overflow a separate downstream
AST-traversal stage, not the parser — tracked as its own follow-up.)

## [0.19.10] - 2026-06-30

### Fixed — function expressions (IIFEs etc.) aborted the compile

A `function` expression in value position made the bridge raise an `Internal`
error, which the CLI treats as a hard failure (`exit 2`, no JS output):

```
(function(){})();     →  bridge internal error: unknown expression rule 'function_expression'
x = function(){};     →  (same)
f(function(){ … });   →  (same)
```

IIFEs, assigned function expressions, and function callbacks are extremely
common, valid JavaScript. Like arrow functions, generator/async function
expressions, and class expressions — all of which already decline gracefully —
a plain function expression is a Phase 2 feature that should DECLINE with
`UnsupportedSyntax` (so the CLI falls back to WHITESPACE_ONLY and still emits
valid output), never abort.

**Cause.** `convert_expression`'s "ES2015+ unsupported" arm listed
`generator_expression`, `async_function_expression`, `arrow_function`,
`class_expression`, … but **omitted** the plain `function_expression`, so it
fell through to the `InternalError` catch-all.

**Fix.** Added `"function_expression"` to that decline list, alongside its
siblings. `(function(){})();` / `x=function(){}` / `f(function(){})` now
round-trip through the WHITESPACE_ONLY fallback (`exit 0`) instead of aborting.

Regression test: `function_expressions_decline_gracefully_not_hard_error`.

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

# Changelog

All notable changes to the `coding-adventures-closure-emitter` crate will be documented in this file.

## [0.53.0] - 2026-07-17

### Fixed — `throw`/`return` drop the space before a punctuation-leading argument

`emit_throw` and `emit_return` emitted an unconditional space between the keyword
and its argument (`throw {a:1}`, `return "x"`). The reference Closure Compiler
emits that space only when the argument would otherwise **fuse** with the
keyword into one token; a punctuation-leading argument needs no separator:

- `throw {a:1}` → `throw{a:1}`   `throw [1,2]` → `throw[1,2]`
- `throw "x"` → `throw"x"`       `throw /re/` → `throw/re/`   `throw !0` → `throw!0`
- **kept** where a word token would fuse: `throw x`, `throw 5`,
  `throw new C`, `throw void x`, `return typeof x`

A new `keyword_needs_space_before` helper decides this. It is **conservative** —
it returns `true` (keep the space, always safe against mis-tokenisation) for
every expression whose leading character is not *provably* punctuation, so it
can never drop a required separator. The punctuation-leading set is exact: object
/ array / string / regex / template literals, plus a unary with a symbol
operator (`!`/`~`/`-`/`+`); the *word* unary operators `void`/`typeof`/`delete`
still take the space.

This was a `SIMPLE`/`ADVANCED`-only divergence (the whitespace-only path already
omitted the space); it also completes the byte-identity of the `new Object()` /
`new Array()` → `{}` / `[]` folds in `throw`/`return` position. Fixes hand-written
`throw {…}` / `throw "…"` too. Byte-identical to the reference compiler at SIMPLE.

## [0.51.0] - 2026-07-17

### Fixed — a `ChainExpression` base of a plain member/call/tagged-template is now parenthesized (optional-chain-scope miscompile)

An optional chain (`a?.b`) used as the object of a **non-optional** member
access, the callee of a plain call, or the tag of a tagged template MUST be
parenthesized — the parens are the chain boundary. Dropping them lets a
following non-optional access join the chain, which is a **semantic** change,
not cosmetic:

- `(a?.b).c` → `a?.b.c`  — `.c` now short-circuits with `?.` (`a?.b.c` returns
  `undefined` when `a` is nullish; `(a?.b).c` throws — different behavior)
- `(a?.b)()` → `a?.b()`, `(a?.[0]).x` → `a?.[0].x`, `(a?.()).x` → `a?.().x`
- `` (a?.b)`x` `` → `` a?.b`x` `` (also outright invalid — a tagged template
  can't tag an optional chain unparenthesized)

A `ChainExpression` is the transparent chain-boundary wrapper, tagged
`PREC_PRIMARY` (its inner spine is a member/call node), so the ordinary
`PREC_PRIMARY` base emit never wrapped it. A new `emit_plain_access_base` helper
wraps a `ChainExpression` base and otherwise keeps the existing `PREC_PRIMARY`
emit; it backs `emit_member`, `emit_call`, and `emit_tagged_template`.

An **optional** access base is deliberately excluded — `(a?.b)?.c` needs no
parens because the chain simply continues (`a?.b?.c`), so
`emit_optional_member`/`emit_optional_call` keep their bare emit. A chain as a
call **argument** (`f(a?.b)`) or in statement position is likewise unwrapped.

Verified byte-identical to the reference Closure Compiler at `SIMPLE` across
member/call/computed/optional-call-result/tagged-template and deep chains
(`(a?.b).c().d`, `((a?.b).c)?.d`). Bug fix; MINOR bump 0.50.0 → 0.51.0.

Not covered (rarer, separate follow-ups): a `ChainExpression` as a `new` callee
(`new (a?.b)`, which also has a `new␣(` spacing quirk) and an **object literal**
at the head of an optional chain that is then member-accessed
(`({}?.x).y` → the chain is now correctly bounded, but Closure additionally
wraps the object literal: `(({})?.x).y`).

## [0.50.0] - 2026-07-16

### Fixed — object literal at the leftmost spine of a statement is now parenthesized (invalid-JS miscompile)

An `ExpressionStatement` whose first emitted token is `{` mis-parses as a
**block**. The direct-expression guard already wrapped a bare `{…}`, but it
missed an object literal reached through the leftmost **emit spine** — a member,
call, assignment target, update, binary/logical/conditional/sequence — so these
printed invalid JS:

- `({}).f` → `{}.f`  (parses `{}` as a block, then `.f` is a syntax error)
- `({}).f()` → `{}.f()`, `({})[0]` → `{}[0]`, `` ({}).f`` `` → `` {}.f`` ``
- `({}).x++` → `{}.x++`, `({}).a = 1` → `{}.a=1`, `({}+"")` → `{}+""`

The reference Closure Compiler wraps the **object literal itself** (`({}).f`),
not the whole statement (`({}.f)`), so a whole-statement wrap would not be
byte-identical. The fix arms a printer flag (`wrap_leftmost_object`) at the start
of an expression statement whose leftmost spine leaf is an object literal;
because every spine construct emits its leftmost child before any token of its
own, the flag lands on exactly that leaf, and `emit_object` consumes it to wrap
only itself. A new `starts_with_object_literal` walks the same leftmost spine to
decide when to arm the flag.

`function`/`class` expressions keep the existing direct-expression wrap: deeper
on the spine they are already parenthesized by the printer's precedence rules
(`(function(){})()` stays single-wrapped), so they are deliberately excluded from
the object-spine walk to avoid double-wrapping. Objects **not** at statement
start are unaffected (`a = {}.f` stays `a={}.f`, `g(({}).f())` stays `g({}.f())`).

Verified byte-identical to the reference Closure Compiler at `SIMPLE`. Bug fix;
MINOR bump 0.49.0 → 0.50.0.

## [0.49.0] - 2026-07-16

### Changed — a no-argument `new` drops its empty `()`; `new` always wraps as a member/call spine

The emitter now prints a zero-argument `NewExpression` without the empty
parentheses — `new C()` → `new C`, `new a.b.C()` → `new a.b.C` — matching the
reference Closure Compiler at `SIMPLE` byte-for-byte. Previously the argument
parens were always printed.

Dropping the `()` makes a `new` a bare `NewExpression` in the grammar, which
binds **looser** than a member access or call: a following `.y` / `[k]` / `(…)`
would otherwise re-associate onto the callee (`new C.y` parses as `new (C.y)`).
To preserve meaning, `expr_prec` now tags every `NewExpression` at a new
`PREC_NEW` (just below `PREC_PRIMARY`), so a member-object or call-callee parent
wraps it. This matches Closure for **both** the argumented and no-argument
forms:

- `new C()` → `new C`
- `new C().foo` → `(new C).foo`
- `new C()()` → `(new C)()`
- `new C().m()` → `(new C).m()`
- `new C(1).foo` → `(new C(1)).foo`
- `new C(1)()` → `(new C(1))()`
- `new new C()` → `new (new C)`
- `new C(1)` (standalone), `typeof new C`, `new C+1` — no wrap (looser parents)

Verified byte-identical against `closure-compiler-v20260712.jar` (`SIMPLE`).
The upstream `CodePrinterTest` `new`-operator port
(`tests/upstream/code_printer_new_test.rs`) previously pinned the always-`()`
spelling; its assertions were corrected to the true jar output (`new X`,
`(new X).y`, …).

Out of scope (separate divergences, unchanged here): the computed-index → dot
normalisation (`(new C)["k"]` → `(new C).k`), sequence-at-statement splitting
(`a,b` → `a;b`), and the space before a parenthesised callee
(`new(f())` vs `new (f())`). MINOR bump 0.48.0 → 0.49.0.

## [0.48.0] - 2026-07-15

### Added — drop the leading zero of a bare fraction in value position (`0.5` → `.5`)

A numeric literal whose magnitude is in `(0, 1)` now emits without its leading
zero in **expression/value position**, matching the reference Closure Compiler's
minification: `0.5` → `.5`, `-0.25` → `-.25`, `0.75` → `.75`. The strip is
applied to the decimal candidate *before* the decimal-vs-exponential
shorter-of comparison, so a stripped decimal can win a tie against the
exponential form (`0.001` → `.001`, not `1E-3`) while a strictly-shorter
exponential still wins (`0.0001` → `1E-4`). A non-zero integer part is untouched
(`10.5`, `3.14`).

The strip is **value-position only**. In object-key position the reference
compiler does not drop the leading zero — it quotes a float key instead
(`{0.5:1}` → `{"0.5":1}`), a separate transform — so `emit_numeric` (value path)
uses the new `format_js_number_value`, while the `PropertyKey::NumericLiteral`
arm keeps the canonical `format_js_number`. Both share `format_js_number_impl`,
parameterized by whether to strip.

## [0.47.0] - 2026-07-14

### Added — emit `name=expr` default parameters — CLOC12.191 PR1

Picks up javascript-ast 0.42.0. New `FunctionParam::AssignmentPattern` emit arm writes the `left`
identifier, `=` (tight in minified mode, spaced `a = 1` in pretty mode via `pretty_ws()`), then the
`right` default expression at `PREC_ASSIGNMENT` — so a looser bare-sequence default parenthesises
(`function f(a=(1,2)){}`) while an ordinary literal prints bare. The arrow single-param paren-elision
guard already restricts to a plain identifier, so a lone default param `(a=1)=>` keeps its parens
(`a=1=>` is invalid JS). Additive; MINOR.

## [0.46.0] - 2026-07-14

### Added — emit `...name` rest parameters — CLOC12.190 PR1

Picks up javascript-ast 0.41.0. New `FunctionParam::RestElement` emit arm writes `...` + the gathered
identifier (`function f(a,...rest){}`). A lone rest param keeps its parens — the arrow concise-param form
now only elides parens for a plain identifier, since `...a=>` is invalid JS. Additive; MINOR.

## [0.45.0] - 2026-07-12

### Added — CLOC12.189 PR1: emit export declarations

`emit_export_named` / `emit_export_default` / `emit_export_all` render the new
`Declaration::Export*` variants in minified form: `export{a,b as c};`,
`export{a}from"y";`, `export const x=1;` (defers to the inner declaration's own
terminator), `export default 1;` (expression) / `export default function f(){}`
(self-terminating), `export*from"y";`, and `export*as ns from"y";`. Six emit
tests.

## [0.44.0] - 2026-07-11

### Added — CLOC12.188 PR1: emit `import` declarations

`emit_import` renders `Declaration::ImportDeclaration` in minified form:
side-effect `import"y";`, default `import x from"y";`, namespace
`import*as ns from"y";` (the `*` punctuator abuts `import` with no space), named
`import{a,b as c}from"y";`, and the default-plus-named combination
`import x,{a}from"y";`. Five emit tests cover each shape.

## [0.43.0] - 2026-07-11

### Added — CLOC12.187 PR1: emit `with (obj) stmt`

New `emit_with` prints `with(` + the object expression + `)` + the body statement
(mirroring `emit_while`), dispatched from a new `TaggedStatement::WithStatement`
arm. Picks up javascript-ast 0.38.0.

## [0.42.0] - 2026-07-11

### Added — CLOC12.183: emit ES2021 logical assignment operators

`assignment_op_str` now maps the three new `AssignmentOperator` variants
(`LogicalAndEq`/`LogicalOrEq`/`NullishCoalescingEq`) to `&&=` / `||=` / `??=`.
Picks up javascript-ast 0.37.0. New `logical_assignment_operators_emit` test.

## [0.41.1] - 2026-07-11

### Added — CLOC12.177 PR3: CodePrinter private-name conformance port

New upstream conformance port `tests/upstream/code_printer_private_name_test.rs`
(the twenty-fourth CodePrinter port), mirroring `CodePrinterTest.java`'s
private-class-member-name printing cases. Isolates the `PrivateName` arm of
`emit_property_key` (CLOC12.177 PR1). 7 active `#[test]`s, 0 `#[ignore]`:
initialized private field (`#x=1;`), bare private field (`#x;`), single-`#`
regression guard, static private field (`static #x=1;`), private method key
(`#m(){}`), and private/public interleave (`#x=1;m(){}`, `x=1;#y=2;`). Inputs are
hand-constructed AST (the emitter is the unit under test; the private-field
bridge is exercised separately in javascript-parser + a closurec e2e fixture, and
building AST directly covers the private-method key shape whose bridge is a later
slice). Test-only; adds a `[[test]]` entry to `Cargo.toml` and a row to
`tests/upstream/ATTRIBUTION.md`. PATCH.

## [0.41.0] - 2026-07-11

### Added — CLOC12.177 PR1: emit private class-member names (`#x`)

`javascript-ast` 0.36.0 added `PropertyKey::PrivateName`. `emit_property_key`
gains a `PrivateName` arm that prints `#` followed by the stored bare name — so a
private field (`class C{#x=1;}`), a bare private field (`class C{#x;}`), a static
private field (`class C{static #x=1;}`), and a private method (`class C{#m(){}}`)
all print correctly. No quote/shorten logic applies (a private name is a hard
token boundary, unlike a string key). 5 emit tests. MINOR.

## [0.40.1] - 2026-07-11

### Added — CLOC12.176 PR3: CodePrinter static-block conformance port

New upstream conformance port `tests/upstream/code_printer_static_block_test.rs`
(the twenty-third CodePrinter port), mirroring `CodePrinterTest.java`'s **static
initialization block** printing cases. Isolates `emit_static_block` + the shared
`emit_class_tail` member loop's `StaticBlock` arm (CLOC12.176 PR1). 9 active
`#[test]`s, 0 `#[ignore]`: empty block (`static{}`, `static` abutting `{`),
statement body (`static{x}`), real initializer (`static{x=1}`), two statements
(`static{x;y}`), brace-termination needing no `;` separator (`static{}m(){}`,
`m(){}static{}`), two blocks back-to-back (`static{}static{}`), and all three
member kinds in source order (`x=1;static{y=2}m(){}`). Inputs are
hand-constructed AST (the emitter is the unit under test; the bridge is exercised
separately in javascript-parser + a closurec e2e fixture). Test-only; adds a
`[[test]]` entry to `Cargo.toml` and a row to `tests/upstream/ATTRIBUTION.md`.
PATCH.

## [0.40.0] - 2026-07-11

### Added — CLOC12.176 PR1: emit static initialization blocks

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). The shared `emit_class_tail` member loop gains a `StaticBlock` arm calling the new
`emit_static_block`, which prints `static{<statements>}` — the `static` keyword
abutting the `{` (a hard token boundary), the body via the shared
`emit_block_statement`, and no trailing `;` (brace-terminated like a method). 4
emit tests (`static{}`, `static{x}`, `static{x;y}`, field/block/method interleave).

## [0.39.1] - 2026-07-11

### Added — CLOC12.175 PR3: CodePrinter class-field conformance port

New `tests/upstream/code_printer_class_field_test.rs` (registered as the
`upstream_code_printer_class_field` `[[test]]`), the third class port — mirroring
upstream Closure `CodePrinterTest`'s class-**field** printing cases. 14 active
`#[test]`s, 0 `#[ignore]`, driving `emit_class_field` + the shared
`emit_class_tail` `Field` arm from HAND-BUILT AST (so it also covers computed /
numeric / string keys and a sequence initializer the grammar/bridge cannot yet
parse): initialized field (`x=1;`), bare field (`y;`, no stray `=`), `static`
prefix (`static z=2;` / `static z;`), computed / literal keys (`[k]=v;` /
`static [k]=v;` / `0=1;` / quoted `"a-b"=1;`), the `PREC_ASSIGNMENT` sequence wrap
(`x=(a,b);`), and field/method interleave (`x=1;m(){}` / `m(){}x=1;` /
`x=1;y;static z=2;`). PATCH — test-only, no emitter change.

## [0.39.0] - 2026-07-11

### Added — CLOC12.175 PR1: emit class fields

`javascript-ast` 0.34.0 added `ClassMember::Field(PropertyDefinition)`. The
shared class-body member loop (`emit_class_tail`, used by both the class
expression and the class declaration) gains a `Field` arm calling the new
`emit_class_field`, which prints `[static ]key[=value];` — the initializer at
`PREC_ASSIGNMENT`, a bare field emitting just `key;`. Six emit tests cover
initialized / bare / static / computed-key fields and a field interleaved with a
method.

## [0.38.1] - 2026-07-11

### Added — CLOC12.174 PR3: CodePrinter class-declaration conformance port

New upstream port `tests/upstream/code_printer_class_declaration_test.rs`
(registered as `[[test]] upstream_code_printer_class_declaration`), the companion
to the class-expression port. Isolates `emit_class_declaration` + the shared
`emit_class_tail` helper from PR1. **20 active `#[test]`s, 0 `#[ignore]`** — the
declaration emits **bare** (no wrapping paren, unlike the expression form's
`(class …);`; no trailing `;`, unlike a `function` declaration), the four
`extends`-operand precedence cases (identifier / member / call bare, conditional
wrapped `extends (a?b:c)`), the member forms (method / params+body / `static` /
`get` / `set` / `constructor` / stacked `static get` / generator `*m` / `async m`
/ computed `[k]`, `[0]`, `[a+b]` / two members back-to-back), and the full shape
`class C extends B{m(){}}`. Inputs are hand-constructed AST, so the port also
covers the generator / async / computed-key / multi-member shapes the grammar
cannot yet parse. Test-only change (no production edit); ATTRIBUTION.md updated.
This closes the CLOC12.174 class-declaration arc.

## [0.38.0] - 2026-07-10

### Added — CLOC12.174 PR1: emit `ClassDeclaration` (`class C [extends S] {…}`)

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant. New
`emit_class_declaration` prints `class <id>[ extends S]{members}`, reusing
`emit_class_member` for each member. The shared `[ extends S]{members}` tail was
factored into a new `emit_class_tail` helper used by **both** `emit_class` (the
expression form) and `emit_class_declaration`.

Three deliberate differences from the class *expression*, each the exact mirror
of the `FunctionDeclaration` vs `FunctionExpression` split:

1. **`id` always prints** (it is non-optional), with a `required_ws()` after
   `class`, like `emit_function_declaration` after `function`.
2. **No precedence wrap / no statement-start parenthesis** — a class expression
   is `PREC_UNARY` and wrapped in statement position (`(class{});`) because a
   leading `class` parses as a declaration, which is precisely what this node
   *is*. So the declaration form has no `expr_prec` entry and is never wrapped.
3. **No trailing `;`** — `emit_function_declaration` appends a normalising `;`
   (gap-030 part B); a class declaration terminates with its `}` alone (upstream
   Closure).

5 hand-constructed AST unit tests: empty (bare, no `;`), heritage, members
(method / static / get / set), `constructor` + computed key, and the full
`class C extends B{m(){}}` shape. Reachable end-to-end once the PR2 bridge
produces the node.

## [0.37.1] - 2026-07-08

### Added — CLOC12.173 PR3: CodePrinter class-expression conformance port

New upstream test port `tests/upstream/code_printer_class_test.rs` (registered
as `[[test]] upstream_code_printer_class`) — the twentieth CodePrinter port,
isolating `emit_class` + `emit_class_member` + the `PREC_UNARY` classification
that landed with `Expression::ClassExpression` (CLOC12.173 PR1). **22 active
`#[test]`s, 0 `#[ignore]`** — the emitter conforms to every covered class shape:

- **statement-start wrap** — a bare `class{}` / `class C{}` in expression-
  statement position is parenthesised (`(class{});`) so a leading `class` does
  not parse as a class *declaration*;
- **surface + heritage** — anonymous/named classes as a call argument print bare
  (`f(class{})`, `f(class C{})`); the `extends` operand prints bare for an
  identifier / member / call heritage (`extends B`, `extends ns.B`,
  `extends mixin(B)`) and wraps for a looser conditional
  (`extends (a?b:c)`);
- **members** — empty method, method with params + body, `static` method,
  `get`/`set` accessors, `constructor`, stacked `static get`, generator `*m`,
  `async m`, computed-key `[k]`, and two members back-to-back with no separator;
- **whole-node precedence** — the class wraps as a member object
  (`(class{}).x`) and a call callee (`(class{})()`) but stays bare under a
  binary parent (`class{}+1`).

Inputs are hand-constructed typed AST (the emitter is the unit under test), so
the port also exercises generator / async / computed-key methods and multi-
member classes the grammar cannot yet parse (bridge conversion is CLOC12.173
PR2, exercised separately in `javascript-parser`). Test-only change — no
production-code edit in this crate; version bumped PATCH.

## [0.37.0] - 2026-07-08

### Added — CLOC12.173 PR1: print `ClassExpression`

`emit_class` prints the `ClassExpression` leaf added in `javascript-ast` 0.32.0:
`class[ id][ extends S]{members}`. Each member goes through `emit_class_member`
(`[static ][get|set ][async ][*]key(params){body}`, computed key `[expr]` via
the shared `emit_property_key`). The method's params + block body reuse a new
`emit_param_list_and_body` helper factored out of `emit_function_expression` (a
class method's `value` is a `FunctionExpression`). The `extends` operand is
emitted at `PREC_PRIMARY` so a LeftHandSide superclass (`extends B`,
`extends ns.B`, `extends mixin(B)`) stays bare while a looser operand wraps
(`extends (a?b:c)`).

**Precedence.** `ClassExpression` tags at `PREC_UNARY` — exactly like
`FunctionExpression` — so it wraps as a member object (`(class{}).x`) or call
callee (`(class{})()`), and a leading `class` in expression-statement position
is wrapped (`(class{});`) since it would otherwise parse as a class
*declaration*. Looser assignment/argument parents leave it bare (`x=class{}`,
`f(class{})`). 8 hand-constructed unit tests (empty, named, heritage,
call/conditional extends operand, method, static, get/set, computed key, plus
the statement-wrap and member-wrap precedence cases).

Emitter-only production change in this crate (the AST node ships from
`javascript-ast`); the bridge is CLOC12.173 PR2 and the CodePrinter conformance
port is PR3.

## [0.36.1] - 2026-07-08

### Added — CLOC12.172 PR3: CodePrinter regex-literal conformance port

New `tests/upstream/code_printer_regexp_test.rs` (a `[[test]]` target,
`upstream_code_printer_regexp`) — the nineteenth CodePrinter port, isolating
`emit_regexp` + the `PREC_PRIMARY` classification that landed with
`Expression::RegExpLiteral` (CLOC12.172 PR1). Mirrors upstream Closure
Compiler's `CodePrinterTest` printing of a `Token.REGEXP` node: the delimiters
and the raw pattern/flags are written **verbatim** — a regex has exactly one
spelling, so unlike a string there is no quote-choice or re-escaping pass. 13
active `#[test]`s, **0 `#[ignore]`**: pattern+flags round-trip (`/ab+c/gi`),
no-flags bare form (`/a.b/`), opaque pattern bodies (groups/alternation
`/(?:a|b)/`, character class `/[a-z]/`, anchors+quantifiers `/^\d+$/`,
backreference `/(a)\1/`, a `/` inside a class `/[/]/`, an escaped delimiter
`/\//`), verbatim flags (full set `/x/dgimsuy`, non-canonical order `/x/ig`
echoed unchanged), and the `PREC_PRIMARY` composition cases where a regex is a
paren-free member base (`/re/.test(a)`, `/re/g.source`) or a bare call argument
(`f(/ab+c/gi,1)`). Inputs are hand-constructed AST; the bridge conversion of a
REGEX token (CLOC12.172 PR2, gap-RegExpAsIdentifier) is exercised separately in
`javascript-parser`. Emitter-tests-only — no production code change.

## [0.36.0] - 2026-07-08

### Added — CLOC12.172 PR1: print `RegExpLiteral` (`/pattern/flags`)

`emit_regexp` prints the regex-literal leaf added in `javascript-ast` 0.31.0:
`/` + `pattern` + `/` + `flags`. The pattern is opaque text (its own `\/`
escapes are already part of it) and no quote-choice/escaping applies — a regex
has exactly one spelling. `RegExpLiteral` is `PREC_PRIMARY` (an atomic primary
that never needs wrapping). 3 unit tests (flags, no-flags, pattern-internal
escapes). Emitter-only; the bridge that builds the node is CLOC12.172 PR2 and
the CodePrinter conformance port is PR3.

## [0.35.1] - 2026-07-08

### Added — CLOC12.171 PR3: CodePrinter optional-chaining `a?.b` / `a?.[k]` / `a?.()` conformance port

Ports upstream `CodePrinterTest`'s optional-chaining printing cases into
`tests/upstream/code_printer_optional_chain_test.rs` (the twenty-fourth
CodePrinter port), isolating `emit_optional_member` / `emit_optional_call` /
`emit_chain` (which landed with CLOC12.171 PR1). 7 hand-constructed-AST cases
cover the two forces that drive the printing: (1) each link keeps its own
optionality — only the `?.`-marked link prints `?.`, so `a?.b.c` prints a PLAIN
`.c` (and the transparent `ChainExpression` wrapper adds no syntax); and (2) the
object/callee binds at `PREC_PRIMARY` — a looser object keeps its parens
(`(a||b)?.c`) and a looser sequence call argument wraps (`a?.((b,c))`). Emitter
code unchanged — test-only conformance port. (CLOC12.171)

## [0.35.0] - 2026-07-08

### Added — CLOC12.171 PR1: print optional chaining `a?.b` / `a?.[k]` / `a?.()`

Teaches the printer the three optional-chain `Expression` variants added in
`javascript-ast` 0.30.0:

- `emit_optional_member` — `a?.b` (dot) / `a?.[k]` (computed). Identical to
  `emit_member` but spells the operator `?.` (`?.` before a name, `?.[` before
  a computed key). The object binds at `PREC_PRIMARY`, so a looser object keeps
  its parens (`(a||b)?.c`).
- `emit_optional_call` — `a?.(args)`. Identical to `emit_call` but spells the
  call operator `?.(`; a looser *sequence* argument still wraps (`a?.((b,c))`).
- `emit_chain` — the `ChainExpression` wrapper is transparent: it emits its
  inner expression with no added syntax, so `a?.b.c` prints with `?.` on only
  the optional link.

All three variants are `PREC_PRIMARY` (they compose like member/call). 5 unit
tests cover dot/computed/call spelling, the optional-then-plain link case
(`a?.b.c`, `a?.b()`), the transparent wrapper, object-precedence parens, and
the sequence-argument wrap. Emitter-only; the bridge that builds these nodes is
CLOC12.171 PR2 and the CodePrinter conformance port is PR3.

## [0.34.1] - 2026-07-07

### Added — CLOC12.170 PR3: CodePrinter object-spread `{...o}` conformance port

Ports upstream `CodePrinterTest`'s object-spread `{...o}` printing cases into
`tests/upstream/code_printer_object_spread_test.rs` (the twenty-third CodePrinter
port), isolating `emit_object_spread` + the `emit_object` member-iteration that
landed with `ObjectMember` (CLOC12.170 PR1). 5 hand-constructed-AST cases
exercise the two forces that drive the printing: (1) the spread argument prints
at `PREC_ASSIGNMENT` inside the braces — an identifier (`{...a}`) and a call
(`{...f()}`) print bare, while a looser *sequence* argument must wrap
(`{...(a,b)}`); and (2) member order is preserved — `{...a, b: 1}` and
`{a: 1, ...b}` print their members in source order (observable, since a later
member overrides an earlier key). Emitter code unchanged — test-only conformance
port. (CLOC12.170)

## [0.34.0] - 2026-07-07

### Added — CLOC12.170: emit object spread `{...o}` (`emit_object_spread`)

`emit_object` now iterates `Vec<ObjectMember>` (the object-literal member type
gained an object-spread arm in `javascript-ast` 0.29.0) and prints an
`ObjectMember::Spread` via the new `emit_object_spread`: the three literal `.`
characters then the `argument` at `PREC_ASSIGNMENT` with no interior space —
identical in shape to the call/array `emit_spread`. The assignment precedence is
the crux: an object-member position is an `AssignmentExpression`, so everything
at or above assignment strength prints bare (`{...a}`, `{...a.b}`, `{...f()}`),
while the one looser form — a **sequence** — must wrap (`{...(a,b)}`) because a
bare `...a,b` would spread only `a` and leave `,b` as a second (invalid) member
slot. 5 hand-constructed-AST unit tests (`{...a}`, `{...a,b:1}`, `{a:1,...b}`,
`{...f()}`, and the sequence wrap `{...(a,b)}`). MINOR because the public
`ObjectExpression` member type changed; no behaviour change for existing inputs.

## [0.33.1] - 2026-07-07

### Added — CLOC12.169 PR3: CodePrinter dynamic `import()` conformance port

Ports upstream `CodePrinterTest`'s dynamic-`import()` printing cases into
`tests/upstream/code_printer_import_expression_test.rs` (the twenty-second
CodePrinter port), isolating `emit_import_expression` + the `PREC_PRIMARY`
classification that landed with `Expression::ImportExpression` (CLOC12.169 PR1).
5 hand-constructed-AST cases exercise the two forces that drive the printing:
(1) the specifier prints at `PREC_ASSIGNMENT` inside the literal parens — a
string (`import("m")`), identifier (`import(x)`), and binary (`import(a+b)`)
specifier all print bare, while a looser *sequence* specifier must wrap
(`import((a,b))`); and (2) the whole node is a `PREC_PRIMARY` leaf, so a
member/call parent composes without extra parens (`import(x).then(f)`). Emitter
code unchanged — test-only conformance port. (CLOC12.169)


## [0.33.0] - 2026-07-07

### Added — CLOC12.169: `emit_import_expression` (dynamic `import(x)`)

Added `emit_import_expression` + the `Expression::ImportExpression` dispatch arm + the `PREC_PRIMARY` classification for the new dynamic `import(specifier)` node (CLOC12.169 PR1). It prints the `import` keyword directly followed by a *literal* parenthesised argument — a call-like primary — with the `source` emitted at `PREC_ASSIGNMENT` (a call-argument level): a looser *sequence* specifier wraps (`import((a,b))`), everything else prints bare (`import("m")`, `import(a+b)`, `import(f())`). As a `PREC_PRIMARY` node the whole import is atomic, so a member/call parent composes without extra parens (`import(x).then(f)`). 5 hand-constructed-AST unit tests. Part of the atomic node PR1 that lands the node + emit + all nine downstream pass arms together. (CLOC12.169)


## [0.32.1] - 2026-07-07

### Added — CLOC12.168 PR3: CodePrinter `import.meta` conformance port

Ports upstream `CodePrinterTest`'s `import.meta` printing cases into
`tests/upstream/code_printer_import_meta_test.rs` (the twenty-first CodePrinter
port), isolating `emit_import_meta` + the `PREC_PRIMARY` classification that
landed with `Expression::ImportMeta` (CLOC12.168 PR1). 6 hand-constructed-AST
cases: the bare `import.meta`, member object (`import.meta.url`), call argument
(`f(import.meta)`), member chain (`import.meta.a.b`), method call
(`import.meta.m()`), and a binary parent (`import.meta+1`) — all print the leaf
bare, confirming it composes paren-free at primary strength (the internal
`.meta` is part of the spelling, not a member access). Registered via an
explicit `[[test]]` entry per CLOC12.01 §3. Test-only; no library change.


## [0.32.0] - 2026-07-07

### Added — CLOC12.168: `emit_import_meta` (`import.meta`)

Added `emit_import_meta` + the `Expression::ImportMeta` dispatch arm + the `PREC_PRIMARY` classification for the new `import.meta` module meta-property (the leaf sibling of `new.target`, CLOC12.168 PR1). `import.meta` prints as its literal eleven-character spelling and binds at primary strength — never wrapped in any parent, never forcing a paren around an operand (it has none); the internal `.meta` is part of the spelling, not a member access. 4 hand-constructed-AST unit tests (`import.meta`, `import.meta.url`, `f(import.meta)`, `import.meta+1`). Part of the atomic node PR1 that lands the node + emit + all nine downstream pass arms together. (CLOC12.168)


## [0.31.1] - 2026-07-07

### Added — CLOC12.167 PR3: CodePrinter `new.target` conformance port

Ports upstream `CodePrinterTest`'s `new.target` printing cases into
`tests/upstream/code_printer_new_target_test.rs` (the twentieth CodePrinter
port), isolating `emit_new_target` + the `PREC_PRIMARY` classification that
landed with `Expression::NewTarget` (CLOC12.167 PR1). 6 hand-constructed-AST
tests pin that `new.target` prints as the bare ten-character spelling and, as a
reserved-word primary, composes without parens in every parent: bare
(`new.target;`), member object (`new.target.x;`), call argument
(`f(new.target);`), member chain (`new.target.a.b;`), method call
(`new.target.m();`), and under a binary parent (`new.target+1;`). Registered as
`[[test]] upstream_code_printer_new_target`; ATTRIBUTION header mirrors the
`super` port. Emitter is driven from hand-constructed AST, so the port does not
depend on the bridge (gap-168 bridge work is CLOC12.167 PR2, exercised in
`javascript-parser`).

## [0.31.0] - 2026-07-04

### Added — CLOC12.167: emit `NewTarget` (`new.target`)

Added `emit_new_target` (prints the literal two-token spelling `new.target` after recording its source-map anchor) and classified `Expression::NewTarget` at `PREC_PRIMARY` in `expr_prec` — a meta-property primary like `this` / `super`. As a primary leaf it never needs wrapping and never forces a paren around an operand; the internal `.` is part of the spelling, not a member access. Four unit tests. (CLOC12.167)


## [0.30.1] - 2026-07-04

### Added — CLOC12.166 PR3: CodePrinter `super` conformance port

Ported the upstream `CodePrinterTest` **super** printing cases into
`tests/upstream/code_printer_super_test.rs` (nineteenth CodePrinter port; test-
only, no library change). 7 active `#[test]`s isolating `emit_super` + the
`PREC_PRIMARY` classification from `Expression::Super` (CLOC12.166 PR1): the
bare keyword `super`, as a member object (`super.x`), call callee (`super()`),
call argument (`f(super)`), member chain (`super.a.b`), method call
(`super.m()`), and left bare under a binary parent (`super+1`). Registered via
an explicit `[[test]]` entry; ATTRIBUTION.md updated. Inputs are hand-
constructed AST, so the port does not depend on the bridge (`super` bridging is
CLOC12.166 PR2). (CLOC12.166 PR3)


## [0.30.0] - 2026-07-04

### Added — CLOC12.166: emit `Super` (`super`)

Added `emit_super` (prints the bare keyword `super` after recording its source-map anchor) and classified `Expression::Super` at `PREC_PRIMARY` in `expr_prec` — the sibling of `this`. As a primary leaf it never needs wrapping and never forces a paren around an operand, so `super.m()`, `super[k]`, `super()` compose paren-free. Five unit tests. (CLOC12.166)


## [0.29.1] - 2026-07-04

### Added — CLOC12.165 PR3: CodePrinter `this` conformance port

Ported the upstream `CodePrinterTest` **this** printing cases into
`tests/upstream/code_printer_this_test.rs` (registered as the
`upstream_code_printer_this` `[[test]]`). 7 active `#[test]`s, 0 `#[ignore]` —
`emit_this` + the `PREC_PRIMARY` classification conform to every covered shape:
the bare keyword `this`, `this` as a member object (`this.x`), call callee
(`this()`), and call argument (`f(this)`), composed in a member chain
(`this.a.b`) and a method call (`this.m()`), and left bare under a binary parent
(`this+1`). Test-only; no emitter behaviour change (the emit logic landed in
CLOC12.165 PR1, closure-emitter 0.29.0).

## [0.29.0] - 2026-07-04

### Added — CLOC12.165: `emit_this` (`this`)

Added `emit_this` + the `Expression::ThisExpression` dispatch arm and its `PREC_PRIMARY` classification. `this` prints as the bare four-character keyword — a primary that never needs wrapping in any parent (`this.x`, `this()`, `f(this)`, `this+1` all print bare) and never forces a paren around an operand. Reachable via hand-constructed AST today; the bridge conversion of `this` (gap-166) lands as CLOC12.165 PR2. (CLOC12.165)


## [0.28.1] - 2026-07-04

### Added — CLOC12.164 PR3: CodePrinter await conformance port

New upstream-cited test file `tests/upstream/code_printer_await_test.rs`
(registered as the `upstream_code_printer_await` `[[test]]` target) porting the
Closure Compiler `CodePrinterTest` `await` printing cases against `emit_await`.
9 active `#[test]`s, **0 `#[ignore]`** — `await` printed like the word-unaries
typeof/void/delete: the surface `await p` (mandatory keyword↔operand space),
tighter operands bare (`await a.b`, `await f()`), a looser binary operand
wrapped (`await (a+b)`), and the whole-node `PREC_UNARY` precedence cases —
bare under a binary parent (`await p+1`), wrapped by member/call parents
(`(await p).x`, `(await f)()`), wrapped as an exponentiation base
(`(await p)**2`, since a bare `await p**2` is a syntax error), and nested
(`await await p`). Inputs are hand-constructed AST; the bridge conversion of
await (gap-165) is deferred — the current grammar treats `await` inside an
async body as a plain identifier, so it does not yet parse. Test-only — no
library behaviour change.

## [0.28.0] - 2026-07-04

### Added — CLOC12.164: emit `AwaitExpression` (`await x`)

Emit `Expression::AwaitExpression` via new `emit_await` — `await ` + operand,
printed like the word-unaries typeof/void/delete: a mandatory keyword↔operand
space, operand at `PREC_UNARY` so a looser binary operand wraps (`await (a+b)`)
while member/call operands print bare (`await a.b`, `await f()`). `expr_prec`
tags await at `PREC_UNARY` so it binds tighter than binary parents
(`await a+b`) but member/call/new parents wrap it (`(await p).x`, `(await f)()`).
8 new unit tests. (CLOC12.164)


## [0.27.1] - 2026-07-04

### Added — CLOC12.163 PR3: CodePrinter yield conformance port

New upstream-cited test file `tests/upstream/code_printer_yield_test.rs`
(registered as the `upstream_code_printer_yield` `[[test]]` target) porting the
Closure Compiler `CodePrinterTest` generator `yield` / `yield*` printing cases
against `emit_yield`. 9 active `#[test]`s, **0 `#[ignore]`** — the three surface
forms (bare `yield`, non-delegate `yield a` with its mandatory keyword↔operand
space, delegate `yield*xs` with no space plus the member-operand `yield*a.b`),
the operand-precedence cases (conditional `yield a?b:c` and assignment
`yield a=b` stay bare, sequence `yield (a,b)` wraps), and the whole-node
precedence cases where a tighter parent wraps the yield (`(yield a)+1`,
`(yield a).b`). Inputs are hand-constructed AST; the bridge conversion of yield
(CLOC12.163 PR2, gap-164) is exercised separately in `javascript-parser` once
generator bodies parse. Test-only — no library behaviour change.

## [0.27.0] - 2026-07-03

### Added — CLOC12.163: emit `YieldExpression` (`yield` / `yield x` / `yield* xs`)

New `emit_yield` prints `Expression::YieldExpression`: a bare `yield`, a value
yield `yield x` (mandatory keyword↔argument space), and a delegating
`yield*xs` (no space after `*`). The argument is printed at `PREC_ASSIGNMENT`,
so a sequence argument wraps (`yield (a,b)`) while a conditional or assignment
argument prints bare. `expr_prec` tags a yield at `PREC_ASSIGNMENT`, so a
tighter parent wraps the whole yield (`(yield a)+1`, `(yield a).b`). 9 new unit
tests.


## [0.26.1] - 2026-07-03

### Added — CLOC12.162 PR3: CodePrinter spread conformance port

New upstream-cited test file `tests/upstream/code_printer_spread_test.rs`
(registered as the `upstream_code_printer_spread` `[[test]]` target) porting the
Closure Compiler `CodePrinterTest` spread (`...arg`) printing cases against
`emit_spread`. 10 active `#[test]`s, **0 `#[ignore]`** — spread call arguments
(sole `f(...a)`, interleaved `f(a,...b,c)`, two adjacent `f(...a,...b)`, member
argument `f(...a.b)`), array elements (sole `[...a]`, interleaved `[1,...a,2]`),
`new` arguments (`new F(...a)`, interleaved `new F(a,...b)`), and the
`PREC_ASSIGNMENT` precedence cases (sequence argument wraps `f(...(a,b))`,
conditional argument stays bare `f(...a?b:c)`). Inputs are hand-constructed
typed AST; the bridge conversion of the spread form is exercised separately
(CLOC12.162 PR2, gap-163, in `javascript-parser`). ATTRIBUTION.md updated.

## [0.26.0] - 2026-07-03

### Added — CLOC12.162: emit `SpreadElement` (`...arg`)

New `emit_spread` prints `...` immediately followed by the argument at
`PREC_ASSIGNMENT` (no interior space). The argument grammar is an
`AssignmentExpression`, so everything at or above assignment strength prints
bare (`...a`, `...a.b`, `...f()`, `...a?b:c`, `...a=b`) while a looser
**sequence** argument is wrapped — `...(a,b)` — because a bare `...a,b` would
spread only `a` and leave `,b` as a second list slot (a miscompile). The node
tags at `PREC_ASSIGNMENT`, matching the assignment-position list slots it lives
in (`f(...a)`, `[...a]`), so it is never spuriously parenthesised there. 6 new
unit tests (sole/interleaved call arg, array element, `new` arg, sequence-arg
wrap, conditional-arg bare). This is CLOC12.162 PR1 (the atomic node); the emit
is exercised by hand-built AST, and the bridge-enable is PR2.


## [0.25.1] - 2026-07-03

### Added — CLOC12.161 PR3: CodePrinter tagged-template conformance port

New upstream-cited test file `tests/upstream/code_printer_tagged_template_test.rs`
(registered as the `upstream_code_printer_tagged_template` `[[test]]` target)
porting the Closure Compiler `CodePrinterTest` tagged-template (`` tag`...` ``)
printing cases against `emit_tagged_template`. 9 active `#[test]`s, **0
`#[ignore]`** — no-substitution tags (`` tag`abc` ``, empty `` tag`` ``),
member-chain tags (`` a.b`x` ``, `` a.b.c`x` ``), `${…}` substitution tags
(`` String.raw`a${x}b` ``, leading `${x}b`, adjacent `${x}${y}`), and the
`PREC_PRIMARY` precedence cases (member-on-tagged `` a`x`.length `` stays
paren-free, a looser sequence tag wraps `` (a,b)`x` ``). Inputs are
hand-constructed typed AST; the bridge conversion of the tagged-template form
(PR2, gap-162) is exercised separately in `javascript-parser`. Test-only change
— no library behaviour change. This completes the CLOC12.161 arc.


## [0.25.0] - 2026-07-02

### Added — CLOC12.161: emit `Expression::TaggedTemplateExpression`

New `emit_tagged_template` method wired into the expression dispatch. A tagged
template `` tag`abc${x}` `` emits the `tag` callee at `PREC_PRIMARY` (so a
looser tag such as a sequence wraps — `` (a,b)`x` ``) followed directly by the
template literal via the existing `emit_template_literal` (no separator seam).
`expr_prec` classifies the node as `PREC_PRIMARY`, so a member access on a
tagged template stays paren-free (`` a`x`.length ``). 5 new inline
`emit_tagged_template` tests. Handles the new `javascript-ast` 0.20.0 variant.


## [0.24.1] - 2026-07-02

### Added — CLOC12.160 PR3: CodePrinter comma-operator conformance port

New upstream-cited test file `tests/upstream/code_printer_sequence_test.rs`
(registered as the `upstream_code_printer_sequence` `[[test]]` target) porting
the Closure Compiler `CodePrinterTest` comma-operator (`a, b, c`) printing cases
against `emit_sequence`. 9 active `#[test]`s, **0 `#[ignore]`** — the two bare
positions (statement `a,b,c`, computed-member key `a[b,c]`) and the wrapped
positions (sole/multi call argument `f((a,b),c)`, array element `[(a,b),c]`,
assignment RHS `x=(a,b)`, conditional branch `x?(a,b):c`, unary operand
`!(a,b)`). Inputs are hand-constructed typed AST; the bridge conversion of the
comma operator (PR2, gap-161) is exercised separately in `javascript-parser`.
Test-only change — no library behaviour change. This completes the CLOC12.160
`SequenceExpression` three-PR arc (node+emit → bridge → conformance).

## [0.24.0] - 2026-07-02

### Added — CLOC12.160: emit `SequenceExpression` (the comma operator)

`emit_sequence` prints the new `Expression::SequenceExpression` node —
comma-joined operands (`a,b,c`), each at `PREC_ASSIGNMENT`. The sequence
itself is classified at the new lowest precedence `PREC_SEQUENCE` (0), so a
parent that emits its child above statement level wraps the whole sequence.

To make that wrapping correct, the four **assignment-position** emit sites now
emit their child at `PREC_ASSIGNMENT` instead of the parent-precedence-0
sentinel: call arguments, `new` arguments, array elements, and the assignment
RHS. A sequence there wraps (`f((a,b),c)`, `[(a,b),c]`, `x=(a,b)`) — never the
arity-changing `f(a,b,c)` / `[a,b,c]` or the mis-parsed `x=a,b`. This is a
**no-op for every existing node** (all bind at `PREC_ASSIGNMENT` or higher, so
nothing new wraps) — the full closurec suite is unchanged. A computed-member
key keeps parent-precedence 0, so `a[b,c]` prints bare (a sequence is legal
unparenthesised inside `[ ]`). 8 new unit tests. No bridge output yet (PR2).


## [0.23.1] - 2026-07-02

### Added — CLOC12.159 PR3: CodePrinter new-operator conformance port

New upstream-cited test file `tests/upstream/code_printer_new_test.rs`
(registered as the `upstream_code_printer_new` `[[test]]` target) porting the
Closure Compiler `CodePrinterTest` `new`-operator (`new Ctor(args)`) printing
cases against `emit_new`. 10 active `#[test]`s, **0 `#[ignore]`** — identifier
and member-chain callees (`new X()`, `new a.b.c()`), argument lists
(`new X(a,b)`, `new X(a.b)`), the callee-with-call wraps (`new (f())()`,
`new (a.b().c)()`), the `PREC_PRIMARY` member-object cases (`new X(a).y`,
`new X().y`), nested `new new X()()`, and a call on a `new` member
(`new X().m()`). Inputs are hand-constructed typed AST; the bridge conversion
of `new` (PR2, gap-160) is exercised separately in `javascript-parser`.
Test-only change — no library behaviour change. This completes the CLOC12.159
`NewExpression` three-PR arc (node+emit → bridge → conformance).

## [0.23.0] - 2026-07-02

### Added — CLOC12.159: emit `NewExpression` (`new X(args)`)

`emit_new` prints the new `Expression::NewExpression` node: `new`, the callee,
then the argument parens (always printed, so a no-argument node is emitted
canonically as `new X()`). Two seams are handled: (1) a keyword-space
separates `new` from an identifier/member callee (`newX` would fuse), spent
only when the callee is not already parenthesised; (2) a callee whose member
spine bottoms out in a **call** is wrapped (`new (f())()`, `new (a.b().c)()`),
or the appended `(args)` would bind to the inner call (`new f()()` reparses as
`(new f())()`). Classified at `PREC_PRIMARY` (the always-argumented form binds
at member/call strength). 8 new unit tests. No behaviour change for existing
inputs — the bridge does not yet produce `new` nodes (PR2).


## [0.22.1] - 2026-07-02

### Added — CLOC12.158 PR3: CodePrinter update-operator conformance port

New upstream-cited test file `tests/upstream/code_printer_update_test.rs`
(registered as the `upstream_code_printer_update` `[[test]]` target) porting
the Closure Compiler `CodePrinterTest` update-operator (`++` / `--`) printing
cases against `emit_update`. 14 active `#[test]`s, **0 `#[ignore]`** — prefix
and postfix increment/decrement, a member operand (`a.b++`), bare printing
under `!` / `typeof` (`!x++`, `typeof x++`), the `PREC_UNARY` precedence wraps
(`(x++).y` member-object, `(++x)**2` exponent-base), and the token-fusion seams
(`a- --b`, `a+ ++b`, `x++ +y`, `x-- -y`, plus the non-fusing `x++*y`). Inputs
are hand-constructed typed AST; the bridge conversion of `++`/`--` (PR2,
gap-159) is exercised separately in `javascript-parser`. Test-only change — no
library behaviour change.

## [0.22.0] - 2026-07-02

### Added — CLOC12.158: emit `UpdateExpression` (`++` / `--`)

`emit_update` prints the new `Expression::UpdateExpression` node —
`++x` / `--x` (prefix: operator then operand) and `x++` / `x--` (postfix:
operand then operator). Classified at `PREC_UNARY`, which is loose enough to
wrap an exponentiation base (`(++x)**2` — a bare `++x**2` is a syntax error)
and a member/call object (`(x++).y`), yet tight enough to print bare under a
`!`/`typeof` parent (`!x++`, `typeof x++`). **Token-fusion seams** are handled
without a new guard: `arg_starts_with_sign` now reports a *prefix* update's
leading `+`/`-`, so the binary/unary emitters space the seam (`a- --b`, never
`a---b` = `(a--)-b`; `a+ ++b`, never `a+++b` = `(a++)+b`), and a *postfix*
update ending in `+`/`-` is spaced by the binary emitter's existing
output-tail check (`x++ +y`). 8 new unit tests. No behaviour change for
existing inputs (the bridge does not yet produce update nodes — PR2).

## [0.21.3] - 2026-07-02

### Fixed — CLOC12.157 (gap-158): newline-aware template quasi emit

`emit_template_element` now prints a template quasi whose `raw` text carries a
**literal newline** — a multiline template `` `a⏎b` `` round-trips its interior
line break byte-for-byte. Previously the quasi `raw` went straight to
`write_str`, which `debug_assert!`s the run is newline-free (every `'\n'` must
route through `newline()` so the source-map line/column bookkeeping stays
correct); a multiline template therefore panicked the emitter worker. The fix
splits `raw` on `'\n'`, writing each line segment via `write_str` and emitting a
real `newline()` between segments, so `line` advances and `col` resets exactly
as for any other newline. Single-line quasis are unaffected (one segment, no
break). Other line-terminator bytes a raw may carry — a lone `'\r'` in a `\r\n`
pair, and `U+2028` / `U+2029` — are written verbatim (bytes round-trip; only
their column bookkeeping is approximate). Un-ignores the conformance port's
`raw_preserves_internal_newline` (now 19 active `#[test]`s, **0 `#[ignore]`**)
and adds three inline emitter tests. Resolves **gap-158**.

## [0.21.2] - 2026-07-02

### Test — CLOC12.156: CodePrinter template-literal conformance port

New upstream port `tests/upstream/code_printer_template_test.rs` (from
`CodePrinterTest.java`) isolating `emit_template_literal` /
`emit_template_element` + the `PREC_PRIMARY` classification that landed with
`Expression::TemplateLiteral` (CLOC12.154). **17 active `#[test]`s + 1
`#[ignore]`.** Coverage: no-substitution templates (empty, escaped backtick
`` `hel\`lo` ``, escaped `` \${ ``); a template as an unwrapped member-access
object (`` `hello`.length ``) and as an unwrapped `+` / `==` operand
(`` `hello`+world ``) — it is primary; and `${…}` substitution templates
(single `` `${world}` ``, adjacent `` `${a}${b}` ``, text-interleaved
`` `hello ${world}` ``, low-precedence body `` `${a+b}` ``, member-access body
`` `${hello.length}` ``). Inputs are hand-constructed AST so the port exercises
`${…}` substitution templates the grammar tokenises only as no-substitution
today (gap-157) — the emitter already prints them; the parser can't yet feed
them. Test-only; no `src/` change. The one ignored case (a quasi carrying a
*literal* embedded newline) surfaces **gap-158** — `emit_template_element`
routes `raw` through `write_str`, which forbids embedded `'\n'`. *Tagged*
templates are intentionally not ported (no `TaggedTemplateExpression` AST node).

## [0.21.1] - 2026-07-02

### Test — CLOC12.153: CodePrinter arrow-function conformance port

New upstream port `tests/upstream/code_printer_arrow_test.rs` (from
`CodePrinterTest.java`) isolating `emit_arrow_function_expression` + the
`PREC_ASSIGNMENT` precedence wrap that landed with
`Expression::ArrowFunctionExpression` (CLOC12.151). **12 active `#[test]`s, no
`#[ignore]`, no new gaps** — the emitter conforms to every covered arrow shape:
single-param param-paren drop (`x=>x`), zero/multi param (`()=>1`, `(a,b)=>a`),
object-literal-body wrap (`()=>({})`), concise vs block body (`x=>{return x}`),
IIFE / member-object wrap (`(()=>{})()`, `(()=>{}).x`), un-parenthesised
call-argument (`g(x=>x)`), and the async prefix (`async x=>x`, `async()=>{}`).
Inputs are hand-constructed AST, so the port also covers block-bodied arrows the
grammar can't yet parse (gap-156). Test-only; no `src/` change.

## [0.21.0] - 2026-07-02

### Added — CLOC12.154: emit `TemplateLiteral` (backtick template strings)

`emit_template_literal` prints the new `Expression::TemplateLiteral` node —
interleaving the `quasis` (each from its verbatim `raw` text) with the `${…}`
expressions: `` `q0${e0}q1${e1}…qN` ``. The `${` / `}` delimiters make each
inserted expression a full-expression context, so it emits at the loosest
precedence with no wrapping. A template tags at `PREC_PRIMARY` (a primary
expression / member-call base, `` `x`.length ``), so no parent ever wraps it and
it needs no statement-start guard. 5 new unit tests. Part of the atomic
`TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0).

## [0.20.0] - 2026-07-02

### Added — CLOC12.151: emit `ArrowFunctionExpression` (the `=>` form)

`emit_arrow_function_expression` prints the new `Expression::ArrowFunctionExpression`
node. Shape rules:

- **Param parens dropped for a single plain identifier** — `x=>x`; zero
  (`()=>`) and two-or-more (`(a,b)=>`) keep them.
- **Dual body** — a block body emits like a function body (`x=>{return x}`,
  with the last statement's trailing `;` dropped in compact mode); a concise
  body emits the bare expression at `PREC_ASSIGNMENT` (`x=>x+1`).
- **Object-literal concise bodies are wrapped** — `()=>({a:1})`, so the leading
  `{` isn't read as a block. (The deeper leftmost-`{` case, e.g. `()=>({}).x`,
  is not yet wrapped — tracked in CLOC12-gaps.)
- **Precedence** — an arrow is tagged `PREC_ASSIGNMENT`, so a call callee /
  member object parent wraps it (`(()=>{})()`, `(()=>{}).x`), while an
  assignment RHS or conditional branch leaves it bare. Unlike a function
  expression it needs no statement-start wrap (`x=>x;` is a valid statement).
- **`async`** — prints an `async` prefix, with a separating space only before
  an unparenthesised identifier param (`async x=>x`, but `async()=>{}`).

12 new unit tests. Part of the atomic `ArrowFunctionExpression` enum-variant
rollout (javascript-ast 0.15.0).

### Changed — `EMIT_STACK_SIZE` 64 MiB → 128 MiB

The arrow arm widens `emit_expression_inner`'s per-frame footprint, and aarch64
(Apple-silicon CI) lays out larger frames than x86-64. Doubling the emit-worker
stack keeps a 2× cushion above the ~20 000-deep adversarial input the deep-emit
DoS regression exercises, so a modest future frame increase can't re-break it
(lazy pages → no cost for real code). Mirrors the same bump in
`closure-pass-constant-fold`'s `FOLD_STACK_SIZE`.

## [0.19.1] - 2026-07-01

### Test — CodePrinter function-expression port (#88, CLOC12.150)

New upstream conformance port `tests/upstream/code_printer_function_test.rs`
(from `CodePrinterTest.java`'s `testFunctionExpression*` / IIFE /
`function`-at-statement-start cases), now unblocked because
`Expression::FunctionExpression` is emitted end-to-end (CLOC12.149 + gap-153).
**12 active `#[test]`s, no `#[ignore]`** — pins that `emit_function_expression`
conforms to every covered shape: anonymous/named (`(function(){})` /
`(function f(){})`), params, a return body (no trailing `;` after `}`), the
three parenthesised contexts (statement-start, IIFE callee `(function(){})()`,
member object `(function(){}).x`), the un-parenthesised call-argument context
(`g(function(){})`), and the `function*` / `async function` prefixes.
Test-only; no `src/` change.

## [0.19.0] - 2026-07-01

### Added — CLOC12.149: emit `FunctionExpression`

Adds `emit_function_expression` (the expression sibling of
`emit_function_declaration`: optional name, and no trailing `;` since a
function *value* is not a declaration). Tags `FunctionExpression` below
`PREC_PRIMARY` in `expr_prec` so a call/member parent wraps it
(`(function(){})()`, `(function(){}).x`), and wraps a leading one in
`emit_expression_statement` (a statement starting with `function` would
otherwise parse as a declaration). +5 tests.

## [0.18.16] - 2026-07-01

### Test — CodePrinter object-literal port (#88, CLOC12.147)

The fifth CodePrinter port into `closure-emitter`. New file
`tests/upstream/code_printer_object_literal_test.rs` (registered as the
`upstream_code_printer_object_literal` test target) reshapes upstream
`CodePrinterTest`'s object-literal cases (`testObjectLit*` and the key-quoting
behaviors) onto our AST surface, pinning the `emit_object` /
`emit_property` / `emit_property_key` output in the default (minified) mode.

- **13 active `#[test]`s pass on the first run** (no new emitter bug): the
  empty object, single/multiple identifier-keyed properties (comma-separated,
  no interior whitespace), string-key quote-stripping (`{"abc":1}` → `{abc:1}`,
  reserved words bared, non-identifier and numeric-looking keys kept quoted,
  the `"__proto__"` exception kept quoted), numeric-literal keys, computed keys
  (`{[a]:1}`), shorthand (`{a}`), a nested object, and a string-valued
  property. Every case shows the object parenthesized at statement start
  (`({…});`).
- **No `#[ignore]` placeholders.** Getters/setters (`{get a(){}}`) and method
  shorthand (`{m(){}}`) are intentionally out of scope — their values are
  `FunctionExpression`s, which the Phase-1 emitter does not yet print and the
  AST cannot fully represent; they join when function-expression emission
  lands.

This is a **test-only** change: no `src/` file is touched, so there is no
ripple into downstream consumers. Bumps the crate 0.18.15 → 0.18.16.

## [0.18.15] - 2026-07-01

### Test — CodePrinter ASCII-only escape port (#88, CLOC12.144)

New upstream port `tests/upstream/code_printer_ascii_escape_test.rs` (registered
as the `upstream_code_printer_ascii_escape` test target), covering the
`--output_charset=US-ASCII` mode from `CodePrinterTest.testUnicode` — our
`EmitOptions { ascii_only: true }` path (`escape_ascii_only`). The sibling
`code_printer_string_escape_test.rs` pins the DEFAULT (non-ASCII passthrough)
mode; this pins the distinct ASCII-only branch of `emit_string`.

- **7 active `#[test]`s** (all pass on the first run — no new emitter bug):
  printable ASCII verbatim; a Latin-1 accent (`é` → `é`); a BMP CJK
  ideograph (`中` → `中`); an astral code point using the braced form
  (`💩` → `{1F4A9}`); a control char (`U+0007` → ``); the named
  `\n`/`\t` escapes still applying; and the `ascii_only`-always-double-quotes
  rule (a value of two `"` prints `"\"\""`, not single-quoted).

No library code changed — this release is test coverage plus docs.

## [0.18.14] - 2026-07-01

### Test — CodePrinter string-escape port (#88, CLOC12.143)

New upstream port `tests/upstream/code_printer_string_escape_test.rs` (registered
as the `upstream_code_printer_string_escape` test target), extending the
CodePrinter coverage beyond the quote-choice case already active in
`code_printer_test.rs` (gap-026). It pins the **escape sequences** `emit_string`
emits, in both the default double-quote path and the single-quote path that
quote-choice selects:

- **7 active `#[test]`s** (all pass on the first run — no new emitter bug):
  backslash doubling (`a\b` → `"a\\b"`), the `\n`/`\r`/`\t` short escapes, an
  "other" control char as upper-case `\uXXXX` (`U+0007` → ``), the
  U+2028/U+2029 line-terminator escapes, printable non-ASCII left verbatim
  (`café` stays `café` in the default non-`ascii_only` mode), and two
  single-quote-path cases (backslash still doubles; the active `'` is escaped
  while `"` stays bare).

No library code changed — this release is test coverage plus docs.

## [0.18.13] - 2026-07-01

### Test — CodePrinter number-formatting port (#88, CLOC12.142)

New upstream port `tests/upstream/code_printer_numbers_test.rs` (registered as
the `upstream_code_printer_numbers` test target), extending the CodePrinter
coverage beyond the core shortest-form cases already active in
`code_printer_test.rs`. It pins the **exponential-vs-decimal cut-over** that
`format_js_number` performs — it keeps whichever spelling is strictly shorter,
breaking ties toward decimal:

- **6 active `#[test]`s** covering the cut-over both ways: `1e18` → `1E18`,
  `1e100` → `1E100`, `2.5e10` → `2.5E10`, `123456789` stays decimal (its
  `1.23456789E8` is longer), `1e-7` → `1E-7`, `1234.5` stays decimal.
- **2 `#[ignore = "blocked on gap-133"]` placeholders** for the leading-zero
  drop upstream applies to bare fractions (`0.25` → `.25`, `0.125` → `.125`).
  Our emitter's `format_js_number` currently keeps the `0`; the placeholders
  document the intended upstream byte output. This is the *emitter* (AST →
  string) path — the separate source-preserving byte-identity path already
  elides the leading zero (gap-107 / gap-113).

No library code changed — this release is test coverage plus docs. Every active
assertion matched the emitter's current output on the first run (no new bug).

## [0.18.12] - 2026-06-30

### Test — activate stale CodePrinter conformance placeholders (#88, CLOC12.138)

Three `#[ignore]`d placeholders in `tests/upstream/code_printer_test.rs` were
left as documentation stubs when their underlying emitter features shipped
(gap-025 number shortest-form in CLOC12.12, gap-026 quote-choice in CLOC12.11,
gap-027 precedence-parens in CLOC12.10). This turns them into **active**
byte-equal conformance tests against the emitter's actual output — no
production code change:

- `test_number_formatting_shortest_form` — `1E9`/`1E6`/`1E21` exponential
  collapse, `100`→decimal tie, `0.5`, and `-0` sign preservation.
- `test_string_quote_choice_minimises_escapes` — single-quote when the value has
  more `"` than `'` (`she said "hi"` → `'she said "hi"'`), double otherwise.
- `test_operator_precedence_inserts_inner_parens` — `a*(b+c)`, `(a+b)*c`, and
  `a+b*c` (no parens where `*` already binds tighter).

Also opened **gap-133** (CLOC12-gaps.md): the number formatter keeps `0.5`
where upstream drops the leading zero to `.5` — a conservative miss (not a
miscompile), tracked for a small `format_js_number` follow-up.

Test-only + docs; crate version 0.18.11 → 0.18.12. Full emitter suite green
(96 lib + 9 code_printer port, 3 unrelated placeholders still ignored).

## [0.18.11] - 2026-06-30

### Fixed — deep operator chains no longer overflow the native stack (DoS)

`emit_binary`/`emit_logical` recurse on the left operand once per operator, so
a deeply left-nested chain — the shape the bridge builds for flat source like
`1+1+…+1` (tens of thousands of terms) — recursed once per operator and, past a
few thousand levels, overflowed the caller's ordinary ~2 MiB stack. That is an
*uncatchable* abort, and closurec feeds this emitter *untrusted* JS, so it must
not be crashable by pathological input.

`emit` now runs the recursive emission on a 64 MiB worker thread
(`EMIT_STACK_SIZE`) via `std::thread::scope` (borrows the program without
`'static`; the shallow source-map serialisation finishes on the caller thread).
Emission is **byte-identical** to before — only the stack size differs — so
every existing fixture is unchanged. Regression test builds a 20 000-deep `+`
chain and asserts it emits `1+1+…+1;` without crashing.

(Very deep ASTs can still stress *other* recursive stages — notably the AST's
own recursive `Drop` — which closurec absorbs on its 8 MiB main thread for the
inputs seen here; full pipeline-level hardening is tracked separately.)

## [0.18.10] - 2026-06-30

### Changed — drop needless parens around assignments in conditional branches

`emit_conditional` parenthesised an assignment in the consequent or alternate
of a `? :`:

```
a ? b = 1 : c = 2   →  a?(b=1):(c=2)     (now: a?b=1:c=2)
cond ? x += 1 : y   →  cond?(x+=1):y     (now: cond?x+=1:y)
```

The ECMAScript grammar is
`ConditionalExpression : ShortCircuitExpression ? AssignmentExpression : AssignmentExpression`,
so both branches are full `AssignmentExpression`s and need no parentheses —
the `?`/`:` punctuation delimits them. The consequent was being emitted at
`PREC_CONDITIONAL + 1` and the alternate at `PREC_CONDITIONAL`, both higher
than assignment precedence, so the wrapper added parens. Both branches now
emit at `PREC_ASSIGNMENT` (the precedence floor of the expression set — there
is no `SequenceExpression` in the AST), so an assignment or nested conditional
branch is never wrapped (`a?b?c:d:e` instead of `a?(b?c:d):e`).

**Soundness.** The TEST is unchanged — it is a `ShortCircuitExpression`, which
does NOT include assignment or conditional, so a test that is itself an
assignment keeps its required parens: `(a=1)?b:c` (without them, `a=1?b:c`
parses as `a=(1?b:c)`).

Tests: `conditional_branches_do_not_parenthesize_assignments`,
`conditional_test_assignment_stays_parenthesized`.

## [0.18.9] - 2026-06-30

### Added — Closure-style boolean shorthand: `true` → `!0`, `false` → `!1`

Boolean literals now minify to their negation-of-digit form, matching the
Closure Compiler: `true` (4 chars) → `!0` (2), `false` (5) → `!1` (2).
`!0 === true` and `!1 === false` in every context (`!` coerces to boolean and
negates), so the rewrite is value-exact.

**Precedence (soundness).** `!0` / `!1` are `UnaryExpression`s, which bind
*looser* than member access, call, `new`, and tagged templates — so a naive
`true.x` → `!0.x` would reparse as `!(0.x)`, a miscompile. To prevent that,
`expr_prec` now tags `BooleanLiteral` at `PREC_UNARY` (exactly as the `void 0`
`UndefinedLiteral` is already handled), so the existing precedence wrapper in
`emit_expression_inner` inserts parentheses automatically where required:

```
true.toString()  →  (!0).toString()
true()           →  (!0)()
x = true         →  x=!0           (no parens needed)
[true, false]    →  [!0,!1]
a && true        →  a&&!0
true == 1        →  !0==1          (unary binds tighter than ==)
```

Tests: `boolean_literals_minify_to_bang_zero_and_bang_one`,
`boolean_as_member_object_is_parenthesized`, `boolean_in_binary_needs_no_parens`
(and `unary_not_no_space` updated: `!true` → `!!0`).

## [0.18.8] - 2026-06-30

### Fixed — large integer literals saturated to `i64::MAX`/`MIN` (miscompile)

An integral numeric literal whose magnitude was at or above `2^63` but below
`1e21` was emitted as the wrong number:

```
console.log(12345678901234567890)  →  console.log(9223372036854775807)
a = 18446744073709551615           →  a = 9223372036854775807
(negative counterparts)            →  -9223372036854775808
```

`format_js_number` chose the integer spelling with `format!("{}", n as i64)`
guarded only by `n.abs() < 1e21`. But `n as i64` is a **saturating** cast in
Rust: any `f64` ≥ `i64::MAX` clamps to `9223372036854775807` and any ≤
`i64::MIN` clamps to `-9223372036854775808`. So every integral value in
`[2^63, 1e21)` collapsed to the same i64 bound — a different number than the
source (a real miscompile, not just a formatting nit).

**Fix.** The i64 path is now guarded by `n.abs() < 2^63`
(`9223372036854775808.0`), the exact range where `n as i64` is lossless.
Integral values at or above `2^63` fall through to `n.to_string()`, which
prints the shortest decimal that round-trips to the same `f64`; the
exponential candidate is still compared so the shorter spelling wins
(`1e20` → `1E20`). Values that fit in i64 are unaffected.

Regression tests: `number_large_integral_does_not_saturate_to_i64_bound`
(round-trip + not-saturated, both signs), `number_values_within_i64_range_keep_exact_integer_spelling`.

## [0.18.7] - 2026-06-30

### Fixed — `**` exponentiation operand precedence (invalid-JS miscompile)

`emit_binary` emitted every operator's left child at `my_prec` and right child
at `my_prec + 1` — correct for the left-associative operators, but wrong for
`**`, which is right-associative AND whose grammar base is an `UpdateExpression`
(it binds tighter than unary). Two bugs resulted:

- **Invalid output.** A unary base printed without parentheses: `(-a)**2` became
  `-a**2`, which is a `SyntaxError` in JavaScript (a unary operator may not be
  the base of `**` without parens). Same for `(~a)**2`, `(!a)**2`.
- **Over-parenthesisation.** Right-associativity was not modelled, so
  `a**b**c` (which natively *is* `a**(b**c)`) printed as `a**(b**c)`.

`**` now emits its left at `PREC_UNARY + 1` (parenthesising a unary/lower base —
`(-a)**2`, `(a**b)**c`) and its right at `my_prec` (a same-precedence right needs
no parens — `a**b**c`; a unary right is still legal bare — `a**-b`). All other
operators are unchanged. New test `exponentiation_base_and_right_precedence`.

## [0.18.6] - 2026-06-30

### Fixed — member object / call callee lost its parentheses (miscompile)

`emit_member` emitted its object, and `emit_call` its callee, via
`emit_expression` (parent precedence 0), which never parenthesises. A
lower-precedence object/callee therefore dropped the parentheses that make it a
unit, changing the parse:

| input        | was         | now (correct) |
|--------------|-------------|---------------|
| `(a\|\|b).c`   | `a\|\|b.c` (= `a\|\|(b.c)`) | `(a\|\|b).c` |
| `(a=b).c`    | `a=b.c`     | `(a=b).c`   |
| `(a?b:c).d`  | `a?b:c.d`   | `(a?b:c).d` |
| `(a+b).c`    | `a+b.c`     | `(a+b).c`   |
| `(-a).b`     | `-a.b`      | `(-a).b`    |
| `(a\|\|b)()`   | `a\|\|b()`    | `(a\|\|b)()`  |
| `(a=b)(c)`   | `a=b(c)`    | `(a=b)(c)`  |

Both now emit the object/callee at `PREC_PRIMARY`. Member and call expressions
are themselves `PREC_PRIMARY`, so `a.b.c`, `f().x`, and `a.b()` stay
parenthesis-free, while anything lower (binary, logical, unary, conditional,
assignment, sequence) is wrapped. Computed member objects (`(a\|\|b)[c]`) are
covered too. New emitter test
`member_and_call_object_below_member_precedence_is_parenthesised`.

## [0.18.5] - 2026-06-30

### Changed — tighten binary/logical operator spacing in compact mode

`emit_binary` and `emit_logical` previously wrote a space on **both** sides of
every operator unconditionally, so compact output carried needless bytes —
`a + b`, `a && b`, `a << b`, `a === b` — diverging from upstream Closure's
`a+b`. Symbolic operators are now emitted tight in compact mode (a space appears
only in `pretty` mode), matching upstream's `assertPrint("2 + 3", "2+3")`.

Two carve-outs keep this sound:

- **Word operators** `in` / `instanceof` still get a mandatory space on both
  sides — `1 in obj`, not `1inobj`.
- **Additive `+` / `-`** keep one minimal space at a seam where the operand
  would otherwise fuse the pair into the increment/decrement token: `a + (+b)`
  emits `a+ +b`, never `a++b` (which reparses as `a++ b`); likewise `a - (-b)` →
  `a- -b`. No other symbolic operator can fuse (no operand begins or ends with
  `<`,`>`,`&`,`|`,`*`,`/`,`%`,`^`,`=`, and `++`/`--` are not representable unary
  operators), so they de-space unconditionally. The right-seam check reuses the
  existing `arg_starts_with_sign` helper (which is parenthesisation-aware: a
  wrapped operand leads with `(` and so needs no guard).

New tests cover tight symbolic output, retained word-operator spaces, and the
`a+ +b` / `a- -b` / negative-literal / mixed-sign (`a+-b`) hazard matrix.

## [0.18.4] - 2026-06-30

### Fixed — trailing array hole lost its comma (miscompile)

`emit_array` wrote one separating comma *between* elements, so an array whose
last element is a hole printed one comma short: `[Some(1), None]` became `[1,]`
(length 1) instead of `[1,,]` (length 2). A trailing hole is semantically real
(`[1,,].length === 2`), so the shortened output is a miscompile. The emitter now
appends one extra comma when the last element is a hole, fixing `[1,,]`, `[,,]`
(from `[None, None]`), `[,]` (from `[None]`), etc.; arrays ending in a real
element are unchanged. Pairs with the bridge elision fix in `javascript-parser`
0.19.4. New test `array_trailing_and_leading_holes_round_trip`.

## [0.18.3] - 2026-06-29

### Fixed — U+2028 / U+2029 emitted unescaped in string literals (invalid JS)

The default (non-`ascii_only`) string escapers `escape_str_dq` / `escape_str_sq`
only escaped control characters below `0x20`, so they passed U+2028 (LINE
SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR) through verbatim. Those two
codepoints are ECMAScript **line terminators**, and before ES2019 an unescaped
one inside a string literal is a `SyntaxError` — so a source string containing
either character could be minified into invalid output. (The `ascii_only`
escaper already escaped them as non-ASCII; only the default path was affected.)

Both escapers now emit ` ` / ` ` explicitly. New test
`line_and_paragraph_separators_are_escaped` covers the double-quoted,
single-quoted, and `ascii_only` paths. Flagged during the security review of the
property-key fix (0.18.2) as a pre-existing latent miscompile.

## [0.18.2] - 2026-06-29

### Added — sound quote-stripping for string object-property keys

`emit_property_key` now drops the quotes on a `PropertyKey::StringLiteral` —
`{"abc":1}` → `{abc:1}`, matching Closure's CodePrinter — but **only** when the
decoded `value` is a valid ASCII identifier name (new private `is_identifier_name`
helper) and is not `__proto__`. The two carve-outs are what make it sound:

- Non-identifier values stay quoted: `"a-b"`, `"a b"`, and `"x\ty"` would be
  syntax errors bare, and `"1"` would become a numeric key.
- `"__proto__"` stays quoted: the bare form `{__proto__: v}` is the §B.3.1
  prototype setter, a *different* object from the own property `{"__proto__": v}`.

Previously the emitter relied on the bridge to pre-decide the key node kind; the
bridge bug (see `javascript-parser` 0.19.3) meant every quoted key arrived as an
`Identifier` and was emitted bare regardless of validity. With the bridge now
producing faithful `StringLiteral` keys, this is the single place that decides
quote-vs-bare. New tests cover identifier, hyphen, space, leading-digit, tab,
reserved-word, and `__proto__` keys.

## [0.18.1] - 2026-06-29

### Fixed — negative zero (`-0`) lost its sign on emit (miscompile)

`format_js_number` short-circuited every value equal to `0.0` to the string
`"0"`. Because Rust's `-0.0 == 0.0` is `true`, negative zero took that path and
emitted `0` — but `-0` is observably distinct from `0` in JavaScript
(`1 / -0 === -Infinity` vs `1 / 0 === Infinity`; `Object.is(-0, 0) === false`),
so dropping the sign is a **miscompile**. It surfaced through the constant-fold
pass: `f(-0)`, `f(-(5-5))`, and `f(0 * -1)` all fold the argument to the
negative-zero numeric literal `-0.0`, which the emitter then printed as `0`.

The zero fast path now checks `is_sign_negative()` and emits `-0` for negative
zero, `0` otherwise. `-0` is also the minimal correct representation. Positive
zero and every other value are byte-identical to before — the blast radius is
exactly negative zero. End-to-end: `f(-0);` → `f(-0);` (was `f(0);`), and
`g(1/-0);` → `g(-Infinity);`.

## [0.18.0] - 2026-06-21

### Fixed — prefix-unary precedence and `--`/`++` token fusion

`emit_unary` emitted its argument with `emit_expression` (parent precedence 0),
so a lower-precedence operand was never parenthesised: `!(a == b)` printed as
`!a == b`, which JS reparses as `(!a) == b` — a **different program**. The
argument is now emitted at `PREC_UNARY`, so binary / logical / conditional /
assignment operands are wrapped:

| AST            | before    | after       |
|----------------|-----------|-------------|
| `!(a == b)`    | `!a == b` | `!(a == b)` |
| `-(a + b)`     | `-a + b`  | `-(a + b)`  |
| `~(a \| b)`    | `~a \| b` | `~(a \| b)` |
| `!(a ? b : c)` | `!a?b:c`  | `!(a?b:c)`  |

A second fusion hazard is now handled too: a sign operator (`-`/`+`) directly
followed by a same-sign operand fused into the decrement/increment token —
`-(-a)` printed `--a` (pre-decrement of `a`), `+(+a)` printed `++a`. `emit_unary`
now inserts a separating space in exactly those cases (`- -a`, `+ +a`, `- -5`),
via the new `sign_op_char` / `arg_starts_with_sign` helpers. `!`/`~` never fuse,
and equal-precedence nests like `!!a` are left paren-free.

These miscompiles were latent until the `javascript-parser` bridge stopped
dropping prefix operators (it had been emitting the bare operand); this change
ships alongside that fix. Added 10 emitter unit tests.

## [0.17.0] - 2026-06-20

### Added — CLOC23: emit `for (… of …)`

New `emit_for_of` writes `for ( <left> of <right> ) <body>` — identical to
`emit_for_in` but with the `of` keyword, spaced on both sides for the same
token-separation reason (the left ends in an identifier and the right starts
with one). Added emitter unit tests for the `var` declaration-left and
expression-left forms.

## [0.16.0] - 2026-06-20

### Added — CLOC22: emit `for (… in …)`

New `emit_for_in` writes `for ( <left> in <right> ) <body>`. The `in` keyword is
separated on both sides with `required_ws` (a single space): the left ends in an
identifier (`var k` / `k` / `o.p`) and the right starts with one, so `kin` /
`inobj` would otherwise mis-lex. In the rare `a[b] in` / `in (x)` cases the space
is one redundant byte but never wrong, matching upstream Closure's spacing around
`in`. Added emitter unit tests for the `var`/`const` declaration left and the
expression-left (bare-body) forms.

## [0.15.0] - 2026-06-20

### Added — CLOC21: emit `debugger;`

New `emit_debugger` writes `debugger;`. The keyword is followed only by its
terminator `;` (or, after the semi is popped, a `}`/EOF), so no token-separation
handling is needed. `DebuggerStatement` is added to
`last_stmt_uses_terminator_semi` — its `;` is popped before a closing `}` (e.g.
`{debugger}`), like `return`/`throw`/expression statements. Added emitter unit
tests for the keyword+semi, terminator-pop, and not-last (semi kept) cases.

## [0.14.0] - 2026-06-20

### Added — CLOC20: emit `do`/`while`

New `emit_do_while` writes `do <body> while ( <test> ) ;`. Token-separation: a
required space is inserted after `do` only when the body is NOT a block (so
`do{…}` stays tight but `do foo();` does not glue into `dofoo()`). The trailing
`;` is a real statement terminator, so `DoWhileStatement` is added to
`last_stmt_uses_terminator_semi` — its `;` is popped before a closing `}` (e.g.
`{do{a}while(b)}`) just like `return`/`throw`/expression statements, but unlike
plain `while` (whose trailing `;` is a body slot). Added emitter unit tests for
the tight, bare-body, pretty-mode, and terminator-pop cases.

## [0.13.0] - 2026-06-20

### Added — CLOC19: emit `try`/`catch`/`finally`

New `emit_try` writes `try <block> [ catch [(param)] <block> ] [ finally <block> ]`.
No `required_ws` is needed anywhere: every boundary is keyword↔`{`/`}` or
`}`↔keyword (`try{…}catch{…}`, `}finally{…}`), which lex cleanly with no
separator. Pretty mode inserts readability spaces (`try {`, `catch (e) {`,
`finally {`); minified mode emits the tight form. The optional-catch-binding
form (`catch { … }`) emits with no parens. Added emitter unit tests pinning the
minified token boundaries, the optional-binding and no-catch forms, and the
pretty-mode spacing.

## [0.12.0] - 2026-06-08

### Changed
- **gap-030 part 1 (AST emitter side):** Function-declaration
  ASI policy now matches upstream Closure v20240317 in compact
  (non-pretty) mode.
  - `emit_block_statement` drops a single trailing `;` before
    `}` via a new `pop_trailing_semi_if_compact()` helper —
    BUT only when the block's last child is a leaf
    statement-terminator type (gated by
    `last_stmt_uses_terminator_semi`). Per ECMAScript §11.9
    (Automatic Semicolon Insertion), the `}` terminates the
    in-progress statement, so a true terminator `;` is
    redundant noise that upstream Closure doesn't emit.
  - **Critical correctness gate:** when the block's last child
    is a compound statement (If/While/For/Labeled) whose body
    is an `EmptyStatement`, the trailing `;` is structurally
    part of the body grammar — NOT a terminator. Popping it
    would produce SyntaxError-emitting output like
    `function f(){if(x)}` or `function f(){for(;;)}`. The
    `last_stmt_uses_terminator_semi` helper enumerates the
    safe-to-pop set (ExpressionStatement / ReturnStatement /
    BreakStatement / ContinueStatement / ThrowStatement); all
    other types fall through to the conservative "preserve
    `;`" path. Caught by pre-push security review.
  - `emit_function_declaration` emits a `;` after the body's
    closing `}` in compact mode. Even at EOF this is a no-op
    `EmptyStatement`, but in concatenation contexts it keeps
    the function-declaration's output shape predictable.
- **Pretty mode is unchanged.** Visual delimiter clarity
  outranks byte minimization in the human-facing pretty mode;
  both gap-030 changes are compact-only.
- The pre-existing `function_declaration_minified` test
  assertion was updated from `function f(x){return x;}` to
  `function f(x){return x};` to track the new compact-mode
  shape.

### Added
- Five new inline tests:
  - `gap030_block_multi_stmts_drops_only_last_semi`: multi-
    statement blocks drop only the final `;` (intermediate
    `;`s between statements are preserved).
  - `gap030_pretty_mode_unchanged`: pretty mode still emits
    inner `;` and no trailing `;` after `}`.
  - `gap030_empty_function_body_compact`: empty body stays
    `{}` with trailing `;` (`function noop(){};`).
  - `gap030_does_not_pop_empty_body_of_if`: regression for the
    security-review-caught bug — `function f(){if(x);}` must
    not collapse to `function f(){if(x)}` (SyntaxError).
  - `gap030_does_not_pop_empty_body_of_while`: same defense
    applied to `while(x);` body shape.

### Known limitation
- **closurec's CLI WHITESPACE_ONLY path uses a separate
  token-level re-stitcher** (`whitespace_only.rs`), not this
  emitter. So gap-030's `minify_function_decl` byte-identity
  fixture stays IGNORED for now; flipping it to PASS requires
  porting the same two rules to the token re-stitcher in a
  follow-up PR (gap-030 spec entry tracks both parts).

## [0.11.0] - 2026-06-04

### Added — CLOC12.33: `SwitchStatement` emitter rule (gap-014)

Adds `emit_switch` + `emit_switch_case` covering the new
`SwitchStatement` / `SwitchCase` variants shipped in
`javascript-ast` 0.7.0.

Output shape (compact mode):

```
switch(<discriminant>){case <test>:<consequent>...case <test>:<consequent>...default:<consequent>...}
```

- `case <test>:` is emitted with a single space between `case`
  and the test expression (required to avoid `case1:` ambiguity).
- `default:` has no space (no expression follows the colon).
- Empty consequent emits just `case <test>:` / `default:` with
  nothing after the colon.
- Multiple cases concatenate in source order. The trailing `}`
  closes the switch directly — no separating semicolons.

Pretty mode (`opts.pretty = true`) lays each case on its own
indented line and each consequent statement on its own
double-indented line under the case header. Same trailing-comma
discipline as block: no separator before the closing brace.

Six new inline tests pin: empty switch, single `case 1:` with
expression-statement body, `default:` with body, `case 1:` with
empty body, full `case/case/default` triple in order, and
`case 1: break;` (break-in-consequent invariant).

## [Unreleased] — CLOC12.32: trailing-comma ports (gap-022)

Test-only. Closes `gap-022`.

* New port file `tests/upstream/code_printer_trailing_comma_test.rs` — 16 hand-built tests pinning that the emitter never emits a trailing comma before `]` or `}`, covering empty / single / multi / nested arrays and objects in **both** compact and pretty modes, plus the elision-isn't-a-trailing-comma edge case.
* Re-annotated the old gap-022 placeholder in `tests/upstream/code_printer_test.rs::test_trailing_comma_in_array_and_object_with_pretty_print` to route to the new home.
* Added the `[[test]]` entry in `Cargo.toml` (Cargo only auto-discovers `tests/*.rs` one level deep).

The original gap entry asked for a `trailing_comma: bool` AST flag and an emitter rule. Reviewing the upstream test family showed the flag is unnecessary: upstream's `assertPrettyPrint("var x = [1,];", "var x = [1];\n")` relies on parse-side trailing-comma stripping (it's purely syntactic in ES2017, not an elision) and emitter-side never-write. Our AST already collapses `[1,]` to `[Some(1)]` (identical to `[1]`) at parse time, and the emitter loop only writes `,` between elements. Spec gap-022 → RESOLVED with that note.

No source-incompatible change. No production code touched.

## [0.10.0] - 2026-06-01

### Added — CLOC12.16: emit `UndefinedLiteral` as `void 0` (gap-001)

Adds emitter support for the new `UndefinedLiteral` variant.
Renders as `void 0` — six characters of shadow-safe goodness.

**Why `void 0` and not the keyword `undefined`?** In ECMAScript
`undefined` is an *identifier*, not a reserved word. Code can
legally do `var undefined = 1;` in non-strict mode (or just declare
a local `undefined` parameter) and that binding shadows the global.
Reading the identifier `undefined` from inside such a scope would
yield the shadow value, not the genuine undefined.

`void <expression>` always evaluates `<expression>` and then
produces the **real** undefined value, regardless of any name in
scope. `void 0` is the shortest spelling and matches upstream
Closure Compiler's `CodePrinter` behaviour.

**Precedence wiring.** `UndefinedLiteral` is mapped to `PREC_UNARY`
(not `PREC_PRIMARY` like other literals) so contexts like
`(void 0).x` and `(void 0)()` automatically get the parens they
need. Without that, the emit would produce `void 0.x` — which
JS parses as `void (0.x)`, a semantically different expression.

Two inline tests cover the bare `void 0;` output for traced and
untraced cases.

## [0.9.0] - 2026-06-01

### Added — CLOC12.15: emit `BigIntLiteral` (gap-021)

Adds emitter support for the new `BigIntLiteral` variant added to
`javascript-ast` in CLOC12.15. The emitter writes the `raw` field
verbatim — we deliberately do NOT reformat from `value` because:

1. Hex/octal/binary radixes (`0x1fn`, `0o17n`, `0b11111n`) are part
   of the literal's source identity and shorter than their decimal
   equivalents.
2. There is no exponential bigint syntax (`1e9n` is not valid JS),
   so the number-formatter's shortest-form trick from CLOC12.12
   doesn't apply.

Therefore: `raw` in, same string out. The PREC_PRIMARY table also
lists `BigIntLiteral` so emit_expression_inner never inserts
unnecessary parens around bigint literals in nested contexts.

Three new inline tests cover decimal (`123n`), zero (`0n`), and
hex (`0x1fn`) cases.

## [0.8.0] - 2026-06-01

### Added — CLOC12.14: emit `ThrowStatement` (gap-020 AST partial close)

Adds emitter support for the new `ThrowStatement` variant added to
`javascript-ast` in CLOC12.14. Compact form is `throw <expr>;` with
a mandatory single space between the keyword and the expression
(without it `throw1` parses as an identifier in lenient lexers and
is ambiguous in strict ones); the trailing `;` is always emitted.

Tests cover `throw 1;` (NumericLiteral), `throw e;` (Identifier),
and `throw "oops";` (StringLiteral — confirms the quote-choice path
runs the same way it does for any other string expression context).

## [0.7.0] - 2026-06-01

### Added — CLOC12.13: emit `LabeledStatement` (gap-009 AST partial close)

Adds emitter support for the new `LabeledStatement` variant added to
`javascript-ast` in CLOC12.13. Compact form is `label:body` with
no whitespace; pretty form is `label: body` (single space between
the colon and the body). The body's own emitter writes its trailing
`;`, so we never double-print.

Tests cover `a: foo();`, `break;`, `break a;`, and the literal
upstream-test input `a: break a;`. The `BreakStatement` emit path
was already in place from the original 0.1.0 scaffold — these tests
pin its current behaviour now that there's a label node to combine
with.

## [0.6.0] - 2026-06-01

### Added — CLOC12.12: number formatting shortest-form (closes gap-025)

`format_js_number` now computes both decimal and exponential
representations for finite non-zero numbers and returns whichever is
shorter. Ties pick decimal (canonical). Matches upstream
`CodePrinter`'s behaviour.

| Value | Old emit | New emit |
|-------|----------|----------|
| `1` | `1` | `1` |
| `100` | `100` | `100` (tie 3=3 → decimal) |
| `1_000_000_000` | `1000000000` | `1E9` |
| `5_000_000` | `5000000` | `5E6` |
| `0.5` | `0.5` | `0.5` |
| `1.5e-10` | `0.00000000015` | `1.5E-10` |
| `NaN` | `NaN` | `NaN` |
| `Infinity` | `Infinity` | `Infinity` |

Exponential form follows JS / upstream conventions: uppercase `E`,
no leading `+` for positive exponents, stripped trailing zeros in
the mantissa (`1E9`, not `1.0E+9`).

### New helper

`format_exponential_uppercase(n: f64) -> String` — wraps Rust's
`{:e}` formatter and uppercases the `E`.

### New inline tests (5)

- `number_shortest_form_small_integers_stay_decimal` — `0`, `1`, `42`, `100`, `-7`.
- `number_shortest_form_big_integers_switch_to_exponential` — `1E9`, `5E6`.
- `number_shortest_form_small_decimals_stay_decimal` — `0.5`, `3.14`.
- `number_shortest_form_tiny_floats_switch_to_exponential` — `1.5E-10`.
- `number_shortest_form_nan_and_infinity_unchanged` — sanity check.

Plus `emit_number_value(v: f64) -> String` helper.

### gap-025 → RESOLVED

### Reconciles missing version bump from CLOC12.11

CLOC12.11 (PR #4703) updated the CHANGELOG to `[0.5.0]` but the
`Cargo.toml` change was dropped, leaving the published crate at
`0.4.0`. This PR bumps directly `0.4.0` → `0.6.0`: the `0.5.0`
CHANGELOG entry below stays valid as the description of quote-choice
work; `0.6.0` is the first published version that actually includes
both quote-choice (CLOC12.11) AND shortest-form number rendering
(CLOC12.12).

### Version

`0.4.0` → `0.6.0` (skips `0.5.0` to absorb the missed CLOC12.11
Cargo.toml bump).

## [0.5.0] - 2026-06-01

### Added — CLOC12.11: string quote-choice optimisation (closes gap-026)

`emit_string` now picks the quote style that minimises required
escape characters. Upstream's CodePrinter does the same; matching it
brings us a step closer to byte-identical output.

**Algorithm** — count occurrences of `"` and `'` in the value. If
`"` strictly outnumbers `'`, emit with single quotes (each saved
`\"` is shorter); otherwise emit with double quotes (canonical form,
ties broken toward double).

```
value                            chosen quote   why
-----------------------------    ------------   -------------------
hello                            double         no quotes anywhere
o'malley                         double         no `"`; cheaper as `"o'malley"`
she said "hi"                    single         `"` saves one escape
"mixed 'both'"                   double         tie (2 each) → double
""x                              single         two `"`, zero `'`
```

`ascii_only` mode still always uses double quotes — switching mid-mode
would confuse downstream readers and upstream itself maintains that
invariant.

### New helpers in `lib.rs`

- `choose_quote_and_escape(value: &str) -> (&'static str, String)` —
  returns the chosen quote character plus the escaped body.
- `escape_str_sq(s: &str) -> String` — single-quoted variant of
  `escape_str_dq`. Identical control-char rules; differs only in
  which quote it escapes.

### New inline tests (6)

- `quote_choice_no_quotes_uses_double` — `"hello"`, `""`.
- `quote_choice_single_quotes_in_value_uses_double` — `"o'malley"`, `"it's"`.
- `quote_choice_double_quotes_in_value_switches_to_single` — `'she said "hi"'`.
- `quote_choice_tie_picks_double` — value `'"`, leading byte = `"`.
- `quote_choice_more_double_than_single_picks_single` — value `""x`, leading byte = `'`.
- (helper) `emit_string_value(value: &str) -> String` — emit a
  synthetic StringLiteral and return the code; used by the four
  parametric assertions.

### Side effect

The previous emit_string path used `s.raw` verbatim when present
(preserving the source-file's quote style). That's no longer used —
quote-choice now applies uniformly. The `raw` field is still
preserved in the AST for tooling but isn't consulted by emit.

### gap-026 → RESOLVED

The `test_string_quote_choice_minimises_escapes` placeholder in
`tests/upstream/code_printer_test.rs` stays `#[ignore]`-d pending a
follow-up that re-ports it with real upstream `assertPrint` cases
now that the underlying emitter behaviour is in place.

### Version bump

`0.4.0` → `0.5.0`.

## [0.4.0] - 2026-06-01

### Added — CLOC12.10: precedence-aware paren insertion (closes gap-024 + gap-027)

Replaces the previous "wrap every expression-statement body in parens"
policy with a precedence-aware emit. `emit_expression_inner(e, parent_prec)`
inspects the expression's own precedence and wraps in parens **only**
when the child binds more loosely than its parent context demands.

Precedence ladder (low → high, per ESTree §13):

```
 0   top level / statement / control-test position
 1   assignment
 2   conditional `? :`
 3   logical-or `||` / nullish `??`
 4   logical-and `&&`
 5-7 bitwise or/xor/and
 8   equality
 9   relational
10   shift
11   additive
12   multiplicative
13   exponent          right-assoc
14   prefix unary
17-18 call/member/primary       atomic — never wraps
```

Three new helper functions in `lib.rs`:

- `binary_prec(BinaryOperator) -> u8`
- `logical_prec(LogicalOperator) -> u8`
- `expr_prec(&Expression) -> u8`

`emit_binary` / `emit_logical` / `emit_conditional` no longer wrap
themselves in parens. They delegate to `emit_expression_inner` for
their children with appropriate `parent_prec` values
(`my_prec` for the left side, `my_prec + 1` for the right side of
left-associative operators).

### Truth table

| Source AST                     | Old emit               | New emit          |
|--------------------------------|------------------------|-------------------|
| `2 + 3`                        | `(2 + 3);`             | `2 + 3;`          |
| `"a" + "b"`                    | `("a" + "b");`         | `"a" + "b";`      |
| `(a + b) * c`                  | `((a + b) * c);`       | `(a + b) * c;`    |
| `a + b * c`                    | `(a + (b * c));`       | `a + b * c;`      |
| `!x`                           | `!x;` (unchanged)      | `!x;`             |
| `({a:1})`                      | `({a:1});` (unchanged) | `({a:1});`        |

### gap-024 → RESOLVED

The two `_is_current_behaviour` ports in
`tests/upstream/code_printer_test.rs` are renamed and their assertions
flipped to upstream's byte-equivalent forms:

| Old name (pinned divergence) | New name (matches upstream) | Was | Now |
|-------------------------------|------------------------------|-----|-----|
| `test_binary_addition_with_parens_is_current_behaviour` | `test_binary_addition_emits_without_outer_parens` | `(2 + 3);` | `2 + 3;` |
| `test_string_concat_with_parens_is_current_behaviour` | `test_string_concat_emits_without_outer_parens` | `("a" + "b");` | `"a" + "b";` |

The remaining whitespace difference between our `2 + 3;` (pretty-printed)
and upstream's `2+3;` (minified) is addressed by the pretty/minify
toggle work, not gap-024.

### gap-027 → RESOLVED (incidental)

The precedence ladder also closes gap-027 (precedence-aware paren
insertion) — they were two views of the same underlying problem. The
`test_operator_precedence_inserts_inner_parens` placeholder stays
`#[ignore]`-d pending a follow-up that adds the actual upstream
`a*(b+c)` test cases now that the emitter supports them.

### Updated inline tests

Three inline tests in `lib.rs`:

- `binary_addition_with_parens` renamed to `binary_addition_emits_without_outer_parens`, assertion flipped to `"2 + 3;"`.
- `string_concat_with_parens` renamed to `string_concat_emits_without_outer_parens`, assertion flipped to `"\"foo\" + \"bar\";"`.
- `untraced_program_still_emits` assertion flipped to `"2 + 3;"`.

### Version bump

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-05-31

### Added — CLOC12.07: port subset of upstream `CodePrinterTest`

Fourth port under CLOC12, first one targeting the emitter rather than
a transform pass.

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5.
- `tests/upstream/code_printer_test.rs` — 12 ported test methods.

### Test breakdown

|     | passing | ignored |
|-----|---------|---------|
| CLOC12.07 | **6** | **6** |

**Passing (6):** literal-position emits and bare unary that match our
current emitter output exactly:

- `test_binary_addition_with_parens_is_current_behaviour` — `2 + 3` emits as `(2 + 3);` (paren-wrapped; pins current behaviour).
- `test_string_concat_with_parens_is_current_behaviour` — `"a" + "b"` emits as `("a" + "b");`.
- `test_unary_not_emits_without_space` — `!x` emits as `!x;`.
- `test_boolean_literal_at_statement_position` — `true;` / `false;`.
- `test_integer_literals_at_statement_position` — `0;`, `42;`, `1;`.
- `test_string_literal_at_statement_position` — `"hello";`, `"a";`.

Two of those (`test_*_is_current_behaviour`) deliberately pin our
*current* paren-wrapping behaviour even though it diverges from
upstream — they serve as regression markers so when gap-024 is closed
and the wrapping comes off, the assertions can flip at the same time.

**Ignored (6):** record upstream's broader scope:

| Test | Gap | Blocker |
|------|-----|---------|
| `test_big_int` | gap-021 | `BigIntLiteral` not in Phase 1 AST |
| `test_trailing_comma_in_array_and_object_with_pretty_print` | gap-022 | array/object trailing-comma policy not modelled |
| `test_no_trailing_comma_in_empty_array_literal` | gap-023 | VariableDeclaration round-trip ports deferred |
| `test_number_formatting_shortest_form` | gap-025 | numeric exponential-form / shortest-form not implemented |
| `test_string_quote_choice_minimises_escapes` | gap-026 | quote-choice optimisation not implemented |
| `test_operator_precedence_inserts_inner_parens` | gap-027 | precedence-aware paren insertion not implemented |

Plus the meta-divergence:

- gap-024 — `ExpressionStatement` paren-wrapping is unconditional in our emitter, whereas upstream only wraps when ambiguity demands it. Not strictly "blocked" (we choose to wrap), but tracked so the eventual byte-identical match can flip the two `_is_current_behaviour` ports.

### Why the bulk of upstream is ignored

`CodePrinterTest` has 263 `@Test` methods. Most cover Phase 2+ AST
nodes (BigInt, optional chaining, template literals, classes,
spread, async/await, regex) or formatting policies (quote choice,
exponential-form numerics, precedence-aware parens) that aren't in
our emitter's v0.2.0 body. Each future emitter slice can re-port
the relevant subset and convert ignored markers into asserts.

### Version bump

`0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-24

### Added — real `emit` body (first real pipeline output)

Replaces v0.1.0's identity emit with a recursive printer that walks every Phase 1 AST node and produces JavaScript text. Step 3 of 4 in the autonomous-chain real-body rollout (after constant-fold + fold-control-flow; before DCE).

- Walks every Phase 1 variant: all expressions (Identifier, literals, Binary/Logical/Unary/Assignment/Conditional/Call/Member, Array with elisions, Object with shorthand/method), all statements (Expression, Block, If, While, For, Return, Break, Continue, Empty), and Declarations (Variable/Function).
- Honors all three `EmitOptions`:
  - `pretty: false` (default) → minified single-line.
  - `pretty: true` → 2-space-indented multi-line for block bodies.
  - `ascii_only: true` → escape non-ASCII as `\uXXXX` / `\u{XXXXXX}`.
  - `source_map: true` (default) → accumulate `(line, col, cv_id)` mappings via `SourceMapBuilder`, serialize as v3 JSON in `EmitOutput.source_map`.
- Tracks line/col cursor (UTF-16 code units per source-map v3 spec).

### Always-parenthesize policy in v1

v1 always parenthesizes `BinaryExpression`, `LogicalExpression`, `ConditionalExpression`, and `AssignmentExpression`. Precedence-aware elision is Phase 1.x. `ObjectExpression` at statement position is also wrapped (else `{}` parses as a block).

### CV tracing — both modes per CLOC09

- **Traced** (`cv: Some` on nodes) → `add_mapping` called per token.
- **Untraced** (`cv: None`) → no mappings recorded; output text identical; `source_map` field still contains a valid empty-mappings v3 blob when enabled.

### Headline test — end-to-end pipeline

```rust
let prog = AST(2 + 3);
let pipeline_out = PassPipeline::new()
    .add(ConstantFoldPass::new())
    .run(prog, &sidecar, &mut cv);
let emit_out = emit(&pipeline_out.program, ...);
assert_eq!(emit_out.code, "5;");
```

The full stack — AST → optimization → emit → text — works end-to-end for the first time.

### Tests

17 tests (up from 9 in v0.1.0): defaults + empty, basic expressions with always-paren, typeof spacing, const/function declarations (minified and pretty), `[1,,3]` array with elision, `({a:1,b:2});` ObjectExpression paren-wrap at statement start, `ascii_only` escapes Unicode (verified with "café"), `source_map` on/off, untraced still emits, **end-to-end pipeline produces `5;` from `2 + 3`**, `EmitError` `std::error::Error` compat.

### Dependencies
- Added `coding-adventures-closure-source-map` as a runtime dep.
- Added `coding-adventures-closure-pass-constant-fold` + `coding-adventures-closure-pass-pipeline` as dev-deps for the end-to-end test.

### Skipped (Phase 1.x / Phase 2+)
- Precedence-aware paren elision.
- Real source-map VLQ encoding (lives in `closure-source-map` v2; mappings accumulate now, final string is still empty).
- `FunctionExpression`, `ArrowFunctionExpression`, `ClassDeclaration` — Phase 2/3.
- JSDoc comment preservation.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC07 emit-and-source-map spec — the back end of the Closure Compiler clone. Takes a finalized `Program` + sidecar and produces output JavaScript text + companion source-map blob.
- `emit(program: &Program, sidecar: &Sidecar, cv: &mut CVLog, opts: &EmitOptions) -> Result<EmitOutput, EmitError>` — the canonical entry point. Signature pinned.
- `EmitOptions` struct with three knobs:
  - `ascii_only: bool` (default `false`) — when `true`, escape non-ASCII codepoints to `\uXXXX` / `\u{XXXXXX}`.
  - `pretty: bool` (default `false`) — production default is minified; switch on for human-reviewed output.
  - `source_map: bool` (default `true`) — production default is to emit a companion `.js.map`.
- `EmitOutput` struct:
  - `code: String` — JavaScript bytes (UTF-8 or ASCII-restricted).
  - `source_map: Option<String>` — source-map v3 blob; `None` when `source_map = false`.
  - `contributions: Vec<Contribution>` — per-token "emitted" CV trail per CLOC03.
- `EmitError` enum (`#[non_exhaustive]`) with `Display` + `std::error::Error` impls:
  - `UnknownCvId { id, site }` — AST referenced a CV id the log doesn't know.
  - `UnsupportedSidecarType { id, kind }` — sidecar held a type the emitter can't render.
- v1 body: emits empty `code`, an empty source-map placeholder when `source_map = true`, no contributions. `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to render. The real AST walk lands once the AST grows `Statement` / `Expression` / `Declaration` variants.
- 9 tests covering: `EmitOptions::default()` values, identity emit on empty program with default opts (code empty, source_map present-but-empty, contributions empty), `source_map = false` drops the source-map field entirely, `ascii_only` flag accepted (output trivially ASCII when empty), `pretty` flag accepted, `EmitOptions` `Clone` + `PartialEq`, `EmitError::Display` formats for both variants include the id/site/kind they carry, `EmitError` implements `std::error::Error`.

### Notes
- Dependencies: `coding-adventures-javascript-ast` (`Program`), `coding-adventures-type-sidecar` (`Sidecar` for future emit hints), `coding_adventures_correlation_vector` (`CVLog`, `Contribution`), `serde` + `serde_json` (for future source-map serialization and `Contribution.meta`). Dev-deps: `coding-adventures-javascript-tokens` for `EsVersion`.
- The emitter does **not** depend on `closure-pass-pipeline` or any pass crate. It runs after the pipeline and only consumes the final `Program` shape — keeping that decoupling means future passes can be added without touching the emit dependency graph.
- v1 is scaffolding. The function signature, options struct, output struct, and error enum are the deliverable that the future `closurec` CLI (CLOC08) and the source-map generator (`closure-source-map`, CLOC07 Phase 2) link against. The body fills in once the AST grows variants.

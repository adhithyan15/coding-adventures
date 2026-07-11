# Changelog

All notable changes to the `coding-adventures-closurec` binary will be documented in this file.

## [0.234.21] - 2026-07-11

### Added — CLOC12.182: private generator methods end-to-end

Picks up javascript-parser 0.46.0, whose bridge now sets the `generator` flag on
a `*#g(){}` private method's `FunctionExpression` value instead of declining. A
private generator method — `class C { *#g(){} }`, optionally `static` — now
survives the full closurec pipeline instead of dropping to WHITESPACE_ONLY.
New e2e diff fixture `tests/diff/simple-private-generator-method/`:
`class C { *#g(){ return 1 + 2 } }` → `class C{*#g(){return 3}}` at SIMPLE.

The fixture proves two things at once: the `*#g` head round-trips (the emitter
reprinted the `*` before the private-name key from the propagated `generator`
flag), and the SIMPLE pipeline descends INTO the private generator's body
(`return 1 + 2` folds to `return 3`). Before this bridge change the private
generator declined, dropping the file to WHITESPACE_ONLY
(`class C{*#g(){return 1+2}};`).
Version-synced cli.spec.json + tests/diff/help-markdown/expected.stdout. PATCH.

## [0.234.20] - 2026-07-11

### Added — CLOC12.181: generator methods end-to-end

Picks up javascript-parser 0.45.0, whose bridge now sets the `generator` flag on
a `*m(){}` method's `FunctionExpression` value instead of declining. A generator
method — `class C { *gen(){} }` or `x = class { *gen(){} }`, optionally `static`
— now survives the full closurec pipeline instead of dropping to WHITESPACE_ONLY.
New e2e diff fixture `tests/diff/simple-generator-method/`:
`class C { *gen(){ return 1 + 2 } }` → `class C{*gen(){return 3}}` at SIMPLE.

The fixture proves two things at once: the `*gen` head round-trips (the emitter
reprinted the `*` from the propagated `generator` flag), and the SIMPLE pipeline
descends INTO the generator method's body (`return 1 + 2` folds to `return 3`).
Before this bridge change the generator method declined, dropping the file to
WHITESPACE_ONLY (`class C{*gen(){return 1+2}};`).
Version-synced cli.spec.json + tests/diff/help-markdown/expected.stdout. PATCH.

## [0.234.19] - 2026-07-11

### Added — CLOC12.180: computed member keys end-to-end

Picks up javascript-parser 0.44.0, whose bridge now lowers a computed `[expr]`
member key to `PropertyKey::Expression`. Computed keys — in a class field
(`[k] = v`), a class method (`[k](){}`), or an object literal (`{[k]: v}`) — now
survive the full closurec pipeline instead of dropping to WHITESPACE_ONLY. New
e2e diff fixture `tests/diff/simple-computed-key/`:
`class C { [k] = 1 + 2 }` → `class C{[k]=3;}` at SIMPLE.

The fixture proves the SIMPLE pipeline descends INTO the computed field's
initializer: `1 + 2` folds to `3`. Before this bridge change the computed key
declined, dropping the file to WHITESPACE_ONLY (`class C{[k]=1+2};`).
Version-synced cli.spec.json + tests/diff/help-markdown/expected.stdout. PATCH.

## [0.234.18] - 2026-07-11

### Added — CLOC12.179: private accessors end-to-end

Picks up javascript-parser 0.43.0, whose bridge now lowers a private getter /
setter (`get #x(){}`, `set #x(v){}`) to a `ClassMember::Method` with
`MethodKind::Get` / `MethodKind::Set` and a `PropertyKey::PrivateName` key. A
private accessor now survives the full closurec pipeline. New e2e diff fixture
`tests/diff/simple-private-getter/`:
`class C { get #x(){ return 1 + 2 } }` → `class C{get #x(){return 3}}` at SIMPLE.

The fixture proves the SIMPLE pipeline descends INTO the getter's body: `1 + 2`
folds to `3`. Before this bridge extension the private getter declined, dropping
the file to WHITESPACE_ONLY (`class C{get #x(){return 1+2}};`). Version-synced
cli.spec.json + tests/diff/help-markdown/expected.stdout. PATCH.

## [0.234.17] - 2026-07-11

### Added — CLOC12.178: private methods end-to-end

Picks up javascript-parser 0.42.0, whose bridge now lowers a
`private_method_definition` node to a `ClassMember::Method` with a
`PropertyKey::PrivateName` key. A private method now survives the full closurec
pipeline. New e2e diff fixture `tests/diff/simple-private-method/`:
`class C { #m(){ return 1 + 2 } }` → `class C{#m(){return 3}}` at SIMPLE.

The fixture proves the SIMPLE pipeline descends INTO the private method's body:
`1 + 2` folds to `3`. Before this bridge change the private method declined,
dropping the file to WHITESPACE_ONLY (`class C{#m(){return 1+2}};`, arithmetic
intact). Version-synced cli.spec.json + tests/diff/help-markdown/expected.stdout.
PATCH.

## [0.234.16] - 2026-07-11

### Added — CLOC12.177 PR2: private class fields end-to-end

Picks up javascript-parser 0.41.0, whose bridge now lowers a private field's
`PRIVATE_NAME` key to `PropertyKey::PrivateName`. A private field now survives the
full closurec pipeline. New e2e diff fixture `tests/diff/simple-private-field/`:
`class C { #x = 1 + 2 }` → `class C{#x=3;}` at SIMPLE.

The fixture proves the SIMPLE pipeline descends INTO the private field's
initializer: `1 + 2` folds to `3`. Before this bridge change the private field
declined, dropping the file to WHITESPACE_ONLY (`class C{#x=1+2};`, arithmetic
intact). Version-synced cli.spec.json + tests/diff/help-markdown/expected.stdout.
PATCH.

## [0.234.15] - 2026-07-11

### Added — CLOC12.176 PR2: static-init blocks end-to-end

Picks up javascript-parser 0.40.0, whose bridge now produces
`ClassMember::StaticBlock`. A `static { … }` block now survives the full closurec
pipeline. New e2e diff fixture `tests/diff/simple-static-block/`:
`class C { static { x = 1 + 2 } }` → `class C{static{x=3}}` at SIMPLE.

The fixture proves the SIMPLE pipeline descends INTO the block's statement list:
the body's `1 + 2` folds to `3`. Before this bridge change the `static_block`
member declined, dropping the file to WHITESPACE_ONLY
(`class C{static{x=1+2}};`, arithmetic intact). The existing WHITESPACE_ONLY
fixtures (`minify_class_static_block`, `minify_class_static_blocks`) exercise the
emitter's grammar-AST path; this one exercises the typed-AST bridge + fold path.
PATCH.

## [0.234.14] - 2026-07-11

### Added — CLOC12.175 PR2: class fields end-to-end

Picks up javascript-parser 0.39.0, whose bridge now produces
`ClassMember::Field`. A class field now survives the full closurec pipeline. New
e2e diff fixture `tests/diff/simple-class-field/`:
`class C { x = 1 + 2; static s = 5 + 6; }` → `class C{x=3;static s=11;}` at SIMPLE
— both field initializers constant-fold (proving the pipeline descends into each
field's initializer via the PR1 pass arms), the `static` modifier survives, and
the class declaration emits bare. PATCH; version-synced `cli.spec.json` +
`tests/diff/help-markdown/expected.stdout` to 0.234.14.

## [0.234.13] - 2026-07-11

### Added — CLOC12.174 PR2: class-declaration end-to-end (`tests/diff/simple-class-decl/`)

With `javascript-parser` 0.38.0's bridge, a top-level class **declaration**
(`class C { … }`) now flows through the full SIMPLE pipeline
(parser → typed-AST bridge → passes → emitter) instead of dropping the whole file
to WHITESPACE_ONLY. New e2e diff fixture `tests/diff/simple-class-decl/`: input
`class C { m() { return 1 + 2 } }` minifies to `class C{m(){return 3}}` —
asserting the class declaration round-trips, the method body folds (`1 + 2` → `3`,
proving the pass descended into the method), and it emits **bare** (no trailing
`;`, no wrapping paren — unlike the class *expression* form's `(class …);`).

## [0.234.12] - 2026-07-08

### Added — CLOC12.173 PR2: class expressions flow through the SIMPLE/ADVANCED pipelines (gap-167)

With the `javascript-parser` 0.37.0 bridge now building a real `ClassExpression`
from a `class_expression` grammar node (instead of declining it, which dropped
the whole file to WHITESPACE_ONLY), a **class expression** minifies through the
full parser → bridge → passes → emitter pipeline.

Two new fixtures:

- `tests/diff/simple-class/`: `f(class { m() { return 1 + 2 } }, 3 + 4);` →
  `f(class{m(){return 3}},7);` at SIMPLE. The class round-trips minified, the
  method body folds (`1 + 2` → `3`, proving `fold_class` descended into the
  method's function body), and the sibling `3 + 4` folds to `7` — none of which
  a WHITESPACE_ONLY fallback would do.
- `tests/diff/advanced-class-constructor/`:
  `f(class { constructor() { return 1 + 2 } });` →
  `f(class{constructor(){return 3}});` at ADVANCED. This is the end-to-end
  regression for the **`constructor` no-rename guard** (PR1): rename-properties
  must never rename a class's `constructor` key (doing so would turn the
  constructor into an ordinary method and break `new C()`). The output keeps
  `constructor` verbatim, and the body folds — proving both that the guard held
  and that the ADVANCED pipeline actually ran over the class.

**Known limitation (grammar):** the grammar requires an explicit `;` *between*
class members, so an un-separated multi-member class (`class { m(){} n(){} }`)
is a parse error and falls back to WHITESPACE_ONLY. Single-member classes
minify cleanly. Computed-key / generator / `async` methods are declined at the
bridge (a later slice) and likewise fall back — never a miscompile.

## [0.234.11] - 2026-07-08

### Added — CLOC12.172 PR2: regex literals `/pat/flags` flow through the SIMPLE pipeline (gap-RegExpAsIdentifier)

With the `javascript-parser` 0.36.0 bridge now building a real `RegExpLiteral`
for the lexer's REGEX token (instead of mis-encoding it as an `Identifier`
named `/pat/flags`), a **regular-expression literal** minifies through the full
parser → bridge → passes → emitter pipeline rather than risking a mangled
identifier or a WHITESPACE_ONLY fallback.

New `tests/diff/simple-regex/` fixture: `f(/ab+c/gi, 1 + 2);` → `f(/ab+c/gi,3);`.
The regex `/ab+c/gi` round-trips verbatim (delimiters + both flags), proving the
bridge produced a real `RegExpLiteral` the emitter can print, **and** the sibling
argument `1 + 2` folds to `3` — together proving the SIMPLE passes walked through
the call rather than re-emitting the file verbatim.

## [0.234.10] - 2026-07-08

### Added — CLOC12.171 PR2: optional chaining `a?.b` flows through the SIMPLE pipeline

With the `javascript-parser` 0.35.0 bridge now building the optional-chain nodes
(closing gap-OptionalChain), **optional chaining** `a?.b` / `a?.[k]` / `a?.()`
(ES2020) minifies through the full parser → bridge → passes → emitter pipeline
instead of dragging the file to WHITESPACE_ONLY.

New `tests/diff/simple-optchain/` fixture: `f(a?.b, 1 + 2);` → `f(a?.b,3);`. The
`?.` link round-trips (the bridge produced a real `ChainExpression` /
`OptionalMemberExpression`) **and** the sibling argument `1 + 2` folds to `3` —
together proving the SIMPLE passes walked through the call rather than re-emitting
the file verbatim.

## [0.234.9] - 2026-07-07

### Added — CLOC12.170 PR2: object spread `{...o}` flows through the SIMPLE pipeline (gap-SpreadProperty)

With the `javascript-parser` bridge now converting `{...o}` →
`ObjectMember::Spread` (v0.34.0), a file containing an object spread no longer
falls back to WHITESPACE_ONLY — it flows through the full SIMPLE pipeline
(parser → typed-AST bridge → passes → emitter). New end-to-end diff fixture
`tests/diff/simple-objspread/` (`f({...o, x: 1 + 2});` → `f({...o,x:3});`): the
spread `...o` round-trips as the first member — proving the bridge produced a
real `ObjectMember::Spread` node — while the sibling member value `1 + 2` folds
to `3`, proving the passes walked *into* the object members rather than
re-emitting the source verbatim. Version bump only; no CLI-surface change.
(gap-SpreadProperty)

## [0.234.8] - 2026-07-07

### Added — CLOC12.169 PR2: `import(x)` flows through the SIMPLE pipeline (gap-170)

With the `javascript-parser` bridge now converting `import(x)` →
`Expression::ImportExpression` (v0.33.0), a file containing a dynamic import no
longer falls back to WHITESPACE_ONLY — it flows through the full SIMPLE pipeline
(parser → typed-AST bridge → passes → emitter). New end-to-end diff fixture
`tests/diff/simple-importexpr/` (`f(import("m"), 1 + 2);` → `f(import("m"),3);`):
`import("m")` round-trips as the first argument — proving the bridge produced a
real `ImportExpression` node (a *compound* node with a real `source` operand,
unlike the atomic `import.meta` leaf) — while the sibling argument `1 + 2` folds
to `3`, proving the pipeline ran rather than re-emitting the source verbatim.
Version bump only; no CLI-surface change. (gap-170)

## [0.234.7] - 2026-07-07

### Added — CLOC12.168 PR2: `import.meta` flows through the SIMPLE pipeline (gap-169)

With the `javascript-parser` bridge now converting `import.meta` →
`Expression::ImportMeta` (v0.32.0), a file containing `import.meta` no longer
falls back to WHITESPACE_ONLY — it flows through the full SIMPLE pipeline
(parser → typed-AST bridge → passes → emitter). New end-to-end diff fixture
`tests/diff/simple-importmeta/` (`f(import.meta, 1 + 2);` → `f(import.meta,3);`):
`import.meta` round-trips as the first argument while the second argument
`1 + 2` folds to `3`, proving the pipeline ran rather than re-emitting the
source verbatim. Version bump only; no CLI-surface change. (gap-169)


## [0.234.6] - 2026-07-07

### Added — CLOC12.167 PR2: `new.target` end-to-end fixture

The `new.target` meta-property now flows through the full SIMPLE pipeline (via
the javascript-parser 0.31.0 bridge) instead of falling back to WHITESPACE_ONLY
(gap-168, now closed). New e2e diff fixture `tests/diff/simple-newtarget/`
proving `f(new.target, 1 + 2);` → `f(new.target,3);` at SIMPLE: the `new.target`
meta-property round-trips (the bridge produced a real `NewTarget` node) and the
argument `1 + 2` folds to `3` (a WHITESPACE_ONLY fallback would leave
`f(new.target, 1 + 2)` unfolded). Version bumped to 0.234.6 (cli.spec.json +
help-markdown fixture synced).

## [0.234.5] - 2026-07-04

### Added — CLOC12.166 PR2: `super` end-to-end fixture

The `super` keyword now flows through the full SIMPLE pipeline (via the
javascript-parser 0.30.0 bridge) instead of falling back to WHITESPACE_ONLY
(gap-167, now closed). New e2e diff fixture `tests/diff/simple-super/` proving
`super.f(1 + 2);` → `super.f(3);` at SIMPLE: the `super` receiver round-trips
(the bridge produced a real `Super` node) and the argument `1 + 2` folds to `3`
(a WHITESPACE_ONLY fallback would leave `super.f(1 + 2)` unfolded). Version
bumped to 0.234.5 (cli.spec.json + help-markdown fixture synced).


## [0.234.4] - 2026-07-04

### Added — CLOC12.165 PR2: `this` end-to-end fixture

The `this` keyword now flows through the full SIMPLE/ADVANCED pipeline (via the
javascript-parser 0.29.0 bridge) instead of falling back to WHITESPACE_ONLY
(gap-166, now closed). New e2e diff fixture `tests/diff/simple-this/` proving
`this.f(1 + 2);` → `this.f(3);` at SIMPLE: the `this` receiver round-trips
(the bridge produced a real `ThisExpression`) and the argument `1 + 2` folds to
`3` (a WHITESPACE_ONLY fallback would leave `this.f(1 + 2)` unfolded).

## [0.234.3] - 2026-07-04

### Added — CLOC12.163 PR2: generator/`yield` end-to-end fixture

Generator functions and `yield` now flow through the full SIMPLE/ADVANCED
pipeline (via the javascript-parser 0.28.0 bridge) instead of falling back to
WHITESPACE_ONLY. New e2e diff fixture `tests/diff/simple-yield/` proving
`use(function*(){yield 1 + 2;})` → `use(function*(){yield 3})` at SIMPLE: the
generator prints as `function*`, the `yield` round-trips, and the operand
`1 + 2` folds to `3` (a WHITESPACE_ONLY fallback would leave `yield 1+2`
verbatim). Version-sync: `cli.spec.json` and the `--help_markdown` fixture
bumped to 0.234.3.

## [0.234.2] - 2026-07-03

### Added — CLOC12.162 PR2: spread end-to-end fixture

New `tests/diff/simple-spread/` fixture (+ `diff_simple_spread.rs` driver)
proving a spread `...arg` now flows through the full SIMPLE pipeline via the
javascript-parser 0.27.0 bridge (closes gap-163). The input
`log(...a, 1 + 2);` minifies to `log(...a,3);`: the spread `...a` round-trips
(the bridge produced a real `SpreadElement` rather than declining) while the
sibling `1 + 2` folds to `3` — proving the whole file ran through SIMPLE rather
than falling back to WHITESPACE_ONLY. Version bumped to 0.234.2 with
`cli.spec.json` and the `help-markdown` fixture kept in sync.

## [0.234.1] - 2026-07-03

### Added — CLOC12.161 PR2: tagged-template end-to-end fixture

New `tests/diff/simple-tagged-template/` e2e diff fixture proving a tagged
template `` tag`abc` `` now flows through the full SIMPLE pipeline (gap-162
closed in `javascript-parser` 0.26.0). Input `log(tag`abc`, 1 + 2);` →
`log(tag`abc`,3);`: the tagged template round-trips AND the sibling `1 + 2`
folds to `3` — the fold proves the whole file ran through SIMPLE rather than
falling back to WHITESPACE_ONLY (which a bridge decline would have forced,
leaving `1 + 2` unfolded). Driver: `tests/diff_simple_tagged_template.rs`.


## [0.234.0] - 2026-06-30

### Fixed — deep-grouping parser DoS no longer aborts the process

Deeply nested grouping in untrusted input (`x=((((…))))` with thousands of
parens, or long unary chains `x=----…a`) previously overflowed the native stack
at `--compilation_level SIMPLE`/`ADVANCED` — an uncatchable abort. closurec now
parses via `coding-adventures-javascript-parser` 0.19.11, which opts into the
`parser` recursion-depth guard at its ASI parse sites; such input returns a
clean parse error, and closurec degrades it to WHITESPACE_ONLY (still valid
output) exactly as it already does for any other parse failure. Verified:
5000-deep parens now emit `x=1;` (exit 0), 10000-deep unary emits cleanly, and
normal JS is byte-identical.

Known remaining: a very deep *flat* expression (`x=1+1+…+1` with ~20000 terms)
still overflows a separate **downstream** AST-traversal stage (the parser itself
survives it — `--print_tree` succeeds). Tracked as a distinct follow-up.

## [0.233.0] - 2026-06-30

### Fixed — `**` operand precedence emitted invalid JS (miscompile)

A unary base of `**` was emitted without its required parentheses — `(-a)**2`
became `-a**2`, a `SyntaxError` (also `(~a)**2`, `(!a)**2`). And right-associativity
was not modelled, so `a**b**c` was over-parenthesised to `a**(b**c)`. The emitter
now parenthesises a unary/lower-precedence `**` base and leaves a same-precedence
right operand bare (see `closure-emitter` 0.18.7). New end-to-end test
`simple_exponentiation_operand_precedence`.

## [0.232.0] - 2026-06-30

### Fixed — member object / call callee lost required parens (miscompile)

At SIMPLE/ADVANCED, a parenthesised lower-precedence object of a member
expression (or callee of a call) dropped its parentheses, changing semantics:
`(a||b).c` became `a||b.c` (i.e. `a||(b.c)`); likewise `(a=b).c`, `(a?b:c).d`,
`(a+b).c`, `(-a).b`, `(a||b)()`, `(a=b)(c)`. The emitter now writes the object /
callee at `PREC_PRIMARY` so the parens survive while `a.b.c` / `f().x` / `a.b()`
stay bare (see `closure-emitter` 0.18.6). New end-to-end test
`simple_member_and_call_object_keeps_required_parens`.

## [0.231.0] - 2026-06-30

### Changed — binary/logical operators emit tight at SIMPLE/ADVANCED

Minified output no longer pads symbolic operators with spaces: `x=a + b;`
becomes `x=a+b;`, `a && b` → `a&&b`, `a << b` → `a<<b`, `a === b` → `a===b`,
matching upstream Closure and the existing WHITESPACE_ONLY path. `in` /
`instanceof` keep their mandatory spaces, and the additive `+`/`-` sign hazard
keeps one space where needed (`a+ +b`, not `a++b`). See `closure-emitter`
0.18.5 for the full rule. Diff fixtures and the ADVANCED big-pass golden are
regenerated to the tighter output; new end-to-end test
`simple_binary_operator_spacing`.

## [0.230.0] - 2026-06-30

### Fixed — array elisions (holes) dropped at SIMPLE/ADVANCED (miscompile)

Sparse array literals lost their holes, changing the array's length and index
membership:

| input         | was        | now (correct) |
|---------------|------------|---------------|
| `f([1,,3])`   | `f([1,3])` | `f([1,,3])`   |
| `f([,,])`     | `f([])`    | `f([,,])`     |
| `f([1,,])`    | `f([1])`   | `f([1,,])`    |
| `f([,1])`     | `f([1])`   | `f([,1])`     |
| `f([1,,,2])`  | `f([1,2])` | `f([1,,,2])`  |
| `f([1,2,3,])` | `f([1,2,3])` | `f([1,2,3])` (trailing comma, no hole) |

Two-part fix: the bridge (`javascript-parser` 0.19.4) now walks the raw
`element_list` children so the elision commas are visible and produces a `None`
per hole; the emitter (`closure-emitter` 0.18.4) appends the extra comma a
trailing hole needs. New end-to-end test `simple_array_elisions_preserved`.

## [0.229.0] - 2026-06-29

### Fixed — quoted object property keys miscompiled at SIMPLE/ADVANCED

Every quoted object-literal key was emitted as a bare identifier built from the
key's un-decoded text, because the bridge recognised STRING/NUMBER keys via the
wrong token field. That produced invalid or semantically-different output:

| input                  | was                         | now (correct)        |
|------------------------|-----------------------------|----------------------|
| `f({"a-b":1})`         | `f({a-b:1})` — SyntaxError   | `f({"a-b":1})`       |
| `f({"a b":1})`         | `f({a b:1})` — SyntaxError   | `f({"a b":1})`       |
| `f({"x\ty":1})`        | `f({x\ty:1})` — invalid      | `f({"x\ty":1})`      |
| `f({"__proto__":1})`   | `f({__proto__:1})` — proto setter | `f({"__proto__":1})` |
| `f(Object.entries({"x\ty":1}))` | `f([["x\\ty",1]])` — double-escaped | `f([["x\ty",1]])` |
| `f({"abc":1})`         | `f({abc:1})`                | `f({abc:1})` (kept) |

The fix spans three crates: the bridge (`javascript-parser` 0.19.3) now parses
keys via `t.type_` and decodes them; the emitter (`closure-emitter` 0.18.2)
drops a key's quotes only when its decoded value is a valid identifier and not
`__proto__`; and the `Object.keys` fold (`closure-pass-constant-fold` 0.76.0)
drops its now-unnecessary escaped-key decline. New end-to-end test
`simple_object_string_keys_quote_handling` locks the behaviour.

## [0.228.0] - 2026-06-29

### Added — per-fold CV provenance reaches the SIMPLE sidecar (CLOC27 P4 + P5)

The headline "tracing" guarantee now holds end-to-end on the SIMPLE pipeline:
running `report("abc".length);` at `--compilation_level SIMPLE
--correlation_vector` emits `report(3);`, and the folded `3` is traceable
through the correlation-vector sidecar back to the exact `"abc"` source span
(line 1, column 8). Previously this lineage dead-ended — the pass pipeline ran
against a *disabled, discarded* `CVLog` and the bridge leaves carried `cv:
None`, so the sidecar recorded only coarse lex/file/pass-summary origins.

- **D5 — wire the run's real CVLog on the SIMPLE cv-on path.**
  `transform_source_with_cv` now, when `--correlation_vector` is set, parses via
  `javascript_parser::parse_javascript_typed_with_cv` (so every leaf literal
  carries its source token's CvId — CLOC27 P2/P3) and threads the run's real
  enabled `CVLog` into `run_typed_pipeline`. The constant-fold pass therefore
  `derive`s each folded literal from its leaf's source CvId, landing real
  per-token provenance in the sidecar. The input file's display name is passed
  as the per-token `Origin.source`, so a fold traces to `<file>:line:col`.
- **Disabled path unchanged.** With CV off, `run_typed_pipeline` falls back to
  an internal disabled `CVLog` and parsing uses the zero-overhead
  `parse_javascript_typed` — byte-identical output, since CV ids never affect
  folding or emission.
- **Tests.** `cv_fold_provenance_gap` is FLIPPED from pinning the *absence* of
  per-fold lineage to asserting its *presence* (per-token source-span origins
  now appear, coexisting with the coarse origins) — that flip is the signal
  tracing became real. New golden trace `cv_fold_trace` nails the exact link:
  the folded `3` traces to the `"abc"` span at `1:8`.

## [0.227.0] - 2026-06-29

### Added — `advanced-bigpass` end-to-end ADVANCED proof (size + runtime equivalence)

A new test-only fixture (`tests/diff/advanced-bigpass/`) and integration test
(`tests/diff_advanced_bigpass.rs`) that prove the whole ADVANCED pipeline
cooperates on a realistic four-function geometry module, shrinking it from a
195-byte `WHITESPACE_ONLY` baseline to 56 bytes (~71%) **without changing
observable behaviour**:

```text
function f(x){return x * 10};report(12,25,f(7));sink(f);
```

The single output line exhibits four passes — dead-code elimination
(`unusedPerimeter` tree-shaken), single-use inlining + constant folding
(`area(3,4)`→`12`, `hypotSq(3,4)`→`25`, the runtime-equivalence anchors), the
ADVANCED-only global rename (`scale`→`f`, contrasted against SIMPLE which keeps
`scale`), and live-reference retention (`f(7)` + `sink(f)`). Size savings are
measured against `WHITESPACE_ONLY` (not the raw source) so the shrink reflects
optimization, not comment stripping. No production-code or CLI-surface change.
## [0.226.0] - 2026-06-29

### Added — SIMPLE/ADVANCED fold static `Object.keys({k: v, …})` → key-string array

At `--compilation_level SIMPLE` (and `ADVANCED`, which routes through the same
typed pipeline) `Object.keys` of a fully-static NON-EMPTY object literal now
folds to the array of its own-enumerable string keys, e.g.
`Object.keys({a:1,b:2})` → `["a","b"]`. The empty-object case
(`Object.keys/values/entries({})` → `[]`) was already handled; this extends the
existing `simple-fold-object-keys` fixture to cover the non-empty key-array fold
and three declines (non-empty `Object.values`, an integer-index-keyed object,
and an array). Folding requires `coding-adventures-closure-pass-constant-fold`
0.75.0. No CLI surface change.
## [0.225.0] - 2026-06-29

### Added — characterization test pinning the per-fold tracing gap (`tests/cv_fold_provenance_gap.rs`)

A test that documents the current `--correlation_vector` contract at the
constant-fold layer: the constant-fold pass runs (listed in the coarse
`compilation_level/simple_v2` contribution), but the emitted sidecar carries NO
per-fold provenance — every CV entry origin is lex/file-level, so a folded
literal (`"abc".length` → `3`) cannot be traced back to its source bytes. The
per-fold lineage each fold records via `fork_cv`/`stamp_literal_cv` is dropped at
the SIMPLE bridge boundary (the typed AST nodes carry `cv: None` and
`run_typed_pipeline` runs the pipeline with a disabled, discarded `CVLog`). This
locks the gap so it is visible and regression-detectable: when per-fold
provenance is wired through the bridge, this test's gap assertion flips and
signals that the fold lineage assertion should be promoted. No production code
change.

## [0.224.0] - 2026-06-27

### Added — differential **soundness** conformance harness (`tests/conformance.rs`)

A new test harness that checks the SIMPLE optimizer is **value-preserving**, not
just byte-stable. For a corpus of source expressions it runs closurec, parses
the optimized output with a self-contained literal evaluator, and asserts the
folded value equals the expression's true runtime value (a canonical,
`Object.is`-faithful golden generated offline by Node/V8 — **CI runs no JS
engine**). Numbers reuse closurec's V8-faithful `format_js_number`, so the
canonical form is the raw token (no float-formatting mismatch). Declined/partial
outputs have no literal value to check and are counted as `skipped` with a loud
end-of-test summary (no silent coverage holes).

This is the soundness net that byte fixtures can't provide: it value-checks
21/23 seed entries against the oracle and pins the two known negative-zero
divergences (`KNOWN_DIVERGENCES`) so the day the underlying unary-minus `-0`→`0`
flattening bug is fixed, the test tells us to promote those entries. Seed corpus
covers numbers, string methods, `split`, `String.fromCharCode`, `Number.is*`,
`Array.isArray`/`of`, `Object.keys`/`entries`/`fromEntries`, `Boolean`/`String`/
`Number`, and `isNaN`/`isFinite`. Generator + docs under `tests/conformance/`.
## [0.223.0] - 2026-06-27

### Added — SIMPLE/ADVANCED fold static `Math.max(…)` / `Math.min(…)` → numeric

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static `Math.max(…)` or
`Math.min(…)` call (ECMAScript §21.3.2.24 / .25) to a numeric literal when there
is at least one argument and every argument is a numeric literal (e.g.
`Math.max(1, 2, 3)` → `3`, `Math.min(-5, -1)` → `-5`). Signed zero follows the
spec exactly (`max` prefers `+0`, `min` prefers `-0`). We decline a non-literal
argument (a runtime value could be `Infinity`/`NaN` or otherwise unknown), the
empty call (`Math.max()` → `-Infinity`), and a non-global receiver
(`m.max(...)`). New end-to-end fixture `tests/diff/simple-fold-math-max-min/`
plus integration test `tests/diff_simple_fold_math_max_min.rs`.

## [0.222.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Array.from("…")` → array of code-point strings

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static `Array.from(x)`
call (ECMAScript §23.1.2.1) to an array literal when `x` is a string literal and
there is no `mapFn`. The string iterator yields one element per code point (like
spread), so `Array.from("abc")` → `["a","b","c"]`, `Array.from("")` → `[]`, and
astral characters stay whole. We decline a second `mapFn` argument, any
non-string-literal first argument, and a shadowed receiver. New end-to-end
fixture `tests/diff/simple-fold-array-from/` plus integration test
`tests/diff_simple_fold_array_from.rs`.
## [0.221.0] - 2026-06-27

### Added — SIMPLE/ADVANCED fold static `Object.entries({…})` → array of pairs

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static
`Object.entries({k: v, …})` call (ECMAScript §20.1.2.5) — the inverse of
`Object.fromEntries` — to an array of `[key, value]` pair literals when the
argument is a non-empty object literal of plain data properties with
primitive-literal values (string / number / boolean / null). Each entry key is
emitted as a string literal. We decline a `"__proto__"` key (the object-literal
form is the §B.3.1 prototype setter, not an own property), any canonical
array-index key (`0`, `1`, `42`, … — enumerated ahead of string keys, which
would reorder the result), any non-literal value (including a shorthand `{x}`),
getters / setters / methods / computed keys, a non-global receiver, and arity
≠ 1; duplicate keys collapse to one entry (first position, last value). The
empty-object case `Object.entries({})` → `[]` was already handled. New
end-to-end fixture `tests/diff/simple-fold-object-entries/` plus integration
test `tests/diff_simple_fold_object_entries.rs`.
## [0.220.0] - 2026-06-27

### Added — SIMPLE/ADVANCED fold static `Object.fromEntries(...)` → object literal

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static
`Object.fromEntries([[k, v], …])` call (ECMAScript §20.1.2.7) — the inverse of
`Object.entries` — to an object literal when its single argument is a static
array of 2-element `[key, value]` array literals, each key a string or numeric
literal (numeric → ToString) and each value a primitive literal (string /
number / boolean / null). Identifier-name keys emit bare (`{a: 1}`), other keys
quoted (`{"1": "x"}`); a duplicate key keeps its first position but takes its
last value; the empty array folds to `{}`. We decline a non-global receiver
(`o.fromEntries(...)`), wrong arity, a non-array argument, a non-pair element, a
non-literal/boolean/null/identifier key, a non-literal value, any array hole,
and a `"__proto__"` key (whose own-property semantics differ from the object
literal's prototype setter). New end-to-end fixture
`tests/diff/simple-fold-object-fromentries/` plus
integration test `tests/diff_simple_fold_object_fromentries.rs`.

## [0.219.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Object.is(a, b)` → boolean (SameValue)

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static `Object.is(a, b)`
call (ECMAScript §20.1.2.13) to a boolean literal when both arguments are
primitive literals. `Object.is` uses SameValue, which differs from `===` only at
`Object.is(NaN, NaN)` (`true`) and `Object.is(+0, -0)` (`false`). We fold two
number literals (NaN is the same as NaN; +0 and −0 distinguished by sign), two
string literals, two booleans, two `null`s, and a literal-kind mismatch
(`false`); we decline when either operand is a non-literal (including the bare
global `NaN` identifier) or the arity is not two. New end-to-end fixture
`tests/diff/simple-fold-object-is/` plus integration test
`tests/diff_simple_fold_object_is.rs`.
## [0.217.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Number.isSafeInteger(x)` → boolean

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static
`Number.isSafeInteger(x)` call (ECMAScript §21.1.2.5) to a boolean literal,
alongside the existing `Number.isInteger` / `isFinite` / `isNaN`. It returns
`true` only for an integer whose magnitude does not exceed 2^53−1
(`Number.MAX_SAFE_INTEGER` = 9007199254740991), does no coercion (a non-Number
literal is `false`), and declines an identifier / non-literal. New end-to-end
fixture `tests/diff/simple-fold-number-issafeinteger/` plus integration test
`tests/diff_simple_fold_number_issafeinteger.rs`.
## [0.216.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Array.of(...)` → array literal `[...]`

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses a static `Array.of(v0, v1,
…)` call (ECMAScript §23.1.2.3) to the array literal `[v0, v1, …]`. Unlike the
`Array(n)` constructor — where a single numeric argument sets the *length* —
`Array.of(7)` is the one-element array `[7]`, so the fold is sound for any
argument list and preserves every element expression in evaluation order. Only
the bare global `Array.of(...)` callee folds (never a shadowed `q.of(...)`).
New end-to-end fixture `tests/diff/simple-fold-array-of/` plus integration test
`tests/diff_simple_fold_array_of.rs`.

## [0.214.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `JSON.stringify(…)` → string

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the static `JSON.stringify(x)`
(ECMAScript §25.5.2) to a string literal for the primitive literal arguments — via
the new `MemberExpression`-arm dispatch in `closure-pass-constant-fold` 0.57.0.

It folds the single-argument form for a NUMBER literal (`JSON.stringify(42)` →
`"42"`, reusing `fold_string_of_number` so fractional/≥2⁵³ values decline), a
BOOLEAN literal (`"true"`/`"false"`), and the NULL literal (`"null"`). A STRING
literal is declined (JSON escaping left to the runtime), as are array/object
literals (side effects + recursion), identifiers, and any call with a second
`replacer`/`space` argument. Only the bare global `JSON.stringify(...)` folds.

New end-to-end fixture `tests/diff/simple-fold-json-stringify/` and integration
test `tests/diff_simple_fold_json_stringify.rs` assert byte-exact SIMPLE output,
the per-binding string folds, the declined string + fractional calls, and a
WHITESPACE_ONLY-fallback regression guard (exactly two `JSON.stringify(` calls
remain). Help-markdown regenerated to Version 0.206.0.
## [0.211.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Array.isArray(…)` → boolean

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the static `Array.isArray(x)`
(ECMAScript §22.1.2.2) to a boolean literal — via the new `MemberExpression`-arm
dispatch in `closure-pass-constant-fold` 0.54.0.

It folds the literal shapes with no side effect to drop: `Array.isArray([])` →
`true`, `Array.isArray({})` → `false`, and a primitive literal
(`Array.isArray("x")`/`(42)`/`(true)`/`(null)`) → `false`. A **non-empty**
array/object literal is declined (`Array.isArray([1,2])` is left intact, since
folding would discard its element evaluation and any side effect), as is an
identifier or non-literal argument. Only the bare global `Array.isArray(...)`
folds — never a shadowed receiver.

New end-to-end fixture `tests/diff/simple-fold-array-isarray/` and integration
test `tests/diff_simple_fold_array_isarray.rs` assert byte-exact SIMPLE output,
the per-binding boolean folds (incl. empty-array `true` and primitive/object
`false`), the declined non-empty array, and a WHITESPACE_ONLY-fallback regression
guard (exactly one `Array.isArray(` call remains). Help-markdown regenerated to
Version 0.203.0.
## [0.209.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold global `isNaN(…)` / `isFinite(…)` → boolean

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the global numeric
predicates `isNaN(x)` / `isFinite(x)` (ECMAScript §19.2.3 / §19.2.2) to a boolean
literal when the single argument is a string- or number-literal — via the new
`js_to_number` ToNumber classifier in `closure-pass-constant-fold` 0.51.0.

Both coerce with `ToNumber` then classify: `isNaN("abc")` → `true`,
`isNaN("42")` → `false`, `isNaN(" ")` → `false` (`ToNumber(" ")` is `+0`),
`isFinite("1e3")` → `true`, `isFinite("Infinity")` → `false`, `isFinite(0)` →
`true`. Unlike `Number(...)`, no shape declines — every string has a
well-defined NaN / Infinity / finite class.

New end-to-end fixture `tests/diff/simple-fold-isnan/` and integration test
`tests/diff_simple_fold_isnan.rs` assert byte-exact SIMPLE output, the
per-binding boolean folds (including the `ToNumber(" ")=+0` and `"Infinity"`
cases), and a WHITESPACE_ONLY-fallback regression guard (zero `isNaN(` / zero
`isFinite(` calls remain). Help-markdown regenerated to Version 0.200.0.
## [0.208.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Object.keys/values/entries({})` → `[]`

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the static `Object.keys(x)`
/ `Object.values(x)` / `Object.entries(x)` (ECMAScript §20.1.2.16/.22/.5) to the
empty array literal `[]` when the single argument is an empty object literal `{}`
— via the new `MemberExpression`-arm dispatch in `closure-pass-constant-fold`
0.59.0.

An empty object has no own enumerable keys and `{}` has no side effect, so `[]`
is exact for all three methods. A non-empty object literal is declined (its
property values may have side effects, and the result is non-empty), as are
arrays, primitives, identifiers, and any call with ≠1 argument. Only the bare
global `Object.keys/values/entries(...)` folds, never a shadowed receiver.

New end-to-end fixture `tests/diff/simple-fold-object-keys/` and integration test
`tests/diff_simple_fold_object_keys.rs` assert byte-exact SIMPLE output, the
three `[]` folds, the declined non-empty-object and array calls, and a
WHITESPACE_ONLY-fallback regression guard (exactly two `Object.` calls remain).
Help-markdown regenerated to Version 0.208.0.

## [0.204.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold legacy global `escape(…)` / `unescape(…)`

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the legacy global string
escapers `escape(str)` / `unescape(str)` (ECMAScript Annex B §B.2.1.1 / §B.2.1.2)
to a string literal when the single argument is a string literal — via the new
`escape_js` / `unescape_js` helpers in `closure-pass-constant-fold` 0.50.0.

These operate on UTF-16 **code units** (not the UTF-8 bytes the `…URI` encoders
use): `escape("a b")` → `"a%20b"`, `escape("~/@")` → `"%7E/@"` (`~` escaped, but
`/` and `@` are unescaped marks), `escape("é")` → `"%E9"`, `escape("😀")` →
`"%uD83D%uDE00"`; `unescape` is the inverse, decoding every escape (so
`unescape("%2F")` → `"/"`, unlike `decodeURI`). `unescape` declines (the call is
left intact) only when its result would contain an unpaired surrogate
(`unescape("%uD83D")`).

New end-to-end fixture `tests/diff/simple-fold-escape/` and integration test
`tests/diff_simple_fold_escape.rs` assert byte-exact SIMPLE output, the
per-binding folds (including the `%uXXXX` astral case and the declined
unpaired-surrogate call), and a WHITESPACE_ONLY-fallback regression guard (zero
`escape(` calls, exactly one `unescape(` call remain). Help-markdown regenerated
to Version 0.199.0.
## [0.202.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Number.parseInt/parseFloat(…)`

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the ES2015 static methods
`Number.parseInt(string[, radix])` / `Number.parseFloat(string)` (ECMAScript
§21.1.2.12/.13) to a numeric literal — via the new `MemberExpression`-arm
dispatch in `closure-pass-constant-fold` 0.53.0.

These are the *same functions* as the global `parseInt`/`parseFloat`
(`Number.parseInt === parseInt`), so the fold reuses the existing
`fold_parse_int`/`fold_parse_float` helpers: `Number.parseInt("12px")` → `12`,
`Number.parseInt("FF", 16)` → `255`, `Number.parseInt("0x1F")` → `31`,
`Number.parseFloat("3.14e2abc")` → `314`. Only the bare global `Number.parseX(...)`
folds (never a shadowed receiver), and a `NaN`/`±Infinity` result is declined
(`Number.parseInt("")` is left intact).

New end-to-end fixture `tests/diff/simple-fold-number-parse/` and integration
test `tests/diff_simple_fold_number_parse.rs` assert byte-exact SIMPLE output,
the per-binding numeric folds (incl. an explicit radix and the `0x` prefix), the
declined `NaN` call, and a WHITESPACE_ONLY-fallback regression guard (exactly one
`Number.parse…(` call remains). Help-markdown regenerated to Version 0.202.0.
## [0.201.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold static `Number.isInteger/isFinite/isNaN(…)`

At `--compilation_level SIMPLE` (and ADVANCED, which routes through the same
pipeline) the typed constant-fold pass now collapses the ES2015 static numeric
predicates `Number.isInteger(x)` / `Number.isFinite(x)` / `Number.isNaN(x)`
(ECMAScript §21.1.2.2/.3/.4) to a boolean literal — via the new
`MemberExpression`-arm dispatch in `closure-pass-constant-fold` 0.52.0.

Unlike the *global* `isNaN`/`isFinite`, these do **no** coercion: a NUMBER
literal classifies its value directly (`Number.isInteger(42)` → `true`,
`Number.isInteger(3.5)` → `false`, `Number.isInteger(1e21)` → `true`,
`Number.isFinite(42)` → `true`, `Number.isNaN(42)` → `false`), while a STRING /
BOOLEAN / NULL literal is provably not a Number and folds to `false`
(`Number.isInteger("42")` → `false`, `Number.isFinite(null)` → `false`). Only
the bare global `Number.isX(...)` folds — never a shadowed receiver.

New end-to-end fixture `tests/diff/simple-fold-number-static/` and integration
test `tests/diff_simple_fold_number_static.rs` assert byte-exact SIMPLE output,
the per-binding boolean folds (incl. the no-coercion `isInteger("42")` and
large-integer `isInteger(1e21)` cases), and a WHITESPACE_ONLY-fallback
regression guard (zero `Number.is…(` calls remain). Help-markdown regenerated to
Version 0.201.0.

## [0.198.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold global `encodeURI(…)` / `decodeURI(…)` → string

Bumps `closure-pass-constant-fold` to 0.49.0, which folds the global whole-URI
escapers `encodeURI(string)` / `decodeURI(string)` whose single argument is a
string literal to the string literal V8 produces at runtime (ECMAScript
§19.2.6.4 / §19.2.6.2). They are the whole-URI siblings of `encodeURIComponent`
/ `decodeURIComponent`, differing only in their treatment of the URI
reserved/structural delimiters `; , / ? : @ & = + $` and `#`:

- `encodeURI` keeps those reserved delimiters unescaped (it escapes only the
  genuinely unsafe bytes — space, non-ASCII, controls, and `< > " { } | \ ^ [ ]`
  `` ` ``): `encodeURI("a b")` → `"a%20b"`, `encodeURI("a/b?c=d")` → `"a/b?c=d"`,
  `encodeURI("é")` → `"%C3%A9"`.
- `decodeURI` keeps a `%XX` escape ENCODED when its byte is a reserved delimiter,
  so reserved structure survives a round trip — the one behavioural difference
  from `decodeURIComponent`: `decodeURI("a%20b")` → `"a b"`, but `decodeURI("%2F")`
  → `"%2F"` (whereas `decodeURIComponent("%2F")` → `"/"`).

Folds the **bare global identifier** only (never `window.encodeURI`); `decodeURI`
DECLINES the fold on the two `URIError` inputs (malformed `%XX` escape, or a
`%`-decoded byte run that is not valid UTF-8), so a runtime throw is never
folded into a value.

Adds the `simple-fold-uri` diff fixture and `diff_simple_fold_uri.rs` integration
test (byte-exact stdout, per-binding folds including the reserved-preservation
distinction and the declined `URIError` call, and a WHITESPACE_ONLY-fallback
regression guard). Regenerates the `--help_markdown` golden for the version bump.
## [0.197.0] - 2026-06-26

### Added — global `encodeURIComponent` / `decodeURIComponent` folding observable end-to-end at SIMPLE

Bumps `closure-pass-constant-fold` to **0.48.0**, which folds a global
`encodeURIComponent(str)` / `decodeURIComponent(str)` call on a string literal
to the string literal V8 produces (ECMAScript §19.2.6.5 / §19.2.6.3),
declining only the `decodeURIComponent` inputs that would throw a `URIError`.

New end-to-end fixture `tests/diff/simple-fold-uricomponent/` proves the fold
is observable through the whole closurec SIMPLE pipeline:

```js
// input
var a = encodeURIComponent("a b");
var b = encodeURIComponent("é");
var c = encodeURIComponent("/");
var d = decodeURIComponent("a%20b");
var e = decodeURIComponent("%C3%A9");
var f = decodeURIComponent("%E0");
report(a, b, c, d, e, f);
// SIMPLE output
var a="a%20b";var b="%C3%A9";var c="%2F";var d="a b";var e="é";var f=decodeURIComponent("%E0");report(a,b,c,d,e,f);
```

`encodeURIComponent` percent-escapes every non-unreserved UTF-8 byte (the URI
reserved delimiters `/` etc. ARE escaped, unlike `encodeURI`);
`decodeURIComponent` reverses it. The truncated multi-byte input `"%E0"` is an
invalid UTF-8 byte run on which JS throws `URIError`, so `f`'s call is left
intact — a runtime throw is never folded into a value. Three diff-integration
tests cover the byte-exact stdout, the per-binding folds, and a regression
guard that exactly one (declined) call survives, proving the typed SIMPLE
optimizer ran rather than the `WHITESPACE_ONLY` fallback.
## [0.196.0] - 2026-06-26

### Added — SIMPLE/ADVANCED fold global `Boolean(…)` → boolean literal

Bumps `closure-pass-constant-fold` to 0.47.0, which folds a global
`Boolean(value)` call whose single argument is a string or number literal to a
boolean literal (the `ToBoolean` coercion, ECMAScript §7.1.2):

```js
// in
var a = Boolean(""), b = Boolean("x"), c = Boolean("0"),
    d = Boolean(0), e = Boolean(1), f = Boolean(z);
// out (SIMPLE)
var a=false,b=true,c=true,d=false,e=true,f=Boolean(z);
```

A string is falsy only when empty (`Boolean("0")` → `true`, a non-empty string);
a number is falsy only for `0`/`-0`. Any non-string/number-literal argument (a
boolean, `null`, an identifier like `Boolean(z)`, a second argument) or a
non-bare callee (`window.Boolean(...)`) is left for the runtime. New fixture
`tests/diff/simple-fold-boolean/` and integration test
`tests/diff_simple_fold_boolean.rs` cover it end-to-end.
## [0.195.0] - 2026-06-25

### Added — SIMPLE/ADVANCED fold global `String(…)` → string literal

Bumps `closure-pass-constant-fold` to 0.46.0, which folds a global
`String(value)` call whose single argument is a string or **integer** number
literal to a string literal (ECMAScript §22.1.3.1 → §7.1.17 `ToString`):

```js
// in
var a = String(42), b = String("x"), c = String(-3),
    d = String(255), e = String(0.5);
// out (SIMPLE)
var a="42",b="x",c="-3",d="255",e=String(0.5);
```

A string argument is the identity; an integer renders as its decimal text.
`String(0.5)` is **not** folded — a fractional number is declined because Rust's
and V8's shortest-decimal formatters can break an exact binary tie in opposite
directions, which would silently mis-fold. Any fractional or `≥ 2^53` number,
or any non-bare callee like `window.String(...)`, is left for the runtime. New
fixture `tests/diff/simple-fold-string/` and integration test
`tests/diff_simple_fold_string.rs` cover it end-to-end.

## [0.194.0] - 2026-06-25

### Added — SIMPLE/ADVANCED fold global `Number("…")` → numeric

Bumps `closure-pass-constant-fold` to 0.45.0, which folds a global
`Number(string)` call whose single argument is a string literal to the numeric
literal V8 produces at runtime (ECMAScript §21.1.1.1 → §7.1.4.1.1
`StringToNumber`). Unlike `parseInt`/`parseFloat`, the coercion is **total** —
the whole trimmed string must be numeric or the result is `NaN`:

```js
// in
var a = Number("42"), b = Number(""), c = Number("  3.5 "),
    d = Number("0x1F"), e = Number("0b101"), f = Number("0o17"),
    g = Number("abc");
// out (SIMPLE)
var a=42,b=0,c=3.5,d=31,e=5,f=15,g=Number("abc");
```

`Number("")` → `0` (the empty string is `+0`, not `NaN`), the `0x`/`0b`/`0o`
forms fold to their integer value, and a leading zero stays decimal
(`Number("017")` → `17`). Calls whose result has no literal token — `NaN`
(`Number("abc")`, `Number("12px")`, `Number("1,2")`) or `±Infinity`
(`Number("Infinity")`) — are left for the runtime, as is any non-bare callee
(`window.Number(...)`). New fixture `tests/diff/simple-fold-number/` and
integration test `tests/diff_simple_fold_number.rs` cover it end-to-end.
## [0.193.0] - 2026-06-25

### Added — static `String.fromCodePoint(...)` folding observable end-to-end at SIMPLE

Picks up `closure-pass-constant-fold` 0.44.0, which folds the static
`String.fromCodePoint(cp0, cp1, …)` into a string literal when every argument is
a non-negative integer literal that is a valid Unicode scalar (`0..=0x10FFFF`,
not a surrogate) — ECMAScript §22.1.2.2. Each argument is a whole code point
(unlike `fromCharCode`'s UTF-16 units), so a single astral argument suffices.

New `tests/diff/simple-fold-fromcodepoint/` fixture + `diff_simple_fold_fromcodepoint.rs`
integration test assert the SIMPLE-level output
`var a="HI";var b="💩";var c="💩A";report(a,b,c);` for
`String.fromCodePoint(72,73)` → `"HI"`, `String.fromCodePoint(128169)` → `"💩"`
(a single astral arg, emitted as its escaped surrogate pair), and
`String.fromCodePoint(128169,65)` → `"💩A"`, and guard that no `fromCodePoint`
call survives (proving the typed SIMPLE pipeline ran, not the WHITESPACE_ONLY
fallback).
## [0.192.0] - 2026-06-25

### Added — static `String.fromCharCode(...)` folding observable end-to-end at SIMPLE

Picks up `closure-pass-constant-fold` 0.43.0, which folds the static
`String.fromCharCode(u0, u1, …)` into a string literal when every argument is a
non-negative integer literal in `0..=0xFFFF` (ECMAScript §22.1.2.1) — the first
fold whose receiver is the bare global `String` rather than a string/number
literal. The arguments are UTF-16 code units, so an adjacent high+low surrogate
pair assembles an astral scalar.

New `tests/diff/simple-fold-fromcharcode/` fixture + `diff_simple_fold_fromcharcode.rs`
integration test assert the SIMPLE-level output
`var a="HI";var b="💩";var c="";report(a,b,c);` for
`String.fromCharCode(72,73)` → `"HI"`, `String.fromCharCode(0xD83D,0xDCA9)` →
`"💩"` (emitted as its escaped surrogate pair), and `String.fromCharCode()` →
`""`, and guard that no `fromCharCode` call survives (proving the typed SIMPLE
pipeline ran, not the WHITESPACE_ONLY fallback).
## [0.191.0] - 2026-06-25

### Added — `codePointAt` string-literal folding observable end-to-end at SIMPLE

Picks up `closure-pass-constant-fold` 0.42.0, which folds
`"…".codePointAt(i)` on a string-literal receiver with a non-negative
integer-literal index into the Unicode code point starting at that UTF-16
code-unit index (ECMAScript §22.1.3.4). When `i` begins a surrogate pair the
two units combine into one astral code point — the defining difference from
`charCodeAt`.

New `tests/diff/simple-fold-codepointat/` fixture + `diff_simple_fold_codepointat.rs`
integration test assert the SIMPLE-level output
`var a=97;var b=128169;var c=56489;report(a,b,c);` for
`"a💩b".codePointAt(0)` → `97` (BMP), `"a💩b".codePointAt(1)` → `128169`
(the pair → `U+1F4A9`), and `"💩".codePointAt(1)` → `56489` (lone low
surrogate), and guard that no `codePointAt` call survives (proving the typed
SIMPLE pipeline ran, not the WHITESPACE_ONLY fallback).

## [0.190.0] - 2026-06-25

### Added — SIMPLE/ADVANCED fold `"abcabc".lastIndexOf(needle)` → numeric

Wires the new `String.prototype.lastIndexOf` fold (constant-fold 0.41.0) end to
end through the closurec CLI. At `--compilation_level SIMPLE` (and ADVANCED,
which routes through the same typed pipeline), a `"…".lastIndexOf("…")` call on
string-literal operands collapses to the UTF-16 code-unit index of the **last**
occurrence, or `-1` when absent — the mirror of the already-folded `indexOf`.

New fixture `tests/diff/simple-fold-lastindexof/` covers last-match, absent,
empty-needle, and the basic case: `"abcabc".lastIndexOf("bc")` → `4`,
`"abcabc".lastIndexOf("z")` → `-1`, `"abc".lastIndexOf("")` → `3` (an empty
needle yields the string length, not 0), `"ab".lastIndexOf("b")` → `1`, so the
output is `var a=4;var b=-1;var c=3;var d=1;report(a,b,c,d);`. Integration test
`diff_simple_fold_lastindexof.rs` asserts the folded stdout, that no
`lastIndexOf` call survives, and that the result is the typed pipeline (not the
WHITESPACE_ONLY fallback).
## [0.189.0] - 2026-06-25

### Added — SIMPLE/ADVANCED fold `"abcde".substr(start[, length])`

Wires the new legacy-`String.prototype.substr` fold (constant-fold 0.40.0) end
to end through the closurec CLI. At `--compilation_level SIMPLE` (and ADVANCED,
which routes through the same typed pipeline), a `"…".substr(startLit[,
lengthLit])` call on a string literal collapses to the substring string literal.

`substr` completes the slice family (`slice`, `substring`, `substr`); unlike the
other two, its second argument is a *length*, not an end index: a negative start
counts from the end (then clamps to 0) and the length clamps into
`[0, len - start]`. New fixture `tests/diff/simple-fold-substr/` exercises those
rules: `"abcde".substr(1, 2)` → `"bc"`, `"abcde".substr(1)` → `"bcde"`,
`"abcde".substr(-2)` → `"de"`, `"abcde".substr(10)` → `""`, so the output is
`var a="bc";var b="bcde";var c="de";var d="";report(a,b,c,d);`. Integration test
`diff_simple_fold_substr.rs` asserts the folded stdout, that no `.substr(` call
survives, and that the result is the typed pipeline (not the WHITESPACE_ONLY
fallback).
## [0.188.0] - 2026-06-24

### Added — SIMPLE/ADVANCED fold `"abcd".substring(start[, end])`

Wires the new `String.prototype.substring` fold (constant-fold 0.39.0) end to
end through the closurec CLI. At `--compilation_level SIMPLE` (and ADVANCED,
which routes through the same typed pipeline), a `"…".substring(startLit[,
endLit])` call on a string literal collapses to the substring string literal.

`substring` is the sibling of the already-folded `slice` but clamps each index
into `[0, len]` (a negative argument becomes 0 — it never counts from the end)
and SWAPS the endpoints when `start > end`. New fixture
`tests/diff/simple-fold-substring/` exercises exactly those rules:
`"abcd".substring(1, 3)` → `"bc"`, `"abcd".substring(3, 1)` → `"bc"` (swap),
`"abcd".substring(-2)` → `"abcd"` (clamp), `"abcd".substring(10)` → `""`, so the
output is `var a="bc";var b="bc";var c="abcd";var d="";report(a,b,c,d);`.
Integration test `diff_simple_fold_substring.rs` asserts the folded stdout, that
no `.substring(` call survives, and that the result is the typed pipeline (not
the WHITESPACE_ONLY fallback).
## [0.187.0] - 2026-06-24

### Added — `"a,b,c".split(separator[, limit])` folds to an array literal at SIMPLE/ADVANCED

Picks up `closure-pass-constant-fold` 0.38.0: a `String.prototype.split` call on
a string-literal receiver with a string-literal separator now folds to an
**array literal** of the piece strings (the first constant-fold that emits an
`ArrayExpression` rather than a scalar). `"a,b,c".split(",")` → `["a","b","c"]`,
`"abc".split("")` → `["a","b","c"]`, `"a,b,c".split(",", 2)` → `["a","b"]`,
`"abc".split()` → `["abc"]`. The fold declines (the call survives for the
runtime) for a non-string-literal/regex separator, a non-integer or negative
limit, or an empty-separator split of a receiver containing an astral character
(its surrogate pair would split into a lone surrogate). New
`tests/diff/simple-fold-split/` end-to-end fixture and `diff_simple_fold_split`
integration test.

## [0.182.0] - 2026-06-23

### Added — string `replace` / `replaceAll` fold at SIMPLE/ADVANCED

The typed-AST optimization pipeline now folds `String.prototype.replace` and
`replaceAll` when the receiver and both the search and replacement arguments
are string literals (via `closure-pass-constant-fold` 0.33.0): `replace`
substitutes the first match, `replaceAll` every match —
`"a-b-c".replaceAll("-","_")` → `"a_b_c"`, `"aXbXc".replace("X","-")` →
`"a-bXc"`. The search string is matched literally (no regex). A `$` in the
replacement (V8 substitution patterns), an empty search string (V8 boundary
insertion), a non-string argument, or a non-literal receiver passes through to
the runtime; `WHITESPACE_ONLY` leaves the calls untouched.

New end-to-end fixture `tests/diff/simple-fold-replace/` and integration test
`tests/diff_simple_fold_replace.rs` (three assertions: byte-exact stdout,
`replaceAll` folds all matches while `replace` folds only the first, and the
SIMPLE typed pipeline ran rather than the whitespace fallback).
## [0.181.0] - 2026-06-23

### Added — SIMPLE folds `"x".startsWith/endsWith/includes(needle)` → boolean

Pulls in `closure-pass-constant-fold` 0.32.0, whose pass folds the
single-argument substring predicates `String#startsWith`, `endsWith`, and
`includes` on two string literals to a boolean literal:
`"hello".startsWith("he")` → `true`, `"hello".endsWith("xo")` → `false`,
`"hello".includes("ell")` → `true`. The whole method call collapses to
`true`/`false`, so no call survives the typed pipeline.

New end-to-end fixture `tests/diff/simple-fold-strpred/` (three integration
tests) exercises all three predicates at `--compilation_level SIMPLE`,
asserting the booleans, that no method call remains, and that the typed
pipeline (not the `WHITESPACE_ONLY` fallback) produced the output.
## [0.180.0] - 2026-06-22

### Added — string `at` fold at SIMPLE/ADVANCED (negative-from-end indexing)

The typed-AST optimization pipeline now folds `String.prototype.at` on a string
literal with an integer-literal index (via `closure-pass-constant-fold`
0.31.0): `"abcde".at(-2)` → `"d"`. Unlike `charAt`, a negative index counts
from the end. An out-of-range index (JS `undefined`, no literal), a
fractional/non-literal index, or a lone-surrogate result passes through to the
runtime; `WHITESPACE_ONLY` leaves the call untouched.

New end-to-end fixture `tests/diff/simple-fold-at/` and integration test
`tests/diff_simple_fold_at.rs` (three assertions: byte-exact stdout, the call
is folded to `"d"`, and the SIMPLE typed pipeline ran rather than the
whitespace fallback).
## [0.183.0] - 2026-06-23

### Added — global `parseInt` / `parseFloat` fold at SIMPLE/ADVANCED

The `constant-fold` pass (bumped to 0.34.0) now folds global `parseInt(lit[,
radix])` and `parseFloat(lit)` calls whose first argument is a string literal to
the numeric literal V8 produces (ECMAScript §19.2.5 / §19.2.4):
`parseInt("12px")` → `12`, `parseInt("FF", 16)` → `255`, `parseInt("0x1F")` →
`31`, `parseFloat("3.14abc")` → `3.14`. Only the bare global identifier folds —
`window.parseInt(...)` is left alone — and calls whose result is `NaN`
(`parseInt("")`) or `±Infinity` (`parseFloat("Infinity")`) are left for the
runtime since neither has a literal token.

New end-to-end fixture `tests/diff/simple-fold-parseint/` and integration test
`diff_simple_fold_parseint.rs` cover the SIMPLE path
(`var a=12;var b=255;var c=3.14;var d=31;report(a,b,c,d);`). The `--help_markdown`
golden and `cli.spec.json` version were regenerated for the 0.183.0 bump.

## [0.179.0] - 2026-06-22

### Added — string `concat` fold at SIMPLE/ADVANCED

The typed-AST optimization pipeline now folds `String.prototype.concat` when
the receiver and every argument are string literals (via
`closure-pass-constant-fold` 0.30.0): `"foo".concat("bar", "baz")` →
`"foobarbaz"`. A non-string argument (which JS coerces via `ToString`), a
non-literal argument, or a result over the optimizer's 100 000-code-unit cap
passes through to the runtime; `WHITESPACE_ONLY` leaves the call untouched.

New end-to-end fixture `tests/diff/simple-fold-concat/` and integration test
`tests/diff_simple_fold_concat.rs` (three assertions: byte-exact stdout, the
call is folded to `"foobarbaz"`, and the SIMPLE typed pipeline ran rather than
the whitespace fallback).

## [0.178.0] - 2026-06-22

### Added — string `trim` / `trimStart` / `trimEnd` fold at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.29.0, which folds
`String#trim` / `trimStart` / `trimEnd` on a string literal: `"  hi  ".trim()`
→ `"hi"`, `.trimStart()` → `"hi  "`, `.trimEnd()` → `"  hi"`. The stripped set
is the exact ECMAScript white-space + line-terminator set (hard-coded, not
Rust's `char::is_whitespace`, which disagrees on U+0085/U+FEFF), so the fold is
sound.

New e2e fixture `tests/diff/simple-fold-trim` (and integration test
`diff_simple_fold_trim.rs`): `var s = "  hi  ".trim(); report(s);` →
`var s="hi";report(s);`, with a whitespace-fallback guard proving the fold
comes from the SIMPLE typed pipeline. The full existing fixture suite remains
byte-for-byte unchanged.

> Version note: bumped to 0.178.0 to sit above main (closurec 0.175.0), the
> open numeric `toString([radix])` fold branch (closurec 0.176.0, PR #6560),
> and the open `padStart/padEnd` fold branch (closurec 0.177.0, PR #6571), so
> the parallel branches never collide on the version line.
## [0.177.0] - 2026-06-22

### Added — string `padStart` / `padEnd` fold at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.28.0, which folds
`String#padStart` / `padEnd` on a string literal with an integer-literal target
length and an optional string-literal pad (default a single space):
`"5".padStart(3, "0")` → `"005"`, `"abc".padEnd(6)` → `"abc   "`,
`"abc".padStart(6, "12")` → `"121abc"`. A non-integer target, a non-literal pad,
a target over the optimizer's 100 000-code-unit cap (a denial-of-service guard),
and a fill that would split a surrogate pair all pass through unfolded.

New e2e fixture `tests/diff/simple-fold-pad` (and integration test
`diff_simple_fold_pad.rs`): `var s = "5".padStart(3, "0"); report(s);` →
`var s="005";report(s);`, with a whitespace-fallback guard proving the fold
comes from the SIMPLE typed pipeline. The full existing fixture suite remains
byte-for-byte unchanged.

> Version note: bumped to 0.177.0 to sit above main (closurec 0.175.0) and the
> merged numeric `toString([radix])` fold (closurec 0.176.0), so the parallel branches never collide on the version line.
## [0.176.0] - 2026-06-22

### Added — numeric `toString([radix])` fold at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.27.0, which folds
`Number.prototype.toString` on a non-negative integer literal with a known radix
to a string literal: `(255).toString()` → `"255"`, `(255).toString(16)` →
`"ff"`, `(255).toString(2)` → `"11111111"`. The radix is the default 10 or a
single integer literal in `2..=36`; a fractional receiver, an out-of-range
radix, and a variable radix pass through.

New e2e fixture `tests/diff/simple-fold-radix` (and integration test
`diff_simple_fold_radix.rs`): `var s = (255).toString(16); report(s);` →
`var s="ff";report(s);`, with a whitespace-fallback guard proving the fold comes
from the SIMPLE typed pipeline. The full existing fixture suite remains
byte-for-byte unchanged.

> Version note: bumped to 0.176.0 to sit above main (closurec 0.173.0), the
> merged `repeat` fold (0.175.0), and the merged `slice` fold (0.173.0), so the
> parallel branches never collide on the version line.

## [0.175.0] - 2026-06-22

### Added — string `repeat` fold at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.26.0, which folds
`String#repeat` on a string literal with a non-negative integer-literal count
to a string literal: `"ab".repeat(3)` → `"ababab"`, `"x".repeat(0)` → `""`. A
negative count (JS `RangeError`), a fractional/non-literal count, and a result
over the optimizer's 100 000-code-unit cap (a denial-of-service guard against
materializing a huge literal at compile time) all pass through unfolded.

New e2e fixture `tests/diff/simple-fold-repeat` (and integration test
`diff_simple_fold_repeat.rs`): `var s = "ab".repeat(3); report(s);` →
`var s="ababab";report(s);`, with a whitespace-fallback guard proving the fold
comes from the SIMPLE typed pipeline. The full existing fixture suite remains
byte-for-byte unchanged.

> Version note: bumped to 0.175.0 to sit above main (closurec 0.173.0) and the
> concurrently-open numeric `toString([radix])` fold branch (closurec 0.174.0,
> PR #6560), so the parallel branches never collide on the version line.

## [0.173.0] - 2026-06-22

### Added — string `slice` fold at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.24.0, which folds
`String#slice` on a string literal with integer-literal arguments to a string
literal: `"abcd".slice(1, 3)` → `"bc"`. JS `slice` indexes by UTF-16 code unit
over the half-open range `[start, end)`; negative arguments count from the end
and both ends clamp to `[0, length]`. Zero, one, or two integer-literal
arguments fold; a non-integer argument, more than two arguments, an identifier
receiver, or a cut that would split a surrogate pair (yielding a lone
surrogate) all pass through unfolded — matching `charAt`'s conservative stance.

New e2e fixture `tests/diff/simple-fold-slice` (and integration test
`diff_simple_fold_slice.rs`): `var s = "abcd".slice(1, 3); report(s);` →
`var s="bc";report(s);`, with a whitespace-fallback guard proving the fold
comes from the SIMPLE typed pipeline. The full existing fixture suite remains
byte-for-byte unchanged.

> Version note: bumped to 0.173.0 to sit above main (closurec 0.171.0) and the
> concurrently-open numeric `toString([radix])` fold branch (closurec 0.172.0,
> PR #6560), so the parallel branches never collide on the version line.

## [0.171.0] - 2026-06-22

### Added — string `indexOf` fold at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.22.0, which folds the
single-argument `String#indexOf` on two string literals to a numeric literal:
`"abcabc".indexOf("b")` → `1` (the UTF-16 code-unit index of the first
occurrence), an absent needle → `-1`, the empty needle → `0`. Only the
one-argument form folds; the `fromIndex` overload and a non-literal receiver
pass through.

New e2e fixture `tests/diff/simple-fold-indexof` (and integration test
`diff_simple_fold_indexof.rs`): `var i = "abcabc".indexOf("b"); report(i);` →
`var i=1;report(i);`, with a whitespace-fallback guard proving the fold comes
from the SIMPLE typed pipeline. The full existing fixture suite remains
byte-for-byte unchanged.

> Version note: bumped to 0.171.0 to sit above the concurrently-developed ASI
> Phase-3 branch (closurec 0.170.0) and the merged charat fold (0.169.0), so the
> parallel branches never collide on the version line.
## [0.170.0] - 2026-06-22

### Added — ASI Phase 3: restricted productions (Rule 3) end-to-end

Pulls in `coding-adventures-javascript-parser` 0.19.0, whose new
`force_restricted_semicolons` pre-pass forces an automatic semicolon after a
restricted keyword (`return`/`throw`/`break`/`continue`/`yield`) whose argument
is pushed to the next line — the ECMAScript §12.10.1 "no LineTerminator here"
rule. Because closurec's grammar is newline-blind, `return` ⏎ `42` previously
parsed (and re-emitted) as `return 42`, a silent **miscompile**; it is now
correctly `return; 42` (the `42` becoming a dead statement the SIMPLE pipeline
drops).

New e2e fixture `tests/diff/simple-asi-restricted` (and integration test
`diff_simple_asi_restricted.rs`): `function f(){return` ⏎ `42}` ⏎ `report(f())`
→ `function f(){return};report(f());`. The absence of `42` is the double proof —
the restricted production was honored *and* the SIMPLE typed pipeline ran (the
`WHITESPACE_ONLY` re-stitcher would emit `function f(){return 42}`). The full
existing fixture suite remains byte-for-byte unchanged.

Context guards keep a `return` that is really a property name (`a.return`,
`{return: 1}`) from being mis-split; postfix `++`/`--` restricted productions
are a documented follow-up.

> Version note: bumped to 0.170.0 to sit above the concurrently-developed
> `simple-fold-charat` branch (0.169.0) and the merged ASI Rule-1 release
> (0.168.0), so the parallel branches never collide on the version line.

## [0.169.0] - 2026-06-22

### Added — string indexing folds (`charCodeAt`/`charAt`) at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.21.0, which folds the
single-integer-index string methods on a string literal: `"abc".charCodeAt(0)`
→ `97`, `"abc".charAt(1)` → `"b"`, `"abc".charAt(9)` → `""` (UTF-16 code-unit
indexing). Non-negative integer-literal index only; out-of-range `charCodeAt`
(JS `NaN`) and lone-surrogate `charAt` results are left unfolded; identifier
receivers and the computed form pass through.

New e2e fixture `tests/diff/simple-fold-charat` (and integration test
`diff_simple_fold_charat.rs`) is the end-to-end oracle, with a
whitespace-fallback guard proving the fold comes from the SIMPLE typed pipeline.

> Version note: bumped to 0.169.0 (skipping 0.168.0, reserved by the
> concurrently-developed ASI Rule-1 string-newline branch) so the parallel
> branches don't collide on the version line.
## [0.168.0] - 2026-06-22

### Changed — ASI Rule 1 now handles string/template-ending statements

The Phase-2 limitation is gone: a semicolon-free statement that ends in a
string/template/regex literal immediately before a newline now parses and gets
optimized at SIMPLE/ADVANCED, where it previously degraded to WHITESPACE_ONLY.
This rides on the `lexer` crate populating `TOKEN_PRECEDED_BY_NEWLINE` (0.6.0)
and `javascript-parser` reading it (0.18.0).

New end-to-end fixture `simple-asi-string-newline`:
`var label = "total"` ⏎ `var n = 1 + 2` ⏎ `show(label, n)` →
`var label="total";var n=3;show(label,n);` (`1 + 2` folds). The full existing
fixture suite remains byte-for-byte unchanged.

> Version note: bumped to 0.168.0 to sit above the three concurrently-developed
> constant-fold releases that merged first — `simple-fold-bitnot` (0.163.0),
> `simple-fold-strlen` (0.165.0), and `simple-fold-strcase` (0.167.0) — so the
> parallel branches never collide on the version line.

## [0.167.0] - 2026-06-22

### Added — ASCII string-casing folds at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.20.0, which folds the
no-argument string-casing methods on a string literal: `"abc".toUpperCase()` →
`"ABC"`, `"ABC".toLowerCase()` → `"abc"`. ASCII-only (locale-independent,
byte-for-byte equal to JS); non-ASCII strings, identifier receivers, the
computed form, and any-argument calls are all left alone.

New e2e fixture `tests/diff/simple-fold-strcase` (and integration test
`diff_simple_fold_strcase.rs`) is the end-to-end oracle, with a
whitespace-fallback guard proving the fold comes from the SIMPLE typed pipeline.

> Version note: bumped to 0.167.0 (skipping 0.166.0, reserved by the
> concurrently-developed ASI Rule-1 string-newline branch) so the parallel
> branches don't collide on the version line.

## [0.165.0] - 2026-06-22

### Added — string-literal `.length` folding at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.19.0, which folds the
`.length` of a string literal to a number: `"hello".length` → `5`, `"".length`
→ `0`, `"💩".length` → `2` (UTF-16 code-unit count, matching JS `String#length`).
Only the dotted non-computed form on a string literal folds; `s.length` on an
identifier and `"abc"["length"]` are left alone.

New e2e fixture `tests/diff/simple-fold-strlen` (and integration test
`diff_simple_fold_strlen.rs`) is the end-to-end oracle, with a
whitespace-fallback guard proving the fold comes from the SIMPLE typed pipeline.

> Version note: bumped to 0.165.0 (skipping 0.164.0, which is reserved by the
> concurrently-developed ASI Rule-1 string-newline release) so the two parallel
> branches don't collide on the version line.

## [0.163.0] - 2026-06-22

### Added — unary bitwise NOT folding (`~5` → `-6`) at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.18.0, which folds the
unary `~` operator on a numeric literal under ES `ToInt32` semantics: `~5` →
`-6`, `~-1` → `0`, `~5.9` → `-6`, `~~9` → `9`. This reuses the same `to_int32`
coercion the binary bitwise operators already use, so the two stay bit-for-bit
consistent.

New e2e fixture `tests/diff/simple-fold-bitnot` (and integration test
`diff_simple_fold_bitnot.rs`) is the end-to-end oracle, with a
whitespace-fallback guard proving the fold comes from the SIMPLE typed pipeline.

The `--help_markdown` golden fixture was regenerated for the version bump
(`Version: 0.163.0`).

## [0.162.0] - 2026-06-21

### Added — negation push (`!(a == b)` → `a != b`) at SIMPLE/ADVANCED

Pulls in `coding-adventures-closure-pass-constant-fold` 0.17.0, which rewrites a
logical-not over an (in)equality comparison into the inverted comparison:
`!(a == b)` → `a != b`, `!(a === b)` → `a !== b` (and the inverses). Relational
operators are left intact — `!(a < b)` is **not** `a >= b` under `NaN`.

New e2e fixture `tests/diff/simple-negation-fold` proves the equality push and
the relational NaN-safety guard (with a whitespace-fallback guard). The
`simple-unary-preserve` fixture's precedence case was switched from `!(a == b)`
to `!(a < b)` so the negation push doesn't rewrite it (it still exercises the
unary-precedence parenthesisation).

> Only reachable now that prefix operators survive the bridge (0.161.0) — before
> that, the `!` vanished before the optimizer could see the comparison.

## [0.161.0] - 2026-06-21

### Fixed — prefix unary operators no longer dropped at SIMPLE/ADVANCED

A program whose statements used a prefix unary operator on a non-foldable
operand — `!a`, `-b`, `~c`, `typeof x`, etc. — was **miscompiled** at SIMPLE
and ADVANCED: the operator silently vanished (`report(!a, -b, ~c)` became
`report(a, b, c)`). The root cause was in the `javascript-parser` bridge, which
mis-classified every prefix-operator form as a grammar pass-through and returned
the bare operand. WHITESPACE_ONLY was unaffected (it runs no typed pipeline).

Fixing the bridge unmasked two emitter precedence/printing bugs that this
release also fixes (`coding-adventures-closure-emitter` 0.18.0): `!(a == b)`
now keeps its parentheses (was `!a == b`, a different program), and `-(-a)` /
`+(+a)` print with a separating space (`- -a` / `+ +a`) instead of fusing into
the `--` / `++` decrement/increment token.

New end-to-end fixture `tests/diff/simple-unary-preserve` proves all prefix
operators survive the SIMPLE pipeline (with a whitespace-fallback guard and an
explicit `!(a == b)` parenthesisation check). Pulls in
`coding-adventures-javascript-parser` 0.17.0 and
`coding-adventures-closure-emitter` 0.18.0.

## [0.160.0] - 2026-06-21

### Changed — CLOC26 Phase 2: optimize newline-separated, semicolon-free source

closurec now applies ASI Rule 1 (insert a `;` before a token preceded by a line
terminator) in addition to the Phase 1 `}`/EOF rule, so a program that uses
newlines instead of semicolons — e.g.

```js
var w = 4
var s = 1 + 2
report(w * s)
```

— **parses and gets optimized** at `SIMPLE`/`ADVANCED` (`1 + 2` folds to `3`)
instead of degrading to `WHITESPACE_ONLY`. ASI lives in the
`coding-adventures-javascript-parser` crate (bumped to 0.16.0).

New end-to-end fixture `simple-asi-newline`; the
`simple_asi_newline_did_not_fall_back_to_whitespace_only` guard asserts the
folded `s=3` is present and `1+2` absent. Two statements on the *same* line
(`a = 1 b = 2`) remain a genuine error and still degrade, and the full existing
fixture suite stays byte-for-byte unchanged (ASI only acts on real parse
failures).

## [0.159.0] - 2026-06-21

### Changed — CLOC26 Phase 1: optimize semicolon-light source via ASI

closurec now applies Automatic Semicolon Insertion (the `}` / end-of-input
rule) before the typed pipeline runs, so programs that omit a `;` before a `}`
or at end of input — e.g. `function f(){return 1}` — **parse and get optimized**
at `SIMPLE`/`ADVANCED` instead of silently degrading to `WHITESPACE_ONLY`. This
was the single largest reason real-world, semicolon-light code got no
optimization. ASI lives in the `coding-adventures-javascript-parser` crate
(bumped to 0.15.0); the change here is the resulting behaviour.

New end-to-end fixture `simple-asi-block`: a function omitting the `;` before
its closing `}` now folds `1 + 2` to `3` at SIMPLE
(`function area(w){var s;s=3;return w * s};report(area(10));`). The
`simple_asi_block_did_not_fall_back_to_whitespace_only` guard asserts the folded
`s=3` is present and `1+2` is absent — an optimization only reachable because
ASI made the program parse.

**No regression:** ASI only inserts a `;` when parsing genuinely failed for lack
of one, so it is a no-op on already-valid input — the entire existing fixture
suite is byte-for-byte unchanged.

## [0.158.0] - 2026-06-20

### Changed — CLOC25: drop a redundant `else` after a terminating `if` consequent

At `--compilation_level SIMPLE` / `ADVANCED`, an `if` whose consequent
unconditionally terminates (`return` / `throw`) now has its `else` removed and
the `else` body hoisted out after the `if` — upstream Closure's
`MinimizeExitPoints`. The transform lives in the `fold-control-flow` pass
(bumped to 0.14.0); `WHITESPACE_ONLY`, which runs no passes, keeps the `else`.

New end-to-end fixture `simple-else-hoist`:

```text
function classify(n){if(n < 0){return negative(n)}record(n);return positive(n)}
```

— the `else` body (`record(n); return positive(n);`) is lifted out of the
`else`. The `simple_else_hoist_did_not_fall_back_to_whitespace_only` guard
asserts the optimized output contains **no** `else` (a whitespace-only fallback
would keep it).

Depends on `coding-adventures-closure-pass-fold-control-flow` 0.14.0.

## [0.157.0] - 2026-06-20

### Changed — CLOC24: strip `debugger` statements at SIMPLE/ADVANCED

`debugger;` statements are now **removed** at `--compilation_level SIMPLE` and
`ADVANCED`, matching the upstream Closure Compiler. A `debugger` statement is a
development-only breakpoint with no effect on a shipped program, so stripping it
is a sound size win. The strip lives in the `closure-pass-dce` pass (block-body
and top-level sweeps), which runs only in the typed pipeline — so `debugger` is
still preserved at `WHITESPACE_ONLY`.

The `simple-debugger` end-to-end fixture (introduced in CLOC21 to prove
`debugger` was *representable*) is repurposed as the strip oracle: its expected
output drops from `report(1);var x=3;debugger;use(x);` to
`report(1);var x=3;use(x);`, and the whitespace-fallback regression guard now
asserts the `debugger` is **absent** (its absence doubly confirms the typed
pipeline ran, since a WHITESPACE_ONLY fallback would have kept it).

Depends on `coding-adventures-closure-pass-dce` 0.15.0 and
`coding-adventures-javascript-ast` 0.13.0 (doc sync).

## [0.156.0] - 2026-06-20

### Added — CLOC23: end-to-end `for`-`of` support

Programs containing a `for`-`of` loop now route through the full typed-AST
optimization pipeline at both SIMPLE and ADVANCED levels. Previously any
`for`-`of` made the parse/bridge step decline, and closurec silently fell back
to WHITESPACE_ONLY (no real optimization). Now inlining, constant folding, DCE,
and (under ADVANCED) local/global/property renaming all run inside the loop body
and recurse into the iterable expression. All the common left forms are
supported (`for (var/let/const v of it)` and `for (v of it)`); `using` bindings
and destructuring left-hand sides decline to WHITESPACE_ONLY. The loop variable
renames consistently under ADVANCED (e.g. `for (var entry of values)` with
`sum + entry` → `for (var c of a)` with `b + c`).

With for-of, every common Phase-2 statement is now representable; only the
`with` statement (renaming-unsafe, a deliberate non-target) remains
bridge-unsupported. The `simple_level_unsupported_syntax_degrades_gracefully`
test accordingly now uses a `with` statement for its example.

The end-to-end `simple-for-of` diff fixture pins the behaviour, with a
regression guard that the output is NOT the whitespace fallback.

## [0.155.0] - 2026-06-20

### Added — CLOC22: end-to-end `for`-`in` support

Programs containing a `for`-`in` loop now route through the full typed-AST
optimization pipeline at both SIMPLE and ADVANCED levels. Previously any
`for`-`in` made the parse/bridge step decline, and closurec silently fell back
to WHITESPACE_ONLY (no real optimization). Now inlining, constant folding, DCE,
and (under ADVANCED) local/global/property renaming all run inside the loop body
and recurse into the enumerated expression. All four left forms are supported
(`for (var/let/const k in o)` and `for (k in o)`); destructuring left-hand sides
decline to WHITESPACE_ONLY. The loop variable renames consistently under
ADVANCED (e.g. `for (var element in c)` with `c[element]` →
`for (var b in c)` with `c[b]`).

The end-to-end `simple-for-in` diff fixture pins the behaviour, with a
regression guard that the output is NOT the whitespace fallback. The
`simple_level_unsupported_syntax_degrades_gracefully` test now uses a `for`-`of`
loop (still bridge-unsupported) for its example.

## [0.154.0] - 2026-06-20

### Added — CLOC21: `debugger;` no longer forces a WHITESPACE_ONLY fallback

Programs containing a `debugger` statement now route through the full typed-AST
optimization pipeline at both SIMPLE and ADVANCED levels. Previously any
`debugger` made the parse/bridge step decline, and closurec silently fell back
to WHITESPACE_ONLY (no real optimization). Now inlining, constant folding,
DCE, and (under ADVANCED) renaming all run across a `debugger` statement, which
is itself preserved verbatim. (Stripping `debugger` at SIMPLE/ADVANCED, as the
upstream Closure Compiler does, is a planned follow-up.)

The end-to-end `simple-debugger` diff fixture pins the behaviour, with a
regression guard that the output is NOT the whitespace fallback.

## [0.153.0] - 2026-06-20

### Added — CLOC20: end-to-end `do`/`while` support

Programs containing a `do`-`while` loop now route through the full typed-AST
optimization pipeline at both SIMPLE and ADVANCED levels. Previously any
`do`-`while` made the parse/bridge step decline, and closurec silently fell back
to WHITESPACE_ONLY (no real optimization). Now inlining, constant folding,
dead-code elimination, and (under ADVANCED) local/global/property renaming all
run inside the loop body and recurse into its test, while control flow is
preserved and the statement after the loop stays reachable (a do-while is not a
terminator).

The end-to-end `simple-do-while` diff fixture pins the behaviour, with a
regression guard that the output is NOT the whitespace fallback. The
`simple_level_unsupported_syntax_degrades_gracefully` unit test now uses a
`for`-`in` loop (still bridge-unsupported) for its example, since `do`-`while`
no longer degrades.

## [0.152.0] - 2026-06-20

### Added — CLOC19: end-to-end `try`/`catch`/`finally` support

Programs containing `try`/`catch`/`finally` now route through the full typed-AST
optimization pipeline at both SIMPLE and ADVANCED levels. Previously *any* `try`
made the parse/bridge step decline, and closurec silently fell back to
WHITESPACE_ONLY — emitting the input with only inter-token whitespace stripped,
zero real optimization. This was closurec's single largest coverage gap.

Now inlining, constant folding, dead-code elimination, and (under ADVANCED)
local/global/property renaming all run inside the protected block, the catch
handler, and the finalizer, while control flow and the catch binding are
preserved. The implementation spans the AST (`TryStatement`/`CatchClause`), the
parser bridge, the emitter, the scope analyzer, and every optimization pass; the
crucial soundness property is that the catch parameter is treated as a reserved
binding so renaming never aliases or rewrites the caught value.

Two end-to-end diff fixtures pin the behaviour:

* `simple-try-catch` — SIMPLE inlining + folding + DCE through try/catch/finally,
  plus a regression guard that the output is NOT the whitespace fallback.
* `advanced-try-catch-rename` — ADVANCED renaming with the catch binding `err`
  preserved verbatim and never collided with a generated short name.

## [0.151.0] - 2026-06-19

### Fixed (CLOC17 — assignment statements no longer force whitespace-only fallback)

Picks up `javascript-parser` 0.9.0, which fixes the `assignment_expression`
grammar-ordering bug. Before this, any program containing an assignment
statement (`a = 1;`, `g = f(5);`, `obj.k = v;`, `count += 1;`) failed to parse
and closurec emitted whitespace-only output for the *entire* program — no
optimization at all. Now such programs parse and run through the full pipeline
(e.g. `function f(p){log(p)} f(1); a=2;` → `log(1);a=2;` — `f` is inlined).

### Changed — fail-closed externs test uses genuinely-malformed JS

`run.rs`'s `BAD_EXTERNS` fixture (used by
`advanced_with_unparseable_externs_disables_property_renaming` to prove
property renaming fails closed when an `--externs` file can't be parsed) was
`"node.innerHTML = 1;"`, which only failed to parse *because* of the CLOC17
bug. With the bug fixed that string is now a valid externs file, so the fixture
was repointed at a hard syntax error (`"function {{{"`) to keep exercising the
fail-closed path independent of which expression forms parse.

## [0.150.0] - 2026-06-18

### Added (CLOC13.K — ADVANCED property renaming, gated on `--externs`)

`--compilation_level ADVANCED` now also shortens program-private **property**
names via the `rename-properties` pass
(`coding-adventures-closure-pass-rename-properties` 0.2.0), appended after
`rename-globals`. Properties live in their own namespace, so this is a second
independent ADVANCED-over-SIMPLE size win on top of top-level renaming:

```js
read(obj.innerHTML); read(obj.secretField); read(obj.secretField);
//  externs file: read(node.innerHTML);   // declares innerHTML external
//  ADVANCED + --externs => read(obj.innerHTML);read(obj.a);read(obj.a);
```

**Safe-by-default — gated on `--externs`.** Property renaming runs ONLY when
the user supplies at least one `--externs` file. The pass's bundled built-in
list covers ECMAScript but NOT the DOM/host, so renaming properties
unconditionally would rewrite `el.innerHTML` / `node.onload` and break browser
code. Supplying `--externs` is the user opting into the externs contract AND
declaring the external property boundary. Without it, ADVANCED leaves property
names untouched (only `rename-globals` runs, exactly as in 0.149.0).

- **New `collect_externs_property_names(config)`** — the property-namespace twin
  of `externs_do_not_rename`. It walks every `--externs` file through the
  rename-properties crate's new `collect_property_names`, unioning every
  property name they mention (dotted, quoted, and object keys) into the pass's
  do-not-rename set.
- **Fail-closed, NOT degrade-safe.** Unlike `externs_do_not_rename` (whose
  pass is sound with an empty keep-set), property renaming is sound only
  *because* the user declared the boundary — so the boundary's contents are
  what make it safe. If any externs source fails to resolve/read/parse/bridge,
  `collect_externs_property_names` returns `None` and property renaming is
  **disabled** for the run, rather than running against an empty/partial
  boundary (which would rename an externally-observable property — a
  miscompile, in exactly the case the user opted into safety). The CV `passes`
  trace is driven off the *same* decision, so it can never claim
  `rename-properties` ran when it didn't.
- **`run_typed_pipeline` now takes `Option<AdvancedConfig>`** (was
  `Option<HashSet<String>>`). `AdvancedConfig` carries both externs boundaries —
  the value-namespace keep-set for `rename-globals` (always runs under ADVANCED)
  and the optional property-namespace keep-set that gates `rename-properties`.
  The now-redundant `run_simple_pipeline` wrapper was removed (the unified
  caller passes `None` for SIMPLE directly).
- The correlation-vector `passes` trace lists `rename-properties` for ADVANCED
  only when `--externs` was supplied (the pass is conditional).
- SIMPLE output is byte-for-byte unchanged; ADVANCED without `--externs` is
  unchanged from 0.149.0.
- 4 new integration tests pin the policy (SIMPLE / no-externs ADVANCED leave
  properties alone; ADVANCED + externs renames a private property, keeps the
  externs-declared one, and shrinks output; ADVANCED + an unparseable externs
  file fails closed and renames nothing).

## [0.149.0] - 2026-06-18

### Added (CLOC13.I — ADVANCED diverges from SIMPLE: aggressive top-level renaming)

`--compilation_level ADVANCED` now produces genuinely smaller output than
SIMPLE for the first time. ADVANCED runs the SIMPLE pipeline PLUS the
`rename-globals` pass (`coding-adventures-closure-pass-rename-globals`),
appended after `rename`, which shortens program-private top-level names
(`function` / `var` / `let` / `const`) that survive the structural passes:

```js
function helper() { sideEffect(); return value; }
helper();
//  SIMPLE   => function helper(){sideEffect();return value};helper();
//  ADVANCED => function a(){sideEffect();return value};a();
```

- **`--externs` is now read** (it was parsed-then-discarded). Each externs file
  is parsed and its top-level declared names collected into the **do-not-rename
  set** — the external boundary ADVANCED must preserve. Degrade-safe: an
  unreadable / unparseable / bridge-rejected externs file contributes no names
  rather than failing the build. A `--externs` file declaring `helper` keeps it
  under ADVANCED.
- SIMPLE is unchanged (it never touches top-level names, since in a Script a
  top-level name may be externally visible). The ADVANCED rename is sound only
  under Closure's whole-program / externs contract — see the
  `closure-pass-rename-globals` crate.
- The `advanced_v1` CV trace's `passes` now lists `rename-globals`
  (`ADVANCED_PASS_NAMES`); SIMPLE's `simple_v2` list is unchanged.

### Fixtures / tests

- New `tests/diff/advanced-rename-globals/` fixture + a dual-level harness that
  runs BOTH levels and asserts ADVANCED renamed the top-level `helper` while
  SIMPLE kept it (and ADVANCED is smaller), plus a
  `advanced_renames_surviving_top_level_function` unit test.
- Updated the `advanced-optimizes` fixture (ADVANCED now renames its surviving
  `compute`) and the `advanced_matches_simple_output` test's comment (the two
  levels match only when no top-level name survives).
- Version bumped `0.148.0 -> 0.149.0` (`Cargo.toml`, `cli.spec.json`,
  help-markdown fixture).

## [0.148.0] - 2026-06-17

### Added (CLOC13.H — `inline-variables` constant propagation in SIMPLE)

The SIMPLE pipeline gains a new pass, `inline-variables`
(`coding-adventures-closure-pass-inline-variables` `0.1.0`), registered
between `inline` and `remove-unused-vars`. It propagates a top-level `const`
bound to a **literal** to its use sites, so remove-unused-vars deletes the
now-unreferenced binding and the fixed-point constant-fold sweep folds the
result:

```js
const RATE = 2;
total(base * RATE);
margin(RATE + 1);
//  closurec --compilation_level SIMPLE  ⇒  total(base * 2); margin(3);
```

Only `const` (never `let`/`var` — reassignable) bound to a literal (never an
identifier/call/member — which could change or have getters) is propagated,
and only when the name is declared exactly once in the whole program (no
shadowing). Single use → always; multiple uses → only when the literal is
short. `SIMPLE_PASS_NAMES` is now
`constant-fold → fold-control-flow → dce → inline → inline-variables →
remove-unused-vars → treeshake → rename`.

### Fixtures / tests

- New `tests/diff/simple-inline-variables/` fixture + harness
  (`const RATE = 2; total(base * RATE); margin(RATE + 1);` →
  `total(base * 2);margin(3);`), plus a
  `simple_propagates_const_literal_and_removes_binding` unit test.
- Updated the `simple_v2` CV trace's `passes` assertion to include
  `inline-variables`.
- Version bumped `0.147.0 → 0.148.0` (`Cargo.toml`, `cli.spec.json`,
  help-markdown fixture).

## [0.147.0] - 2026-06-17

### Changed (CLOC13.G — `inline` now inlines small functions at multiple sites)

The `inline` pass (`coding-adventures-closure-pass-inline` `0.3.0 → 0.4.0`)
no longer inlines only single-use functions. A small pure function is now
substituted at **all** its call sites when every use is an inlinable call and
the body fits the size budget (`expr_node_count(body) <= 2 + params.len()`):

```js
function sq(x) { return x * x; }
a(sq(3));
b(sq(4));
//  closurec --compilation_level SIMPLE  ⇒  a(9); b(16);
```

(Both sites inlined → `sq` removed by treeshake → fixed-point `constant-fold`
folds `3 * 3` / `4 * 4`.) A function with any value use (`g(f)`) or a
non-inlinable call, or a multi-use body over the budget, is left alone.

### Fixtures / tests

- New `tests/diff/simple-inline-multiuse/` fixture + harness
  (`a(sq(3)); b(sq(4));` → `a(9);b(16);`), plus a
  `simple_inlines_small_function_at_multiple_sites` unit test.
- Updated the `simple-rename`, `simple-treeshake`, `advanced-optimizes`
  fixtures (and matching unit tests) to give their demonstration function a
  *value use* (`sink(f)`) so the now-stronger inliner declines it — keeping
  each focused on rename / treeshake / fold rather than on inlining. (The
  previous "call it twice" guard no longer suffices now that multi-use small
  bodies are inlined.)
- Version bumped `0.146.0 → 0.147.0` (`Cargo.toml`, `cli.spec.json`,
  help-markdown fixture).

## [0.146.0] - 2026-06-17

### Changed (CLOC13.F — SIMPLE/ADVANCED pipeline now runs to a fixed point)

The pass pipeline (`coding-adventures-closure-pass-pipeline` `0.2.0 → 0.3.0`)
no longer runs each pass exactly once — it sweeps the pass order repeatedly
while any `FixedPoint` pass still reports a change, so a transform one pass
exposes is picked up by an earlier pass on the next sweep. This makes
optimizations **cascade**:

```js
// in.js
function double(x) { return x * 2; }
log(double(7));
//  closurec --compilation_level SIMPLE --js in.js
//  before: log(7 * 2);   (inline ran after constant-fold, so 7*2 never folded)
//  now:    log(14);       (sweep 2's constant-fold folds inline's output)
```

This applies to both SIMPLE and ADVANCED (which share the pipeline). No new
passes were added — existing real passes (`constant-fold`, `inline`,
`dce`, `treeshake`, …) simply compose to convergence now. Bounded by a
generous per-run sweep cap as a backstop against a non-convergent pass.

### Fixtures / tests

- New `tests/diff/simple-fixpoint/` fixture + harness proving the two-sweep
  `inline → constant-fold` cascade (`log(double(7))` → `log(14)`), plus a
  `simple_pipeline_iterates_to_a_fixed_point` unit test.
- Version bumped `0.145.0 → 0.146.0` (`Cargo.toml`, `cli.spec.json`,
  help-markdown fixture).

## [0.145.0] - 2026-06-17

### Changed (CLOC13.B.1 — `inline` pass now does real work in SIMPLE/ADVANCED)

The `inline` pass in the SIMPLE (and therefore ADVANCED) pipeline was an
identity stub; it now performs real single-use function inlining
(`coding-adventures-closure-pass-inline` `0.2.0 → 0.3.0`). A single-use
top-level leaf function whose body is `{ return EXPR; }` (with no free
identifiers) is substituted at its call site, and the now-dead declaration is
removed by the downstream `remove-unused-vars` / `treeshake` passes:

```js
// in.js
function double(x) { return x * 2; }
log(double(7));
//  closurec --compilation_level SIMPLE --js in.js  ⇒  log(7 * 2);
```

The inliner is deliberately conservative — see the `closure-pass-inline`
crate for the full provably-safe slice (top-level plain function, pure
`return` body, no free identifiers, declared once, used once, side-effect-free
arguments). Multi-use callees, function expressions, and bodies with
locals/branches are left untouched.

### Fixtures / tests

- Updated the `simple-rename`, `simple-treeshake`, and `advanced-optimizes`
  diff fixtures (and the corresponding `run.rs` unit tests) to call their
  demonstration function **twice**, so the single-use inliner leaves it in
  place — keeping each fixture focused on the pass it is meant to exercise
  (rename / treeshake / fold) rather than having the function inlined away.
- Version bumped `0.144.0 → 0.145.0` (`Cargo.toml`, `cli.spec.json`,
  help-markdown fixture).

## [0.144.0] - 2026-06-16

### Changed (CLOC12.161 — ADVANCED now optimizes instead of being a no-op)

`--compilation_level ADVANCED` was a **literal no-op**: it returned the source
verbatim (`source.to_string()`), so ADVANCED users got no optimization at all.
ADVANCED now runs the **same typed optimization pipeline as SIMPLE**
(`constant-fold → fold-control-flow → dce → inline → remove-unused-vars →
treeshake → rename`). ADVANCED is specified to be *at least* as aggressive as
SIMPLE, so reusing the SIMPLE pipeline is a correct lower bound. Advanced-only
passes (aggressive property/global renaming, cross-module tree-shaking) will
layer on here as they are implemented.

```js
var dead = 1 + 2; function compute(longName) { return longName + 1; } report(compute(7));
//  --compilation_level ADVANCED  ⇒  function compute(a){return a + 1};report(compute(7));
```

- The `Advanced` arm now shares the `Simple` match arm (the same
  parse→bridge→pipeline→emit path, degrade-safe to whitespace_only).
- `Bundle` / `TranspileOnly` remain identity (module bundling and language
  down-levelling are orthogonal and land separately).
- The `compilation_level` correlation-vector contribution for ADVANCED is now
  `advanced_v1` (same shape as `simple_v2` — level, bridge_status, passes,
  byte lengths) instead of the former `identity` tag.

### Verified
- New `tests/diff/advanced-optimizes/` fixture + `tests/diff_advanced_optimizes.rs`.
- New unit tests `advanced_optimizes_like_simple` (ADVANCED is no longer
  identity) and `advanced_matches_simple_output` (ADVANCED ≡ SIMPLE today).

## [0.143.0] - 2026-06-16

### Added (CLOC12.160 — SIMPLE pipeline gains `rename`)

The `--compilation_level SIMPLE` pass pipeline is now
`constant-fold → fold-control-flow → dce → inline → remove-unused-vars →
treeshake → rename`. The final pass shortens the parameters of **leaf
functions** (functions with no nested function) to short names, while keeping
the potentially-externally-visible function name:

```js
function distance(horizontal, vertical) { return horizontal*horizontal + vertical*vertical; }
distance(3, 4);
//  ⇒  function distance(a,b){return a * a + b * b};distance(3,4);
```

`rename` runs last (it has no dependencies; registration order places it at the
end) so it shortens names after every structural pass has finished. It relies
on `closure-pass-rename` 0.3.0's conservative α-rename — property names, free
globals, redeclared parameters, and non-leaf functions are all left untouched.

- `SIMPLE_PASS_NAMES` is now
  `[constant-fold, fold-control-flow, dce, inline, remove-unused-vars, treeshake, rename]`;
  the `simple_v2` correlation-vector trace lists all seven.

### Verified
- New `tests/diff/simple-rename/` fixture + `tests/diff_simple_rename.rs`:
  `function distance(horizontal, vertical) {…} distance(3, 4);` ⇒
  `function distance(a,b){return a * a + b * b};distance(3,4);`.
- New unit tests `simple_rename_shortens_leaf_function_params`,
  `simple_rename_keeps_property_names` (property names preserved),
  `simple_rename_whitespace_only_keeps_param_names`.
- `simple_v2` CV test updated to expect all seven pass names.

## [0.142.0] - 2026-06-16

### Added (CLOC12.159 — SIMPLE pipeline gains `treeshake`)

The `--compilation_level SIMPLE` pass pipeline is now
`constant-fold → fold-control-flow → dce → inline → remove-unused-vars →
treeshake`. The final pass deletes top-level `function`/`class` declarations
that nothing references — the function-shaped complement to
`remove-unused-vars` (which deliberately skips functions):

| Source | SIMPLE output |
|--------|---------------|
| `function dead() { return 1; }` | *(removed — never called)* |
| `function live() { return 2; } log(live());` | `function live(){return 2};log(live());` |

Removing an unused function declaration is unconditionally safe — declaring a
function has no side effect, so (unlike a `var` initializer) `treeshake` needs
no purity gate. It runs after `remove-unused-vars` so a function that only a
now-removed `var` referenced is itself swept in the same pipeline.

`treeshake`'s apply step was already implemented (it drops dead
`ProgramItem::Declaration(FunctionDeclaration)`s); functions bridge as bare
declarations, so — unlike `remove-unused-vars` — it had no Statement-wrapping
bug and works end-to-end as-is. Verified empirically before wiring.

- `SIMPLE_PASS_NAMES` is now
  `[constant-fold, fold-control-flow, dce, inline, remove-unused-vars, treeshake]`;
  the `simple_v2` correlation-vector trace lists all six.

### Verified
- New `tests/diff/simple-treeshake/` end-to-end fixture +
  `tests/diff_simple_treeshake.rs`:
  `function dead(){…} function live(){…} log(live());` ⇒
  `function live(){return 2};log(live());`.
- New unit tests `simple_treeshake_drops_unused_function`,
  `simple_treeshake_keeps_called_function`,
  `simple_treeshake_whitespace_only_keeps_function`.
- `simple_v2` CV test updated to expect all six pass names.

### Fixture/test churn
- The two `simple_dce_*` unit tests used an uncalled top-level `function f` as
  the carrier for the dce behavior under test; `treeshake` now removes it, so
  they call `f()` to keep it alive (the dce-inside-the-body effect is unchanged).

## [0.141.0] - 2026-06-16

### Added (CLOC12.158 — SIMPLE pipeline gains `remove-unused-vars`)

The `--compilation_level SIMPLE` pass pipeline is now
`constant-fold → fold-control-flow → dce → inline → remove-unused-vars`. The
final pass deletes top-level `var`/`let`/`const` bindings that nothing
references, when their initializer is side-effect-free:

| Source | SIMPLE output |
|--------|---------------|
| `var dead = 1 + 2; …` | *(removed — folds to `3`, then dropped)* |
| `var live = 10; log(live);` | `var live=10;log(live);` *(referenced)* |
| `var impure = run();` | `var impure=run();` *(kept — call may have a side effect)* |

The `var dead = 1 + 2` case shows `constant-fold` and `remove-unused-vars`
composing: the initializer must fold to a literal before the binding reads as a
pure, removable declaration.

- `inline` is now also registered. `remove-unused-vars` declares
  `depends_on = ["dce", "inline"]`, so the scheduler will not run it unless
  `inline` is in the pipeline. `inline` is an identity pass today; it holds the
  canonical slot until real function inlining lands.
- `SIMPLE_PASS_NAMES` is now
  `["constant-fold", "fold-control-flow", "dce", "inline", "remove-unused-vars"]`;
  the `passes` field in the `simple_v2` correlation-vector trace lists all five.

This relies on `closure-pass-remove-unused-vars` 0.4.0, which made the pass
actually remove bindings (it was previously a no-op on bridged programs).

### Verified
- New `tests/diff/simple-remove-unused-vars/` end-to-end fixture +
  `tests/diff_simple_remove_unused_vars.rs`:
  `var dead = 1 + 2; var live = 10; var impure = run(); log(live);` ⇒
  `var live=10;var impure=run();log(live);`.
- New unit tests `simple_remove_unused_drops_dead_top_level_var`,
  `simple_remove_unused_composes_with_constant_fold`,
  `simple_remove_unused_keeps_impure_initializer` (purity gate), and
  `simple_remove_unused_whitespace_only_keeps_var`.
- Existing `simple_v2` CV test updated to expect all five pass names.

### Fixture / test churn (default level is SIMPLE)

Adding `remove-unused-vars` to the default pipeline means an unreferenced
top-level `var` is now deleted by default. Several existing tests used a bare
`var x = 1;` as inert filler and broke when it vanished. Fixed two ways:
- **Feature-orthogonal tests pinned to `WHITESPACE_ONLY`** (they test charset,
  `--emit_use_strict`, IIFE/output-wrapper isolation, glob expansion,
  output-file plumbing, source maps, externs, concatenation/newline handling —
  none of which depend on the optimization level): the `charset-us-ascii`,
  `charset-utf8`, `emit-use-strict`, `isolation-iife`, `js-glob`,
  `js-output-file`, and `output-wrapper` diff fixtures, plus the corresponding
  `run.rs` unit tests. This isolates them from future optimizer changes too.
- **SIMPLE-level tests given referenced vars** so the binding survives and the
  behavior under test stays observable: `simple_level_constant_folds_arithmetic`,
  `simple_level_strips_whitespace_not_identity`,
  `simple_level_bridge_status_n_a_without_cv`, and the `simple-constant-fold`
  fixture now pass their values to a `report(...)`/`use(...)` call.

## [0.140.0] - 2026-06-16

### Added (CLOC12.157 — SIMPLE pipeline gains `dce`)

The `--compilation_level SIMPLE` pass pipeline is now
`constant-fold → fold-control-flow → dce` (was `constant-fold → fold-control-flow`).
The dead-code-elimination pass does two things, both scoped to block bodies:

1. **Dead-after-terminator** — drops every statement after a `return` in a
   block (`function f(){g();return 1;dead()}` ⇒ `function f(){g();return 1}`).
2. **Empty-statement removal** — sweeps `;` no-ops out of a block. This is what
   cleans up the empty statement `fold-control-flow` leaves behind when it folds
   away an `if (false) {…}` with no `else`.

dce runs **last**: both it and `fold-control-flow` declare
`depends_on = ["constant-fold"]` (so constant-fold runs first), but neither
depends on the other, so registration order is the tie-breaker — and we register
dce after fold-control-flow so it can sweep that pass's `;` debris.

- `SIMPLE_PASS_NAMES` is now `["constant-fold", "fold-control-flow", "dce"]`;
  the `passes` field in the `simple_v2` correlation-vector trace lists all three.
- `run_simple_pipeline` registers `DcePass` after `FoldControlFlowPass`.

### Verified
- New `tests/diff/simple-dce/` end-to-end fixture +
  `tests/diff_simple_dce.rs`: a function body exercising all three passes ⇒
  `function f(){keep();return 1};` (the dead `if (4 > 5) {…}` folds and is swept,
  the post-`return` `alsoDead()` is dropped).
- New unit tests `simple_dce_drops_dead_after_return`,
  `simple_dce_sweeps_folded_if_empty_statement` (all three passes composing), and
  `simple_dce_whitespace_only_keeps_dead_code`.
- Existing `simple_v2` CV test updated to expect all three pass names in `passes`.

## [0.139.0] - 2026-06-16

### Added (CLOC12.156 — SIMPLE pipeline gains `fold-control-flow`)

The `--compilation_level SIMPLE` pass pipeline is now
`constant-fold → fold-control-flow` (was just `constant-fold`). With the
control-flow folder, an `if` whose condition is statically decidable has its
dead branch pruned:

| Source | SIMPLE output |
|--------|---------------|
| `if (2 > 3) { keepElse(); } else { takeThis(); }` | `{takeThis()}` |
| `if (true) { alsoKept(); } else { dropped(); }` | `{alsoKept()}` |
| `if (4 > 5) { vanishes(); }` | `;` (empty statement) |

The `if (2 > 3)` case is the load-bearing one: `constant-fold` first turns the
comparison `2 > 3` into the literal `false`, and only then can
`fold-control-flow` decide the branch — so the two passes must compose. The
pass registers a `depends_on = ["constant-fold"]`, so the pipeline's
dependency topo-sort guarantees that order regardless of registration order.

- `SIMPLE_PASS_NAMES` is now `["constant-fold", "fold-control-flow"]`; the
  `passes` field in the `simple_v2` correlation-vector trace lists both.
- `run_simple_pipeline` registers `FoldControlFlowPass` alongside
  `ConstantFoldPass`.

### Verified
- New `tests/diff/simple-fold-control-flow/` end-to-end fixture +
  `tests/diff_simple_fold_control_flow.rs`: three decidable `if`s ⇒
  `{takeThis()}{alsoKept()};`.
- New unit tests `simple_fold_control_flow_prunes_dead_branch`
  (`if (2 > 3) {a()} else {b()}` ⇒ `{b()}`) and
  `simple_fold_control_flow_whitespace_only_keeps_if` (same input under
  WHITESPACE_ONLY keeps the whole `if`).
- Existing `simple_v2` CV test updated to expect both pass names in `passes`.
- `tests/diff/define/` re-pinned to `--compilation_level WHITESPACE_ONLY`.
  `--define` is level-independent, and the compilation level runs *before*
  the define pass, so at SIMPLE the now-present fold-control-flow rewrites
  `if (DEBUG) {…}` → `DEBUG && …` (while `DEBUG` is still a variable) before
  the substitution — a correct but surprising interaction that would churn
  this fixture on every SIMPLE PR. Pinning WHITESPACE_ONLY isolates the
  define-substitution oracle; SIMPLE behavior lives in the `simple-*` fixtures.

## [0.138.0] - 2026-06-15

### Added (CLOC12.155 — SIMPLE runs the typed-AST optimization pipeline, v2)

`--compilation_level SIMPLE` no longer degrades to whitespace-only output.
It now runs the real typed-AST optimization pipeline:

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program
       ──passes──▶ optimized Program ──emit──▶ JS text
```

In this first slice (PR-1) the pass pipeline holds a single pass —
`constant-fold` — so constant expressions are evaluated at compile time
(`1 + 2` ⇒ `3`, `3 * 4` ⇒ `12`, `2 + 3 * 4` ⇒ `14`). Follow-up PRs append
the remaining SIMPLE-appropriate passes (fold-control-flow, dce,
remove-unused-vars, local inline/rename), one pass per PR.

- **New `run_simple_pipeline` helper** in `run.rs`: takes the bridged
  `Program`, runs a `closure-pass-pipeline::PassPipeline` holding
  `ConstantFoldPass`, then serialises the optimized tree back to JS with
  `closure-emitter::emit` (minified, no source map). All four
  previously-wired-but-unused crates (`closure-pass-pipeline`,
  `closure-pass-constant-fold`, `closure-emitter`, `type-sidecar`) are now
  actually invoked.
- **`SIMPLE_PASS_NAMES` constant** — the ordered pass list the SIMPLE level
  runs (`["constant-fold"]` today). Each follow-up PR appends one entry.
- **Degrade-safe**: the typed path is best-effort. A grammar-parse
  rejection, a Phase-2+ bridge `UnsupportedSyntax`, a pass error, or an
  emitter error all fall back to `whitespace_only` so the compiler never
  errors on valid-but-not-yet-supported input. Only
  `BridgeError::InternalError` (a broken invariant) still propagates as
  `CompilerError::Bridge`.
- **Correlation-vector trace**: the `compilation_level` contribution tag
  moves from `simple_v1` to `simple_v2` and gains a `passes` field listing
  the pipeline. `bridge_status` now distinguishes `"ok"` (true optimized
  emit) from the degrade reasons `"parse_error:…"`,
  `"unsupported_syntax:…"`, `"pass_error:…"`, and `"emit_error:…"`.

### Verified
- New `tests/diff/simple-constant-fold/` end-to-end fixture +
  `tests/diff_simple.rs`: `var sum = 1 + 2; …` ⇒
  `var sum=3;var product=12;var nested=14;`.
- New unit tests `simple_level_constant_folds_arithmetic` (SIMPLE folds
  `1 + 2` ⇒ `3`) and `simple_level_whitespace_only_leaves_arithmetic_unfolded`
  (the same input under WHITESPACE_ONLY keeps `1+2`, proving the fold is the
  pipeline's doing).
- Existing SIMPLE unit tests updated for the `simple_v2` tag and `passes`
  field; degrade-on-unsupported-syntax behavior unchanged.
- `tests/diff/define/expected.stdout` regenerated: it runs at the default
  level (now SIMPLE), so its output is the emitter's form — the `if` keeps
  its block braces (`if(false){…}`) where the older whitespace-only path
  stripped them. The `--define` substitution meaning (`DEBUG` → `false`) is
  unchanged and identical across levels (define is a token-level pre-pass).

## [0.137.0] - 2026-06-15

### Fixed
- **gap-044b — template literals with non-identifier expressions no longer crash.**
  `${obj.name}`, `${a + b}`, `${f()}`, `${{a:1}}`, `${x ? y : z}`, and multiple
  substitutions all lex cleanly under ES2025.  The fix is in the `lexer` crate
  (GrammarLexer brace-depth tracking); closurec picks it up transitively.

## [0.136.0] - 2026-06-14

### Added

- **CLOC12.137 — wire typed-AST bridge into SIMPLE compilation level (v1).**
  `--compilation_level SIMPLE` now routes through the `javascript-parser`
  typed-AST bridge instead of identity passthrough.

  *Two-phase bridge call*: `parse_javascript_typed()` returns a
  `GrammarASTNode`; `bridge::grammar_to_program()` converts it to a typed
  `Program`. Phase separation is required to preserve `BridgeError` variant
  matching: `BridgeError::UnsupportedSyntax` causes a silent degrade to
  `whitespace_only` output (bridge status logged as
  `"unsupported_syntax:<rule>@<loc>"`), while `BridgeError::InternalError`
  propagates as the new `CompilerError::Bridge` variant.

  *Bridge status field*: a `simple_bridge_status: Option<String>` tracks the
  bridge result per compilation and is threaded into the correlation-vector
  contribution tag `"simple_v1"` (replacing the old `"identity"` tag for
  SIMPLE). Fields: `level`, `bridge_status`, `input_byte_len`,
  `output_byte_len`.

  *v1 output*: after bridge validation, the compiled output is still produced
  by `whitespace_only_minify` on the original source — typed AST optimization
  passes land in follow-up PRs (CLOC12.138+).

  *New tests*:
  - `simple_level_strips_whitespace_not_identity` — verifies SIMPLE no longer
    identity-passes; `"var  x   =   1 ;"` → `"var x=1;"`
  - `simple_level_bridge_ok_status_in_cv` — CV sidecar has `"tag":"simple_v1"`
    and `"bridge_status":"ok"` for parseable source
  - `simple_level_unsupported_syntax_degrades_gracefully` — `do-while`
    triggers `BridgeError::UnsupportedSyntax`; no error returned, CV shows
    `"unsupported_syntax:"` prefix
  - `simple_level_bridge_status_n_a_without_cv` — pipeline runs without CV
    enabled

  *Fixture updates*: eight integration test fixtures that previously relied on
  SIMPLE being identity passthrough are updated to match whitespace_only
  output: `charset-utf8`, `charset-us-ascii`, `define`, `emit-use-strict`,
  `isolation-iife`, `js-glob`, `js-output-file`, `output-wrapper`. Comment-
  only fixture inputs are replaced with real JS variables so test assertions
  remain meaningful after comment stripping.

  Bumped `javascript-parser` dependency (now provides `parse_javascript_typed`,
  `bridge`, and `bridge::BridgeError`).

## [0.135.0] - 2026-06-14

### Changed

- **CLOC12.135 — reconcile gap-044 spec: first slice already resolved, gap-044b introduced.**
  The spec had a stale OPEN entry for gap-044 (template literal substitutions)
  even though the F10 declarative lexer mode work already resolved the first
  slice: simple-identifier substitutions (`${name}`, `${x}`) are correctly
  lexed and emitted via `TEMPLATE_HEAD`/`TEMPLATE_MIDDLE`/`TEMPLATE_TAIL` mode
  transitions. Both `minify_template_subst` and `minify_tagged_subst` fixtures
  pass. No code changes in this release — spec-only reconciliation.

  - gap-044 entry updated to **RESOLVED (first slice)** with a precise
    description of what works and what the residual limitation is.
  - New **gap-044b** entry added for the open residual: expressions with
    operators (`.`, `+`, `(`, …) or nested `{}` inside `${…}` trip the
    div/default mode reset, losing template context. Root cause is that the
    F10 mode table has no brace-depth tracking, so `}` inside `${a.b}` reads
    as a plain RBRACE instead of a `TEMPLATE_TAIL`. The fix requires a mode
    stack in `GrammarLexer` (push template mode on `${`, pop on matching `}`).

## [0.134.0] - 2026-06-14

### Added

- **CLOC12.134 — close gap-049: `minify_for_body_inner_close` fixture.**
  The gap-032 single-statement block-flatten already suppresses the inlined
  trailing `;` when the closing `}` of the block is immediately followed by
  an outer `}` (`drop_trailing_semi = true`, `emit_end = close_idx - 1`).
  This was implemented implicitly alongside the gap-032 flatten, but gap-049
  remained open in the spec with no pinning fixture. This release adds
  `tests/diff/minify_for_body_inner_close/` which locks in the behaviour:
  `async function f(){for await(var v of a){a;}}` →
  `async function f(){for await(var v of a)a};` — byte-for-byte identical
  to upstream Closure v20240317.

### Fixed

- **Dead assignment in `whitespace_only_minify` gap-045 arm.** The arrow-paren
  elision path wrote `prev_emitted_tok = Some(ident)` immediately before
  overwriting it with `prev_emitted_tok = Some(kept[idx + 3])` (the `=>`
  token). Removed the dead intermediate assignment; no behaviour change.

## [0.133.0] - 2026-06-14

### Added

- **CLOC12.133 — correlation-vector emit-loop skip tombstones in
  `whitespace_only_minify`.**

  CLOC12.132 covered tokens dropped by gap *pre-passes* (tokens
  removed from `kept` before the emit loop begins). This release
  extends CV tracing to tokens that survive the pre-passes but are
  **suppressed during emission** — the second class of observable
  deletions in WHITESPACE_ONLY mode.

  **Seven emit-loop skip sites now emit `DeletionRecord`s:**

  | Site | Gap | Example |
  |------|-----|---------|
  | Empty `new X()` paren elision | `gap-050` | `new Foo()` → `new Foo` |
  | Rule-A `;` before `}` | `gap-030-rule-a` | `{a;}` → `{a}` |
  | Rule-C redundant `;` after synthetic `;` | `gap-030-rule-c` | `};` dedup |
  | Trailing `,` in array literal | `gap-046` | `[1,2,]` → `[1,2]` |
  | Trailing `,` in object literal | `gap-046b` | `{a:1,}` → `{a:1}` |
  | Single-stmt block flatten `{` / `}` | `gap-032` | `if(x){a();}` → `if(x)a();` |
  | Empty-block `{}` → `;` substitution | `gap-031` | `for(;;){}` → `for(;;);` |

  All tombstones use:
  - `source = "whitespace_only"`
  - `reason = "emit_skip"`
  - `meta.gap` — identifies the specific rule
  - `meta.lexeme` — the original token value

  **Implementation notes:**
  - `whitespace_only_minify` now takes `mut cv` (was non-mut).
  - A `ptr_to_cv_id: HashMap<*const Token, String>` is built once
    before the emit loop — O(n) setup, O(1) lookup per skip site.
  - The pre-pass sweep (CLOC12.132) now uses `cv.as_mut()` instead
    of consuming the option, so `cv` remains available for the emit
    loop.
  - A `tombstone_emit_skip` closure captures `emit_cv` and
    `ptr_to_cv_id`; each skip site calls it with the token and gap
    name.

  **Two new integration tests:**
  - `correlation_vector_emit_skip_gap050_new_empty_args`:
    `var x=new Foo();` — gap-050 drops the parens in the emit loop →
    `emit_skip` tombstone with `gap="gap-050"`.
  - `correlation_vector_emit_skip_gap030_rule_a_semi_before_brace`:
    `(function(){var x=1;})();` — rule-A drops the `;` before `}` →
    `emit_skip` tombstone with `gap="gap-030-rule-a"`.

  **Total test count: 649** (up from 647 in v0.132.0).

## [0.132.0] - 2026-06-14

### Added

- **CLOC12.132 — correlation-vector gap-drop tombstones in
  `whitespace_only_minify`.**

  Before this release, the CV sidecar recorded tombstones for
  *trivia* tokens (whitespace, comments) and EOF via the
  `whitespace_only_dropped` path in `run.rs`, but was silent about
  *non-trivia* tokens removed by the gap pre-passes (e.g. gap-053
  strips redundant parentheses from `var x=(1)` → `var x=1`).

  This release threads per-token CV IDs into `whitespace_only_minify`
  and adds a post-pass-pass tombstone sweep: after all pre-passes
  settle (`let kept = kept`), any non-trivia, non-EOF token from the
  original stream that is no longer referenced in `kept` receives a
  `DeletionRecord` with
  - `source = "whitespace_only"`
  - `reason = "gap_drop"`
  - `meta.token_index`, `meta.lexeme` for traceability.

  **API change:** `whitespace_only_minify` gains a third argument:
  ```rust
  cv: Option<(&mut CVLog, &str, &[String])>
  // (log, file_cv_id, token_cv_ids)
  ```
  Callers that don't need CV pass `None` — identical byte behaviour.

  **`transform_source_with_cv` signature update** (CLOC12.132): the
  `cv` tuple gains a `&[String]` (per-token CV ID slice, parallel to
  `tokenize_javascript_typed`'s output). `run_compiler` hoists
  `token_cv_ids` to outer scope so it's available at the
  `transform_source_with_cv` call site; empty when lex fails (safe —
  out-of-bounds indices are skipped).

  **Two new tests** pin the behaviour:
  - `correlation_vector_tombstones_gap_dropped_tokens_under_whitespace_only`:
    `var x=(1);` — gap-053 drops the parens → `gap_drop` tombstone
    with `source="whitespace_only"`.
  - `correlation_vector_no_gap_drop_tombstones_when_no_gaps_fire`:
    `var x=1;` — no pre-pass drops → no `gap_drop` tombstone.

  **Scope / known limitation:** this slice covers pre-pass drops
  (gap rules that remove tokens from `kept` before the emit loop).
  Emit-loop drops (e.g. gap-050's `new Foo()` → `new Foo` empty-paren
  elision) are NOT yet tombstoned here — that requires threading CV
  into the emit loop and is tracked as follow-up work.

## [0.131.0] - 2026-06-14

### Fixed

- **CLOSES gap-095 (WHITESPACE_ONLY)** — chained `new new A` now emits
  `new (new A)` (byte-identical to upstream Closure).

  **Rule:** when two consecutive operator `new` tokens appear, the inner
  NewExpression (callee + optional dot-chain, WITHOUT the following
  arg-list) is wrapped in `(…)`. The following `(…)`, if any, belongs
  to the outer `new`.

  **Examples:**
  - `a=new new A;`   → `a=new (new A);`
  - `a=new new A.B;` → `a=new (new A.B);`
  - `a=new new A();` → `a=new (new A)();`
    (`()` belongs to the outer `new`)

  **Implementation:** added a gap-095 pre-pass block in
  `whitespace_only.rs` (after gap-089, before gap-051). Detects two
  consecutive operator `new` tokens (guarded against `.new` / `?.new`
  property forms), scans the inner callee chain, and inserts synthetic
  `(` / `)` using `synth_num_open` / `synth_num_close`. Five unit tests
  added; `minify_chained_new` fixture now **enforced** in CI.

  **Status:** this was the **last** open IGNORE_FIXTURES entry.
  `IGNORE_FIXTURES` is now empty — every WHITESPACE_ONLY byte-identity
  fixture is enforced in CI.

## [0.130.0] - 2026-06-14

### Fixed

- **CLOSES gap-083 (WHITESPACE_ONLY)** — precedence-aware operand paren elision.
  When a binary operator's right operand is parenthesised and every top-level
  binary operator inside has **strictly greater** precedence than the outer
  operator, the grouping parens are redundant and are now dropped.

  **Example:** `a==(b+c)` → `a==b+c`
  - Outer `==` has precedence 9; inner `+` has precedence 12.
  - `12 > 9` → parens dropped.

  **Kept correctly:** `a*(b+c)` stays `a*(b+c)`
  - Outer `*` has precedence 13; inner `+` has precedence 12.
  - `12 < 13` → parens kept (inner is weaker; removing changes grouping).

  **Implementation:** two new `pub(crate)`-adjacent helpers added to
  `whitespace_only.rs`:
  - `binary_op_prec(tok)` — returns the JS binary operator precedence (3–14)
    for recognised symbol operators; `None` for unrecognised tokens, assignment
    operators, and comma.
  - `min_toplevel_binary_prec(span)` — scans the span for top-level binary
    operators (depth-0 w.r.t. nested `()`/`[]`/`{}`), returns the minimum
    precedence found.

  The existing gap-078 drop block is extended: after the atomic-operand check
  fails, `is_binary_sym` guards a second attempt that calls both helpers.  Only
  BINARY outer operators participate (prefix-unary operators like `-(b+c)` are
  excluded).

  `minify_precedence_operand` fixture is now **enforced** in CI.
  Updated existing `gap078_operator_operand_kept` test (now
  `gap083_precedence_aware_paren_elision`) to reflect the resolved behaviour
  and add boundary cases.

## [0.129.0] - 2026-06-14

### Fixed

- **CLOSES gap-085 (WHITESPACE_ONLY)** — both remaining fractional-shortest-form
  sub-cases were discovered to already produce byte-identical output (silently fixed
  by earlier gap work). The two fixtures are now enforced:

  - `num_neg_exp_frac`: `a=5e-3;` → `a=.005;`
    (negative-exponent scientific → fractional shortest-form)
  - `num_small_frac`: `a=0.0001;` → `a=1E-4;`
    (small decimal → exponential shortest-form)

  Both entries removed from `IGNORE_FIXTURES` in `tests/diff_minify.rs`.
  `CLOC12-gaps.md` gap-085 updated to RESOLVED.

- **CLOSES gap-106 (WHITESPACE_ONLY)** — non-integer numeric float property key
  canonicalisation was discovered to already be byte-identical (silently fixed by
  earlier gap work):

  - `minify_obj_numkey_float`: `x={.5:1};` → `x={"0.5":1};`

  Entry removed from `IGNORE_FIXTURES`. `CLOC12-gaps.md` gap-106 updated to
  RESOLVED.

## [0.128.0] - 2026-06-14

### Fixed
- **CLOSES gap-090 (CORRECTNESS)** — string escape sequences (`\xNN`, `\uNNNN`,
  `\u{N+}`, `\0`, and any other non-standard escape) were previously mangled:
  the `grammar_lexer.rs` `process_escapes` function had `other => result.push(other)`
  which **dropped the backslash**, so `"\x41"` arrived in `whitespace_only.rs`
  as `x41` and was emitted as `"x41"` (corrupted string value, not mere
  byte-identity divergence).

  **Root cause:** the lexer's escape-processing ran before the emitter had a
  chance to re-normalise, and left an ambiguous `tok.value` (`x41` is
  indistinguishable from a source string `"x41"`).

  **Fix:** `es2025.tokens` now declares `escapes: none` on the string rule
  section, which instructs the grammar lexer to deliver the **raw string
  interior** (quotes stripped, backslash sequences untouched) in `tok.value`.
  `whitespace_only.rs` gained three new functions:

  - `decode_js_string(raw)` — decodes every ECMAScript escape form to actual
    Unicode chars: `\xNN` (2-hex byte), `\uNNNN` (BMP unit), `\u{N+}` (code
    point), `\0` (null when not followed by 1–9), `\n`/`\t`/`\r`/`\b`/`\f`/`\v`,
    `\\`/`\"`/`\'`, and the ES-spec `\X → X` fallback for anything else.
  - `encode_js_char(out, c, quote)` — re-emits one decoded char in Closure
    WHITESPACE_ONLY canonical form: `\x00` for null, `\b`/`\t`/`\n`/`\f`/`\r`,
    `\x0b` for VT, `\\`, escaped-quote, `\xNN` for C0/DEL, `\uHHHH\uHHHH`
    surrogate pairs for non-BMP (U+10000+), literal otherwise.
  - `emit_quoted_string(out, raw)` — decode → choose delimiter (more `"` than
    `'` → single-quote, else double-quote; ties → double) → re-encode each char.
    Subsumes the old `push_quoted_string_content` / `emit_quoted_string`.

  Both functions are `pub(crate)`.  `defines.rs` updated to call
  `emit_quoted_string` for pass-through string tokens (their `tok.value` is
  also now raw with `escapes: none`).

  **Before/after:**
  ```
  "\x41"        →  "A"             (was "x41")
  "A"      →  "A"             (was "A" — already worked via process_escapes)
  "\u{1F600}"   →  "😀"  (was "u{1F600}")
  "\x27s"       →  "'s"            (was "x27s")
  "\0"          →  "\x00"          (was "0")
  ```

  Five end-to-end fixtures (`str_codepoint_esc`, `str_unicode4_esc`,
  `str_hex_esc`, `str_hex27_esc`, `str_null_esc`) are now **enforced**
  (un-ignored).  All 640 unit tests + the full diff_minify suite pass.

## [0.127.0] - 2026-06-14

### Fixed
- **CLOSES gap-044 (first slice)** — template literal substitutions `${expr}`
  are now lexed correctly for the common single-identifier case.  The
  `es2025.tokens` grammar adds two new flat modes (`template` and
  `template_div`) with `TEMPLATE_TAIL`/`TEMPLATE_MIDDLE` patterns at higher
  priority than the inherited `RBRACE`, so the closing `}` after a simple
  expression is recognised as a template closer, not a block-close.

  A companion fix in `whitespace_only.rs`'s `needs_separator` function
  prevents spurious whitespace from being inserted around the template
  boundary: `TEMPLATE_HEAD`/`TEMPLATE_MIDDLE` end with `${` and
  `TEMPLATE_MIDDLE`/`TEMPLATE_TAIL` start with `}` — both are
  punctuator boundaries that never need a separator.

  End-to-end fixtures `minify_template_subst` and `minify_tagged_subst` are
  now **enforced** (un-ignored).  Harness: 451/451 non-skipped fixtures pass
  (was 449/449 before these fixtures were added to the enforced set).

  Documented limitation: template expressions containing operators (`.`, `+`,
  `(`, `[`, …) or nested `{ }` reset the mode to `default`/`div`, losing the
  template context.  Full brace-depth support is a follow-up.

## [0.126.0] - 2026-06-13

### Fixed
- **CLOSES gap-092, gap-115, gap-119** — regex-vs-division disambiguation,
  via the new F10 declarative lexer mode transitions (no hand-written
  per-language lexer callback). The `es2025.tokens` grammar now declares a
  flat `div` mode entered after value-producing tokens; the shared
  `GrammarLexer` interprets the transition table.
  - **gap-115 (CORRECTNESS)** — `a/b/c` previously mis-lexed as `a` + regex
    `/b/` + `c`, emitting the INVALID `a /b/ c`. It now lexes as three
    divisions and round-trips byte-identically.
  - **gap-092** — `var x=a/b/c;` byte-identical (was `a /b/ c`).
  - **gap-119** — a regex after `return` no longer takes a spurious
    separating space (`return/a/g`, not `return /a/g`). The lexer half is
    F10; the emitter half is a `needs_separator` refinement: a `REGEX`
    literal as the RIGHT token never needs a leading separator from a
    word-like token (a regex starts with `/`, a punctuator), guarded
    against the `//`-comment merge hazard. New `is_regex` helper; unit
    tests `gap092_single_division_no_space`,
    `gap115_division_chain_round_trips`,
    `gap119_regex_after_return_no_space`,
    `gap119_regex_after_assign_preserved`. The three byte-identity
    fixtures `regex_div` / `div_chain` / `regex_after_return` are
    un-ignored and ENFORCED.

## [0.125.0] - 2026-06-13

### Fixed
- **CLOSES gap-120** — a NON-INTEGER NUMBER property key is now emitted as
  a QUOTED string of its canonical JS number form (`String(Number(key))`),
  matching upstream Closure v20240317:

      {.5:1}    ->  {"0.5":1}      {1.5:1}    ->  {"1.5":1}
      {1.50:1}  ->  {"1.5":1}      {1e-3:1}   ->  {"0.001":1}
      {1e-7:1}  ->  {"1e-7":1}     {2.5e-8:1} ->  {"2.5e-8":1}

  Float-key counterpart of gap-116 (canonical INTEGER string key →
  unquoted number); INTEGER numeric keys stay BARE (`{5:1}`, `{1e3:1}` →
  `{1E3:1}`). The canonical key string is JS `String(Number(key))`, which
  DIFFERS from closurec's value number printer (gap-040/082/113): it KEEPS
  the leading `0` before the point (`0.5`, not `.5`), strips trailing
  fractional zeros, and uses LOWERCASE-`e` exponential only for magnitudes
  below `1e-6` (`1e-7`, `2.5e-8`) — verified against the JAR (`1e-6` →
  `0.000001` stays decimal, `1e-7` goes exponential).

  New `noninteger_numeric_key_string` helper in `whitespace_only.rs`,
  wired into the number-emit branch. The coefficient and base-10 exponent
  are computed EXACTLY from the source digits (no Grisu/Ryu); after
  trailing-zero stripping, `E < 0` is exactly the non-integer case. The
  property-key position guard (prev `{`/`,`, next `:`) reuses gap-116's —
  it excludes the ternary `a?1.5:2` confound (the number is preceded by
  `?`), array/call elements, bare values, and the value half of a
  `{key:value}` pair (`{1.5:.5}` → `{"1.5":.5}` quotes only the key). An
  f64-range magnitude guard (`-324..=308`) bounds the leading-zero run
  (no DoS).

  `minify_float_key_quoted` un-ignored; two `gap120_*` unit tests added.

## [0.124.0] - 2026-06-13

### Fixed
- **CLOSES gap-113** — a FRACTIONAL number literal whose value is in the
  open interval (0, 1) — written in plain decimal (`.0001`) or scientific
  (`1e-5`, `1.5e-3`) form — is now canonicalised to the SHORTER of its
  leading-zero-stripped decimal and uppercase-`E` scientific forms,
  matching upstream Closure v20240317:

      1e-5    ->  1E-5       .0001   ->  1E-4       .000012 ->  1.2E-5
      1e-3    ->  .001       5e-1    ->  .5         1.5e-3  ->  .0015
      120e-3  ->  .12        2.5e-8  ->  2.5E-8     12e-5   ->  1.2E-4

  On a length TIE the form Java's `Double.toString` natively produces
  wins — DECIMAL for magnitudes `>= 1e-3`, SCIENTIFIC below — so `1e-3`
  keeps `.001` (tie at magnitude `1e-3`) but `1.2e-4` switches to
  `1.2E-4` (tie at magnitude `1e-4`). This is the negative-exponent
  counterpart of the existing positive-side shortest-form (gap-040/082).

  New `small_fraction_shortest_form` helper in `whitespace_only.rs`,
  wired into `normalize_number_value` before the gap-107 decimal-strip
  branch (which it subsumes for value < 1; values `>= 1` fall through
  unchanged). The coefficient and base-10 exponent are taken EXACTLY from
  the source digit string, so no Grisu/Ryu rounding is performed — the
  helper is a pure string transform.

  SECURITY: a magnitude guard (`-324..=308`, f64's finite range) rejects
  pathological exponents BEFORE building the decimal form, so a crafted
  literal like `1e-2147483648` can no longer make the printer allocate
  billions of zero bytes (DoS); it is left verbatim instead.

  Non-regression: integers, integer-valued floats, values `>= 1`, hex,
  and positive-exponent scientific are all untouched (gap-113 fires only
  for sub-1 fractions); already-shortest fractions (`.5`, `.001`) are
  idempotent. `minify_num_neg_exp` + `minify_num_frac_4dp` un-ignored;
  five `gap113_*` unit tests added. The value-`>= 1` scientific-fractional
  case (`1.23e1` -> `12.3`) and sub-normal-boundary f64 rounding remain
  the deferred true-Ryu residual.

## [0.123.0] - 2026-06-13

### Fixed
- **CLOSES gap-116** — a STRING property key that is a CANONICAL
  non-negative integer (`< 2^53`) is now UNQUOTED to a numeric key and
  printed in shortest numeric form, matching upstream Closure v20240317:

      {"123":1}              ->  {123:1}
      {"0":1}                ->  {0:1}
      {"1000":1}             ->  {1E3:1}
      {"123456789012345":1}  ->  {0x7048860ddf79:1}
      {"9007199254740991":1} ->  {9007199254740991:1}   (MAX_SAFE_INTEGER)

  The unquoted digits flow through the ordinary number printer
  (`normalize_number_value`), so the emitted key composes with the
  scientific (gap-040/gap-082) and hex (gap-114) shortest-forms — exactly
  what a bare numeric key (`{1000:1}` -> `{1E3:1}`) produces.

  New `numeric_string_key_unquoted(kept, idx)` helper in
  `whitespace_only.rs`, wired into the string-emit branch. Discriminator
  verified against the JAR:
  - **position**: previous emitted token is `{` or `,` and next is `:`.
    This excludes the ternary confound (`a?"1":"2"` — the string is
    preceded by `?`, not `{`/`,`), string VALUES, `case "1":`, and
    computed/method keys.
  - **canonical integer**: non-empty, all ASCII digits, `"0"` or no
    leading zero (`"00"`/`"01"` stay quoted).
  - **`< 2^53`**: `9007199254740991` unquotes but `9007199254740992`
    (= 2^53) stays quoted, because `String(Number(s))` no longer
    round-trips once the value is not exactly representable as an
    IEEE-754 double. Non-integer (`"1.5"`), signed (`"-1"`), and
    non-numeric (`"123abc"`) keys stay quoted.

  Float-key counterpart gap-120 (non-integer key -> quoted canonical
  string) remains OPEN. Two `gap116_*` unit tests + diff_minify
  (`minify_num_str_key` un-ignored) stay green.

## [0.122.0] - 2026-06-13

### Fixed
- **CLOSES gap-118** — an UPPERCASE hex literal that is RETAINED in hex
  form (because hex is the shortest representation, so it is not
  decimalised) is now emitted with LOWERCASE digits, matching upstream
  Closure v20240317:

      0xFFFFFFFFFFFFF  ->  0xfffffffffffff
      0xFFFFFFFFFF     ->  0xffffffffff
      0XA0000000000    ->  0xa0000000000

  Inverse/sibling of gap-114 (decimal → lowercase hex when shorter). The
  `cleaned` shortest-form candidate in `normalize_number_value` is the
  separator-stripped SOURCE, so for a hex literal it kept the author's
  case; when it tied the synthesised lowercase-`hex` candidate on length
  (both 15 chars for `0xFFFFFFFFFFFFF`) the `decimal > cleaned >
  scientific > hex` tie-break preferred the verbatim uppercase `cleaned`,
  so the uppercase form survived. The fix lowercases the `cleaned` HEX
  form (`cleaned.to_lowercase()` when it starts with `0x`/`0X`) so both
  candidates are byte-identical and either wins the tie correctly. Scoped
  to hex: decimal/octal/binary cleaned forms have no case-significant
  letters (a scientific `e`/`E` goes through the gap-082 path, and small
  octal/binary never stay in radix form).

  Non-regression verified against the JAR: short hex still decimalises
  regardless of case (`0xFF` → `255`, `0xAbC` → `2748`), already-
  lowercase retained hex is unchanged (`0xffffffffffff`), and the
  gap-114 large-non-round-integer decimal→hex emission stays lowercase
  (`123456789012345678` → `0x1b69b4ba630f350`).

## [0.121.0] - 2026-06-12

### Fixed
- **CLOSES gap-117** — a `case` clause whose operand begins with a UNARY
  operator (`-`, `+`, `!`, `~`) now gets the separating space upstream
  Closure v20240317 emits, which closurec used to omit:

      case-1:   ->  case -1:
      case+1:   ->  case +1:
      case!a:   ->  case !a:
      case~a:   ->  case ~a:
      case-a.b: ->  case -a.b:

  Like the `case`/`get`/`set`/`new` keyword + string-literal rule
  (gap-111), `case` followed by a word-like keyword needs whitespace to
  stay a distinct token from its operand. A plain-number operand
  (`case 1:`) already round-trips because the number is not adjacent to
  the keyword in a way that would glue; the unary PUNCTUATOR is what
  closurec's separator OR-chain previously skipped over.

  New `case_unary_needs_space(kept, idx)` helper in `whitespace_only.rs`
  returns true exactly when `kept[idx]` is a structural `-`/`+`/`!`/`~`
  punctuator and `kept[idx-1]` is the word-like keyword `case`; wired
  into the emit-loop separator OR-chain. Scoped strictly to `case`:
  `return-1`, `throw-1`, `typeof-1` stay glued (they match the JAR), and
  binary `x=a-1;` is untouched.

## [0.120.0] - 2026-06-12

### Fixed
- **CLOSES gap-114** — a large integer literal whose lowercase
  hexadecimal form is STRICTLY shorter than its decimal form is now
  emitted as `0x…`, matching upstream Closure v20240317:

      123456789012345678  ->  0x1b69b4ba630f350   (18 digits -> 17)

  `normalize_number_value` gains a `hex` candidate (`format!("0x{n:x}")`)
  in the integer shortest-form comparison, slotted at the LOWEST
  tie-break priority (decimal > cleaned > scientific > hex) so it is
  chosen only when strictly shortest — verified against the JAR:
  `4294967295` (decimal 10 == hex 10) stays decimal, round powers of ten
  still prefer scientific (`1000000000` -> `1E9`).

  f64 ROUNDING (hex candidate only): JS numbers are IEEE-754 f64, so an
  integer above 2^53 prints its NEAREST-f64 hex bits, not the exact
  source digits (`123456789012345678` denotes the double
  `123456789012345680` -> `…350`, not the exact `…34e`). The hex
  candidate is therefore computed over `(n as f64) as u128`. The
  decimal/scientific forms are deliberately left over the EXACT integer:
  upstream uses shortest-round-trip (Ryu) decimal there, which for a
  clean power of ten reproduces `1×10^e` (`100000000000000000000000` ->
  `1E23`) — rounding `n` globally would corrupt `scientific_form_of` (the
  rounded 10^23 is no longer a clean power). The exact-vs-double decimal
  mismatch for >2^53 integers that PRINT as decimal is the separate
  deferred Grisu/Ryu gap, unchanged here. For n ≤ 2^53 the rounding is
  the identity. Full unit suite (625) + diff_minify walk-test + all
  gap-038/040/082/091 number tests stay green.

  Six `gap114_*` unit assertions cover the hex wins, the tie/shorter-form
  non-regressions, and the f64-rounded hex value. The
  `minify_num_bigint_hex` fixture (added CLOC14.55) is now ENFORCED. The
  fractional/negative-exponent scientific cases (gap-113) remain OPEN.

## [0.119.0] - 2026-06-12

### Fixed
- **CLOSES gap-112** — a `for await(...)` async-iteration loop header no
  longer emits a spurious space between the `await` keyword and the `(`:

      async function f(){for await(const x of y)z()}
        ->  async function f(){for await(const x of y)z()};   (adjacent)

  Previously `await_operator_needs_space` (gap-072) — which forces a
  separating space before the `await` UNARY OPERATOR's operand
  (`await (a+b)`) — wrongly fired for the `for await` header, whose `(`
  is the loop HEAD, not an operand, producing `for await (const x of y)`.
  The fix adds a one-line FOR-AWAIT guard to that helper: when the token
  two before the `(` (i.e. the token before `await`) is the `for`
  keyword, the space is suppressed. EXACT and SAFE — a genuine unary
  `await(...)` is never preceded by `for`; the only `for await` form is
  this loop header. The empty-block body `for await(x of y){}` (which
  formerly passed only by coincidence via the method-name guard, since
  its `)` is followed by `{`) is subsumed by the new guard. Five
  `gap112_*` unit assertions cover the const/bare headers plus the
  unary-await-keeps-space and empty-block non-regressions. The
  `minify_for_await_bare_stmt` fixture (added CLOC14.54) is now ENFORCED.
  Verified byte-identical to upstream Closure v20240317. (The separate
  for-await loop-body single-statement block flatten — `for await(let x
  of y){z()}` → `for await(let x of y)z()`, a sibling of gap-074 — is NOT
  part of this gap and remains future work.)

## [0.118.0] - 2026-06-12

### Fixed
- **CLOSES gap-111** — a keyword that grammatically takes a STRING
  LITERAL as its immediately-following operand now gets the separating
  space upstream emits:

      switch(x){case"a":…}  ->  switch(x){case "a":…}   (case clause)
      x={get"a"(){}}        ->  x={get "a"(){}}          (string getter key)
      x={set"a"(v){}}       ->  x={set "a"(v){}}         (string setter key)
      x=new"s"              ->  x=new "s"                (new on a string)

  The fix adds a `keyword_string_needs_space` helper to the emit-time
  separator OR-chain in `whitespace_only.rs`: when the current token is a
  string literal and the previous token is a word-like keyword in the set
  `{case, get, set, new}`, a single space is inserted. The keyword set is
  EXACT — `typeof"s"`, `void"s"`, `throw"e"`, `a in"s"`, and
  `a instanceof"s"` are already byte-identical with NO space and are
  deliberately excluded (verified against the JAR). SAFE: in valid JS a
  bare `KEYWORD"string"` adjacency only occurs in these grammatical
  positions — two adjacent primary expressions are a syntax error, and
  these words as property keys/values are always separated from a string
  by `:`/`(`/etc. — so there is no alternative reading to corrupt. Nine
  `gap111_*` unit assertions cover the four wrap cases plus the
  excluded-keyword and keyword-as-key/identifier non-regressions. The
  three diff fixtures (`minify_case_string_space` /
  `minify_accessor_string_key` / `minify_new_string_callee`, added in
  CLOC14.53) are now ENFORCED. Verified byte-identical to upstream
  Closure v20240317.

## [0.117.0] - 2026-06-12

### Fixed
- **CLOSES gap-110** — a string method KEY preceded by a method MODIFIER
  (`*` generator and/or `async`) is now ALSO normalised to a COMPUTED
  key, matching upstream:

      x={*"m"(){}};            ->  x={*["m"](){}};
      class A{async"m"(){}}    ->  class A{async["m"](){}};
      x={async*"m"(){}};       ->  x={async*["m"](){}};

  This extends the gap-109 pre-pass: gap-109 only fired when the string's
  immediate predecessor was a property boundary (`{`/`,`/`}`/`static`),
  so a `*`/`async`-prefixed key was missed. The fix walks BACK over the
  contiguous run of method modifiers (`*`, `async`, `static`) to the
  ANCHOR — the token opening the member position — and requires that
  anchor to be a property-start (`{`/`,`/`}`). This proves a leading
  `*`/`async` is a method modifier and NOT a multiply/identifier in an
  expression: for `a=async*b` the string guard never matches (`b` is not
  a string), and for `a*"m"(){}` the anchor walk lands on the identifier
  `a` (not a property-start), so the generator/multiply ambiguity is
  correctly rejected — no spurious `[...]` wrap. The same method-body
  guard as gap-109 applies (the `)` matching the key's `(` must be
  followed by `{`). Seven `gap110_*` unit tests cover the generator,
  async, async-generator, and class-member wrap cases plus the
  `{*m(){}}`, `a=async*b`, and `a*"m"(x)` non-regressions. The three
  `*_string_method` diff fixtures (added in CLOC14.53) are now ENFORCED.
  Verified byte-identical to upstream Closure v20240317.

## [0.116.0] - 2026-06-12

### Fixed
- **CLOSES gap-109** — a method whose KEY is a STRING LITERAL is now
  normalised to a COMPUTED key, matching upstream:

      x={"m"(){}};        ->  x={["m"](){}};
      class A{"m"(){}}    ->  class A{["m"](){}};

  The fix adds a gap-109 pre-pass in `whitespace_only.rs` that wraps the
  string key in a synthetic `[`…`]` pair. Detection mirrors
  `get_set_computed_needs_space`'s property-start + method-body guards:
  the string sits at a property-start position (preceded by
  `{`/`,`/`}`/`static`, not a `.`/`?.` member access), is immediately
  followed by `(` (the parameter list), AND the `)` matching that `(` is
  immediately followed by `{` (the method body). The method-body guard
  is the decisive disambiguator — a string CALLED as a function
  (`"m"(x);`) has its `)` followed by `;`/operator/EOF, never `{`, so it
  is rejected; a string property VALUE (`{"a":1}`) has `:` after the
  string, not `(`. Identifier methods (`{m(){}}`), already-computed keys
  (`{["m"](){}}`), and call arguments (`f("m")`) are all untouched. Eight
  `gap109_*` unit tests cover the wrap cases and every non-regression
  guard. Verified byte-identical to upstream Closure v20240317. NOTE: a
  string-keyed ACCESSOR (`get"a"(){}` → `get "a"(){}`) is a SEPARATE
  space-insertion gap (upstream inserts a space, does not wrap), left
  for follow-up.

## [0.115.0] - 2026-06-12

### Fixed
- **CLOSES gap-108** — a do-while loop whose body is a single
  un-terminated statement now has its braces removed, matching the
  flattening upstream already applies to other single-statement
  bodies:

      do{x()}while(a);  ->  do x();while(a);

  The fix adds a gap-108 token-re-stitcher block in `whitespace_only.rs`,
  a direct sibling of the gap-080 else-body flatten: anchor on a `do`
  keyword (reserved, so `do{…}` is unambiguously the loop body — never
  an object literal), scan the body `{…}` to its matching `}`, and if it
  holds exactly one statement (no nested `{`, no control-flow keyword at
  depth 1, zero top-level `;`), drop the braces and replace the `}` with
  a synthetic `;`. The trailing `while(cond)` is untouched. A
  multi-statement body (`do{x();y()}while(a)`) keeps its braces; an empty
  body (`do{}while(a)`) is left for a follow-up (a `do;while` spacing
  nit). Six `gap108_*` unit tests cover the flatten, the multi-statement
  and nested-keyword guards, consecutive loops, and the
  already-flat/sibling non-regression cases. Two existing
  property-key-safety tests (`gap033_try_as_object_property_does_not_arm`,
  `gap034_class_as_property_does_not_arm`) had their expected output
  updated to the now-flattened do-body — the property-literal safety
  they guard is unchanged. Verified byte-identical to upstream Closure
  v20240317.

## [0.114.0] - 2026-06-12

### Fixed
- **CLOSES gap-107** — a FRACTIONAL (non-integer-valued) float literal
  with trailing zeros in its fractional part now has them stripped to
  the shortest exact decimal, plus a lone leading `0` before the `.`
  elided, matching upstream Closure v20240317:

      x=1.50;     ->  x=1.5;
      x=1.500;    ->  x=1.5;
      x=123.4500; ->  x=123.45;
      x=0.50;     ->  x=.5;     (trailing strip then leading-`0` drop)
      x=.50;      ->  x=.5;
      x=10.20;    ->  x=10.2;   (multi-digit int part kept)

  Previously these fell through `normalize_number_value`'s fractional
  fallback and were emitted verbatim. The fix adds a gap-107 arm in
  that fallback: for a literal that has a `.`, a non-integer value (so
  the gap-082 u128/integer path did not apply), and NO exponent, strip
  trailing `0`s from the fractional part (and a now-bare trailing `.`)
  then elide a lone `0` integer part. This is pure decimal-string
  normalisation — the value is exactly representable as written, so NO
  Grisu/Ryu is needed. As a bonus the long-standing `0.5` -> `.5`
  (gap-082's deferred "fractional left verbatim") now also resolves.
  The genuinely Grisu-needing residuals stay untouched and remain
  gap-085: anything with an exponent (`5e-3`, `1e-5`) is excluded by
  the no-`e`/`E` guard, and f64-precision cases like
  `12345678901234567890` -> `1.2345678901234567E19` never reach this
  arm (all-digits, no `.`). Eight `gap107_*` unit tests plus the
  updated `gap082_fractional_leading_zero_elided` cover the strip
  cases and every non-regression guard (`1.5`/`1.05`/`2.0`/`2.00`).

## [0.113.0] - 2026-06-12

### Fixed
- **CLOSES gap-105 (CORRECTNESS)** — LEGACY OCTAL number literals are
  now decoded as base-8 instead of being re-emitted as decimal. A
  number of the shape `0` followed by octal digits (`0`–`7`) is a
  sloppy-mode legacy octal and denotes its OCTAL value:

      var x=010;    ->  var x=8;     (was: var x=10;  — WRONG VALUE)
      var x=017;    ->  var x=15;    (was: var x=17;)
      var x=0123;   ->  var x=83;    (was: var x=123;)
      a=[010,020];  ->  a=[8,16];    (was: a=[10,20];)

  Previously such a token fell into the bare-decimal arm of
  `normalize_number_value` and was parsed as decimal, **changing the
  numeric value** — a real corruption, not a byte-only difference. The
  fix adds a legacy-octal arm (reached only after the `0x`/`0o`/`0b`
  prefix arms): when the separator-stripped literal has `len() > 1`,
  starts with `0`, and every byte is an octal digit, it is decoded with
  `u128::from_str_radix(.., 8)`. The decoded value flows through the
  same shortest-form selection as the other radix arms (decimal always
  wins for octal). Guards verified vs upstream Closure v20240317:
  - `00` → `0` (octal 0; unchanged),
  - lone `0` → `0` (excluded by `len() > 1`),
  - modern `0o17` → `15` (handled by the earlier `0o` arm),
  - `08`/`09` are not legacy octal (non-octal digit) and upstream
    rejects them, so they are never byte-identity inputs.

  Nine `gap105_*` unit tests cover the decode cases and every guard.

## [0.112.0] - 2026-06-12

### Fixed
- **CLOSES gap-104 (CORRECTNESS)** — the trailing-`;`-after-`}` rule
  (gap-030/041 family) no longer injects a stray `;` after a `}` that
  closes a destructuring pattern or object-default VALUE inside a
  function's PARAMETER LIST. Previously this produced **invalid JS**:

      function f({a=1}={}){}  ->  function f({a=1};={}){}   (was corrupt)
      function f({a=1}){}     ->  function f({a=1};){}      (was corrupt)
      function f(a={}){}      ->  function f(a={};){}       (was corrupt)

  The `}` in those positions closes a pattern/default value, not a
  statement block or function body, so no synthetic `;` is due. The
  fix suppresses the `;` whenever the `}`'s immediate follower is `=`,
  `,`, or `)` — a param-list/expression *continuation* token, never a
  statement boundary. A genuine function-DECLARATION body `}` (the only
  `}` that owes a `;` at this site) can never be followed by those
  tokens (declarations are statements, never lvalues, comma operands,
  or parenthesised), so the FINAL body `}` still receives its `;`:

      function f({a=1}={}){}  ->  function f({a=1}={}){};   (now correct)

  Verified byte-identical to upstream Closure v20240317. The three
  `param_*` byte-identity fixtures (added in CLOC14.48) are now
  enforced, and six `gap104_*` unit tests guard both the corruption
  cases and the genuine-body cases that MUST still terminate.

  NOTE: this is distinct from the separate `function f(){}a;` →
  `function f(){};a;` issue (a body `}` followed by an *identifier*),
  which is outside this `=`/`,`/`)` follower set and remains open.

## [0.111.0] - 2026-06-12

### Fixed
- **CLOSES gap-103** — a CLASS-BODY computed `get`/`set` accessor now
  gets the same separating space gap-073 gives object-literal accessors:

      class A{get[x](){}set[x](v){}}  -> class A{get [x](){}set [x](v){}}
      class A{m(){}get[x](){}}        -> class A{m(){}get [x](){}}
      class A{static get[x](){}}      -> class A{static get [x](){}}

  gap-073's `get_set_computed_needs_space` only fired when the accessor
  was preceded by `{` or `,` (object-literal property starts), so a
  class member preceded by a previous member's `}` (consecutive
  methods/accessors) or the `static` modifier lost the space. The fix
  adds `}` and `static` to that before-context set. Because a bare `}`
  is ambiguous (a statement-block close, e.g. `if(x){}get[k](x)` where
  `get` is a variable index + call, would be a false positive), a new
  **method-body guard** makes it safe: a real accessor's parameter list
  `)` is followed by a `{` body, whereas a variable-index-call's `)` is
  followed by `;`/an operator. The guard is applied uniformly (an
  accessor always has a body), so it also strengthens the existing
  `{`/`,` cases. JAR-verified across class accessor pairs, after-method,
  and `static` forms, plus the `if/for/while`-block-then-`get[k](x)`
  false-positive cases; +2 `gap103_*` unit tests; the three
  `minify_class_accessor_*` fixtures are un-ignored and enforced.

## [0.110.0] - 2026-06-12

### Fixed
- **CLOSES gap-072** — an `await` operator's grouping parens are now
  elided, and the operator is always emitted with a separating space:

      async function f(){await(x)}     -> async function f(){await x};
      async function f(){await(a.b)}   -> async function f(){await a.b};
      async function f(){a=await(b)}   -> async function f(){a=await b};
      async function f(){await(-b)}    -> async function f(){await -b};
      async function f(){await(a+b)}   -> async function f(){await (a+b)};

  `await` binds at UNARY precedence — exactly like `typeof`/`void`/
  `delete` — so it was added to gap-101's `is_safe_unary_kw_operand`
  keyword block (NOT the gap-056 return/throw block, which is for the
  looser-binding `yield`). A safe operand (identifier / literal /
  member-chain / call / leading unary) drops its parens; a parenthesised
  BINARY operand keeps them (`await` binds tighter than the binary op).
  Two extra concerns are handled:
  1. **Always-space.** Upstream emits the `await` operator with a space
     before its operand even when the operand is non-word-like
     (`await -b`, `await (a+b)`), to keep it distinct from `await(...)`
     call syntax. A new `await_operator_needs_space` emit predicate
     forces that space.
  2. **Contextual-keyword safety.** `await` can be a function/method
     NAME or a property. `function await(x){}` / `{await(x){}}` (the
     matched `)` is followed by `{`) and `o.await(x)` (preceded by
     `.`/`?.`) are guarded out of BOTH the paren-drop and the space, so
     they are emitted unchanged. (`await` as a plain value is a parse
     error in the upstream compiler, so it never appears in a
     byte-identity input — only the operator form needs handling.)

  Verified byte-identical against the upstream Closure JAR (v20240317)
  across identifier / member / call / unary / binary / comma operands
  plus the name and property guards. +3 `gap072_*` unit tests; the
  `minify_await_paren_elide` and `minify_await_binary_kept` fixtures are
  un-ignored and enforced. Known residual: a deeply-nested
  `await(await(x))` keeps the inner parens (the keyword block does not
  recurse into a dropped span — a pre-existing pattern shared with the
  other keywords; still valid JS, just not byte-identical).

## [0.109.0] - 2026-06-12

### Fixed
- **CLOSES gap-102** — a `yield` operand's grouping parens are
  redundant and are now dropped, matching the upstream Closure JAR:

      function*g(){yield(a);}     -> function*g(){yield a};
      function*g(){yield(a.b);}   -> function*g(){yield a.b};
      function*g(){yield(a+b);}   -> function*g(){yield a+b};
      function*g(){a=yield(b);}   -> function*g(){a=yield b};

  `yield` takes an `AssignmentExpression`, which binds looser than every
  binary operator, so a grouping paren around the operand never carries
  meaning — exactly like `return`/`throw` (gap-056, CLOC12.65). The fix
  adds `yield` to the gap-055/056 prefix-classification block in
  `whitespace_only.rs` (a new `is_yield_prefix`), reusing that pass's
  structural matching-`)` scan and its two guards verbatim:
  the top-level-comma guard keeps `yield(a,b)` wrapped (`yield a,b` ≡
  `(yield a),b`), and the property guard keeps `o.yield(x)` a method
  call (a `yield` preceded by `.`/`?.` is a property, not the keyword).
  The `yield*` delegate form is excluded for free — the token after
  `yield` is then `*`, not `(`, so the pass never fires. Verified
  byte-identical against the upstream Closure JAR (v20240317) across
  ident / member-chain / binary / call / unary / assignment-RHS
  operands plus the comma, delegate, and property cases; +3 `gap102_*`
  unit tests; the three `minify_yield_paren_*` fixtures from CLOC14.45
  are now enforced. (The `yield(a).b` member-follower case stays wrapped
  — a shared conservative limitation with `return`/`throw`, tracked
  separately.)

## [0.108.0] - 2026-06-12

### Fixed
- **CLOSES gap-101** — a prefix unary operator (`typeof`/`void`/
  `delete`/`!`/`-`/`+`/`~`) — and the binary `instanceof` — with a
  PARENTHESISED higher-arity operand now drops the grouping parens:

      a=typeof(void 0)       -> a=typeof void 0
      a=typeof(typeof b)     -> a=typeof typeof b
      a=typeof(-b)           -> a=typeof-b
      a=typeof(!b)           -> a=typeof!b
      a=typeof(b())          -> a=typeof b()
      a=typeof(a.b())        -> a=typeof a.b()
      a=void(void 0)         -> a=void void 0
      a=b instanceof(C())    -> a=b instanceof C()

  Every prefix unary operator (and `instanceof`) binds LOOSER than
  member access, call, and any prefix unary, so a parenthesised
  operand that is itself a UnaryExpression or a CallExpression
  re-associates identically with or without the grouping parens.
  Before this, gap-054's safe-operand set (CLOC12.63) only covered a
  single identifier/literal token or a member-reference chain, so
  these higher-arity operands kept their parens. The gap-054 keyword
  block's operand predicate was widened from `is_safe_unary_operand`
  to the new `is_safe_unary_kw_operand`, which additionally accepts a
  leading symbol/keyword unary chain (`is_safe_unary_paren_operand` +
  a `typeof`/`void`/`delete` recursion) and a call/member chain
  (`is_call_ref_chain`). A parenthesised BINARY / comma / assignment /
  ternary operand (`typeof(b+c)`, `typeof(a,b)`, `typeof(a=b)`,
  `typeof(b?c:d)`) is still REJECTED and keeps its parens; the
  property guard (`o.delete(a)` stays a method call) is unchanged.
  Separator nuance preserved: a word-like inner operator keeps the
  space (`typeof void 0`), a symbol inner operator collapses it
  (`typeof-b`). Verified byte-identical against the upstream Closure
  JAR (v20240317) across 26 operand shapes. +3 `gap101_*` unit tests;
  the three `minify_unary_*` fixtures from CLOC14.44 are now enforced.

## [0.107.0] - 2026-06-12

### Fixed
- **CLOSES gap-100** — grouping parens around a `function`/`class`
  EXPRESSION are elided in expression position:

      a=(function(){})()       -> a=function(){}()
      a=(class{})()            -> a=class{}()
      a=(async function(){})() -> a=async function(){}()
      b=1,(function(){})()     -> b=1,function(){}()

  Those parens are only needed at STATEMENT position, where the
  leading `function`/`class` keyword would otherwise start a
  *declaration*. A new pass (after gap-099) finds a `(` immediately
  followed by `function`/`class`/`async function`, locates the
  matching `)` by a structural paren-depth scan, and drops the pair.

  MINIMAL SAFE SLICE — fires only when the `(` is preceded by a
  statement-level assignment `IDENT=` (the target is a plain
  identifier at a `;`/`{`/`}`/start boundary) or by `,`. This
  deliberately preserves two load-bearing cases:
    - the statement-position IIFE `(function(){})();` (preceded by
      `;`/`{`/`}`/start, never `=`/`,`) — unwrapping it would reparse
      the function as a declaration;
    - a DEFAULT-PARAMETER default value `function g(a=(function(){})())`
      (the `=`'s target sits after `(`, not a statement boundary) —
      unwrapping there exposes the body `}` to the function-decl
      trailing-`;` rule and corrupts the output.
  Broader expression contexts (after `(`/`[`/`return`/`=>`/operators,
  member-target or `var`-target assignments) are left to a follow-up.
  +3 `gap100_*` unit tests; JAR-verified. The funcexpr_iife_assign /
  classexpr_call fixtures leave the ignore list.

  (Note: a PRE-EXISTING corruption surfaced while testing —
  `function g(a=(function(){})()){}` already mis-emits a stray `;`
  inside the default value on origin/main, from the function-decl
  trailing-`;` rule mis-firing on a nested function expression in a
  param list. gap-100 does NOT touch that case; tracked separately.)

## [0.106.0] - 2026-06-12

### Fixed
- **CLOSES gap-099** — grouping parens around the OBJECT of a
  computed-member `[…]` access are now elided when the object is a
  simple reference:

      a=(b)[c]    -> a=b[c]      a=(b.c)[d]  -> a=b.c[d]
      a=(b)[c][d] -> a=b[c][d]   (a)[b]=c    -> a[b]=c

  This is the `[index]` sibling of gap-065 (callee `(f)(x)` -> `f(x)`)
  and gap-057 (`.member` `(a).b` -> `a.b`); `[` binds tighter than any
  operator a grouping paren could protect around a bare reference. The
  new pass mirrors gap-065's guards exactly — GROUPING (not a call/index
  paren), SIMPLE REFERENCE inside (identifier + `.IDENT` chain, no
  operators/commas/computed members), and a `[` FOLLOWER — only the
  follower differs. Non-trivial operands keep their parens (`(a+b)[c]`,
  `(b||c)[d]`), and a call paren is never grouping (`f(b)[c]` stays).
  Distinct from gap-087, which elided parens INSIDE the index
  (`a[(b)]` -> `a[b]`); gap-099 is the object side.

  Behaviour change: `(a)[")"]` now minifies to `a[")"]` (was left as
  `(a)[")"]` before gap-099 — the parens were previously kept because
  gap-057 only handled `.member`). JAR-verified. The stale gap-057
  `string_content_not_bracket` test (which asserted the pre-gap-099
  form, while still verifying the string `")"` isn't mistaken for a
  bracket) was updated and renamed to `gap099_string_content_not_bracket`.
  +4 new `gap099_*` unit tests. The `computed_member_paren` /
  `computed_member_chain` byte-identity fixtures leave the ignore list.

## [0.105.0] - 2026-06-12

### Fixed
- **CLOSES gap-097** — an async generator method now gets the
  separating space between `async` and `*` that upstream emits:

      o={async*m(){}}      -> o={async *m(){}}
      class A{async*m(){}} -> class A{async *m(){}}
      class A{static async*m(){}} -> class A{static async *m(){}}

  `async` is only a *contextual* keyword, so `async*x` is equally
  valid as MULTIPLICATION (`a=async*b` means `async * b`). Upstream
  adds the space only for the method form, and the trap is that
  `a=async*f()` (multiply) and `o={async*m(){}}` (method) share the
  prefix `async * NAME (`. The new `async_gen_method_needs_space`
  helper (a `needs_separator`-style lookahead in the emit loop)
  distinguishes them by the FULL method signature: `async * NAME (
  <params> ) {` — a named method (identifier name, not `[computed]`)
  with a parameter list AND a body `{`. It mirrors
  `get_set_computed_needs_space`'s structural depth-scan to find the
  param list's matching `)`, then requires a `{` body to follow — the
  exact thing the arithmetic forms lack. Multiplication (`a=async*b`,
  `a=async*f()`, `a=b,async*c`), computed methods (`async*[x](){}` —
  `*[` can't merge), and `async function*f(){}` are all left
  untouched. +6 `gap097_*` unit tests; JAR-verified across 15 forms.
  The `async_gen_method_class` / `async_gen_method_obj` fixtures leave
  the ignore list.

## [0.104.0] - 2026-06-12

### Fixed
- **CLOSES gap-098** — a trailing bare decimal point on an integer
  literal is now dropped, matching upstream:

      a=5.;    -> a=5;       a=5.+1;  -> a=5+1;
      a=50.;   -> a=50;      a=b=5.;  -> a=b=5;
      f(5.);   -> f(5);      a=[5.];  -> a=[5];

  The lexer splits `5.` (the float `5.0`) into NUMBER `5` + DOT `.`, so
  the redundant decimal point survived into the output as `5.`. This is
  the exact complement of gap-093: that pre-pass fires when a NUMBER is
  followed by a `.member` access (post-dot token is a property name)
  and parenthesises the number; gap-098 reuses the same pre-pass and
  fires on the *other* branch — when the dot's follower is NOT a
  property name (`;`, an operator, `)`, `,`, `]`, EOF), the dot cannot
  be member access, so it is a pure decimal-point remnant and is
  removed. A genuine float like `5.5` is a single NUMBER token (no
  separate DOT) and is never touched; `5.[0]` collapses to the bare
  index `5[0]`. +6 `gap098_*` unit tests; JAR-verified. The
  `num_trailing_dot` / `num_trailing_dot_arith` fixtures leave the
  ignore list.

  Known limitation surfaced while testing: `5.e3` (scientific notation
  = 5000) is mis-lexed into NUMBER `5` + DOT + NAME `e3`, which gap-093
  wraps to the invalid `(5).e3`. That is a separate, pre-existing
  lexer/NUMBER-pattern issue (the spacing that would disambiguate
  `5.e3` from `5 .e3` is lost at lex time) tracked separately — out of
  scope here.

## [0.103.0] - 2026-06-12

### Fixed
- **CLOSES gap-096 (CORRECTNESS)** — regexes carrying the `u` (unicode)
  flag are no longer corrupted under the default ES2025 mode. The bug
  lived in the shared `javascript-lexer`: the ES2024/ES2025 `REGEX`
  token's flag character class read `[dgimsvy]`, accidentally dropping
  the ES2015 `u` flag (a typo when `v`/unicodeSets was added for
  ES2024). So `/x/gimsuy` lexed as the truncated regex `/x/gims` plus a
  stray identifier `uy`, emitted as the invalid `/x/gims uy`. Fixed the
  flag class to the full ES2024 set `[dgimsuvy]` in `es2024.tokens` and
  `es2025.tokens` and regenerated the compiled lexer pattern; closurec
  picks up the fix through its `javascript-lexer` dependency (bumped to
  0.5.1). The `minify_regex_flags_all` byte-identity fixture leaves the
  ignore list. (Lexer-level fix — no closurec source change beyond the
  version bump; regression tests live in `javascript-lexer`.)

## [0.102.0] - 2026-06-12

### Fixed
- **CLOSES gap-093 (CORRECTNESS)** — a NUMBER literal that is the
  object of a `.member` access is now paren-wrapped so the dot reads
  as member access, never the number's decimal point:

      1 .x             -> (1).x
      255 .toString(16)-> (255).toString(16)
      1.5.toString()   -> (1.5).toString()
      1..toString()    -> (1).toString()   (double-dot collapses to one)

  Previously closurec re-stitched `1 .x` verbatim to the INVALID
  `1.x` (a JS parser reads `1.` as the float `1.0` then a stray `x`)
  and emitted the double-dot `1..toString()` for the integer-method
  case. A token-stream pre-pass in `whitespace_only.rs` rebuilds the
  kept-token list, replacing `<number>` with `( <number> )` whenever
  its immediate follower is a structural `.` and the post-dot token is
  a property name. For the double-dot form (`1..x` — the lexer splits
  the float's decimal point from the member dot) it also drops the
  first, redundant dot so exactly one survives. The synthetic parens
  are cloned from any source token (the source frequently has none,
  e.g. `1 .x`), the same trick `synth_semi` uses.

  Non-member numbers are untouched: index access (`1[0]`, follower
  `[`), object keys (`{1:2}`, follower `:`), arithmetic (`1+2`), and
  already-parenthesised numbers (`(1).x`, follower `)`). gap-082
  number normalisation runs first, so the wrapped value is canonical
  (`1.5e3.toFixed(2)` → `(1500).toFixed(2)`, `0xff .toString()` →
  `(255).toString()`). Identifier member access (`(foo).x` → `foo.x`)
  remains gap-057's job. JAR-verified across 17 cases; the three
  `minify_num_member_*` / `minify_num_float_method` byte-identity
  fixtures move out of the ignore list; +10 `gap093_*` unit tests.

## [0.101.0] - 2026-06-12

### Fixed
- **CLOSES gap-094 (CORRECTNESS)** — the array trailing-comma drop
  (gap-046) no longer corrupts arrays with a trailing HOLE. `[1,,]` is
  `[1, <hole>]` (length 2); the old rule dropped the comma before `]`
  to `[1,]` (length 1), silently shrinking the array. The drop is now
  guarded so it only fires when the comma follows a REAL element —
  the token before it must be neither a structural `,` (a preceding
  hole) nor a structural `[` (a leading hole):

      [1,,]    -> [1,,]    (kept — length 2)
      [,]      -> [,]      (kept)
      [,,]     -> [,,]     (kept)
      [1,2,,]  -> [1,2,,]  (kept)
      [1,2,]   -> [1,2]    (still drops — real element)
      [[1],]   -> [[1]]    (still drops)
      [f(),]   -> [f()]    (still drops)

  `minify_array_hole_trail` enforced. A stale gap-046 test that
  asserted the buggy `[1,,]` -> `[1,]` (and noted the rule was
  "technically WRONG") was corrected; +1 dedicated test.

## [0.100.0] - 2026-06-12

### Changed
- **CLOSES gap-091** — a BigInt RADIX literal is now canonicalised to
  its shortest decimal form under WHITESPACE_ONLY, matching upstream
  Closure (mirrors gap-038 for non-BigInt numbers):

      0xFFn   -> 255n
      0o17n   -> 15n
      0b101n  -> 5n
      0x1_FFn -> 511n   (separator + radix combined)

  The BigInt branch of `normalize_number_value` now, after stripping
  `_` separators (gap-048), parses a `0x`/`0o`/`0b` body into `u128`
  (`from_str_radix`) and re-emits `{decimal}n`. A decimal BigInt body
  (no radix prefix) is already shortest and passes through unchanged
  (`255n` stays `255n`); a magnitude beyond `u128::MAX` (e.g. a 140-bit
  `0xFF…FFn`) is left verbatim — real bigint arithmetic is a residual.
  `minify_bigint_hex` / `minify_bigint_bin` are now enforced.

  Three pre-existing gap-038/048 unit tests that asserted the deferred
  radix-BigInt behaviour (`0xfn` unchanged, `0x1FFFn` unchanged) were
  updated to the now-correct decimal forms (`15n`, `8191n`).

## [0.99.0] - 2026-06-12

### Changed
- **CLOSES gap-089** — the empty `()` of a `new` with a
  MEMBER-expression callee now drops under WHITESPACE_ONLY, matching
  upstream Closure:

      new a.b()    -> new a.b
      new a.b.c()  -> new a.b.c
      new a[x]()   -> new a[x]

  A new forward pre-pass anchors on each `new` keyword, parses the
  member callee (base identifier + `.IDENT` / balanced `[ … ]`
  accessors), and drops a trailing empty `( )`. It EXTENDS gap-050
  (which only handled bare-identifier callees, `new A()` -> `new A`) to
  member-expression callees. `minify_new_member_empty` is now enforced.

  Gated by the same follower test as gap-050: a following `(`, `.`,
  `[`, or template `` ` `` re-binds the result (`new a.b().c` ≠
  `new a.b.c`), so those blocked cases are left to the new-expr
  member-wrap pass (`new a.b().c` -> `(new a.b).c`). The callee must
  contain at least one accessor, so a bare `new IDENT()` stays gap-050's
  job — the two passes never both fire on the same `()`. Non-empty args
  are kept (`new a.b(1)`); a benign operator follower strips
  (`new a.b()+1` -> `new a.b+1`).

## [0.98.0] - 2026-06-12

### Changed
- **CLOSES gap-088** — EMPTY-STATEMENT (`;`) elimination under
  WHITESPACE_ONLY, matching upstream Closure:

      ;;var x=1;          -> var x=1;
      var x=1;;;          -> var x=1;
      var a=1;;var b=2;   -> var a=1;var b=2;
      ;;;                 -> (empty)
      ;x();               -> x();
      function f(){;;x();} -> function f(){x()};

  A new FIRST pre-pass drops a `;` whose immediate predecessor is `{`,
  `;`, or start-of-input — exactly the statement-list positions with no
  statement before them. `minify_empty_stmt` is now enforced.

  Every other `;` is preserved automatically: a real terminator (`a;` —
  predecessor is a value) or a control-flow BODY (`while(a);`,
  `if(a);`, `for(;;);`, `do;while(a);` — predecessor `)`/`do`, not in
  the droppable set). The one hazard — the second separator in a
  `for(;;)` header (preceded by the first `;`) — is handled by a
  bracket stack that marks `for(` parens (detected via the preceding
  `for` keyword, excluding a `.for(` property call) and refuses to drop
  a `;` inside a for-header.

## [0.97.0] - 2026-06-12

### Changed
- **CLOSES gap-086** — a paren wrapping a whole CALL ARGUMENT now
  elides under WHITESPACE_ONLY, matching upstream Closure:

      f((a))      -> f(a)
      f((a+b))    -> f(a+b)      (any expression — no precedence guard)
      f((a||b))   -> f(a||b)
      f((a),(b))  -> f(a,b)      (each argument independently)
      f((a),b)    -> f(a,b)
      f(g((a)))   -> f(g(a))     (nested calls)
      a.b((c))    -> a.b(c)      (member / computed / new calls)
      new C((a))  -> new C(a)

  A new pre-pass anchors on the CALL-open paren (a `(` preceded by a
  value-producing token), walks the argument list, and via
  `maybe_strip_arg_paren` drops a wrapping `(`/`)` when the argument is
  entirely parenthesised and the inner span has no top-level comma.
  `minify_call_arg_paren` is now enforced.

  **The one load-bearing case is preserved:** a single comma-operator
  argument `f((a,b))` keeps its parens (dropping them would resplit one
  argument into two), guarded by `minify_call_arg_comma_keep`. A
  parenthesised arrow param list (`f((a,b)=>a)`) is also left alone (its
  `)` is followed by `=>`, not an argument boundary). Anchoring on the
  call open keeps array literals out of scope entirely.

## [0.96.0] - 2026-06-12

### Changed
- **CLOSES gap-087** — a paren wrapping the WHOLE index of a
  computed-member subscript now elides under WHITESPACE_ONLY, matching
  upstream Closure:

      a[(b)]      -> a[b]
      a[(b+c)]    -> a[b+c]
      a[(b,c)]    -> a[b,c]      (comma operator, single index — safe)
      a[(b=c)]    -> a[b=c]
      x()[(b)]    -> x()[b]      (value object ending in `)`)
      a[b[(c)]]   -> a[b[c]]     (nested subscripts)

  `minify_index_paren` is now enforced. A new subscript-anchored
  pre-pass fires when a `[` is preceded by a value-producing token (so
  it is a subscript, not an array literal) and immediately followed by
  `(`, whose structural-depth-matched `)` is immediately followed by
  the matching `]`. No comma / atomic guard is needed — the `[ … ]`
  already delimits a single expression.

  Array-literal element parens are NOT affected: `[(a,b)]` keeps its
  parens (the value-preceded requirement excludes array-literal `[`),
  since there a top-level comma is an element separator. That
  element-paren case is the comma-guarded gap-086 family.

## [0.95.0] - 2026-06-12

### Changed
- **CLOSES gap-082 (integer-valued subset)** — a decimal float /
  scientific NUMBER literal that denotes a non-negative INTEGER fitting
  in `u128` is now canonicalised to the same shortest form as a bare
  integer in WHITESPACE_ONLY, matching upstream Closure:

      1e3     -> 1E3      (lowercase e -> uppercase E; sci beats 1000)
      1.0     -> 1        (trailing .0 dropped)
      1.5e10  -> 15E9     (mantissa digit folds into the exponent)
      1.23e2  -> 123
      100.00  -> 100      (trailing fractional zeros)
      1.5e3   -> 1500     (decimal vs 15E2 tie -> decimal)
      12e3    -> 12E3
      1e21    -> 1E21     (10^21 < u128::MAX)

  `minify_num_exp_case` is now enforced. New helper
  `decimal_float_as_u128` parses `INT[.FRAC][eEXP]` to its exact value
  `digits × 10^(EXP − len(FRAC))` and returns the integer only when it
  is non-negative and fits in `u128` (all arithmetic via
  `checked_pow`/`checked_mul`/`parse::<u128>()`, so out-of-range inputs
  fall through to verbatim rather than panicking). Recovered integers
  reuse the existing decimal-vs-`scientific_form_of` shortest-form pick.

### Deferred → gap-085
- The V8 **fractional** shortest-form (`0.5` -> `.5`, `1e-5` -> `1E-5`,
  `0.0001` -> `1E-4`, `1.50` -> `1.5`) and over-`u128` magnitudes
  (`1e100` -> `1E100`) are left verbatim (valid, not byte-identical) —
  they need a Grisu/Ryū-style `f64` formatter, tracked as gap-085.
- The stale `gap058_scientific_mantissa_separator_stripped` unit test
  was corrected: `1_0e3` (= 10000) now canonicalises to `1E4`
  (JAR-verified). Its previous `10e3` assertion only stripped the
  separator and was never checked against the JAR — it was wrong.

## [0.94.0] - 2026-06-12

### Changed
- **CLOSES gap-084** — a nested double- (or deeper) paren around a
  var-init RHS now fully strips, matching upstream Closure: `((a))` →
  `a`, `(((a)))` → `a`, `((a+b))` → `a+b`.
  `minify_double_paren_varinit` is now enforced.

### Changed — gap-053 var-init paren elision now runs to a fixpoint

The gap-053 elision strips only the OUTERMOST `=(…)` layer per pass
and then advances past it, so `((a))` peeled to `(a)` and stopped —
one layer short of upstream. Wrapping the whole pass in a **fixpoint
loop** (repeat until an iteration drops nothing) peels every redundant
layer while the existing top-level-comma guard still halts at the
load-bearing layer:

  ((a))   -> (a)   -> a
  (((a))) -> ((a)) -> (a) -> a
  ((a+b)) -> (a+b) -> a+b          (each layer is the whole RHS)
  ((a,b)) -> (a,b)                 (inner comma operator — kept)

Termination is guaranteed: each iteration removes ≥2 tokens or makes
no change and breaks. **Deferred (valid, not byte-identical):**
`((a))+b` → upstream `a+b` (gap-053 never fires when the RHS is not
*just* the parens) and `if((a))b();` → `if(a)b();` (different anchor).
2 new `gap084_*` unit tests + the enforced byte-identity fixture.

## [0.93.0] - 2026-06-12

### Changed
- **CLOSES gap-081** — a grouping paren around a ternary `?:`
  CONDITION now elides, matching upstream Closure: `(a)?b:c` →
  `a?b:c`, `(a.b)?c:d` → `a.b?c:d`. The condition-side mirror of
  gap-055 (ternary ARMS). `minify_ternary_cond_paren` is now enforced.

### Added — gap-081 ternary condition paren elision

The parenthesised condition sits to the LEFT of the `?`, so it is
exactly the gap-077 LEFT-operand shape (a `(` that STARTS an
expression whose matching `)` is followed by an operator). Resolved by
adding a structural `?` to the gap-077 after-set
(`is_binary_or_cond_after`). All the existing machinery applies
unchanged:

- the starts-an-expression guard keeps a CALL condition
  (`f(a)?b:c` stays — dropping would corrupt to `fa?b:c`);
- the `is_safe_unary_paren_operand` atomic guard keeps a comma
  condition (`(a,b)?c:d`) and an operator condition (`(a||b)?c:d` —
  the precedence-aware strip is the deferred gap-083; closurec keeps
  it, which is valid);
- `?.` lexes as a single `"?."` token, so `is_structural_punct(t,
  "?")` matches ONLY the bare ternary and never `(a)?.b`.

3 new `gap081_*` unit tests + the enforced byte-identity fixture; the
now-stale `gap077_non_binary_after_not_stripped_here` test (which
asserted `(a)?b:c` unchanged) was replaced.

## [0.92.0] - 2026-06-12

### Changed
- **CLOSES gap-077** — a binary operator's parenthesised ATOMIC LEFT
  operand now elides, matching upstream Closure: `(a)+b` → `a+b`,
  `(a)*b` → `a*b`, `(a.b)+c` → `a.b+c`. The LEFT-hand mirror of
  gap-075/078 (RIGHT operand). `minify_left_operand_paren` is now
  enforced. With this, all four CLOC14.37 binary-operand /
  block-flatten gaps (077–080) are closed.

### Added — gap-077 binary LEFT-operand paren elision

A new pre-pass that fires on a structural `(` which (1) STARTS an
expression (the preceding token does NOT produce a value — a
call/member paren `f(a)+b` is preceded by a value-producing
word-like / string / `)`/`]`/`}` and is never stripped, else
`f(a)+b` would corrupt to `fa+b`), (2) has a matching `)` immediately
followed by a BINARY operator (so the span is that operator's LEFT
operand — `)` followed by `.`/`?.`/`(`/`[` is a member/call, left to
gap-057 / the callee passes), and (3) the span passes
`is_safe_unary_paren_operand`. An operand with a top-level binary
operator (`(a+b)*c`) or comma (`(a,b)+c`) is rejected → parens kept
(precedence / comma-operator safety).

**Exponentiation hazard (correctness).** `**` forbids an
*unparenthesised* unary LEFT operand — `-a**b` is a `SyntaxError`
(ECMAScript: the left side of `**` must be an `UpdateExpression`, not
a `UnaryExpression`). So `(-a)**b`, `(!a)**b`, `(typeof a)**b`, …
KEEP their parens; the pre-pass detects a unary-starting span before
a `**` and skips it. The byte-identity fixture `minify_exp_of_unary`
(`(-a)**b`) caught this and now guards it.

5 new `gap077_*` unit tests (strip / precedence-kept / call+comma-kept
/ `**`-unary-hazard / ternary-condition-untouched) + the enforced
fixture. `gap062_call_arg_grouping_preserved` updated to
`g((a)+(b))` → `g(a+b)` (both grouping layers now elide).

## [0.91.0] - 2026-06-12

### Changed
- **CLOSES gap-078** — the right-operand paren-elision pre-pass
  (gap-075) now also anchors on the binary comparison / logical /
  arithmetic / bitwise symbol operators, matching upstream Closure:
  `a==(b)` → `a==b`, `a||(b)` → `a||b`, `a*(b)` → `a*b`, `a<<(b)` →
  `a<<b`, … `minify_eq_operand_paren` is now enforced.

### Added — gap-078 binary symbol-operator right-operand elision

Extended the gap-075 pre-pass anchor (`is_sym_unary`, the prefix
symbols `-`/`+`/`!`/`~`) with an `is_binary_sym` clause covering the
full binary symbol-operator set — comparison (`==` `!=` `===` `!==`
`<` `>` `<=` `>=`), logical (`&&` `||` `??`), arithmetic (`*` `/` `%`
`**`), and bitwise (`&` `|` `^` `<<` `>>` `>>>`) — each
`is_structural_punct`-gated so a string/regex literal whose CONTENT is
an operator (e.g. `"=="`) never matches.

The existing `is_safe_unary_paren_operand` operand guard is unchanged
and remains the single safety gate: it accepts ONLY a self-delimiting
operand (a single safe token, a member-reference chain, or a leading
prefix-symbol-unary chain). An atomic operand has no precedence
interaction with the outer operator, so the strip is sound for *every*
binary operator.

**Deferred (precedence-aware refinement):** the JAR also strips when
the parenthesised operand's lowest-precedence operator binds at least
as tightly as the outer operator (`a==(b+c)` → `a==b+c`, since `+`
binds tighter than `==`, while `a*(b+c)` KEEPS its parens). That needs
an operator-precedence table; here `a==(b+c)` conservatively keeps its
parens (valid, just not yet byte-identical). 4 new `gap078_*` unit
tests (binary set / member-chain operand / operator-operand-kept /
literal+call safety) + the enforced byte-identity fixture.

## [0.90.0] - 2026-06-12

### Changed
- **CLOSES gap-080** — an `else` alternate that is a single
  un-terminated statement block now flattens, matching upstream
  Closure: `if(x)a();else{b()}` → `if(x)a();else b();`. The
  `else`-arm sibling of gap-079 (if-body flatten).
  `minify_else_body_flatten` is now enforced.

### Added — gap-080 else-body single-statement block flatten

A parallel `else`-anchored pre-pass, added right after the gap-074/079
header-keyword body-flatten pass. Unlike gap-074/079, the `else`
keyword has NO `(…)` header — its body `{` follows immediately, so the
anchor is simply `is_word_like(kept[i]) && kept[i].value == "else" &&
is_structural_punct(kept[i+1], "{")`.

`else` is a reserved word, so `else{…}` can never be an object literal
or a labelled block, and the only grammar that admits `else { … }` is
the alternate of an `if`. `else if(…)` is NOT matched (the token after
`else` is `if`, not `{`) — its inner consequent flattens via the
gap-079 `if` arm. The same provably-safe body scan as gap-074/079 (no
nested `{`, no control-flow keyword at depth 1, exactly zero top-level
`;`) gates the brace-drop, reusing gap-067's `synth_semi`.

**Deferred:** a nested-control `else` body (`else{if(y)b()}` →
upstream `else if(y)b();`) keeps its braces for now (output stays
valid); multi-statement and empty `else` bodies keep their braces. 4
new `gap080_*` unit tests (flatten / multi-keep / nested-control-kept
/ property-key-untouched) + the enforced byte-identity fixture.

## [0.89.0] - 2026-06-12

### Changed
- **CLOSES gap-079** — an `if` consequent that is a single
  un-terminated statement block now flattens, matching upstream
  Closure: `if(x){y()}` → `if(x)y();`. The `if`-sibling of gap-074
  (for/while loop-body flatten) and gap-076 (with-body flatten).
  `minify_if_body_flatten` is now enforced.

### Added — gap-079 if-body single-statement block flatten

`if` joins the gap-074 header-keyword body-flatten pre-pass anchor
set (`for`/`while`/`with`/`if`). A `{` immediately after an `if(…)`
header is unambiguously the consequent (never an object literal), so
the identical single-statement / property-guard / synthetic-`;`
machinery applies unchanged.

**Dangling-else safety came for free.** Stripping the braces around
an `if` consequent is unsound exactly when the body contains a nested
un-`else`-d `if` AND the outer `if` has an `else`:
`if(a){if(b)c()}else d()` must KEEP its braces — flattening to
`if(a)if(b)c();else d()` would re-bind the `else` to the inner
`if(b)` (the JAR keeps the braces too, verified). The existing
no-control-flow-keyword guard (`has_blocking_keyword`, which lists
`if`) already prevents the brace-drop for any body containing a
nested `if`, so the dangling-else case can never reach the drop. A
single non-control consequent (`{y()}`) has no such hazard.

`else`-arm flatten (`else{z()}` → `else z()`) remains the separate
open gap-080. 4 new `gap079_*` unit tests (flatten / multi-keep /
dangling-else-kept / else-if-chain) + the enforced byte-identity
fixture.

## [0.88.0] - 2026-06-12

### Changed
- **CLOSES gap-075** — a SYMBOL `-`/`+`/`!`/`~` operator now drops
  the redundant grouping parens around a simple-reference operand,
  matching upstream Closure: `-(a)` → `-a`, `!(a)` → `!a`, `~(a)` →
  `~a`, and the same-sign `-(-a)` → `- -a`, `+(+a)` → `+ +a` (a
  separating space, from gap-063, prevents the `--`/`++` glue).
  `minify_unary_minus_paren` is now enforced.

### Added — gap-075 prefix-unary symbol operand elision

A new pre-pass anchored on `is_structural_punct(kept[i],
"-"|"+"|"!"|"~")` with `kept[i+1]` a `(`. Prefix-vs-binary is
irrelevant: stripping a grouping paren around a self-delimiting
operand is sound whether the operator is a prefix unary (`-(a)`) or
a binary operator whose RIGHT operand is parenthesised (`a-(b)` →
`a-b`, which the JAR also does). The operand check is the new
`is_safe_unary_paren_operand`, which accepts everything
`is_safe_unary_operand` does PLUS a leading chain of prefix symbol
unaries applied to such an operand (`-a`, `!a`, `~a.b`) — this is
what makes `-(-a)`'s operand (itself a UnaryExpression) strippable.
Operator operands (`-(a+b)`, `a-(b+c)`) keep their parens. `--`/`++`
are single tokens whose `.value` is `"--"`/`"++"`, so they never
match the bare-`-`/`+` anchor (`-(--a)` left alone). The matching
LEFT-operand elision, predecrement operand (`a-(--b)`), and binary
comparison operands (`a!=(b)`) remain deferred. 3 new gap075_* unit
tests + the `minify_unary_minus_paren` byte-identity fixture.

## [0.87.0] - 2026-06-12

### Changed
- **CLOSES gap-076** — a `with` statement whose body is a single
  un-terminated statement now drops its braces, matching upstream
  Closure: `with(o){a()}` → `with(o)a();`. `minify_with_body_flatten`
  is now enforced.

### Added — gap-076 with-body single-statement flatten

`with` was added to the gap-074 header-keyword body-flatten pre-pass
anchor set (`for`/`while`/`with`). A `with(o){…}` statement has the
same `keyword (…) {body}` shape as a loop, and a `{` immediately
after the `with(…)` header is unambiguously the with-body — so the
identical single-statement / property-guard (`o.with(x){…}` left
alone) / synthetic-`;` machinery applies unchanged. Multi-statement
bodies (`with(o){a();b()}`) keep their braces. A `with` body that
already ends in `;` (`with(o){a();}`) is not yet flattened (the
gap-032 emit-time flatten does not set body-position after a
`with(…)` header) — deferred. 2 new gap076_* unit tests + the
`minify_with_body_flatten` byte-identity fixture.

## [0.86.0] - 2026-06-12

### Changed
- **CLOSES gap-071** — the binary `instanceof` operator now drops
  the redundant grouping parens around a simple-reference RIGHT
  operand, matching upstream Closure: `a instanceof(B)` →
  `a instanceof B` (also `a instanceof(b.c)`, `a instanceof(b[c])`).
  `minify_instanceof_paren` is now enforced.

### Added — gap-071 instanceof operand elision

`instanceof` was added to the gap-054/070 unary-keyword paren-elision
pre-pass keyword set (`void`/`typeof`/`delete`/`instanceof`). Although
`instanceof` is a binary operator, the right-operand elision is
mechanically identical to the prefix-unary cases — the left operand
sits at `kept[i-1]` and is irrelevant to the right operand's parens.
`instanceof` binds looser than member access, so `a instanceof(B.c)`
≡ `a instanceof B.c` and whatever follows the close paren
re-associates identically. The existing `is_safe_unary_operand`
check and property guard apply unchanged: operator operands
(`a instanceof(B||C)`) keep their parens, and `o.instanceof(x)` (a
property method call) is skipped. 3 new gap071_* unit tests + the
`minify_instanceof_paren` byte-identity fixture.

## [0.85.0] - 2026-06-12

### Changed
- **CLOSES gap-074** — a `for`/`while` loop body that is a SINGLE
  un-terminated statement drops its braces, matching upstream
  Closure: `l:for(;;){continue l}` → `l:for(;;)continue l;` (also
  `for(;;){break}`, `while(x){g()}`, `for(a in o){h(a)}`,
  `for(a of o){h(a)}`). `minify_loop_body_flatten` is now enforced.

### Added — gap-074 loop-body single-statement block flatten

A pre-pass (loop-body sibling of gap-067's labeled-block flatten)
anchored on a `for`/`while` STATEMENT keyword — word-like and NOT a
property (a `.`/`?.` look-behind disqualifies `o.while(x){…}`
method calls). The header `(…)` is matched by a structural depth
scan; the token after `)` must be a `{`. A `{` immediately
following a loop header is UNAMBIGUOUSLY a loop body (never an
object literal), so no completion-keyword guard is needed. The body
braces are dropped and a synthetic `;` (reusing gap-067's
`synth_semi`) terminates the flattened statement. Scoped to the
provably-safe slice: the body has no nested `{`, no control-flow
keyword at depth 1, and exactly zero top-level `;`. Bodies ending
in `;` are left to the gap-032 emit-time flatten; multi-statement,
empty, and nested-control-flow bodies keep their braces. `if`-body
and `do…while`-body flatten are deferred. 5 new gap074_* unit tests
+ the `minify_loop_body_flatten` byte-identity fixture.

## [0.84.0] - 2026-06-12

### Changed
- **CLOSES gap-073** — a `get`/`set` accessor in an object literal
  whose key is COMPUTED gains a separating space before the `[`,
  matching upstream Closure: `var o={get[k](){return 1}}` →
  `var o={get [k](){return 1}}` (also `set[k](v){}`).
  `minify_get_computed_space` is now enforced.

### Added — gap-073 `get`/`set` computed-key space

A two-token look-behind + forward-check helper
`get_set_computed_needs_space(kept, idx)`, consulted at the main
emit site (NOT in `needs_separator`, which sees only the adjacent
pair). `get`/`set` are *contextual* keywords — accessors only
inside an object/class body, plain identifiers elsewhere — and the
JS lexer types them identically, so distinguishing a real accessor
from member access (`o.get[k]`) or variable indexing (`get[k](x)`)
needs context. The helper fires only when (a) `kept[idx]` is a
structural `[`, (b) `kept[idx-1]` is the word-like `get`/`set`
keyword, (c) `kept[idx-2]` is an object-literal property-start
`{`/`,` (excludes member access and statement-level indexing), and
(d) the token after the matching `]` is a structural `(` (the
accessor parameter list). Class-body accessors after a previous
member (`}`-/`static`-preceded) are deferred. 2 new gap073_* unit
tests + the `minify_get_computed_space` byte-identity fixture.

## [0.83.0] - 2026-06-12

### Changed
- **CLOSES gap-070** — `delete`/`typeof`/`void` followed by a
  parenthesised MEMBER-REFERENCE CHAIN now drops the redundant
  grouping parens: `delete(a.b)` → `delete a.b`, `delete(a[b])` →
  `delete a[b]`, `typeof(a.b)` → `typeof a.b`. `minify_delete_paren_elide`
  is now enforced.

### Fixed
- **Correctness (property guard)** — `o.delete(a)` (a Map/Set
  `.delete()` method call) previously mis-emitted as the INVALID
  `o.delete a` because the unary-operand paren-elision pass lacked
  a property guard. The keyword is now skipped when preceded by a
  `.`/`?.` member accessor, so `o.delete(a)`, `o.typeof(x)`, and
  `o?.delete(a)` keep their call parens.

### Added — gap-070 member-chain operand elision

The gap-054 unary-keyword paren-elision pre-pass (previously
single-token only) was generalised. The operand validator
`is_safe_unary_operand` now accepts either a single safe token
(identifier / number / string — the original gap-054 case) OR a
member-reference chain: an identifier base followed by any run of
`.name` / `?.name` / `[…]` accessors with no top-level operator,
call, or comma. Both shapes bind tighter than a prefix unary
operator and are self-delimiting, so `OP(REF)` ≡ `OP REF`.
Operands with a top-level binary operator (`delete(a+b)`) are
left alone. The matching close paren is located by a structural
depth scan instead of the old fixed `i+3` offset. 4 new gap070_*
unit tests + the `minify_delete_paren_elide` byte-identity fixture.

## [0.82.0] - 2026-06-12

### Changed
- **CLOSES gap-069** — a `new` keyword followed by a KEPT grouping
  paren (compound callee) now gets a separating space, matching
  upstream Closure: `new(a+b)` → `new (a+b)`, `new(a,b)` →
  `new (a,b)`. `minify_new_paren_space` is now enforced.

### Added — gap-069 `new (` emit-adjacency space

A two-token look-behind helper, `new_paren_needs_space(kept, idx)`,
consulted at the main emit site (NOT in `needs_separator`, which
sees only the adjacent pair). Distinguishing the genuine
NewExpression keyword `new` from a PROPERTY named `new` (`o.new(f)`
— a method call) requires the token *before* `new`: the JavaScript
lexer is context-free and types `new` identically in both, so only
a preceding `.`/`?.` member accessor tells them apart. The helper
fires only when (a) `kept[idx]` is a structural `(`, (b)
`kept[idx-1]` is the word-like `new` keyword, and (c) `kept[idx-2]`
(if any) is not a `.`/`?.` accessor. The companion `new(f)()`
simple-reference form never reaches here — gap-068's pre-pass has
already elided those parens to `new f`. 3 new gap069_* unit tests
+ the `minify_new_paren_space` byte-identity fixture.

## [0.81.0] - 2026-06-11

### Changed
- **CLOSES gap-067** (provably-safe minimal slice) — a labeled
  single-statement block flattens: `label:{break label}` →
  `label:break label;`.

### Added — gap-067 token pre-pass + synthetic `;`

Flattens `IDENT : { <completion-keyword> … }` (body starting
with `break`/`continue`/`return`/`throw`) when the label sits at
a hard statement boundary. CRITICAL SAFETY: the boundary set is
`;`/`}`/start and DELIBERATELY EXCLUDES `{`, so an object-literal
value `{x:{break:1}}` (whose inner `IDENT:{…}` is preceded by the
object's `{`) and a ternary `a?b:{c}` are never touched. The
completion-keyword body guard proves the `{` is a block, not an
object. The opening `{` is dropped; the closing `}` becomes the
statement terminator — a synthetic `;` token (cloned from the
stream and re-typed) is injected when the body had no trailing
`;`. Multi-statement bodies keep their braces; nested labels
flatten only the innermost (conservative). 5 new gap067_* unit
tests (incl. object-literal + ternary safety).

## [0.80.0] - 2026-06-11

### Changed
- **CLOSES gap-068** — redundant parens around a `new` callee:
  `new(f)()` → `new f`, `new(a.b)` → `new a.b`. Sibling of
  gap-065/066.

### Added — gap-068 token pre-pass

Strips the grouping parens around a `new` callee when the callee
is a simple reference (identifier + `.IDENT` chain), anchored on
the `new` KEYWORD. The trailing empty `()` of the call form
(`new(f)()`) is then dropped by the existing gap-050 empty-paren
elision in the emit loop. Guards: operator `new` only (not a
property named `new` — `o.new(f)` is a method call), not a
string literal whose content is `new`; all bracket checks via
`is_structural_punct`. Operator inner `new(a+b)` keeps its
parens (`new a+b` would parse as `(new a)+b`). 5 new gap068_*
unit tests.

## [0.79.0] - 2026-06-11

### Changed
- **CLOSES gap-066** (minimal safe slice) — redundant parens
  after `extends`: `class A extends(B){}` → `class A extends B{}`
  (also `extends(a.b)` → `extends a.b`, class expressions).

### Added — gap-066 token pre-pass

Strips the grouping parens after the `extends` KEYWORD when the
superclass is a simple reference (identifier + `.IDENT` chain).
Guards: anchored on the `extends` keyword, not a string literal
whose content is `extends`, and not a PROPERTY named `extends`
(`o.extends(x)` is a method call — prev-prev must not be
`.`/`?.`); all bracket checks via `is_structural_punct`.

DELIBERATELY CONSERVATIVE vs upstream: `extends(B||C)` keeps its
parens because `B||C` is not a LeftHandSideExpression, so
`extends B||C` would be INVALID JS (upstream strips it anyway,
producing arguably-invalid output). Call-chain inners
(`extends(f())`) are deferred. 5 new gap066_* unit tests.

## [0.78.0] - 2026-06-11

### Changed
- **CLOSES gap-065** — callee paren elision: `(f)(x)` → `f(x)`,
  `(a.b)(x)` → `a.b(x)`, `` (f)`t` `` → `` f`t` ``. Sibling of
  gap-057 (member-object paren elision).

### Added — gap-065 token pre-pass

A pre-pass (mirroring gap-057's structure) strips the grouping
parens around the CALLEE of a call / tagged template when the
callee is a *simple reference* — a plain identifier plus
zero-or-more `.IDENT` accessors. Guards: GROUPING-not-CALL (the
`(` must follow punctuation other than `)`/`]`/`?.`, or start —
so `f(g)(x)` keeps `(g)`); the inner must be a bare
identifier-dot chain (the scan stops at the first non-`.IDENT`
token, so `(a,b)(x)` and `(a+b)(x)` keep their parens); the
follower must be a real `(` call paren or a template literal
(tagged template, gated on `is_word_like`). All bracket checks
via `is_structural_punct`. 6 new gap065_* unit tests.

## [0.77.0] - 2026-06-11

### Fixed
- **CLOSES gap-064 (CORRECTNESS)** — string `)` argument misread
  as empty-paren close. The gap-050 `new X()` → `new X`
  empty-paren-drop pass checked `kept[idx+1].value == ")"`
  WITHOUT the `is_structural_punct` guard, so a string argument
  whose content is `)` (stored `.value == ")"` after the lexer
  strips delimiters) was mistaken for the empty-arg close paren.
  `new A(")")` was mangled to `new A);` (invalid JS — dropped the
  string arg and left a stray `)`); `new A(")").b` to
  `(new A)).b`. Discovered by the CLOC14.32 byte-identity
  harness.

### Changed — gap-064 fix

Line 976 now gates the close-paren check on
`is_structural_punct(t, ")")`, so only a genuine `)` punctuator
token triggers the empty-paren elision — a string/regex/template
argument never can. The sibling `next2_blocks_drop` checks were
left as-is: they only ever BLOCK a drop (fail-safe — at worst a
missed optimization on malformed input, never wrong output). The
genuine empty-paren drop (`new A()` → `new A`) and real args
(`new A(x)`) are preserved. `minify_new_str_paren_arg` +
`minify_new_str_paren_member` flip IGNORED → PASS. 3 new
gap064_* unit tests.

## [0.76.0] - 2026-06-11

### Fixed
- **CLOSES gap-063 (CORRECTNESS)** — same-sign `+`/`-` token
  adjacency. The WHITESPACE_ONLY re-stitcher joined two adjacent
  operator tokens that both begin with `+` (or both `-`) into a
  spurious compound operator, CORRUPTING semantics: `- -a`
  (double negation) became `--a` (pre-decrement); likewise
  `+ +a`→`++a`, `a- -b`→`a--b`, `- --a`→`---a`. Discovered by
  the CLOC14.31 byte-identity harness (`minify_neg_neg`).

### Added — gap-063 same-sign space rule

`needs_separator()` now inserts a single space when the previous
token's last char and the next token's first char are both `+`,
or both `-`. Different signs (`a+ -b` → `a+-b`) stay joined —
`+-` is unambiguous. CRITICAL GUARD: the rule gates on
`is_punct(a) && is_punct(b)`, so a string/regex/template literal
whose `.value` ends/starts with a sign char (e.g. `"a-"`, whose
stored value is `a-`) can never trigger a spurious space — the
emitted char there is the delimiter, not the sign. Verified
`"a-"-1` and `"a-"- -b` against the upstream JAR. `minify_neg_neg`
flips IGNORED → PASS. 6 new gap063_* unit tests.

## [0.75.0] - 2026-06-10

### Changed
- **CLOSES gap-061** — arg-bearing new-expression member wrap:
  `new A(y).b` → `(new A(y)).b`. Completes the new-expr-member
  family (gap-059 single-ident, gap-060 member-callee, gap-061
  arg-bearing).

### Added — gap-061 synthetic-paren pre-pass

Unlike gap-059/060 (which REORDER the empty arg-list parens),
the arg-bearing wrap has no spare parens, so this pass INSERTS
synthetic ones. Two grouping tokens — one `(` and one `)` —
are cloned from the source's own parens and declared before
`kept` so they outlive it; the pass inserts `&`-references (a
`(` before `new`, a `)` after the arg-list's depth-balanced
close). Reuses the gap-060 callee scan, so member-chain callees
(`new a.b.C(y,z).d`), multiple args, and nested-call args
(`new A(f(x)).b`) all wrap correctly. Guards: operator `new`
only; non-empty args; follower ∈ `.`/`[`/`(`; all checks via
`is_structural_punct`. 5 new gap061_* unit tests; the former
gap059_arg_bearing_new_deferred test is updated to assert the
wrapped form.

## [0.74.0] - 2026-06-10

### Changed
- **CLOSES gap-060** — member-callee new-expression wrap:
  `new a.b.C().d` → `(new a.b.C).d`. Generalizes gap-059 from a
  single-identifier callee to a member-chain callee.

### Changed — gap-059 pre-pass generalized

The new-expr wrap pre-pass (gap-059) now scans a CALLEE EXTENT
— the leading identifier plus zero-or-more `.IDENT` accessors —
before the empty `()`, instead of requiring exactly one
identifier. The single-identifier case (gap-059) is the
zero-accessor special case, so both are handled by one unified
pass that reorders the `(` to before `new` (no synthetic
tokens). Computed `[...]` callees and arg-bearing forms
(gap-061) stay deferred. 3 new gap060_* unit tests.

## [0.73.0] - 2026-06-10

### Changed
- **CLOSES gap-062** (minimal slice) — redundant double-paren
  collapse: `((a+b))*c` → `(a+b)*c`. One directly-nested
  grouping-paren layer is stripped.

### Added — gap-062 token pre-pass

When a GROUPING `(` is directly followed by another `(` and the
inner group's matching `)` is directly followed by the outer
`)` (purely-nested `(( ... ))`), the outer pair is dropped.
Guards: the outer `(` must be a grouping paren (so a CALL paren
like `f((a,b))` is never collapsed to `f(a,b)` — a different
program); no top-level comma inside; all bracket checks via
`is_structural_punct`.

Upstream eliminates parens more aggressively (`((a))` → `a`,
`(a)+(b)` → `a+b`, `f((a))` → `f(a)`); this slice strips only
one directly-nested grouping layer — the broader pass is a
follow-up. Four guard unit tests added.

## [0.72.0] - 2026-06-10

### Changed
- **CLOSES gap-059** (minimal slice) — member/call on a `new`
  expression now wraps in parens: `new A().b` → `(new A).b`.
  Upstream wraps because `new A.b` parses as `new (A.b)` (a
  different program), and drops the empty `()` arg list.

### Added — gap-059 token pre-pass

A new pre-pass wraps the new-expression WITHOUT synthesising
tokens: the empty arg-list `()` already contributes a `(` and a
`)`, so the pass just REORDERS them — moving the `(` to before
`new` (`new A ( ) .` → `( new A ) .`). Minimal safe slice:
single plain-identifier callee, empty arg list, followed by
`.`/`[`/`(`. Guards: operator `new` only (a property `.new` is
left alone), all bracket checks via `is_structural_punct`.
Complements gap-050 (which drops `new A()` → `new A` only when
NO member/call follows — exactly the cases this pass handles).

Verified against the upstream JAR: `new A().b` → `(new A).b`,
`.b.c`/`[i]`/`()` chains likewise wrap, standalone `new A()` →
`new A` (unchanged), and `a.new()` (property) is untouched.
Member-callee (`new a.b.C().d`) and arg-bearing (`new A(y).b`)
shapes are deferred follow-ups.

Updated three former gap-050 "keeps_parens" unit tests — they
asserted the pre-gap-059 (upstream-divergent) output and now
assert the wrapped form.

## [0.71.0] - 2026-06-10

### Changed
- **CLOSES gap-058** — the ES2021 `_` numeric separator is now
  stripped from FLOAT and scientific literals, not just integers:
  - `1_000.5` → `1000.5`
  - `1_0e3`   → `10e3`

  `normalize_number_value`'s float/scientific branch previously
  returned the literal verbatim (separators intact). It now
  returns `cleaned` — the value with every `_` already removed —
  so separators are stripped while the float/scientific *shape*
  is otherwise untouched. Full float shortest-form (`0.5` → `.5`,
  `1000e3` → `1E6`) remains a separate deferred gap. The
  separator is purely lexical sugar, so its removal needs no
  numeric reasoning and is always safe.

## [0.70.0] - 2026-06-10

### Changed
- **CLOSES gap-057** — member-object paren elision:
  `(a).b` → `a.b`. Upstream Closure (WHITESPACE_ONLY) strips
  the redundant grouping parens around a member-expression's
  object when that object is a single identifier.

### Added — gap-057 pre-pass + safety guards

A new token-stream pre-pass (separate from the gap-055/056
arm-elision block) drops the grouping parens in the shape
`GROUPING_PREFIX ( IDENT ) .`. Three guards make it
provably safe:

- **grouping-not-call guard** — the token before `(` must be
  a punctuation/operator other than `)`, `]`, or `?.`. This
  keeps CALL and index parens intact: `f(a).b` stays
  `f(a).b`, `x?.(a).b` (optional call) stays untouched.
- **single-identifier guard** — the parens must wrap exactly
  one plain-identifier token. Numbers are excluded
  (`(1).toString()` must keep its parens — `1.` mis-lexes);
  so are keywords, strings, regex, and templates.
- **member-position guard** — the token after `)` must be
  `.`. (`(a)[i]` and `(a)(x)` are also safe for a lone
  identifier but are left to a follow-up.)

All bracket/operator comparisons route through the existing
`is_structural_punct` guard, so a string literal whose
content looks like punctuation (e.g. `")"`) can never
corrupt the depth scan.

Helpers added: `is_punct` (operator-vs-value category test)
and `is_plain_identifier` (NAME/IDENT only).

## [0.69.0] - 2026-06-10

### Changed
- **CLOSES gap-056** — paren elision after the `return` /
  `throw` statement keywords and the concise-arrow `=>`
  prefix. Extends the gap-055 whole-arm peephole:
  - `return (a+b);`            → `return a+b;`
  - `throw (new Error("x"));`  → `throw new Error("x");`
  - `x => (x+1)`               → `x=>x+1`

### Added — prefix set + two new guards

The gap-055 pre-pass now also fires when the token before
`(` is `=>`, `return`, or `throw`. Two prefix-specific
safety guards:

- **property-name guard** — `return`/`throw` are stripped
  only as STATEMENT keywords, never as property names:
  `gen.throw((e))` / `it.return((b))` (preceded by `.`/`?.`)
  are left untouched.
- **arrow-brace guard** — a concise arrow body that starts
  with `{` is ambiguous (`x=>{...}` is a function BLOCK), so
  `()=>({a:1})` keeps its parens. After `?`/`:`/`return`/
  `throw` the operand is unambiguously an expression, so `{`
  is fine there (`return ({a:1})` → `return{a:1}`).

All comparisons continue to route through `is_structural_punct`
(the gap-055 literal-content guard).

## [0.68.0] - 2026-06-10

### Changed
- **CLOSES gap-055** — paren elision around a whole-arm
  sub-expression following `?` or `:`. Matches upstream on
  ternary arms (`x?(a=1):(b=2)` → `x?a=1:b=2`), object-literal
  values (`{a:(b+c)}` → `{a:b+c}`), and label/case bodies
  (`foo:(x);` → `foo:x;`).

### Added

Token-stream pre-pass: when prev is `?` or `:` and next is
`(`, scan to the matching `)`. Drop both parens iff:
- the token after `)` is an arm-terminator (`:`/`;`/`,`/`)`/
  `]`/`}`/EOF) — so the parens span the COMPLETE arm; and
- no top-level `,` inside (preserves the comma operator).

Whole-arm guard prevents precedence-shift bugs:
`x?(a=1)+2:c` stays (next-after-`)` is `+`). `?.` lexes as a
single OPTIONAL_CHAIN token so optional calls (`a?.(b)`) are
never matched. 9 edge cases verified against upstream.

(Skipping no versions — 0.68.0 follows 0.67.0.)

## [0.67.0] - 2026-06-10

### Changed
- **CLOSES gap-054** — paren elision around unary operand.
  `void(0);` → `void 0;` matches upstream. Also handles
  `typeof(x);` → `typeof x;`, `delete(o);` → `delete o;`.

Token-stream pre-pass for `KW ( SINGLE )` where KW is
`void`/`typeof`/`delete` and SINGLE is one safe token.
Conservative: multi-token operands left alone.

## [0.66.0] - 2026-06-10

### Changed
- **CLOSES gap-053** — paren elision around var-init RHS.
  `var t = (x == null);` → `var t=x==null;` matches upstream.

Token-stream pre-pass that scans for `= ( ... )` where the
contents have no `,` at depth 0, don't start with `function`,
and are followed by `;`/`,`/EOF.

## [0.65.0] - 2026-06-09

### Changed
- **CLOSES gap-052** — trailing `;` after `}` at EOF for
  `BlockKind::Other` (control-flow body, labeled block,
  bare block). `if(x){a;b;}` at EOF → `if(x){a;b};` matches
  upstream Closure.

## [0.64.0] - 2026-06-09

### Changed
- **CLOSES gap-051** — IIFE paren normalisation.
  `(function(){...}())` → `(function(){...})()` — the call
  `()` moves outside the wrapping parens. Same byte count;
  matches upstream Closure's preferred normalisation.

### Added

Token-stream pre-pass right after `kept` is built: scan for
`} ( ) )` 4-token sequence and rotate `[i+1..=i+3]` right by
1 to reorder to `} ) ( )`. Safe-by-construction — this
token sequence can ONLY appear in IIFE contexts in valid JS.

6 inline `gap051_*` tests covering target case + 4 explicit
non-regression cases (already-outer-call form, plain
function call, arrow IIFE, IIFE-with-args).

## [0.63.0] - 2026-06-09

### Changed
- **CLOSES gap-046b** — object literal / object destructuring
  trailing-comma elision. `{a:1,b:2,}` → `{a:1,b:2}`,
  `var {a,b,}=o` → `var {a,b}=o`.

### Added

Token-level peephole right after gap-046's `,`-before-`]`
drop: same shape, but for `,` before `}`. The drop is
unconditional — in valid ECMAScript, `,` immediately before
`}` can ONLY appear in object-literal / object-destructuring
contexts (block bodies, class bodies, switch bodies don't
allow `,` between members).

7 inline `gap046b_*` tests covering target case + 5 explicit
non-regression cases (no-comma forms, empty obj, call
trailing comma, nested objects).

## [0.62.0] - 2026-06-09

### Changed
- **CLOSES gap-050** — empty constructor arg-list elision.
  `new Foo()` → `new Foo` when followed by anything OTHER than
  member-access (`.`, `[`), chained call (`(`), or tagged
  template (`` ` ``). Those four cases would change parse
  precedence and are kept verbatim.

### Added

Token-level peephole right before the existing arrow-fn
parens-drop optimisation in `whitespace_only.rs`. Triggers when
`kept[idx-2] == "new"` AND `kept[idx-1]` is a simple identifier
AND the bracket pair is empty AND the follower is safe.

8 inline `gap050_*` tests including 5 explicit non-regression
cases (with-args, member-access, bracket-access, chained call,
paren-expr-constructor).

## [0.61.0] - 2026-06-09

### Changed
- **CLOSES gap-049** — gap-032's single-statement flatten now
  peeks the token after the closing `}`. When the next token
  is another `}`, the trailing `;` is suppressed from the
  inline emission.
- `function f(){for(var v of a){a;}}` → `function f(){for(var v of a)a};`
  (was: `function f(){for(var v of a)a;};`)
- Affects all loop/conditional flattening: `for`, `for-of`,
  `for-in`, `for-await-of`, `while`, `if` (and the inner arm
  of `if-else`). All produce one byte less when wrapped in a
  function body or another block.

### Fixed

`gap032_nested_if_does_not_flatten` expectation tightened —
`if(x){if(y){a();}}` now produces `if(x){if(y)a()}` (15 bytes)
instead of `if(x){if(y)a();}` (17 bytes). Both are valid JS;
the new form is one byte closer to upstream's
`if(x)if(y)a();` (16 bytes, requires also flattening through
the outer `if` keyword — a separate future improvement).

### Added

6 inline `gap049_*` tests covering for-of/for-in/while
flattening + 3 explicit non-regression cases (top-level
flatten preserves `;`, if-else inside function suppresses
correctly).

## [0.60.0] - 2026-06-09

### Changed
- **CLOSES gap-048** — BigInt literals with ES2021 `_`
  numeric separators now normalize: `1_000_000n` →
  `1000000n`, `0x1_FFFn` → `0x1FFFn`. The separator is
  pure lexical sugar; stripping it doesn't require
  bigint arithmetic (which keeps gap-038's bigint
  shortest-form deferred).

### Fixed

Two-line fix:
1. `is_number_literal` now recognizes `BIGINT` /
   `BIGINT_LITERAL` token-names. Without this gate,
   BigInt tokens never reached the normalize path.
2. `normalize_number_value`'s BigInt early-return now
   strips `_` from the body before re-appending `n`.

### Added

5 inline `gap048_*` tests + 1 byte-identity fixture
(`minify_bigint_separator`, flipped IGNORED → PASS).

## [0.59.0] - 2026-06-09

### Changed
- **CLOSES gap-047** — synthetic `;` after a `}` is now
  suppressed when the next non-trivia token is a
  statement-starting keyword. ASI cleanly covers that
  boundary; the `;` is wasted bytes. Harness now **87/89**
  — **only gap-044 (lexer-level template substitution)
  remains open**.
- Added a 5th branch to the `}` 4-way decision machine
  (gap-030/033/041): when `next_is_stmt_keyword`, neither
  emit nor defer.
- Keyword set: `var`, `let`, `const`, `function`, `class`,
  `if`, `for`, `while`, `do`, `switch`, `try`, `return`,
  `throw`, `break`, `continue`, `import`, `export`.
- **EOF (None)** is NOT in the set — the gap-030 trailing
  `;` after a final function-decl is preserved.

### Added

7 inline `gap047_*` tests including 5 explicit non-
regression cases (EOF still emits, close-brace defer
preserved, Other-block unaffected, return keyword,
trychain continuation).

## [0.58.0] - 2026-06-09

### Changed
- **CLOSES gap-046** (array case) — trailing comma in array
  literal is now suppressed. Harness 86/89. Only gap-044
  (lexer-level template substitution) and gap-047 (suppress
  synthetic `;` before stmt-keyword) remain.
- Top-of-loop check: when current is `,` AND next non-trivia
  is `]`, skip the comma. Handles `[1,2,]` → `[1,2]` and
  degenerate elision `[1,,]` → `[1,]` (matches upstream's
  lossy normalisation under WHITESPACE_ONLY).
- Object-literal case deferred to gap-046b.

### Added
- 6 inline `gap046_*` tests covering target, single-
  element, inner-comma non-regression, elision
  normalisation, call-expr non-regression, empty array.

## [0.57.0] - 2026-06-09

### Changed
- **CLOSES gap-045** — single-argument arrow function drops
  its enclosing parens. Harness now 79/81; only gap-044
  (template substitution, lexer-level) remains open.
- Added a top-of-loop pattern detector: when the current
  token is `(` AND `kept[idx+1]` is a Name AND
  `kept[idx+2]` is `)` AND `kept[idx+3]` is `=>`, emit just
  the IDENT and `=>`, advancing idx by 4. Both parens are
  skipped — the `(` push and `)` pop both bypassed, leaving
  paren_stack net-zero.
- Composes with `async` keyword: `var f=async(x)=>x+1;` →
  `var f=async x=>x+1;`. The `async` keyword + Name IDENT
  pair triggers `needs_separator` (both word-like → space).
- Added `is_simple_identifier_token` helper that returns
  true only for `TokenType::Name` — keywords, punctuation,
  strings, and numbers all fail. This filters out
  destructuring (`{`), rest (`...`), and reserved-word
  param names.

### Added

8 inline `gap045_*` tests:
- target single-arg arrow + async composition
- 6 non-regression cases: zero-arg, multi-arg, default,
  rest, destructuring, `(x).y` member access (not arrow)

### Pre-push security review

Verdict PASS. Traced 6 concerns: paren_stack balance,
other stack non-interaction, false-positive on `(x).y`,
async composition + needs_separator interaction, future
template-substitution composition, prev_emitted_tok stored
as the `=>` token (PUNCT) to avoid spurious space after
IDENT body.

## [0.56.0] - 2026-06-09

### Changed
- **CLOSES gap-043** — CLI quote-choice optimisation for
  string literals. **Harness now 57/57 PASS — sixth 100%
  milestone today** (17/17 → 25/25 → 33/33 → 41/41 → 49/49
  → 57/57).
- Added `emit_quoted_string(out, content)`: counts `"` vs
  `'` occurrences in content. When `"` count > `'` count,
  switches to single-quoted form (no `\"` escape needed
  for the content's `"`). Otherwise default to double
  (tie-break per upstream's `CodePrinter`).
- Mirrors the logic in `closure-emitter`'s
  `choose_quote_and_escape` (closed CLOC12 gap-026). The
  CLI path uses an independent copy because it doesn't go
  through the AST.
- Both string-emit sites in the WHITESPACE_ONLY path now
  call `emit_quoted_string` (main loop + gap-032 pre-emit).

### Added

4 new `gap043_*` inline tests:
- `gap043_no_quotes_in_content_picks_double` — default
  case
- `gap043_single_quotes_only_stay_double` — no escape
  savings from switching
- `gap043_more_double_switches_to_single` — target case
  (one `"`, no `'`)
- `gap043_tie_picks_double` — tie-break verification

## [0.55.0] - 2026-06-09

### Changed
- **CLOSES gap-042** — `do` keyword now arms
  `body_position_next = true`. `do{a;}while(x);` flattens
  via gap-032 to `do a;while(x);` matching upstream.
  Harness 55/57 → 56/57. Only gap-043 left.

Unlike `if`/`while`/`for` whose body slot opens after
their `)`, `do`'s body opens IMMEDIATELY after the keyword
(per §13.7.2). A one-keyword branch mirroring `else`.

### Added
- 2 inline `gap042_*` tests (single-stmt flatten target +
  multi-stmt non-regression).

### Documented (orthogonal, not fixed here)
- Empty-body `do{}while(x);` produces `do; while(x);` —
  the synthetic `;` from gap-031 doesn't update
  `prev_emitted_tok`, leaving `needs_separator` to see
  word-like(do, while). Future gap.

## [0.54.0] - 2026-06-09

### Changed
- **CLOSES gap-040** — numeric separator stripping +
  scientific shortest-form normalisation. **Harness now
  reports `49 matched, 0 failed, 0 skipped (of 49 total)`
  — fifth 100% milestone today** (17/17, 25/25, 33/33,
  41/41, 49/49).
- Extended `normalize_number_value()` to consider three
  candidates: cleaned form (with `_` stripped), decimal,
  and scientific. Picks the shortest with tie-break order
  decimal > cleaned > scientific.
- Added `scientific_form_of(n)` helper: for `n = m × 10^e`
  with `m % 10 ≠ 0` and `e ≥ 1`, returns `Some("{m}E{e}")`.
- Decimal source (no radix prefix) is now also considered
  for normalisation — `1000` → `1E3`, `12000` → `12E3`.
- Underscores in source are stripped before parsing
  (`u128::from_str_radix` doesn't accept ES2021 numeric
  separators).
- Floating-point and exponential-source literals (anything
  containing `.`, `e`, `E`) hit the early-return branch and
  stay verbatim — those normalisations are separate gaps.

### Added

12 new `gap040_*` inline tests covering:
- separator + scientific (`1_000`, `1_000_000`)
- separator without trailing zeros (`1_234_567`)
- hex + separator (`0xff_ff`)
- bare decimal → scientific (`1000`)
- decimal/scientific tie (`100`)
- tiny decimal stays (`10`)
- multi-digit mantissa scientific (`12000` → `12E3`)
- mantissa-exponent tie (`1234500`)
- zero/no-norm/float non-regression

### Verified against upstream JAR

All worked examples in the function docstring were verified
against `closure-compiler-v20240317.jar` directly during
implementation. Boundary cases confirmed:
- `1000` → `1E3` (sci strictly shorter)
- `100` → `100` (decimal-sci tie → decimal)
- `12000` → `12E3` (multi-digit mantissa)
- `1234500` → `1234500` (cleaned-decimal-sci tie → decimal)
- `0xff_ff` → `65535` (decimal strictly shortest)

## [0.53.0] - 2026-06-09

### Changed
- **CLOSES gap-041** — synthetic `;` propagation through
  closing braces. Harness now 48/49 (only gap-040 left).
- Introduced `deferred_synthetic_semi: bool` carried across
  iterations. When a `}` would emit a synthetic `;` but the
  next non-trivia is another `}`, the `;` is **deferred**
  to that outer brace. The outer brace then consumes the
  deferred state, collapsing with any own-`;` it would emit
  to a single output.
- 4-way decision at every `}`:
  1. owes + next-is-`}` → defer
  2. doesn't owe + next-is-`}` → propagate state
  3. next-is-`catch`/`finally` → carry across chain
  4. else → emit if `kind_wants_semi || deferred`, clear flag
- Verified against `closure-compiler-v20240317.jar` for:
  - `function f(){function g(){}}` → `function f(){function g(){}};`
  - `if(x){function f(){}}` → `if(x){function f(){}};`
  - `try{function f(){}}catch(e){b;}` → `try{function f(){}}catch(e){b};`
  - `try{try{a;}catch(e){b;}}catch(f){c;}` → `try{try{a}catch(e){b}}catch(f){c};`

### Updated

Four pre-existing inline tests had encoded the buggy
double-`;` output as their expected rhs:
- `gap032_body_with_function_does_not_flatten`
- `gap032_body_with_try_does_not_flatten` (partial — gap-032
  conservatism still keeps the outer braces)
- `gap033_function_decl_inside_try_block_still_gets_semi`
- `gap033_nested_try_catch_each_gets_semi`

All four updated to the upstream-matching form with
references to the JAR probe in their docstrings.

### Pre-push security review

Verdict PASS. Traced 7 concerns: source-`;` non-interference,
sequential `}}}}` collapse, Function-Other mix, Rule A
interaction, EOF handling, multi-defer collapse, TryChain
across non-`}` boundary. No counterexample.

## [0.52.0] - 2026-06-09

### Changed
- **CLOSES gap-039** — tagged template literal needs no
  separator between the tag function (IDENT) and the
  template's opening `` ` ``. Harness back to **41/41 PASS**
  — fourth 100% milestone in this rolling marathon.
- Added a short-circuit at the top of `needs_separator`:
  when next token's value starts with `` ` ``, return
  false unconditionally. Runs BEFORE the word-like rule.
- Matches §13.3.11 grammar:
  `TaggedTemplateExpression → MemberExpression TemplateLiteral`
  forbids whitespace between the two.

### Added
- 4 new `gap039_*` inline tests:
  - target fixture (`tag\`hi\`` round-trip)
  - member access after tagged template
  - bare (untagged) template still works
  - composition with gap-035 (`var{a}=tag\`hi\`;`)

## [0.51.0] - 2026-06-08

### Changed
- **CLOSES gap-038** — hex/oct/bin numeric literal
  shortest-form normalisation. **The byte-identity harness
  now reports `33 matched, 0 failed, 0 skipped (of 33
  total)`** — third 100% milestone today (after 17/17 and
  25/25 earlier).
- Added `normalize_number_value()`: detects hex/oct/bin
  prefixes, parses to `u128`, formats as decimal, emits
  whichever is shorter (tie-break to decimal). Tie-break
  rule verified directly against
  `closure-compiler-v20240317.jar`: `0xffffffff` (10 chars)
  → `4294967295` (10 chars, ties → decimal),
  `0xfffffffff` (11 chars) → `68719476735` (11 chars, ties
  → decimal).
- Added `is_number_literal()` helper for grammar-name
  detection (mirrors `is_string_literal`).
- Wired the normaliser into both emit sites: the main loop
  emit and the gap-032 single-statement pre-emit pathway.

### Added
- 9 new `gap038_*` inline tests covering: hex short, hex
  tie picks decimal, hex kept when shorter (14-char hex vs
  15-char decimal), octal, binary, uppercase prefix (`0X`),
  decimal unchanged, BigInt verbatim, overflow safety.

### Limitations carried forward (each becomes its own gap if a fixture surfaces it)
- **BigInt literals** (`0xfn`) need bigint arithmetic.
  Left verbatim; upstream emits `15n`.
- **Decimal floating-point shortest-form** (`0.5` → `.5`,
  `10.0` → `10`).
- **Scientific notation uppercasing** (`1e3` → `1E3`).
- **u128 overflow**: hex literals exceeding `u128::MAX` stay
  verbatim rather than panicking.

## [0.50.0] - 2026-06-08

### Changed
- **CLOSES gap-037** — async function declaration trailing `;`.
  The byte-identity harness now reports
  `32 matched, 0 failed, 1 skipped (of 33 total)` — only
  gap-038 (hex literal → decimal normalisation) remains.
- Added `saw_async_kw_at_boundary` flag. `async` keyword at
  a statement boundary arms it ONLY when the very next
  non-trivia token is `function` (gate). The next
  `function` keyword consumes the flag and arms
  `saw_function_kw_at_boundary` as if `function` itself had
  been at the boundary. The matching `}` then emits the
  gap-030 trailing `;`.
- The `function` keyword arm is now `at_stmt_boundary
  || saw_async_kw_at_boundary` (was just `at_stmt_boundary`),
  and the async flag is cleared whether or not the function
  branch fires.

### Added
- 5 new inline `gap037_*` tests:
  - `gap037_async_function_trailing_semi` — target fixture
  - `gap037_empty_async_function_trailing_semi`
  - `gap037_async_method_shorthand_does_not_arm` —
    non-regression: `{async f(){}}` doesn't arm.
  - `gap037_async_arrow_does_not_arm` — non-regression:
    `async()=>x` doesn't arm.
  - `gap037_async_function_expression_no_trailing_semi` —
    non-regression: `var f=async function(){};` doesn't get
    extra `;`.

### Design — keyword-arming guard family

This continues the keyword-as-property defense pattern
established by gap-033 (`try`), gap-034 (`class`), and
gap-036 (`switch`): never arm a keyword flag without
checking the next non-trivia token. The `async` keyword is
particularly important to guard because it's NOT a reserved
word in ES (it's a contextual keyword), so users can name
methods `async` legally. The guard requires next-is-`function`
which is grammatically necessary for the async-function-decl
shape (per §15.8) and surgically excludes async arrow
functions, async methods, and async-named properties.

## [0.49.0] - 2026-06-08

### Changed
- **CLOSES gap-034, gap-035, gap-036** in a single PR. The
  byte-identity harness now reports **`25 matched, 0 failed,
  0 skipped (of 25 total)`** — full parity with upstream
  Closure v20240317 across the expanded 25-fixture seed set.
- **gap-034**: class declaration trailing `;`. Added
  `BlockKind::Class` variant. `class` keyword at a statement
  boundary arms `saw_class_kw_at_boundary`. The next `{`
  consumes the flag and pushes `BlockKind::Class`. On the
  matching `}`, a synthetic `;` is appended — mirroring
  upstream's normalisation of `class C{m(){}}` to
  `class C{m(){}};`.
- **gap-035**: `var`/`let`/`const` followed by `{`/`[`
  (destructuring) gets a space inserted. Extended
  `needs_separator` with a 3-keyword whitelist. Without
  this, `var{a}=x;` round-trips identical; upstream emits
  `var {a}=x;` (with space) to match its own preference.
- **gap-036**: switch statement trailing `;`. Added
  `BlockKind::Switch` variant + a parallel
  `paren_is_switch_stack`. `switch` keyword (when followed
  by `(`) arms `next_paren_is_switch_head`. The matching
  `)` arms `next_block_is_switch_body`. The next `{` pushes
  `BlockKind::Switch`. On the matching `}`, a synthetic `;`
  is appended — mirroring upstream's
  `switch(x){...};` shape.

### Pre-push security review caught the keyword-as-property bug family

Round-1 verdict identified a real correctness failure: when
`class` appears as an OBJECT-LITERAL PROPERTY NAME (e.g.
`var o={class:1};do{y}while(x);`), the original cut armed
`saw_class_kw_at_boundary` unconditionally on
`at_stmt_boundary`. The flag would then leak forward and
contaminate the next unrelated `{` — specifically breaking
do/while grammar by emitting `do{y};while(x);` (the
spurious `;` after the do-body's `}` terminates the
do-statement and orphans the `while(x)` clause). Same
defect family as the `try`-as-property bug from CLOC12.40.

A parallel-but-non-fatal defect existed for `switch` as a
property name — it would leak the switch-head flag and add
spurious trailing `;`s to unrelated while/if bodies.

### Fix

Two guard refinements added:

- `class` keyword: only arm when `at_stmt_boundary` is true
  AND the next non-trivia token "looks like" a class-decl
  continuation (`{`, `extends`, or an identifier — i.e. NOT
  `:`, `,`, `;`, `}`, `)`, `]`, `.`, `=`, `(`). This filters
  property-name (`class:1`), method shorthand
  (`class(){...}`), member-access (`obj.class`), and other
  expression-position uses.
- `switch` keyword: only arm when the next non-trivia token
  is `(` — which is grammatically required per §13.12. Same
  shape as gap-033's `try` guard.

### Added

- 13 new inline tests in `whitespace_only::tests::gap03[456]_*`:
  - **gap-034:** class decl trailing `;`, empty class body,
    class expression doesn't get `;`, **regression for the
    security-review bug**: `class` as object-property
    doesn't arm.
  - **gap-035:** `var`/`let`/`const` destructuring with `{`,
    `var` with `[` (array destructuring), simple `var x=1`
    unchanged.
  - **gap-036:** switch trailing `;`, default-clause shape,
    **regression for the security-review bug**: `switch` as
    object-property doesn't arm.

## [0.48.0] - 2026-06-08

### Changed
- **CLOSES gap-032** — single-statement if/else block
  flattening at the CLI WHITESPACE_ONLY layer.
  `minify_if_else` flipped IGNORED → PASS. **The byte-
  identity harness now reports `17 matched, 0 failed,
  0 skipped (of 17 total)`** — the entire seed set is byte-
  for-byte identical to upstream Closure v20240317.
- When `whitespace_only_minify` encounters a `{` token in
  body position (i.e. `body_position_next == true`), it
  scans forward to find the matching `}` and checks
  eligibility:
  - matching `}` found,
  - exactly one `;` at depth 0 inside the block,
  - no nested `{` at depth 0,
  - no `function` / `try` / `if` / `while` / `for` / `do` /
    `switch` / `class` keyword at depth 0,
  - the token immediately before the close-`}` IS `;`.
  When all hold, the inner content tokens are pre-emitted
  directly (bypassing the main loop's rule A and other
  state-machine logic), then both braces are skipped.
- `else` keyword now also arms `body_position_next = true`
  so the else-clause body can be a flatten target.

### Added
- 13 new inline tests in `whitespace_only::tests::gap032_*`:
  - **Positive (rule fires):**
    - `gap032_if_else_single_stmts_flatten` (target fixture)
    - `gap032_if_single_stmt_flattens`
    - `gap032_while_single_stmt_flattens`
    - `gap032_for_single_stmt_flattens`
    - `gap032_body_with_var_decl_flattens`
  - **Non-regression (rule does NOT fire):**
    - `gap032_multi_stmt_body_does_not_flatten`
    - `gap032_nested_if_does_not_flatten` (dangling-else
      safety)
    - `gap032_nested_brace_does_not_flatten`
    - `gap032_body_with_function_does_not_flatten`
    - `gap032_body_with_try_does_not_flatten`
    - `gap032_top_level_block_does_not_flatten`
    - `gap032_function_body_does_not_flatten`
    - `gap032_try_body_does_not_flatten`

### Updated
- Two pre-existing tests had their expectations updated to
  reflect the new, MORE correct behaviour gap-032 introduces:
  - `gap030_if_block_drops_inner_semi_no_trailing`:
    `if(x){y();}` was previously expected as `if(x){y()}`
    (just the inner `;` dropped). With gap-032 it now
    flattens to `if(x)y();` — matching upstream.
  - `gap031_nonempty_for_body_unaffected`:
    `for(...){a;}` was previously expected as
    `for(...){a}`. Now flattens to `for(...)a;` — matching
    upstream.

### Pre-push security review

Verdict PASS. Six concerns traced through the change:

1. Dangling-else binding — preserved by the
   `has_blocking_keyword` guard which keeps outer braces.
2. String literals containing `;`/`{`/`}` — the scan
   matches token values like `"x}y" != "}"`, so depth
   tracking is safe.
3. Regex literals — same safety as strings.
4. Comments — already stripped pre-scan.
5. `prev_emitted_tok` after flatten — points to the
   trailing `;`, which is not word-like, so next token gets
   no spurious separator.
6. `paren_stack` / `brace_stack` invariants — eligibility
   requires balanced parens and no nested braces inside the
   block, so pre-emit doesn't corrupt the stacks.

## [0.47.0] - 2026-06-08

### Changed
- **CLOSES gap-031**: empty `{}` body collapses to `;`
  (EmptyStatement substitution per ECMAScript §13.2). When
  the CLI WHITESPACE_ONLY re-stitcher encounters `{` with
  `body_position_next` true (i.e. it's the body slot of a
  `for`/`while`/`if`/`labeled` after the closing `)`) AND the
  next non-trivia token is `}`, the emitter writes a single
  `;` in place of the `{}` pair and skips both braces.
- `minify_for_loop` flipped IGNORED → PASS. Harness now
  reports `16 matched, 0 failed, 1 skipped (of 17 total)`;
  only gap-032 (single-statement if/else block flattening)
  remains open.

### Added
- 8 new inline tests in `whitespace_only::tests::gap031_*`:
  - **Positive (rule fires):**
    - `gap031_empty_for_body_collapses` — the target fixture.
    - `gap031_empty_while_body_collapses` — `while(x){}` → `while(x);`.
    - `gap031_empty_if_body_collapses` — `if(x){}` → `if(x);`.
  - **Non-regression (rule does NOT fire):**
    - `gap031_function_empty_body_unaffected` — function-decl
      body stays `{}` (no control-flow paren).
    - `gap031_nonempty_for_body_unaffected` — `{...}` with
      content stays as is.
    - `gap031_top_level_empty_block_unaffected` — `{}` at
      statement position (not body position) stays as is.
    - `gap031_empty_object_literal_unaffected` — `{}` in
      expression position stays as is.
    - `gap031_try_empty_body_unaffected` — `try{}` body stays
      `{}` (try doesn't have a `(...)` head); composes
      correctly with gap-033 try-chain processing.

## [0.46.0] - 2026-06-08

### Changed
- **CLOSES gap-033**: try/catch trailing `;` after `}`. The
  CLI WHITESPACE_ONLY token re-stitcher now emits a synthetic
  `;` after the last clause of a try/catch/finally chain,
  mirroring upstream Closure v20240317's behaviour and same
  family as gap-030's function-decl trailing `;`.
- `brace_stack` was refactored from `Vec<bool>` to
  `Vec<BlockKind>` with three variants: `Function`,
  `TryChain`, `Other`. The new `BlockKind::TryChain` is pushed
  whenever a `{` immediately follows a `try` / `catch` /
  `finally` keyword (tracked via a new
  `next_block_is_try_chain` flag).
- When a `}` pops a `BlockKind::TryChain`, the emitter peeks
  the very next non-trivia token. If it's `catch` or `finally`
  the chain continues, no `;` is emitted. Otherwise the chain
  has ended and a synthetic `;` is appended (and tracked by
  `last_emit_was_synthetic_semi` so rule-C dedup applies).
- `minify_try_catch` flipped from IGNORED → PASS. The
  `diff_minify` harness now reports `15 matched, 0 failed,
  2 skipped (of 17 total)` (gap-031 and gap-032 remain).

### Added
- 6 new inline tests in `whitespace_only::tests::gap033_*`:
  - `gap033_try_catch_gets_trailing_semi` — the target fixture
    shape.
  - `gap033_try_finally_gets_trailing_semi` — try/finally
    (no catch).
  - `gap033_try_catch_finally_only_final_semi` — only the
    LAST `}` in a 3-clause chain gets `;`.
  - `gap033_nested_try_catch_each_gets_semi` — depth
    regression; brace_stack must track all open chains.
  - `gap033_function_decl_inside_try_block_still_gets_semi` —
    `BlockKind::Function` and `BlockKind::TryChain` don't
    interfere; nested `function f(){}` inside a try-body still
    gets its gap-030 trailing `;`.
  - `gap033_optional_catch_binding` — ES2019 `catch{...}`
    without `(e)` binding.

## [0.45.0] - 2026-06-08

### Changed
- **gap-030 part 2 (CLI WHITESPACE_ONLY side): CLOSES gap-030.**
  Ports the same ASI-policy rules CLOC12.38 added to the AST
  emitter to closurec's CLI token re-stitcher
  (`src/whitespace_only.rs`). The previously-IGNORED
  `minify_function_decl` byte-identity fixture now PASSes
  against upstream Closure v20240317.
- The simple for-loop re-stitcher is replaced with a
  state-machine while-loop tracking:
  - `brace_stack: Vec<bool>` — for each open `{`, true iff it
    opens a function-declaration body.
  - `paren_stack: Vec<bool>` — for each open `(`, true iff it
    is the head of an `if`/`while`/`for`.
  - `body_position_next` — set on `)` popping a control-flow
    head; suppresses the rule-A `;`-drop so we never strip an
    `EmptyStatement` body (`if(x);`, `while(x);`, `for(;;);`).
  - `saw_function_kw_at_boundary` — captured on `function` at
    a statement boundary; consumed by the next `{`. Only
    DECLARATIONS get the rule-B trailing `;` (function
    expressions like `var f=function(){};` don't).
  - `last_emit_was_synthetic_semi` — rule-C dedup so
    `function f(){};var g=1;` stays as `};var` instead of
    `};;var`.

### Added
- 10 new inline tests in `whitespace_only::tests::gap030_*`.

### Fixed
- `tests/diff/whitespace-only/expected.stdout` updated. The
  pre-existing golden was hand-traced from closurec's old
  shape (`function add(a,b){return a+b;}`); upstream Closure
  v20240317 actually emits `function add(a,b){return a+b};`.
  Re-captured from the upstream JAR.

## [0.44.0] - 2026-06-02

### Added — CLOC14: end-to-end byte-identity test harness

Introduces the missing instrument for measuring real upstream parity.
Until now, every gap-fix PR was theoretical: unit tests went green but
the *composed* compiler's output was never compared byte-for-byte
against Google Closure Compiler's. CLOC14 closes that loop.

- **`tests/diff_minify.rs`**: single discovery-based test runner. Walks
  `tests/diff/` at test time, runs every `minify_*` fixture through the
  closurec binary, compares stdout against `expected.stdout`, and panics
  with an aggregate report if any non-ignored fixture diverged.
- **Verdict model**: each fixture reports `Match` / `Diverge` / `Error` /
  `Skipped`. Skipped fixtures are listed in `IGNORE_FIXTURES` with a
  documented reason; this is intentionally an embarrassment that shrinks
  as gaps close.
- **Adding a fixture is data, not code**: drop a `minify_<name>/`
  directory with `flags.txt`, `input/*`, `expected.stdout`, and a
  README. The runner picks it up automatically — no Rust changes.
- **Seed fixture set (v0.1)**:
  - `minify_minimal_var` (PASS) — `var x=1;` round-trip pins
    trailing-newline contract + lex/parse/emit identity.
  - `minify_string_literal` (PASS) — `var x="hi";` pins quote-style
    preservation under WHITESPACE_ONLY.
  - `minify_two_statements` (PASS) — `var x=null;var y=1;` pins
    statement-separator behaviour and null-literal round-trip.
  - `minify_empty` (IGNORED) — first real divergence surfaced by the
    harness: closurec emits `\n` for empty input; upstream may emit
    zero bytes. Pinned as IGNORED with reason until a real upstream
    capture resolves it.

Spec: `code/specs/CLOC14-byte-identity-harness.md`.

## [0.43.0] - 2026-05-31

### Added — CLOC11.80: end-to-end integration test for JSON summary format

New integration test `tests/diff_cv_json_summary.rs` exercises the CLOC11.74 JSON summary format end-to-end via the actual binary:

```bash
closurec \
  --correlation_vector \
  --correlation_vector_summary \
  --correlation_vector_summary_format JSON \
  --js tests/diff/cv-json-summary/input/a.js \
  --js_output_file <tmpdir>/out.js
```

### Contract pinned

1. stdout contains a single-line JSON object that parses cleanly via `serde_json::from_str`.
2. Top-level key is `cv_sidecar`; its value is an object.
3. The object has all six expected fields with the right JSON types:
   - `path`: string (non-empty when a sidecar was written)
   - `skipped`: bool (false when written)
   - `entries`: integer
   - `contributions`: integer
   - `tombstones`: integer
   - `pass_order`: array of strings
4. closurec exits 0 and writes the JS output normally.

### Why a separate integration test

CLOC11.74 unit tests verify the JSON serializer in isolation. This test drives it through the full binary path (CLI parse → wire → `run_compiler` → `summary_line` → stdout) and parses the result back with serde_json, so any drift that breaks JSON well-formedness (e.g. an unescaped path containing a quote) shows up here immediately. Completes the trio (TEXT covered by 11.77, KV by 11.79, JSON by 11.80).

### Changed

- Versions: `Cargo.toml` `0.42.0` → `0.43.0`, `cli.spec.json` `0.42.0` → `0.43.0`.

## [0.42.0] - 2026-05-31

### Added — CLOC11.79: end-to-end integration test for KV summary format

New integration test `tests/diff_cv_kv_summary.rs` exercises the CLOC11.74 KV summary format end-to-end via the actual binary:

```bash
closurec \
  --correlation_vector \
  --correlation_vector_summary \
  --correlation_vector_summary_format KV \
  --js tests/diff/cv-kv-summary/input/a.js \
  --js_output_file <tmpdir>/out.js
```

### Contract pinned

1. stdout contains the CV summary in space-separated `key=value` form, every key prefixed with `cv_sidecar.`.
2. Path RHS is quoted (`cv_sidecar.path="..."`) so shell tooling splitting on whitespace can recover the (possibly-spaced) path safely.
3. Numeric and bool RHS values are bare (`cv_sidecar.entries=N`, `cv_sidecar.skipped=false`) — `awk`/`cut` consumers don't have to strip quotes.
4. `pass_order` is quoted (comma-joined list would otherwise look like multiple keys).
5. closurec exits 0 and writes the JS output normally — KV summary coexists with a real compile.

### Why a separate integration test

CLOC11.74 unit tests verify the KV serializer in isolation. This test drives it through the full binary path (CLI parse → wire → `run_compiler` → `summary_line` → stdout), catching layer drift that per-feature unit tests would miss.

### Changed

- Versions: `Cargo.toml` `0.41.0` → `0.42.0`, `cli.spec.json` `0.41.0` → `0.42.0`.

## [0.41.0] - 2026-05-30

### Added — CLOC11.78: end-to-end integration test for NDJSON CV sidecar

New integration test `tests/diff_cv_ndjson_streaming.rs` exercises the CLOC11.69 NDJSON format end-to-end:

```bash
closurec \
  --correlation_vector \
  --correlation_vector_format NDJSON \
  --js tests/diff/cv-ndjson-streaming/input/a.js \
  --js_output_file <tmpdir>/out.js
```

### Contract pinned

1. Sidecar lands at `<js_output_file>.cv.json` (the CLOC11.67 default-path policy).
2. Sidecar is **newline-delimited JSON** — every non-empty line parses standalone.
3. At least 2 lines (≥1 entry + the `_meta` footer).
4. Final line is the `{"_meta": {"pass_order": [...], "enabled": ...}}` footer so streaming consumers (`tail -f`, `jq`) can reliably extract `pass_order` after the producer finishes.
5. closurec exits 0.

### Why this exists as its own integration test

CLOC11.69 unit tests verify `format_cv_log_ndjson` in isolation. This test exercises the full path — CLI parse → wire → `run_compiler` → formatter → disk write → consumer-style readback — through the actual binary. Catches drift in any of those layers, especially path resolution and the `--js_output_file`-sibling sidecar convention.

### Changed

- Versions: `Cargo.toml` `0.40.0` → `0.41.0`, `cli.spec.json` `0.40.0` → `0.41.0`.

## [0.40.0] - 2026-05-30

### Added — CLOC11.77: end-to-end integration test for CV pure-analysis combo

New integration test `tests/diff_cv_pure_analysis.rs` exercises the combo built up across CLOC11.60 → 11.76:

```
--correlation_vector              (CLOC11.60)
--correlation_vector_summary      (CLOC11.73)
--correlation_vector_summary_only (CLOC11.76)
--correlation_vector_format NONE  (CLOC11.69)
```

Contract pinned:
1. With `--correlation_vector_summary_only`, no JS file lands on disk even though `--js` is supplied.
2. With `--correlation_vector_format NONE`, no CV sidecar lands either — pure in-memory analysis, no writes.
3. The CV summary line still makes it to stdout because `--correlation_vector_summary` is on and `summary_stderr` is off (default).
4. closurec exits 0 — pure-analysis is a normal successful invocation.

### Why this exists as a separate integration test

The combination touches CLI parsing, wire reading, four config fields, three skip-gates in `run_compiler`, and the summary serializer. A single end-to-end test through the actual binary catches integration drift that per-feature unit tests would miss — e.g. a future refactor that splits `SpecialModesConfig` and forgets to thread one of the four flags would fail here even if every isolated test still passed.

### Changed

- Versions: `Cargo.toml` `0.39.0` → `0.40.0`, `cli.spec.json` `0.39.0` → `0.40.0`.

## [0.39.0] - 2026-05-30

### Added — CLOC11.76: `--correlation_vector_summary_only` (pure analysis mode)

Boolean flag (default false). When on, the run **skips every output write**: no JS file (or stdout), no source map, no manifest. The CV log is still computed in memory, so `--correlation_vector_summary` can still print real counts.

Pairs naturally with `--correlation_vector_format NONE` to skip the CV sidecar too — a pure-analysis invocation that does no disk writes whatsoever. With both set, the only externally observable output is the summary line on stdout (or stderr under CLOC11.75).

Use case: `closurec --correlation_vector --correlation_vector_summary --correlation_vector_summary_only` answers "what would the CV trace look like" without rebuilding artifacts.

### Implementation

- `SpecialModesConfig` gains `correlation_vector_summary_only: bool`.
- Three call sites in `run_compiler` are gated on `!summary_only`: the JS output write, the source map write, and the manifest write. The matching `js_output_file` / `source_map_output` / `manifest_output` CV records are also skipped (they describe writes that didn't happen).
- The CV sidecar write block (Step 7) is unchanged — `--correlation_vector_format NONE` is the right way to skip it, summary_only doesn't reach into that policy.
- Default false → byte-identical to CLOC11.75.

### Changed

- `wire.rs`: `read_special_modes` pulls the new bool.
- Versions: `Cargo.toml` `0.38.0` → `0.39.0`, `cli.spec.json` `0.38.0` → `0.39.0`.

## [0.38.0] - 2026-05-30

### Added — CLOC11.75: `--correlation_vector_summary_stderr`

Boolean flag (default false). When on, the `--correlation_vector_summary` line is routed to `stderr_text` instead of `stdout_text`. Useful when stdout carries the actual JS payload (no `--js_output_file`) — without this flag, the summary line would corrupt the JS that downstream tooling pipes into.

### Changed

- `CompilerOutput` gains `stderr_text: String` (default empty). Existing callers ignoring stderr see no behavior change; tests can assert on routing without grepping file descriptors.
- `parse_and_run`'s contract is now: returns `(stdout_text + stderr_text, ExitCode)` for back-compat with existing callers. A new `parse_and_run_with_streams` returns `(stdout, stderr, ExitCode)` separately; `main()` calls the streaming variant and writes stderr via `eprint!`.
- `SpecialModesConfig` gains `correlation_vector_summary_stderr: bool`.
- Versions: `Cargo.toml` `0.37.0` → `0.38.0`, `cli.spec.json` `0.37.0` → `0.38.0`.

### Implementation note

Why split the CompilerOutput rather than threading an `io::Write`: keeping run_compiler pure (no I/O, returns a value) preserves the existing test ergonomics — tests inspect the returned struct. The cost is one extra `String` field that's empty on the common path.

## [0.37.0] - 2026-05-30

### Added — CLOC11.74: `--correlation_vector_summary_format` enum (TEXT | JSON | KV)

Machine-readable rendering for the CLOC11.73 summary line. Lets CI/build pipelines consume the summary without regex-matching the human-readable text.

- `TEXT` (default) — CLOC11.73 line: `cv sidecar: <path>: N entries, M contributions, T tombstones, pass_order=[a,b,c]`
- `JSON` — single-line JSON object:
  ```json
  {"cv_sidecar":{"path":"<path>","skipped":false,"entries":N,"contributions":M,"tombstones":T,"pass_order":["a","b","c"]}}
  ```
  Under format=NONE: `"path": null, "skipped": true`.
- `KV` — space-separated `key=value`:
  ```
  cv_sidecar.path="<path>" cv_sidecar.skipped=false cv_sidecar.entries=N cv_sidecar.contributions=M cv_sidecar.tombstones=T cv_sidecar.pass_order="a,b,c"
  ```
  Path and pass_order are quoted on the RHS via `serde_json::to_string` so shell tooling can split on whitespace safely.

Flag is only consulted when `--correlation_vector_summary` is also on. With summary off, the format selector is dead.

### Implementation

- `compute_cv_summary` and `summary_line` gain a `summary_format: CorrelationVectorSummaryFormat` parameter. The count walk is unchanged; only the terminal rendering branches.
- JSON and KV use `serde_json` for string escaping (paths can contain quotes, backslashes, control chars on weird filesystems — let serde handle it).
- New public enum `CorrelationVectorSummaryFormat` with `#[default] = Text`.

### Changed

- `SpecialModesConfig` gains `correlation_vector_summary_format: CorrelationVectorSummaryFormat`.
- `wire::read_special_modes` maps the string value to the enum; unknown / empty falls back to `Text`.
- Versions: `Cargo.toml` `0.36.0` → `0.37.0`, `cli.spec.json` `0.36.0` → `0.37.0`.

## [0.36.0] - 2026-05-30

### Added — CLOC11.73: `--correlation_vector_summary` stdout one-liner

Boolean flag. When on, prints a single summary line to stdout (`CompilerOutput.stdout_text`) after the CV sidecar write (or skipped under `--correlation_vector_format NONE`). Lets build pipelines see how many entries / contributions / tombstones the run produced without parsing the JSON itself.

Output format:

```
cv sidecar: <path>: N entries, M contributions, T tombstones, pass_order=[a,b,c]
```

When the format is `NONE`:

```
cv sidecar: skipped (format=NONE): N entries, M contributions, T tombstones, pass_order=[a,b,c]
```

### Counts are post-filter

`compute_cv_summary` calls the same `prune_entries_by_source` helper the formatters use (with the same `include_origin` and `invert` flags), so the printed counts describe **what's actually on disk** when a filter is in play.

### Composition

- Default off → no change to existing stdout output.
- Composes orthogonally with every other CV flag — `summary` reads what the formatters wrote, it doesn't second-guess them.
- Trails the JS / source map / manifest stdout (only fires when CV is on; the line ends in `\n`).

### Implementation

- New private `compute_cv_summary(cv_log, filter, include_origin, invert, wrote_path)` returns the rendered line. Parses `cv_log.to_json_string()` once, applies the filter, counts entries / contributions / tombstones, extracts `pass_order`.
- `summary_line` helper formats the rendered string; isolated from the count walk so format changes don't bleed into the counting path.
- `result.stdout_text` is now bound `mut` to allow the summary append.

### Changed

- `SpecialModesConfig` gains `correlation_vector_summary: bool`.
- `wire::read_special_modes` reads the bool.
- Versions: `Cargo.toml` `0.35.0` → `0.36.0`, `cli.spec.json` `0.35.0` → `0.36.0`.

## [0.35.0] - 2026-05-30

### Added — CLOC11.72: `--correlation_vector_filter_invert`

Boolean flag that flips the CLOC11.70 allowlist into a **blocklist**. With:

```bash
closurec --correlation_vector \
         --correlation_vector_filter lex \
         --correlation_vector_filter_invert
```

entries that DO match (i.e. carry a `lex` contribution and/or — under `--correlation_vector_filter_includes_origin` — a `lex` origin) are dropped; everything else is kept.

Use case: "everything except X" filters where enumerating the allowlist would be impractical (the lexer alone produces many `lexer_token` entries; flipping to `--correlation_vector_filter_invert` lets you exclude one stage instead of listing every other).

### Composition

Invert composes orthogonally with `include_origin`:
- `include_origin` selects WHICH sources count as a match (contribution.source only, or contribution.source ∪ origin.source).
- `invert` then decides whether matches are kept or dropped.

| include_origin | invert | Behavior                                                  |
|----------------|--------|-----------------------------------------------------------|
| false          | false  | CLOC11.70 strict allowlist on contribution.source         |
| true           | false  | CLOC11.71 broadened allowlist (also origin.source)        |
| false          | true   | CLOC11.72 blocklist on contribution.source                |
| true           | true   | CLOC11.72 blocklist on contribution.source ∪ origin.source |

### Implementation

- `prune_entries_by_source` signature: now `(root, allowlist, include_origin, invert)`. The match-computation logic is unchanged; only the keep-rule terminal expression is `if invert { !matches } else { matches }`.
- Both `format_cv_log_json` and `format_cv_log_ndjson` thread the flag through. Empty-allowlist short-circuit unchanged: an inverted empty filter is a no-op (no entry can match → none are blocked), so we keep the fast path.

### Changed

- `SpecialModesConfig` gains `correlation_vector_filter_invert: bool`.
- `wire::read_special_modes` reads the bool.
- Versions: `Cargo.toml` `0.34.0` → `0.35.0`, `cli.spec.json` `0.34.0` → `0.35.0`.

## [0.34.0] - 2026-05-30

### Added — CLOC11.71: `--correlation_vector_filter_includes_origin`

Adds an opt-in sub-flag to extend the CLOC11.70 filter so it also matches against each entry's `origin.source`, not just `contribution.source`.

Default `false` preserves CLOC11.70's strict semantics byte-for-byte: only entries with a contribution whose source is in the allowlist survive.

With `--correlation_vector_filter_includes_origin true` and `--correlation_vector_filter lex`, an entry is kept iff:

1. any element of `contributions` has `source` in the allowlist (the CLOC11.70 rule), OR
2. the entry's `origin.source` is in the allowlist.

This is how you get a `--correlation_vector_filter lex` invocation to also retain the per-token CV entries created with `Origin{source: "lexer_token", ...}` — their "lex" association lives in the Origin, not in a contribution.

### Why opt-in rather than the new default

Default-on would silently change the result of every existing `--correlation_vector_filter X` invocation. Default-off keeps CLOC11.70 unchanged; users who want the broader match flip the flag. Standard backward-compat policy for evolving CLI semantics.

### Changed

- `SpecialModesConfig` gains `correlation_vector_filter_includes_origin: bool`.
- `wire::read_special_modes` reads the bool flag.
- `prune_entries_by_source` signature: now `(root, allowlist, include_origin)`. Inline doc updated; the contribution-match branch still short-circuits before the Origin check so the fast path is unchanged.
- Both `format_cv_log_json` and `format_cv_log_ndjson` thread the flag through.
- Versions: `Cargo.toml` `0.33.0` → `0.34.0`, `cli.spec.json` `0.33.0` → `0.34.0`.

## [0.33.0] - 2026-05-30

### Added — CLOC11.70: `--correlation_vector_filter` allowlist flag

Adds a CSV allowlist of CV `contribution.source` names. When non-empty, the sidecar serializer prunes any CV entry whose `contributions` does not include at least one record whose `source` is in the allowlist.

Example: `--correlation_vector_filter lex,defines` writes only entries that the `lex` or `defines` stages touched.

### Semantics

- Strict match on `contribution.source`. The per-token CV entries created with `Origin{source: "lexer_token", ...}` but with zero contributions are dropped when the filter is `lex` — their "lex" association lives in the Origin, not in a contribution. The per-file CV root (which holds the `lex.tokens_emitted` contribution) is kept. Documented in the config-level rustdoc and pinned by tests.
- Empty allowlist = no pruning (default behavior). The fast path in `format_cv_log_json` short-circuits the round-trip when both `pretty` and `filter` are unset.
- Whitespace around CSV tokens is trimmed; empty tokens are ignored. `"lex, defines"` is the same as `"lex,defines"`.

### Implementation

- New shared helper `prune_entries_by_source(&mut serde_json::Value, &[String])` mutates the parsed CV log in-place. Uses a `HashSet` for O(1) source lookup.
- Both `format_cv_log_json` and `format_cv_log_ndjson` now take the filter slice and call the helper between parse and re-emit.

### Changed

- `SpecialModesConfig` gains `correlation_vector_filter: Vec<String>`.
- `wire::read_special_modes` splits the comma-separated string; trims whitespace; drops empty tokens.
- `format_cv_log_json` signature: now `(cv_log, pretty, filter) -> String`. Private to the crate.
- `format_cv_log_ndjson` signature: now `(cv_log, filter) -> String`. Private to the crate.
- Versions: `Cargo.toml` `0.32.0` → `0.33.0`, `cli.spec.json` `0.32.0` → `0.33.0`.

## [0.32.0] - 2026-05-30

### Added — CLOC11.69: `--correlation_vector_format` enum (JSON | NDJSON | NONE)

Adds a sidecar format selector to cover streaming consumers and benchmark modes.

- `JSON` (default) — single JSON document, same shape as CLOC11.60+. `--correlation_vector_pretty` still applies.
- `NDJSON` — newline-delimited JSON: one CV entry per line, ending with a `{"_meta": {"pass_order":[...], "enabled":...}}` footer line. Tooling can `tail -f` mid-build without waiting for a closing brace. The `pretty` flag is ignored under NDJSON (line-delimited JSON is inherently single-line per record).
- `NONE` — compute the CV log but **do not** write the sidecar. Lets benchmarks measure CV compute overhead in isolation from write/serialize overhead.

The flag is ignored when `--correlation_vector` is off.

### Implementation notes

- `format_cv_log_ndjson` round-trips through `serde_json::Value` (same approach as the pretty path) to walk the `entries` map without touching CV crate internals. Fallback chain: any parse/serialize hiccup yields the compact single-doc JSON instead of an empty file.
- The `None` arm is a single-statement no-op gated by the existing `if config.special_modes.correlation_vector` block, so default behavior is unchanged when the flag is absent.

### Changed

- `SpecialModesConfig` gains `correlation_vector_format: CorrelationVectorFormat` (new enum).
- `wire::read_special_modes` reads the enum from the parse result; unknown / empty values fall back to `Json`.
- Versions: `Cargo.toml` `0.31.0` → `0.32.0`, `cli.spec.json` `0.31.0` → `0.32.0`.

## [0.31.0] - 2026-05-30

### Added — CLOC11.68: `--correlation_vector_pretty` flag

Adds a toggle between compact and pretty-printed CV sidecar JSON. Default is compact (single-line, what CI / build pipelines want); `--correlation_vector_pretty` switches to multi-line, 2-space-indented output for human inspection.

Resolution:
- `--correlation_vector_pretty` (default `false`) → compact JSON via `CVLog::to_json_string`.
- `--correlation_vector_pretty true` → round-trip via `serde_json::Value` and `to_string_pretty` for the multi-line form.

The flag is only consulted when `--correlation_vector` is also enabled. With CV off, the formatter never runs.

### Why round-trip rather than a new upstream method

`CVLog::to_json_string` is the only path that knows the `LogSnapshot` shape (the fields aren't `pub`). Parsing back to a `serde_json::Value` and re-emitting via `to_string_pretty` is wasteful but correct, and only happens on the opt-in slow path. The performance hit is irrelevant — humans-eyes mode is already off the critical path of a build.

### Changed

- `SpecialModesConfig` gains `correlation_vector_pretty: bool`.
- `wire::read_special_modes` now reads `correlation_vector_pretty` from the parse result.
- `format_cv_log_json` signature changed from `(&CVLog) -> String` to `(&CVLog, pretty: bool) -> String`. Private to the crate; no external API impact.
- Versions: `Cargo.toml` `0.30.0` → `0.31.0`, `cli.spec.json` `0.30.0` → `0.31.0`.

## [0.30.0] - 2026-05-30

### Added — CLOC11.67: `--correlation_vector_output <path>` flag

Adds an explicit path override for the correlation-vector sidecar JSON. Lets CI pipelines route the CV trace to an artifact directory (or `/dev/null` for benchmarks) without relying on the sidecar-of-output convention.

Resolution order (highest precedence first):

1. `--correlation_vector_output <path>` → that path, verbatim, no decoration.
2. Else if `--js_output_file` is set → `<output>.cv.json` beside it.
3. Else (stdout output) → `closurec-cv.json` in the working directory.

The flag is only consulted when `--correlation_vector` is also enabled — the trace itself is still opt-in. With CV off, the path flag is ignored.

### Changed

- `SpecialModesConfig` gains `correlation_vector_output: Option<PathBuf>`.
- `wire::read_special_modes` now reads `correlation_vector_output` from the parse result; empty string maps to `None`.
- Versions: `Cargo.toml` `0.29.0` → `0.30.0`, `cli.spec.json` `0.29.0` → `0.30.0`.

## [0.29.0] - 2026-05-30

### Added — CLOC11.66: WHITESPACE_ONLY token tombstones

When `--correlation_vector` is on and `--compilation_level WHITESPACE_ONLY` is set, every token CV that the minifier drops (trivia + EOF) now gets a `DeletionRecord` (tombstone) via `CVLog::delete`. The CV trace shows precisely which input bytes the WHITESPACE_ONLY pass killed.

Tombstone shape (one per dropped token):

```
source: "compilation_level"
reason: "whitespace_only_dropped"
meta: {
  kind:                  "trivia" | "eof",
  token_index:           <0-based position in lexer stream>,
  token_lexeme_byte_len: <token.value.len()>,
}
```

Implementation reuses `whitespace_only::is_trivia` / `is_eof` (now `pub(crate)`) — the same predicate the minifier itself uses — so the tombstone set is guaranteed identical to the dropped set without a second lex pass.

Other compilation levels (SIMPLE, ADVANCED, BUNDLE, TRANSPILE_ONLY) are currently identity on the string and don't drop tokens, so no tombstones land for those. As those levels grow real bodies in later CLOC11.* slices, each will need its own tombstone block.

### Changed

- `whitespace_only::is_trivia` and `whitespace_only::is_eof` promoted from private `fn` to `pub(crate) fn` so the per-token CV path can call them directly without duplicating the predicate.
- Versions: `Cargo.toml` `0.28.0` → `0.29.0`, `cli.spec.json` `0.28.0` → `0.29.0`.

## [0.28.0] - 2026-05-30

### Added — CLOC11.65: per-token `defines.applied` contributions

Uses the per-token CV substrate from CLOC11.64. When `--correlation_vector` is on and `--define K[=V]` flags are present, every `Name` token in the input whose lexeme matches a define key gets a `defines.applied` contribution recorded **on its token CV**, not on the per-file root.

Per-token `defines.applied` contribution shape:

```
source: "defines"
tag:    "applied"
meta: {
  define_name:        <token.value>,
  define_value:       <Bool | Number | String | Null>,
  define_value_kind:  "bool" | "number" | "string" | "null",
  token_index:        <0-based position in the lexer stream>,
}
```

The per-file `defines.applied` summary (defines_count, byte deltas) still fires from `transform_source_with_cv` — the per-token records are *in addition*, so visualization tools get both "the stage ran" (file-level) and "this specific token was hit" (token-level).

Implementation: the token loop now keeps a `Vec<String>` of derived token CV IDs in lock-step with the token vector, so post-loop lookups are O(1). The defines check skips non-Name tokens — strings, numbers, regex literals — matching the existing string-level `apply_defines` behaviour.

Caveats (unchanged from the string-level pass):
- Defines inside string literals are not substituted (correct — the Name filter excludes string tokens).
- Object shorthand (`{ FOO }`) would change semantics if substituted; same caveat as `apply_defines`.

### Changed

- Versions: `Cargo.toml` `0.27.0` → `0.28.0`, `cli.spec.json` `0.27.0` → `0.28.0`.

## [0.27.0] - 2026-05-29

### Added — CLOC11.64: per-token CV entries (children of per-file CV)

Continues the "every feature CV-traceable when enabled" series. When `--correlation_vector` is on, after reading each input file we now tokenize with `coding-adventures-javascript-lexer::tokenize_javascript_typed` and derive a **child CV entry per token** under the per-file CV root.

This is the substrate for the next slices (CLOC11.65+) to migrate token-level contributions (`defines.applied`, `whitespace_only` drops, rename mappings) off the per-file summary entry and onto the precise token CV they touched — so the trace tells you *which token* a transform mutated, not just *which file*.

Per-token CV entry shape:

| Field         | Value                                            |
|---------------|--------------------------------------------------|
| parent_ids    | `[per_file_cv_id]` (via `CVLog::derive`)         |
| `source`      | `"lexer_token"`                                  |
| `location`    | `"<path>:<line>:<column>"` (1-based, lexer-native) |
| `meta.kind`   | lowercased `TokenType` debug name                |
| `meta.lexeme_byte_len` | `value.len()` (post escape resolution)  |
| `meta.token_index` | 0-based position in the token stream        |

Per-file CV gains one summary contribution after the token loop:

```
source: "lex", tag: "tokens_emitted", meta: {token_count: N}
```

Error policy: a lex failure does **not** abort the build. The string-only pipeline still runs (WHITESPACE_ONLY can copy verbatim, defines can no-op). We record `lex.failed` with the lexer error message on the per-file CV and skip per-token creation.

Cost: only paid when `--correlation_vector` is on. Default-off path is byte-identical to 0.26.0.

### Changed

- Versions: `Cargo.toml` `0.26.0` → `0.27.0`, `cli.spec.json` `0.26.0` → `0.27.0`.

## [0.26.0] - 2026-05-27

### Added — CLOC11.63: CV records for output writes (JS, source map, manifest)

Extends CLOC11.62 to record the three output-file writes as derived CV entries. Every byte that hits disk now has a CV ID, and the trace forms a proper DAG from per-file sources through combined-output to disk artifacts.

Three new derived CV entities:

| Entity                 | Created via   | Parent(s)            | Records                              |
|------------------------|---------------|----------------------|--------------------------------------|
| `js_output_file`       | `derive()`    | `combined_cv_id`     | `write_output_file.wrote` + byte_len |
| `source_map_output`    | `derive()`    | `combined_cv_id`     | `write_output_file.wrote` + byte_len |
| `manifest_output`      | `merge()`     | `per_file_cv_ids[]`  | `write_output_file.wrote` + byte_len |

**Why manifest uses `merge()` with per-file parents:** the manifest enumerates input files, not the merged output. Conceptually it's an index of the per-file CVs, not a derivative of the merged JS. A consumer following provenance from a manifest entry walks straight back to the per-file CV roots.

**Why JS / source_map use `derive()` with `combined_cv_id`:** they derive their bytes from the combined post-transform substrate.

Gates: each record only contributes when the corresponding flag is set (`--js_output_file`, `--create_source_map`, `--output_manifest`).

### Coverage milestone

After CLOC11.63, the CV trace covers every step:

```
input → per-file CV → combined CV → js_output_file CV → disk
                                  → source_map_output CV → disk
                                  → manifest_output CV (merge of per-file) → disk
```

The user's policy ("every feature CV-traceable when enabled") is structurally complete for the pipeline that exists today. CLOC11.64–66 add granularity (per-token, tombstones) and convenience (`--correlation_vector_output`), not coverage.

### Implementation

- Captured `encoded_byte_len` before the JS-write match block to avoid borrow-of-moved when the None arm consumes `encoded`.
- Output writes now followed by `cv_log.derive(...)` or `cv_log.merge(...)` when CV is on.
- 4 new unit tests in `run::tests`.

## [0.25.0] - 2026-05-27

### Added — CLOC11.62: CV records for post-combine stages

Extends CLOC11.61's per-stage instrumentation to the four post-concatenation pipeline stages: `emit_use_strict`, `output_wrapper`, `isolation_mode` (IIFE), and `charset`. After the per-file loop, the CV log derives a new "combined" entry whose parents are every per-file CV ID — so a downstream output byte's provenance walks `combined → all source files` automatically.

The combined entry is the substrate every post-concat contribution lands on:

| Stage             | `source`           | `tag`           | `meta`                                                        |
|-------------------|--------------------|-----------------|---------------------------------------------------------------|
| emit_use_strict   | `emit_use_strict`  | `prepended`     | `{input_byte_len, output_byte_len}` (only when flag set)      |
| output_wrapper    | `output_wrapper`   | `substituted`   | `{input_byte_len, output_byte_len}` (only when wrapper changed bytes) |
| isolation_mode    | `isolation_mode`   | `iife_wrapped`  | `{input_byte_len, output_byte_len}` (only when IIFE set)      |
| charset           | `charset`          | `normalized`    | `{mode: "US_ASCII"\|"UTF-8", input_byte_len, output_byte_len}` (always) |

Contribution-or-not policy: the `charset` stage always contributes (it always runs); the other three skip the contribution when they're pass-throughs (no flag set / no bytes changed). This keeps the trace focused on actual byte movement while still recording the structural step.

### New CV entity: `concatenated_combined_source`

After the per-file loop, when CV is on, `run_compiler` calls `CVLog::merge(per_file_ids, Some(combined_origin))` to create a new entry whose `parent_ids` are every per-file root. Meta carries `file_count` and `byte_len`. Origin: `source = "concatenated_combined_source"`, no location (it's not a file on disk). All four post-combine contributions attach here.

### Implementation

- **`per_file_cv_ids: Vec<String>`** accumulated through the per-file loop.
- **`combined_cv_id`** computed after the loop via `cv_log.merge(...)`.
- **Each post-combine stage** wrapped with `if let Some(id) = &combined_cv_id { ... cv_log.contribute(id, ...) }`.
- **`isolation_mode = None` branch** had to switch from move-of-`wrapped` to `.clone()` so we can record the input byte length in the CV branch above the move.
- **6 new unit tests** in `run::tests` (combined entry exists with parents, emit_use_strict on, emit_use_strict off → no contribution, output_wrapper changing bytes, IIFE on, charset always with mode).
- **Existing CLOC11.61 `pass_order` test** updated to assert prefix `[compilation_level, defines, ...]` rather than exact `[compilation_level, defines]`, since CLOC11.62 + later slices grow the pass_order.

### Pipeline matrix (unchanged structurally)

Same 15 steps; CLOC11.62 adds per-stage CV records inside steps 8–11 (and a new derived "combined" CV entity between steps 6 and 7).

### Still queued

- CLOC11.63: source-map / manifest writes recorded as derived CV entries.
- CLOC11.64–66: per-token granularity, tombstones for removals, custom `--correlation_vector_output` path flag.

## [0.24.0] - 2026-05-27

### Changed — CLOC11.61: per-stage `--correlation_vector` contributions

Builds on CLOC11.60's plumbing. Replaces the single `transform_source.applied` summary contribution with one record per pipeline stage so the CV trace shows which pass touched the bytes and how much they grew/shrank.

Per-file CV entry now gains:

| Stage              | `source`            | `tag`             | `meta`                                                  |
|--------------------|---------------------|-------------------|---------------------------------------------------------|
| WhitespaceOnly     | `compilation_level` | `whitespace_only` | `{input_byte_len, output_byte_len}`                     |
| Simple             | `compilation_level` | `identity`        | `{level: "SIMPLE"}`                                     |
| Advanced           | `compilation_level` | `identity`        | `{level: "ADVANCED"}`                                   |
| Bundle             | `compilation_level` | `identity`        | `{level: "BUNDLE"}`                                     |
| TranspileOnly      | `compilation_level` | `identity`        | `{level: "TRANSPILE_ONLY"}`                             |
| Defines            | `defines`           | `applied`         | `{input_byte_len, output_byte_len, defines_count}`      |

The `defines.applied` contribution lands even when `--define` is empty (`defines_count: 0`) — the stage *ran*, it just had nothing to substitute. Keeps the trace symmetric across files; visualization tools don't have to special-case zero-defines runs.

### Implementation

- **New `transform_source_with_cv(source, config, cv) -> Result<String>`** with `cv: Option<(&mut CVLog, &str id)>`. When `cv` is `None`, byte-identical behavior to `transform_source`.
- **`transform_source` is now a thin facade** delegating to `transform_source_with_cv(..., None)`.
- **`run_compiler`'s per-file loop** calls `transform_source_with_cv` when CV is on, passing the per-file `cv_id`. The CLOC11.60 post-call summary contribution is removed (superseded by the per-stage records).
- **5 new unit tests** in `run::tests`:
  - WHITESPACE_ONLY → `compilation_level.whitespace_only` contribution lands
  - SIMPLE default → `compilation_level.identity` with `level: "SIMPLE"` lands
  - `--define` entries → `defines.applied` with `defines_count`
  - Both stages present + `pass_order: [compilation_level, defines]`
  - `transform_source` facade ≡ `transform_source_with_cv(_, _, None)`
- **CLOC11.60 multi-file test updated** to count 2 × `compilation_level` + 2 × `defines` contributions instead of 2 × the old `transform_source` summary.

### Pipeline matrix (unchanged structurally)

Same 15 steps as 0.23.0; the change is in step 5's instrumentation, not in pipeline order.

### Still queued

- CLOC11.62: CV records for wrapper / IIFE / charset stages.
- CLOC11.63: source-map / manifest writes recorded as derived CV entries.
- CLOC11.64–66: per-token granularity, tombstones for removals, custom `--correlation_vector_output` path.

## [0.23.0] - 2026-05-27

### Added — CLOC11.60: opt-in `--correlation_vector` plumbing through pipeline

**Architectural milestone.** First slice of the correlation-vector traceability work specified in `feedback_closurec_correlation_vectors.md`. When `--correlation_vector` is set, the pipeline threads a [`coding_adventures_correlation_vector::CVLog`] through every input file and records per-file contributions for the transform stage. When the flag is unset (default), the `CVLog` is constructed in disabled mode — every `create`/`contribute` call is a no-op, so the existing zero-overhead pipeline behavior is preserved.

The CV trace is written as a JSON sidecar file at the end of the run. Path policy:

- When `--js_output_file` is set, the sidecar lives next to it as `<output>.cv.json`. Build pipelines consuming the compiled JS automatically pick up the trace without a separate flag.
- When `--js_output_file` is absent (stdout output), the sidecar lands at `closurec-cv.json` in the current working directory.

### What this slice covers (intentionally narrow)

- **Per-file root CV entry**: assigns a CV ID at file ingestion with `Origin::source = "input_file"` and `location = <path>`.
- **One summary contribution per file**: tags the entry with `source = "transform_source"`, `tag = "applied"`, and includes input + output byte lengths in `meta`. This is the placeholder for the deeper per-stage instrumentation queued in CLOC11.61..11.66.
- **JSON sidecar emission** via `CVLog::to_json_string()`.

### What's still queued

- CLOC11.61: split the per-file summary contribution into one-per-stage (`whitespace_only`, `defines`).
- CLOC11.62: wrapper / IIFE / charset stages.
- CLOC11.63: source-map / manifest writes recorded as derived CV entries.
- CLOC11.64–66: per-token granularity, tombstones for removals, custom `--correlation_vector_output` path flag.

### Implementation

- **`SpecialModesConfig.correlation_vector: bool`** field + cli.spec.json entry + wire.rs parsing.
- **`run_compiler` instantiates `CVLog::new(config.special_modes.correlation_vector)`** once before the per-input loop. The boolean toggle threads down into every CV call; disabled-mode short-circuits at the crate level.
- **`format_cv_log_json(&CVLog) -> String`** wraps `CVLog::to_json_string()` with a `{}` fallback so a serialization error doesn't break the otherwise-successful run.
- **Step 7 (NEW) in `run_compiler`** — writes the sidecar after the manifest write so `wrote_files` ends up in pipeline order: JS, source map, manifest, CV sidecar.
- **4 new unit tests** in `run::tests` (default → no sidecar, opt-in → sidecar next to output, opt-in stdout → default sidecar in CWD, multi-file → entry-per-file).

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source` (now records one CV contribution per file when `--correlation_vector` is set)
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution
10. `--isolation_mode IIFE` wrap
11. `--charset` US_ASCII escape
12. Write JS
13. Write source map
14. Write input manifest
15. **Write CV sidecar (CLOC11.60, NEW) if `--correlation_vector` was set**

## [0.22.0] - 2026-05-26

### Added — CLOC11.04: `--define` numeric edge case test coverage

Test-coverage slice pinning behavior of `--define VALUE` for the full set of numeric literals CC's `Double.parseDouble` accepts. Rust's `f64::parse` covers all of these already — these tests make the contract explicit so a future refactor (e.g. switching to a hand-rolled number parser, or tightening to integer-only) can't quietly regress CC compat.

Forms covered:

| Form          | Accepted by closurec | Example       |
|---------------|----------------------|---------------|
| Integer       | ✓                    | `42`          |
| Negative int  | ✓ (NEW PIN)          | `-42`         |
| Float         | ✓                    | `1.5`         |
| Negative float| ✓ (NEW PIN)          | `-1.5`        |
| Fractional-only | ✓ (NEW PIN)        | `.5`          |
| Scientific    | ✓ (NEW PIN)          | `1e3`         |
| Negative exp  | ✓ (NEW PIN)          | `1e-6`        |
| Leading `+`   | ✓ (NEW PIN)          | `+1`          |
| Zero          | ✓ (NEW PIN)          | `0`           |
| Negative zero | ✓ (NEW PIN)          | `-0`          |
| `NaN`         | ✗ (NEW PIN)          | rejected      |
| `Infinity`    | ✗ (NEW PIN)          | rejected      |
| Hex `0xFF`    | ✗ (NEW PIN)          | rejected      |

NaN/Infinity rejection is deliberate — they parse as `f64` in Rust but aren't valid JS *literals* (you'd write `0/0` or `1/0` to get them at runtime). Hex literals would be valid JS but CC's `Double.parseDouble` rejects them; we match.

- **10 new unit tests** in `wire::tests`, no behavior change.

### Pipeline matrix (unchanged)

Same 14-step pipeline as 0.21.0. This is a test-only release that pins the contract on existing config-build behavior.

## [0.21.0] - 2026-05-26

### Added — CLOC11.34: `--output_manifest` writes input file list

Behavioral compat slice with CC's `--output_manifest=path` flag. Previously closurec parsed the flag and stored the path but never wrote any file — build systems (Bazel `rules_closure`, ninja-driven builds) that read the manifest to verify input set saw nothing.

Now writes a newline-separated list of every input the compilation consumed (post-glob expansion), one path per line, with a trailing newline so `wc -l` and concatenation behave.

- **Pipeline placement**: Step 6 in `run_compiler`, after JS write + source-map write. So `wrote_files` in `CompilerOutput` lists outputs in pipeline order: JS, source map (if `--create_source_map`), manifest (if `--output_manifest`).
- **Empty inputs case** (banner mode): writes an empty manifest file (0 bytes) — still useful as a "compilation ran" marker, matches CC.
- **Paths in the manifest are the resolved form** (after glob expansion), not the raw user patterns. This lets the user see exactly which files the compilation consumed.
- **New `format_manifest(&[PathBuf]) -> String`** private helper. Pure function: no I/O.
- **5 new unit tests** in `run::tests` (empty-inputs format, multi-line format with newline count, end-to-end write with resolved path verification, no-write when flag unset, trifecta with JS + source map + manifest in pipeline order).

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source`
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution
10. `--isolation_mode IIFE` wrap
11. `--charset` US_ASCII escape
12. Write JS to `--js_output_file` or stdout
13. Write source map to `--create_source_map` path if set
14. **Write input list to `--output_manifest` path if set (CLOC11.34, NEW)**

## [0.20.0] - 2026-05-26

### Added — CLOC11.42: `--create_source_map` writes minimal v3 source map

Behavioral compat slice with CC's `--create_source_map=path` flag. Previously closurec parsed the flag and stored the path but never wrote any file — build scripts expecting a source map at the path saw nothing. Now writes a minimal valid v3 source map JSON at the path.

Wire format:

```json
{
  "version": 3,
  "file": "<--js_output_file basename or empty>",
  "lineCount": 0,
  "sourceRoot": "",
  "sources": [],
  "sourcesContent": [],
  "names": [],
  "mappings": ""
}
```

The mappings are intentionally empty — real position tracking lands with the parser-bridge in CLOC11.07+. The goal of this slice is that build pipelines (Bazel rules, webpack `source-map-loader` shims, etc.) expecting a file at the path see one with the right shape. Debuggers that try to use the map for position lookup get the correct response of "no information available" rather than a broken document.

- **New `source_map` module** with `format_minimal_v3(Option<&Path>) -> String`. Pure function: no I/O, fully deterministic.
- **Pipeline placement**: Step 5 in `run_compiler`, after the JS output write. Source map write runs *after* the JS write so callers get a consistent on-disk pair (or no source map at all if the flag is unset).
- **Source-map writing works even when `--js_output_file` is absent** (compiled JS goes to stdout). In that case the map's `file` field is empty.
- **`file` field is the basename** of the compiled-output path, not the full path — keeps the map portable across CDN paths.
- **9 new unit tests** in `source_map::tests` (empty-path → empty file key, basename extraction, version-3 marker, eight required keys present, empty arrays well-formed, empty mappings string, trailing newline, JSON escaping of weird file names, byte-stable output).
- **4 new unit tests** in `run::tests` (writes file when path set, no-write when path empty, stdout+map combination, basename-only `file` field).

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source`
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution
10. `--isolation_mode IIFE` wrap
11. `--charset` US_ASCII escape
12. Write JS to `--js_output_file` or stdout
13. **Write source map to `--create_source_map` path if set (CLOC11.42, NEW)**

## [0.19.0] - 2026-05-26

### Changed — CLOC11.16: `--charset` US_ASCII output escaping (BEHAVIOR DEFAULT CHANGE)

Behavioral compat slice with CC's documented `--charset` default. Previously closurec accepted `--charset` and stored it but never escaped non-ASCII characters in the output — every non-ASCII codepoint passed through verbatim regardless of flag value. That diverged from CC's documented default of "UTF-8 in, US_ASCII out".

Now matches CC:

| `--charset` value | Output behavior                              |
|-------------------|----------------------------------------------|
| (unset)           | **US_ASCII — escape non-ASCII as `\uXXXX`** (matches CC default) |
| `US_ASCII`        | same as unset                                |
| `US-ASCII`        | accepted alias                               |
| `UTF-8` / `UTF8`  | pass-through (raw UTF-8 bytes)               |
| anything else     | pass-through (CC ignores unknown values)     |

**This is a default-behavior change**: existing users who relied on raw-UTF-8 output and didn't pass `--charset` will now see `\uXXXX` escapes. To restore prior behavior, pass `--charset UTF-8` explicitly. CC users get this default already, so closurec invocations that worked against CC will continue to work against closurec.

Escape format: BMP codepoints (`U+0000..U+FFFF`) emit `\uXXXX`. Astral codepoints (`U+10000..U+10FFFF`) emit a UTF-16 surrogate pair (`\uXXXX\uXXXX`) — not the ES2015 `\u{XXXXX}` form — for maximum compatibility with legacy minifiers / ES5-only environments.

- **New `charset` module** with `OutputCharset::from_raw(&str)` + `apply_charset(&str, OutputCharset) -> String`. Pure-function, no I/O, fully deterministic.
- **Pipeline placement**: Step 3.75 in `run_compiler`, between IIFE wrap and write. Runs *after* the output wrapper so any non-ASCII the user injected via `--output_wrapper` (e.g. a `©` banner) gets escaped too.
- **12 new unit tests** in `charset::tests` (default → US_ASCII, value parsing for all aliases + case-insensitive, unknown → UTF-8 fallback, UTF-8 pass-through, US_ASCII pass-through for pure-ASCII text, BMP escape, CJK escape, surrogate pair, lowercase hex, byte-identical ASCII).
- **Diff fixtures** `tests/diff/charset-us-ascii/` (default) and `tests/diff/charset-utf8/` (opt-out).
- **New integration test** `tests/diff_charset.rs` pinning both ends of the toggle, including `is_ascii()` invariant under default.
- **`tests/diff/js-glob/expected.stdout` regenerated**: em-dashes in test input comments now appear as `—`. This is the new default; the test continues to exercise glob expansion logic.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. Resolve `--externs` globs
3. `--print_tree` short-circuit
4. `--print_tree_json` short-circuit
5. Per-input `transform_source`
6. Concatenate transformed inputs
7. `--checks_only` short-circuit
8. `--emit_use_strict` prepend
9. `--output_wrapper` substitution (validated)
10. `--isolation_mode IIFE` wrap
11. **`--charset` US_ASCII escape (CLOC11.16, NEW)**
12. Write to `--js_output_file` or stdout

## [0.18.0] - 2026-05-26

### Changed — CLOC11.55: `--version` emits CC-style banner

Drop-in compat surface fix. Previously `--version` printed just `0.18.0\n` — a bare semver with no marker that tools could grep for to identify this binary as a Closure Compiler drop-in. Now matches the shape of CC's `closure-compiler.jar --version`:

```
Closure Compiler (closurec — drop-in replacement, https://github.com/adhithyan15/coding-adventures)
Version: 0.18.0
```

Why this shape:
- First line starts with `Closure Compiler ` — toolchains that grep CC's stdout for that marker (e.g. Bazel rules that pin a compiler identity) keep working.
- Second line is `Version: <semver>` — standard hook for version-extracting scripts.
- Project URL points at this clone rather than upstream so users know what they're running.
- No `Built on:` line (CC has one) — we don't embed build timestamps. The two `grep`-worthy lines are what tools actually depend on.

- **Updated `ParserOutput::Version` arm** in `main::parse_and_run`. cli-builder still surfaces `--version` as `ParserOutput::Version(v)`; we just format `v.version` differently.
- **4 new unit tests** in `main::tests` (starts-with-marker, Version-colon line, embedded semver still present, trailing-newline cleanliness).
- **Diff fixture** `tests/diff/version-banner/` with `flags.txt` driving the integration test.
- **New integration test** `tests/diff_version_banner.rs` pinning the structural invariants. We don't pin byte-for-byte because the embedded semver changes every release.

### Pipeline matrix (unchanged)

Same 11-step pipeline; change is in `main.rs`'s top-level dispatch, not `run_compiler`.

## [0.17.0] - 2026-05-26

### Changed — CLOC11.41: `--source_map_location_mapping` malformed values now error

Sibling fix to CLOC11.40. The same silent-drop `filter_map` bug existed in `read_source_map`'s `source_map_location_mapping` parser. Pre-CLOC11.41, a typo'd `--source_map_location_mapping src/` (no `|`) silently vanished, leaving the user wondering why their map URLs didn't rewrite.

Now the parser errors out with a typed `ConfigError::InvalidSourceMapLocationMapping { raw }`:

```
--source_map_location_mapping <raw>: missing required `|` separator (expected `filesystem-path|web-server-path`)
```

Argv-order processing — first bad entry surfaces, matching the CLOC11.40 policy.

Edge cases preserved (match CC):
- `|web/` and `fs/|` remain well-formed (only pipe presence is checked).

- **New `ConfigError::InvalidSourceMapLocationMapping { raw }` variant** + Display arm.
- **`filter_map` replaced** with an explicit `for` loop that propagates the typed error.
- **4 new unit tests** in `wire::tests` (missing-pipe errors, error message format, multi-entry first-bad-wins, empty halves still well-formed).
- **Diff fixture** `tests/diff/source-map-location-mapping-bad/`.
- **New integration test** `tests/diff_source_map_location_mapping_bad.rs`.

### Pipeline matrix (unchanged)

Same 11-step pipeline; change is config-build validation only.

## [0.16.0] - 2026-05-25

### Changed — CLOC11.40: `--source_map_input` malformed values now error

Behavioral compat slice with CC's source-map-input handling. Previously closurec parsed `--source_map_input` entries via `filter_map(|s| s.split_once('|'))`, which silently dropped malformed values that lacked the required `|` separator. Effect on users: typo'd separator → entry quietly vanishes → user wonders why their source map chain didn't apply.

Now the parser errors out with a typed `ConfigError::InvalidSourceMapInput { raw }`. The error message names both the flag and the offending value:

```
--source_map_input <raw>: missing required `|` separator (expected `input-file-path|input-source-map`)
```

Processing order: argv-order, first bad entry surfaces. So a user fixes typos one at a time rather than playing whack-a-mole after each retry.

Edge cases preserved:
- `|map.map` and `input.js|` are still well-formed (only the *presence* of the pipe is checked; empty halves are accepted, matching CC). When the source-map chain step lands later, the FS resolver will catch missing files separately.

- **New `ConfigError::InvalidSourceMapInput { raw }` variant** + Display arm.
- **`filter_map` replaced** with an explicit `for` loop that propagates the typed error.
- **5 new unit tests** in `wire::tests` (happy path two paths, missing pipe errors, error message format, multi-entry first-bad-wins, empty-halves still well-formed).
- **Diff fixture** `tests/diff/source-map-input-bad/` exercising the error path.
- **New integration test** `tests/diff_source_map_input_bad.rs` pinning that both the flag and the offending value appear in the error.

### Pipeline matrix (unchanged)

Same 11-step pipeline as 0.15.0; the change is config-build validation (`wire.rs::read_source_map`), not pipeline behavior.

## [0.15.0] - 2026-05-25

### Changed — CLOC11.05: `--externs` is now glob-resolved + validates missing files

Behavioral compat slice with CC's externs file handling. Previously closurec accepted `--externs <path>` as a literal `PathBuf` and never touched the filesystem to verify the path existed — a typo would silently drop the externs definitions and only manifest later (or never, at the current pipeline stage where externs aren't yet consumed).

Now `--externs` goes through the same glob expansion as `--js`:

- Patterns like `externs/*.js` are expanded against the filesystem.
- Exclusion patterns (`!path/to/skip.js`) are respected.
- A pattern that matches zero files errors out with `JSC_NO_JS_FILES_FOUND_FOR_PATTERN`-style behavior.
- The error is prefixed `--externs: ...` so the user sees which flag's glob was bad without re-reading the command line.

The resolved externs list is discarded today — the goal of this slice is to catch typos at config-validation time. When the typechecker bridge lands (CLOC11.07+), the resolved list will flow into the typecheck stage.

### Internal: `IoConfig.externs` shape

Refactored from `Vec<PathBuf>` to `Vec<String>` (raw pattern strings). Resolution happens in `run.rs::resolve_externs` rather than `wire.rs`, keeping `wire.rs` pure (no FS I/O during config build). Matches the `js_patterns`/`resolve_inputs` shape.

- **New `CompilerError::ExternsGlobExpansion(GlobError)` variant** + Display arm with `--externs: ` prefix.
- **New `resolve_externs(&CompilerConfig) -> Result<Vec<PathBuf>, CompilerError>`** helper. Empty-externs fast path (most invocations don't pass it) bypasses the glob machinery.
- **Pipeline insertion**: Step 1.25 in `run_compiler`, right after `resolve_inputs`. So both `--js` and `--externs` patterns are validated before any transform pass runs.
- **5 new unit tests** in `run::tests` (empty → empty, real-files → expanded, missing → typed error, end-to-end flag-prefix Display, happy path with both `--js` + `--externs`).
- **Diff fixture** `tests/diff/externs-missing/` exercising the missing-pattern error path end-to-end.
- **New integration test** `tests/diff_externs_missing.rs` pinning the `--externs:` flag prefix + missing-path in the error.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` globs
2. **Resolve `--externs` globs (CLOC11.05, NEW) — validate-only today, flows into typecheck post-CLOC11.07**
3. `--print_tree` short-circuit (CLOC11.52)
4. `--print_tree_json` short-circuit (CLOC11.53)
5. Per-input `transform_source` (level + defines)
6. Concatenate transformed inputs
7. `--checks_only` short-circuit (CLOC11.51)
8. `--emit_use_strict` prepend (CLOC11.18)
9. `--output_wrapper` substitution (CLOC11.30, validated CLOC11.32)
10. `--isolation_mode IIFE` wrap (CLOC11.31)
11. Write to `--js_output_file` or stdout

## [0.14.0] - 2026-05-25

### Changed — CLOC11.32: `--output_wrapper` missing `%output%` is now a typed error

Behavioral compat slice with CC's `AbstractCommandLineRunner.checkFlags`. When `--output_wrapper` (or `--output_wrapper_file`) is set but the resolved template contains no `%output%` placeholder, closurec now errors out with the **exact** CC message:

```
ERROR - No %output% placeholder in the output wrapper
```

Previously closurec accepted any template silently — which meant a typo'd wrapper (e.g. `(function(){%otput%})()`) produced output that didn't contain the compiled JS at all, leaving the user to chase a confusing empty-bundle bug.

- **New `WrapperError::MissingOutputPlaceholder` variant** + Display arm pinned to CC's wording so toolchains that grep stderr for the message keep working when they swap `closure-compiler.jar` for `closurec`.
- **Validation runs in `apply_output_wrapper` after template resolution** — i.e. after `--output_wrapper_file` content is read. So a bad wrapper coming from a file produces the same typed error as a bad inline `--output_wrapper`.
- **Empty wrapper is still pass-through.** An empty/absent wrapper means "no wrapping requested," not "user supplied an invalid wrapper" — the fast-path early return for empty templates runs *before* the validation.
- **7 new unit tests** in `wrapper::tests` (inline-missing errors, exact CC message wording, empty wrapper pass-through still works, file-missing-placeholder also errors, happy path still works with placeholder, `%n%` still expands alongside `%output%`, `std::error::Error` impl pinned).
- **1 unit test updated** (`wrapper_without_output_placeholder_drops_compiled_js` → `wrapper_without_output_placeholder_errors_per_cc`).
- **Diff fixture** `tests/diff/output-wrapper-error/` exercising the error path end-to-end.
- **New integration test** `tests/diff_output_wrapper_error.rs` pinning the CC-compat message + non-zero exit.

### Pipeline matrix (unchanged structurally)

Same 10-step pipeline as 0.13.0; the change is that step 7 (`--output_wrapper` substitution) now rejects placeholder-less templates instead of silently passing them through.

## [0.13.0] - 2026-05-25

### Added — CLOC11.54: `--help_markdown` markdown flag dump

Fourth slice of Track 11 (special modes). When `--help_markdown` is set, closurec prints a markdown document listing every flag in the CLI spec — name, type, default, description — and exits successfully. Mirrors CC's `--help_markdown`, intended for documentation tooling that pipes the output into a docs page.

- **Wire format**: per-flag `### `--long` (type, default: X)` heading + body description. Heading-per-flag (not table) chosen so a diff stays readable when flags are added or descriptions change, and so GitHub's auto-anchors give linkable section IDs.
- **Pipeline placement**: short-circuit in `main::parse_and_run` Step 3.5 — after `cfg` is built from parsed flags, before `run_compiler`. So a config-level user error (e.g. invalid `--define` value) still surfaces, but the markdown dump replaces the rest of the run.
- **No new dependencies**. Uses `cli_builder::types::{CliSpec, FlagDef}` directly (already a transitive dep) and `serde_json::Value` for default-value rendering (already in the dep tree).
- **Spec re-use**: clones the loaded `CliSpec` before passing it to `Parser::new` so the help-markdown branch can iterate the flag list after parsing. The clone is ~10 KB; cheap.
- **7 new unit tests** in `help_markdown::tests` (title, version line, one section per flag, type+default in heading, body carries description, empty-string default disambiguated, no-default omits clause).
- **2 new unit tests** in `main::tests` (flag emits markdown, doesn't run pipeline even with bogus `--js`).
- **Diff fixture** `tests/diff/help-markdown/` with the full pinned markdown output (~400 lines, 100 flags).
- **New integration test** `tests/diff_help_markdown.rs`. Pins the exact output so any change to the user-facing flag surface — a new flag, a renamed flag, a re-described flag — fails the diff and must be acknowledged by regenerating `expected.stdout`.

### Pipeline matrix (cumulative across CLOC11)

`main::parse_and_run`:
1. Load embedded `cli.spec.json`
2. cli-builder parses argv → typed flags
3. `wire::config_from_parsed` → `CompilerConfig`
4. **`--help_markdown` short-circuit (CLOC11.54, NEW) — markdown dump, return**
5. `run::run_compiler(&cfg)`:
   1. Resolve `--js` globs
   2. `--print_tree` short-circuit (CLOC11.52)
   3. `--print_tree_json` short-circuit (CLOC11.53)
   4. Per-input `transform_source` (level + defines)
   5. Concatenate transformed inputs
   6. `--checks_only` short-circuit (CLOC11.51)
   7. `--emit_use_strict` prepend (CLOC11.18)
   8. `--output_wrapper` substitution (CLOC11.30)
   9. `--isolation_mode IIFE` wrap (CLOC11.31)
   10. Write to `--js_output_file` or stdout

## [0.12.0] - 2026-05-25

### Added — CLOC11.53: `--print_tree_json` JSON token-stream dump

Third slice of Track 11 (special modes), companion to `--print_tree` from 0.11.0. When `--print_tree_json` is set, closurec dumps the lexer's token stream as a JSON document to stdout and exits. Same diagnostic intent as CC's `--print_tree_json` — until our parser produces the typed AST (CLOC11.07+ bridge), tokens are the closest analogue.

- **Two wire shapes** depending on input count:
  - **Single file** (typical): a bare JSON array of token objects:
    ```json
    [
      {"type": "KEYWORD", "value": "var"},
      {"type": "NAME", "value": "x"}
    ]
    ```
  - **Multi-file**: an array of file-objects so consumers can disambiguate which tokens came from which file:
    ```json
    [
      {"path": "a.js", "tokens": [{"type": "KEYWORD", "value": "var"}]},
      {"path": "b.js", "tokens": [{"type": "KEYWORD", "value": "let"}]}
    ]
    ```
- **Same trivia + EOF filter** as `--print_tree`. Comments/whitespace/newlines/indent/dedent never appear; significant tokens only.
- **Hand-rolled JSON emission** (no `serde_json` dep) to keep the format byte-stable for diff fixtures. Escapes `"`, `\`, U+0000..U+001F (short forms for `\b \f \n \r \t`); non-ASCII printables pass through as UTF-8.
- **Pipeline placement**: extends Step 1.5's short-circuit alongside `--print_tree`. If both flags are set, `--print_tree` (older, simpler) wins. Glob expansion still runs; the rest of the pipeline (transform, wrap, write) is skipped. `--js_output_file` is ignored.
- **6 new unit tests** in `print_tree::tests` (empty → `[]`, one-object-per-token, trivia drop, quote/backslash escaping, control-char escape, bracket framing).
- **5 new unit tests** in `run::tests` (single-file array, multi-file file-objects, no-write-when-output-file-set, both-flags-set precedence, lex-error surfaces).
- **Diff fixture** `tests/diff/print-tree-json/` with `expected.stdout` pinned for `var x = 1;`.
- **New integration test** `tests/diff_print_tree_json.rs`.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` glob patterns
2. `--print_tree` short-circuit (CLOC11.52)
3. **`--print_tree_json` short-circuit (CLOC11.53, NEW) — JSON token dump, return**
4. Per-input `transform_source` (level + defines)
5. Concatenate transformed inputs
6. `--checks_only` short-circuit (CLOC11.51)
7. `--emit_use_strict` prepend (CLOC11.18)
8. `--output_wrapper` substitution (CLOC11.30)
9. `--isolation_mode IIFE` wrap (CLOC11.31)
10. Write to `--js_output_file` or stdout

## [0.11.0] - 2026-05-25

### Added — CLOC11.52: `--print_tree` token-stream dump

Second slice of Track 11 (special modes). When `--print_tree` is set, closurec dumps the lexer's token stream to stdout and exits without running the rest of the pipeline. Stand-in for the upstream Java Closure Compiler's `--print_tree`, which dumps the parsed AST — until our parser produces the typed AST (CLOC11.07+ bridge), the token stream is the closest analogue diagnostic users actually find useful.

- **Wire format.** Per input file:
  - One banner line `=== <path> ===\n`.
  - One line per significant token: `<TYPE_NAME>\t<value>\n`.
  - Trivia (comments, whitespace, newlines, indent/dedent) and EOF filtered.
  - `TYPE_NAME` is the grammar-supplied `type_name` when present, else the upper-cased `TokenType` debug name (fallback).
- **New module `print_tree`** holds the pure-string formatter `format_token_dump(&str, EsVersion) -> Result<String, PrintTreeError>`. 5 unit tests inline.
- **Pipeline insertion.** Added a "Step 1.5" guard at the top of `run_compiler`, right after `resolve_inputs` returns — before `transform_source`, `--checks_only` short-circuit, wrapping, and write. So:
  - Glob expansion still runs (catches `JSC_NO_JS_FILES_FOUND_FOR_PATTERN`-equivalent errors).
  - The compilation-level transform and the rest of the pipeline are skipped entirely.
  - `--js_output_file` is ignored under `--print_tree` (CC's behavior too — diagnostic dumps go to stdout).
- **New `CompilerError::PrintTree(print_tree::PrintTreeError)`** variant + Display arm so lex failures during the dump surface as typed errors, not panics.
- **Diff fixture** `tests/diff/print-tree/` with input/, flags.txt, and pinned expected.stdout for `var x = 1;`.
- **New integration test** `tests/diff_print_tree.rs`.
- **4 new unit tests** in `run::tests`: basic dump with banner, multi-file banner ordering, no-write-when-output-file-set, lex-error-surfaces.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Resolve `--js` glob patterns
2. **`--print_tree` short-circuit (CLOC11.52, NEW) — token-stream dump, return**
3. Per-input `transform_source` (level + defines)
4. Concatenate transformed inputs
5. `--checks_only` short-circuit (CLOC11.51)
6. `--emit_use_strict` prepend (CLOC11.18)
7. `--output_wrapper` substitution (CLOC11.30)
8. `--isolation_mode IIFE` wrap (CLOC11.31)
9. Write to `--js_output_file` or stdout

## [0.10.0] - 2026-05-25

### Added — CLOC11.51: `--checks_only` mode skips emission

First slice of Track 11 (special modes). When `--checks_only` is set, closurec validates the inputs (runs transform_source over each, so any tokenizer/parser errors still surface) but emits **no** JS — no stdout text, no file write. Matches CC's behavior.

- **Pipeline insertion.** Added a guard at "Step 2.25" in `run_compiler`, right after the per-input transform loop accumulates `combined` but before `--emit_use_strict` prepend, `--output_wrapper` substitution, and `--isolation_mode IIFE` wrap. So:
  - The tokenizer/transform validation still runs (errors propagate normally).
  - The wrapping/write stages never run when `checks_only` is true.
  - Returns `CompilerOutput { stdout_text: "", wrote_files: [] }`.
- **CI-script-friendly semantics.** Exit code 0 on validation success; non-zero on any error from earlier stages (lex/glob/IO). Matches what a CI invocation expects from a `closure-compiler --checks_only` lint step.
- **No `--js_output_file` interaction**: even when set, no file is written. Pinned by `checks_only_does_not_write_output_file`.
- **Diff fixture** `tests/diff/checks-only/` with an empty `expected.stdout`.
- **New integration test** `tests/diff_checks_only.rs`.
- **3 new unit tests** in `run::tests`: empty-output basic, no-write-when-output-file-set, lex-error-still-surfaces.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Per-input `transform_source` (level + defines)
2. Concatenate transformed inputs
3. **`--checks_only` short-circuit (CLOC11.51, NEW) — return empty if set**
4. `--emit_use_strict` prepend (CLOC11.18)
5. `--output_wrapper` substitution (CLOC11.30)
6. `--isolation_mode IIFE` wrap (CLOC11.31)
7. Write to `--js_output_file` or stdout

### Behavior changes (user-visible)

- `closurec --checks_only --js app.js` now actually skips emission. Previously the flag was parsed but ignored (output emitted anyway).
- `closurec --checks_only --js broken.js` still surfaces lex/parse errors (exit 2) — validation runs even though emission is skipped.

### Tests

120 unit + 8 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.9.0 → 0.10.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.9.0] - 2026-05-25

### Added — CLOC11.18: `--emit_use_strict` prelude

First slice of Track 3 (language level). When `--emit_use_strict` is passed, closurec prepends `"use strict";` to the compiled output. Matches CC's behavior.

- **Pipeline ordering.** The directive is prepended to `combined` *before* both `--output_wrapper` template substitution and `--isolation_mode IIFE` wrapping. Reason: a `"use strict"` directive only takes effect when it's the *first* directive of the function body it governs. Both wrapping layers build syntactic envelopes around the body, so the directive has to sit just inside the innermost wrapper — which means we attach it to `combined` and let the outer wrappers wrap *around* it. Matches CC.
- **No `--output_wrapper_file` interaction.** Same ordering — the directive is part of the body that gets substituted into `%output%`.

### Pipeline matrix (cumulative across CLOC11)

`run_compiler`:
1. Per-input `transform_source`:
   - 1a. `--compilation_level` (WHITESPACE_ONLY active)
   - 1b. `--define / -D` substitution (CLOC11.19)
2. Concatenate transformed inputs (`combined`)
3. **`--emit_use_strict` prepend (CLOC11.18, new)**
4. `--output_wrapper` template substitution (CLOC11.30)
5. `--isolation_mode IIFE` wrap (CLOC11.31)
6. Write to `--js_output_file` (auto-create parents) or stdout

### Tests

4 new unit tests in `run::tests`:
- `emit_use_strict_prepends_directive` — basic prelude at top of output.
- `emit_use_strict_default_does_not_prepend` — flag off → no directive.
- `emit_use_strict_lands_inside_iife` — pipeline order pinned: directive sits between the IIFE opener and the body.
- `emit_use_strict_lands_inside_output_wrapper` — directive sits inside the `%output%` slot of a user template.

Plus diff fixture `tests/diff/emit-use-strict/` + new integration test `tests/diff_emit_use_strict.rs`.

117 unit + 8 integration tests passing. Clippy clean.

### Behavior changes (user-visible)

- `closurec --emit_use_strict --js app.js` now actually emits the directive. Previously parsed but ignored.

### Version

Bumps closurec 0.8.0 → 0.9.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.8.0] - 2026-05-25

### Added — CLOC11.31: `--isolation_mode IIFE` wrapping

Companion to CLOC11.30 in Track 6. When `--isolation_mode IIFE` is passed, the compiled output is wrapped in `(function(){…}).call(this);` — matching CC's `CompilerOptions` IIFE behavior. `--isolation_mode NONE` (the default) is unchanged.

- **New `wrapper::apply_iife_wrap(compiled) -> String`.** Emits `(function(){<compiled>}).call(this);` — using `.call(this)` rather than the simpler `()` form to preserve outer `this` binding the same way CC has since the option was introduced. Pinned by a test (`iife_wrap_uses_call_this_not_bare_invocation`) so a future "simplification" can't silently regress.
- **Pipeline ordering.** IIFE wrapping runs *after* `--output_wrapper` template substitution but *before* writing to disk/stdout. So a `--output_wrapper '// banner%n%%output%'` + `--isolation_mode IIFE` produces `(function(){// banner\n<compiled>}).call(this);` — banner sits *inside* the IIFE, matching CC's layered behavior.
- **Diff fixture** `tests/diff/isolation-iife/` per CLOC11 §3.
- **New integration test** `tests/diff_isolation_iife.rs` drives the built binary against the fixture.
- **4 new unit tests** in `wrapper::tests`: basic wrap, empty body, content-preservation, `.call(this)` form is pinned.

### Pipeline matrix (cumulative across CLOC11)

After CLOC11.31 lands, `run_compiler` does:

1. Per-input `transform_source`:
   - 1a. `--compilation_level` (CLOC11.06: WHITESPACE_ONLY active)
   - 1b. `--define / -D` substitution (CLOC11.19)
2. Concatenate transformed inputs (`combined`)
3. `--output_wrapper` template substitution (CLOC11.30)
4. **`--isolation_mode IIFE` wrap (CLOC11.31, new)**
5. Write to `--js_output_file` (auto-create parents) or stdout

### Behavior changes (user-visible)

- `closurec --isolation_mode IIFE --js app.js` now actually wraps. Previously the flag was parsed but ignored.

### Tests

113 unit + 7 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.7.0 → 0.8.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.7.0] - 2026-05-25

### Added — CLOC11.30: `--output_wrapper` template substitution

Third behavioral slice of [CLOC11], landing the first piece of Track 6 (output formatting). `closurec` now honors `--output_wrapper <template>` (and the companion `--output_wrapper_file <path>`) end-to-end.

- **New `wrapper` module.** Single forward-scan template substituter recognizing two placeholders per CC's documented behavior:
  - `%output%` → the compiled JS (the result of all prior pipeline stages: transform_source + defines + concatenation).
  - `%n%` → a literal newline character.
  
  Unrecognized `%name%` placeholders (e.g. `%foo%`) pass through verbatim — CC's behavior. Lone `%` signs without a closing partner before a non-name character (e.g. `50% off`) also pass through unchanged.

- **`--output_wrapper_file` overrides `--output_wrapper`** when both are supplied. The file's contents become the wrapper template. Matches CC's documented behavior ("loads the specified file and passes its contents to `--output_wrapper`").

- **Pipeline ordering**: applied in `run_compiler` *after* the per-input transform and concatenation, *before* writing to disk or stdout. So the wrapper sees the final compiled JS — including everything WHITESPACE_ONLY, defines, and any future passes contributed.

- **Fast-path passthrough.** When neither `--output_wrapper` nor `--output_wrapper_file` is set, `apply_output_wrapper` returns the compiled string unchanged without allocating. The common case stays cheap.

- **New `CompilerError::Wrapper(WrapperError)`** variant. The single failure path today is `--output_wrapper_file` pointing at a non-readable path; we surface a typed `WrapperFileReadError` with the path, `io::ErrorKind`, and message.

- **Diff fixture** `tests/diff/output-wrapper/` per CLOC11 §3: a tiny input file, a flags file invoking `--output_wrapper '(function(){%output%})();'`, and the expected wrapped output.

- **New integration test** `tests/diff_output_wrapper.rs` drives the built binary against the fixture and asserts byte-equal stdout.

- **14 new unit tests in `wrapper::tests`** covering: no-wrapper passthrough, `%output%` substitution, `%n%` newline expansion, unrecognized placeholder passthrough, lone `%` passthrough (`50% off`), wrapper without `%output%`, multiple `%output%`s all substitute, `--output_wrapper_file` override, missing file → typed error, trailing `%n%`, empty compiled + wrapper, Unicode-in-template, error display, low-level scanner edge cases.

### Pipeline matrix (cumulative across CLOC11)

After CLOC11.30 lands, `transform_source` per input does:

1. `--compilation_level` transform (CLOC11.06: WHITESPACE_ONLY active; others identity until CLOC11.07+).
2. `--define / -D` substitution (CLOC11.19).

Then `run_compiler` does:

3. Concatenation of transformed inputs.
4. **`--output_wrapper` template substitution (CLOC11.30, new).**
5. Write to `--js_output_file` (auto-create parent dirs) or stdout.

### Behavior changes (user-visible)

- `closurec --output_wrapper '(function(){%output%})();' --js app.js` now actually wraps. Previously the flag was parsed but ignored.
- `closurec --output_wrapper_file banner.txt --js app.js` reads `banner.txt` as the wrapper template.

### Tests

109 unit + 6 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.6.0 → 0.7.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.6.0] - 2026-05-25

### Added — CLOC11.19: `--define / -D` value substitution

Second behavioral slice of [CLOC11]. Users can now pass `--define NAME=value` (or `-D NAME=value`) and closurec will substitute every reference to `NAME` with `value` in the output.

- **New `defines` module.** Token-level substitution: tokenize via `javascript-lexer`, walk tokens, replace any identifier-type token whose value matches a `--define` key with the typed value rendered as JS source. Keywords (`if`, `var`, etc.) are explicitly NOT eligible. String-literal content is NOT substituted — `"DEBUG"` stays a string even if `DEBUG` is defined.
- **`DefineValue` rendering** for each variant of [`crate::config::DefineValue`]:
  - `Bool(true)` → `true`
  - `Bool(false)` → `false`
  - `Null` → `null`
  - `Number(42.0)` → `42` (integer-valued doubles emit without trailing `.0`, matching CC)
  - `Number(3.14)` → `3.14`
  - `Number(NaN)` → `NaN` sentinel
  - `Number(Infinity)` → `Infinity` (or `-Infinity`)
  - `String("hi")` → `"hi"` (re-quoted with JS escapes for `"`, `\`, LF, CR, TAB)
- **`transform_source` now runs in two phases:**
  1. **Level transform** — WHITESPACE_ONLY / identity per the compilation level (CLOC11.06 behavior).
  2. **Define substitution** — applies `cfg.defines.defines` over the level's output.
  This ordering means `--define DEBUG=false` composes naturally with `--compilation_level WHITESPACE_ONLY` (or with any future level transform).
- **Fast path:** when `cfg.defines.defines` is empty, `apply_defines` is a string-copy no-op (skips tokenization entirely).
- **New `CompilerError::Define(defines::DefineError)`** variant for substitution failures (currently only "tokenizer rejected the source").
- **Diff fixture `tests/diff/define/`** per CLOC11 §3.
- **New integration test `tests/diff_define.rs`** drives the actual binary against the fixture.
- **17 new unit tests in `defines::tests`** covering: empty defines passthrough, every DefineValue variant (bool/integer/fractional/string/null), case-sensitive identifier matching (`DEBUG` doesn't match `debug`), string-literal content protection, word-boundary preservation (`return DEBUG` → `return false`, not `returnfalse`), no-space-around-punctuation, multiple defines, keyword non-substitution, NaN/Infinity sentinels, embedded-quote re-escape, error display.

### Looseness vs. real CC

This is v1: we substitute *every* reference to a `--define` name, not only references to `goog.define`-annotated variables. In practice this matches what users expect when they pass `--define FLAG_DEBUG=false` for a flag they own. The cases where CC would NOT substitute (e.g. a `var FLAG_DEBUG` shadowing the same name) are rare in real builds. CLOC11.21+ will tighten the rule once we have JSDoc `@define`-aware metadata.

### Behavior changes (user-visible)

- `closurec --define DEBUG=false --js app.js` now actually substitutes `DEBUG` references in the output. Previously the flag was parsed but ignored.
- As a side-effect of routing through the tokenizer, the substitution output is already minified (single-space gap between word-like tokens, no spaces elsewhere). CC's WHITESPACE_ONLY does the same thing; we just get it for free.

### Tests

95 unit + 5 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.5.0 → 0.6.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.5.0] - 2026-05-25

### Added — CLOC11.06: `--compilation_level WHITESPACE_ONLY` wired

First *behavioral* compilation-level slice of [CLOC11]. CLOC11.01–03 wired the I/O layer; CLOC11.06 starts actually transforming JavaScript. The closurec binary now honors `--compilation_level WHITESPACE_ONLY` end-to-end, matching Closure's documented behavior at this level.

- **New `whitespace_only` module.** Token-level minifier: tokenize via `javascript-lexer::tokenize_javascript_typed`, drop trivia (comments / whitespace / newlines), re-stitch survivors with the minimum-necessary inter-token whitespace. Conservative space-insertion rule: a single space goes between two adjacent *word-like* tokens (identifier, number, keyword, regex, template, BigInt, private name); other adjacencies emit back-to-back.
- **String-literal re-quoting.** The lexer's `Token.value` is *unescaped* content (escape sequences resolved), so emitting it raw would corrupt `var s = "a\"b"`. The minifier re-quotes string tokens with double quotes and re-escapes `"`, `\`, LF, CR, TAB. Matches CC's WHITESPACE_ONLY canonicalization.
- **`transform_source(source, config)` dispatch added to `run.rs`.** New per-level matrix:
  - `WhitespaceOnly` → call into `whitespace_only::whitespace_only_minify`.
  - `Simple` / `Advanced` / `Bundle` / `TranspileOnly` → identity for now; CLOC11.07–10 land their real bodies.
- **`map_language_in_to_es_version`** projects `LanguageVersion` enum → `EsVersion` for the lexer. `Stable` / `EcmascriptNext` / `Unstable` / `NoTranspile` shortcuts all resolve to `EsVersion::latest()` so modern JS isn't silently downgraded.
- **`CompilerError::Minify(MinifyError)`** new variant; carries the underlying tokenizer error message with the offending source context.
- **Two new crate dependencies**: `coding-adventures-javascript-lexer` (the `tokenize_javascript_typed` entry point) and `lexer` (the underlying `Token` / `TokenType` types from the grammar-driven lexer).
- **Diff fixture `tests/diff/whitespace-only/`** per CLOC11 §3: a JS input with line comments, block comments, mixed whitespace, function bodies — `expected.stdout` is the canonical compact emission.
- **New integration test `tests/diff_whitespace_only.rs`** drives the built binary against the fixture and asserts byte-equal stdout.
- **11 new unit tests in `whitespace_only::tests`**: empty input, line-comment stripping, block-comment stripping, whitespace collapsing around punctuation, space-between-keywords (`return typeof x`), space-between-keyword-and-number (`return 1`, must not become `return1`), no-space-around-punctuation, string-literal content preservation through re-quoting, multiline-to-single-line, mixed-comments-and-whitespace, error display.

### Behavior changes (user-visible)

- **`closurec --compilation_level WHITESPACE_ONLY --js foo.js`** now actually minifies. Previously this invocation ran the identity pipeline.
- All other levels still run identity (concatenation) — that changes in CLOC11.07+.

### Implementation note — operating at the token level (not the AST)

The CLOC09 typed AST (`javascript_ast::Program`) and the parser (which produces `GrammarASTNode`) don't yet have a bridge. WHITESPACE_ONLY doesn't need the AST — it operates on tokens — so this PR skips the bridge and uses the lexer directly. Building the AST bridge is on the critical path for CLOC11.07 (`--compilation_level SIMPLE`) and will land then.

### Tests

78 unit + 4 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.4.0 → 0.5.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.4.0] - 2026-05-25

### Added — CLOC11.03: `--js_output_file` write semantics

Third implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). CLOC11.01 wired `--js_output_file` to a simple `fs::write` call; this release brings the disk-write side to behavioral parity with the upstream Java tool.

- **Auto-create parent directories.** A `--js_output_file build/dist/app.min.js` no longer requires a preceding `mkdir -p build/dist`. The upstream `closure-compiler.jar` creates the parent tree automatically; we now match. Implemented as `fs::create_dir_all` on the resolved parent path, gated on `path.parent().is_some()` && `parent.exists() == false` so a bare filename in CWD doesn't try to `create_dir_all("")`.
- **`write_output_file(path, contents)` extracted as its own pub function.** Mirrors the CLOC11.02 pattern of splitting concerns into independently-testable units. The full pipeline (`run_compiler`) now calls it; unit tests can also call it directly.
- **Typed error on parent-create failure.** When `fs::create_dir_all` fails (e.g. the path collides with an existing regular file), we surface `CompilerError::OutputWriteError { path: <parent>, kind, message }` — the path field points at the parent so the user can fix the right thing.
- **Diff fixture `tests/diff/js-output-file/`** per CLOC11 §3: two .js inputs + flags.txt + expected.stdout.
- **Two new integration tests in `tests/diff_output_file.rs`**:
  - `js_output_file_writes_to_disk_with_auto_create_parents` — invokes the real binary with `--js_output_file <fresh-nested-path>`, asserts the file lands with the expected content and stdout stays empty.
  - `omitting_js_output_file_falls_back_to_stdout` — same fixture without the flag, asserts content lands on stdout.
- **Five new unit tests in `run::tests`**:
  - `write_output_file_creates_missing_parent_directories`
  - `write_output_file_bare_filename_does_not_create_dot` (regression: `parent()` of bare filename is `Some("")`; we must skip the `create_dir_all` rather than ask the OS to create an empty path)
  - `write_output_file_reports_create_dir_failure_as_typed_error` (file-where-directory-expected)
  - `run_compiler_autocreates_output_parent_dirs` (end-to-end)
  - `run_compiler_stdout_fallback_when_output_file_absent` (regression pin on the CLOC11.01 behavior)

### Known gap deferred to a follow-up

- **Empty-string value (`--js_output_file ""`) still rejected** by cli-builder's string validator at parse time (per `positional_resolver.rs`). The upstream Closure tool accepts it as a synonym for stdout. Closing this gap requires either (a) a cli-builder change to support `allow_empty: true` per-flag, or (b) a closurec-side argv preprocessor that special-cases the empty value. Both are out of scope for CLOC11.03 — tracked for a separate small PR. Workaround today: simply omit the flag to get stdout.

### What's NOT new

- v0.4.0 does not lex, parse, optimise, or emit JavaScript yet — the pipeline body remains "concatenate inputs". That work begins with CLOC11.06 (`--compilation_level WHITESPACE_ONLY`). CLOC11.03's value is making the I/O layer trustworthy for every later PR to build on.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.3.0] - 2026-05-25

### Added — CLOC11.02: `--js` glob expansion + `!` exclusion

Second implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). CLOC11.01 read `--js` values as literal file paths; this release replaces that with a real glob expander matching Closure's documented semantics.

- **New module `globs`.** Hand-rolled (zero-dep) glob matcher supporting:
  - `*` — matches any sequence within a single path segment.
  - `**` — matches zero or more whole path segments. Only special as a full segment per CC's docs; `src/**.js` is literal.
  - `?` — exactly one character within a segment.
  - `[abc]` / `[a-z]` / `[!abc]` — character classes with range and negation.
  - Literal text otherwise.
- **`!` exclusion.** A `--js` value starting with `!` removes everything it matches from the running included set. Mirrors Closure's behavior: `--js 'src/**/*.js' --js '!src/legacy/**'` includes all `src/` JS then drops the legacy subtree.
- **Walk strategy.** For each inclusion pattern we identify the longest fixed (glob-free) prefix and walk under it only — same optimisation as upstream `CommandLineRunner.findJsFiles`. Directory entries are sorted lexicographically before recursion so expansion is deterministic.
- **`resolve_inputs(config)`** extracted as its own pub function so glob behavior is unit-testable without going through full `run_compiler`. Result: `run_compiler` calls `resolve_inputs` first, then reads the resolved paths.
- **New `CompilerError::GlobExpansion(globs::GlobError)` variant** carrying the typed glob failure (NoMatches / InvalidPattern / WalkError) with the offending pattern.
- **Diff fixture `tests/diff/js-glob/`** per CLOC11 §3:
  - `input/` directory tree with 4 .js files including one excluded subtree.
  - `flags.txt` invoking `--js 'tests/diff/js-glob/input/**/*.js' --js '!tests/diff/js-glob/input/excluded/**'`.
  - `expected.stdout` with the concatenated content of the surviving 3 files in lex order.
- **`tests/diff_glob.rs`** integration test that runs the actual built binary against the fixture and asserts byte-equal output.

### Behavior changes (potentially user-visible)

- **Missing literal paths now error with `GlobExpansion(NoMatches)` instead of `InputReadError(NotFound)`**. Matches Closure's behavior (it emits `JSC_NO_JS_FILES_FOUND_FOR_PATTERN` regardless of whether the input was a glob or a literal). The `missing_input_returns_typed_error` test was updated to assert the new variant.
- **A `--js` invocation that produces zero matches is now a hard error** (exit code 2), even for literal paths. Closure does the same.

### Tests

21 new unit tests in `globs::tests`:
- 6 pure-function tests: literal vs glob detection, fixed-prefix splitting (including absolute paths), segment-matcher behavior for literals, `*`, `**`, `?`, char classes (positive, range, negative), invalid char class, error display.
- 9 filesystem-backed tests: literal-path passthrough, missing literal errors, `*.js`, `**/*.js` recursion, exclusion, no-matches error, invalid-pattern error, dedupe across overlapping inclusions, order preservation across patterns, subtree exclusion via `**`.

Plus the integration diff test brings the binary's total to 60 tests passing.

### Architecture

`globs.rs` is a single self-contained module under `code/programs/rust/closurec/src/`. No new crate dependencies. Per the repo's zero-dep working principle, this implements just enough of POSIX glob to match Closure's documented surface. Brace expansion (`{a,b}`), capture groups, and other features beyond Closure's surface are not supported and are not part of the v1 scope.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.2.0] - 2026-05-24

### Added — CLOC11.01: CompilerConfig + identity build wiring

First implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). Previously `closurec` validated argv, then printed `"closurec v0.1.0 - identity pipeline\n"` and exited — flag values were dropped on the floor. This release threads them through.

- **New module `config`.** A typed `CompilerConfig` struct with 18 per-feature sub-structs (`IoConfig`, `CompilationConfig`, `LanguageConfig`, `FormattingConfig`, `SourceMapConfig`, `DiagnosticsConfig`, `DefinesConfig`, `DependenciesConfig`, `ChunksConfig`, `PolyfillsConfig`, `RenamingReportsConfig`, `ExportsConfig`, `ConformanceConfig`, `InstrumentationConfig`, `SpecialModesConfig`, `SpecialPassesConfig`, `TranslationsConfig`, `JsonStreamsMode`). One sub-struct per row in CLOC11 §4's flag inventory, so later CLOC11.* PRs add lines, never new architecture.
- **New module `wire`.** `pub fn config_from_parsed(parsed: &ParseResult) -> Result<CompilerConfig, ConfigError>` translates cli-builder's `HashMap<String, serde_json::Value>` into the typed config. Every one of the 100 declared Closure Compiler flags gets read here; v1 of this PR only *uses* the I/O fields downstream, but all 100 flag slots are populated and tested.
  - `ConfigError::SpecMismatch` for "cli.spec.json says string but runtime got integer" — catches spec/wire drift loudly.
  - `ConfigError::InvalidDefine` for `--define NAME=value` values that aren't valid JS literals. Closure-strict semantics: bare unquoted strings rejected.
  - `ConfigError::Conflict` reserved for incompatible flag combinations in later PRs.
- **New module `run`.** `pub fn run_compiler(config: &CompilerConfig) -> Result<CompilerOutput, CompilerError>` executes the compiler. v1 = identity pipeline: read every `--js` literal path, concatenate with newline separators in input order, write to `--js_output_file` or stdout. CLOC11.02 will replace literal-path reads with glob expansion.
  - `CompilerError::InputReadError` / `OutputWriteError` carry the `io::ErrorKind` so callers format meaningfully without losing the underlying cause.
- **`main::parse_and_run` rewired.** The `ParserOutput::Parse` branch now calls `wire::config_from_parsed` → `run::run_compiler` and surfaces their results. Exit codes:
  - `0` — success (clean parse + successful compile).
  - `1` — argv parse error (unchanged).
  - `2` — compilation error (new; covers I/O failures and config validation).
- **23 new tests** across the three modules (config: 3, wire: 12, run: 7) plus updated existing CLI tests.

### Changed

- The "identity pipeline" banner now appears only when `--js` is absent. With `--js` inputs the binary reads + writes them.
- Pre-existing CLI-surface tests that fed nonexistent `--js` paths and pinned the banner string now assert "parses cleanly" (no `unknown`/`invalid` markers) rather than pinning the banner. The CLI *surface* contract is unchanged.

### Architecture notes

Per [CLOC11 §5], the bridge between cli-builder's untyped flag map and the compiler pipeline is one typed `CompilerConfig` with per-feature sub-structs. Adding a flag in any later CLOC11.* PR follows a fixed recipe:

1. Add a field to the appropriate sub-struct in `config.rs`.
2. Map it in the corresponding `read_*` function in `wire.rs`.
3. Consume it in `run.rs`.
4. Add a diff test under `tests/diff/<feature>/` (CLOC11 §3).

No new architectural pieces are needed per flag.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.1.0] - 2026-05-23

### Added
- New program per CLOC08 — the CLI driver that ties together every crate in Stages 1–4 (lexer, parser, type sidecar, JSDoc extractor, type-checker, pass pipeline + every canonical pass per CLOC06, emitter, source-map generator).
- **Drop-in compatibility with the upstream Java Closure Compiler at the command-line surface.** A script written against `java -jar closure-compiler.jar --js foo.js --js_output_file out.js --compilation_level ADVANCED` works unchanged when the `java -jar …` invocation is swapped for `closurec`.
- All ~100 flags from `CommandLineRunner.java` declared in [`cli.spec.json`](./cli.spec.json), a [cli-builder](../../../packages/rust/cli-builder) JSON spec embedded into the binary via `include_str!`:
  - inputs/outputs: `--js`, `--externs`, `--js_output_file`, `--chunk`, `--chunk_output_path_prefix`, `--chunk_wrapper`;
  - compilation control: `--compilation_level` (`BUNDLE`/`WHITESPACE_ONLY`/`SIMPLE`/`TRANSPILE_ONLY`/`ADVANCED`), `--checks_only`, `--continue_after_errors`, `--use_types_for_optimization`;
  - language: `--language_in`/`--language_out` with the full ECMAScript-3-through-2021 + `STABLE`/`NEXT`/`UNSTABLE` enumeration;
  - source maps: `--create_source_map`, `--source_map_format`, `--source_map_location_mapping`, `--source_map_input`, `--apply_input_source_maps`, `--source_map_include_content`, `--parse_inline_source_maps`;
  - modules: `--module_resolution`, `--js_module_root`, `--process_common_js_modules`, `--rewrite_polyfills`, `--isolate_polyfills`, `--inject_libraries`, `--force_inject_library`;
  - warnings: `--warning_level`, `--jscomp_error`/`--jscomp_warning`/`--jscomp_off`, `--hide_warnings_for`, `--warnings_allowlist_file`, `--extra_annotation_name`;
  - renaming + reports: `--variable_renaming_report`, `--property_renaming_report`, `--rename_variable_prefix`, `--rename_prefix_namespace`, `--variable_map_input_file`, `--property_map_input_file`;
  - output shape: `--isolation_mode`, `--output_wrapper`/`--output_wrapper_file`, `--chunk_output_type`;
  - formatting: `--formatting` (repeatable enum: `PRETTY_PRINT`/`PRINT_INPUT_DELIMITER`/`SINGLE_QUOTES`), `--charset`, `--emit_use_strict`;
  - conformance + framework hooks: `--conformance_configs`, `--angular_pass`, `--polymer_version`, `--chrome_pass`, `--j2cl_pass`, `--remove_j2cl_asserts`;
  - defines: `--define name[=val]` (short `-D`);
  - coverage: `--instrument_for_coverage_option`, `--production_instrumentation_array_name`, `--instrument_mapping_report`;
  - dependency management: `--dependency_mode`, `--entry_point`;
  - tracing + debugging: `--debug`, `--print_tree`/`--print_tree_json`/`--print_ast`, `--print_source_after_each_pass`, `--tracer_mode`, `--logging_level`, `--summary_detail_level`, `--output_manifest`, `--output_chunk_dependencies`, `--help_markdown`;
  - dynamic imports: `--allow_dynamic_import`, `--dynamic_import_alias`;
  - JSON streams: `--json_streams` (`NONE`/`IN`/`OUT`/`BOTH`);
  - misc: `--browser_featureset_year`, `--env`, `--third_party`, `--flagfile`, `--num_parallel_threads`, `--continue_after_errors`, `--assume_function_wrapper`, `--assume_static_inheritance_is_not_used`, `--assume_no_prototype_method_enumeration`, `--renaming`, `--error_format`, `--expected_diagnostics`.
- Short aliases honored: `-O` → `--compilation_level`, `-W` → `--warning_level`, `-D` → `--define`.
- `--help` / `-h` and `--version` injected automatically by cli-builder; version sourced from `Cargo.toml`.
- `parse_and_run(&[String]) -> (String, ExitCode)` is a **pure function** with no I/O — tests drive it directly without spawning the binary.
- Exit codes: `0` success, `1` parse error, `70` internal error (`EX_SOFTWARE`).
- 15 tests covering: `cli.spec.json` loads cleanly (90+ flags), `--help` long + short produce help text, `--version` returns the crate version, canonical Closure invocations parse (`--js`/`--js_output_file`/`--compilation_level`/`--create_source_map`), `--js` is repeatable, unknown flag returns error mentioning the bad flag, invalid enum value returns error, short aliases (`-O`, `-W`, `-D`) work, `--formatting` is a repeatable enum, deprecated hyphenated alias `--checks-only` is rejected (known v0.1.0 gap — see notes), empty argv parses cleanly with defaults, `version_string_matches_crate_version` locks the Cargo.toml ↔ spec sync.

### Changed from the (unmerged) earlier draft
- The earlier `feat/scaffold-closurec` revision used a hand-rolled `std::env::args` parser and a custom flag surface (`--input`, `--output`, `--source-map BOOL`, `--ascii-only BOOL`, `--pretty BOOL`, `--disable NAME`). It was reworked **before merge** at user direction to (a) use `cli-builder` declaratively and (b) be drop-in compatible with the Java Closure Compiler. The custom flag surface is retired.

### Notes
- **Known compatibility gaps in v0.1.0**: cli-builder doesn't currently support multiple long-form aliases per flag, so a handful of deprecated upstream aliases are not implemented. Use the canonical name instead:
  - `--checks-only` → `--checks_only`
  - `--dev_mode` → `--jscomp_dev_mode`
  - `--warnings_whitelist_file` → `--warnings_allowlist_file`
  - `--D` (long form) → `--define` or `-D`
  Real-world Closure invocations use the canonical underscored names; these deprecated forms are rarely seen. Adding alias support to cli-builder is tracked as a v0.2 enhancement.
- v1 is scaffolding. The whole pipeline is identity today (`javascript-ast` ships only `Program` / `SourceType` per CLOC02 Phase 1), so a successful compile prints `closurec v0.1.0 - identity pipeline\n` and exits 0. Real wiring lands when the AST grows nodes. Pinning the Closure-compatible CLI surface now means scripts that invoke the Java tool today can target `closurec` with no flag changes when the body fills in.
- Dependencies: `cli-builder`; every crate scaffolded in Stages 1–4; `serde`/`serde_json`.
- Required capabilities: `fs.read` + `fs.write`. v1 doesn't actually touch the filesystem yet (identity body skips it) but the manifest declares the future surface.
- Source of truth: when upstream Closure Compiler adds a flag, `cli.spec.json` is updated and the binary picks it up via `include_str!`; no Rust code changes are required.

# Changelog

All notable changes to the `coding-adventures-closure-pass-constant-fold` crate will be documented in this file.

## [0.98.0] - 2026-07-17

### Added — standard-constructor `new`-drop: `new Error(…)` → `Error(…)`

The reference Closure Compiler rewrites `new Error(args)` to a plain call
`Error(args)` (saving four bytes). Calling the built-in `Error` as an ordinary
function constructs an Error object *identically* to `new` — ECMAScript
§20.5.1.1: the `Error` constructor's `[[Call]]` and `[[Construct]]` paths
converge, so `Error(m)` and `new Error(m)` both yield a fresh Error with the
same `.message`. The drop is therefore semantics-preserving for **every**
argument list, including the no-arg form:

- `throw new Error("x")` → `throw Error("x")`
- `x = new Error(y)` → `x = Error(y)`, `g(new Error("x"))` → `g(Error("x"))`
- `new Error` / `new Error()` → `Error()`

Byte-identical to the reference compiler at `SIMPLE` across all of the above.

**Scope — deliberately `Error` only.** Gated to a *bare* `Error` identifier
callee (a member callee `obj.Error` or any other name is untouched); like
Closure at SIMPLE this assumes the global `Error` is not shadowed. Three
adjacent folds are intentionally **excluded**:

- **`RegExp`** — `RegExp(r)` returns its argument unchanged when `r` is already
  a regex, whereas `new RegExp(r)` always makes a fresh copy, so
  `new RegExp(x)` → `RegExp(x)` would be an observable change (a potential
  miscompile). The reference compiler does it anyway; we decline to stay sound.
- **`Object` / `Array`** — also spec-safe to drop, but Closure folds their
  *no-arg* forms further to `{}` / `[]` literals, a larger transform.
- **Error subtypes** (`TypeError`, `RangeError`, …) — the reference compiler
  does not fold them, so neither do we.

All three are follow-ups. Additive; MINOR bump 0.96.0 → 0.98.0 (0.97.0 is the
in-flight numeric-object-key PR).

## [0.96.0] - 2026-07-16

### Added — computed string-key member → dot member: `o["foo"]` → `o.foo`

A computed member access whose key is a string literal that is a valid,
**non-reserved ASCII identifier name** now folds to a dot member, matching the
reference Closure Compiler at `SIMPLE` byte-for-byte:

- `o["foo"]` → `o.foo`, `o["$x"]` → `o.$x`, `o["_y"]` → `o._y`, `o["a1"]` → `o.a1`
- `o["let"]` / `o["yield"]` / `o["await"]` / `o["of"]` / `o["undefined"]` → dot
  (none is an ES3 keyword)
- `o.foo["bar"]` → `o.foo.bar` (chains), `o["foo"]=1` → `o.foo=1` (assignment target)

A key that is an **ES3 reserved word** stays bracketed — `o["class"]`,
`o["static"]`, `o["delete"]`, `o["int"]`, `o["boolean"]`, `o["enum"]`,
`o["super"]`, `o["true"]`, `o["null"]`, … — because Closure keeps ES3-unsafe keys
quoted so the output parses under an ES3 target. The reserved set is the classic
Rhino `TokenStream.isKeyword` list (ES3 keywords + the Java-flavoured
future-reserved words + `null`/`true`/`false`), captured in the new
`is_es3_reserved_word` helper. A non-identifier key also stays bracketed:
`o["1a"]` (leading digit), `o[""]`, `o["a b"]`, `o["a-b"]`, `o["é"]` (non-ASCII).

The rewrite composes with the existing string-method / `.length` folds through
the pipeline fixed point: `"abc"["toUpperCase"]()` → `"abc".toUpperCase()` →
`"ABC"`, and `"abc"["length"]` → `"abc".length` → `3`. It also generalises the
string-index fold's `["foo"]` case — an array-literal member with a
non-numeric identifier key (`[a,b,c]["foo"]`) now dots to `[a,b,c].foo` instead
of declining. Additive; MINOR bump 0.95.0 → 0.96.0.

## [0.95.0] - 2026-07-16

### Added — computed **string-key** access into an array literal: `[…]["K"]`

Extends the CLOC12.196 / .196b integer-index fold to computed **string** keys.
The reference Closure Compiler coerces the string key with the SAME full JS
`ToNumber` used by `Number("…")` (this crate's `fold_number`) and then applies
the integer-index fold, so every spelling `ToNumber` maps to an integer selects
that element — **including the non-canonical ones**. Verified byte-identical at
`SIMPLE` against `closure-compiler-v20260712.jar`:

| access                   | ToNumber | result   |
|--------------------------|----------|----------|
| `[a,b,c]["0"]`           | `0`      | `a`      |
| `[a,b,c]["01"]`          | `1`      | `b`      (leading zero) |
| `[a,b,c]["1.0"]`         | `1`      | `b`      (trailing `.0`) |
| `[a,b,c][" 1"]` / `["1 "]`| `1`     | `b`      (whitespace trimmed) |
| `[a,b,c]["0x1"]`         | `1`      | `b`      (hex) |
| `[a,b,c]["1e0"]`         | `1`      | `b`      (exponent) |
| `[a,b,c][""]`            | `+0`     | `a`      (`ToNumber("")` is `0`) |
| `[a,b,c]["3"]`           | `3`      | `void 0` (out of bounds) |
| `[a,b,c]["-1"]`          | `-1`     | `void 0` (negative) |
| `[a,b,c]["1.5"]`         | `1.5`    | declines (fractional — an ordinary absent-property read) |
| `[a,b,c]["foo"]`         | `NaN`    | declines (see below) |
| `[a,b,c]["length"]`      | —        | `3`      (the length property) |

This is a deliberate match to Closure's behaviour even where it is
technically unsound — `["01"]`/`["1.0"]` are not canonical array indices in the
language, yet Closure folds them, so byte-identity requires we do too.

Two keys `fold_number` can't express get special handling: `"length"` is the
length property (routes to the same element-count fold as `.length`, with the
same no-spread / all-elements-pure guard); and a key whose `ToNumber` is `NaN`
(`"foo"`) declines here — Closure rewrites `["foo"]` to `.foo`, a member-access
*normalisation* that is a separate slice, so the bracket access is left intact.

The logic lives in a new `#[inline(never)]` `fold_array_string_key_access`
helper (mirroring `fold_array_index_access`) so `fold_member`'s recursive frame
stays small. Soundness is inherited from the integer-index fold: a present
element keeps its own side effect while the others must be pure; a `void 0` /
`length` result drops the whole literal, so every element must be pure.

Additive; MINOR bump 0.94.0 → 0.95.0.

## [0.94.0] - 2026-07-15

### Added — array-index out-of-bounds / negative / hole → `void 0` — CLOC12.196b

Extends the CLOC12.196 computed array-index fold (`[a, b, c][K]` → the K-th
element) to cover the cases that read as `undefined`, which the emitter spells
`void 0`:

- **out of bounds** — `K ≥ len`: `[1,2,3][5]` → `void 0`, `[1,2][2]` → `void 0`,
  `[][0]` → `void 0`;
- **negative index** — `[1,2,3][-1]` → `void 0` (post-fold a negative index is a
  `NumericLiteral` with a negative `value`, so it flows through the same
  out-of-range path);
- **in-bounds hole** — `[1,,3][1]` → `void 0` (an elided slot reads as
  `undefined`).

All rows are verified byte-identical against the reference Closure Compiler
(`SIMPLE`). Two guards keep the fold sound:

- a **fractional** index (`[1,2,3][1.5]`) is an ordinary absent-property read,
  not an element pick, so it declines — matching Closure;
- a `void 0` result drops the *whole* array literal, so **every** element must be
  side-effect-free: `[f(),2][5]` declines even though the index is out of bounds
  (this is stricter than the in-bounds present-element fold, which preserves the
  selected element and so only requires the *other* elements to be pure).

The computed-index logic moved into a dedicated `#[inline(never)]`
`fold_array_index_access` helper so the recursive `fold_member` stack frame stays
small (deeply-nested expressions must not overflow the stack — the frame-size
lesson).

## [0.93.0] - 2026-07-15

### Added — contract compound self-assignment `x = x OP E` → `x OP= E` — CLOC12.198

`fold_expression` now contracts a plain-`=` assignment whose right-hand side is a
binary expression on the same target into the compound-assignment form:
`x = x + 1` → `x += 1`, `n = n - 2` → `n -= 2`, `k = k * 2` → `k *= 2`,
`a = a + b` → `a += b`, `s = s + "b"` → `s += "b"`. The rewrite fires for every
arithmetic / bitwise operator that has a compound counterpart
(`+ - * / % ** << >> >>> & | ^`); the relational, equality, `in`, and
`instanceof` operators have no `OP=` form and decline. Verified byte-identical to
the reference Closure Compiler (`SIMPLE`), which performs the same contraction.

The contraction is matched on the binary's **left** operand only, so `x = x + 1`
folds but `x = 1 + x` does **not** — for non-commutative operators (`-`, `/`,
`**`, the shifts) `x = x - 1` (→ `x -= 1`) and `x = 1 - x` are different
computations, and the reference compiler likewise only contracts the
left-operand shape. A different left identifier (`x = y + 1`) also declines.

**Scope:** identifier targets only. A member target such as `o[f()] = o[f()] + 1`
is a deliberate follow-up (CLOC12.198b): its expanded form evaluates the
reference sub-expressions (`f()`) twice while the compound form evaluates them
once, so contracting it is only sound once the object/property is proven
side-effect-free. For a bare identifier binding the reference `x` has no
observable side effect, so `x = x OP E` and `x OP= E` are always interchangeable.

The contraction body lives in an `#[inline(never)]` `fold_assignment` helper so
its locals don't inflate the shared recursive `fold_expression` frame (the same
DoS-guard discipline the member / optional / template arms follow).

## [0.92.0] - 2026-07-15

### Added — fold template literal → string when every substitution is constant — CLOC12.197

`fold_expression` now collapses a `TemplateLiteral` to a `StringLiteral` when
every `${…}` substitution is a stringifiable constant (number / string / boolean
/ null / undefined) and every quasi carries a cooked value: `` `a${1}b` `` →
`"a1b"`, `` `${1}-${2}-${3}` `` → `"1-2-3"`, `` `hello` `` → `"hello"`, `` `` `` →
`""`. Recursion handles nested templates (`` `a${`b${1}c`}d` `` → `"ab1cd"`) and
constant sub-expressions (`` `a${1+2}b` `` → `"a3b"`, the `1+2` folds first).
Verified byte-identical to the reference Closure Compiler; no emitter work is
needed because closurec's string emitter already matches its quote choice
(single-quote when the value contains a `"`) and escaping (`\n`, `\t`, `\\`). A
non-constant substitution (`` `a${x}b` ``, `` `a${f()}b` ``) or an illegal-escape
(tagged-only) quasi declines.

The fold body lives in an `#[inline(never)]` `fold_template_literal` helper so its
`Vec`/`String` locals don't inflate the shared recursive `fold_expression` frame
(that frame is walked once per AST depth level; bloating it lowers the nesting
depth the pass survives — the `deeply_nested_*` stack-safety tests pin this).

Adds a `stringify_const_operand` predicate + four unit tests. NOTE: end-to-end at
SIMPLE this currently only fires on **no-substitution** templates (`` `hello` `` →
`"hello"`) — *substituted* templates (`` `a${x}b` ``) do not yet parse in
closurec's grammar, so the fold, though correct and unit-tested, can't receive one
until that separate grammar/bridge arc lands. Additive; MINOR 0.91.0 → 0.92.0.

## [0.91.0] - 2026-07-15

### Added — fold array-literal index access `[a, b, c][K]` → element — CLOC12.196

`fold_member`'s computed-access path (`arr[K]`) now folds a constant integer index
into the selected element, the companion to the CLOC12.193 array-`.length` fold.
`[1,2,3][1]` → `2`, `[a,b,c][1]` → `b`, nested `[[1,2],[3,4]][1][0]` → `3`. Guards
keep it byte-identical to the reference Closure Compiler (verified against the real
jar): no spread anywhere (`[1,...x][0]` declines — indices are runtime-unknown); the
index is a non-negative integer literal in range (a NumericLiteral is never negative
in the AST, and a fractional / out-of-range index is not a canonical array index, so
`[1,2,3][1.5]` / `[1,2,3][5]` / `[1,2,3][-1]` are left intact); the element at `K` is
present (a hole is left, pending the `void 0` follow-up); and every element EXCEPT
the selected one is side-effect-free — the SELECTED element is preserved verbatim, so
`[a, b()][1]` folds to `b()` (its call still runs) while `[a, b()][0]` declines (it
would drop `b()`). CV provenance recorded per fold. Three unit tests (fold cases +
selected-side-effect preservation + each decline). Out of scope for this PR (a tight
follow-up needing `void 0` construction): out-of-bounds / selected-hole → `void 0`,
and canonical string-index keys (`[1,2,3]["0"]` → `1`). Additive; MINOR 0.90.0 → 0.91.0.

## [0.90.0] - 2026-07-14

### Added — fold array-literal `.length` → element count — CLOC12.193

`fold_member` now folds `[e0, e1, …].length` to the element count, mirroring the existing string-literal
`.length` fold. Two guards keep it byte-identical to the reference Closure Compiler (verified against the
real jar): a spread element (`[...x]`) makes the length statically unknown, so it declines; and an element
with a side effect must not be dropped (evaluating the array literal runs it), so it declines unless every
present element is side-effect-free. Holes (`[,,]`) evaluate nothing but still count toward the length
(`[,,].length === 2`). Adds a conservative `is_side_effect_free` predicate (literals, identifier, property
read, and pure operators over pure operands are free; call/new/assignment/`++`/`--`/await/yield/tagged-
template/dynamic-import/spread/object-literal/class-expression are not). Truth table matched byte-for-byte:
`[1,2,3]`→3, `[]`→0, `[,,]`→2, `[a,b]`→2, `[a+b,c]`→2, `[x.y]`→1 FOLD; `[a=1]`, `[g()]`, `[1,2,...x]` DON’T.
Additive; MINOR.

## [0.89.0] - 2026-07-14

### Added — fold default-parameter expressions — CLOC12.191 PR1

Picks up javascript-ast 0.42.0. New `fold_params` helper folds each `FunctionParam::AssignmentPattern`
default `right` through the same `fold_expression` path a function body uses, so `function f(a = 1 + 2){}`
shrinks to `function f(a = 3){}`. Applied at all four param-list sites (function declaration/expression,
class method, arrow). Plain and rest params clone verbatim (no default expression).

## [0.88.0] - 2026-07-12

### Added — CLOC12.189 PR1: export declaration rebuild arms clone each export verbatim

Exhaustive-match arms for the three new `Declaration::Export*` variants
(`ExportNamedDeclaration` / `ExportDefaultDeclaration` / `ExportAllDeclaration`).
PR1 keeps the nodes unreachable (no bridge yet), so the arms are conservative —
rebuild arms clone each export verbatim. Proper descent into an `export const x = 1`'s inner declaration and the
renaming-soundness gate land with the bridge PR.

## [0.87.0] - 2026-07-11

### Added — CLOC12.188 PR1: `ImportDeclaration` rebuild arm

Exhaustive-match arm for the new `Declaration::ImportDeclaration` variant — an
import binds foreign-linked names and has no foldable body, so the rebuild clones
it unchanged.

## [0.86.0] - 2026-07-11

### Added — CLOC12.187 PR1: fold through `WithStatement`

New `TaggedStatement::WithStatement` arm folds the object expression and recurses
into the body (mirroring the `while` arm). Picks up javascript-ast 0.38.0.

## [0.85.17] - 2026-07-11

### Added — CLOC12.177 PR1: `PropertyKey::PrivateName` arms

`javascript-ast` 0.36.0 added the `PropertyKey::PrivateName` variant. The three
exhaustive `PropertyKey` matches gain an arm: the object-literal key rebuild
passes a private name through unchanged (unreachable in valid input — an object
literal never holds a private key — but the match must stay exhaustive), and the
two `Object.keys`/`Object.entries` key-name extractors decline the fold on a
private name (same as a computed key). PATCH.

## [0.85.16] - 2026-07-11

### Added — CLOC12.176 PR1: `ClassMember::StaticBlock` arm

`javascript-ast` 0.35.0 added `ClassMember::StaticBlock(BlockStatement)`, the third class member (a `static { … }` initialization block). Added a `StaticBlock` arm to `fold_class_body`: each statement of the block is folded (rebuilding the `BlockStatement`), mirroring the method-body fold. Reachable once the PR2 bridge produces the node.

## [0.85.15] - 2026-07-11

### Added — CLOC12.175 PR1: fold class-field initializers

`javascript-ast` 0.34.0 added `ClassMember::Field`. Added a `Field` arm to the
class-body member map: a field's initializer (and computed key) is folded with
`fold_expression`, mirroring how a method value is folded. Reachable once the
CLOC12.175 PR2 bridge produces the node.

## [0.85.14] - 2026-07-10

### Added — CLOC12.174 PR1: `Declaration::ClassDeclaration` fold arm

`javascript-ast` 0.33.0 added the `Declaration::ClassDeclaration` variant, making
this crate's exhaustive `Declaration` match non-exhaustive. Added a
`fold_class_declaration` arm that folds inside the `extends` operand and each
method body — identical to `fold_class` for the expression form. The shared
member-folding logic was factored into a new `#[inline(never)] fold_class_body`
helper used by both. Reachable once the CLOC12.174 PR2 bridge produces the node.

## [0.85.13] - 2026-07-08

### Added — CLOC12.173 PR1: fold inside a `ClassExpression`

`fold_expression` gains a `ClassExpression` arm (required now that
`javascript-ast` 0.32.0 adds the variant to the exhaustive `Expression` match).
It folds the `extends` operand (an expression) and each method body (statements)
exactly as the `FunctionExpression` arm folds inside a function body — a class
is never itself a foldable constant. Delegated to an `#[inline(never)]`
`fold_class` helper so the new arm does not enlarge `fold_expression`'s
debug-build stack frame; the whole `match` sits on the hot recursive-dispatch
path, where a fat frame is a deep-nesting stack-overflow (DoS) hazard (per
lessons.md). Reachable once the CLOC12.173 PR2 bridge produces `ClassExpression`
nodes.

## [0.85.12] - 2026-07-07

### Changed — CLOC12.169: `ImportExpression` exhaustive-match arm

Added an `Expression::ImportExpression` case so the pass stays exhaustive over the new `javascript-ast` single-operand variant (part of the CLOC12.169 atomic node PR1). A dynamic `import(source)` carries one sub-expression (the module specifier), so the pass recurses into `source` exactly like the sibling `AwaitExpression` arm. No behaviour change to any existing node.


## [0.85.11] - 2026-07-07

### Changed — CLOC12.168: `ImportMeta` exhaustive-match arm

Added an `Expression::ImportMeta` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.168 atomic node PR1). `import.meta` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.85.10] - 2026-07-04

### Changed — CLOC12.167: `NewTarget` exhaustive-match arm

Added an `Expression::NewTarget` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.167 atomic node PR1). `new.target` is a leaf meta-property with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.85.9] - 2026-07-04

### Changed — CLOC12.166: `Super` exhaustive-match arm

Added an `Expression::Super` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.166 atomic node PR1). `super` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals (and, in fold-control-flow, returns its own `cv` from the `expression_cv` accessor). No behaviour change to any existing node.


## [0.85.8] - 2026-07-04

### Changed — CLOC12.165: `ThisExpression` exhaustive-match arm

Added an `Expression::ThisExpression` case so the pass stays exhaustive over the new `javascript-ast` leaf variant (part of the CLOC12.165 atomic node PR1). `this` is a leaf keyword with no sub-expression and is never itself a constant, so it clones through unchanged alongside the literals. No behaviour change to any existing node.


## [0.85.7] - 2026-07-04

### Changed — CLOC12.164: `AwaitExpression` exhaustive-match arm

Added an `Expression::AwaitExpression` arm that rebuilds the node, folding/
recursing into its `argument`, so the pass stays exhaustive over the new
`javascript-ast` variant (part of the CLOC12.164 atomic node PR1). No behaviour
change to any existing node; the await argument is now visited/rewritten exactly
like any other sub-expression the pass already handles.


## [0.85.6] - 2026-07-03

### Changed — CLOC12.163: `YieldExpression` exhaustive-match arm

Added an `Expression::YieldExpression` arm that rebuilds the node, preserving
`delegate` and folding/recursing into the optional `argument` when present, so
the pass stays exhaustive over the new `javascript-ast` variant (part of the
CLOC12.163 atomic node PR1). No behaviour change to any existing node; the
yield argument is now visited/rewritten exactly like any other sub-expression
the pass already handles.


## [0.85.5] - 2026-07-03

### Changed — CLOC12.162: `SpreadElement` exhaustive-match arm

Added an `Expression::SpreadElement` arm that recurses into the spread's
`argument` so the pass stays exhaustive over the new `javascript-ast` variant
(part of the CLOC12.162 atomic node PR1). No behaviour change to any existing
node; the spread argument is now visited/rewritten exactly like any other
sub-expression the pass already handles.

## [0.85.4] - 2026-07-02

### Changed — CLOC12.161: handle `Expression::TaggedTemplateExpression`

Added a `TaggedTemplateExpression` match arm recursing into the `tag` callee
and each `${…}` insert of the applied template, so this pass keeps
compiling and traverses the new `javascript-ast` 0.20.0 node. No behaviour
change for any existing node.

## [0.85.3] - 2026-07-02

### Changed — CLOC12.160: handle `Expression::SequenceExpression`

Added a `SequenceExpression` match arm recursing into each operand so this
crate compiles and traverses the new `Expression::SequenceExpression`
variant. No behaviour change until the bridge produces sequence nodes
(CLOC12.160 PR2).


## [0.85.2] - 2026-07-02

### Changed — CLOC12.159: handle `Expression::NewExpression`

Added a `NewExpression` match arm mirroring `CallExpression` (recurse into the
callee and each argument) so this crate compiles and traverses the new
`Expression::NewExpression` variant. No behaviour change until the bridge
produces `new` nodes (CLOC12.159 PR2).


## [0.85.1] - 2026-07-02

### Changed — CLOC12.158: exhaustiveness for new `Expression::UpdateExpression`

Handle the new `Expression::UpdateExpression` (`++x` / `x++` / `--x` / `x--`)
variant added to `javascript-ast` (0.17.0): the pass recurses into the operand and preserves the read-modify-write verbatim (an update is never a constant and carries a side effect, so it is never folded away). No behaviour
change for existing inputs — the bridge does not yet produce update
expressions (that lands in the CLOC12.158 PR2 bridge-enable), so these arms
are exercised only via hand-constructed AST today.

## [0.85.0] - 2026-07-02

### Added — CLOC12.154: `TemplateLiteral` traversal

Handle the new `Expression::TemplateLiteral` variant by recursing into its `${{…}}` sub-expressions (the `expressions` vector); the `quasis` are fixed leaf string segments with nothing to recurse. Part of the atomic `TemplateLiteral` enum-variant rollout (javascript-ast 0.16.0) — adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR. Template literals introduce no bindings or scopes, so the renaming/inlining arms need no map reduction.

## [0.84.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR.

### Changed — `FOLD_STACK_SIZE` 64 MiB → 128 MiB

The new arrow arm widens `fold_expression`'s per-frame footprint, and on aarch64 (Apple-silicon CI) frames are larger than x86-64. At the ~20 000-deep adversarial chain that the deep-chain DoS regression exercises, the 64 MiB fold-worker stack was near its edge and overflowed on macOS. Doubling to 128 MiB restores a 2× cushion (lazy pages → no cost for real code) so a modest future frame increase can't re-break the regression.

## [0.83.0] - 2026-07-01

### Added — CLOC12.149: fold inside `FunctionExpression` bodies

`fold_expression` now recurses into a `FunctionExpression` body via
`fold_statement`, exactly as `fold_declaration` does for a
`FunctionDeclaration`, so constants inside `var f = function(){ return
1+1; }` fold. The function itself is not a foldable constant.

## [0.82.0] - 2026-07-01

### Added — `String#concat` coerces primitive-constant arguments (closes gap-143)

`"x".concat(1, 2)` now folds to `"x12"`. The existing `String.prototype.concat`
fold only accepted string-literal arguments; it now applies `ToString` to the
primitive-constant argument forms, matching ECMAScript §22.1.3.4:

```
"x".concat(1, 2)   → "x12"       "a".concat(true)       → "atrue"
"a".concat(null)   → "anull"     "a".concat(undefined)  → "aundefined"
```

- Coercion table: string → itself, number → `String(n)`, boolean →
  `"true"`/`"false"`, `null` → `"null"`, `undefined` → `"undefined"`. **Note
  this differs from `Array#join`** (0.81.0), where `null`/`undefined` coerce to
  `""` — `concat` runs `ToString`, whose null/undefined forms are the words.
- A non-constant argument (identifier, call), an **object**, or an array still
  makes the fold **decline** — those have runtime-dependent `toString`.
- The existing UTF-16 length cap (`MAX_CONCAT_UNITS`) still guards against a
  crafted oversized result.

This resolves gap-143 — the last open gap from the `PeepholeReplaceKnownMethods`
port, so **that port is now fully active** (gaps 141/142/143 all closed). Four
new/updated unit tests cover the numeric/boolean/nullish coercions and the
object-argument decline; the `folds_string_concat_with_coerced_args` upstream
placeholder is now an active conformance test. Verified against the full
closurec end-to-end suite and the downstream pass consumers — all green; the
change is purely additive (previously-declined concat calls now fold).

## [0.81.0] - 2026-07-01

### Added — fold `Array.prototype.join` on an array literal of constants (closes gap-142)

`[a, b, c].join(sep)` on an array literal whose elements are all compile-time
constants now folds to a string literal:

```
["a","b","c"].join("-")  → "a-b-c"      [1,2,3].join()   → "1,2,3"
[].join("-")             → ""           [1,true,null].join(",") → "1,true,"
```

- Each element is coerced the way `Array.prototype.join` does (ECMAScript
  §23.1.3.16): `null`, `undefined`, and array **holes** all become `""`;
  numbers and booleans take their `String(...)` form; strings pass through.
- The separator is either **absent** (defaults to `","`) or a single **string
  literal**. A numeric separator (`[1,2].join(0)` → `"102"`, separator coerces
  to `"0"`) or any non-literal separator is DECLINED — left for the runtime so
  the fold stays obviously correct.
- We DECLINE (leave the call intact) when any element is something whose string
  form is runtime-dependent — a nested array/object, an identifier, a call, a
  template literal, etc.
- **DoS guard:** `fold_array_join` caps the accumulated result length
  (mirroring `fold_string_repeat`), so a crafted array literal can't materialize
  an oversized string at compile time.

This resolves gap-142 (opened by the `PeepholeReplaceKnownMethods` port). Seven
new unit tests cover the string/number/boolean/nullish/hole coercions, the
default-comma separator, the empty array, and the non-constant-element /
non-string-separator / nested-array declines; the `folds_array_join` upstream
placeholder is now an active conformance test. Verified against the full
closurec end-to-end suite and the downstream pass consumers — all green; the
change is purely additive (previously-unfoldable joins now fold).

## [0.80.0] - 2026-07-01

### Test — activate the gap-141 upstream placeholder (#88, CLOC12.141)

`folds_math_unary_methods` in
`tests/upstream/peephole_replace_known_methods_test.rs` was an
`#[ignore = "blocked on gap-141"]` placeholder pinning upstream's
`Math.abs`/`floor`/`ceil`/`round` folds. gap-141 shipped in 0.79.0 (#7217), so
the placeholder is now an **active** conformance test — and it gained `ceil`
(`Math.ceil(4.2)` → `5`) and `round` (`Math.round(2.5)` → `3`, half toward
`+Infinity`) assertions alongside the original `abs`/`floor` cases. The port's
module header and `CLOC12-gaps.md` §CLOC12.141 are updated to mark gap-141
RESOLVED; gaps 142 (`Array#join`) and 143 (`String#concat` coercion) remain the
two `#[ignore]` placeholders.

No library code changed — this release is test coverage plus docs.

## [0.79.0] - 2026-07-01

### Added — fold the unary numeric `Math` methods (closes gap-141)

`Math.abs`, `Math.floor`, `Math.ceil`, and `Math.round` on a single numeric
literal now fold to a numeric literal, extending the existing `Math.max`/`Math.min`
support:

```
Math.abs(-5)   → 5      Math.floor(4.7)  → 4
Math.ceil(4.2) → 5      Math.round(2.5)  → 3
```

- Folds only the exact-arity-1, numeric-literal case with a bare-global `Math`
  callee (a shadowed `m.abs(...)` is left alone), so there is no `ToNumber` side
  effect and no receiver ambiguity.
- `Math.round` follows ECMAScript §21.3.2.28 — rounds a half **toward
  +Infinity** (`Math.round(-2.5) === -2`), NOT Rust's round-half-away-from-zero.
  A dedicated `js_math_round` helper handles the one divergent case (an exact
  `.5` fraction), and agrees with JS on the fp-pathological
  `0.49999999999999994 → 0`.
- **Negative-zero care** (mirrors the `Math.max`/`min` folds): any result that is
  `-0`, or any zero-magnitude result from a negative input where JS yields `-0`
  (`Math.ceil(-0.4)`, `Math.round(-0.4)`, `Math.round(-0.5)`, `Math.floor(-0)`),
  is DECLINED — `-0` has no numeric-literal spelling, so emitting it would be a
  sign-bit miscompile. Leaving the call intact is always safe.

This resolves gap-141 (opened by the `PeepholeReplaceKnownMethods` port). Six
new unit tests cover the folds, the half-rounding rule, the negative-zero
declines, the arity / non-literal / non-global declines, and the `js_math_round`
helper directly. Verified against the full closurec end-to-end suite and the
downstream pass consumers (fold-control-flow, inline-variables, dce, emitter) —
all green; the change is purely additive (previously-declined calls now fold).

## [0.78.0] - 2026-07-01

### Added — CLOC12 upstream test port (`PeepholeReplaceKnownMethodsTest`)

Ported the applicable cases from Google Closure Compiler's
`PeepholeReplaceKnownMethodsTest.java` into
`tests/upstream/peephole_replace_known_methods_test.rs` — the eighth CLOC12 port
and the second into this crate (alongside the `PeepholeFoldConstants` port). It
drives the same AST-builder surface as the sibling port (the crate keeps a
minimal dev-dependency set) and pins the String-method folds this pass performs
today.

- **10 active `#[test]`s pass**: `indexOf`, `lastIndexOf`, case conversion,
  `slice`, `substring`, `substr`, `charAt`, `charCodeAt`, `repeat`, `trim`, the
  boolean `includes`/`startsWith`/`endsWith`, and the decline on a non-constant
  receiver.
- **No new closurec bug** — every fold matched on the first run.
- **3 `#[ignore = "blocked on gap-NNN"]` placeholders** for upstream folds this
  pass does not perform: `Math.abs`/`floor`/`ceil`/`round` (gap-141),
  `Array.prototype.join` on array literals (gap-142), and `String#concat` with
  coerced non-string args (gap-143). Pinned to `code/specs/CLOC12-gaps.md`
  §CLOC12.141; run with `--include-ignored` to track progress as they close.

No library code changed — this release is test coverage plus docs.

## [0.77.0] - 2026-06-30

### Fixed — deep operator chains no longer overflow the native stack (DoS)

`fold_binary` folds `b.left`/`b.right` bottom-up, recursing once per operator.
A deeply left-nested chain — the shape the bridge builds for flat source like
`1+1+…+1` (tens of thousands of terms) — recursed once per operator and, past a
few thousand levels, overflowed the caller's ordinary ~2 MiB stack: an
*uncatchable* abort on *untrusted* input. This is the stage that overflowed
*first* on the SIMPLE pipeline (before the emitter).

`ConstantFoldPass::run` now runs `fold_program` on a 64 MiB worker thread
(`FOLD_STACK_SIZE`) via `std::thread::scope` (borrows the program without
`'static`). Output is **identical** to before — deep chains still collapse
fully (`1+1+…+1` → a single number); only the stack size differs — so every
existing fold is unchanged. Regression test folds a 20 000-deep `+` chain to a
single `NumericLiteral` without crashing. Mirrors the emitter's
`EMIT_STACK_SIZE` hardening (`coding-adventures-closure-emitter` 0.18.11).

## [0.76.0] - 2026-06-29

### Changed — `Object.keys` now folds escaped string keys (bridge decode)

`fold_object_keys_names` carried a `key_string.contains('\\')` decline that
guarded against the old bug where a `PropertyKey::StringLiteral`'s `value` held
RAW, un-decoded source text — re-escaping it would have produced a different
name than `Object.keys` returns. The bridge now decodes property-key string
values (see `javascript-parser` 0.19.3), so `value` holds the true property
name and the existing re-escape produces the correct source form. The decline
is therefore removed: `Object.keys({"a\"b": 1})` now folds to `["a\"b"]` instead
of being left as a call. This brings `Object.keys` in line with its sibling
`Object.entries` fold, which never had the guard. The stale soundness comment is
updated, and the `object_keys_escaped_string_key_does_not_fold` test is
repurposed to assert the now-correct fold.

## [0.75.0] - 2026-06-29

### Added — fold static `Object.keys({k: v, …})` → array of key string literals

`Object.keys` (ECMAScript §20.1.2.16) now folds a fully-static, NON-empty
object literal to an array of its own-enumerable string keys:

| call                        | result       |
|-----------------------------|--------------|
| `Object.keys({a: 1, b: 2})` | `["a", "b"]` |
| `Object.keys({x: "hi"})`    | `["x"]`      |
| `Object.keys({a: 1, a: 3})` | `["a"]`      |

(The empty-object case `Object.keys({})` → `[]` was already handled; the new
`fold_object_keys_names` helper drives the non-empty case, mirroring
`fold_object_entries_pairs`.)

The soundness conditions are IDENTICAL to `Object.entries`, deliberately NOT
weaker even though the value is dropped: evaluating the source object literal
still runs each value expression, so `Object.keys({a: foo()})` and
`Object.keys({a: x})` (undeclared `x`) must be left untouched. Every property
must therefore be a plain data property with a side-effect-free primitive
literal value (string / number / boolean / null) and a non-`__proto__`,
non-array-index key; getters/setters/methods, computed keys, and any canonical
integer-index key (which would reorder the result) decline the whole fold.
Duplicate keys collapse to a single first-position entry. Only the bare global
`Object.keys(...)` callee folds — `o.keys(...)` is left alone.

## [0.74.0] - 2026-06-27

### Added — fold static `Math.max(…)` / `Math.min(…)` on numeric literals → numeric

`Math.max` / `Math.min` (ECMAScript §21.3.2.24 / .25) now fold to a numeric
literal when there is at least one argument and EVERY argument is a numeric
literal:

| call                | result |
|---------------------|--------|
| `Math.max(1, 2, 3)` | `3`    |
| `Math.min(1, 2, 3)` | `1`    |
| `Math.max(-5, -1)`  | `-1`   |
| `Math.min(-5, -1)`  | `-5`   |

The reducers `js_math_max` / `js_math_min` model the spec's signed-zero rule
exactly — `Math.max` prefers `+0` over `-0`, `Math.min` prefers `-0` over `+0`
— rather than relying on Rust's `f64::max`/`min`. We DECLINE: any non-literal
argument (`Infinity` and `NaN` are global identifiers, never numeric literals,
and a runtime value is unknown / could change the result), the empty call
`Math.max()` / `Math.min()` (which would need an infinite literal), and a
non-global receiver (`m.max(...)`). We also decline when the result is NEGATIVE
ZERO (`Math.min(0, -0)` → `-0`): `-0` has no numeric-literal spelling (it is
`UnaryMinus` on `0`, and `ToString(-0)` is `"0"`), so emitting it would flip the
sign bit to `+0`. Only `max` and `min` are modelled.

## [0.73.0] - 2026-06-26

### Added — fold static `Array.from("…")` → array of code-point strings

The static `Array.from` (ECMAScript §23.1.2.1) now folds to an array literal when
its single argument is a string literal (and there is no `mapFn`). The string
iterator yields one element per **code point** (like spread `[..."…"]`), so the
astral characters stay whole:

| call                  | result            |
|-----------------------|-------------------|
| `Array.from("abc")`   | `["a", "b", "c"]` |
| `Array.from("")`      | `[]`              |
| `Array.from("a💩b")`  | `["a", "💩", "b"]` (💩 is one element) |

We decline a second `mapFn` argument (its return values are unknown), any
non-string-literal first argument (array-likes/iterables/identifiers/numbers),
and a shadowed receiver — only the bare global `Array.from(...)` callee folds.
## [0.72.0] - 2026-06-27

### Added — fold static `Object.entries({k: v, …})` → `[["k", v], …]`

The static `Object.entries` (ECMAScript §20.1.2.5) — the inverse of
`Object.fromEntries` — now folds a NON-empty object literal to an array of
`[key, value]` pair literals (the empty-object case `Object.entries({})` → `[]`
was already handled):

| call                            | result                |
|---------------------------------|-----------------------|
| `Object.entries({a: 1, b: 2})`  | `[["a", 1], ["b", 2]]`|
| `Object.entries({x: "hi"})`     | `[["x", "hi"]]`       |

Each entry key is emitted as a string literal; the value expression is copied
verbatim. The fold applies ONLY when every property is a plain data property
(`k: v` — no getter, setter, method, or computed key) with a primitive-literal
value (string / number / boolean / null). We DECLINE: a `"__proto__"` key (a
non-computed `{__proto__: v}` is the §B.3.1 prototype setter, not an own
property, so it is not enumerated); any canonical array-index key (`0`, `1`,
`42`, …, which `[[OwnPropertyKeys]]` enumerates ahead of string keys, breaking
the source order we emit); any non-literal value (including the implicit
identifier of a shorthand `{x}`); a non-global receiver; and any arity ≠ 1.
Duplicate keys collapse to one entry (first position, last value), mirroring the
object the literal builds.
## [0.71.0] - 2026-06-27

### Added — fold static `Object.fromEntries([[k, v], …])` → object literal

The static `Object.fromEntries` (ECMAScript §20.1.2.7) — the inverse of
`Object.entries` — now folds to an object literal when its single argument is a
fully-static array of `[key, value]` pairs:

| call                                          | result        |
|-----------------------------------------------|---------------|
| `Object.fromEntries([["a", 1], ["b", 2]])`    | `{a: 1, b: 2}`|
| `Object.fromEntries([[1, "x"]])`              | `{"1": "x"}`  |
| `Object.fromEntries([["a", 1], ["a", 2]])`    | `{a: 2}`      |
| `Object.fromEntries([])`                      | `{}`          |

The fold applies ONLY when every soundness condition holds: exactly one
argument that is an array literal with no holes; every element a 2-element array
literal (no holes) `[key, value]`; the key a string or numeric literal (a
numeric key folds to its ECMAScript ToString, e.g. `1` → `"1"`); and the value a
primitive literal (string / number / boolean / null). Duplicate keys follow the
spec — the property keeps the POSITION of its first occurrence but takes the
value of its LAST (CreateDataPropertyOnObject overwrites in place). Keys that are
valid identifier names emit as bare identifiers (`{a: 1}`), others as quoted
strings (`{"1": "x"}`). We DECLINE any other shape: wrong arity, a non-array
argument, a non-pair element, a non-literal/boolean/null/identifier key, a
non-literal value, or any hole. We also DECLINE a `"__proto__"` key —
`Object.fromEntries` makes an OWN property named `"__proto__"`, whereas
`{__proto__: …}` in an object literal is the §B.3.1 prototype setter, so folding
it would silently change semantics. Only the bare global `Object.fromEntries(...)`
callee folds (never a shadowed `o.fromEntries(...)`).

## [0.70.0] - 2026-06-26

### Added — fold static `Object.is(a, b)` → boolean (SameValue)

The static `Object.is` (ECMAScript §20.1.2.13) now folds to a boolean literal
when BOTH arguments are primitive literals. `Object.is` uses the SameValue
algorithm (§7.2.11), which differs from `===` in exactly two cases:

| comparison          | `Object.is` | `===`   |
|---------------------|-------------|---------|
| `Object.is(NaN, NaN)` | `true`    | `false` |
| `Object.is(+0, -0)`   | `false`   | `true`  |

Everywhere else SameValue agrees with `===`. We fold two number literals (NaN is
the same as NaN; +0 and −0 are distinguished by sign; otherwise `==`), two string
literals (byte-equal), two boolean literals, two `null` literals (`true`), and a
mismatch of literal kinds (`false`, different Type). We DECLINE if either argument
is a non-literal (value unknown) or the arity is not two. Only the bare global
`Object.is(...)` callee folds (never a shadowed `o.is(...)`).
## [0.68.0] - 2026-06-26

### Added — fold static `Number.isSafeInteger(x)` → boolean

The ES2015 static predicate `Number.isSafeInteger` (ECMAScript §21.1.2.5) now
folds to a boolean literal, alongside the existing `Number.isInteger` /
`isFinite` / `isNaN`. Like its siblings it does **no** coercion: a non-Number
argument is `false`. It returns `true` only for an integer whose magnitude does
not exceed 2^53−1 (`Number.MAX_SAFE_INTEGER` = 9007199254740991), the largest
integer the f64 mantissa represents without colliding with a neighbour:

| call                               | result  |
|------------------------------------|---------|
| `Number.isSafeInteger(7)`          | `true`  |
| `Number.isSafeInteger(9007199254740991)` | `true`  |
| `Number.isSafeInteger(9007199254740992)` (2^53) | `false` |
| `Number.isSafeInteger(3.5)`        | `false` |
| `Number.isSafeInteger(1e21)`       | `false` |
| `Number.isSafeInteger("7")`        | `false` |

Only the bare global `Number.isSafeInteger(...)` callee folds (a member access,
not a shadowable free identifier). An identifier / non-literal argument is
declined (type unknown at compile time).
## [0.67.0] - 2026-06-26

### Added — fold static `Array.of(v0, v1, …)` → array literal `[v0, v1, …]`

The static `Array.of` (ECMAScript §23.1.2.3) now folds to an array literal
whose elements are exactly its arguments, in order. Unlike the `Array(n)`
constructor — where a single numeric argument sets the *length* (`Array(7)` is a
length-7 hole array) — `Array.of(7)` is the one-element array `[7]`, so the fold
is sound for any argument list:

| call                | result      |
|---------------------|-------------|
| `Array.of()`        | `[]`        |
| `Array.of(7)`       | `[7]`       |
| `Array.of(1, 2, 3)` | `[1, 2, 3]` |
| `Array.of(f(), x)`  | `[f(), x]`  |

Every element expression is preserved in evaluation order, so no argument is
dropped, duplicated, or reordered and all side effects are retained. Only the
bare global `Array.of(...)` callee folds (never a shadowed `a.of(...)`). A
spread argument would be declined, but the AST has no call-argument spread
variant yet, so every argument is a plain expression and the fold always applies.

## [0.65.0] - 2026-06-26

### Added — fold static `JSON.stringify(…)` → string literal (primitive subset)

The static `JSON.stringify(x)` (ECMAScript §25.5.2) now folds to a string literal
for the primitive literal arguments whose JSON text we can render exactly, and
only the single-argument form (a `replacer`/`space` second argument can change
the result — a replacer function is invoked even on a primitive):

- a NUMBER literal → its `ToString`: `JSON.stringify(42)` → the string `"42"`,
  `JSON.stringify(-7)` → `"-7"`, `JSON.stringify(0)` → `"0"`. Reuses
  `fold_string_of_number`, which declines fractional values and magnitudes
  ≥ 2⁵³ (whose shortest-decimal / exponential spelling could diverge), so those
  leave the call intact (`JSON.stringify(1e21)` is `"1e+21"` in V8 — declined);
- a BOOLEAN literal → `"true"` / `"false"`;
- the NULL literal → `"null"`.

A STRING literal is DECLINED (JSON escaping — quotes, backslashes, control chars,
and the U+2028/U+2029 edge cases — is left to the runtime), as is any array/object
literal (its elements/properties may have side effects, and serialisation
recurses), an identifier, or any other non-literal. Dispatches through the
`MemberExpression` callee arm (alongside the `String.from*` / `Number.isX`/`parseX`
statics) — only the bare global `JSON.stringify(...)` folds, never a shadowed
receiver. The folded text is pure ASCII, so it needs no escaping. Added six unit
tests (the primitive V8-oracle table, and the string / fractional-and-large /
array-object-identifier / second-argument / non-`JSON`-receiver declines).
## [0.62.0] - 2026-06-26

### Added — fold static `Array.isArray(…)` → boolean literal

The static `Array.isArray(x)` (ECMAScript §22.1.2.2) now folds to a boolean
literal for the literal argument shapes whose evaluation has **no observable side
effect to drop**:

- an EMPTY array literal `Array.isArray([])` → `true` (the only literal that IS
  an Array);
- an EMPTY object literal `Array.isArray({})` → `false`;
- a primitive literal — `Array.isArray("x")` / `Array.isArray(42)` /
  `Array.isArray(true)` / `Array.isArray(null)` → `false`.

A **non-empty** array/object literal is DECLINED: replacing the call with a
boolean would discard the element/property expressions and drop any side effect
they evaluate (`Array.isArray([f()])` must still call `f`). An identifier or any
other non-literal argument (unknown type at compile time), or a call with ≠1
argument, is also left for the runtime.

Dispatches through the `MemberExpression` callee arm (alongside the
`String.from*` and `Number.isX`/`parseX` statics) — only the bare global
`Array.isArray(...)` folds, never a shadowed receiver (`a.isArray(...)`). Added
six unit tests (the empty-array `true`, the non-array-literal `false` set, the
non-empty-array decline, and the identifier / second-argument / non-`Array`-
receiver guards).
## [0.60.0] - 2026-06-26

### Added — fold global `isNaN(…)` / `isFinite(…)` → boolean literal

The global predicates `isNaN(x)` / `isFinite(x)` (ECMAScript §19.2.3 / §19.2.2)
now fold to a boolean literal when their single argument is a string- or
number-literal. Both coerce the argument with `ToNumber`, then classify:
`isNaN` is `true` exactly when the result is `NaN`; `isFinite` is `true` exactly
when it is neither `NaN` nor `±Infinity`:

- `isNaN("abc")` → `true`, `isNaN("42")` → `false`, `isNaN(" ")` → `false`
  (`ToNumber(" ")` is `+0`), `isNaN(0)` → `false`;
- `isFinite("1e3")` → `true`, `isFinite("Infinity")` → `false`,
  `isFinite("abc")` → `false`, `isFinite(0)` → `true`.

A new `js_to_number` helper implements the FULL ECMAScript string→number
coercion returning the exact `f64` — **including** `NaN` and `±Infinity` — rather
than declining like `fold_number` (which returns `None` for `NaN`/`Infinity` and
integers beyond `2^53`, because it must emit an exact literal). Since the
predicates only ever read `.is_nan()` / `.is_finite()` and never emit the number,
`js_to_number` can classify every string correctly (non-decimal digits are folded
into an `f64` accumulator that overflows to `Infinity` exactly when the true value
does). The grammar mirrors `fold_number` (same `is_js_trim_whitespace` /
`is_js_decimal_literal`). Booleans are emitted as `!0` / `!1`, matching the
`Boolean(…)` fold. Both are free identifiers, so only the bare callee folds —
never a member access (`window.isNaN`). Added nine unit tests (a V8-oracle
classification table for `js_to_number`, exact-value checks, through-pass folds
for both predicates, the number-literal and `"Infinity"`-string cases, and
non-literal/second-arg/member guards).
## [0.59.0] - 2026-06-26

### Added — fold static `Object.keys/values/entries({})` → `[]`

The static `Object.keys(x)` / `Object.values(x)` / `Object.entries(x)`
(ECMAScript §20.1.2.16/.22/.5) now fold to the empty array literal `[]` when the
single argument is an **empty object literal** `{}`. An empty object has no own
enumerable keys, and evaluating `{}` has no observable side effect, so collapsing
the call to `[]` is sound for all three methods.

Only the empty-object case folds. A **non-empty** object literal is declined: its
property values (and any computed keys / spreads) may have side effects that
collapsing to `[]` would drop, and the result is non-empty anyway. An array
literal, a primitive (`Object.keys("ab")` → `["0","1"]`), an identifier, or any
call with ≠1 argument is also declined. Dispatches through the `MemberExpression`
callee arm (alongside the `String.from*` / `Number.isX`/`parseX` statics) — only
the bare global `Object.keys/values/entries(...)` folds, never a shadowed
receiver. Emits an empty `ArrayExpression` (like the `split` fold). Added five
unit tests (the empty-object `[]` for all three methods, and the
non-empty-object / array-primitive-identifier / second-argument /
non-`Object`-receiver declines).

## [0.55.0] - 2026-06-26

### Added — fold legacy global `escape(…)` / `unescape(…)` → string literal

The legacy global string escapers now fold to a string literal when their single
argument is a string literal (ECMAScript Annex B §B.2.1.1 / §B.2.1.2):

- `escape("a b")` → `"a%20b"`, `escape("~")` → `"%7E"`, `escape("é")` →
  `"%E9"`, `escape("中")` → `"%u4E2D"`, `escape("😀")` → `"%uD83D%uDE00"`;
- `unescape("a%20b")` → `"a b"`, `unescape("%2F")` → `"/"`,
  `unescape("%uD83D%uDE00")` → `"😀"`, `unescape("%")` → `"%"`.

Unlike `encodeURIComponent`/`encodeURI`, `escape`/`unescape` operate on UTF-16
**code units**, not UTF-8 bytes — so the new `escape_js` helper iterates
`encode_utf16()`, emitting `%XX` for a unit below `0x100` and `%uXXXX` for a unit
`0x100` and above. The unescaped set is the ASCII alphanumerics plus the seven
marks `@ * _ + - . /` (note `~` is **not** unescaped here, unlike the `…URI`
encoders). `unescape_js` is the inverse: `%uXXXX` → that code unit, `%XX` → that
code unit, and any `%` not starting a complete escape passes through literally.

Both are free identifiers, so only the bare `escape(...)`/`unescape(...)` callee
folds — never a member access (`window.escape`). `unescape` **declines** (the
call is left for the runtime) only when its result would contain an unpaired
surrogate (e.g. `unescape("%uD83D")`), which has no Rust-`String` / string-literal
representation; since `unescape` never throws, declining is always sound. Added
nine unit tests (V8-oracle tables for both helpers, a round-trip, through-pass
folds, the unpaired-surrogate decline, non-string/second-arg/member guards).
## [0.53.0] - 2026-06-26

### Added — fold static `Number.parseInt(…)` / `Number.parseFloat(…)` → numeric

The ES2015 static methods `Number.parseInt(string[, radix])` and
`Number.parseFloat(string)` (ECMAScript §21.1.2.12/.13) now fold to a numeric
literal. These are the *same function objects* as the global
`parseInt`/`parseFloat` (`Number.parseInt === parseInt`), so they run the
identical algorithm — the fold reuses the existing `fold_parse_int` /
`fold_parse_float` helpers:

- `Number.parseInt("12px")` → `12`, `Number.parseInt("FF", 16)` → `255`,
  `Number.parseInt("0x1F")` → `31`, `Number.parseInt("101", 2)` → `5`;
- `Number.parseFloat("3.14abc")` → `3.14`, `Number.parseFloat("1e3")` → `1000`.

They dispatch through the `MemberExpression` callee arm (alongside the
`String.fromCharCode`/`fromCodePoint` and `Number.isInteger` statics), so only
the bare global `Number.parseX(...)` folds — never a shadowed receiver
(`n.parseInt(...)`). As with the global forms, a `NaN`/`±Infinity` result is
DECLINED (no literal token to substitute: `Number.parseInt("")`,
`Number.parseFloat("Infinity")`), and `parseInt` only folds with a missing or
integer-literal radix. Added five unit tests (V8-oracle through-pass tables for
both methods incl. a radix, the NaN/Infinity declines, the fractional-radix and
non-string-arg declines, and the non-`Number`-receiver guard).
## [0.52.0] - 2026-06-26

### Added — fold static `Number.isInteger/isFinite/isNaN(…)` → boolean literal

The ES2015 static numeric predicates `Number.isInteger(x)` / `Number.isFinite(x)`
/ `Number.isNaN(x)` (ECMAScript §21.1.2.2/.3/.4) now fold to a boolean literal.
**Unlike** the global `isNaN`/`isFinite`, these do **no** `ToNumber` coercion —
the argument must already be a Number or the answer is `false`:

- a NUMBER literal classifies its value directly: `Number.isInteger(42)` →
  `true`, `Number.isInteger(3.5)` → `false`, `Number.isInteger(1e21)` → `true`
  (every f64 magnitude ≥ 2⁵² is integer-valued), `Number.isFinite(42)` → `true`,
  `Number.isNaN(NaN)` → `true`, and `Infinity`/`NaN` → `false` for `isInteger`
  and `isFinite`;
- a STRING / BOOLEAN / NULL literal → `false` for all three, with no coercion
  (`Number.isNaN("NaN")` === `false`, `Number.isInteger("5")` === `false`).

These are STATIC METHOD calls, so they dispatch through the `MemberExpression`
callee arm (alongside `String.fromCharCode`/`fromCodePoint`) — only the bare
global `Number.isX(...)` folds, never a shadowed receiver (`n.isInteger(5)`). An
identifier/array/object argument, or any call with ≠1 argument, is left for the
runtime. Added five unit tests (a V8-oracle table over number literals incl.
`Infinity`/`NaN`/`1e21`, the non-number-literal `false` cases, and
non-literal/second-arg/non-`Number`-receiver guards).

## [0.49.0] - 2026-06-26

### Added — fold global `encodeURI(…)` / `decodeURI(…)` → string literal

The whole-URI escapers `encodeURI(string)` and `decodeURI(string)` now fold to a
string literal when their single argument is a string literal (ECMAScript
§19.2.6.4 / §19.2.6.2). They are the siblings of `encodeURIComponent` /
`decodeURIComponent`, differing only by their treatment of the URI
reserved/structural delimiters `; , / ? : @ & = + $` and `#`:

- `encode_uri` percent-escapes every UTF-8 byte that is not unreserved, but —
  unlike the `…Component` variant — KEEPS the reserved delimiters intact, so an
  already-assembled URI is not corrupted. `encodeURI("a b")` → `"a%20b"`,
  `encodeURI("a/b?c=d")` → `"a/b?c=d"`, `encodeURI("é")` → `"%C3%A9"`,
  `encodeURI("a<b>")` → `"a%3Cb%3E"`.
- `decode_uri` is the inverse, and crucially KEEPS a `%XX` escape ENCODED when
  the byte it would decode to is a reserved delimiter (so reserved structure
  survives a round trip) — the one behavioural difference from
  `decodeURIComponent`. `decodeURI("a%20b")` → `"a b"`, but `decodeURI("%2F")`
  → `"%2F"` (`/` is reserved) where `decodeURIComponent("%2F")` would give `"/"`.

Soundness mirrors the existing global-coercion folds: `encodeURI`/`decodeURI`
are free identifiers a local could shadow, so we fold the **bare identifier**
only — never a member access (`window.encodeURI` is left alone). A string
literal's value is a Rust `&str` (whole Unicode scalars), so the bytes we emit
are exactly the UTF-8 bytes V8 encodes; there is no lone-surrogate input (the
only `encodeURI` throw) to hit. `decodeURI` DECLINES the fold on exactly the two
`URIError` inputs — a malformed `%XX` escape and a `%`-decoded byte run that is
not valid UTF-8 — so we never substitute a value where the runtime would throw.

Added direct-oracle, round-trip, through-the-pass, decline, non-string-argument,
extra-argument, and member-access unit tests, all V8-confirmed.
## [0.48.0] - 2026-06-26

### Added — fold global `encodeURIComponent(str)` / `decodeURIComponent(str)` over a string literal

The two global URI-component functions now fold to a string literal when their
single argument is a string literal (ECMAScript §19.2.6.5 / §19.2.6.3),
modelled exactly like the sibling `parseInt`/`parseFloat` free-identifier
folds.

- `encodeURIComponent` percent-escapes every byte of the literal's UTF-8
  encoding that is **not** an unreserved character — ASCII alphanumerics plus
  the nine marks ``- _ . ! ~ * ' ( )`` — emitting `%XX` with uppercase hex
  otherwise: `encodeURIComponent("a b")` → `"a%20b"`, `encodeURIComponent("é")`
  → `"%C3%A9"`, `encodeURIComponent("/")` → `"%2F"`. The URI *reserved*
  delimiters (`; , / ? : @ & = + $`) that `encodeURI` keeps intact ARE escaped
  here — that asymmetry is the whole point of the `…Component` variant.
- `decodeURIComponent` is the inverse: `decodeURIComponent("a%20b")` → `"a b"`,
  `decodeURIComponent("%C3%A9")` → `"é"`.

**Soundness.** Same "builtins intact" premise and *free identifier* caveat as
`parseInt`/`parseFloat`: we fold only the bare global identifier, never a
member access (`window.decodeURIComponent` is left alone). A string literal's
value is a Rust `&str` (whole Unicode scalars), so `encodeURIComponent` never
hits the lone-surrogate input that throws — every emitted byte is a real UTF-8
byte V8 would encode. `decodeURIComponent` **declines** (returns the call to the
runtime) for exactly the two inputs JS throws a `URIError` on: a malformed
escape (a `%` not followed by two hex digits) and a `%`-decoded byte run that
is not valid UTF-8. Declining a throw is always sound.
## [0.47.0] - 2026-06-26

### Added — fold global `Boolean(…)` → boolean literal on string/number literals

The global `Boolean(value)` coercion (the `ToBoolean` operation, ECMAScript
§7.1.2) now folds to a boolean literal when its single argument is a string or
number literal — the answer is exact and total, so no decline:

- string literal → `false` only for the EMPTY string, else `true`:
  `Boolean("")` → `false`, `Boolean("x")` → `true`, and crucially
  `Boolean("0")` → `true` (a **non-empty** string is truthy even when it looks
  falsy);
- number literal → `false` for `0`/`-0`, else `true`: `Boolean(0)` → `false`,
  `Boolean(-0)` → `false` (since `-0.0 == 0.0`), `Boolean(1)` → `true`. `NaN` is
  falsy but cannot appear as a numeric literal token.

Every other argument — a boolean, `null`, an identifier, a second argument — is
left for the runtime, and like `parseInt`/`parseFloat`/`Number`/`String` it
folds only the **bare** global identifier, never a member access
(`window.Boolean(...)` is untouched).
## [0.46.0] - 2026-06-25

### Added — fold global `String(…)` → string literal on string/number literals

The global `String(value)` coercion now folds to a string literal when its
single argument is a **string** or **integer** number literal (ECMAScript
§22.1.3.1 → §7.1.17 `ToString`):

- string literal → returned unchanged (identity): `String("x")` → `"x"`;
- integer number literal → its decimal spelling: `String(42)` → `"42"`,
  `String(-3)` → `"-3"`, `String(255)` → `"255"`.

The numeric case is handled by a new `fold_string_of_number` helper that folds
**only integer-valued** numbers in the exact-`i64` range (`|n| < 2^53`, where an
integer is both exactly representable and safe to render through `i64`).
**Fractional** numbers are deliberately declined: Rust's `f64::to_string` and
V8's `Number::toString` are both shortest-round-trip but can break an exact
binary tie in opposite directions (a last-digit-off-by-one, e.g.
`String(108868734838530.12)`), so folding them could silently change the
program. An integer, by contrast, has a unique decimal spelling, so the `i64`
path is byte-identical to V8. Declining is always sound (the call is left for the
runtime).

Every other argument — a fractional or `≥ 2^53` number, a boolean, `null`, an
identifier, a second argument — is left intact, and like
`parseInt`/`parseFloat`/`Number` it folds only the **bare** global identifier,
never a member access (`window.String(...)` is untouched).

## [0.45.0] - 2026-06-25

### Added — fold global `Number("…")` → numeric on string literals

The global `Number(string)` coercion now folds to a numeric literal when its
single argument is a string literal (ECMAScript §21.1.1.1 → §7.1.4.1.1
`StringToNumber`). Unlike `parseInt`/`parseFloat` — which read a *prefix* and
ignore trailing garbage — `Number` is **total**: the entire trimmed string must
be a numeric literal, otherwise the result is `NaN`. So `Number("42")` → `42`
but `Number("12px")` is left intact.

Supported spellings, all V8-confirmed:

- decimal, with optional sign / fraction / exponent: `Number("42")` → `42`,
  `Number("  3.5 ")` → `3.5` (surrounding whitespace trimmed), `Number("2.5e-3")`
  → `0.0025`, `Number(".5")` → `0.5`, `Number("5.")` → `5`;
- the empty (or all-whitespace) string → `+0`: `Number("")` → `0`,
  `Number("   ")` → `0` — the one shape that catches people out;
- non-decimal integer literals, **no sign permitted**: `Number("0x1F")` → `31`,
  `Number("0b101")` → `5`, `Number("0o17")` → `15`;
- a leading zero is decimal, *not* octal: `Number("017")` → `17`.

It **declines** (leaves the call for the runtime) whenever the result has no
literal token: `NaN` (`Number("abc")`, `Number("1,2")`, `Number("12px")`,
`Number("1_000")`, `Number("0x+1")`, `Number("-0x1F")`) or `±Infinity`
(`Number("Infinity")`, `Number("1e400")`). For the `0x`/`0b`/`0o` forms it also
declines values above `2^53`, beyond which an `f64` can no longer hold every
integer exactly — so any literal it does emit is bit-identical to the engine's.

Like `parseInt`/`parseFloat` it folds only the **bare** global identifier, never
a member access (`window.Number("5")` is untouched), under the same
builtins-intact premise the rest of the pass relies on.
## [0.44.0] - 2026-06-25

### Added — fold the static `String.fromCodePoint(cp0, cp1, …)` into a string literal

`String.fromCodePoint` now folds to a string literal when every argument is a
non-negative integer literal that is a **valid Unicode scalar** — in
`0..=0x10FFFF` and not a surrogate `0xD800..=0xDFFF` (ECMAScript §22.1.2.2).
Unlike the sibling `fromCharCode` (whose arguments are 16-bit UTF-16 *units*),
each argument here is a whole **code point**, so a single astral argument
suffices: `String.fromCodePoint(128169)` → `"💩"` (U+1F4A9),
`String.fromCodePoint(72, 73)` → `"HI"`, `String.fromCodePoint(128169, 65)` →
`"💩A"`, no args → `""`.

Lands in the same bare-global-`String` identifier-receiver dispatch shape as
`fromCharCode` (`window.String.…` and other non-identifier receivers never
match); the "builtins intact" soundness note carries over (a local `let String`
could mask the global, but we fold anyway, matching Closure Compiler).

New `fold_string_from_code_point` helper builds the string via
`char::from_u32`, which returns `None` for exactly the invalid inputs — a
**surrogate code point** (a valid JS argument that yields a lone-surrogate
string no Rust `String` can hold) and anything **past U+10FFFF**. JS throws a
`RangeError` for an out-of-range / fractional argument, so declining there also
avoids folding a call that would have thrown. A fractional, negative, or
non-literal argument likewise declines. Three V8-oracle unit tests (BMP +
single-astral + empty, surrogate/out-of-range/fractional decline, non-`String`
receiver decline).
## [0.43.0] - 2026-06-25

### Added — fold the static `String.fromCharCode(u0, u1, …)` into a string literal

`String.fromCharCode` now folds to a string literal when every argument is a
non-negative integer literal in `0..=0xFFFF` (ECMAScript §22.1.2.1). The
arguments are UTF-16 code **units**, so `String.fromCharCode(72, 73)` → `"HI"`,
an adjacent high+low surrogate pair assembles one astral scalar
(`String.fromCharCode(0xD83D, 0xDCA9)` → `"💩"`), and `String.fromCharCode()` →
`""`.

This is the first fold whose receiver is the **bare global identifier
`String`** rather than a string/number literal — it lands in a new
identifier-receiver branch of the MemberExpression arm (`window.String.…` and
other non-identifier receivers never match). Soundness follows the same
"builtins intact" premise as the rest of the pass, one notch weaker (like
`parseInt`/`parseFloat`): a local `let String = …` could mask the global, but
we fold anyway, matching Closure Compiler.

New `fold_string_from_char_code` helper. It **declines** (leaving the call) for
a fractional, negative, `>0xFFFF`, or non-literal argument — we don't model
JS's `ToUint16` wrap-around — and for any unit sequence that is not valid
UTF-16, i.e. a **lone surrogate** (a legal JS string but not a Rust `String`,
the same guard `slice`/`charAt`/`codePointAt` use). Five V8-oracle unit tests
(basic + empty, surrogate-pair assembly, lone-surrogate decline,
out-of-range/fractional decline, non-`String` receiver decline).
## [0.42.0] - 2026-06-25

### Added — fold `"a💩b".codePointAt(i)` on string literals into a number

`String.prototype.codePointAt` on a string-literal receiver with a
non-negative integer-literal index now folds to a numeric literal (ECMAScript
§22.1.3.4). The index is a UTF-16 code-unit position; when it lands on a
**high surrogate** immediately followed by a **low surrogate**, the two units
are combined into one astral code point in `U+10000..=U+10FFFF` — the defining
difference from the already-folded `charCodeAt`, which returns a single 16-bit
code unit. Examples: `"abc".codePointAt(0)` → `97`, `"a💩b".codePointAt(1)` →
`128169` (vs `charCodeAt(1)` → `55357`), `"💩".codePointAt(1)` → `56489` (the
lone trailing low surrogate, returned as its bare unit value). An out-of-range
index is JS `undefined`, for which there is no literal, so the call is left
unfolded (conservative).

The branch sits alongside `charCodeAt`/`charAt` in the numeric-index match arm.
All surrogate arithmetic is performed on 16-bit values widened to `u32`, so the
pair combination cannot overflow. Four V8-oracle unit tests cover the BMP path
(agreement with `charCodeAt`), surrogate-pair combination, the lone-low-surrogate
unit value, and the out-of-range decline.

## [0.41.0] - 2026-06-25

### Added — fold `"abcabc".lastIndexOf(needle)` → numeric on string literals

`String.prototype.lastIndexOf` now folds to a numeric literal — the UTF-16
code-unit index of the **last** occurrence of `needle`, or `-1` when absent
(ECMAScript §22.1.3.9, the single-argument form) — when both the receiver and
the search string are string literals. It is the mirror of the already-folded
`indexOf`: it reuses the same machinery but with Rust's `str::rfind` in place of
`str::find`, then re-measures the matched prefix in UTF-16 units with
`encode_utf16()` (an astral char before the hit counts as two units), so
`"💩x💩x".lastIndexOf("x")` → `5`, matching V8.

`"abcabc".lastIndexOf("bc")` → `4`, `"abc".lastIndexOf("z")` → `-1`. An **empty
needle** yields the string *length* in UTF-16 units — `"abc".lastIndexOf("")` →
`3`, not `0` — because the empty string matches at every position and
`lastIndexOf` takes the highest; `str::rfind("")` returns `Some(byte_len)`, whose
UTF-16 re-measure is exactly that length.

Conservative scope mirrors `indexOf`: only the single-argument form folds; the
`fromIndex` overload (`"abc".lastIndexOf("b", 0)`) carries a second argument,
lands in the two-argument arm, and passes through to the runtime, as does a
non-string needle or a non-literal receiver.
## [0.40.0] - 2026-06-25

### Added — fold `"abcde".substr(start[, length])` on string literals

The legacy `String.prototype.substr` (ECMAScript Annex B §B.2.3.1) now folds to
a string literal when the receiver is a string literal and any provided
arguments are integer literals. It completes the slice family (`slice`,
`substring`, `substr`); unlike the other two, its **second argument is a
*length*, not an end index**:

- **Start** — a negative `start` counts from the end and then clamps to 0
  (`"abcde".substr(-2)` begins at index 3 → `"de"`); a non-negative `start`
  clamps to `len`.
- **Length** — when omitted it defaults to "the rest"; the requested count is
  clamped into `[0, len - start]`, so it can never read past the end. A length
  `<= 0` yields `""`.

Examples: `"abcde".substr(1, 2)` → `"bc"`, `"abcde".substr(1)` → `"bcde"`,
`"abcde".substr(-2, 1)` → `"d"`, `"abcde".substr(0, 100)` → `"abcde"`,
`"abcde".substr(10)` → `""`. Indices are UTF-16 code units (sharing the
`slice`/`charAt` machinery), so `"💩ab".substr(2)` → `"ab"`.

Conservative scope mirrors `slice`/`substring`: the helper declines (leaving the
call for the runtime) for a non-integer-literal argument (we don't model
`ToInteger` coercion), more than two arguments, or a cut that would split a
surrogate pair into a lone surrogate (a valid JS string but not a Rust
`String`).
## [0.39.0] - 2026-06-24

### Added — fold `"abcd".substring(start[, end])` on string literals

`String.prototype.substring` now folds to a string literal when the receiver is
a string literal and any provided arguments are integer literals (ECMAScript
§22.1.3.24). It is the sibling of the already-folded `slice`, but with two
distinct semantics, both modelled:

- **Clamping into `[0, len]`** — a negative (or `NaN`) argument becomes `0`; it
  never counts from the end the way `slice` does. `"abcd".substring(-2)` →
  `"abcd"` (whereas `"abcd".slice(-2)` → `"cd"`).
- **Endpoint ordering** — after clamping, the smaller index is the start, so
  `start > end` makes the two SWAP: `"abcd".substring(3, 1)` and
  `"abcd".substring(1, 3)` both → `"bc"`.

Examples: `"abcd".substring(1, 3)` → `"bc"`, `"abcd".substring(2)` → `"cd"`,
`"abc".substring()` → `"abc"`, `"abcd".substring(10)` → `""` (start clamps to
`len`). Indices are UTF-16 code units (sharing `slice`/`charAt` machinery), so
`"💩ab".substring(2)` → `"ab"`.

Conservative scope mirrors `slice`: the helper declines (leaving the call for
the runtime) for a non-integer-literal argument (we don't model `ToInteger`
coercion), more than two arguments, or a cut that would split a surrogate pair
into a lone surrogate (a valid JS string but not a Rust `String`).
## [0.38.0] - 2026-06-24

### Added — fold `"a,b,c".split(separator[, limit])` on string literals into an array

`String.prototype.split` on a string-literal receiver with a string-literal
separator now folds to an **array literal** of the piece strings (ECMAScript
§22.1.3.23) — the first constant-fold that produces an `ArrayExpression` rather
than a scalar. Examples: `"a,b,c".split(",")` → `["a","b","c"]`,
`"axbxc".split("x")` → `["a","b","c"]`, `"abc".split("")` → `["a","b","c"]`
(empty separator splits into single UTF-16 code units), `"".split(",")` →
`[""]`, `"".split("")` → `[]`, `"abc".split()` (no separator) → `["abc"]`. An
optional non-negative integer `limit` caps the piece count
(`"a,b,c".split(",", 2)` → `["a","b"]`, limit 0 → `[]`).

New `fold_string_split` helper returns the pieces (or `None` to decline). The
array node and every produced element carry correlation-vector provenance forked
from the original call, so each output byte traces back to the `split` it came
from. The fold **declines** (leaving the call for the runtime) for: a
non-string-literal separator (a regex separator needs a regex engine; a
numeric/identifier separator would need `ToString` coercion we don't model); a
non-integer / negative / non-literal limit; more than two arguments; and — for
the empty-separator per-code-unit split — a receiver containing an astral
(non-BMP) character, whose surrogate pair would split into a lone surrogate no
Rust `String` can hold (the same hazard `slice`/`charAt` guard against). A
non-empty separator never cuts inside a surrogate pair, so it stays foldable
even for astral receivers (`"a💩b".split("💩")` → `["a","b"]`). No output-size
cap is needed: `split` never amplifies, so unlike `repeat`/`pad` there is no
algorithmic-blowup vector to bound. 11 new V8-oracle unit tests.

## [0.33.0] - 2026-06-23

### Added — fold `"a".replace(from, to)` / `replaceAll(from, to)` on string literals

`String.prototype.replace` and `replaceAll` now fold to a single string
literal when the receiver and **both** the search (`from`) and replacement
(`to`) are string literals — the string-pattern, string-replacement overload
(ECMAScript §22.1.3.19 / §22.1.3.20). `replace` substitutes the first match,
`replaceAll` every match: `"aXbXc".replace("X","-")` → `"a-bXc"`,
`"a-b-c".replaceAll("-","_")` → `"a_b_c"`. A new `fold_string_replace` helper
performs the substitution (`replacen(.., 1)` for `replace`, `replace` for
`replaceAll`).

The string overload matches `from` **literally** — no regex interpretation —
so `"a.b".replace(".","X")` → `"aXb"` (the `.` is a literal dot, not "any
char"). Both operands are valid strings, so a literal substitution can only
produce valid UTF-16; no surrogate pair is ever split.

Two cases JS handles differently from a plain literal copy are declined and
left for the runtime:

- **`$` in the replacement.** When the replacement is a *string*, V8 still
  expands `$$`, `$&`, `` $` ``, `$'`, and `$n` substitution patterns, which a
  verbatim copy would not reproduce (`"abc".replace("b","$&")` → `"abc"` in
  JS). We decline whenever `to` contains `$`.
- **Empty search string.** `replaceAll("", "X")` inserts `X` at every
  code-unit boundary (`"abc".replaceAll("","X")` → `"XaXbXcX"`); a literal
  find/replace cannot reproduce that. An empty `from` declines.

A non-string argument (e.g. `"a1b".replace(1,"X")`), a non-literal receiver,
or the one-argument form leaves the call for the runtime. As with the
`repeat` / `pad` folds, the worst-case output length is bounded *before*
allocating (a `MAX_REPLACE_BYTES` cap, 100 000) so a pathological pair of
large literals can't OOM the optimizer at compile time. Verified against V8.
Adds the `fold_string_replace` helper and ten unit tests (first-vs-all,
literal-not-regex, no-match identity, `$`-decline, empty-search-decline,
non-string-arg, identifier-receiver, wrong-arity, over-size-cap).
## [0.32.0] - 2026-06-23

### Added — fold `"x".startsWith/endsWith/includes(needle)` → boolean on string literals

The single-argument substring predicates `String.prototype.startsWith`,
`endsWith`, and `includes` (ECMAScript §22.1.3.{23,7,9}) now fold to a boolean
literal when both the receiver and the search string are string literals:
`"abc".startsWith("a")` → `true`, `"abc".endsWith("b")` → `false`,
`"abc".includes("b")` → `true`. A new `fold_string_predicate` helper dispatches
to Rust's `str::starts_with` / `ends_with` / `contains`.

These are sound for any pair of literals: JS compares by UTF-16 code unit and
Rust by UTF-8 byte, but both operands are valid `String`s (whole Unicode
scalars, no lone surrogates), so a prefix / suffix / substring relation holds
identically in either encoding — `"a💩b".includes("💩")` is `true` in both, and
the empty needle is always present. Only the single-argument form folds; the
position overloads (`startsWith(needle, pos)`, etc.) carry a second argument,
land in the two-argument arm, and pass through to the runtime. Eight unit tests
cover true/false results, the empty needle, astral-char matching, the position
overload, an identifier receiver, and a non-string needle.
## [0.31.0] - 2026-06-22

### Added — fold `"abc".at(i)` on string literals (negative-from-end indexing)

`String.prototype.at` now folds to a one-code-unit string literal on a string
literal with an integer-literal index (ECMAScript §22.1.3.1):
`"abc".at(0)` → `"a"`, `"abc".at(2)` → `"c"`, and — unlike `charAt` — a
**negative** index counts from the end: `"abc".at(-1)` → `"c"`,
`"abc".at(-3)` → `"a"`. Indexing is by UTF-16 code unit (sharing `charAt`'s
machinery), so `"a💩b".at(-1)` → `"b"`.

Conservative scope: the index must be an integer literal of any sign. An
out-of-range index (`"abc".at(5)`, `"abc".at(-5)`) is `undefined` in JS, for
which there is no literal — so we decline rather than invent `""` (that is
`charAt`'s behavior, not `at`'s). A fractional/non-literal index (we don't
model `ToIntegerOrInfinity` coercion) and a lone-surrogate result
(`"💩".at(0)`, unrepresentable as a Rust `String`) are also left for the
runtime. `saturating_add` keeps the `len + i` index computation from
overflowing on a huge negative literal (the `as i64` cast already saturates a
float past the i64 range), so a saturated index simply lands out of range and
declines — never panics.

When `--correlation_vector` tracking is on, the fold forks a contribution
recording the rewrite (`"abc".at(-1)` → `"c"`).
## [0.34.0] - 2026-06-23

### Added — fold global `parseInt(lit[, radix])` / `parseFloat(lit)` on string literals

The global `parseInt` and `parseFloat` functions now fold to a numeric literal
when their first argument is a string literal (ECMAScript §19.2.5 / §19.2.4):
`parseInt("12px")` → `12`, `parseInt("0x1F")` → `31`, `parseInt("FF", 16)` →
`255`, `parseInt("-7")` → `-7`, `parseInt("08")` → `8` (not octal in modern JS),
`parseFloat("3.14abc")` → `3.14`, `parseFloat("1e3")` → `1000`,
`parseFloat(".5")` → `0.5`, `parseFloat("5.")` → `5`. New `fold_parse_int` and
`fold_parse_float` helpers reproduce the engine algorithm: skip leading
whitespace, read an optional sign, (for `parseInt`) resolve the radix honouring
a `0x`/`0X` prefix, then consume the longest valid numeric prefix and ignore the
trailing garbage. `parseInt` accumulates in `f64` so values beyond `2^53` round
exactly as V8 does.

Conservative scope: the callee must be the **bare identifier** `parseInt` /
`parseFloat` — a member access such as `window.parseInt(...)` is left untouched.
A `parseInt` radix argument, when present, must be an integer literal in
`2..=36` (a non-literal or out-of-range radix leaves the call for the runtime).
We **decline** (leave the call) whenever the runtime result is `NaN`
(`parseInt("")`, an invalid radix) or `±Infinity` (`parseFloat("Infinity")`):
JavaScript has no literal token for either, so there is nothing sound to
substitute.

Soundness note: this folds under the same "builtins are intact" premise the
whole pass relies on, one notch weaker — unlike a string literal's `.slice`,
`parseInt`/`parseFloat` are free identifiers that a local binding could mask, so
we fold them only as the bare global, matching Closure Compiler's treatment of
redefining these globals as out of scope.

When `--correlation_vector` tracking is on, the fold forks a contribution
recording the rewrite (e.g. `parseInt("FF",16)` → `255`).

## [0.30.0] - 2026-06-22

### Added — fold `"a".concat("b", "c")` on string literals

`String.prototype.concat` now folds to a single string literal when the
receiver and **every** argument are string literals (ECMAScript §22.1.3.4):
`"a".concat("b", "c")` → `"abc"`, `"".concat("x")` → `"x"`,
`"foo".concat("bar")` → `"foobar"`, `"a".concat()` → `"a"` (identity, still
dropping the call). A new `fold_string_concat_call` helper performs the join.

Conservative scope: every argument must already be a string literal. JS coerces
non-string arguments via `ToString` (`"a".concat(1)` → `"a1"`), but we don't
model that coercion, so a numeric or identifier argument (and any non-string
receiver) leaves the call for the runtime. Concatenating valid strings can only
produce valid UTF-16 — no surrogate pair is ever split, the hazard `slice` and
`charAt` guard against — so the result is always a representable literal. The
joined length is bounded by a fixed 100_000-UTF-16-code-unit cap (with
`checked_add` on the running total) as a defensive algorithmic-blowup guard,
mirroring `repeat` and `padStart`/`padEnd`.

When `--correlation_vector` tracking is on, the fold forks a contribution
recording the rewrite (`"a".concat("b","c")` → `"abc"`).
## [0.29.0] - 2026-06-22

### Added — fold `"  x  ".trim()` / `trimStart()` / `trimEnd()` on string literals

`String.prototype.trim` / `trimStart` / `trimEnd` (ECMAScript §22.1.3.32/.34/.33)
now fold to a string literal when the receiver is a string literal:
`"  abc  ".trim()` → `"abc"`, `.trimStart()` → `"abc  "`, `.trimEnd()` →
`"  abc"`. Trimming works on whole Unicode scalars, so — unlike `slice` — it
can never split a surrogate pair.

**Soundness note:** the stripped set is hard-coded as the exact ECMAScript
white-space + line-terminator set (`is_js_trim_whitespace`), *not* Rust's
`char::is_whitespace`, because the two disagree: Rust treats U+0085 (NEL) as
whitespace but JS does not, and JS treats U+FEFF (BOM) as whitespace but Rust
does not. Folding with the wrong set would silently miscompile. The set is
U+0009–000D, U+0020, U+00A0, U+1680, U+2000–200A, U+2028, U+2029, U+202F,
U+205F, U+3000, U+FEFF.

8 new unit tests (basic/mixed/empty/interior cases, the full non-ASCII JS set,
explicit exclusion of U+200B/U+2060, identifier-receiver and argument
declines), with V8-derived oracle values. The pre-existing
`unknown_string_method_does_not_fold` test (which used `trim` as its example)
now uses `normalize`.

> Version note: bumped to 0.29.0 — above the merged `repeat` fold (0.26.0), the
> merged numeric `toString(radix)` fold (0.27.0), and the open `padStart/padEnd`
> fold (0.28.0, PR #6571) — so the parallel branches don't collide on the
> version line.
## [0.28.0] - 2026-06-22

### Added — fold `"x".padStart(target[, pad])` / `padEnd(...)` on string literals

`String.prototype.padStart` / `padEnd` (ECMAScript §22.1.3.16 / §22.1.3.17) now
fold to a string literal when the receiver is a string literal, the target
length is a non-negative integer literal, and the optional pad is a string
literal (default a single space): `"5".padStart(3, "0")` → `"005"`,
`"abc".padEnd(6)` → `"abc   "`, `"abc".padStart(6, "12")` → `"121abc"` (the pad
repeats and truncates to the shortfall). A string already at or over the target
is returned unchanged. A new `fold_string_pad` helper works in UTF-16 code
units.

Conservative scope, with a **denial-of-service guard**: declines (leaves the
call) for no argument or more than two, a non-integer target, a non-string-
literal pad, a target over `MAX_PAD_UNITS` (100 000) UTF-16 code units, or a
fill truncation that would split a surrogate pair into a lone surrogate (a valid
JS string but not a Rust `String` — the same guard `slice`/`charAt` use). 6 new
unit tests with V8-derived oracle values.

> Version note: bumped to 0.28.0 — above the merged `repeat` fold (0.26.0) and
> the merged numeric `toString(radix)` fold (0.27.0) — so the parallel branches
> don't collide on the version line.

## [0.27.0] - 2026-06-22

### Added — fold `(N).toString([radix])` on non-negative integer literals

A numeric literal's `toString` now folds to a string literal when the receiver
is a non-negative integer and the radix is known (ECMAScript §21.1.3.6):
`(255).toString()` → `"255"`, `(255).toString(16)` → `"ff"`,
`(255).toString(2)` → `"11111111"`, `(35).toString(36)` → `"z"`. A new
`to_radix_string` helper renders the integer with JS's lowercase `0-9a-z`
digits.

Conservative scope: the receiver must be a non-negative integer below `2^53`
(beyond the safe-integer ceiling JS switches to exponential notation, which a
digit loop would not reproduce); the radix is the default 10 or a single
integer literal in `2..=36`. A fractional receiver (`(3.5).toString(2)` is a
binary fraction we don't model), an out-of-range radix (1, 0, 37 → RangeError),
and a variable radix all pass through unchanged. 7 new unit tests.

> Version note: bumped to 0.27.0 — above the merged `repeat` fold (0.26.0),
> `slice` fold (0.24.0), and `indexOf` fold (0.22.0) — so the parallel branches
> don't collide on the version line.

## [0.26.0] - 2026-06-22

### Added — fold `"ab".repeat(count)` on string literals

`String.prototype.repeat` (ECMAScript §22.1.3.18) now folds to a string literal
when the receiver is a string literal and the single argument is a non-negative
integer literal: `"ab".repeat(3)` → `"ababab"`, `"x".repeat(0)` → `""`. A new
`fold_string_repeat` helper concatenates the whole receiver `count` times —
unlike `slice` it can never split a surrogate pair, since the string is
duplicated, not cut.

Conservative scope, with an explicit **denial-of-service guard**: declines
(leaves the call) for a negative count (JS throws a `RangeError`, which folding
would erase), a fractional/non-finite/non-literal count, or a result that would
exceed `MAX_REPEAT_UNITS` (100 000) UTF-16 code units — `"x".repeat(1e9)` is a
valid program but must not be materialized into a gigabyte literal at compile
time. `checked_mul` keeps the length computation itself from overflowing. 6 new
unit tests.

> Version note: bumped to 0.26.0 — above the merged `slice` fold (0.24.0) and
> the open numeric `toString(radix)` fold (0.25.0, PR #6560) — so the parallel
> branches don't collide on the version line.

## [0.24.0] - 2026-06-22

### Added — fold `"abcd".slice(start[, end])` on string literals

`String.prototype.slice` (ECMAScript §22.1.3.22) now folds to a string literal
when the receiver is a string literal and the (0, 1, or 2) arguments are integer
literals: `"abcd".slice(1, 3)` → `"bc"`, `"abcd".slice(1)` → `"bcd"`,
`"abcd".slice(-2)` → `"cd"`, `"abcd".slice(0, -1)` → `"abc"`,
`"abcd".slice(2, 1)` → `""`, `"abc".slice()` → `"abc"`. A new `fold_string_slice`
helper implements the spec's clamp-and-half-open-range over UTF-16 code units (a
negative index counts from the end).

UTF-16 indexing means `"💩ab".slice(2)` → `"ab"` (the astral char is two units).
Conservative scope: declines (leaves the call) for more than two arguments, a
non-integer-literal argument, an identifier receiver, or a cut that would split
a surrogate pair into a lone surrogate (a valid JS string but not a Rust
`String` — the same guard `charAt` uses). 6 new unit tests.

> Version note: bumped to 0.24.0 (skipping 0.23.0, reserved by the concurrently
> -developed numeric `toString(radix)` fold) so the parallel branches don't
> collide on the version line.

## [0.22.0] - 2026-06-22

### Added — fold `"haystack".indexOf("needle")` on string literals

The single-argument `String#indexOf` (ECMAScript §22.1.3.8) now folds to a
numeric literal when both receiver and needle are string literals:
`"abcabc".indexOf("b")` → `1`, an absent needle → `-1`, and the empty needle →
`0`. The result is the **UTF-16 code-unit** index, not a byte or scalar index:
Rust's `str::find` returns a UTF-8 byte offset, so the matched prefix is
re-measured with `encode_utf16().count()` — `"💩x".indexOf("x")` → `2`
(matching V8), where a naive byte index would be `4` and a char index `1`. For
ASCII the indices coincide.

Conservative scope: only the single-argument form folds. The two-argument
`fromIndex` overload (`"abc".indexOf("b", 1)`) and an identifier/expression
receiver pass through unchanged. 5 new unit tests (found/not-found, empty
needle, UTF-16 counting, two-arg passthrough, identifier-receiver passthrough).

## [0.21.0] - 2026-06-22

### Added — string indexing folds (`"abc".charCodeAt(0)` → `97`, `"abc".charAt(1)` → `"b"`)

`fold_call` now folds the single-integer-index string methods on a string
literal with an integer-literal index:

| before                  | after | reasoning                                  |
|-------------------------|-------|--------------------------------------------|
| `"abc".charCodeAt(0)`   | `97`  | UTF-16 code unit at index 0                |
| `"abc".charCodeAt(2)`   | `99`  | code unit at index 2                       |
| `"💩".charCodeAt(0)`    | `55357` | high surrogate — JS indexes UTF-16 units |
| `"abc".charAt(1)`       | `"b"` | 1-code-unit substring                      |
| `"abc".charAt(9)`       | `""`  | out of range → empty string (JS semantics) |

JS indexes a string by **UTF-16 code unit**, so the fold indexes into
`encode_utf16()` (an astral char occupies two units). The index must be a
**non-negative integer literal**; fractional, negative, or non-literal indices
are left for the runtime (we don't model `ToInteger` coercion or the NaN/`""`
out-of-range cases for those). For `charCodeAt`, an out-of-range index is JS
`NaN` — no literal exists, so it isn't folded. For `charAt`, an out-of-range
index folds to `""`; an in-range index that would yield a **lone surrogate**
(e.g. `"💩".charAt(0)`) isn't folded, because a Rust `String` can't hold a lone
surrogate (`String::from_utf16` fails) — conservative and still sound.

Only the dotted form on a string literal folds; identifier receivers, the
computed form, and other arities pass through. Emits a CV contribution. Seven
new unit tests (in-range/out-of-range/astral/lone-surrogate/fractional/negative/
identifier).

## [0.20.0] - 2026-06-22

### Added — ASCII string-casing folds (`"abc".toUpperCase()` → `"ABC"`)

`fold_expression` now folds the no-argument string-casing methods on a string
literal, via the new `fold_call` helper:

| before                | after   |
|-----------------------|---------|
| `"abc".toUpperCase()` | `"ABC"` |
| `"ABC".toLowerCase()` | `"abc"` |
| `"".toUpperCase()`    | `""`    |

**ASCII-only.** The fold fires only when the literal `is_ascii()`, using Rust's
`to_ascii_uppercase`/`to_ascii_lowercase`. ASCII case mapping is
locale-independent and byte-for-byte identical between Rust and JavaScript, so
the fold is exactly sound. Non-ASCII strings are deliberately left alone — JS
`toUpperCase`/`toLowerCase` use full Unicode default case mapping with
length-changing special cases (`ß` → `SS`, final sigma `ς`) that a conservative
fold-set shouldn't reproduce here, so `"é".toUpperCase()` stays a call.

Narrow surface: only the dotted, zero-argument form on a string literal folds.
`s.toUpperCase()` on an identifier, an argument (`"x".toUpperCase(1)`), the
computed form `"x"["toUpperCase"]()`, and unmodelled methods (`"x".trim()`) all
pass through unchanged. The fold emits a correlation-vector contribution.

Six new unit tests (`fold_ascii_string_to_upper_and_lower_case`,
`non_ascii_string_casing_does_not_fold`, `string_casing_on_identifier_does_not_fold`,
`string_casing_with_argument_does_not_fold`, `computed_string_casing_does_not_fold`,
`unknown_string_method_does_not_fold`).

## [0.19.0] - 2026-06-22

### Added — string-literal `.length` folding (`"hello".length` → `5`)

`fold_expression` now folds the `.length` of a **string literal** to a numeric
literal, via the new `fold_member` helper:

| before        | after | reasoning                                       |
|---------------|-------|-------------------------------------------------|
| `"hello".length` | `5` | five UTF-16 code units                          |
| `"".length`      | `0` | empty string                                    |
| `"💩".length`    | `2` | one astral char = a UTF-16 surrogate pair       |
| `("a"+"b").length` | `2` | object folds to `"ab"` first, then `.length`  |

JavaScript's `String#length` is the count of **UTF-16 code units** (ECMAScript
String exotic objects), not Unicode scalar values or bytes — so the fold uses
`str::encode_utf16().count()`, which expands astral-plane characters
(U+10000…U+10FFFF) to a surrogate pair, matching V8/SpiderMonkey exactly.
`.count()` is total and allocation-free; it cannot panic.

The fold is deliberately narrow — **dotted, non-computed `"...".length` only**:
the object must fold to a `StringLiteral`, and the access must be non-`computed`
with an `Identifier` property named `length`. `s.length` on an identifier,
`"x".charCodeAt`, and the computed form `"abc"["length"]` all pass through
unchanged (the first needs the runtime value of `s`; the last would mean
reasoning about arbitrary computed keys). The fold emits a correlation-vector
contribution like every other fold.

Five new unit tests (`fold_string_literal_length`,
`fold_string_length_counts_utf16_code_units_not_scalars`,
`length_on_identifier_does_not_fold`, `non_length_property_on_string_does_not_fold`,
`computed_string_length_does_not_fold`).

## [0.18.0] - 2026-06-22

### Added — unary bitwise NOT folding (`~5` → `-6`)

`fold_unary` now folds the unary `~` operator on a `NumericLiteral` under ES
`ToInt32` semantics (ECMAScript §13.5.6 `BitwiseNOT`):

| before  | after | reasoning                                   |
|---------|-------|---------------------------------------------|
| `~5`    | `-6`  | `~ToInt32(5)  = ~5  = -6`                    |
| `~-1`   | `0`   | `~ToInt32(-1) = ~-1 =  0`                    |
| `~5.9`  | `-6`  | `ToInt32` truncates toward zero first → `~5` |
| `~~9`   | `9`   | double complement is the `ToInt32` identity (folds bottom-up in one walk) |

This was the lone bitwise gap: the binary `&`/`|`/`^`/`<<`/`>>`/`>>>` operators
already fold via [`to_int32`]/[`to_uint32`] (CLOC15.D), and `~` now reuses the
very same `to_int32` coercion so the unary and binary bitwise paths stay
bit-for-bit consistent. Rust's prefix `!` on `i32` *is* the two's-complement
bitwise NOT, matching JS exactly.

Folding is restricted to a `NumericLiteral` argument: `~x` for an identifier or
call needs the runtime value, and `~"5"`/`~true` would require string/boolean
`ToNumber` coercion that the conservative fold-set deliberately leaves to a
later phase. The fold emits a correlation-vector contribution like every other
fold, so `~5 → -6 → emitted token` stays traceable.

Two new unit tests (`fold_bitwise_not_on_numeric_literal`,
`bitwise_not_on_identifier_does_not_fold`).

## [0.17.0] - 2026-06-21

### Added — negation push for (in)equality (`!(a == b)` → `a != b`)

`fold_unary` now rewrites a logical-not over an (in)equality comparison into the
inverted comparison (upstream Closure's `PeepholeMinimizeConditions`):

| before        | after        |
|---------------|--------------|
| `!(a == b)`   | `a != b`     |
| `!(a != b)`   | `a == b`     |
| `!(a === b)`  | `a !== b`    |
| `!(a !== b)`  | `a === b`    |

Sound for these four operators **only**, because `!=`/`!==` are *defined* as the
boolean negation of `==`/`===` (ECMAScript §13.10) — both sides yield booleans,
so the rewrite is value-identical in every context. Relational operators
(`<`/`<=`/`>`/`>=`) are deliberately **not** inverted: `!(a < b)` is not
`a >= b` when an operand is `NaN` (`!(NaN < 1)` is `true`, `NaN >= 1` is
`false`). The literal-fold path still runs first, so `!(1 == 1)` folds to
`false` rather than `1 != 1`. The rewrite emits a correlation-vector
contribution via `fork_cv`. New helper `invert_equality_operator`; 6 unit tests.

> Only reachable now that the `javascript-parser` bridge stopped dropping the
> `!` operator (it had emitted the bare comparison) — see closurec 0.161.0.

## [0.16.0] - 2026-06-20

### Added — CLOC23: fold inside `for`-`of`

`fold_tagged` now has a `ForOfStatement` arm that recurses constant folding into
the loop's left, the iterable right expression, and the body — identical to the
`for`-`in` handling.

## [0.15.0] - 2026-06-20

### Added — CLOC22: fold inside `for`-`in`

`fold_tagged` now has a `ForInStatement` arm that recurses constant folding into
the loop's left (declaration/expression), the enumerated right expression, and
the body. Constant expressions inside a for-in body fold like anywhere else.

## [0.14.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

`fold_tagged` now covers `DebuggerStatement` (grouped with the other childless
leaf statements). A `debugger;` has no foldable sub-expressions, so it is
returned unchanged — added to keep the match exhaustive over the new variant.

## [0.13.0] - 2026-06-20

### Added — CLOC20: fold inside `do`/`while`

`fold_tagged` now has a `DoWhileStatement` arm that recurses constant folding into
the loop body and test. Constant expressions inside a do-while body (e.g.
`1 + 2` ⇒ `3`) now fold like anywhere else.

## [0.12.0] - 2026-06-20

### Added — CLOC19: fold inside `try`/`catch`/`finally`

`fold_tagged` now has a `TryStatement` arm that recurses constant folding into
the protected block, the catch handler body, and the finalizer, preserving the
catch `param` verbatim. Constant initializers and expressions inside try/catch
blocks (e.g. `1 + 2` ⇒ `3`) now fold like anywhere else.

## [0.11.0] - 2026-06-19

### Added — CLOC15.D: fold bitwise & shift operators on numeric literals

`try_fold_binary_op` now folds the six integer operators on two numeric
literals, matching ECMAScript's 32-bit semantics exactly:

```js
0xFF & 0x3C   // ⇒ 60
1 << 4 | 2    // ⇒ 18
8 >>> 1       // ⇒ 4
```

- `&` / `|` / `^` — both operands coerced via **`ToInt32`**, result is a
  signed 32-bit integer.
- `<<` / `>>` — left operand `ToInt32`; the shift COUNT is `ToUint32(rhs) &
  31` (the low 5 bits). `>>` is arithmetic (sign-propagating).
- `>>>` — left operand **`ToUint32`**, logical (zero-fill) shift; the result
  is an **unsigned** 32-bit value, so it can exceed `i32::MAX`
  (`-1 >>> 0 ⇒ 4294967295`).

New `to_int32` / `to_uint32` helpers implement the spec coercions (non-finite
and `±0` → 0; otherwise truncate toward zero and reduce modulo 2³²). Because
the operands are already numeric literals, the coercions are exact and the
fold can never diverge from the runtime value — deterministic and sound.

`FoldedLiteral` gained `#[derive(Debug)]` (for test diagnostics only).

- 3 new tests: `to_int32`/`to_uint32` spec-vectors, the six operators against
  exact JS reference values (incl. fractional-operand coercion, the
  `>= 2³¹ → negative` wrap, 5-bit shift-count masking, arithmetic `>>`, and
  unsigned `>>>`), and an end-to-end pass run confirming the emitter renders a
  `> i32::MAX` result. Full closurec suite + all constant-fold consumers green,
  no fixture churn.

## [0.10.1] - 2026-06-04

### Added — CLOC12.23: gap-006 unary plus / minus on identifier bookkeeping

Closes `gap-006` from the CLOC12 gap tracker. Pure test-only change —
no production code modified.

The pass already does the right thing structurally: `fold_unary` only
folds `+<literal>` / `-<literal>` (the runtime value of `+x` is
unknown when `x` is an identifier), and `try_fold_binary_op` declines
when either side isn't a recognised literal. So `+x > +y` and friends
pass through verbatim. gap-006 was waiting on bookkeeping — port the
upstream `testSame("+x > +y")` / `testSame("+x == +y")` lines from
`PeepholeFoldConstantsTest::testNumberNumberComparison`.

The new `test_same_unary_on_identifier_in_comparison` test in
`peephole_fold_constants_test.rs` pins:

* `+x > +y`, `+x == +y`, `+x === +y` survive unchanged.
* `-x < -y` survives (Negate variant — same reasoning).
* Asymmetric `0 < +x` survives (literal on one side, unary-of-
  identifier on the other — fold must bail because identifier side
  can't be resolved).
* `+x == +x` survives even with the same identifier on both sides:
  `x` could be NaN at runtime, and `NaN == NaN` is `false`.

Upstream test count: 13 → 14.

## [0.10.0] - 2026-06-04

### Added — CLOC12.22: gap-004 Number/String cross-type abstract equality + relational comparison

Closes `gap-004` from the CLOC12 gap tracker. `try_fold_binary_op` now
coerces a String operand against a Number operand by calling a new
conservative subset of ECMAScript §StringToNumber and evaluating the
resulting Number-vs-Number comparison for the loose equality
operators (`==` / `!=`) and the abstract relational operators
(`<` / `<=` / `>` / `>=`).

Worked examples (upstream-pinned):

* `1 < '2'`  → `true`   (string coerced to 2, then 1 < 2)
* `1 == '2'` → `false`  (loose equality: ToNumber('2') === 2)
* `'2' < 1`  → `false`  (order preserved — NOT swapped to `1 < 2`)
* `'1' == 1` → `true`   (symmetric, string-on-left)
* `1.5 == '1.5'` → `true`

What the new `js_string_to_number_strict` helper recognises:

1. **Empty / ASCII-whitespace-only** → `0.0` (per §StringNumericValue).
2. **`Infinity` / `+Infinity` / `-Infinity`** (case-sensitive, after trim).
3. **Decimal-style numeric literals** — `[+-]?\d*(\.\d*)?([eE][+-]?\d+)?` with at least one digit, lone signs/dots rejected.

What it **does not** handle (deliberate follow-ups; returning `None` bails the fold soundly):

* Hex / binary / octal prefixes (`0x...`, `0b...`, `0o...`).
* Non-ASCII JS WhiteSpace (NBSP, ZWNBSP, U+2028, U+2029, ...).
* Strings that evaluate to NaN per spec (e.g. `"hi"`) — folding these
  to `false` for `==` (or `true` for `!=`) is a future optimisation.

Strict equality on Number/String is untouched — gap-008's branch
already returns `false` / `true` and runs after this one is gated out
by `matches!(op, Eq | NotEq | Lt | LtEq | Gt | GtEq)`.

Tests:

* upstream test `test_number_string_comparison_literal_lines`
  un-ignored — was the canonical pin for this gap.
* 8 new inline unit tests cover: (1) the helper's recognised decimal
  cases, (2) explicit Infinity, (3) the conservative-bail set (hex,
  non-numeric, lone-sign, malformed exponent), (4) the upstream
  cases, (5) order-preservation and symmetry, (6) a full truth table
  for both `1 OP '2'` and `1.5 OP '1.5'`, (7) the gap-008 strict
  regression, and (8) the conservative-bail behaviour on `'hi'`.

Touched pre-existing test: `mixed_type_loose_equality_not_folded`
was asserting the old "don't fold mixed-type comparisons" sound
default, which gap-004 has now narrowed to "don't fold when the
string is unrecognisable". The test was renamed to
`mixed_type_loose_equality_with_unrecognised_string_not_folded` and
its example changed from `1 == "1"` (now folds to `true`) to
`1 == "hi"` (still bails, original intent preserved).

No changes to public API or AST surface.

## [0.9.0] - 2026-06-02

### Added — CLOC12.21: gap-003 `null == <primitive>` cross-type loose-equality fold

Closes `gap-003` from the CLOC12 gap tracker. `try_fold_binary_op` now
implements the `null`-side branch of the ECMAScript abstract-equality
algorithm (§IsLooselyEqual) for compile-time-known partner literals.

What's folded:

* `null == X` and `X == null` where `X` is any non-null primitive
  literal (`number`, `string`, `boolean`, `bigint`, or `undefined`).
* The result is `true` iff the partner is `undefined` (the spec
  hard-codes `null == undefined → true`); every other partner is `false`.
* `null != X` and `X != null` fold to the boolean negation.

Truth table:

```
partner          ==     !=
---------------+------+------
null           | true | false   (already covered by gap-007 path)
undefined      | true | false
number         | false| true
string         | false| true
boolean        | false| true
bigint         | false| true
```

Unsoundness guard: if the partner side is an `Identifier` (or anything
non-literal we can't statically classify), the fold bails out. The
identifier's runtime value could itself be `null`/`undefined`, and
folding to a concrete boolean would change observable behaviour.

Ordering: the new branch runs *after* the existing null/null branch
(gap-007) — so by the time we reach it, at most one side is a
NullLiteral — and *before* the cross-type strict-equality branch
(gap-008), which is unaffected because that branch only fires on
`===`/`!==`. A regression test in the inline tests pins gap-008's
behaviour for `null === 0` / `null !== 0`.

Tests:

* `peephole_fold_constants_test::test_null_comparison_1_loose_against_other_types`
  is un-ignored (was `#[ignore = "blocked on gap-003"]`).
* 6 new inline unit tests cover both directions, the `!=` complement,
  the `null == undefined → true` special case, the identifier
  unsoundness guard, and the gap-008 regression check.

No changes to public API or AST surface.

## [0.8.0] - 2026-06-02

### Added — CLOC12.20: gap-002 `void <pure-literal>` → `undefined` fold

Closes `gap-002` from the CLOC12 gap tracker. `UnaryExpression { operator: Void, argument: <primitive-literal> }` now folds to `UndefinedLiteral`. The canonical case `void 0` (a Closure-Compiler-style synonym for `undefined`) is now resolved.

What's folded:

- `void <NumericLiteral>` → `undefined`
- `void <StringLiteral>` → `undefined`
- `void <BooleanLiteral>` → `undefined`
- `void <NullLiteral>` → `undefined`
- `void <BigIntLiteral>` → `undefined`
- `void <UndefinedLiteral>` → `undefined` (idempotent)

What's deliberately NOT folded:

- `void <Identifier>` — the identifier could refer to a function/getter with side effects.
- `void <CallExpression>` — same, the call has observable side effects.
- `void <MemberExpression>` — property accesses can trigger getters / proxies.
- `void <BinaryExpression>` / etc. — recurses through fold; if the inner folds to a primitive literal, the void rule fires on the next iteration.

Soundness: the general rule `void <expr> → undefined` only holds when `<expr>` has no observable side effects. By restricting to primitive literals, we have a strict subset that's *always* sound. Closes the test surface for `testUndefinedComparison2` from the upstream Closure test suite.

### Implementation

- `FoldedLiteral` enum: new `Undefined` variant. Stamp + label helpers updated to handle it.
- `fn fold_unary` `UnaryOperator::Void` arm: matches the 6 primitive-literal variants, returns `Some(FoldedLiteral::Undefined)`; everything else falls through to `None`.
- `use coding_adventures_javascript_ast::UndefinedLiteral` added to the imports.

### Tests

- `tests/upstream/peephole_fold_constants_test.rs::test_undefined_comparison_2`: un-ignored. 4 assertions (`void 0`, `void 1`, `void "x"`, `void undefined`).

Before this PR:
- Total upstream tests: 14, passing: 10, ignored: 4 (gap-001, gap-002, gap-003, gap-004).

After:
- Total upstream tests: 14, passing: 11, ignored: 3 (gap-001, gap-003, gap-004).

The pending gaps (gap-003 cross-type null comparison, gap-004 abstract-equality / abstract-comparison) are independent of this PR — they need the abstract-equality algorithm implemented, which is a separate body of work.

### Bumped 0.7.1 → 0.8.0

`fold` API and CV-stamping semantics unchanged. Version bump reflects the new fold rule (closes one observable upstream test parity gap).

## [0.7.1] - 2026-06-01

### Added — CLOC12.16: typeof `UndefinedLiteral` folds to `"undefined"`

The constant-fold pass gained three `Expression::UndefinedLiteral`
arms so it compiles against the new `javascript-ast 0.6.0` AST:

1. Leaf passthrough — undefined is itself the folded form.
2. `js_literal_type` returns `"undefined"` so the strict-equality
   fold knows `undefined === <other type>` is `false`.
3. `UnaryOperator::TypeOf` over an `UndefinedLiteral` folds to
   `"undefined"`. This closes the final hole in CLOC12.09's
   typeof-literal fold table.

## [0.7.0] - 2026-06-01

### Changed — CLOC12.15 rebase: handle new `BigIntLiteral` Expression variant

The constant-fold pass gained `Expression::BigIntLiteral` arms in
three places so it compiles against the new `javascript-ast 0.5.0`
AST:

1. Leaf passthrough — a `BigIntLiteral` is already in folded form,
   no children to recurse into.
2. `js_literal_type` returns `"bigint"` so the strict-equality
   fold knows two bigint literals share a type tag with each other
   but not with `NumericLiteral` / `StringLiteral`.
3. `UnaryOperator::TypeOf` over a `BigIntLiteral` folds to
   `"bigint"` (the ECMAScript-correct typeof result).

Bigint arithmetic folding (`1n + 2n` → `3n`) is **not** implemented —
it would require a bigint runtime in the pass crate, which is out
of scope for CLOC12.15. The literal is itself the folded form.

Bumped to 0.7.0 (rather than 0.5.3 originally planned) because this
PR was rebased on top of CLOC12.17 (0.6.0, already on main) — both
landings are additive, so a single fresh minor captures the union.

## [0.6.0] - 2026-06-01

### Added — CLOC12.17: typeof-identity fold (closes gap-029)

Adds a new structural-equality arm in `try_fold_binary_op` that
recognises `typeof <Identifier> === typeof <same Identifier>` and
folds to `true`; the `!==` form folds to `false`.

Truth table:

| Input                       | Output      | Why                          |
|-----------------------------|-------------|------------------------------|
| `typeof a === typeof a`     | `true`      | identical sub-expressions    |
| `typeof a !== typeof a`     | `false`     | identical sub-expressions    |
| `typeof a === typeof b`     | unchanged   | different identifier names   |
| `typeof a == typeof a`      | unchanged   | only strict ops are folded   |

**Safety:** ECMAScript §UnaryTypeofExpression special-cases
`typeof <undeclared-identifier>` to return the string `"undefined"`
instead of throwing a ReferenceError, so even when the binding
doesn't exist, evaluating `typeof x` twice produces the same string
both times. This makes the fold sound regardless of whether the
identifier resolves to a real binding.

The fold deliberately fires only on `Identifier` arguments — not
on member/call expressions — because those can have observable
side effects (getter invocation, function call) that we can't
prove are absent without a heavier purity analysis.

Un-ignores `test_typeof_identifier_identity_fold` in the upstream
test port (`tests/upstream/peephole_fold_constants_test.rs`).

## [0.5.2] - 2026-06-01

### Changed — CLOC12.14: handle new `ThrowStatement` variant

The constant-fold pass gained a `TaggedStatement::ThrowStatement`
match arm so it compiles against the new `javascript-ast 0.4.0` AST.
Behaviour: fold the argument expression (so `throw 2+3;` → `throw 5;`),
preserve the throw semantics.

## [0.5.1] - 2026-06-01

### Changed — CLOC12.13: handle new `LabeledStatement` variant

The constant-fold pass gained a `TaggedStatement::LabeledStatement`
match arm so it compiles against the new `javascript-ast 0.3.0` AST.
Behaviour: recurse into the labelled body (so inner constant-folds
reach inside `a: { foo(2+3); }`), preserve the label verbatim. No
new optimisation; this is purely the "stay non-exhaustive-safe"
mechanical change.

## [0.5.0] - 2026-06-01

### Added — CLOC12.09: close gap-005 typeof literal fold

Implements `typeof <primitive literal>` constant-folding per the
ECMAScript §UnaryTypeofExpression table:

| Operand                  | Folded result |
|--------------------------|---------------|
| `NumericLiteral`         | `"number"`    |
| `StringLiteral`          | `"string"`    |
| `BooleanLiteral`         | `"boolean"`   |
| `NullLiteral`            | `"object"`    |

The `NullLiteral → "object"` case preserves the famous JavaScript quirk
where `typeof null === "object"` (a historical bug baked into the spec).

The four remaining `typeof` cases stay deferred:

- `typeof undefined → "undefined"` — gated on gap-001 (no
  `UndefinedLiteral` AST variant yet).
- `typeof <BigIntLiteral> → "bigint"` — gated on gap-021 (no
  `BigIntLiteral` AST variant yet).
- `typeof <function expression> → "function"` — Phase 1.x AST work.
- `typeof <Identifier>` — left alone (identifier may bind to anything at
  runtime; matches upstream `testSame` lines).

### gap-005 → RESOLVED via CLOC12.09

The CLOC12.02 ignored port `test_typeof_lines_from_string_string_comparison`
is replaced by three focused tests:

| New test | Status |
|----------|--------|
| `test_typeof_literal_comparison_folds` | **passing** (`typeof 3 > typeof 4` → `false`) |
| `test_typeof_identifier_is_left_alone` | **passing** (`testSame` shape) |
| `test_typeof_identifier_identity_fold` | `#[ignore]` on **new gap-029** |

### gap-029 — identity-of-typeof-same-identifier fold (NEW)

Upstream folds `typeof a === typeof a` → `true` and
`typeof a !== typeof a` → `false` because the two sub-expressions are
structurally identical. Implementing that requires a *structural
equality* check between operands, which is conceptually distinct from
value-substitution folding. Filed as gap-029 for a future PR.

### Port score (this crate)

|             | passing | ignored |
|-------------|---------|---------|
| CLOC12.03   | 7       | 5       |
| **CLOC12.09** | **9** | **5**   |

(Net +2 passing, 0 net change to ignored. The previously-ignored
`test_typeof_lines_from_string_string_comparison` stub got replaced
by 2 new passing tests + 1 new `#[ignore]`-d gap-029 test, so total
test count went 12 → 14.)

### Version bump

`0.4.0` → `0.5.0`.

## [0.4.0] - 2026-05-31

### Added — CLOC12.03: close gap-007 and gap-008

Two small fold-pass body extensions in `try_fold_binary_op`, each
~15 lines, sitting after the existing per-type branches:

**gap-007 — `NullLiteral OP NullLiteral`.** New branch returns the
JS-spec result for every comparison operator on two `null` literals:

```text
null ==  null   →  true
null === null   →  true
null !=  null   →  false
null !== null   →  false
null <   null   →  false   (both coerce to 0; 0 < 0 is false)
null >   null   →  false
null <=  null   →  true
null >=  null   →  true
```

Relational operators run through ECMAScript §IsLessThan, which
calls ToNumber on each side. `ToNumber(null)` is `0`, so the four
relational cases reduce to `0 OP 0`.

**gap-008 — cross-type strict equality.** New branch handles
`StrictEq`/`StrictNotEq` when both operands are literals of
*different* JS types. Per ECMAScript §IsStrictlyEqual, `===` is
`false` for any pair of values with different types, and `!==` is
`true`. So:

```text
1 === "1"          →  false
1 !== "1"          →  true
true === 1         →  false
true !== 1         →  true
null === 0         →  false
"a" === true       →  false
```

This branch fires *after* the same-type branches (numeric/numeric,
string/string, boolean/boolean, null/null), so the only cases left
to handle are literals of recognised-but-different JS types. Loose
`==` is still left alone — that goes through the abstract-equality
algorithm and stays gated by gap-003 / gap-004.

A new internal helper `js_literal_type(&Expression) → Option<&'static str>`
tags each Phase 1 primitive literal with a string discriminator
(`"number"`, `"string"`, `"boolean"`, `"null"`). The tags are
internal — they're not the result of the JS `typeof` operator
(which has its own quirks like `typeof null === "object"`) — but
they're sufficient to decide whether two literals have the same JS
type for the strict-equality fold.

### Test impact

`tests/upstream/peephole_fold_constants_test.rs`:

- `test_null_comparison_1_self_relations` was `#[ignore]`-ed in
  CLOC12.02 with `gap-007` — now passes.
- `test_number_string_strict_equality_lines` was `#[ignore]`-ed in
  CLOC12.02 with `gap-008` — now passes.

Total port score:

|             | passing | ignored |
|-------------|---------|---------|
| CLOC12.02   | 5       | 7       |
| **CLOC12.03** | **7** | **5**   |

`code/specs/CLOC12-gaps.md` updated: `gap-007` and `gap-008` marked
`RESOLVED-in-#NNNN` (PR number filled in once we know it).

### Version bump

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-05-31

### Added — CLOC12.02: first port of upstream `PeepholeFoldConstantsTest`

This is the **first** ported file under the CLOC12 byte-identical
contract. Establishes the per-crate `tests/upstream/` layout:

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5, lists ported files with upstream paths and blob SHAs.
- `tests/upstream/peephole_fold_constants_test.rs` — ports a subset
  of upstream's `PeepholeFoldConstantsTest`:
  - `test_null_comparison_1_self_relations` — `null OP null` for
    `==`, `===`, `!=`, `!==`, `<`, `>`, `>=`, `<=`.
    `#[ignore = "blocked on gap-007"]` (the fold pass has no
    `NullLiteral`/`NullLiteral` branch yet — small, self-contained
    fix).
  - `test_number_number_comparison_literal_lines` — literal-only
    arithmetic comparisons. **Passes today.**
  - `test_string_string_comparison_literal_lines` — literal-only
    string comparisons across `<`, `<=`, `>`, `>=`, `==`, `!=`,
    `===`, `!==`. **Passes today.**
  - `test_number_string_strict_equality_lines` — strict equality
    between Number and String is `false` regardless of values.
    `#[ignore = "blocked on gap-008"]` — the pass falls through its
    same-type branches and returns the binary expression unchanged.
    Trivial small fix queued.
  - `test_basic_number_comparisons` — sanity check of the
    same-type-numeric comparison happy path. **Passes today.**
  - `test_basic_arithmetic_folds` — `2 + 3 = 5`, `"a" + "b" = "ab"`,
    `"x" + 1 = "x1"`, `5 * 4 = 20`, `10 / 2 = 5`, `7 % 3 = 1`,
    `2 ** 8 = 256`. **Passes today.**
  - `test_same_when_either_side_has_an_identifier_subset` —
    `testSame`-style asserts that identifier-bearing comparisons are
    left alone. **Passes today.**
  - `test_undefined_comparison_1` — `#[ignore = "blocked on gap-001"]`.
  - `test_undefined_comparison_2` — `#[ignore = "blocked on gap-002"]`.
  - `test_null_comparison_1_loose_against_other_types` —
    `#[ignore = "blocked on gap-003"]`.
  - `test_number_string_comparison_literal_lines` —
    `#[ignore = "blocked on gap-004"]`.
  - `test_typeof_lines_from_string_string_comparison` —
    `#[ignore = "blocked on gap-005"]`.

Each ignored test cites a `gap-NNN` entry in
`code/specs/CLOC12-gaps.md` describing what's blocked and what
unblocks it. Running `cargo test -- --include-ignored` exercises
the ignored ports too; the gap count is the measurable progress
metric for byte-identical convergence.

### Test scaffolding

The ported file does not depend on a source-string parser bridge
(no such bridge exists yet — `javascript-parser::parse_javascript`
returns the generic `GrammarASTNode`, not our typed `Program`).
Instead, the file constructs typed-AST inputs by hand using the
same literal builders as `closure-pass-constant-fold`'s own inline
tests:

```rust
let input  = b(n(2.0), BinaryOperator::Add, n(3.0));
let expect = n(5.0);
assert_fold(input, expect);
```

When the parser bridge lands (a future CLOC11.* slice), we can
re-port these tests to take the upstream `test("2 + 3", "5")`
source-string form verbatim. Until then, every port both records
the upstream `test(...)` line in a doc-comment and asserts the
same byte output via constructed AST.

### Cargo wiring

Added explicit `[[test]]` entry in `Cargo.toml` pointing at
`tests/upstream/peephole_fold_constants_test.rs` because Cargo's
auto-discovery only picks up `tests/*.rs` one level deep. CLOC12.01
§3 specifies the `tests/upstream/` layout; this is the small price
for keeping ports physically grouped.

### Version bump

`0.2.0` → `0.3.0`.

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body (first non-identity optimization)

Now that `javascript-ast` Phase 1 (CLOC09) is in main, `constant-fold` becomes the first pass that does real work. The `Pass::run` body is a recursive bottom-up walker over `Program → ProgramItem → Statement → Expression` that collapses every compile-time-evaluable subexpression:

**Arithmetic on `NumericLiteral` pairs** — `+`, `-`, `*`, `/`, `%`, `**`. `2 + 3 → 5`, `5 ** 8 → 390625`, etc.

**String concatenation and mixed-type coercion for `+`** — `"foo" + "bar" → "foobar"`, `"x" + 1 → "x1"`, `2 + "x" → "2x"`. Per ECMAScript, if either operand is a string then `+` is concatenation. Numbers stringify via the JS `String(n)` convention (`42` not `42.0`).

**Comparison** — `==`, `!=`, `===`, `!==`, `<`, `<=`, `>`, `>=` on matching literal types (number/number, string/string, boolean/boolean). Mixed-type loose equality (`1 == "1"`) is **not** folded — sound default until we have an explicit toggle.

**Logical short-circuit** — `false && X → false`, `true && X → X`, `true || X → true`, `false || X → X`, `null ?? X → X`, `0 ?? X → 0`. Folds when the LEFT side is a literal; right side may be any expression (including identifiers/calls that would have evaluation side effects — we elide them because the JS short-circuit semantics say they wouldn't have run).

**Unary** — `!` on any literal (numeric → boolean via truthiness; string → boolean via length; null → true; boolean → flipped), `-` on numeric, `+` on numeric / boolean / null / parseable-numeric-string.

**Conditional (ternary)** — `true ? a : b → a`, `0 ? a : b → b`. Test must be a literal we can judge for truthiness.

**Recursion** — the walker descends through every Phase 1 node type: `Statement` (including `IfStatement.test`, `WhileStatement.test`, `ForStatement.{init,test,update,body}`, `ReturnStatement.argument`, `BlockStatement.body`), `Declaration` (including `VariableDeclarator.init` and `FunctionDeclaration.body`), and every `Expression` (`AssignmentExpression.right`, `CallExpression.{callee,arguments}`, `MemberExpression.{object,property}`, `ArrayExpression.elements`, `ObjectExpression.properties`). So `1 + (2 * 3) → 1 + 6 → 7` happens in a single bottom-up pass.

### CV tracing — both modes work

Per the CLOC09 amendment:
- **Traced input** (`cv: Some(parent)`) → folded replacement gets a new id via `CVLog::derive(parent, None)`, and a `Contribution { source: "constant-fold", tag: "folded", meta: {before, after, parent_cv, new_cv} }` is appended.
- **Untraced input** (`cv: None`) → folded replacement also has `cv: None`, **no** contribution is emitted. The `changed: true` flag is still set so the pipeline knows something happened.

Both modes verified by separate tests (`fold_in_untraced_mode_skips_cv_and_contributions`).

### Skipped (intentionally) for v0.2.0 — queued for v0.3.0+
- `typeof`, `void` — need an undefined-literal node (Phase 1 doesn't have one).
- `delete` — has observable side effects.
- Bitwise (`&`, `|`, `^`, `<<`, `>>`, `>>>`) — needs int32 coercion semantics; queued for v0.3.0 once test fixtures drive demand.
- Mixed-type loose equality — sound default; opt-in toggle planned.
- `AssignmentExpression`, `CallExpression`, `MemberExpression`, etc. — recursed-through but not collapsed (require runtime knowledge / have side effects).

### Tests
27 tests covering: pass metadata (unchanged from v0.1.0), empty-program identity (still produces no contributions), each arithmetic operator, each comparison operator, string concatenation in both directions, every unary operator we support, every logical operator with both left-wins and right-wins paths, conditional with both truthy and falsy tests, **nested folding in a single bottom-up pass** (`1 + (2 * 3) → 7` with 2 contributions emitted), unfoldable expressions pass through unchanged with `changed: false`, mixed-type loose equality is preserved, **untraced mode** (cv: None) produces no contributions but still folds, recursion through `VariableDeclarator.init` and `IfStatement.test/consequent/alternate`, pipeline integration.

### Notes
- The implementation is split into module-internal helpers (`fold_program`, `fold_statement`, `fold_expression`, `fold_binary`, `fold_logical`, `fold_unary`, `fold_conditional`) plus a `FoldState` struct that threads CV log + accumulators through the walk.
- `try_fold_binary_op` is a pure function (no I/O, no CV) that returns `Option<FoldedLiteral>` — separated from the IO-touching wrapper so the fold *semantics* are testable independently of CV bookkeeping in future tests.
- `format_js_number(n)` renders numbers the way JS's `String(x)` does (`42` not `42.0`, `0.5` not `.5`, `NaN`/`Infinity` literal-cased) so `"x" + 1 === "x1"` not `"x1.0"`.
- The `lit_label` / `literal_label` / `op_label` family produces human-readable strings for the `before` / `after` fields of the emitted `Contribution.meta` — useful for debugging via the CV log.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 — first concrete optimization pass plugged into the `closure-pass-pipeline` harness.
- `ConstantFoldPass` zero-sized type implementing `Pass`:
  - `name = "constant-fold"`
  - `iteration_policy = IterationPolicy::FixedPoint` (folds expose further folds; full multi-iteration loop arrives when the pipeline grows past v0.1.0)
  - `cost = 2` pass-units (tree walk + small constant work per visit)
  - `depends_on()` / `invalidates()` empty in v1
- `ConstantFoldPass::new()` zero-arg constructor for ergonomic `PassPipeline::add(Box::new(ConstantFoldPass::new()))` registration.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to fold. The pass clones the input `Program` unchanged, returns `changed = false`, `nodes_touched = 1`, no contributions (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy` is FixedPoint, `cost` is 2, `depends_on`/`invalidates` empty, run on empty Program is identity (program unchanged, no contributions, stats correct), full `PassPipeline` integration as solo pass (verifies FixedPoint note diagnostic flows through), pipeline integration alongside an unrelated upstream pass (registration order preserved), pass is `Default` + `Clone`.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline` (Pass trait + types), `coding-adventures-javascript-ast` (Program), `coding-adventures-type-sidecar` (future type-aware fold safety), `coding_adventures_correlation_vector` (Contribution plumbing), `serde_json` (meta JSON values). Dev-dep: `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- v1 is scaffolding. Real folding (number/string/boolean/typeof/negation/comparison/conditional) lands once `javascript-ast` grows `Statement` / `Expression` variants — at that point this file becomes a real pass without any API churn upstream.

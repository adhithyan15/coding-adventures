# CLOC22 — `for`-`in` end-to-end

> **Status:** Shipped. AST node, parser bridge, emitter, scope analyzer, and
> every optimization pass handle the `for`-`in` loop. An end-to-end diff fixture
> (`simple-for-in`) pins the behaviour at the CLI.

## Why this spec exists

`for (left in right) body` enumerates the property keys of `right`. It was a
Phase-2 statement gap. The grammar already *parsed* it, but the typed AST had no
node to represent it, so the parser→typed-AST **bridge** declined
(`UnsupportedSyntax`) and the CLI fell back to **`WHITESPACE_ONLY`** — applying
zero real optimization to *any* program with a for-in loop (a very common
construct). CLOC22 closes that gap with the established playbook: make the
statement representable, then recurse every pass into it.

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program (ForInStatement)
       ──passes──▶ optimized Program ──emit──▶ JS text
```

## The AST node (`coding-adventures-javascript-ast`)

```rust
pub struct ForInStatement {
    pub cv: Option<CvId>,
    pub left: ForInit,        // VariableDeclaration | Expression
    pub right: Expression,
    pub body: Box<Statement>,
}
```

The `left` reuses [`ForInit`] (the for-loop init type), which is a perfect
structural match:

- `ForInit::VariableDeclaration` for `for (var k in o)` / `for (let k in o)` /
  `for (const k in o)` — a single-declarator binding with no initializer.
- `ForInit::Expression` for `for (k in o)` / `for (o.p in src)` — an existing
  assignment target.

Destructuring left-hand sides are **not** represented; the bridge declines them
(see below).

## The bridge (`coding-adventures-javascript-parser`)

The grammar production is:

```text
for_in_statement = "for" "(" ( "var" variable_declaration
                              | "let" binding_element
                              | "const" binding_element
                              | left_hand_side_expression ) "in" expression ")" statement
```

`convert_for_in_statement` walks the children using the `in` and `)` tokens as
phase delimiters (phase 0 = left, phase 1 = right expression, phase 2 = body)
and detects the `var`/`let`/`const` keyword (a phase-0 token) to set the binding
kind. The left binding is converted with the shared `convert_variable_declarator`
(which already declines destructuring `binding_pattern`). **Soundness guard:**
any binding shape the declarator converter can't represent is mapped to a
graceful `UnsupportedSyntax` decline rather than a hard error, so an
unrepresentable for-in left never aborts compilation — it falls back to
whitespace-only, which is sound.

All four left forms are covered end-to-end (`var`/`let`/`const` and a
left-hand-side expression); destructuring declines.

## The emitter (`coding-adventures-closure-emitter`)

`emit_for_in` writes `for ( <left> in <right> ) <body>`. The `in` keyword is
separated on **both** sides with `required_ws`: the left ends in an identifier
(`var k` / `k` / `o.p`) and the right starts with one, so `kin` / `inobj` would
mis-lex. In the rare `a[b] in` / `in (x)` cases the space is one redundant byte
but never wrong, matching upstream Closure's spacing around `in`.

## Soundness: the loop variable is a binding

The crux is that `for (var/let/const k in o)` declares the loop variable `k` — a
binding the renaming passes must handle exactly like a for-loop init binding:

- **`rename` / `rename-globals`**: the `left` declaration is recorded as a rename
  occurrence (block-scoped to the loop) and the declared name is rewritten via
  the rename map, so the binding **and its uses inside the body** rename
  *consistently*. The expression-left form (`for (k in o)`) has its assignment
  target rewritten as a use.
- **`inline` / `inline-variables`**: the `left` declaration is counted as a
  binding in the shadow-guard tallies.

This is verified end-to-end: `for (var element in collection)` with
`collection[element]` ⟶ `for (var c in a)` with `a[c]` under ADVANCED.

Like the other loops, a `for`-`in` is **not** a terminator — the body may run
zero times (an object with no enumerable keys), so control can fall through and
statements after it stay reachable.

### Per-pass handling

| Pass | What it does for `ForInStatement` |
|------|------------------------------------|
| `constant-fold` / `fold-control-flow` | recurse fold into left / right / body (never elide) |
| `dce` | recurse DCE into left / right / body; not a terminator |
| `inline-variables` / `inline` | recurse; count the `left` declaration as a binding |
| `rename` / `rename-globals` | recurse; rename the loop variable consistently |
| `rename-properties` | recurse classify/rewrite into left / right / body |

The scope analyzer (`closure-scope-analyzer`) walks the left (binding or target),
the right, then the body.

## End-to-end oracle (`closurec` diff fixture)

* **`simple-for-in`** — at SIMPLE: arithmetic inside the for-in body folds, the
  loop survives verbatim, and the statement after the loop stays reachable.
  `function log` is KEPT — SIMPLE is open-world and never inlines or removes a
  top-level name (that inline is ADVANCED-only). A companion assertion proves
  the output is NOT the whitespace fallback (the `1 + 2` ⇒ `3` fold in the loop
  body can only come from the typed pipeline).

## Out of scope (future work)

* Destructuring for-in left-hand sides (`for (var [a] in o)`) — currently decline
  to WHITESPACE_ONLY.
* `ForOfStatement` (`for (x of iter)`) and `WithStatement` remain the last
  bridge-unsupported Phase-2 statements. `for`-`of` is structurally almost
  identical to `for`-`in` and is the natural next item.

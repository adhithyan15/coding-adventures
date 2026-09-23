# CLOC19 — `try` / `catch` / `finally` end-to-end

> **Status:** Shipped. AST node, parser bridge, emitter, scope analyzer, and
> every optimization pass handle `try`/`catch`/`finally`. Two end-to-end diff
> fixtures (`simple-try-catch`, `advanced-try-catch-rename`) pin the behaviour at
> the CLI.

## Why this spec exists

`try`/`catch`/`finally` was closurec's single largest correctness/coverage gap.
The grammar already *parsed* try statements, but the typed AST had no node to
represent them, so the parser→typed-AST **bridge** declined (`UnsupportedSyntax`)
and the CLI fell back to **`WHITESPACE_ONLY`**: it emitted the source with only
inter-token whitespace stripped and applied **zero** real optimization. Any
real-world JavaScript bundle uses `try`/`catch`, so this fallback meant closurec
effectively no-op'd on most realistic inputs.

CLOC19 closes that gap by making try/catch a first-class statement that flows
through the entire pipeline:

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program (TryStatement)
       ──passes──▶ optimized Program ──emit──▶ JS text
```

## The AST node (`coding-adventures-javascript-ast`)

ESTree-shaped, mirroring the structure every JS tool expects:

```rust
pub struct TryStatement {
    pub cv: Option<CvId>,
    pub block: BlockStatement,            // the protected block
    pub handler: Option<CatchClause>,     // the `catch` arm (optional)
    pub finalizer: Option<BlockStatement>,// the `finally` block (optional)
}

pub struct CatchClause {
    pub cv: Option<CvId>,
    pub param: Option<Identifier>,        // None = ES2019 `catch { … }`
    pub body: BlockStatement,
}
```

A new `TaggedStatement::TryStatement` variant and a `Statement::try_statement`
constructor expose it. The grammar guarantees at least one of `handler` /
`finalizer` is present (a bare `try {}` is a SyntaxError), but the AST does not
enforce that — it models what the parser produces.

**Serde note (a bug fixed during implementation):** `TryStatement` must NOT carry
its own `#[serde(tag = "type")]`. It is a variant of the internally-tagged
`TaggedStatement` enum, which already injects `"type": "TryStatement"` from the
variant name. A second struct-level tag double-tags the node and breaks
deserialization back into the untagged outer `Statement` enum. Every sibling
statement struct carries only `rename_all`; `TryStatement` follows suit.
`CatchClause` (a nested struct, not an enum variant) keeps its own
`tag = "type"`, exactly like `SwitchCase`.

## The bridge (`coding-adventures-javascript-parser`)

`try_statement` routes to `convert_try_statement`, which takes the first node
child as the `block` and walks the remaining children for a `catch_clause` /
`finally_clause`. `convert_catch_clause` reads the single `NAME` token as the
catch binding (`param`), or `None` for the optional-catch-binding form.

The grammar restricts the catch binding to a simple `NAME`, so a **destructuring**
catch param (`catch ({ message }) { … }`) cannot parse/bridge into a
`TryStatement` — it declines, which surfaces as `WHITESPACE_ONLY` at the CLI. This
is the sound choice: a destructuring binding must never be silently lowered to a
fabricated simple identifier.

## The emitter (`coding-adventures-closure-emitter`)

`emit_try` writes `try <block> [ catch [(param)] <block> ] [ finally <block> ]`.
No inter-token separator (`required_ws`) is needed anywhere: every boundary is
keyword↔`{`/`}` or `}`↔keyword (`try{…}catch{…}`, `}finally{…}`), all of which
lex cleanly with no space. Pretty mode adds readability spaces; the
optional-catch-binding form emits `catch{…}` with no parens.

## The catch-param rule

The catch parameter is a binding scoped to the handler body and **nowhere else**.
Every pass that renames or removes bindings must treat it as such. Concretely:

1. **We never rename it.** Renaming passes collect the catch param as an
   *ineligible* declaration occurrence — it stays in the output verbatim.
   **This is a conservative choice, not a soundness requirement** — see below.
2. **Nothing is renamed onto it.** The catch param joins the fresh-name **avoid
   set**, so no generated short name can collide with it. *This one is a
   soundness requirement.*
3. **It is counted as a declared binding** in every `count_decl_names_*` /
   shadow-guard tally, so a free identifier elsewhere that is also bound by a
   catch clause is correctly treated as shadowed (CLOC16 linchpin).
4. **A `try` is not a terminator.** It can catch and continue, so DCE/control-flow
   passes keep statements *after* a try/catch reachable.

If (2) is missing, a generated short name can alias the caught value and
miscompile the handler. The regression test that pins it is
`fresh_name_avoids_catch_param_unused_in_its_own_body`: the handler is
`catch (a) { use(longName); }`, which never mentions its own binding, so only
the explicit avoid-set insertion can keep `longName` off `a`.

Note which test that is. `fresh_name_avoids_colliding_with_catch_param` reads as
though it pins the same guard and does not — its handler is `use(a, longName)`,
so `a` reaches the avoid set through the body walk regardless, and deleting
`out.insert(param.name)` left the whole crate green. The upstream port
`fresh_name_avoids_catch_binding` shares the blind spot. Both are still useful
as end-to-end cover; neither discriminates the guard.

### (1) is ours, not a law — corrected 2026-09-23

An earlier revision of this section listed (1) and (2) together as things every
pass **MUST** do, and concluded "if either of (1)/(2) is missing, a generated
short name can alias the caught value and miscompile the handler". That is true
of (2) and false of (1), and upstream Closure is the counterexample. Measured
against the pinned oracle (`closure-compiler-v20260915`, sha256 verified),
with externs for the free globals and a value use to stop the inliner:

```js
function process(value) {
  var temp = value + 1;
  try { compute(temp); } catch (err) { report(err, temp); }
  return temp;
}
sink(process);
```

```
SIMPLE   : function process(a){a+=1;try{compute(a)}catch(b){report(b,a)}return a}sink(process);
ADVANCED : sink(function(a){a+=1;try{compute(a)}catch(b){report(b,a)}return a});
```

`err` → `b`. Upstream renames catch parameters at **both** levels. Two further
probes show why that is safe, and they are the reason (1) and (2) are not the
same rule:

```
in            : function f(a1,a2,a3){var x=a1+a2+a3;try{compute(x)}catch(err){report(err,x,a1,a2,a3)}return x}sink(f);
out (SIMPLE)  : function f(b,c,d){var a=b+c+d;try{compute(a)}catch(e){report(e,a,b,c,d)}return a}sink(f);
out (ADVANCED): sink(function(b,c,d){var a=b+c+d;try{compute(a)}catch(e){report(e,a,b,c,d)}return a});
```

With `a` through `d` already taken, upstream gives the catch binding `e`. It
satisfies (2) by **choosing a non-colliding fresh name**, which is a different
mechanism from reserving the original one. And renaming stays correct across
shadowing:

```
in            : var err=1;function f(v){try{compute(v)}catch(err){report(err)}return err}sink(f);
out (SIMPLE)  : var err=1;function f(a){try{compute(a)}catch(b){report(b)}return err}sink(f);
out (ADVANCED): sink(function(a){try{compute(a)}catch(b){report(b)}return 1});
```

The inner binding becomes `b` while the outer `err` reference keeps its own
identity — at SIMPLE it survives verbatim, and at ADVANCED it is constant-folded
to `1`, which is the same fact seen through one more pass.

So reserving the catch param is a sound way to satisfy (2), and it is the one we
implement, but it is not the only one and it is not required.

Two limits on how far this evidence reaches, since "upstream does it" is not the
same as "it is always safe":

* **`eval` in the handler defeats any rename, and upstream renames anyway.**
  `function f(v){try{compute(v)}catch(err){eval("report(err)")}return v}sink(f);`
  compiles at both levels to `…catch(b){eval("report(err)")}…` — the string still
  names `err`, which no longer exists. Upstream ships that miscompile by policy,
  the same way it does for any renamed binding an `eval` string reaches. So the
  probes show renaming is not *required* to be avoided; they do not show it is
  safe in the presence of `eval`, and neither does our reserving rule make us
  safe there for any other local.
* **Upstream refuses two of the hard cases rather than renaming them.**
  `with (o) { report(err) }` is `JSC_USE_OF_WITH`, and a handler that
  redeclares its binding (`catch (err) { var err = err + 1; }`) is
  `JSC_REDECLARED_VARIABLE_ERROR`, at both levels. Its evidence therefore covers
  only the subset of JavaScript it accepts. It has a cost:
upstream emits shorter output than we do wherever a catch binding has a long
name, which is part of **CCR-022**
([#15856](https://github.com/adhithyan15/coding-adventures/issues/15856)).
That issue currently frames catch params as a binding kind our renamer *skips* —
incompleteness. The probes above say it is a **divergence**, and at SIMPLE as
well as ADVANCED. Anyone picking up CCR-022 should expect to change this rule
and the `advanced-try-catch-rename` golden together, not just add a code path.

### Per-pass handling

| Pass                          | What it does for `TryStatement` |
|-------------------------------|----------------------------------|
| `constant-fold`               | recurse fold into block/handler.body/finalizer |
| `fold-control-flow`           | recurse fold into the three blocks |
| `dce`                         | recurse DCE; dead-after-terminator inside blocks; `try` is not a terminator |
| `inline-variables`            | recurse; count catch param as a binding (shadow guard) |
| `inline`                      | recurse all phases; count catch param (CLOC16 shadow-guard); add to used-idents avoid set |
| `rename`                      | recurse; catch param reserved (ineligible + avoid set) |
| `rename-globals`              | recurse; catch param reserved (counted + avoid set) |
| `rename-properties`           | recurse only — properties ≠ variable bindings, so the catch param is irrelevant |

The scope analyzer (`closure-scope-analyzer`) emits a `ScopeKind::Block` scope for
the handler and a `BindingKind::Let` binding for the catch param.

## End-to-end oracles (`closurec` diff fixtures)

* **`simple-try-catch`** — at SIMPLE: arithmetic inside the try/catch blocks
  folds, dead-code after a `return` in the catch is dropped, and
  `try`/`catch (e)`/`finally` survive verbatim. `function log` is KEPT — SIMPLE
  is open-world and never inlines or removes a top-level name (that inline is
  ADVANCED-only). A companion assertion proves the output is NOT the whitespace
  fallback (the unreachable `dead(99)` after `return` is dropped by DCE, which
  only the typed pipeline runs).
* **`advanced-try-catch-rename`** — at ADVANCED: `process`/`value`/`temp` get
  short names, uses inside both the try block and the catch body are rewritten,
  and the catch binding `err` is preserved verbatim and never aliased to a
  generated name. Note this fixture pins **our** behaviour, not upstream's:
  upstream renames the catch binding (see "(1) is ours, not a law" above) and
  in fact inlines this whole function away. It is a regression test for the
  conservative rule we chose, not an oracle for it.

## Out of scope (future work)

* Destructuring catch params (`catch ({ message })`) — currently decline to
  `WHITESPACE_ONLY`; modelling them needs a `BindingTarget` catch param.
* Optimizations that *reason about* exceptional control flow (e.g. proving a
  `try` block can't throw and unwrapping it) — CLOC19 only makes try/catch
  *transparent* to the existing passes; it does not add try-specific rewrites.

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

## The catch-param soundness rule (the crux)

The catch parameter is a binding scoped to the handler body and **nowhere else**.
Every pass that renames or removes bindings MUST treat it as such. Concretely:

1. **It is never renamed.** Renaming passes collect the catch param as an
   *ineligible* declaration occurrence — it stays in the output verbatim.
2. **Nothing is renamed onto it.** The catch param joins the fresh-name **avoid
   set**, so no generated short name can collide with it.
3. **It is counted as a declared binding** in every `count_decl_names_*` /
   shadow-guard tally, so a free identifier elsewhere that is also bound by a
   catch clause is correctly treated as shadowed (CLOC16 linchpin).
4. **A `try` is not a terminator.** It can catch and continue, so DCE/control-flow
   passes keep statements *after* a try/catch reachable.

If either of (1)/(2) is missing, a generated short name can alias the caught
value and miscompile the handler. The regression test
`fresh_name_avoids_colliding_with_catch_param` pins the killer case: with a catch
param literally named `a`, the function's own param renames to `b`, not `a`.

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
  generated name.

## Out of scope (future work)

* Destructuring catch params (`catch ({ message })`) — currently decline to
  `WHITESPACE_ONLY`; modelling them needs a `BindingTarget` catch param.
* Optimizations that *reason about* exceptional control flow (e.g. proving a
  `try` block can't throw and unwrapping it) — CLOC19 only makes try/catch
  *transparent* to the existing passes; it does not add try-specific rewrites.

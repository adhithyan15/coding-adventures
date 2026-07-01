# sir-keyword-params — named keyword parameters & arguments for SIR

## Status

New. This spec **precedes implementation** per the repo specs-first rule
because its first milestone is a `semantic-ir` core schema change.

It closes the **single critical Ruby-1.0 blocker** surfaced by the
2026-06-30 Ruby readiness survey: `def f(x:)` / `def f(x: 1)` (keyword
parameters) and `f(x: 1)` (keyword arguments) are core Ruby 2.0+ syntax that
the pipeline cannot represent at all today. It is also the natural follow-on
explicitly deferred by [`sir-variadic-params.md`](sir-variadic-params.md)
("Out of scope: Required-keyword params (`def f(a:)`) and optional-with-default
params") and the default-parameter work.

It is the next cascade under the project's **full-syntax mandate** (support ANY
source syntax via direct-lowering, or a backend runtime library where no native
form exists). As shown in §4, every backend here can lower keyword params
**directly** — no runtime library is required.

## Vocabulary — three parameter concepts, kept distinct

Ruby (and Python) have three superficially-similar `def`-side constructs. SIR
already models two; this spec adds the third. Keeping them distinct is the
whole game:

| Source                | Meaning                                   | SIR model |
|-----------------------|-------------------------------------------|-----------|
| `def f(a)`            | positional required                       | `ParamKind::Required` (exists) |
| `def f(a = 1)`        | positional **optional** (default)         | `ParamKind::Required` + `default: Some` (exists) |
| `def f(*rest)`        | positional **rest** (collects positionals)| `ParamKind::Rest` (exists) |
| `def f(**opts)`       | keyword **rest** (collects keywords)      | `ParamKind::KwRest` (exists) |
| **`def f(a:)`**       | **keyword required** (matched by name)    | **`ParamKind::Keyword` + `default: None` (NEW)** |
| **`def f(a: 1)`**     | **keyword optional** (name + default)     | **`ParamKind::Keyword` + `default: Some` (NEW)** |

A *keyword* parameter is bound by **name** at the call site, not by position.
`def f(a: 1)` and `def f(a = 1)` look alike but are not: the former is called
`f(a: 9)` (or `f()`), the latter `f(9)` (or `f()`). The distinguishing axis is
therefore `ParamKind`, and required-vs-optional rides on the **existing**
`default` field — exactly as it already does for positional optionals. No new
"is-required" flag is introduced.

## Core schema change (milestone KW1 — `semantic-ir`)

### 1. `ParamKind` gains `Keyword`

```rust
pub enum ParamKind {
    #[default]
    Required,   // positional  (exists)
    Rest,       // *rest        (exists)
    KwRest,     // **opts       (exists)
    Keyword,    // a:  /  a: default   (NEW — matched by name)
}
```

`Keyword` + `default == None`  → a **required** keyword param (`def f(a:)`).
`Keyword` + `default == Some(e)` → an **optional** keyword param (`def f(a: e)`).

Because `ParamKind` is `Copy` with a `#[default]` of `Required`, no existing
`Param { … }` construction changes — only code that *matches* on `ParamKind`
(printer, backends, validator) gains a `Keyword` arm.

### 2. Call sites carry keyword arguments via a new `Expr::KeywordArg`

Call arguments are `args: Vec<Expr>` on `DirectCall` / `IndirectCall` /
`MakeClosure`. Rather than add a parallel `kwargs` field to **every** call
variant — a wide, multi-site core-field change that is precisely the cross-PR
struct-field merge hazard we have already been bitten by — a keyword argument
is modelled as a **wrapper expression that appears inside the existing `args`
vector**:

```rust
pub enum Expr {
    // … existing variants …
    /// A keyword argument at a call site: `name: value` (Ruby `f(a: 1)`,
    /// Python `f(a=1)`).  Appears ONLY inside a call's `args` vec, and only
    /// AFTER all positional arguments.  The validator enforces both rules.
    KeywordArg {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
}
```

Rationale:

- **Minimal blast radius.** One new `Expr` variant; zero changes to the call
  structs' fields, so no existing call *construction site* needs editing. This
  keeps KW1 a small, self-contained core PR that backends/frontends rebase on
  top of (see §6 sequencing).
- **Positional args stay bare.** `f(1, a: 2)` lowers to
  `args: [IntLit(1), KeywordArg{ name:"a", value: IntLit(2) }]`. A backend that
  ignores keywords still sees the positionals in order.
- **Walker/printer already recurse `Expr`.** The new variant slots into the
  existing traversal with one arm each.

### 3. `Feature::KeywordParams`

```rust
pub enum Feature { /* … */ KeywordParams }
```

Set in the manifest when **any** `Param.kind == Keyword` **or** any call `args`
element is a `KeywordArg`. The validator rejects the feature's constructs when
the manifest does not declare it (same contract as `DefaultParams`).

## Validator rules (milestone KW1)

1. **Def-side ordering.** Within a parameter list the order is:
   positional (`Required`, incl. positional-optional) → `Rest` → `Keyword`* →
   `KwRest`. A `Keyword` param before a positional one, or after `KwRest`, is
   rejected. (Ruby's own order; keeps lowering unambiguous.)
2. **At most one `KwRest`** (already implied by variadic spec) and it is last.
3. **Call-side ordering.** In a call's `args`, every `KeywordArg` follows all
   positional (non-`KeywordArg`) elements. A positional after a keyword is
   rejected.
4. **No duplicate keyword names** in one call's `args`.
5. **DirectCall name resolution (known callee).** For a `DirectCall` whose
   target function is in the module, every `KeywordArg.name` must match a
   `Keyword` param of the callee **or** the callee must declare a `KwRest`
   (which absorbs unmatched keywords). Every **required** keyword param
   (`Keyword`, `default: None`) of the callee must be supplied by a matching
   `KeywordArg`. Optionals may be omitted (the backend fills the default).
   `IndirectCall` / closure calls skip name resolution (callee signature not
   statically known) — v0 passes keywords through positionally-by-name to the
   backend, which handles them structurally (see §4 TS/JS).
6. **`KeywordArg` only in call position.** A `KeywordArg` anywhere other than a
   call's `args` vec is rejected (it is not a first-class value).

Helper (mirrors `missing_defaults`): `Function::keyword_params()` returns the
`Keyword` params, and `missing_keywords(supplied: &[&str])` returns the
`Keyword` params whose name is not in `supplied` — every one is guaranteed to
carry a `default` for a call the validator accepted, so a backend emits its
default unconditionally.

## Backend emission (milestones KW2–KW6) — all direct, no runtime library

| Backend | Def-side `Keyword` param | Call-side `KeywordArg` |
|---|---|---|
| **Python** (`…-to-python`) | native keyword-only: emit a bare `*` separator once, then `name` (required) / `name=default` (optional): `def f(x, *, y, z=1)` | native `name=value`: `f(1, y=2)` |
| **Ruby** *(frontend, not a backend)* | — | — |
| **TypeScript** (`…-to-typescript`) | trailing **options object**: `function f(x, __kw){ const { y, z = <default> } = __kw ?? {}; … }` | `f(1, { y: 2 })` |
| **JavaScript** (`…-to-javascript`) | same options-object shape as TS | same |
| **Rust** (`…-to-rust`) | ordinary positional params in declared order (drop the by-name-ness; the name is a source affordance) | **static resolution**: map each `KeywordArg.name` → callee param position, emit an ordinary positional call in declared order, filling omitted optionals with their defaults |
| **Go** (`…-to-go`) | same as Rust | same static resolution |

Key insight making Rust/Go **direct** (no runtime lib): a `DirectCall`'s callee
signature is statically known, and `KeywordArg` names its target param, so the
backend can **resolve keyword→position at emit time** and produce a plain
positional call. Python/Ruby keep it native; TS/JS use a zero-dependency
object-destructure. Thus every backend lowers keyword params with **direct
lowering only** — satisfying the full-syntax mandate without a runtime library.

TS/JS options-object and Rust/Go positional-resolution both require the callee
param names/order; for `IndirectCall`/closures where that is unknown, v0 emits
the TS/JS options-object form (structural, needs no signature) and **documents**
that Rust/Go indirect keyword calls are deferred (the Ruby/Python frontends do
not yet emit indirect keyword calls, so this is not on the critical path).

## Frontend production (milestones KW7–KW8)

- **Ruby** (`ruby-to-semantic-ir` + `code/grammars/ruby.grammar`): extend
  `param` with a `NAME COLON [ expression ]` branch (`a:` / `a: expr`) and
  `call_arg` with `NAME COLON expression` (`f(a: 1)`). Lower `NAME COLON` params
  to `Param { kind: Keyword, default: <opt expr> }` and keyword call args to
  `Expr::KeywordArg`. Regenerate `_grammar.rs`. This is the milestone that
  finally makes modern Ruby keyword code parse — the Ruby-1.0 unblock.
- **Python** (`python-to-semantic-ir`): `def f(*, x, y=1)` (keyword-only after
  a bare `*`) → `Keyword` params; `f(x=1)` → `KeywordArg`.
- **JavaScript** (`javascript-to-semantic-ir`): JS has **no** native
  keyword-argument syntax; the JS frontend produces **no** keyword params
  (documented). JS participates only as a *backend* consumer (options-object).

## Verification (per milestone)

- **Core (KW1):** validator unit tests for each rule (ordering both sides,
  duplicate names, required-keyword-supplied, unknown-name-without-kwrest,
  `KeywordArg` out of call position); printer round-trip for `a:` / `a: expr`
  and a `name: value` call arg; `Feature::KeywordParams` gating test.
- **Backends (KW2–KW6):** emitted-shape tests + **execution-proof** through the
  native toolchain (skip gracefully if absent): a program
  `def greet(greeting:, name: "world"); "#{greeting}, #{name}"; end;
  greet(greeting: "hi")` must print `hi, world` through Python (`python3`),
  Node (`node`), rustc, and `go run`, and match the reference backend.
- **Frontends (KW7–KW8):** lowering-assertion tests
  (`def f(a:)` → `Param{kind:Keyword, default:None}`;
  `def f(a: 1)` → `default: Some`; `f(a: 2)` → `KeywordArg`).
- Per-crate `cargo test -p <crate>` (linker override), clippy clean,
  security-review gate before each push.

## Milestones (one PR per crate; core lands first)

| # | Crate | Content |
|---|-------|---------|
| KW0 | `code/specs/` | **this spec** (design PR, surfaced for review before code) |
| KW1 | `semantic-ir` | `ParamKind::Keyword`, `Expr::KeywordArg`, `Feature::KeywordParams`, validator rules, walker + printer, helpers |
| KW2 | `semantic-ir-to-python` | native keyword-only def + `name=value` call |
| KW3 | `semantic-ir-to-typescript` | options-object def + call |
| KW4 | `semantic-ir-to-javascript` | options-object def + call |
| KW5 | `semantic-ir-to-rust` | positional-resolution def + call |
| KW6 | `semantic-ir-to-go` | positional-resolution def + call |
| KW7 | `ruby-to-semantic-ir` (+ grammar) | `a:` param + `f(a:)` arg production → **Ruby-1.0 unblock** |
| KW8 | `python-to-semantic-ir` | `*`-keyword-only param + `f(a=1)` arg production |

**Sequencing (core-field hazard, per the cross-PR merge lesson):** KW1 merges
**first**; KW2–KW8 rebase onto the merged core. Never run a keyword-arg
*construction-site* PR (a backend/frontend) concurrently with KW1 — the merged
result, not the individually-green branch, is what must compile. After KW1
lands, the backends (KW2–KW6) are mutually disjoint crates and may run in
parallel; the frontends (KW7–KW8) likewise.

After KW7 lands, revisit Ruby-1.0 readiness and (with explicit user sign-off)
bump `ruby-to-semantic-ir` to 1.0.

## Out of scope (documented, honest)

- **Indirect keyword calls in Rust/Go** (`IndirectCall`/closure with keywords) —
  deferred; not produced by the frontends yet.
- **Keyword `**opts` forwarding interplay** beyond what `KwRest` already models
  (splatting a map into keywords, `f(**h)` on the call side) — the existing
  Q10f double-splat call treatment stands; unifying it with named keywords is a
  later refinement.
- **JS-frontend keyword production** — JS has no such syntax.

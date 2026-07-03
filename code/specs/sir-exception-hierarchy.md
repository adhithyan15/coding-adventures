# sir-exception-hierarchy — exceptions across all backends + user-defined class ancestry

## Status

New. Design/spec PR (specs-first). The next cascade under the full-syntax mandate.

**A grounding survey corrected the premise.** The initial motivation ("`rescue
ArgumentError` vs `rescue StandardError` lower identically") is **already false
for Python & TypeScript** — the IR carries the rescued types and those backends
match by a built-in class hierarchy at runtime. The real, still-open gaps are
narrower and different, enumerated below. This spec targets those.

## Current state (from the 2026-07-01 exception survey — what already works)

- **Core IR carries typed rescue.** `Stmt::TryCatch { body, rescues:
  Vec<RescueClause>, ensure_body, span }` (nodes.rs:505), and `RescueClause {
  exception_types: Vec<String>, binding: Option<String>, body, span }`
  (nodes.rs:335) — the rescued class names ARE captured. `Feature::Exceptions`
  gates it (manifest.rs:75; validator enforces declared⇒used).
- **Ruby frontend already lowers them.** `begin/rescue Foo, Bar => e/ensure/end`
  → a `TryCatch` with `exception_types: ["Foo","Bar"]`, `binding: Some("e")`
  (lower.rs:5328). `raise Foo, "m"` → `BuiltinCall("raise", [Const("Foo"),
  StrLit("m")])` with `MayThrow|Divergent` effects.
- **Python + TS backends already do hierarchy-aware matching.** Both emit a
  single native catch, then dispatch via `rescue_matches(exc, class_names)` which
  walks a hardcoded built-in ancestry (`sir-runtime-exceptions`:
  `StandardError→Exception`, `ArgumentError/TypeError/RuntimeError/…→StandardError`,
  `NoMethodError→NameError`, `KeyError→IndexError`, …). `raise` builds a
  `SirError` tagged with its class name. **So typed rescue over BUILT-IN classes
  already executes correctly in Python & TS.**

The real gaps this cascade closes:
1. **Go & Rust backends reject exceptions entirely.** `Feature::Exceptions` is not
   in their `ACCEPTED_FEATURES`; `Stmt::TryCatch`/`raise` hit `panic!` guards
   (emit.rs). Neither has ANY begin/rescue support. Go has no native exceptions
   (needs `panic`/`recover`); Rust has none (needs `catch_unwind` or a Result
   discipline) — each needs a runtime `SirError`-equivalent + `rescue_matches`.
2. **JS backend defers `TryCatch`** (`panic!("…not accepted yet")`,
   semantic-ir-to-javascript emit.rs:428). Cheapest gap — mirror the existing TS
   emission + port the `sir-runtime-exceptions` matcher to the JS runtime.
3. **User-defined exception ancestry is not threaded.** The runtime ancestry is
   hardcoded to built-ins; a user `class MyErr < StandardError` matches only by
   EXACT name, so `rescue StandardError` misses a raised `MyErr`. `Stmt::ClassDef`
   already carries `superclass: Option<String>` (nodes.rs:451) but it is not
   connected to the exception matcher.

## Design

### E1 — JavaScript backend exceptions (cheapest, mirrors TS)

Add `Stmt::TryCatch` emission to `semantic-ir-to-javascript` (mirror the TS
backend's try/catch + `rescueMatches(__exc, [...])` else-chain + `finally`), add
the `raise` builtin emission (→ a `SirError` throw), accept `Feature::Exceptions`,
and provide the runtime matcher in the JS backend's runtime (port
`sir-runtime-exceptions`'s `SirError` + `rescueMatches` + the built-in `ANCESTRY`;
reuse the TS package's logic verbatim — it is plain JS-compatible). Execution-proof
via `node`.

### E2 — user-defined exception ancestry (Python + TS + JS runtimes + emit)

Thread `ClassDef.superclass` into rescue matching so `class MyErr < StandardError;
… rescue StandardError` catches a raised `MyErr`.
- **Approach (lazy, minimal-core):** at emit time, collect the module's
  `ClassDef { name, superclass }` pairs into a user-ancestry map and emit it into
  the program's runtime init (e.g. `__SirExc.registerAncestry({"MyErr":
  "StandardError", …})`). `rescue_matches` consults the merged built-in + user
  map when walking ancestry. No core-IR change — the data already exists on
  `ClassDef`; this is backend/runtime wiring in Python, TS, and JS.
- Validation stays advisory (unknown class names remain unresolved, matching the
  existing `superclass`/`exception_types` "advisory names" contract) — do NOT add
  a symbol-table/hard validation in v0.

### E3 — Go backend exceptions (runtime + panic/recover)

Go has no exceptions. Emit `Stmt::TryCatch` as a `func(){ defer func(){ if r :=
recover(); r != nil { … rescue dispatch … } }(); <body> }()` shape; `raise` →
`panic(_sir_new_error("Class", msg))`. Add a Go runtime `SirError` struct
(class-name-tagged) + `_sir_rescue_matches(r, []string{...})` with the built-in
ancestry table + the user-ancestry registration from E2, all via an EXPLICIT
table (no reflection, per the collection-methods RCE lesson). Accept
`Feature::Exceptions`. `ensure` → a second `defer`. Execution-proof via `go run`.

### E4 — Rust backend exceptions (runtime + catch_unwind or Result)

Rust has no exceptions. v0 approach: model a thrown `SirError` value and implement
`TryCatch` via `std::panic::catch_unwind` over a closure (with a custom panic
payload carrying the `SirError`), or a Result-threading discipline — pick per what
fits the crate's `Value`/emit model (investigate first). `raise` →
`__sir::raise(class, msg)` (panics with the payload). Runtime `SirError` +
`rescue_matches` with built-in + user ancestry, explicit table. Accept
`Feature::Exceptions`. Execution-proof via rustc.

## Milestones (one PR per crate)

| # | Crate | Content | Phase |
|---|-------|---------|-------|
| E0 | `code/specs/` | this spec | design |
| E1 | `semantic-ir-to-javascript` (+ JS runtime) | TryCatch/raise emit + JS `sir-runtime-exceptions` port | 1 (cheap) |
| E2 | `semantic-ir-to-python` + `-typescript` + `-javascript` (+ their runtimes) | user-defined ancestry threading (`ClassDef.superclass` → runtime ancestry) | 1 (cheap) |
| E3 | new Go runtime-exceptions + `semantic-ir-to-go` | Go panic/recover exceptions + runtime matcher | 2 (big) |
| E4 | new Rust runtime-exceptions + `semantic-ir-to-rust` | Rust catch_unwind exceptions + runtime matcher | 2 (big) |

**No core-IR change is required** — `TryCatch`, `RescueClause.exception_types`,
`ClassDef.superclass`, and `Feature::Exceptions` all already exist. Every milestone
is a disjoint backend/runtime PR. Sequencing: E1/E2 (Phase 1) are independent
disjoint lanes; E3/E4 (Phase 2) are the large ones. Each PR: tests via linker
override, clippy clean, execution-proof through the native toolchain vs the
Python/TS reference, security-review gate (explicit ancestry tables, never
reflection), `gh pr create`.

## Verification

- **E1/E2:** emitted-shape tests + `node`/`python3` execution-proof — a
  `begin; raise ArgumentError, "x"; rescue StandardError => e; …; end` catches
  (built-in ancestry), a bare `rescue` catches all, an unmatched type re-raises,
  and (E2) a `class MyErr < StandardError; raise MyErr; rescue StandardError`
  catches. Diff stdout vs the reference.
- **E3/E4:** the same golden exception programs run through `go run`/rustc and
  match the Python/TS reference (built-in AND user-defined ancestry).
- Cross-backend parity: one golden exception+hierarchy suite runs through all 5.

## Out of scope (documented)

- A compile-time exception-class symbol table / hard validation of unknown class
  names (stays advisory, per the existing `superclass`/`exception_types` contract).
- `retry`/`else`-in-begin beyond what already lowers; custom `Exception` subclass
  method bodies beyond name+superclass ancestry.
- Non-local exception control (throw/catch tags, `Thread`/`Fiber`).

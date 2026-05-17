# Changelog — @coding-adventures/forme-style-orchestrator

## 0.1.0 — 2026-05-17

Initial release.  Sixth (and capstone) package of the FM04 Style IR
family.  Pure integration glue over the five sibling packages.

### Added

- `compile(doc, target, options): CompileResult` — single entry point
  that:
  1. Validates the input via `validateStyleDocument` (captures any
     `StyleError.errors[]` into `result.errors`; never throws on
     shape).
  2. Optionally composes a theme onto the validated document via
     `composeWithTheme` — theme provided either by value or by
     string name (looked up via the optional `themeRegistry`).
  3. Dispatches to one of `translateToCss` / `translateToLatex` /
     `translateToTerminal` based on `target`.
- `CompileOptions` — `activeContexts`, `usedRuleIds?`, `scope?`,
  `theme?`, `themeRegistry?`.
- `CompileResult` — `target`, `output`, `emittedRules`, `warnings`,
  `errors`.  Frozen everywhere.
- `CompileTarget` type alias for the closed set
  `"css" | "latex" | "terminal"`.
- `isCompileError` / `isCompileSuccess` — complementary type
  guards on `result.errors.length`.
- `fingerprintDocument(doc): string | null` — convenience that
  runs the validator + canonical serializer to produce a
  byte-stable cache key (or `null` if validation fails).

### Spec adherence

Per FM04 §13 (composition concerns) and FM03 (orchestrator
integration).  No new spec ground broken — wraps existing primitives.

### Behavioural notes

- **Validator failure ⇒ captured in `result.errors`, never
  thrown.**  The orchestrator's contract is "never throws on
  shape".  Programmer errors (unknown target, theme-name-without-
  registry) still throw `TypeError`.
- **Theme name not found in registry ⇒ WARNING, not error.**
  Translation continues with base tokens.  Caller distinguishes via
  the warning's `code: "THEME_NOT_FOUND"`.
- **No theme provided ⇒ base document translates verbatim.**
  Reference-equal to `validated.document` when no composition runs.
- **`emittedRules` and `warnings` are frozen `readonly` arrays.**
  Spread before sorting / mutating in tests.
- **Same `(doc, target, options)` triple ⇒ byte-identical output.**
  FM03 reproducibility.

### Spec divergences

None.

### Security posture

Three concerns explicitly addressed at the orchestrator boundary:

- **Validator-error capture.**  `StyleError.errors[]` is copied via
  spread into a frozen array; the original error object (and its
  stack) doesn't leak.  Tests pin the capture path for both `null`
  input and a structurally-broken document.
- **Theme registry lookup safety.**  Pass-through to
  `forme-style-theme`'s `Map`-backed registry — own-key semantics
  by construction, prototype-pollution names refused defensively.
- **Error/warning message safety.**  User-supplied theme names land
  in messages via `JSON.stringify` (never raw) — a malicious name
  containing control chars or quotes lands as its JSON-escaped
  form, not as raw bytes that could pollute a log stream.

### Tests

21 tests in `orchestrator.test.ts`:

- 3 happy-path dispatches (CSS / LaTeX / terminal)
- 2 validator failure captures (`null`; partial doc)
- 5 theme composition scenarios (by value; by name; unknown name
  warns; throws on missing-registry; no theme)
- 4 options pass-through (activeContexts / usedRuleIds / scope
  for both CSS and terminal)
- 1 unknown target throws
- 2 reproducibility pins (CSS and LaTeX with theme)
- 1 type guard complementary
- 2 fingerprintDocument (valid / invalid)
- 1 non-StyleError re-raise verification

Coverage: **97.77% line / 94.73% branch** — above the FM04 §14.4
≥95% line target.  Uncovered branches are the defensive
"validator threw something other than `StyleError`" re-raise path
(unreachable via the validator's documented behaviour; defence
against future breakage).

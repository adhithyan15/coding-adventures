# Changelog — @coding-adventures/forme-style-theme

## 0.1.0 — 2026-05-17

Initial release.  Third package of the FM04 Style IR family —
theme composition + in-memory registry + bulk `TokenRef` resolution.

### Added

- `composeWithTheme(base, theme): StyleDocument` — apply a
  `Theme`'s sparse token overrides + appended rules to a base
  `StyleDocument` per FM04 §7.2.  Per-named-entry merge within
  every bucket (recurses into the five `typography` sub-buckets);
  theme rules are appended (FM04 §4.9 specificity = source order);
  inputs are not mutated.
- `createThemeRegistry(): ThemeRegistry` — in-memory registry per
  FM04 §13.3.  `register(theme)` (replace-on-duplicate for dev-mode
  hot reload), `lookup(name)`, `list()` (sorted, frozen).
- `resolveTokenRefs(doc, refs): Map<string, ResolvedValue | null>`
  per FM04 §3.5 — bulk-resolve `TokenRef`s against a document's
  tokens for analyser pre-passes (AOT CSS slicer, LaTeX preamble
  extractor, theme coverage reporters).  Chain depth capped at 8
  hops; cycles return `null`.
- `ResolvedValue` type = `Color | Length | Shadow | FontStack |
  number` — the recognised leaf value types a `TokenRef` may
  resolve to.

### Spec adherence

Implements FM04 §7.2 (theme composition), §13.3 (theme registry),
§3.5 (`TokenRef` resolution).  No exports beyond those three concerns
— keeps the surface focused.

### Behavioural notes

- **`composeWithTheme` returns a *new* `StyleDocument`.**  When
  `theme.tokens` is absent the resulting `tokens` is reference-equal
  to the base's (zero-copy fast path); same for `rules` when
  `theme.rules` is absent.
- **`typography` is a two-level bucket** — `families`, `scale`,
  `weights`, `leading`, `tracking`.  Each is independently
  per-name-overridable.  TypeScript's `Partial<TokenSet>` is
  one-level partial, so partial typography overrides may need a
  cast at the call site; runtime handling is correct.
- **Theme rules trail base rules in the merged document.**  Per
  FM04 §4.9 source-order = specificity, so theme rules naturally
  win on equal-specificity ties without any explicit "cascade"
  machinery in this package.
- **`createThemeRegistry()` yields independent instances.**  No
  global state; per-tenant / per-project registries are a one-line
  ergonomic.
- **`list()` is sorted lexicographically AND frozen.**  Sorted so
  output is byte-stable across runs (FM04 §12 reproducibility);
  frozen so accidental in-place sorts by callers don't race future
  calls.
- **`resolveTokenRefs` collapses duplicate input paths.**  Two refs
  with the same `path` produce one map entry (idempotent — the
  resolution is pure, so the value is identical).

### Spec divergences

None.

### v0 simplifications (documented)

- **In-memory registry only.**  Persistent backing (filesystem,
  database) is deferred until FM06 (AOT compiler) needs it.
- **One-theme-at-a-time composition.**  Multi-theme stacks compose
  by chaining: `composeWithTheme(composeWithTheme(base, t1), t2)`.

### Security posture

Pre-push security focus areas (per the task brief):

- **Deep-merge prototype-pollution defence.**  Token names
  `__proto__`, `constructor`, `prototype` refused unconditionally
  across `mergeRecord` / `mergeTypography` / `mergeTokens`.  Merged
  records backed by `Object.create(null)` so even a successful
  bypass of the deny-list leaves `Object.prototype` untouched.
  Two tests pin the behaviour (forbidden `__proto__` and
  `constructor` keys are dropped; pollution doesn't reach
  `Object.prototype`).
- **Registry mutation safety.**  Backed by `Map<string, Theme>`
  (own-key semantics by construction).  The three forbidden names
  are additionally refused with a thrown error — they'd only ever
  appear through a programming mistake.  Two tests pin the
  rejection paths.
- **`TokenRef.path` walks** mirror the
  `forme-style-to-css/token-resolver` defence: deny-list +
  `hasOwnProperty.call` (own-only).  Three tests pin
  `__proto__` / `constructor` / `prototype` in path segments.

Validator-trust posture: the `forme-style-ir` validator already
restricts token names to a dotted-identifier grammar that doesn't
admit the forbidden keys.  Defending here too means a stage that
hands us a hand-rolled `Theme` (bypassing the validator) can't
poison `Object.prototype` through us.

### Tests

51 tests across 3 files:

- `compose.test.ts` (18 tests — token override, rule append,
  immutability, passthrough fields, token extensions merge,
  reproducibility, prototype-pollution defence covering
  `__proto__` / `constructor` / `prototype`)
- `registry.test.ts` (13 tests — CRUD, replace-on-duplicate, input
  validation, isolation, bounded self-referential lookup)
- `resolve.test.ts` (20 tests — concrete leaves, chains, failure
  modes including cycle and NaN, bulk semantics, prototype-traversal
  defence)

Coverage: **100% line / 96.51% branch** — above the FM04 §14.4
≥95% line target.

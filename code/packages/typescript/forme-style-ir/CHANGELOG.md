# Changelog — @coding-adventures/forme-style-ir

## 0.1.0 — 2026-05-17

Initial release.  First package of the FM04 Style IR family.

### Added

- **Tokens** (`tokens.ts`) — `TokenSet`, `Color` (rgb / hsl / oklch /
  named), `Length` (11 units), `Shadow`, `TokenRef`,
  `TypographyTokens`, `FontStack`.  `LENGTH_UNITS` frozen tuple +
  `LengthUnit` type alias.  `isTokenRef` predicate.  `emptyTokenSet`
  constructor for sparse theme bases.
- **Selectors** (`selectors.ts`) — the 13-variant `Selector` union:
  `node-type`, `node-type-level`, `custom-kind`, `tag`, `id`, `role`,
  `nth`, `child-of`, `descendant-of`, `adjacent`, `and`, `or`, `not`.
  `SELECTOR_KINDS` frozen tuple + `SelectorKind` alias.  `sel.*`
  ergonomic constructors that preserve the static union.
- **Properties** (`properties.ts`) — `StyleProperty` discriminated
  union covering 29 kernel-known kinds plus an open
  ``ExtensionProperty`` (`ext:<name>`) slot.  `PROPERTY_KINDS` frozen
  tuple + `PropertyKind` alias.  `isExtensionKind` predicate.
  Supporting value types: `BoxSides<T>`, `TextDecoration`,
  `BorderSpec`.
- **Contexts** (`contexts.ts`) — kernel-blessed context constants
  (`CONTEXT_PRINT` / `SCREEN` / `DARK` / `NARROW` / `WIDE` /
  `REDUCED_MOTION` / `HIGH_CONTRAST`), `STANDARD_CONTEXTS` frozen
  tuple, `isExtensionContext` and `isRecognisedContext` helpers.
- **Style document** (`style-document.ts`) — `StyleDocument`,
  `StyleRule`, `Theme`, branded `StyleRuleId`.  `styleRuleId(s)`
  and `emptyStyleDocument()` constructors.
- **Errors** (`style-error.ts`) — `StyleError` class with structured
  `errors[]` (one-pass-many-errors pattern).  `STYLE_ERROR_CODES`
  frozen vocabulary with 14 codes.  `StyleErrorEntry`,
  `StyleWarning` interfaces.  `name = "StyleError"`.
- **Validator** (`validate.ts`) — `validateStyleDocument(value)`
  returns `{ document, warnings }` on success; throws single
  `StyleError` carrying every violation on failure.  Walks the
  document tree exhaustively; defensive top-level checks bail early
  only when subsequent traversal would crash on `undefined.xyz`.
- **Canonical serializer** (`canonical.ts`) —
  `canonicalStyleDocument(doc)` returns byte-stable JSON for
  hashing.  Sorted keys at every depth; `rules` preserved (source
  order is specificity per FM04 §4.9); `contexts` treated as a set
  (sorted); non-finite numbers throw `RangeError`.

### Spec adherence

Implements FM04 §3 (tokens) / §4 (selectors) / §5 (properties) / §6
(contexts) / §7 (theme types) / §8 (StyleDocument) / §9 (error/
warning types only — translators ship in sibling packages) / §12
(canonical serialisation) / §14 (testing contract).

### Spec divergences (documented)

- **Translator interface (FM04 §9.1) is NOT exported here.** It
  belongs in each translator package (`forme-style-to-css`, etc.).
  Defining the abstract interface in this package would force
  translators into a circular-ish coupling for a single-method
  contract; keeping it next to the concrete implementation is
  cleaner.  The `StyleWarning` type IS here so translators can
  re-use it without importing each other.
- **Theme composition function (FM04 §7.2) is NOT in this package.**
  Belongs in `forme-style-theme` (FM04 §13.3) along with the theme
  registry.  This package defines the `Theme` shape; composition is
  a follow-up.
- **`TokenRef` resolution is NOT in this package.**  Per FM04 §3.5
  resolution happens at translate time (because that's where the
  composed theme is in scope).  The validator only checks `TokenRef`
  *shape* — a dotted-identifier path.
- **JSON Schema (FM04 Appendix A) is NOT shipped.**  Appendix A is
  informative; deferring to forme-pipeline-config's compiled-from-
  TS approach when JSON Schema is actually needed at the boundary.

### v0 simplifications (documented)

- **No `Gradient` color variant** (FM04 §15.5 open question).  Add
  when a translator needs it.
- **No `transition` / `animation` properties** (FM04 §15.3 open
  question).  Add when FM05 Interactivity IR needs them.
- **No logical properties** (FM04 §15.4) — box sides are physical
  (`top` / `right` / `bottom` / `left`).  A logical-property layer
  would land as an `ext:i18n:*` extension.

### Security posture

Pre-push security review (PASS-WITH-NOTES) flagged unbounded
recursion in `validateSelector` and `stableStringify` as a
stack-overflow risk for hand-rolled cyclic inputs (not reachable via
`JSON.parse`, which is acyclic by construction).  Both functions
now carry a depth guard at 1000 levels — real documents hit
single-digit depth; the guard converts a crash into a clean
`MALFORMED` error (validator) or `RangeError` (serializer).  Two
tests pin the behaviour (self-referential `not` selector inside
the validator, self-referential `extensions` object inside the
serializer).

Validator regexes (`TOKEN_REF_PATH_RE`, `EXTENSION_KEY_RE`) are
anchored with disjoint character classes across quantified groups
— no catastrophic backtracking.

No prototype-pollution surface: the validator never assigns to
attacker-controlled keys.  It only reads input and pushes to its
own `errors` / `warnings` / `seenIds` collections.

### Tests

147 tests across 7 files:
- `tokens.test.ts` (13 tests)
- `selectors.test.ts` (9 tests)
- `properties.test.ts` (7 tests)
- `contexts.test.ts` (9 tests)
- `validate.test.ts` (71 tests — every documented rejection reason
  plus multi-error collection)
- `validate-coverage.test.ts` (27 tests — happy-path variants
  pushing branch coverage plus cycle-guard pin)
- `canonical.test.ts` (11 tests — byte-stability, set-vs-sequence
  preservation, round-trip, cycle-guard pin)

Coverage: **98.1% line / 96.38% branch** — above the FM04 §14.4
≥95% target.

# Changelog — @coding-adventures/forme-style-to-css

## 0.1.0 — 2026-05-17

Initial release.  Second package of the FM04 Style IR family —
the reference Style IR → CSS translator.

### Added

- `translateToCss(doc, options): TranslateResult<string>` — the
  public entry point per FM04 §9.2.
- `TranslateOptions`: `activeContexts`, `usedRuleIds?`, `scope?`.
- `TranslateResult<string>`: `output`, `emittedRules`, `warnings`.
- Property mappers (`property-mappers.ts`) — exhaustive `switch`
  over every kernel-known `StyleProperty.kind`; unknown `ext:*`
  warns + skips per FM04 §9.6.
- Selector mapper (`selector-mapper.ts`) — every selector form maps
  to its FM04 §9.2 CSS form; `nth` translates 0-based IR indices to
  1-based CSS; composition (`and`, `or`, `not`) supports cartesian
  product over comma-separated paths.
- Context mapper (`context-mapper.ts`) — kernel-blessed contexts
  each map to their FM04 §9.2 `@media` query body; `ext:*` returns
  null (caller warns + skips).
- Token resolver (`token-resolver.ts`) — `TokenRef` → concrete value
  walk through the `TokenSet` tree.  Chains up to 8 hops (covers
  any sensible design system); cycles return null + warning.
  Typed wrappers for each leaf type (Color, Length, Shadow,
  FontStack, number) reject type-mismatched refs.
- Value mappers (`value-mappers.ts`) — Color / Length / FontStack /
  Shadow → CSS literal form.  CSS Color L4 4-arg `rgb(... / a)`
  form when alpha < 1; 3-arg form when opaque (cleaner output).
  Font families with spaces / special chars get quoted.

### Spec adherence

Implements FM04 §9.2 reference CSS translator end-to-end.  All three
mapping tables (properties, selectors, contexts) covered.

### Behavioural notes

- **Unconditional rules emit before `@media` blocks.**  Preserves
  cascade semantics (unconditional rules form the base; contextual
  rules layer on top).
- **Rules sharing a context are grouped under one `@media` block**
  in source order — matches what hand-authored CSS looks like and
  avoids context-bleeding when later non-contextual rules would
  otherwise interleave.
- **Empty rule blocks (every property warned + skipped) are
  suppressed** — no `p { }` in output, no rule id in `emittedRules`.
- **`nth` 0-based-to-1-based shift at the boundary** — programmers
  index from 0; CSS uses 1-based.  IR's `n: 0` ⇒ CSS
  `:nth-child(1)`.
- **Scope applies per comma-path** — `scope=".p"` + selector `p, h1`
  → `.p p, .p h1`, not `.p p, h1`.

### Spec divergences (documented)

- **`forme-style-to-css` does NOT define `StyleTranslator<Out>`
  abstract interface.**  Per FM04 §9.1 the interface lives next to
  each translator (avoids forcing all translators into a circular
  dependency for a single-method contract).  The `TranslateOptions`
  / `TranslateResult` types are exported here; other backends will
  redeclare equivalents.

### Security posture

Pre-push security review (PASS-WITH-NOTES) raised three findings,
all addressed before push:

- **MEDIUM** — `walkPath` in `token-resolver.ts` used bracket-access
  on attacker-controllable keys, leaving a path-traversal vector
  through `__proto__` / `constructor` / `prototype` for
  hand-rolled TokenRefs that bypass the IR validator's grammar.
  Now guards with an explicit deny-list AND
  `Object.prototype.hasOwnProperty.call` to refuse inherited
  properties.  Two new tests pin the behaviour.
- **LOW** — `fontFamilyEntry` escaped `"` but not `\`, leaving a
  malformed-input pollutant when an input family name contained a
  raw backslash.  Now escapes `\` first (so subsequent `\"`
  escapes don't collide), then `"`; and strips ASCII control
  characters that would terminate the CSS string literal.  Two new
  tests pin the behaviour.
- **INFO** — `TranslateOptions.scope` is concatenated verbatim
  into output.  Documented in JSDoc as "caller-trusted": callers
  must supply a valid CSS selector fragment.

CSS escape helpers (`escapeIdent`, `escapeAttrValue`) were confirmed
sound — `\<hex> ` for identifiers (CSS Syntax L3), `\"` / `\\`
inside double-quoted attribute-value context (CSS Strings L3).

### Tests

118 tests across 6 files:
- `value-mappers.test.ts` (20 tests — adds backslash + control-char escape pins)
- `selector-mapper.test.ts` (22 tests)
- `context-mapper.test.ts` (3 tests)
- `token-resolver.test.ts` (14 tests — adds prototype-traversal guard pins)
- `property-mappers.test.ts` (40 tests, with exhaustive-coverage
  meta-check that every PROPERTY_KINDS entry produces output)
- `translate.test.ts` (19 tests — end-to-end + slicing + scoping +
  reproducibility)

Coverage: **98.94% line / 93.97% branch** — above the FM04 §14.4
≥95% line target.

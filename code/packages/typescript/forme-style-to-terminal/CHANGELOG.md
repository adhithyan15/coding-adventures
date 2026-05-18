# Changelog — @coding-adventures/forme-style-to-terminal

## 0.1.0 — 2026-05-17

Initial release.  Fifth package of the FM04 Style IR family —
the third concrete backend translator (after `forme-style-to-css`
and `forme-style-to-latex`), completing the v0 multi-backend story.

### Added

- `translateToTerminal(doc, options): TranslateResult<string>` —
  public entry point per FM04 §9.4.  Emits a TS module source
  string exporting `formeStyles: ReadonlyMap<string, AnsiStyle>`
  where `AnsiStyle = { prefix, suffix }`.  Consumers wrap document
  content with `entry.prefix + content + entry.suffix`.
- `TranslateOptions`: `activeContexts`, `usedRuleIds?`, `scope?`.
  Same shape as CSS / LaTeX translators, redeclared locally per
  FM04 §9.1.
- `TranslateResult<string>`: `output`, `emittedRules`, `warnings`.
- Property mappers (`property-mappers.ts`) — exhaustive `switch`
  over every kernel-known `StyleProperty.kind` (29 kinds).  Native
  SGR for color / background / font-weight / font-style /
  text-decoration / visible-conceal; warn-skip for everything that
  needs page geometry (padding, max-width, border-*, shadow,
  opacity, page-break, etc.) — terminals are a character grid.
- Selector mapper (`selector-mapper.ts`) — produces a human-readable
  description string for each Selector (used in per-rule comments).
  No selector→key mapping happens here; consumers look up by rule
  id.
- Context mapper (`context-mapper.ts`) — `contextRecognised(name)`
  returns true for kernel-blessed contexts, false for `ext:*`.
  No per-context emission machinery: the translator filters rules
  through `activeContexts` and emits the survivors flat.
- Token resolver (`token-resolver.ts`) — same FM04 §3.5 walker as
  the CSS / LaTeX translators, with the same proto-pollution
  defence (deny-list + `hasOwnProperty`).  Cycles cap at 8 hops.
- Value mappers (`value-mappers.ts`) — `Color` → `[R, G, B]` triple
  (HSL inline-converts; OKLCH warn-skips; named via small safe map);
  plus `colorToSgrFg` / `colorToSgrBg` convenience wrappers for the
  full `38;2;...` / `48;2;...` SGR prefixes.
- Escape helpers (`escape.ts`) — `stripAnsiUnsafe` (strips ESC, C1
  CSI, C1 OSC, ASCII control bytes 0x00–0x1F + 0x7F + C1 0x80–0x9F);
  `escapeTsString` (escapes `\` and `"` for double-quoted TS string
  literal, single-pass per CodeQL's incomplete-string-escaping rule);
  `sanitiseKey` (alias of `escapeTsString` for intent).

### Spec adherence

Implements FM04 §9.4 reference terminal translator end-to-end.
All three mapping tables (properties, selectors, contexts) covered
with documented warn-skip for the cases terminals can't natively
express.

### Behavioural notes

- **Output is a TS module source string.**  Unusual compared to
  the CSS / LaTeX translators (which emit content directly); the
  rationale: terminals have no preamble — wrappers must be applied
  per-rule at content-emit time, so the most useful artefact is a
  lookup table.
- **Rules where every property warn-skipped still emit** with an
  empty `prefix` / `suffix`.  Consumers can look up the rule id
  and get a no-op wrap (less surprising than a Map miss).  But the
  id is NOT added to `emittedRules` — the AOT compiler doesn't
  need to track rules that produce no visual change.
- **`important` has no terminal equivalent** (terminals have no
  cascade).  Honoured as a no-op.
- **24-bit truecolour only.**  Future option may add a 256-colour
  / 16-colour quantised fallback for older terminals.

### Spec divergences (documented)

- **`forme-style-to-terminal` does NOT define `StyleTranslator<Out>`
  abstract interface.**  Same rationale as the other backends:
  per FM04 §9.1 the interface lives next to each translator.

### v0 simplifications (documented)

- **24-bit truecolour only** — modern terminals all support it.
- **OKLCH warn-skips** — CIE round-trip out of scope.
- **No SGR idempotence elision** — consumer is responsible for not
  double-wrapping the same rule on the same content.

### Security posture

Pre-push focused security review areas:

- **ANSI escape-sequence injection.**  Every caller-controlled
  string interpolated into the output routes through
  `stripAnsiUnsafe` (strips ESC 0x1B, C1 CSI 0x9B, C1 OSC 0x9D,
  all ASCII control bytes 0x00–0x1F + 0x7F, and the full C1 range
  0x80–0x9F).  Even a hand-rolled IR bypassing the validator's
  grammar cannot drive cursor moves, screen clears, or arbitrary
  SGR through us.
- **TS-string-literal escaping.**  Map keys and SGR strings land
  in double-quoted JS literals; `escapeTsString` handles `\` and
  `"` in a single pass (the form CodeQL's
  incomplete-string-escaping rule accepts).
- **Prototype-pollution in `walkPath`.**  Same defence as the CSS
  / LaTeX translators: deny-listed segments + own-key
  `hasOwnProperty.call`.
- **Defensive numeric coercion.**  `colorToRgbTriple` treats `NaN`
  / `Infinity` channels as 0 rather than letting them propagate
  into the SGR sequence as `NaN` literal text.
- **Recursion depth cap.**  `selectorDescription` recurses on
  composition selectors; an adversarial hand-rolled IR could nest
  10k+ deep and blow the JS call stack.  `MAX_DESC_DEPTH = 64`
  matches the spirit of `token-resolver`'s `MAX_RESOLVE_DEPTH`;
  past the cap we return a `…(truncated)` marker rather than
  throw.  One test pins the behaviour.

### Tests

128 tests across 7 files:

- `escape.test.ts` (15 — ANSI-unsafe stripping for every dangerous
  byte range; TS-string escaping for `\` and `"`; passthrough for
  Unicode > 0x9F)
- `value-mappers.test.ts` (13 — color models, clamping, NaN
  defensiveness, named-color lookup, SGR fg/bg prefixes)
- `selector-mapper.test.ts` (19 — every Selector kind including
  composition; defensive control-char sanitisation; depth cap pin)
- `context-mapper.test.ts` (3 — kernel contexts; ext: rejection;
  unknown / empty)
- `token-resolver.test.ts` (18 — happy path, proto-pollution
  defence including inherited-key check, typed wrappers for all
  five leaf types, cycle cap, NaN rejection)
- `property-mappers.test.ts` (44 — every kernel kind, exhaustive
  meta-check over `PROPERTY_KINDS`, defensive fallthroughs)
- `translate.test.ts` (16 — end-to-end happy path, filtering,
  scope, reproducibility, ANSI / TS-string injection defence)

Coverage: **100% line / 97.43% branch** — above the FM04 §14.4
≥95% line target.

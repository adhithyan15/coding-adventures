# Changelog — @coding-adventures/forme-style-to-latex

## 0.1.0 — 2026-05-17

Initial release.  Fourth package of the FM04 Style IR family —
the second concrete backend translator (after `forme-style-to-css`),
validating the multi-backend story FM04 §9.1 promises.

### Added

- `translateToLatex(doc, options): TranslateResult<string>` — public
  entry point per FM04 §9.3.  Emits a LaTeX preamble fragment
  (`\definecolor` / `\setlength` / `\newcommand` / `\if<flag>`).
- `TranslateOptions`: `activeContexts`, `usedRuleIds?`, `scope?`.
  Same shape as the CSS translator's options, redeclared locally per
  FM04 §9.1 (each translator owns its own `TranslateOptions` /
  `TranslateResult` rather than importing a shared package — avoids
  a circular dependency for a single-method contract).
- `TranslateResult<string>`: `output`, `emittedRules`, `warnings`.
- Property mappers (`property-mappers.ts`) — exhaustive `switch`
  over every kernel-known `StyleProperty.kind` (29 kinds).  Native
  LaTeX for color / font / leading / page-break / spacing;
  warn-and-skip for decoration without a preamble form
  (`background`, `border`, `border-radius`, `shadow`, `opacity`,
  `padding`, `display`, `vertical-align`, `tracking`).
- Selector mapper (`selector-mapper.ts`) — simple kinds become
  stable `\formeNode<Type>` / `\formeHeading<Word>` / `\formeKind`
  / `\formeTag` / `\formeId` / `\formeRole` macros.  Composition
  kinds (`and`, `or`, `not`, `nth`, `child-of`, `descendant-of`,
  `adjacent`) warn-and-skip — no preamble equivalent.
- Context mapper (`context-mapper.ts`) — seven kernel contexts map
  to `\if<flag>` conditionals; ext:* returns null.  Exports the
  `CONTEXT_FLAG_DECLARATIONS` constant (frozen) that the translator
  emits at the top of the preamble.
- Token resolver (`token-resolver.ts`) — same FM04 §3.5 walker as
  `forme-style-to-css`, intentionally duplicated.  Cycles cap at 8
  hops; prototype-pollution defence (deny-list + `hasOwnProperty`).
- Value mappers (`value-mappers.ts`) — `Color` → xcolor `{RGB}{r,g,b}`
  (HSL converts inline; OKLCH warn-skips for v0; named colors via a
  small safe map); `Length` → LaTeX dimension (px→pt at 0.75×, rem→em,
  pass-through for pt/mm/in/ex/em, skip for %/vh/vw/ch); `FontStack`
  → first family escaped (LaTeX has no fallback chain).
- Escape helpers (`escape.ts`) — `escapeLatexText` (all ten LaTeX
  specials + control-char strip; uses placeholder substitution for
  multi-character escapes so synthetic braces don't get
  double-escaped); `latexIdent` (encodes non-letter chars as `Z<hex>Z`
  so command names stay valid).

### Spec adherence

Implements FM04 §9.3 reference LaTeX translator end-to-end.  All
three mapping tables (properties, selectors, contexts) covered with
documented warn-and-skip for the cases LaTeX can't natively express.

### Behavioural notes

- **Preamble header is fixed.**  Every output begins with
  `% forme-style-to-latex generated preamble` followed by the seven
  `\newif\if<flag>` declarations.  Document authors `\printtrue`
  etc. to switch contexts at compile time.
- **Unconditional rules emit BEFORE `\if<flag>` blocks.**  Same
  cascade-friendly ordering as the CSS translator.
- **Rules sharing a context group under one `\if...\fi` block**
  in source order — minimises `\fi` noise and matches readable
  hand-authored LaTeX.
- **Empty rule blocks suppressed.**  A rule where every property
  warn-skipped (or the selector itself was unmappable) does not
  emit; its id is not in `emittedRules`.
- **`important` becomes a `% !important` comment trailer.**  LaTeX
  has no specificity to override, so we preserve traceability
  rather than the semantics.
- **Scope is concatenated before the macro name.**  e.g. `scope =
  "\\page"` + macro `\formeNodeParagraph` → `\page\formeNodeParagraph`.
  Caller-trusted — same posture as the CSS translator's `scope`.

### Spec divergences (documented)

- **`forme-style-to-latex` does NOT define `StyleTranslator<Out>`
  abstract interface.**  Same rationale as `forme-style-to-css`:
  per FM04 §9.1 the interface lives next to each translator.
  `TranslateOptions` / `TranslateResult` redeclared here.

### v0 simplifications (documented)

- **OKLCH warn-skips.**  Round-trip through CIE / sRGB gamut
  mapping is out of scope.
- **`align` is LTR-only** — `start`/`end` → `\raggedright`/`\raggedleft`.
  A future i18n layer (`ext:i18n:*`) re-emits contextually for RTL.
- **`tracking` warns** — needs `microtype`.
- **`text-decoration: line-through` warns** — needs `ulem`.

### Security posture

Pre-push focused security review areas:

- **LaTeX injection.**  Every interpolated string routes through
  `escapeLatexText` (text-mode) or `latexIdent` (command-name).
  All ten LaTeX specials (`\ % $ & _ # { } ^ ~`) covered.  The
  backslash and accent escapes use placeholder substitution so
  their synthetic `{` / `}` don't get double-escaped on the brace
  pass — a subtle bug we pinned with a composite test
  (`escapes all ten in one string in the right order`).
- **Prototype-pollution in `walkPath`.**  Mirrors
  `forme-style-to-css`'s defence: deny-listed segments + own-key
  `hasOwnProperty` check.  Three tests pin the rejections; one
  test pins inherited-key non-traversal.
- **Control-character stripping.**  ASCII control chars (0x00–0x1F,
  0x7F) stripped from every escape helper.  Tests pin behaviour
  for both text and identifier paths, plus end-to-end through
  rule-id comments.
- **Numeric heading levels routed through `latexIdent`.**  Defence
  in depth: a hand-rolled IR with `node-type-level: -1` or
  `node-type-level: 1.5` would otherwise produce broken
  `\formeHeadingD-D1` / `\formeHeadingD1D.D5` macros (raw `-` / `.`
  are invalid in LaTeX command names — not injection, but
  cosmetically broken).  Now encoded as `Z<hex>Z` runs.  Two tests
  pin the behaviour.

### Tests

155 tests across 7 files:

- `escape.test.ts` (18 — every LaTeX-special, control-char strip,
  identifier sanitisation, all-ten-in-one composite)
- `value-mappers.test.ts` (25 — color models, length units,
  font-stack escaping, fallback comments)
- `selector-mapper.test.ts` (20 — simple kinds, identifier
  sanitisation, composition warn-skips, defensive numeric
  encoding for negative / fractional / out-of-range heading levels)
- `context-mapper.test.ts` (5 — kernel contexts + flag declarations)
- `token-resolver.test.ts` (14 — happy path, prototype-pollution
  defence, typed wrappers, cycle cap, NaN rejection)
- `property-mappers.test.ts` (52 — every kernel kind, exhaustive
  meta-check over `PROPERTY_KINDS`, defensive fallthroughs)
- `translate.test.ts` (16 — end-to-end happy path, filtering, scope,
  important, reproducibility, LaTeX-injection defence)

Coverage: **100% line / 96.15% branch** — above the FM04 §14.4
≥95% line target.

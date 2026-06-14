# Changelog — @coding-adventures/forme-transform-typography

## 0.1.0 — 2026-05-18

Initial release.  Seventh FM00 v0 stage package — third concrete
§5.3 transform.  Walks a `DocumentNode` and produces a
typography-corrected copy with smart quotes, em/en dashes,
ellipsis, and (opt-in) common ligatures applied to every
`TextNode`.

Sits alongside `forme-feeds`, `forme-opengraph`,
`forme-index-renderer`, `forme-transforms`,
`forme-transform-autolink-headings`, and `forme-transform-toc`.

### Added

- `typography(doc, options?): DocumentNode` — main entry.  Walks
  the document depth-first, returns a fresh copy with every
  `TextNode.value` run through `typeset`.  Input is never
  mutated.
- `typeset(text, options?): string` — string-level entry for
  callers outside the AST context.  Single-pass character loop
  with `charCodeAt`-based lookahead.  Zero regex backtracking
  surface.
- `TypographyOptions { smartQuotes?, dashes?, ellipsis?, ligatures? }`
  type.  All default `true` except `ligatures` (default `false`).

### Spec adherence

Implements FM00 v0 §5.3 `transform-typography`.  No spec
divergences.

### Behavioural notes

- **Single-pass O(N) character loop, not regex.**  Avoids both
  the multi-rule precedence fragility and the CodeQL
  polynomial-regex warnings that chained `.replace()` would
  introduce.  Lookahead via `charCodeAt(i + 1 / 2 / 3)` decides
  dash length, ellipsis, and ligature matches.
- **Quote direction context.**  `"` becomes left-DQ (U+201C) at
  the start of the string or after whitespace, right-DQ
  (U+201D) elsewhere.  `'` becomes right-SQ (U+2019, apostrophe)
  between alphanumerics — handles `don't` / `it's` correctly.
- **Substitution precedence.**  `---` matched before `--`;
  `...` matched before single `.`.  Longer patterns win because
  the lookahead check happens in length-descending order within
  the same character branch.
- **All flags independently toggleable.**  Useful for
  documentation sites that want smart quotes but NOT dashes
  (since `---` is also a Markdown thematic break in source code
  samples).
- **Identity fast-path.**  When all flags are disabled,
  `typeset` returns the input unchanged without scanning.
- **Pass-through nodes.**  `CodeBlockNode`, `CodeSpanNode`,
  `RawBlockNode`, `RawInlineNode`, URLs on `LinkNode` /
  `ImageNode` / `AutolinkNode`, and `ImageNode.alt` pass
  through verbatim.  Smart-quoting code samples or URLs would
  break syntax / link resolution.
- **Fresh tree per call.**  Even pass-through nodes are
  re-allocated (new object with the same primitive fields), so
  the output guarantee "no shared references with input" holds
  uniformly.

### Security posture

Four concerns explicitly addressed (pre-push review):

- **No AST mutation.**  Input `DocumentNode` is never
  modified; every returned node is freshly constructed.
  JSON-snapshot tests confirm.
- **Deterministic substitution.**  Single forward pass, no
  global state, no `Map`/`Set` iteration, no randomness.
  Same input → byte-identical output.
- **No ReDoS surface.**  `for` loop with `charCodeAt`-based
  lookahead — zero regex, zero backtracking.  Trivially passes
  CodeQL polynomial-regex analysis.
- **Transformed text is data, not markup.**  Output contains
  only the source characters plus typographic replacements
  (`U+201C`, `U+2013`, `U+00A9`, etc.) — no HTML metacharacters
  introduced.  Renderers still own the HTML-escape boundary
  exactly as they would for raw text.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

76 tests across 2 files:

- `typeset.test.ts` (44) — smart quotes (double-quote pairs,
  position-dependent open/close, after non-breaking space),
  single quotes / apostrophes (between letters, after digits,
  after punctuation), dashes (`--` en, `---` em, longer-wins
  precedence, four-hyphen edge case, single-hyphen
  passthrough), ellipsis (`...`, 1/2/4 dots), ligatures
  (default off, all six lower/upper variants, non-match
  passthrough, end-of-string `(`), option toggles (each flag
  off independently, all-off identity fast-path, partial
  combos), real-world combinations (full prose sentence),
  purity / determinism / non-string coercion / empty string /
  already-prettified passthrough, Unicode (CJK passthrough,
  emoji, quote-after-CJK).
- `walk.test.ts` (32) — basic transformation inside
  paragraph / heading, recursion into emphasis / strong /
  strikethrough / link label (with URL passthrough),
  pass-through nodes (code_block, code_span, raw_block,
  raw_inline, image alt + destination, autolink, hard_break,
  soft_break, thematic_break), block containers (blockquote,
  list / list_item, task_item, table cells header + body,
  nested DocumentNode), options propagation to deep text,
  purity (no input mutation, fresh tree even for passthrough
  nodes, byte-identical output, defaults match no-options
  call), defensive non-tree BlockNode variants as direct
  siblings (list_item / task_item / table_row / table_cell).

Coverage: **97.32% line / 98.61% branch** across all source
files with logic.  Uncovered lines are TypeScript `never`
exhaustiveness guards (`walk.ts` 138-141, 179-182) that cannot
fire at runtime.

### v0 simplifications (documented)

- **No locale-aware quote pairs.**  Always uses English curly
  quotes (`"" ''`).  German `„""`, French `«»` need a separate
  option deferred to v1.
- **Image alt-text passes through unchanged.**  v1 might add
  an `imageAlt: true` option for callers that want it typeset.
- **Heuristic limits.**  The apostrophe rule (between
  alphanumerics → right-SQ) doesn't catch every edge case
  ('twas, `rock 'n' roll`).  Renders correctly for the common
  cases; over-fitting without context would harm worse.
- **No abbreviation protection.**  `Mr...` becomes `Mr…`.
  Common abbreviations should use a hard space or trailing
  zero-width joiner if needed.

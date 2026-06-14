# Changelog — @coding-adventures/forme-transform-autolink-headings

## 0.1.0 — 2026-05-18

Initial release.  Fifth FM00 v0 stage package — first concrete
spec §5.3 transform.  Generates deterministic slug ids +
self-link anchor metadata for every `HeadingNode` in a
`DocumentNode`.

Sits alongside `forme-feeds`, `forme-opengraph`,
`forme-index-renderer`, and `forme-transforms`.

### Added

- `autolinkHeadings(doc): HeadingSlug[]` — walks `doc` depth-first
  in document order, finds every `HeadingNode`, slugifies its
  plain-text content via `extractText` + `slugify`, resolves
  collisions globally via `resolveCollisions`, returns one
  annotation per heading.
- `slugify(text): string` — GitHub-flavoured slugification.
  Lowercase, strip control bytes, strip non-`[a-z0-9 -]`,
  collapse runs, trim, fall back to `"section"` if empty.
- `resolveCollisions(candidates): string[]` — disambiguate
  repeated slug candidates by appending `-2`, `-3`, ... to later
  occurrences.  First occurrence stays unsuffixed.  Skips
  already-taken numeric suffixes.
- `extractText(inlines): string` — flatten an `InlineNode[]` to
  plain text.  Recurses into formatting wrappers, uses image alt
  text, treats breaks as single spaces, skips `raw_inline`.
- `HeadingSlug` type:
  `{ level: 1-6, text: string, slug: string, anchorHref: string }`.

### Spec adherence

Implements FM00 v0 §5.3 `transform-autolink-headings`.  No spec
divergences.  Spec calls for "add id + self-link to headings";
this package produces the annotation stream that renderers
consume to emit exactly that markup.

### Behavioural notes

- **Annotations, not AST mutation.**  The `document-ast` IR is
  immutable and has no `id` field on `HeadingNode`.  Rather than
  introducing a coordinated cross-package extension, this
  transform returns a parallel `HeadingSlug[]` indexed by
  encounter order.  Renderers walk the document and consume the
  annotation stream in lockstep.  Side benefit: the annotation
  stream is JSON-serialisable, supporting cross-process Forme
  deployments where parser and renderer run separately.
- **Global collision namespace.**  All headings in the document
  share one slug namespace — `## Setup` followed by `### Setup`
  produces `setup` / `setup-2` regardless of nesting depth.
  Matches GitHub's behaviour; prevents broken in-page links.
- **Walks nested containers.**  Headings inside blockquote, list,
  list_item, task_item are all found in document order.  Tables
  don't contain blocks (cell children are inline), so no walk
  into tables.
- **Defensive no-op for non-tree BlockNode variants.**  The
  `BlockNode` union includes `DocumentNode` / `ListItemNode` /
  `TaskItemNode` / `TableRowNode` / `TableCellNode` for type-
  system simplicity, but well-formed AST never places these as
  direct siblings of other blocks.  The walker silently ignores
  them rather than throwing — keeps the transform robust against
  hand-constructed inputs.
- **GitHub-flavoured slugify.**  Lowercase, ASCII-only (no
  Unicode preservation), digits kept, runs collapsed, fallback
  `"section"` for empty inputs.  Output guaranteed to match
  `/^[a-z0-9-]+$/`.

### Security posture

Three concerns explicitly addressed (pre-push review):

- **HTML injection via attacker-controlled heading text.**
  `slugify` strips everything except `[a-z0-9 -]`, so
  `<script>alert(1)</script>` becomes `scriptalert1script` —
  safe to interpolate into an `id="..."` attribute without
  escaping.  Pinned by `slugify.test.ts` and `autolink.test.ts`.
- **Attribute-breakout via quotes / equals / brackets.**  All of
  `"`, `'`, `=`, `<`, `>`, `&` are outside the kept character
  class and get stripped before the slug is ever emitted.
- **ASCII control-byte smuggling.**  NUL (`\x00`), DEL (`\x7F`),
  and every other ASCII control character is stripped explicitly
  before the main regex pass — defence-in-depth against a
  hypothetical parser that lets controls leak through.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

85 tests across 4 files:

- `slugify.test.ts` (25) — basic shape (lowercase, hyphen
  replacement, run collapse, trim, digit preservation, GitHub
  idiom matching), fallback for empty / whitespace / punctuation /
  non-ASCII / hyphens-only inputs, security (control bytes:
  NUL/ESC/DEL; HTML injection; quotes/angle brackets/ampersands),
  output guarantees (matches `/^[a-z0-9-]+$/`, non-empty,
  idempotent, deterministic), non-string defensive coercion.
- `collisions.test.ts` (15) — basic numbering (no collision /
  duplicate / triple / interleaved / first-unsuffixed), skip
  already-taken suffixes (single / multiple / non-contiguous
  gaps), determinism (same input byte-identical output,
  no-mutation, fresh-array), edge cases (empty / single / output
  length invariant / empty-string slugs).
- `extract-text.test.ts` (17) — basic text nodes, formatting
  wrappers (emphasis / strong / strikethrough / nested),
  link / code_span / image / autolink (URL + email) handling,
  hard / soft breaks → space, raw_inline skipped, real-world
  mixed-inline heading shapes.
- `autolink.test.ts` (28) — end-to-end (flat doc, document order
  preservation, level propagation, empty inputs, no-heading
  inputs), anchorHref = #slug invariant, global collision
  resolution (within doc, across nesting levels, across
  different-text-same-slug pairs), text extraction from mixed
  inline content (strong+code+text, empty → section fallback,
  multiple empty → section-2/-3), nested container walks
  (blockquote / list / task_item / deep nesting), skip non-
  walkable blocks (paragraph/code_block/thematic_break/raw_block/
  table), defensive no-op for non-tree BlockNode variants,
  reproducibility (FM03 byte-identical output, no-mutation of
  input AST), security (script tags, NUL, attribute-breakout
  chars).

Coverage: **95.77% line / 95.74% branch**.  Uncovered lines are
TypeScript `never` exhaustiveness guards in switch defaults
that cannot fire at runtime (`autolink.ts` line 123-125,
`extract-text.ts` line 80-82).

### v0 simplifications (documented)

- **Annotations, not AST mutation** — see Behavioural notes.  A
  future v1 might extend `document-ast` with an optional `id`
  field on `HeadingNode`, but that's a coordinated cross-package
  change deferred from v0.
- **No anchor-text customisation.**  All anchors get `#slug`;
  no `prefix` option for namespaced docs.
- **No Unicode slug preservation.**  Non-ASCII text gets stripped
  (heading may reduce to `"section"`).  Matches GitHub; full
  Unicode support requires percent-encoding decisions deferred
  to v1.
- **No anchor-link visibility class / icon.**  Renderers decide
  how to present the self-link visually; this package emits only
  the slug + href.

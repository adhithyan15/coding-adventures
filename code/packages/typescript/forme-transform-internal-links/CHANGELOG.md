# Changelog — @coding-adventures/forme-transform-internal-links

## 0.1.0 — 2026-05-18

Initial release.  Eighth FM00 v0 stage package — fourth concrete
§5.3 transform.  Walks a `DocumentNode` and rewrites every
internal `LinkNode.destination` (root-relative `/slug` references)
to caller-supplied canonical URLs, with strict validation of the
resolver's output to defend against malicious or buggy
resolvers.

Sits alongside the rest of the FM00 v0 stage cluster.

### Added

- `rewriteInternalLinks(doc, resolver, options?): DocumentNode`
  — main entry.  Walks the document, calls the resolver on
  every internal link, validates resolved URLs, returns a fresh
  document copy.  Input never mutated.
- `isInternalSlug(url): boolean` — exposed sub-helper for
  callers wanting the same internal-link detection logic in
  their own code paths.
- `assertResolvedUrl(url): asserts url is string` — exposed
  sub-helper.  Throws `TypeError` if the URL is not in the
  http(s)://-or-root-relative accept set.
- `SlugResolver`, `UnresolvedPolicy`, `InternalLinksOptions`
  types.

### Spec adherence

Implements FM00 v0 §5.3 `transform-internal-links`.  No spec
divergences.

### Behavioural notes

- **Internal-link detection.**  A `LinkNode.destination`
  counts as internal iff it starts with exactly one `/` (single
  slash, not protocol-relative `//`) and is a non-empty
  string.  External (`http(s)://`, `mailto:`, etc.) and
  fragment-only (`#section`) links pass through unchanged.
- **Resolver contract.**  `(slug: string) => string | null`.
  Must be pure and synchronous.  `null` / `undefined` mean
  "unresolved".
- **Resolved-URL validation.**  Every resolver-returned string
  is checked against the `http(s)://` (case-insensitive) or
  root-relative `/path` accept set.  Throws `TypeError` for
  `javascript:`, `data:`, `file:`, `vbscript:`, protocol-
  relative `//`, bare relative, empty string, non-string.
  This is defence-in-depth: even if a buggy resolver returns
  an XSS payload for an innocent slug, the rewriter refuses
  to splice it into the AST.
- **Unresolved-link policy.**  Three options:
    - `"keep"` (default) — preserve original `/slug`.  Most
      forgiving for incremental authoring.
    - `"strip"` — replace the `LinkNode` with its inline
      children (drop the wrapper).
    - `"throw"` — throw `Error` with slug in message.  For
      pre-publish validation pipelines.
- **Walks nested containers.**  Headings, blockquotes, lists,
  task items, table cells, emphasis / strong / strikethrough
  wrappers — internal links inside any of these are rewritten.
- **Pass-through nodes.**  `ImageNode.destination` (image
  rewrite is a separate transform), `AutolinkNode.destination`
  (user's explicit external URL), `CodeBlock` / `CodeSpan` /
  `RawBlock` / `RawInline` values (would corrupt source /
  output if rewritten).
- **Fresh tree per call.**  Even passthrough nodes are
  re-allocated, so the output guarantee "no shared references
  with input" holds uniformly.
- **Resolver called exactly once per internal LinkNode.**  No
  quadratic re-lookups; cost is O(N) in document size.

### Security posture

Four concerns explicitly addressed (pre-push review):

- **Hostile resolver output.**  Validation of resolver return
  values against the http(s)-or-root-relative accept-list is
  the security chokepoint.  Tests pin every forbidden form
  (javascript:, data:, file:, vbscript:, protocol-relative,
  bare relative, empty, non-string).  An XSS payload returned
  by a buggy resolver never reaches the rendered HTML.
- **No AST mutation.**  Input `DocumentNode` never modified.
  Fresh-tree tests confirm output `!== input` at every level.
- **Deterministic.**  Single forward walk, no Map/Set iteration
  affecting output, no randomness.  Same input + pure resolver
  → byte-identical output.
- **Bounded computation.**  O(N) walk; resolver called once
  per internal LinkNode.  No regex used (URL detection is
  character-class checks via `string[i]` indexing) — zero
  ReDoS surface.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

72 tests across 2 files:

- `url.test.ts` (31) — `isInternalSlug` accept set (root-
  relative paths, bare `/`) and reject set (absolute http(s),
  protocol-relative, bare relative, `./about`, `mailto:`,
  `javascript:`, empty, non-string, fragment-only);
  `assertResolvedUrl` accept set (http://, https://,
  case-insensitive scheme, port + query + fragment, root-
  relative) and reject set (javascript:, data:, file:,
  vbscript:, protocol-relative, bare relative, mailto:,
  empty / null / undefined / number with descriptive error
  messages, long URL truncation in error).
- `walk.test.ts` (41) — internal link resolution (basic,
  title preservation, bare /), external pass-through,
  resolver-NOT-called for external links, unresolved policy
  matrix (keep default + explicit, strip with single + multi-
  child expansion, throw with slug in message, undefined =
  null), resolver-returned-URL validation rejecting each
  forbidden form, walks every nested container (blockquote,
  list / list_item, task_item, heading, table cells with
  header + body, emphasis / strong / strikethrough, nested
  DocumentNode), pass-through (image, autolink, code_block,
  code_span, raw_block, raw_inline, thematic_break, breaks),
  defensive non-tree BlockNode variants as direct siblings,
  purity (no input mutation, fresh tree, byte-identical
  output across calls, resolver-called-once-per-link).

Coverage: **97.1% line / 97.89% branch** across all source
files with logic.  Uncovered lines are TypeScript `never`
exhaustiveness guards (`walk.ts` 138-141, 210-213) that cannot
fire at runtime.

### v0 simplifications (documented)

- **No image-src rewriting.**  Image destinations pass through;
  `transform-image-rewrite` is a separate spec transform.
- **No internal-link predicate customisation.**  "Internal"
  hardcoded to "starts with `/` but not `//`".  Custom
  predicates deferred to v1.
- **No async resolver support.**  Resolvers are synchronous.
  Manifest lookup is in-memory anyway.
- **No batch / multi-document optimisation.**  Each call
  re-walks the document.  Pipelines share the resolver but
  not the walk.

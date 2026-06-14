# Changelog — @coding-adventures/forme-doc-toc-extractor

## 0.1.0 — 2026-05-21

Initial release.  Third concrete DOC00 v0 package — walk a
`DocumentNode` AST and produce a nested table-of-contents tree.
Output is a plain JSON-able shape (`{ text, id, level, children }`)
that a sidebar widget, in-page TOC, or PDF outline pane can render
directly.

Pure transform: `DocumentNode` → `{ toc, document, anchors }`.
Stack-based nesting algorithm, O(N) time, O(depth) space.

### Added

- `extractToc(doc): TocResult` — main entry.  Internally calls
  `generateHeadingAnchors(doc)` from
  `@coding-adventures/forme-doc-heading-anchors`, then nests the
  resulting flat anchor list into a hierarchical tree using a
  classic stack-based algorithm.  Returns `{ toc, document, anchors }`
  so downstream consumers (HTML renderer, sidebar, sequential
  search-index builder) all get their preferred projection without
  re-walking.
- `buildTocTree(anchors): readonly TocEntry[]` — standalone pure
  list-to-tree helper, exported for callers who already have an
  anchors list and just want the nesting.
- `TocEntry` type — `{ text, id, level, children }`, all
  `readonly`, all primitive/string except the recursive `children`
  array.  JSON-safe.
- `TocResult` type — `{ toc, document, anchors }`.

### Spec adherence

Implements DOC00 v0's `forme-doc-toc-extractor` per
`code/specs/DOC00-docs-vision.md`:

> Walk a `DocumentNode` AST → table of contents tree (heading text +
> slug + depth).  Output is a plain JSON-able structure the sidebar /
> in-page TOC widget can render.

Spec adherence is exact — text + slug (`id`) + depth (`level`) +
recursive children, all readonly, all JSON-serialisable.

### Algorithm

Classic stack-based nesting (textbook):

```
stack = [virtual root with level=0]
for heading h in source order:
    while stack.top.level >= h.level: pop
    create entry { ...h, children: [] }
    append to stack.top.children
    push entry onto stack
return root.children
```

O(N) total time — each heading pushed and popped exactly once.
O(depth) space.  Handles:

- Skipped levels (`# A` → `### C` with no h2) → C nests under A.
- Multi-level drops (`### C` → `# D`) → pops the stack to root.
- Multiple top-level h1s → no auto-grouping under a synthetic root.
- First heading at any depth → top-level, no synthetic parents.

### Behavioural notes

- **Pure transform.**  Input `DocumentNode` and its children are
  never mutated.  Verified by JSON snapshot.
- **Three consistent projections.**  `toc`, `document`, and
  `anchors` all share the same slugs and collision suffixes —
  they're derived from the same single pass.  Tested explicitly.
- **Null-prototype entries.**  TOC entries are built via
  `Object.create(null)` — defensive for sidebar code that iterates
  via `for...in` (no inherited `toString` etc. polluting the
  iteration).
- **Scalable.**  Tested with 10,000 headings (flat) and
  alternating deep/shallow (500 × `h1+h6` pairs); no stack
  overflow, no quadratic behaviour.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data manipulation.
- **No I/O** — capabilities `[]`.  No fs, network, env, shell.
- **Both transitive deps `[]`-capability** —
  `forme-doc-heading-anchors` (which uses a precompiled-grammar-free
  approach; its own walker is pure JS) and `document-ast` (type-only
  IR package).  No transitive capability cascades.
- **Output is null-prototype** — no shadowing of inherited
  accessors in case a heading is literally titled `__proto__`,
  `constructor`, etc. (the slug is just a string here; the heading
  text becomes `e.text` which is just a string value, not a key).

### Tests

21 tests in 1 file:

- `buildTocTree` (10) — degenerate inputs (empty, single heading),
  straightforward nesting (h1→h2→h3, two h2 siblings, multiple
  h1s), level jumps (h1→h3, h2→h4→h6, first heading at h3), level
  drops (h3→h1, h6→h2), id preservation, null-prototype output.
- `extractToc` (8) — empty doc, no-heading doc, realistic doc
  (headings + paragraphs interleaved), slug-id consistency between
  toc tree / anchored AST / flat anchors list, collision-suffix
  propagation across all three projections, immutability.
- Scalability (2) — 10k flat headings, 500 × alternating h1/h6.
- Determinism (1) — same input → identical output.

Coverage: **100% line / 100% branch / 100% function** across all
source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **Top-level headings only.**  `document-ast` v0 guarantees
  headings are top-level `BlockNode`s (not nested inside
  blockquotes or lists).  If GFM blockquote-wrapped headings ever
  land in the IR, both this and `heading-anchors` will need to
  recurse.
- **No min/max depth filtering.**  v0 emits every heading.
  Sidebar configs (e.g. "only show h2 and h3 in TOC") are a
  presentation-layer concern, not a parser concern.
- **No anchor renaming.**  Pandoc-style `{#custom-id}` suffixes
  are not honoured — see `forme-doc-heading-anchors` for the
  rationale.
- **No virtual root.**  Multiple h1s become multiple top-level
  TOC entries, not children of a synthetic root.  Sidebar widgets
  that want a single-rooted tree must wrap manually.

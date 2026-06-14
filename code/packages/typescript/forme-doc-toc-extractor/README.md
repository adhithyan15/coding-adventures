# @coding-adventures/forme-doc-toc-extractor

> Third DOC00 v0 package — walk a `DocumentNode` AST and produce a
> nested table-of-contents tree. Output is a plain JSON-able shape
> a sidebar widget, in-page TOC script, or PDF outline pane can
> render directly.

Pure transform. Capabilities: `[]`. Depends on
`@coding-adventures/forme-doc-heading-anchors` (which depends on
`document-ast`) — both transitive deps are themselves `[]`.

## What it does

```ts
import { extractToc } from "@coding-adventures/forme-doc-toc-extractor";
import { parseCommonMark } from "@coding-adventures/commonmark-parser";

const doc = parseCommonMark(`
# Introduction
## Setup
### Prerequisites
### Install
## Quick start
# Reference
## API
`);

const { toc, document, anchors } = extractToc(doc);
// toc = [
//   { text: "Introduction", id: "introduction", level: 1, children: [
//     { text: "Setup", id: "setup", level: 2, children: [
//       { text: "Prerequisites", id: "prerequisites", level: 3, children: [] },
//       { text: "Install",       id: "install",       level: 3, children: [] },
//     ]},
//     { text: "Quick start", id: "quick-start", level: 2, children: [] },
//   ]},
//   { text: "Reference", id: "reference", level: 1, children: [
//     { text: "API", id: "api", level: 2, children: [] },
//   ]},
// ]
```

You get **all three projections** from a single walk so downstream
consumers don't have to re-traverse:

- **`toc`** — the hierarchical tree (sidebar / in-page TOC / PDF
  outline)
- **`document`** — the anchored AST (HTML renderer reads
  `heading.id` directly)
- **`anchors`** — the flat in-document-order list (any consumer that
  prefers sequential iteration over recursion — e.g. a search-index
  builder that emits one record per heading)

All three share the same slugs and collision suffixes — they're
derived from the same single source-order pass.

## Two public functions

| Function                                | Input                       | Returns                                | Use when                                                       |
|-----------------------------------------|-----------------------------|----------------------------------------|----------------------------------------------------------------|
| `extractToc(doc)`                       | `DocumentNode`              | `{ toc, document, anchors }`           | You have a raw doc and want everything in one call.            |
| `buildTocTree(anchors)`                 | `readonly HeadingAnchor[]`  | `readonly TocEntry[]`                  | You already ran `generateHeadingAnchors(doc)` separately.      |

`extractToc` internally calls `generateHeadingAnchors` then
`buildTocTree` — there's no logic duplication.

## Nesting algorithm

Stack-based, O(N) — each heading is pushed and popped at most
once:

1. Push a virtual root with `level=0`.
2. For each heading `h`:
   - While `stack.top.level >= h.level`: pop. (Anyone at our level
     or deeper is a sibling/descendant — they're done receiving
     children.)
   - Create a new TOC entry for `h` with empty `children[]`.
   - Append it to `stack.top.children`.
   - Push it onto the stack.
3. Return `root.children`.

This handles every edge case the spec needs:

| Source                          | Tree                                                 |
|---------------------------------|------------------------------------------------------|
| `# A\n## B\n### C`              | A → B → C (3-deep chain)                             |
| `# A\n## B\n## C`               | A → [B, C] (two h2 siblings under one h1)            |
| `# A\n# B\n# C`                 | [A, B, C] (three top-level h1s, no auto-grouping)    |
| `# A\n### Deep` (skip h2)       | A → Deep (h3 nests directly under h1)                |
| `### Deep first\n# Top later`   | [Deep, Top] (h3 alone, then h1 — both top-level)     |
| `# A\n## B\n### C\n# D`         | [A → B → C, D] (h3→h1 pops all the way to root)      |
| `## A\n###### Deep\n## B`       | [A → Deep, B] (h2→h6→h2 pops 4 levels in one go)     |

## Slug derivation

Delegated to `@coding-adventures/forme-doc-heading-anchors`. See
that package's README for the full GitHub-compatible algorithm
(lowercase, Unicode-letter/digit/`_`/`-`/space allowed, spaces →
hyphens, collisions → `-1`/`-2`/…).

The `id` field on every `TocEntry` is **byte-identical** to the
`id` field on the corresponding `AnchoredHeadingNode` in the
returned `document`. HTML renderers can use either as the source
of truth.

## Security posture

- **No `eval` / `new Function` / `JSON.parse` reviver.** Pure data
  manipulation.
- **No input mutation.** Verified by JSON snapshot test.
- **Output entries built via `Object.create(null)`** — no
  prototype chain, defensive for sidebar code that iterates via
  `for...in`.
- **No I/O.** Capabilities `[]`. Both transitive deps
  (`heading-anchors`, `document-ast`) also `[]`.
- **Deterministic.** Same input → identical output structure.
- **Scalable.** O(N) algorithm — tested with 10,000 headings and
  with adversarial alternating-depth patterns (500 × `h1+h6`
  pairs).

## Tests

21 tests in `extractor.test.ts`:
- **`buildTocTree` (10)** — degenerate inputs, straightforward
  nesting, level jumps (skip), level drops (multi-pop), id
  preservation, null-prototype output.
- **`extractToc` (8)** — empty doc, no-heading doc, realistic
  interleaved doc, slug-id consistency across all three
  projections, collision-suffix propagation, immutability.
- **Scalability (2)** — 10k headings, alternating deep/shallow.
- **Determinism (1)** — same input → identical output.

Coverage: **100% line / 100% branch / 100% function** on all
source files with logic (`types.ts` is type-only).

## How it fits in the stack

Third concrete DOC00 v0 package after `forme-doc-frontmatter` and
`forme-doc-heading-anchors`. Sits between heading-anchors and the
sidebar / page-shell renderers:

```
.md → frontmatter → commonmark-parser → heading-anchors → toc-extractor
                                                                ↓
                                                          { toc, document, anchors }
                                                                ↓
                                                    HTML renderer + sidebar widget
```

Next DOC00 v0 packages: `forme-doc-code-block-decorator`,
`forme-doc-syntax-highlighter`, `forme-doc-sidebar-builder`,
`forme-doc-page-shell`, `forme-doc-search-*`,
`forme-doc-site-emitter`.

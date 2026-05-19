# @coding-adventures/forme-transform-autolink-headings

Generate deterministic slug ids + self-link anchor metadata for
every `HeadingNode` in a `DocumentNode`.  FM00 v0 §5.3 transform.

Pure transform: walks a `DocumentNode` once, produces an ordered
`HeadingSlug[]` (one entry per heading in document order).
Renderers consume the annotation stream to emit

```html
<h2 id="my-slug"><a href="#my-slug" class="forme-anchor">Heading text</a></h2>
```

Fifth FM00 v0 stage package — sits alongside
[`forme-feeds`](../forme-feeds),
[`forme-opengraph`](../forme-opengraph),
[`forme-index-renderer`](../forme-index-renderer), and
[`forme-transforms`](../forme-transforms).

## Quick start

```ts
import { autolinkHeadings } from "@coding-adventures/forme-transform-autolink-headings";

const slugs = autolinkHeadings(doc);
for (const { level, text, slug, anchorHref } of slugs) {
  console.log(`h${level} → ${anchorHref}  (${text})`);
}
// h1 → #installation  (Installation)
// h2 → #setup  (Setup)
// h2 → #setup-2  (Setup)   ← second occurrence gets -2 suffix
```

## Why this package exists

Headings in Forme docs become deep-link targets — feeds reference
them, TOCs link to them, social cards anchor to them.  The IR
(`document-ast`) is immutable and doesn't carry an `id` on
`HeadingNode`, so this transform produces the parallel annotation
stream that downstream stages consume.

Two non-obvious design choices:

1. **Annotations, not AST mutation.**  Returns a `HeadingSlug[]`
   indexed by encounter order rather than modifying the document.
   This keeps the AST contract intact and makes the annotation
   stream JSON-serialisable for cross-process Forme deployments
   where parser and renderer run separately.
2. **Global collision resolution.**  All headings in the document
   participate in one collision namespace — `## Setup` followed by
   `### Setup` produces `setup` and `setup-2` regardless of
   nesting depth.  Matches GitHub's behaviour and prevents
   broken in-page links.

## API

### `autolinkHeadings(doc): HeadingSlug[]`

Walks `doc` depth-first in document order, finds every
`HeadingNode`, slugifies its plain-text content, resolves
collisions, and returns one annotation per heading.

```ts
interface HeadingSlug {
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly text: string;        // plain-text label
  readonly slug: string;        // [a-z0-9-]+, never empty
  readonly anchorHref: string;  // "#" + slug
}
```

### `slugify(text): string`

GitHub-flavoured slugification.  Algorithm:

1. Lowercase (locale-independent).
2. Strip ASCII control bytes (`U+0000-U+001F`, `U+007F`).
3. Strip everything except `[a-z0-9 -]`.
4. Collapse whitespace + hyphen runs into one hyphen.
5. Trim leading / trailing hyphens.
6. Empty result → `"section"` fallback.

Output guarantees:
- non-empty,
- matches `/^[a-z0-9-]+$/`,
- never begins or ends with `-`,
- contains no consecutive hyphens.

### `resolveCollisions(candidates): string[]`

Disambiguate a stream of slug candidates by appending `-2`, `-3`,
... to later occurrences.  First occurrence stays unsuffixed
(matches the most-likely link target for external references).
Skips already-taken numeric suffixes:

```
["setup", "setup-2", "setup"] → ["setup", "setup-2", "setup-3"]
                                                       ^ jumps past -2
```

### `extractText(inlines): string`

Flatten an `InlineNode[]` to plain text — used internally for slug
generation and useful for TOC labels.  Recurses into formatting
wrappers (emphasis, strong, link, etc.), uses image alt text,
treats breaks as single spaces, skips `raw_inline` (back-end-
specific).

## Behavioural contract

| Aspect                          | Behaviour                              |
|---------------------------------|----------------------------------------|
| Input AST                       | Never mutated                          |
| Output array length             | Exactly one entry per `HeadingNode`    |
| Order                           | Document-order DFS                     |
| Walks into                      | blockquote, list (item / task_item)    |
| Walks past (no nested blocks)   | paragraph, code_block, thematic_break, raw_block, table |
| Defensive no-op                 | document / list_item / task_item / table_row / table_cell as direct siblings (well-formed AST never has this) |
| Empty / non-heading document    | `[]`                                   |
| Slug always matches             | `/^[a-z0-9-]+$/`                       |
| `anchorHref` always equals      | `#${slug}`                             |

## Reproducibility (FM03)

Same `DocumentNode` → byte-identical `HeadingSlug[]`.  The
collision resolver is deterministic; slugify is deterministic;
the walk order is deterministic.  Safe to use as a cache key
input.

## Security posture

Three concerns explicitly addressed (pre-push review):

- **HTML injection via attacker-controlled heading text.**
  Heading text like `<script>alert(1)</script>` is stripped to
  `scriptalert1script` by `slugify` — the `[^a-z0-9 -]` strip
  pass removes angle brackets, quotes, ampersands, parens, equals
  signs.  Slugs are guaranteed safe to interpolate into HTML
  `id` attributes without escaping.
- **Attribute-breakout via quotes / equals.**  `"`, `'`, `=`, `<`,
  `>`, `&` all live outside `[a-z0-9 -]` and get stripped before
  the slug is ever emitted.
- **Control-byte smuggling.**  NUL (`\x00`), DEL (`\x7F`), and
  every other ASCII control character is stripped explicitly
  before the regex pass — defence-in-depth against a
  hypothetical parser that lets controls leak through into
  heading text.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.
Same posture as `forme-feeds` / `forme-opengraph` /
`forme-index-renderer` / `forme-transforms`.

## Tests

85 tests across 4 files:

- `slugify.test.ts` (25) — basic shape, fallback for empty /
  whitespace / punctuation / non-ASCII, security (control bytes,
  script tags, attribute-breakout chars), output guarantees
  (regex, non-empty, idempotent, deterministic).
- `collisions.test.ts` (15) — basic numbering, skip-taken-suffix
  semantics, non-contiguous gap behaviour, determinism, edge
  cases (empty / single, output length invariants).
- `extract-text.test.ts` (17) — every inline node kind, nested
  formatting wrappers, link/code/image/autolink handling,
  hard/soft break → space, `raw_inline` skipped, real-world
  heading shapes.
- `autolink.test.ts` (28) — end-to-end transform, document order
  preservation, level propagation, anchorHref = #slug, global
  collision resolution across nesting, text extraction from
  mixed inline content, walks blockquote / list / task_item /
  deep nesting, defensive no-op for non-tree BlockNode variants,
  reproducibility, no-mutation, security (script tags, NUL,
  attribute-breakout chars).

Coverage: **95.77% line / 95.74% branch**.  Uncovered lines are
TypeScript `never` exhaustiveness guards that cannot fire at
runtime.

## Spec adherence

Implements FM00 v0 §5.3 `transform-autolink-headings`.  No spec
divergences.  The spec calls for "add id + self-link to
headings"; this package produces the annotation stream that
renderers consume to emit exactly that markup.  The annotation
shape — list of `{ level, text, slug, anchorHref }` — is the
v0 contract.

## v0 simplifications

- **Annotations, not AST mutation** — see "Why this package
  exists".  A future v1 might extend `document-ast` with an
  optional `id` field on `HeadingNode`, but that's a coordinated
  change across every front-end / back-end and was deferred.
- **No anchor-text customisation.**  All anchors get `#slug`;
  there's no `prefix` option for namespaced docs (e.g. one big
  page with sections from multiple chapters).  Add an `options`
  arg in v1 if the need materialises.
- **No Unicode slug support.**  Non-ASCII text is stripped (the
  whole heading might reduce to `"section"`).  GitHub does the
  same; full Unicode support would require percent-encoding
  decisions deferred to v1.
- **No anchor-link visibility class.**  Renderers add their own
  `class="forme-anchor"` (or omit it) when consuming the
  `anchorHref` — this package only emits the slug + href, not
  the surrounding markup.

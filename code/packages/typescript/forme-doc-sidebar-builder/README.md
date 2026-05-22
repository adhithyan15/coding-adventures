# @coding-adventures/forme-doc-sidebar-builder

> Sixth DOC00 v0 package — take a directory layout (file paths) +
> each file's frontmatter and produce a hierarchical, JSON-able
> sidebar navigation tree the page-shell can render to HTML.

Pure transform. Capabilities: `[]`. **Zero runtime dependencies.**

## What it does

```ts
import { buildSidebar } from "@coding-adventures/forme-doc-sidebar-builder";

const sidebar = buildSidebar([
  { path: "intro.md",          frontmatter: { sidebar_position: 1 } },
  { path: "guide/index.md",    frontmatter: { title: "Guide", sidebar_position: 2 } },
  { path: "guide/setup.md",    frontmatter: { sidebar_position: 1 } },
  { path: "guide/api.md",      frontmatter: { sidebar_position: 2 } },
  { path: "advanced.md",       frontmatter: { draft: true } },  // skipped
]);
// sidebar = [
//   { kind: "page",  label: "Intro", path: "intro.md", position: 1 },
//   { kind: "group", label: "Guide", path: "guide/index.md", position: 2, children: [
//     { kind: "page", label: "Setup", path: "guide/setup.md", position: 1 },
//     { kind: "page", label: "API",   path: "guide/api.md",   position: 2 },
//   ]},
// ]
```

## Frontmatter keys consulted

The builder reads four well-known keys from each page's
frontmatter. **All other keys are ignored** — pages can carry
arbitrary user metadata without affecting the sidebar.

| Key                 | Type      | Effect                                                                                  |
|---------------------|-----------|-----------------------------------------------------------------------------------------|
| `title`             | `string`  | Display label for the entry. Falls back to humanised filename if absent.                |
| `sidebar_label`     | `string`  | Sidebar-specific label. Takes precedence over `title` when both are present.            |
| `sidebar_position`  | `number`  | Sort key within the directory (ascending). Missing / non-finite → sorts last.           |
| `draft`             | `boolean` | When strictly `true`, the page is omitted. A group with all-draft children disappears.  |

All reads use `Object.hasOwn(frontmatter, key)` so a frontmatter
literal like `{ __proto__: { title: "Polluted" } }` doesn't leak
inherited values into the output — defence-in-depth for callers
using arbitrary frontmatter sources. In practice
`@coding-adventures/forme-doc-frontmatter` returns null-prototype
objects anyway.

## Output shape

JSON-safe — only strings, numbers, booleans, nulls, and
arrays/objects of the same. `JSON.stringify`-cacheable.

```ts
type SidebarEntry = SidebarPageEntry | SidebarGroupEntry;

interface SidebarPageEntry {
  readonly kind: "page";
  readonly label: string;           // sidebar_label ?? title ?? humanised slug
  readonly path: string;            // original (non-normalised) path
  readonly position: number | null; // sidebar_position or null
}

interface SidebarGroupEntry {
  readonly kind: "group";
  readonly label: string;           // index page's label, or humanised dir name
  readonly path: string | null;     // index page's path, or null if no index
  readonly position: number | null; // index page's position, or null
  readonly children: readonly SidebarEntry[];
}
```

## Algorithm (three phases, O(N log N))

**Phase 1 — Filter + Normalise.** Drop drafts. Normalise each
remaining path:
- Strip leading slashes, trailing slashes, recognised extensions
  (`.md`, `.mdx`, `.html`, `.htm` — case-insensitive).
- Detect `index` (case-insensitive) as the last segment → mark
  as the directory's index page; otherwise the last segment is
  the file slug.
- Apply optional `root` prefix stripping (pages outside `root`
  are skipped).

**Phase 2 — Trie build.** Insert each normalised page into a
directory trie. Non-index pages go at leaves; index pages attach
to their directory node. Duplicate slugs at the same directory
or duplicate indexes throw `TypeError`.

**Phase 3 — Emit.** Recursively walk the trie depth-first. At
each level, sort by `(position ?? +Infinity, label)` — positioned
entries first by position ascending; unpositioned entries last,
alphabetical among themselves. Locale-independent string compare
for cross-machine stability.

## Index page handling

A `<dir>/index.md` (or `index.mdx` / `Index.md`, case-insensitive)
becomes the **destination** of the group, not a separate page
entry:

| Input                                    | Effect                                                                      |
|------------------------------------------|-----------------------------------------------------------------------------|
| `guide/index.md` only                    | Group "Guide" with `path: "guide/index.md"`, no children.                   |
| `guide/index.md` + `guide/setup.md`      | Group "Guide" with `path: "guide/index.md"`, one child "Setup".             |
| `guide/setup.md` only (no index)         | Group "Guide" with `path: null`, one child "Setup".                          |
| Two index pages for same directory       | Throws `TypeError`.                                                          |

The group's `label`, `path`, and `position` are inherited from
the index page's frontmatter. Sidebar widgets typically make
the group label clickable in that case.

## Acronym-aware humanisation

Directory and filename slugs are humanised via a small alias
table so `api` becomes `"API"`, not `"Api"`. The table covers
HTTP / web / cloud / language acronyms:

| Slug           | Label         |
|----------------|---------------|
| `api`          | `API`         |
| `sdk`          | `SDK`         |
| `url`          | `URL`         |
| `http`         | `HTTP`        |
| `https`        | `HTTPS`       |
| `json`         | `JSON`        |
| `yaml`         | `YAML`        |
| `html`         | `HTML`        |
| `css`          | `CSS`         |
| `cli`          | `CLI`         |
| `gpu` / `cpu`  | `GPU` / `CPU` |
| `io`           | `I/O`         |
| ... (~40 more) |               |

Unknown words fall back to first-letter-uppercase, rest-lower
(`getting-started` → `"Getting Started"`).

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver.**
  Pure data construction.
- **No mutation of input.** Verified by JSON snapshot test —
  input array, input frontmatter objects, all unchanged.
- **Output is JSON-safe.** No AST refs, no `Date`s, no
  symbols. Tested via `JSON.parse(JSON.stringify(result))`.
- **`Object.hasOwn` on every frontmatter read.** Defends against
  hand-crafted `{ __proto__: { … } }` inputs that would otherwise
  leak prototype-chain values into the sidebar.
- **Acronym table built via `Object.create(null)`.** Directory
  named `__proto__` falls through to default capitalisation
  rather than reading `Object.prototype.__proto__`.
- **64-level directory-depth cap.** Adversarial deeply-nested
  inputs (e.g. `a/b/c/…/z.md` with 10k segments) would
  otherwise trigger `RangeError` during the recursive `emit`
  walk. The cap throws a clear `TypeError` instead. Real docs
  sites never exceed ~8 levels.
- **No I/O.** Capabilities `[]`. **Zero runtime dependencies.**
- **Deterministic.** Same input → identical output bytes.
- **Total ordering.** Sort comparator is a total order (numeric
  then alphabetical); `Array.prototype.sort` is stable in V8.

## Tests

97 tests across three files:

- `labels.test.ts` (23) — humanise() coverage: basic kebab/snake,
  acronyms (API/SDK/HTTP/etc.), case-insensitive acronym lookup,
  edge cases (empty, only-separators, only-whitespace,
  `__proto__` falls through).
- `path-utils.test.ts` (31) — extension stripping (`.md` / `.mdx`
  / `.html` / `.htm`, case-insensitive, non-doc extensions left
  alone), slash handling (leading / trailing / multiple / empty
  segments), index detection (root / nested / case-insensitive /
  not-index variants), root-prefix stripping (matching /
  non-matching / multi-part / leading slash / whitespace),
  empty-path errors, `"/"`-style stripped-to-empty cases.
- `builder.test.ts` (43) — degenerate inputs, ordering (position
  ascending, alphabetical fallback, positioned-before-unpositioned,
  tie-breaks), labels (title / sidebar_label / fallthrough /
  non-string ignored / acronyms), drafts (skipped / draft-false
  not skipped / string "true" not draft / group collapses when
  empty), grouping (single-level / mixed root pages and groups /
  deeply nested), index pages (group destination / null path /
  not listed in children / root index / duplicate throws),
  duplicate detection, root-prefix option, frontmatter robustness
  (non-numeric position / NaN / Infinity / negative / unknown
  keys / `__proto__` defence), determinism, immutability,
  JSON-safety, realistic-doc scenario.

Coverage: **100% line / 100% branch / 100% function** on every
source file with logic (`types.ts` is type-only).

## How it fits in the stack

Sixth concrete DOC00 v0 package. Sits at the site-structure
layer, consuming the per-page frontmatter from
`forme-doc-frontmatter` and emitting the structural tree for the
page-shell:

```
.md files ─┬─► frontmatter (per file) ──────────┐
           └─► commonmark-parser → headings ──┐ │
                                              │ │
                                              ▼ ▼
                                       sidebar-builder ────► sidebar tree
                                                                  ↓
                                                            page-shell renders HTML
```

Next DOC00 v0 packages: `forme-doc-page-shell`,
`forme-doc-search-tokenizer`, `forme-doc-search-index-builder`,
`forme-doc-search-client-js`, `forme-doc-site-emitter`.

## v0 simplifications (documented)

- **No multi-sidebar support.** v0 produces ONE tree. Sites that
  want separate sidebars per section (e.g. Docusaurus's
  `docs.sidebars.js`) wrap the function and pass filtered page
  subsets.
- **No `category.json`-style explicit group metadata.** Group
  labels come from `index.md`'s frontmatter or from the
  humanised directory name. v1 can add explicit metadata
  injection.
- **Root index page's metadata is unsurfaced.** A root `index.md`
  contributes to the implicit root group's metadata, but since
  we return `root.children`, the root group itself isn't visible
  in the output. Use a top-level `intro.md` if you want a
  navigable root entry.
- **No collapsed/expanded state.** The output is structural only;
  collapse state is the page-shell's concern.

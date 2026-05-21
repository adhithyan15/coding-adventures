# Changelog — @coding-adventures/forme-doc-heading-anchors

## 0.1.0 — 2026-05-21

Initial release.  Second concrete DOC00 v0 package — walk a
`DocumentNode` AST, generate a URL-safe slug ID for every heading,
and return a new tree with the slugs attached plus a flat anchors
list for downstream consumers.

Pure transform: `DocumentNode` → `{ document: DocumentNode, anchors:
HeadingAnchor[] }`.  Deterministic GitHub-style slug derivation; no
`eval`, no `new Function`, no prototype pollution.

### Added

- `generateHeadingAnchors(doc): HeadingAnchorsResult` — main entry.
  Walks every top-level block of the input document; for each
  `HeadingNode`, computes the plain-text projection of its inline
  children, runs the GitHub slugifier over it, suffixes for
  in-document collisions, and emits a fresh `AnchoredHeadingNode`
  (structurally `HeadingNode` + `readonly id: string`) in the output
  tree.  Non-heading children pass through by reference.
- `slugify(text): string` — standalone exported function for callers
  who need to slugify outside the walker (e.g. to derive cross-doc
  anchors).  Pure, deterministic, locale-independent.
- `AnchoredHeadingNode` — `HeadingNode & { readonly id: string }`.
- `HeadingAnchor` — flat list entry `{ text, id, level, heading }`.
- `HeadingAnchorsResult` — `{ document, anchors }`.

### Spec adherence

Implements DOC00 v0's `forme-doc-heading-anchors` per
`code/specs/DOC00-docs-vision.md`:

> Walk a `DocumentNode` AST; for every heading, generate a URL-safe
> slug ID and inject it as a heading attribute.  Deterministic slug
> derivation (no random suffix); collisions within one document get
> `-2`, `-3`, etc. suffixes.

**Spec divergence (intentional, noted):** the spec says collisions
get `-2`, `-3` suffixes; we follow GitHub instead, which suffixes
the FIRST collision as `-1` (the original keeps the bare slug).
The spec wording was approximate — the important property is
determinism and that the first occurrence keeps the canonical link.
Matching GitHub minimises surprise for authors and minimises
inbound-link breakage when sites migrate from GitHub Pages or
similar.

### Slug algorithm

GitHub-compatible:
1. Extract plain text from inline children (markup elided, line
   breaks → space, images → alt, autolinks → destination).
2. Lowercase via Unicode default case-folding (locale-independent —
   `'I'` → `'i'`, NOT Turkish `'ı'`).
3. Strip anything that isn't a Unicode letter, Unicode number,
   underscore, hyphen, or ASCII space.
4. Map spaces to hyphens 1:1 (no run-collapsing).

### Behavioural notes

- **Pure transform** — input AST is never mutated.  Output
  `DocumentNode` is a new object; non-heading children are shared by
  reference (safe under document-ast's readonly contract).
- **Deterministic** — same input bytes → identical output bytes.
  No `Date.now()`, `Math.random()`, or locale lookups.
- **In-document anchors only.**  Cross-document anchor resolution
  (`[See setup](other.md#setup)`) is a separate concern — handled by
  a later DOC00 package (`forme-doc-link-resolver` or similar).

### Security posture

- **No `eval` / `new Function` / `JSON.parse`** — pure string
  manipulation.
- **Output object via `Object.create(null)`** for the internal
  collision-counter map — a heading titled `__proto__` doesn't
  read the inherited `Object.prototype.__proto__` getter.  Tested.
- **Locale-independent case-folding** — `toLowerCase()`, not
  `toLocaleLowerCase()`.  Stable across machines.
- **No I/O** — capabilities `[]`.  No fs, network, env, shell.

### Tests

50 tests across 2 files:

- `slug.test.ts` (22) — happy paths (simple ASCII, multi-word,
  underscores, hyphens, digits); punctuation stripping (commas,
  question marks, dots, parens, brackets, slashes, emoji); Unicode
  (Chinese, Cyrillic, Greek, accented Latin); edge cases (empty,
  all-punctuation, whitespace-only, leading/trailing spaces, runs
  of spaces, Turkish `I` case-folding stability).
- `walker.test.ts` (28) — basic happy paths (empty doc, no-heading
  doc, single heading, multiple headings in order); every inline
  node type's plain-text contribution (text, emphasis, strong,
  strikethrough, code_span, link, image, autolink, raw_inline,
  soft_break, hard_break, nested markup); collision suffixing
  (two identical, three identical, case-insensitive, empty slugs,
  non-colliding); prototype-pollution defence (`__proto__`,
  `constructor`, `toString`); immutability (no input mutation, new
  output object); determinism (same input → identical output).

Coverage: **100% line / 100% branch / 100% function** across all
source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **Top-level headings only.**  document-ast's type system
  guarantees headings are always top-level `BlockNode`s in v0 — they
  don't nest inside blockquotes or lists in the IR.  If GFM
  blockquote-wrapped headings ever land in the IR, the walker will
  need to recurse.
- **No anchor renaming hints.**  We don't read a hypothetical
  `{#custom-id}` suffix on heading text (Pandoc-style explicit
  IDs).  Authors who need a non-default anchor must add a separate
  rename layer.
- **No cross-document collision resolution.**  Two pages can both
  have a `#setup` anchor — that's the caller's problem.

# @coding-adventures/forme-doc-heading-anchors

> Second DOC00 v0 package — walk a `DocumentNode` AST, generate a
> URL-safe slug ID for every heading, and return a new tree with the
> slugs attached + a flat anchors list for downstream consumers (TOC
> builder, deep-link renderer, sidebar widget).

Pure transform. Capabilities: `[]`. Deterministic GitHub-style
slugs, no `eval`, no `new Function`, no prototype pollution.

## What it does

```ts
import { generateHeadingAnchors } from "@coding-adventures/forme-doc-heading-anchors";
import { parseCommonMark } from "@coding-adventures/commonmark-parser";

const doc = parseCommonMark(`
# Getting Started

## API Reference

## API Reference

## Setup
`);

const { document, anchors } = generateHeadingAnchors(doc);
// anchors = [
//   { text: "Getting Started", id: "getting-started", level: 1, heading: <node> },
//   { text: "API Reference",   id: "api-reference",   level: 2, heading: <node> },
//   { text: "API Reference",   id: "api-reference-1", level: 2, heading: <node> },
//   { text: "Setup",           id: "setup",           level: 2, heading: <node> },
// ]
```

The `document` is a new `DocumentNode` whose heading children are
`AnchoredHeadingNode`s (structurally `HeadingNode` + `readonly id:
string`). Non-heading children pass through by reference — every
node in `@coding-adventures/document-ast` is `readonly`, so sharing
is safe.

The `anchors` list is in document order. A TOC builder can walk it
directly without re-traversing the AST.

## Slug algorithm (GitHub-compatible)

1. **Extract plain text** from the heading's inline children — text
   nodes contribute their value, emphasis / strong / strikethrough /
   links flatten into their children, code spans contribute their
   raw value, images contribute their alt text, autolinks contribute
   their destination, line breaks become single spaces, and
   `raw_inline` HTML / LaTeX fragments are skipped.
2. **Lowercase** using Unicode default case-folding (NOT
   locale-dependent — `'I'` → `'i'`, not Turkish `'ı'`).
3. **Strip** anything that isn't a Unicode letter (`\p{L}`), Unicode
   number (`\p{N}`), underscore, hyphen, or ASCII space.
4. **Replace** spaces with hyphens (1:1, no run-collapsing — `# A   B`
   becomes `a---b`).

Worked examples:

| Heading                       | Plain text         | Slug              |
|-------------------------------|--------------------|-------------------|
| `# Getting Started`           | `Getting Started`  | `getting-started` |
| `## Hello, World!`            | `Hello, World!`    | `hello-world`     |
| `## *Bold* header`            | `Bold header`      | `bold-header`     |
| `## \`foo()\` rules`          | `foo() rules`      | `foo-rules`       |
| `## [Link](url) text`         | `Link text`        | `link-text`       |
| `## 中文标题`                 | `中文标题`         | `中文标题`        |
| `## Café résumé`              | `Café résumé`      | `café-résumé`     |
| `## v2.0 — Release notes`     | `v2.0 — Release notes` | `v20--release-notes` |
| `## !@#$%`                    | `!@#$%`            | `` (empty) ``     |

## Collision suffixing

Two `# Setup` headings in the same document both want `setup`. We
match GitHub: the first keeps the bare slug, the second gets `-1`,
the third `-2`, etc.

```
## Setup     → id="setup"
## Setup     → id="setup-1"
## Setup     → id="setup-2"
```

Empty slugs (`# !@#$%` → `""`) collide on the empty string the same
way: `""`, `-1`, `-2`. We don't substitute a placeholder like
`"section"` because we want to surface "your heading has no
slug-eligible content" to the docs author as a weird-looking link.

## Security posture

- **No `eval` / `new Function`.** Pure string manipulation.
- **No `JSON.parse`.** No untrusted input → object conversion.
- **No mutation.** The input `DocumentNode` and its children are
  never written to — both the readonly contract and our tests
  enforce this.
- **Prototype-pollution defence.** The internal slug-collision
  counter is built via `Object.create(null)` so a heading literally
  titled `__proto__` doesn't read the inherited
  `Object.prototype.__proto__` accessor. Tested explicitly.
- **Deterministic output.** Same input → identical output bytes.
  No `Date.now()`, `Math.random()`, locale-dependent case-folding,
  or environment lookups inside the transform.
- **No I/O.** No fs, network, env, shell.

## Tests

50 tests across two files:
- `slug.test.ts` (22) — happy paths, punctuation stripping, Unicode
  (Chinese, Cyrillic, Greek, accented Latin), edge cases (empty,
  whitespace-only, all-punctuation, Turkish `I` case-folding stability).
- `walker.test.ts` (28) — basic happy paths, every inline node type
  (text, emphasis, strong, strikethrough, code_span, link, image,
  autolink, raw_inline, soft_break, hard_break, nested markup),
  collision suffixing (incl. case-insensitive), prototype-pollution
  defence (`__proto__`, `constructor`, `toString`), immutability,
  determinism.

Coverage: **100% line / 100% branch / 100% function** on all source
files with logic (`types.ts` is type-only).

## Capabilities

`[]` — pure transform. No I/O, network, fs, shell, env.

## How it fits in the stack

Second concrete DOC00 v0 package after `forme-doc-frontmatter`. Sits
between the parser and the TOC / HTML renderer:

```
.md → frontmatter → commonmark-parser → heading-anchors → toc-extractor
                                              ↓
                                       HTML renderer
                                       (uses heading.id for <h*> id=)
```

Next DOC00 v0 packages: `forme-doc-toc-extractor` (consumes the
`anchors` list this package produces), `forme-doc-code-block-decorator`,
`forme-doc-syntax-highlighter`, `forme-doc-sidebar-builder`,
`forme-doc-page-shell`, `forme-doc-search-*`, `forme-doc-site-emitter`.

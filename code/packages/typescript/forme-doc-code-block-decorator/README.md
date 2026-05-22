# @coding-adventures/forme-doc-code-block-decorator

> Fourth DOC00 v0 package — walk a `DocumentNode` AST and decorate
> every fenced code block with the presentation metadata an HTML
> chrome / sidebar / static site renderer needs: copy-button hook,
> language label, filename badge, and optional line-number gutter
> flag.

Pure transform. Capabilities: `[]`. Depends only on
`@coding-adventures/document-ast` (also `[]`-capability).

## What it does

```ts
import { decorateCodeBlocks } from "@coding-adventures/forme-doc-code-block-decorator";
import { parseCommonMark } from "@coding-adventures/commonmark-parser";

const doc = parseCommonMark(
  "```ts\n" +
  "// file: src/auth.ts\n" +
  "export function login(user) { return user; }\n" +
  "```"
);

const decorated = decorateCodeBlocks(doc, { lineNumbers: true });
const block = decorated.children[0];
// block.type           = "code_block"
// block.language       = "ts"                              // unchanged
// block.value          = "export function login(user) { return user; }\n"  // hint line stripped
// block.copyable       = true
// block.languageLabel  = "TypeScript"
// block.filename       = "src/auth.ts"
// block.lineNumbers    = true
```

## Four decorations

| Field            | What it is                                                                                              |
|------------------|---------------------------------------------------------------------------------------------------------|
| `copyable`       | Always `true` in v0. HTML renderer surfaces a "Copy" button anchored to a `data-copyable` attribute.    |
| `languageLabel`  | Human-readable language name (`ts` → `"TypeScript"`, `py` → `"Python"`, etc.). 70+ aliases.             |
| `filename`       | Extracted from a `// file: foo.ts` first-line hint in any of six comment styles. Stripped from `value`. |
| `lineNumbers`    | `true` iff caller opted in via `decorateCodeBlocks(doc, { lineNumbers: true })`. Default `false`.       |

## Six filename-hint styles

The first non-blank line is scanned; if it matches one of these
six comment styles with a `file:` keyword, the captured filename
becomes `filename` and the entire line is stripped from `value`:

| Style                       | Example                                | Languages                                       |
|-----------------------------|----------------------------------------|-------------------------------------------------|
| `// file: …`                | `// file: src/auth.ts`                 | C, C++, Java, JS, TS, Rust, Go, Swift, Kotlin   |
| `# file: …`                 | `# file: app.py`                       | Python, Ruby, Bash, Perl, YAML, TOML, R, Make   |
| `-- file: …`                | `-- file: schema.sql`                  | SQL, Haskell, Lua, Elm, Ada                     |
| `% file: …`                 | `% file: paper.tex`                    | LaTeX, MATLAB, Erlang                           |
| `<!-- file: … -->`          | `<!-- file: index.html -->`            | HTML, XML, SVG, Markdown                        |
| `/* file: … */`             | `/* file: theme.css */`                | CSS, C-block-comment fallback                   |

The `file:` keyword is **case-insensitive** (`File:`, `FILE:`).
Optional whitespace around the keyword, colon, and comment
delimiters is tolerated. Trailing content after the filename on
line-comment styles is discarded. Hints that aren't on the first
line are ignored — keeps the rule simple.

## Container recursion

Code blocks nested inside blockquotes, lists, list items, and
task-list items all get decorated. The walker recurses into:

- `BlockquoteNode.children`
- `ListNode.children` (each `ListItemNode` / `TaskItemNode`)
- `ListItemNode.children` and `TaskItemNode.children`

Top-level blocks that aren't containers and aren't code blocks
(headings, paragraphs, thematic breaks, raw blocks, tables) pass
through by reference — every node in `document-ast` is `readonly`,
so reference-sharing is safe.

## Language alias table

70+ entries covering the v0 syntax-highlighter language set plus
common config / data formats authors drop into prose without
thinking. See `src/language-labels.ts` for the full table.

Unknown hints **pass through verbatim** (preserving the author's
capitalisation):

```ts
languageLabel("ts")        // "TypeScript"
languageLabel("Cobol")     // "Cobol"      (unknown — pass through)
languageLabel("my-dsl")    // "my-dsl"     (unknown — pass through)
languageLabel(null)        // null
languageLabel("")          // null
languageLabel("  ts  ")    // "TypeScript" (whitespace trimmed)
```

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver.** Pure
  string + regex manipulation.
- **No mutation of input AST.** Verified by JSON snapshot test.
  Containers are freshly allocated when descended into; leaves
  pass by reference.
- **Language alias table built via `Object.create(null)`.** A
  code block tagged ` ```__proto__ ` (or `constructor`, or
  `toString`) doesn't read the inherited `Object.prototype`
  accessor during lookup — it falls through to the raw string
  fallback, same as any other unknown hint.
- **Filename regex caps.** Each regex anchors `^…$` and uses
  bounded character classes (`[^\s]+` for the filename) — no
  catastrophic-backtracking patterns.
- **No I/O.** Capabilities `[]`. Both this package and its single
  transitive dep (`document-ast`) declare `[]`.
- **Deterministic.** Same input bytes → identical output bytes.

## Tests

77 tests across three files:

- `language-labels.test.ts` (33) — known-language lookups,
  case-insensitive lookup, fallthrough, null / empty / whitespace
  handling, prototype-pollution defence.
- `filename.test.ts` (21) — all six comment styles, case-insensitive
  `file:` keyword, whitespace tolerance, absolute/relative paths,
  CRLF defensive handling, no-hint cases, hint not on line 1,
  single-line code-block-that-IS-a-hint edge case, malformed
  HTML/C-block patterns.
- `decorator.test.ts` (23) — basic happy path, lineNumbers
  propagation, filename integration, container recursion
  (blockquote / list / task-item / nested), ordered-list metadata
  preservation, non-container non-code pass-through (headings,
  thematic break, raw block, table), defensive top-level
  list_item / task_item, immutability (JSON snapshot),
  determinism, realistic doc.

Coverage: **100% line / 100% branch / 100% function** on every
source file with logic (`types.ts` is type-only).

## How it fits in the stack

Fourth concrete DOC00 v0 package after `forme-doc-frontmatter`,
`forme-doc-heading-anchors`, and `forme-doc-toc-extractor`.
Sits between the parser and the syntax highlighter:

```
.md → frontmatter → commonmark-parser → heading-anchors → toc-extractor
                                                                ↓
                                              code-block-decorator (this package)
                                                                ↓
                                                    syntax-highlighter (next)
                                                                ↓
                                                    HTML renderer + sidebar
```

Next DOC00 v0 packages: `forme-doc-syntax-highlighter`,
`forme-doc-sidebar-builder`, `forme-doc-page-shell`,
`forme-doc-search-*`, `forme-doc-site-emitter`.

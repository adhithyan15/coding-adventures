# Changelog — @coding-adventures/forme-doc-code-block-decorator

## 0.1.0 — 2026-05-21

Initial release.  Fourth concrete DOC00 v0 package — walk a
`DocumentNode` AST and decorate every fenced code block with
presentation metadata: copy-button hook, language label, filename
badge, and optional line-number gutter flag.

Pure transform: `DocumentNode → DocumentNode` (with `CodeBlockNode`s
replaced by `DecoratedCodeBlockNode`s).  Capabilities `[]`.

### Added

- `decorateCodeBlocks(doc, options?): DocumentNode` — main entry.
  Walks every block, descends into blockquotes / lists / task
  items, replaces each `CodeBlockNode` with a
  `DecoratedCodeBlockNode` carrying four added fields:
    - `copyable: true` (always)
    - `languageLabel: string | null` (humanised — `ts` →
      `"TypeScript"`, `py` → `"Python"`, etc.)
    - `filename: string | null` (extracted from a first-line
      `// file: foo.ts` hint in any of six comment styles)
    - `lineNumbers: boolean` (from `options.lineNumbers`,
      default `false`)
- `extractFilenameHint(value): { filename, strippedValue }` —
  standalone exported helper for callers who want just the
  filename extraction without the full AST walk.
- `languageLabel(raw): string | null` — standalone exported alias
  lookup.  70+ entries covering DOC00 v0's syntax-highlighter
  language set + common config / data formats.  Unknown hints
  pass through verbatim.
- `DecoratedCodeBlockNode` type — extends `CodeBlockNode` with
  the four decoration fields.
- `DecorateOptions` type — `{ lineNumbers?: boolean }`.

### Spec adherence

Implements DOC00 v0's `forme-doc-code-block-decorator` per
`code/specs/DOC00-docs-vision.md`:

> AST transform that decorates fenced code blocks with:
> - A "copy" button hook (data attribute the JS shim attaches to).
> - A language label.
> - An optional filename badge (from `// file:foo.ts` style hints).
> - Line-number gutter markup if requested.

All four decorations implemented as documented.  Filename hint
extraction supports six comment styles (line `//`, hash `#`, SQL
`--`, LaTeX `%`, HTML `<!-- -->`, C-block `/* */`) to cover every
v0 language; spec said "// file: foo.ts style" — interpreted as
"any conventional comment-leader style", not "C-style only".

### Behavioural notes

- **Pure transform.**  Input AST never mutated.  Output document
  is freshly allocated; non-code, non-container leaves pass by
  reference (safe under document-ast's readonly contract).
- **Container recursion.**  Blockquotes, lists, list items, and
  task items recurse — code blocks nested inside any of those
  get decorated.  Non-container, non-code blocks (headings,
  paragraphs, thematic breaks, raw blocks, tables) pass through.
- **Filename hint extracted only from line 1.**  Authors who want
  a filename badge must put the hint as the first line.  Keeps
  the rule simple and unambiguous.
- **Line-number opt-in is global per call** (`{ lineNumbers: true }`
  applies to every block).  Per-block opt-in via in-source magic
  comments (`// linenos`) is intentionally deferred to v1 —
  opinions on the syntax differ across projects.
- **Deterministic** — same input bytes → identical output bytes.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure string + regex manipulation.
- **Language alias table is `Object.create(null)`** — a code
  block tagged ` ```__proto__ ` falls through to the raw-string
  fallback instead of reading `Object.prototype.__proto__`.
- **Filename regexes anchored `^…$`** with bounded character
  classes (`[^\s]+` for the filename capture) — no catastrophic
  backtracking patterns.
- **No I/O** — capabilities `[]`.  Single transitive dep
  (`document-ast`) also `[]`.

### Tests

77 tests across 3 files:

- `language-labels.test.ts` (33) — known-language lookups
  (TypeScript, JavaScript, Python, Ruby, Go, Rust, Bash, JSON,
  HTML, CSS, Markdown, YAML, TOML, SQL, C++, Dockerfile, …),
  case-insensitive lookup, fallthrough on unknown hints, null /
  empty / whitespace handling, prototype-pollution defence
  (`__proto__`, `constructor`, `toString`).
- `filename.test.ts` (21) — all six comment styles, case-insensitive
  `file:` keyword, whitespace tolerance, absolute and relative
  paths, CRLF defensive handling, no-hint cases, hint on line 2
  (not extracted), single-line code-block-that-IS-a-hint edge
  case, malformed HTML / C-block patterns.
- `decorator.test.ts` (23) — basic happy path, lineNumbers
  propagation, filename integration, container recursion
  (blockquote / list / task-item / nested), ordered-list metadata
  preservation, non-container non-code pass-through (headings,
  thematic break, raw block, table), defensive top-level
  list_item / task_item, immutability (JSON snapshot before/after),
  determinism, realistic doc with interleaved structure.

Coverage: **100% line / 100% branch / 100% function** across all
source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No per-block line-number opt-in.**  `lineNumbers` is set
  globally per call.  Per-block in-source magic comments deferred
  to v1.
- **No HTML escaping in `filename` or `languageLabel`.**  Those
  are plain strings; downstream HTML renderers must escape on
  emission (standard practice).
- **No language detection.**  If a code block has `language: null`,
  the label stays null and the renderer shows no chrome.
  Auto-detection from content is a separate concern (and a
  notoriously hard one — best handled by the syntax-highlighter
  package's heuristics, if at all).
- **No filename extraction from deeper lines.**  Author convention
  is line 1.  Multi-file code blocks (`// file: a.ts ... // file: b.ts`)
  not supported — split into separate code blocks.

# Changelog — @coding-adventures/forme-doc-syntax-highlighter

## 0.1.0 — 2026-05-21

Initial release.  Fifth concrete DOC00 v0 package — **v0
interface-first stub** for the documentation-site syntax
highlighter.  Walks a `DocumentNode` and attaches a
`highlighted: HighlightSpan[]` field to every code block.

**v0 doesn't actually highlight anything yet.**  Every code
block gets a single `plain` span covering its full text (or `[]`
for empty blocks).  The TYPE-LEVEL CONTRACT
(`HighlightSpan`, `HighlightedCodeBlockNode`, `TokenType`) is
FINAL — v1's real TextMate-grammar engine will populate the
spans richly without changing any signatures.

Pure transform.  Capabilities: `[]`.  Depends only on
`@coding-adventures/document-ast`.

### Added

- `highlightCodeBlocks(doc, options?): DocumentNode` — main
  entry.  Walks the AST, recurses into blockquotes / lists /
  list-items / task-items, replaces every `CodeBlockNode` with
  a `HighlightedCodeBlockNode` (input fields preserved via
  object spread, so decorator metadata rides through).
- `highlight(value, language?): HighlightSpan[]` — stand-alone
  helper.  Returns `[{ type: "plain", value }]` for non-empty
  input, `[]` for empty.
- `SUPPORTED_LANGUAGES: ReadonlySet<string>` — informational
  list of hints v1's engine will recognise.  v0 itself never
  branches on this set.
- `isSupportedLanguage(lang): boolean` — convenience boolean
  (case-insensitive, whitespace-tolerant, null-safe).
- `HighlightSpan` — `{ readonly type: TokenType; readonly value: string }`.
- `HighlightedCodeBlockNode` — `CodeBlockNode & { readonly highlighted: readonly HighlightSpan[] }`.
- `HighlightOptions` — `{ readonly theme?: string }`, reserved for v1.
- `TokenType` — string union: `plain` / `keyword` / `string` /
  `number` / `comment` / `operator` / `punctuation` / `identifier` /
  `function` / `type` / `constant` / `tag` / `attribute` / `regex`.

### Spec adherence

Implements DOC00 v0's `forme-doc-syntax-highlighter` per
`code/specs/DOC00-docs-vision.md`:

> **AOT (build-time) syntax highlighter.**  Themes are baked into
> the output HTML at build time; zero JS shipped to the browser
> for syntax colouring.  v0 supports a handful of languages:
> TypeScript, JavaScript, Python, Ruby, Go, Rust, Bash, JSON,
> HTML, CSS, Markdown.  More languages added as needed.
> The highlighter uses TextMate-style grammars (the same format
> VS Code uses) — these are well-documented, well-tested, and
> cover essentially every language anyone writes in practice.

**Spec divergence (intentional, documented):** v0 ships an
interface-first stub instead of a real TextMate-grammar engine.
Rationale:

1. A real engine is a v1-sized effort (~thousands of lines,
   per-language grammar bundles, theme system, scope-stack
   tokeniser).
2. Downstream consumers (HTML renderer, page-shell,
   site-emitter) all need to know "every code block has a
   `highlighted` field" but none of them care HOW the spans
   were derived.  Settling the TYPE CONTRACT today unblocks
   that work to proceed in parallel with the v1 engine.
3. The tiling invariant — `span.value` concatenation reconstructs
   the original `value` byte-for-byte — is preserved by both v0's
   stub (one span = whole block) and v1's planned engine.

v1 will swap the implementation without changing any signatures
or test contracts.  The `SUPPORTED_LANGUAGES` set documents the
target language list so v1's engine has a clear scope.

### Behavioural notes

- **Pure transform.**  Input AST never mutated; output document
  is freshly allocated; non-code, non-container leaves pass by
  reference (safe under document-ast's readonly contract).
- **Tiling invariant.**  For every code block, concatenating
  `span.value` across `highlighted` reconstructs the original
  `value` byte-for-byte.  v0 enforces trivially (one span);
  v1's engine must enforce via an exhaustive lexer.
- **Container recursion.**  Same shape as
  `forme-doc-code-block-decorator` — blockquotes, lists,
  list-items, task-items all recurse so nested code blocks are
  highlighted.
- **Composability.**  Object-spread in `decorate()` means any
  fields on the input code block (e.g. decorator's `copyable`,
  `languageLabel`, `filename`, `lineNumbers`) ride through to
  the output unchanged.  Test verifies this.
- **Deterministic.**  Same input bytes → identical output bytes.
- **Empty-block edge case.**  An empty `value` (`""`) gets
  `highlighted: []`, NOT `[{ type: "plain", value: "" }]`.
  Spans must always have non-empty `value` to satisfy v1's
  invariants cleanly.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data construction.
- **No mutation of input AST** — JSON-snapshot verified.
- **Capabilities `[]`** — no fs / network / env / shell.
  Single transitive dep (`document-ast`) is also `[]`.  v1's
  engine will also be `[]`-capability (TextMate grammars are
  static data, not code).
- **No unbounded recursion in v0.**  Walker recurses via the JS
  call stack (same as the other DOC00 walkers); adversarial
  10k+ nested blockquotes could trigger `RangeError`, but
  that's a parser-level concern (caps are `[]`; the package
  only processes whatever AST a trusting upstream hands it).
- **Composable object-spread** is safe — copies own enumerable
  properties of the source node; doesn't pollute the output
  object's prototype chain.

### Tests

34 tests across 2 files:

- `supported-languages.test.ts` (8) — eleven DOC00 v0 spec
  languages and aliases all present, case-insensitive lookup,
  whitespace tolerance, null/empty/unknown handling.
- `highlighter.test.ts` (26) — `highlight` stand-alone helper
  (non-empty, empty, language parameter ignored, whitespace
  preserved), `highlightCodeBlocks` top-level behaviour
  (empty doc, no-code pass-through, single block, multiple
  blocks, empty-block edge case, options.theme ignored),
  tiling-invariant fuzz across 6 input shapes (empty, single
  char, normal code, weird whitespace, Unicode/emoji,
  100-line code), container recursion (blockquote / list /
  task-item / deeply nested / defensive top-level list_item /
  task_item / non-code pass-through inside containers),
  ordered-list metadata preservation, pass-through types
  (headings / thematic break / raw block / table),
  composability with decorator fields (spread preserves
  copyable, languageLabel, filename, lineNumbers),
  immutability via JSON snapshot, determinism.

Coverage: **100% line / 100% branch / 100% function** across
all source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No actual syntax highlighting.**  Single `plain` span per
  block.  v1 ships the engine.
- **`SUPPORTED_LANGUAGES` is informational only.**  v0 highlights
  every block identically regardless of language; the set
  exists for downstream UI/build-report code.
- **`HighlightOptions.theme` is accepted but ignored.**  v1 will
  honour it.
- **No per-block opt-out.**  v0 always emits a `highlighted`
  field on every code block — even if it's a single plain span.
  v1 may add an opt-out for very large generated code blocks
  where the cost outweighs the value.

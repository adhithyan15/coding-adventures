# @coding-adventures/forme-doc-syntax-highlighter

> Fifth DOC00 v0 package — v0 **interface-first stub** for the
> documentation-site syntax highlighter. Walks a `DocumentNode`
> and attaches a `highlighted: HighlightSpan[]` field to every
> code block.

**v0 doesn't actually highlight anything yet.** Every block gets
a single `plain` span covering its full text. The TYPE-LEVEL
CONTRACT (`HighlightSpan`, `HighlightedCodeBlockNode`,
`TokenType`) is FINAL and downstream consumers (HTML renderer,
page-shell) can be built against it RIGHT NOW. v1 will swap in
the real TextMate-grammar engine without changing any signatures.

Pure transform. Capabilities: `[]`. Depends only on
`@coding-adventures/document-ast`.

## Why a stub in v0?

A real TextMate-style syntax highlighter — the kind VS Code,
Atom, and Sublime ship — is thousands of lines of code: a
non-trivial grammar interpreter (Oniguruma-style regex with
begin/end/while/captures/patterns), per-language grammar bundles
(TypeScript's is ~3000 lines of JSON), a theme system, and a
scope-stack tokeniser that handles nested constructs (a comment
inside a string inside a template literal). That's a v1-sized
effort.

What v0 ships is the stable type-level contract downstream
consumers can write against today. Building the HTML renderer,
the page-shell, the search-index emitter — they all need to know
"every code block has a `highlighted` field" but none of them
care HOW the spans were derived. With this stub in place, that
work can proceed in parallel with v1's engine.

## What it does

```ts
import { highlightCodeBlocks } from "@coding-adventures/forme-doc-syntax-highlighter";
import { parseCommonMark } from "@coding-adventures/commonmark-parser";

const doc = parseCommonMark("```ts\nconst x = 1;\n```");
const result = highlightCodeBlocks(doc);
const block = result.children[0];
// block.type        = "code_block"
// block.language    = "ts"
// block.value       = "const x = 1;\n"
// block.highlighted = [{ type: "plain", value: "const x = 1;\n" }]
//                     (v1 will emit a richer span sequence here)
```

The v0 output's `highlighted` field always tiles `value`
byte-for-byte (one span = the whole thing, or `[]` for empty
blocks). This is the **tiling invariant** the renderer relies on:
concatenating every `span.value` must reconstruct the original
code exactly. v1's real engine will honour the same invariant.

## Public API

| Export                       | Purpose                                                                                  |
|------------------------------|------------------------------------------------------------------------------------------|
| `highlightCodeBlocks(doc, options?)` | Walk a `DocumentNode`, attach `highlighted` to every code block. Recurses into containers. |
| `highlight(value, language?)` | Stand-alone helper — return spans for a raw string. `[{ type: "plain", value }]` in v0. |
| `SUPPORTED_LANGUAGES`        | `ReadonlySet<string>` of language hints v1's engine will recognise.                       |
| `isSupportedLanguage(lang)`  | Convenience boolean check (case-insensitive, whitespace-tolerant, null-safe).             |
| `HighlightSpan`              | `{ type: TokenType; value: string }`.                                                     |
| `HighlightedCodeBlockNode`   | `CodeBlockNode` + `readonly highlighted: readonly HighlightSpan[]`.                       |
| `HighlightOptions`           | `{ theme?: string }` — reserved for v1; v0 ignores.                                       |
| `TokenType`                  | String union: `plain` / `keyword` / `string` / `number` / `comment` / `operator` / `punctuation` / `identifier` / `function` / `type` / `constant` / `tag` / `attribute` / `regex`. |

## Container recursion

Code blocks nested inside blockquotes, lists, list items, or
task-list items all get highlighted. Same recursion shape as
`forme-doc-code-block-decorator` — keeps the two packages
behaviourally aligned so callers can chain them without surprises.

## Composability with the decorator

`HighlightedCodeBlockNode` extends `CodeBlockNode` on the TYPE
side. At runtime, the highlighter spreads the input node's own
enumerable properties into the output, so any additional fields
(like the decorator's `copyable` / `languageLabel` / `filename` /
`lineNumbers`) ride through unchanged.

```ts
import { decorateCodeBlocks } from "@coding-adventures/forme-doc-code-block-decorator";
import { highlightCodeBlocks } from "@coding-adventures/forme-doc-syntax-highlighter";

// Pipeline: decorate then highlight.
const out = highlightCodeBlocks(decorateCodeBlocks(doc));
// Every code block in `out.children` has BOTH the decorator fields
// AND the `highlighted` span sequence.
```

The opposite order (`decorateCodeBlocks(highlightCodeBlocks(doc))`)
also works — the decorator preserves all fields too.

## v0 supported-language list (informational)

`SUPPORTED_LANGUAGES` exposes the hint set v1's engine will
recognise — useful for UI / build-report code that wants to
surface "will be highlighted in v1" badges today:

- **TypeScript** — `ts`, `tsx`, `typescript`
- **JavaScript** — `js`, `jsx`, `javascript`, `mjs`, `cjs`
- **Python** — `py`, `python`
- **Ruby** — `rb`, `ruby`
- **Go** — `go`, `golang`
- **Rust** — `rs`, `rust`
- **Bash** — `sh`, `bash`, `shell`, `zsh`
- **JSON** — `json`
- **HTML** — `html`, `htm`
- **CSS** — `css`
- **Markdown** — `md`, `markdown`

The v0 stub itself never branches on this set — it highlights
every block identically (one `plain` span). The set exists
purely so downstream code can make UI decisions today.

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver.**
  Pure data construction.
- **No mutation of input AST.** Verified by JSON snapshot.
  Containers are freshly allocated; non-code leaves pass by
  reference (safe under document-ast's readonly contract).
- **Object spread preserves null-prototype.** The `decorate`
  helper uses `{ ...block, highlighted: … }`; this is value
  spread that doesn't pollute the output's prototype chain.
- **No I/O.** Capabilities `[]`. Single transitive dep
  (`document-ast`) is also `[]`. v1's TextMate engine will be
  pure-transform too — grammars are static data, not code.
- **Deterministic.** Same input bytes → identical output bytes.
- **No unbounded recursion in v0.** The walker recurses through
  containers via the JS call stack (same as the other DOC00
  packages); adversarial 10,000-level nested blockquotes could
  cause `RangeError`, but caps are `[]` — the package only
  processes whatever AST a trusting parser hands it.

## Tests

34 tests across two files:

- `supported-languages.test.ts` (8) — every spec language and
  alias present, case-insensitive lookup, whitespace tolerance,
  null/empty/unknown handling.
- `highlighter.test.ts` (26) — `highlight` stand-alone helper,
  empty-block edge case, options.theme accepted-and-ignored,
  tiling-invariant fuzz (6 input shapes), container recursion
  (blockquote / list / task-item / nested / defensive top-level),
  ordered-list metadata preservation, pass-through types,
  composability with decorator fields (object spread preserves
  them), immutability (JSON snapshot before/after), determinism.

Coverage: **100% line / 100% branch / 100% function** on every
source file with logic (`types.ts` is type-only).

## How it fits in the stack

Fifth concrete DOC00 v0 package after `forme-doc-frontmatter`,
`forme-doc-heading-anchors`, `forme-doc-toc-extractor`, and
`forme-doc-code-block-decorator`. Sits between the code-block
decorator and the HTML renderer:

```
.md → frontmatter → commonmark-parser → heading-anchors → toc-extractor
                                                                ↓
                                                  code-block-decorator
                                                                ↓
                                          syntax-highlighter (this package)
                                                                ↓
                                                    HTML renderer + sidebar
```

Next DOC00 v0 packages: `forme-doc-sidebar-builder`,
`forme-doc-page-shell`, `forme-doc-search-tokenizer`,
`forme-doc-search-index-builder`, `forme-doc-search-client-js`,
`forme-doc-site-emitter`.

## v1 roadmap

v1 keeps every signature in this package unchanged. The only
behavioural change: `highlightCodeBlocks` and `highlight` will
emit richer span sequences for supported languages. Specifically:

1. Bundle TextMate grammars for the eleven v0 spec languages.
2. Implement a scope-stack tokeniser (Oniguruma-compatible
   regex; could use a portable JS port).
3. Map TextMate scopes (e.g. `keyword.control.if.ts`) onto the
   coarse `TokenType` union — themes do the fine mapping.
4. Implement `HighlightOptions.theme` (resolve a theme name to
   a colour table; downstream renderer applies it).
5. Add `extensions` field to `HighlightedCodeBlockNode` for
   richer-than-`TokenType` info (e.g. full scope path) — purely
   additive, doesn't break v0 consumers.

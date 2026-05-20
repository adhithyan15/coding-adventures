# @coding-adventures/forme-doc-frontmatter

> First DOC00 v0 package — strip YAML or TOML frontmatter from a
> markdown source string. Returns `{ body, frontmatter, format }`.

Pure transform. Capabilities: `[]`. Tiny in-house YAML and TOML
parsers (no `eval`, no `new Function`, no prototype pollution).

## What it does

```ts
import { extractFrontmatter } from "@coding-adventures/forme-doc-frontmatter";

const md = `---
title: Hello
date: 2026-05-20
tags: [a, b]
---

# Body

Content here.`;

const { body, frontmatter, format } = extractFrontmatter(md);
// body        = "\n# Body\n\nContent here."
// frontmatter = { title: "Hello", date: "2026-05-20", tags: ["a", "b"] }
// format      = "yaml"
```

The `body` is then passed to `commonmark-parser` or `gfm-parser`
for HTML rendering. The `frontmatter` is consumed by
site-structure packages (sidebar position, title overrides,
draft flag, etc.).

## Supported frontmatter formats

| Format | Opening delim | Closing delim | Parser |
|--------|---------------|---------------|--------|
| YAML   | `---`         | `---`         | tiny in-house |
| TOML   | `+++`         | `+++`         | tiny in-house |
| none   | —             | —             | (passthrough)  |

Both delimiters must be on their own line. CRLF line endings
are handled. A UTF-8 BOM at the very start of the file is
silently stripped (some Windows editors add one).

If frontmatter delimiters are detected but the block is
malformed (unclosed delimiter, unparseable inner content),
the function throws `TypeError`. No partial output.

## Supported YAML / TOML subset

This is **not a general-purpose YAML or TOML parser.** It covers
exactly what documentation frontmatter uses in practice:

- Flat key/value maps (no nested tables / no inline tables).
- Scalar values: integers, floats, booleans, null (YAML only),
  strings (quoted or bare), RFC 3339 dates as strings.
- Arrays of scalars: inline `[a, b, c]` or YAML multi-line
  `- item` / `- item`.
- YAML quoted strings: single or double, escapes `\\` and the
  matching quote.
- TOML basic strings (`"..."` with `\n`/`\t`/`\r`/`\"`/`\\` escapes)
  and literal strings (`'...'`, no escapes).
- TOML inline comments (`# ...` after a value).

**Anything beyond this throws.** No nested tables, no anchors,
no custom tags, no multi-line strings, no array-of-tables, no
inline tables.

Full YAML is a security hazard (the spec includes tag
resolution that some implementations turn into code execution).
This subset is deliberately the minimal viable surface.

## Security posture

Six concerns explicitly addressed:

1. **No `eval` / `new Function`.** Both parsers walk the input
   character-by-character; nothing reaches a JS interpreter.
2. **No `JSON.parse`.** Even with a reviver, JSON.parse has
   subtle prototype-pollution interactions. We don't use it.
3. **Prototype-pollution defence (parser output).** Both
   parsers build their result via `Object.create(null)`, so
   the output has no `Object.prototype` link. Even if a key
   slipped past the next check, downstream consumers can't be
   tricked into reading inherited values.
4. **Prototype-pollution defence (key allowlist).** Keys
   literally named `__proto__`, `constructor`, or `prototype`
   are rejected outright in both parsers.
5. **Duplicate-key rejection.** Both parsers throw on
   duplicate top-level keys (catches typos + defends against
   a class of "last write wins" attacks).
6. **Strict reject on malformed input.** Indented continuation
   lines, structural characters in bare scalars, integer
   overflow past `Number.MAX_SAFE_INTEGER`, unrecognised
   escapes, unterminated quoted strings — all throw with
   line numbers.

## Tests

93 tests across three files:
- `yaml.test.ts` — scalar types, inline + multi-line arrays,
  multiple keys, comments, every security defence, every error
  path including escape handling and unterminated strings.
- `toml.test.ts` — same coverage shape for TOML.
- `extract.test.ts` — end-to-end: no frontmatter, YAML, TOML,
  CRLF, BOM, missing closing delim, input-type validation,
  determinism + no input mutation, real-world Hugo / Jekyll
  examples.

Coverage: **96.12% line / 96.15% branch** on all source files
with logic.

## Capabilities

`[]` — pure transform.

## How it fits in the stack

First concrete DOC00 v0 package. Sits at the very start of the
content pipeline:

```
.md file → extractFrontmatter → commonmark-parser → DocumentNode AST
                  ↓
              frontmatter (passes to sidebar-builder,
                           page-shell, search-index-builder)
```

Next DOC00 v0 packages: `forme-doc-heading-anchors`,
`forme-doc-toc-extractor`, `forme-doc-code-block-decorator`,
`forme-doc-syntax-highlighter`, `forme-doc-sidebar-builder`,
`forme-doc-page-shell`, `forme-doc-search-*`,
`forme-doc-site-emitter`.

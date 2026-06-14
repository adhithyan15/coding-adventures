# @coding-adventures/forme-doc-frontmatter

> First DOC00 v0 package — strip YAML or TOML frontmatter from a
> markdown source string. Returns `{ body, frontmatter, format }`.

Pure transform. Capabilities: `[]`. YAML uses a tiny in-house
subset parser; TOML delegates to the repo's full
`@coding-adventures/toml-parser` (lexer + grammar-driven parser
covering TOML 1.0) and walks the resulting AST to enforce the
docs-frontmatter subset. No `eval`, no `new Function`, no
prototype pollution.

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
| YAML   | `---`         | `---`         | tiny in-house subset parser |
| TOML   | `+++`         | `+++`         | `@coding-adventures/toml-parser` + AST walker |
| none   | —             | —             | (passthrough)  |

Both delimiters must be on their own line. CRLF line endings
are handled. A UTF-8 BOM at the very start of the file is
silently stripped (some Windows editors add one).

If frontmatter delimiters are detected but the block is
malformed (unclosed delimiter, unparseable inner content),
the function throws `TypeError`. No partial output.

## Supported YAML / TOML subset

For YAML this is **not a general-purpose parser** — it covers
the subset documentation frontmatter actually uses.  For TOML
the underlying parser IS general-purpose (full TOML 1.0), but
the AST walker rejects anything outside the docs subset before
returning a value.

**Supported, both formats:**

- Flat key/value maps (no nested tables / no inline tables).
- Scalar values: integers, floats, booleans, null (YAML only),
  strings (quoted or bare), RFC 3339 dates as strings.
- Arrays of scalars: inline `[a, b, c]` (both formats), or
  multi-line `- item` lists (YAML), or multi-line `[\n…\n]`
  with optional trailing comma (TOML).

**TOML-only details:**

- All four TOML string forms (`"…"`, `"""…"""`, `'…'`, `'''…'''`).
- All TOML integer bases (decimal, `0x…`, `0o…`, `0b…`) with
  underscore separators (`1_000_000`).
- All TOML float forms including `inf`, `nan`, scientific
  notation, and underscores.
- All four date/time tokens (returned as their RFC 3339 string
  — we never construct a `Date`).
- Inline `# comments` after values.
- Basic-string escapes: `\\`, `\"`, `\n`, `\t`, `\r`, `\b`,
  `\f`, `\/`, `\uXXXX`, `\UXXXXXXXX`.

**YAML-only details:**

- Single- and double-quoted strings with escapes `\\` and the
  matching quote.

**Rejected (throws `TypeError`):**

- Nested tables (`[server]` / `[a.b]`).
- Array-of-tables (`[[products]]`).
- Dotted keys (`a.b.c = 1`).
- Quoted keys (`"127.0.0.1" = 1`).
- Inline tables (`{x = 1, y = 2}`).
- Arrays-of-arrays / arrays-of-inline-tables.
- YAML anchors / aliases / custom tags.

Full YAML is a security hazard (the spec includes tag
resolution that some implementations turn into code execution),
so the YAML side stays tiny on purpose.  Full TOML is safe but
nested structures are out of scope for the v0 docs pipeline —
when DOC0X needs them we'll widen the subset.

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

135 tests across three files:
- `yaml.test.ts` (45) — scalar types, inline + multi-line
  arrays, multiple keys, comments, every security defence,
  every error path including escape handling and unterminated
  strings.
- `toml.test.ts` (71) — every supported scalar token (all four
  string forms, all four integer bases with underscores, every
  float form including `inf` / `nan`, every datetime token),
  single- and multi-line arrays, security defences, subset
  rejections (table headers, array-of-tables, dotted keys,
  quoted keys, inline tables, arrays-of-arrays, long bare
  keys), and surface-syntax errors that propagate from
  `@coding-adventures/toml-parser`.
- `extract.test.ts` (19) — end-to-end: no frontmatter, YAML,
  TOML, CRLF, BOM, missing closing delim, input-type
  validation, determinism + no input mutation, real-world Hugo
  / Jekyll examples.

Coverage: **98.88% line / 97.85% branch** on all source files
with logic.

## Capabilities

`[]` — pure transform.  `@coding-adventures/toml-parser` v0.1.1+
is pure-transform too (its grammar is precompiled at build time
into a TypeScript object literal, so no `fs:read` happens at
parse time), which keeps every downstream consumer's
capabilities at `[]`.

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

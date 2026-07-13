# javascript-tokens (C)

The shared JavaScript/TypeScript token vocabulary in pure ISO C17. A faithful
port of the Rust `javascript-tokens` crate.

This is the vocabulary every layer of a JS toolchain talks in — the lexer,
parser, AST, and tooling — without depending on any layer above:

- **`EsVersion`** — the ECMAScript editions with a grammar (`es1` … `es2025`),
  with string round-tripping and a chronological order.
- **`JsSpan`** — a half-open `[start, end)` byte range within one source file.
- **`JsTokenKind`** — the broad classification of a token (`Name`, `Number`,
  `Keyword`, …), plus an `Other` tag carrying a grammar-specific token name.

## API

```c
#include "javascript_tokens.h"

EsVersion v;
if (es_version_from_str("es2020", &v) == 0) { /* v == ES_ES2020 */ }
es_version_as_str(ES_ES2025);                 /* "es2025" */
ES_ES2015 < ES_ES2025;                        /* chronological (integer) order */

JsSpan s = js_span_new(10, 20);               /* len 10, not empty */
js_token_kind_is_trivia(js_token_kind(JS_TOK_COMMENT));  /* 1 */
```

`es_version_from_str` returns a status (the "unknown version" message is
formatted on demand by `es_version_unknown_message`). `JsTokenKind` is a plain
trivially-copyable value; the `Other` name is **borrowed** (the caller keeps it
alive — grammar token names are static in practice). The `EsVersion` enum values
are laid out chronologically, so a plain integer comparison is a chronological
comparison.

## Portability

Pure ISO C17 — no extensions. Compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```

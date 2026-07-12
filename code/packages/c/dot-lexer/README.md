# dot-lexer (C)

A tokeniser for the **Graphviz DOT** language in pure ISO C17. A faithful port
of the Rust `dot-lexer` crate.

`dot_tokenise` scans DOT source into a stream of tokens (always ending in an
`DOT_EOF` sentinel) plus a list of non-fatal errors — the lexer recovers after
an error by skipping the offending character, so a partial token stream is
always returned.

Token categories: the six case-insensitive keywords (`strict`, `graph`,
`digraph`, `node`, `edge`, `subgraph`), punctuation (`{ } [ ] = ; , :`), the
edge operators `->` and `--`, and `DOT_ID` for every identifier — unquoted word,
numeral, double-quoted string (quotes stripped, `\"` `\\` `\n` `\t` unescaped),
or HTML string (`<...>` with balanced angle brackets). Line/column are 1-based.

## API

```c
#include "dot_lexer.h"

DotLexResult *r = dot_tokenise("digraph G { A -> B }");
/* r->tokens[0].kind == DOT_DIGRAPH; r->tokens[1].value == "G"; ... */
for (size_t i = 0; i < r->nerrors; i++) {
    /* r->errors[i].message / .line / .col */
}
dot_lex_result_free(r);
```

`dot_tokenise` returns a malloc'd result (`dot_lex_result_free` releases it), or
NULL on allocation failure.

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```

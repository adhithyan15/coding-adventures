# dot-lexer (C++)

A tokeniser for the **Graphviz DOT** language in pure ISO C++17, header-only, in
namespace `ca::dot`. A faithful port of the Rust `dot-lexer` crate.

`tokenise` scans DOT source into a stream of tokens (always ending in an `Eof`
sentinel) plus a list of non-fatal errors — the lexer recovers after an error by
skipping the offending character.

Token categories: the six case-insensitive keywords (`strict`, `graph`,
`digraph`, `node`, `edge`, `subgraph`), punctuation (`{ } [ ] = ; , :`), the
edge operators `->` and `--`, and `Id` for identifiers (unquoted word, numeral,
double-quoted string with `\"` `\\` `\n` `\t` escapes, or balanced HTML string).
Line/column are 1-based.

## API

```cpp
#include "dot_lexer.hpp"

ca::dot::LexResult r = ca::dot::tokenise("digraph G { A -> B }");
// r.tokens[0].kind == ca::dot::TokenKind::Digraph; r.tokens[1].value == "G"
for (const auto& e : r.errors) { /* e.message / e.line / e.col */ }
```

`tokenise` returns a `LexResult { std::vector<Token> tokens; std::vector<LexError>
errors; }`.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```

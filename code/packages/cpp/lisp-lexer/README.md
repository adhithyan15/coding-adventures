# lisp-lexer (C++)

A **Lisp tokenizer**, **header-only** in pure ISO C++17 (namespace
`ca::lisp_lexer`). A faithful port of the Rust
[`lisp-lexer`](../../rust/lisp-lexer) crate.

## What it does

`tokenize` scans Lisp source text and returns a `std::vector<Token>` — the first
stage of a language pipeline. Lisp has only 7 meaningful token types (plus EOF):
`Number`, `Symbol`, `String`, `LParen`, `RParen`, `Quote`, `Dot`.

Whitespace and `;`-to-end-of-line comments are skipped. `-42` is one `Number`;
`-` before a non-digit is a `Symbol`. String values include the surrounding
quotes.

## API

- `std::vector<Token> tokenize(const std::string& source)` — throws
  `LexerError` (carrying a `position`) on an unrecognised construct. The vector
  always ends with an `Eof` token.
- `const char* token_type_name(TokenType)` — `"NUMBER"`, `"SYMBOL"`, ….

## Design notes

- **Exceptions.** Rust's `Result<Vec<Token>, LexerError>` becomes a
  `std::vector<Token>` returned normally, or a thrown `LexerError` (a
  `std::runtime_error`).
- **Bytes, not code points.** The Rust original scans a `Vec<char>`; this port
  scans bytes, so `position` is a byte offset — identical for any ASCII input.
- **Header-only.** `#include "lisp_lexer.hpp"` and go.

## Usage

```cpp
#include "lisp_lexer.hpp"
using namespace ca::lisp_lexer;

auto tokens = tokenize("(+ 1 2)");
// tokens[0].type == TokenType::LParen
// tokens[1] == Token{TokenType::Symbol, "+"}
// tokens.back().type == TokenType::Eof
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.

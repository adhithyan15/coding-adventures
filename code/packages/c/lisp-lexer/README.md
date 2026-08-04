# lisp-lexer (C)

A **Lisp tokenizer** in pure ISO C17. A faithful port of the Rust
[`lisp-lexer`](../../rust/lisp-lexer) crate.

## What it does

`ll_tokenize` scans Lisp source text and returns the stream of tokens — the
first stage of a language pipeline. Lisp has only 7 meaningful token types
(plus EOF), so the scanner is small and fast.

| Type     | Examples                          |
| -------- | --------------------------------- |
| `NUMBER` | `42`, `-7`, `0`                   |
| `SYMBOL` | `define`, `+`, `car`, `null?`     |
| `STRING` | `"hello"` (value includes quotes) |
| `LPAREN` / `RPAREN` | `(` / `)`              |
| `QUOTE`  | `'` (sugar for `(quote …)`)       |
| `DOT`    | `.` (dotted pairs)                |
| `EOF`    | always the last token             |

Whitespace and `;`-to-end-of-line comments are skipped. The one ambiguity —
`-42` (a number) vs `-` (a symbol) — is resolved by lookahead: `-` before a
digit is a number, otherwise a symbol.

## API

- `int ll_tokenize(const char *source, LlTokenList *out, LlError *err)` — returns
  1 on success (fills `*out`; release with `ll_token_list_free`), 0 on a lexing
  or allocation error (fills `*err`).
- `const char *ll_token_type_name(LlTokenType)` — `"NUMBER"`, `"SYMBOL"`, ….

## Design notes

- **Bytes, not code points.** The Rust original scans a `Vec<char>`; this port
  scans bytes, so `position` is a byte offset. Every token of interest is ASCII,
  so results are identical for any ASCII input; a stray non-ASCII byte falls
  through to the same "unexpected character" error.
- **Owned token list.** Each token's `value` is a malloc'd NUL-terminated copy;
  the growable token array is overflow-guarded and freed by
  `ll_token_list_free`.

## Usage

```c
#include "lisp_lexer.h"

LlTokenList list;
LlError err;
if (ll_tokenize("(+ 1 2)", &list, &err)) {
    /* list.tokens[0].type == LL_LPAREN, [1] == LL_SYMBOL ("+"), … ,
       last == LL_EOF */
    ll_token_list_free(&list);
} else {
    /* err.message, err.position */
}
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.

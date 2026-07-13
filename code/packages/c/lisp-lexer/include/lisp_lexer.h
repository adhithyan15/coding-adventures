/*
 * lisp_lexer.h — tokenize Lisp source into tokens, pure ISO C17.
 * =============================================================
 *
 * A faithful port of the Rust `lisp-lexer` crate. A lexer is the first stage of
 * a language pipeline: it breaks raw source text into a stream of typed tokens.
 * Lisp has only 7 meaningful token types (plus EOF), so the scanner is small.
 *
 *   Number   integer literal, possibly negative:  42  -7  0
 *   Symbol   identifier or operator name:          define  +  car  null?
 *   String   double-quoted, value includes quotes: "hello"
 *   LParen   (      RParen  )      Quote  '      Dot  .
 *   Eof      end of input — every token stream ends with exactly one
 *
 * Whitespace and `;`-to-end-of-line comments are skipped. The one ambiguity —
 * `-42` (a number) vs `-` (a symbol) — is resolved by lookahead: `-` followed
 * by a digit is a number, otherwise a symbol.
 *
 * ## Bytes vs code points
 *
 * The Rust original scans a `Vec<char>` (Unicode code points). This port scans
 * bytes, so `position` is a byte offset. Every token of interest is ASCII, so
 * results are identical for any ASCII input; a non-ASCII byte falls through to
 * the same "unexpected character" error the Rust version raises for the
 * corresponding code point.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef LISP_LEXER_H
#define LISP_LEXER_H

#include <stddef.h> /* size_t */

#ifdef __cplusplus
extern "C" {
#endif

/* The kinds of tokens Lisp source can contain. */
typedef enum {
    LL_NUMBER,
    LL_SYMBOL,
    LL_STRING,
    LL_LPAREN,
    LL_RPAREN,
    LL_QUOTE,
    LL_DOT,
    LL_EOF
} LlTokenType;

/* Uppercase name of a token type ("NUMBER", "SYMBOL", …, "EOF"). */
const char *ll_token_type_name(LlTokenType type);

/* A single token: its type and the original source text it came from. For EOF,
 * `value` is the empty string. `value` is an owned, NUL-terminated copy. */
typedef struct {
    LlTokenType type;
    char *value;
} LlToken;

/* The result of tokenizing: an owned array of tokens ending with one EOF. */
typedef struct {
    LlToken *tokens;
    size_t count;
} LlTokenList;

/* An error produced when the source contains an unrecognised construct. */
typedef struct {
    char message[64]; /* e.g. "Unexpected character: '@'" */
    size_t position;  /* byte offset where the error occurred */
} LlError;

/* Tokenize `source`. On success returns 1 and fills `*out` (release with
 * ll_token_list_free); the list always ends with an LL_EOF token. On a lexing
 * error returns 0 and fills `*err` (`*out` is left empty). Returns 0 with an
 * "out of memory" error on allocation failure. */
int ll_tokenize(const char *source, LlTokenList *out, LlError *err);

/* Release a token list produced by ll_tokenize. NULL-safe. */
void ll_token_list_free(LlTokenList *list);

#ifdef __cplusplus
}
#endif

#endif /* LISP_LEXER_H */

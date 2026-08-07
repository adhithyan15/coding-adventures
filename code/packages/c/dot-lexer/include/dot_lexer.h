/*
 * dot_lexer.h — a tokeniser for the Graphviz DOT language, in pure ISO C17. A
 * faithful port of the Rust `dot-lexer` crate.
 * ===========================================================================
 *
 * `dot_tokenise` scans a DOT source string into a stream of tokens (always
 * ending in an EOF sentinel) plus a list of non-fatal lexical errors — the
 * lexer recovers after an error by skipping the offending character, so a
 * partial token stream is always returned.
 *
 * Token categories: the six case-insensitive keywords (strict / graph / digraph
 * / node / edge / subgraph), punctuation ({ } [ ] = ; , :), the edge operators
 * `->` and `--`, and `DOT_ID` for every identifier — unquoted word, numeral,
 * double-quoted string (quotes stripped, \" \\ \n \t unescaped), or HTML string
 * (`<...>` with balanced angle brackets). Line and column are 1-based.
 *
 * OWNERSHIP. `dot_tokenise` returns a malloc'd result the caller frees with
 * `dot_lex_result_free`.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef DOT_LEXER_H
#define DOT_LEXER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint32_t */

typedef enum {
    DOT_STRICT,
    DOT_GRAPH,
    DOT_DIGRAPH,
    DOT_NODE,
    DOT_EDGE,
    DOT_SUBGRAPH,
    DOT_LBRACE,
    DOT_RBRACE,
    DOT_LBRACKET,
    DOT_RBRACKET,
    DOT_EQUALS,
    DOT_SEMICOLON,
    DOT_COMMA,
    DOT_COLON,
    DOT_ARROW,    /* -> */
    DOT_DASHDASH, /* -- */
    DOT_ID,
    DOT_EOF
} DotTokenKind;

typedef struct {
    DotTokenKind kind;
    char *value; /* resolved text for DOT_ID; "" for keywords/punctuation/EOF */
    uint32_t line;
    uint32_t col;
} DotToken;

typedef struct {
    char *message;
    uint32_t line;
    uint32_t col;
} DotLexError;

typedef struct {
    DotToken *tokens;
    size_t ntokens;
    DotLexError *errors;
    size_t nerrors;
} DotLexResult;

/* dot_tokenise — tokenise the NUL-terminated DOT source `source`. Returns a
 * malloc'd result (free with dot_lex_result_free), or NULL on allocation
 * failure. */
DotLexResult *dot_tokenise(const char *source);

/* dot_lex_result_free — free a result and all its tokens/errors (safe NULL). */
void dot_lex_result_free(DotLexResult *r);

#endif /* DOT_LEXER_H */

// lisp_lexer.hpp — tokenize Lisp source into tokens.
// ===================================================
//
// A faithful, header-only port of the Rust `lisp-lexer` crate (namespace
// `ca::lisp_lexer`). A lexer breaks raw source text into a stream of typed
// tokens — the first stage of any language pipeline. Lisp has only 7 meaningful
// token types (plus EOF), so the scanner is small.
//
//   Number   integer literal, possibly negative:  42  -7  0
//   Symbol   identifier or operator name:          define  +  car  null?
//   String   double-quoted, value includes quotes: "hello"
//   LParen   (      RParen  )      Quote  '      Dot  .
//   Eof      end of input — every token stream ends with exactly one
//
// Whitespace and `;`-to-end-of-line comments are skipped. `-42` is one Number
// token, while `-` followed by non-digit is a Symbol.
//
// Rust's `Result` becomes a thrown `LexerError`. The Rust original scans a
// `Vec<char>`; this port scans bytes, so `position` is a byte offset — identical
// for any ASCII input.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef LISP_LEXER_HPP
#define LISP_LEXER_HPP

#include <cstddef>
#include <cstdio>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca::lisp_lexer {

// The kinds of tokens Lisp source can contain.
enum class TokenType {
    Number,
    Symbol,
    String,
    LParen,
    RParen,
    Quote,
    Dot,
    Eof
};

// Uppercase name of a token type ("NUMBER", "SYMBOL", …, "EOF").
inline const char* token_type_name(TokenType type) {
    switch (type) {
        case TokenType::Number: return "NUMBER";
        case TokenType::Symbol: return "SYMBOL";
        case TokenType::String: return "STRING";
        case TokenType::LParen: return "LPAREN";
        case TokenType::RParen: return "RPAREN";
        case TokenType::Quote: return "QUOTE";
        case TokenType::Dot: return "DOT";
        case TokenType::Eof: return "EOF";
    }
    return "?";
}

// A single token: its type and the original source text (for EOF, empty).
struct Token {
    TokenType type;
    std::string value;

    friend bool operator==(const Token& a, const Token& b) {
        return a.type == b.type && a.value == b.value;
    }
    friend bool operator!=(const Token& a, const Token& b) { return !(a == b); }
};

// Thrown when the source contains an unrecognised construct.
class LexerError : public std::runtime_error {
   public:
    LexerError(const std::string& message, std::size_t position)
        : std::runtime_error(message), position(position) {}
    std::size_t position;  // byte offset where the error occurred
};

namespace detail {
// Rust `is_ascii_whitespace`: space, tab, LF, FF, CR (NOT vertical tab).
inline bool is_ws(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f';
}
inline bool is_digit(unsigned char c) { return c >= '0' && c <= '9'; }
inline bool is_alpha(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
inline bool is_symbol_start(unsigned char c) {
    return is_alpha(c) || c == '_' || c == '+' || c == '-' || c == '*' ||
           c == '/' || c == '=' || c == '<' || c == '>' || c == '!' ||
           c == '?' || c == '&';
}
inline bool is_symbol_continue(unsigned char c) {
    return is_symbol_start(c) || is_digit(c);
}
}  // namespace detail

// Tokenize `source` into a vector of tokens ending with one Eof.
// Throws `LexerError` on an unrecognised construct.
inline std::vector<Token> tokenize(const std::string& source) {
    using namespace detail;
    std::vector<Token> tokens;
    const std::size_t n = source.size();
    std::size_t pos = 0;

    while (pos < n) {
        unsigned char c = static_cast<unsigned char>(source[pos]);

        // Step 1: whitespace and `;` comments.
        if (is_ws(c)) {
            ++pos;
            continue;
        }
        if (c == ';') {
            while (pos < n && source[pos] != '\n') ++pos;
            continue;
        }

        // Step 2: single-character delimiters.
        if (c == '(') {
            tokens.push_back({TokenType::LParen, "("});
            ++pos;
            continue;
        }
        if (c == ')') {
            tokens.push_back({TokenType::RParen, ")"});
            ++pos;
            continue;
        }
        if (c == '\'') {
            tokens.push_back({TokenType::Quote, "'"});
            ++pos;
            continue;
        }
        if (c == '.') {
            tokens.push_back({TokenType::Dot, "."});
            ++pos;
            continue;
        }

        // Step 3: string literals (value includes the surrounding quotes).
        if (c == '"') {
            std::size_t start = pos;
            ++pos;  // opening quote
            while (pos < n && source[pos] != '"') {
                if (source[pos] == '\\') ++pos;  // skip the escaped byte
                ++pos;
            }
            if (pos >= n)
                throw LexerError("Unterminated string literal", start);
            ++pos;  // closing quote
            tokens.push_back(
                {TokenType::String, source.substr(start, pos - start)});
            continue;
        }

        // Step 4: numbers, including a leading `-` before a digit.
        if (is_digit(c) ||
            (c == '-' && pos + 1 < n &&
             is_digit(static_cast<unsigned char>(source[pos + 1])))) {
            std::size_t start = pos;
            if (c == '-') ++pos;
            while (pos < n && is_digit(static_cast<unsigned char>(source[pos])))
                ++pos;
            tokens.push_back(
                {TokenType::Number, source.substr(start, pos - start)});
            continue;
        }

        // Step 5: symbols.
        if (is_symbol_start(c)) {
            std::size_t start = pos;
            ++pos;
            while (pos < n &&
                   is_symbol_continue(static_cast<unsigned char>(source[pos])))
                ++pos;
            tokens.push_back(
                {TokenType::Symbol, source.substr(start, pos - start)});
            continue;
        }

        // Step 6: unrecognised byte.
        char buf[64];
        if (c >= 0x20 && c < 0x7f)
            std::snprintf(buf, sizeof buf, "Unexpected character: '%c'",
                          static_cast<char>(c));
        else
            std::snprintf(buf, sizeof buf, "Unexpected character: '\\x%02x'", c);
        throw LexerError(buf, pos);
    }

    tokens.push_back({TokenType::Eof, ""});
    return tokens;
}

}  // namespace ca::lisp_lexer

#endif  // LISP_LEXER_HPP

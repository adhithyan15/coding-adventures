// dot_lexer.hpp — a tokeniser for the Graphviz DOT language, in pure ISO C++17,
// header-only, in namespace ca::dot. A faithful port of the Rust `dot-lexer`
// crate.
// ===========================================================================
//
// `tokenise(source)` scans a DOT source string into a stream of tokens (always
// ending in an Eof sentinel) plus a list of non-fatal lexical errors — the lexer
// recovers after an error by skipping the offending character.
//
// Token categories: the six case-insensitive keywords (strict / graph / digraph
// / node / edge / subgraph), punctuation ({ } [ ] = ; , :), the edge operators
// `->` and `--`, and `Id` for identifiers (unquoted word, numeral, double-quoted
// string with \" \\ \n \t escapes, or HTML string `<...>` with balanced angle
// brackets). Line and column are 1-based.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_DOT_LEXER_HPP
#define CA_DOT_LEXER_HPP

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace dot {

enum class TokenKind {
    Strict, Graph, Digraph, Node, Edge, Subgraph,
    LBrace, RBrace, LBracket, RBracket, Equals, Semicolon, Comma, Colon,
    Arrow, DashDash, Id, Eof
};

struct Token {
    TokenKind kind;
    std::string value;  // resolved text for Id; empty for keywords/punct/Eof
    std::uint32_t line;
    std::uint32_t col;
};

struct LexError {
    std::string message;
    std::uint32_t line;
    std::uint32_t col;
};

struct LexResult {
    std::vector<Token> tokens;
    std::vector<LexError> errors;
};

namespace detail {

class Lexer {
public:
    explicit Lexer(const std::string& source)
        : src_(reinterpret_cast<const unsigned char*>(source.data())),
          len_(source.size()) {}

    LexResult run() {
        scan_all();
        return LexResult{std::move(tokens_), std::move(errors_)};
    }

private:
    const unsigned char* src_;
    std::size_t len_;
    std::size_t pos_ = 0;
    std::uint32_t line_ = 1;
    std::uint32_t col_ = 1;
    std::vector<Token> tokens_;
    std::vector<LexError> errors_;

    std::optional<int> peek() const {
        return pos_ < len_ ? std::optional<int>(src_[pos_]) : std::nullopt;
    }
    std::optional<int> peek2() const {
        return pos_ + 1 < len_ ? std::optional<int>(src_[pos_ + 1]) : std::nullopt;
    }
    std::optional<int> advance() {
        if (pos_ >= len_) {
            return std::nullopt;
        }
        int ch = src_[pos_++];
        if (ch == '\n') {
            ++line_;
            col_ = 1;
        } else {
            ++col_;
        }
        return ch;
    }
    bool at_end() const { return pos_ >= len_; }

    void emit(TokenKind kind, std::string value, std::uint32_t line,
              std::uint32_t col) {
        tokens_.push_back(Token{kind, std::move(value), line, col});
    }
    void error_at(const std::string& msg, std::uint32_t line,
                  std::uint32_t col) {
        errors_.push_back(LexError{msg, line, col});
    }
    void error(const std::string& msg) { error_at(msg, line_, col_); }

    static bool is_digit(int c) { return c >= '0' && c <= '9'; }
    static bool is_alpha(int c) {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
    }
    static bool is_alnum(int c) { return is_alpha(c) || is_digit(c); }

    void skip_ws_comments() {
        for (;;) {
            while (auto c = peek()) {
                if (*c == ' ' || *c == '\t' || *c == '\r' || *c == '\n') {
                    advance();
                } else {
                    break;
                }
            }
            if (peek() == std::optional<int>('/') &&
                peek2() == std::optional<int>('/')) {
                advance();
                advance();
                while (auto ch = advance()) {
                    if (*ch == '\n') {
                        break;
                    }
                }
                continue;
            }
            if (peek() == std::optional<int>('/') &&
                peek2() == std::optional<int>('*')) {
                advance();
                advance();
                for (;;) {
                    if (at_end()) {
                        error("unterminated block comment");
                        break;
                    }
                    if (peek() == std::optional<int>('*') &&
                        peek2() == std::optional<int>('/')) {
                        advance();
                        advance();
                        break;
                    }
                    advance();
                }
                continue;
            }
            break;
        }
    }

    std::string scan_quoted_string(std::uint32_t sl, std::uint32_t sc) {
        std::string buf;
        for (;;) {
            auto ch = advance();
            if (!ch) {
                error_at("unterminated string literal", sl, sc);
                break;
            }
            if (*ch == '"') {
                break;
            }
            if (*ch == '\\') {
                auto esc = advance();
                if (!esc) {
                    error("unexpected end of input in string escape");
                    break;
                }
                if (*esc == '"') buf.push_back('"');
                else if (*esc == '\\') buf.push_back('\\');
                else if (*esc == 'n') buf.push_back('\n');
                else if (*esc == 't') buf.push_back('\t');
                else {
                    buf.push_back('\\');
                    buf.push_back(static_cast<char>(*esc));
                }
            } else {
                buf.push_back(static_cast<char>(*ch));
            }
        }
        return buf;
    }

    std::string scan_html_string(std::uint32_t sl, std::uint32_t sc) {
        std::string buf;
        int depth = 1;
        for (;;) {
            auto ch = advance();
            if (!ch) {
                error_at("unterminated HTML string", sl, sc);
                break;
            }
            if (*ch == '<') {
                ++depth;
                buf.push_back('<');
            } else if (*ch == '>') {
                --depth;
                if (depth == 0) {
                    break;
                }
                buf.push_back('>');
            } else {
                buf.push_back(static_cast<char>(*ch));
            }
        }
        return buf;
    }

    std::string scan_unquoted_id(int first) {
        std::string buf(1, static_cast<char>(first));
        while (auto c = peek()) {
            if (is_alnum(*c) || *c == '_' || *c >= 0x80) {
                advance();
                buf.push_back(static_cast<char>(*c));
            } else {
                break;
            }
        }
        return buf;
    }

    std::string scan_numeral(int first) {
        std::string buf(1, static_cast<char>(first));
        while (auto c = peek()) {
            if (is_digit(*c)) {
                advance();
                buf.push_back(static_cast<char>(*c));
            } else {
                break;
            }
        }
        if (peek() == std::optional<int>('.')) {
            advance();
            buf.push_back('.');
            while (auto c = peek()) {
                if (is_digit(*c)) {
                    advance();
                    buf.push_back(static_cast<char>(*c));
                } else {
                    break;
                }
            }
        }
        return buf;
    }

    static TokenKind keyword_or_id(const std::string& word) {
        std::string lower;
        lower.reserve(word.size());
        for (char c : word) {
            lower.push_back((c >= 'A' && c <= 'Z')
                                ? static_cast<char>(c - 'A' + 'a')
                                : c);
        }
        if (lower == "strict") return TokenKind::Strict;
        if (lower == "graph") return TokenKind::Graph;
        if (lower == "digraph") return TokenKind::Digraph;
        if (lower == "node") return TokenKind::Node;
        if (lower == "edge") return TokenKind::Edge;
        if (lower == "subgraph") return TokenKind::Subgraph;
        return TokenKind::Id;
    }

    void scan_all() {
        for (;;) {
            skip_ws_comments();
            if (at_end()) {
                emit(TokenKind::Eof, "", line_, col_);
                break;
            }
            std::uint32_t line = line_, col = col_;
            int ch = *advance();
            switch (ch) {
                case '{': emit(TokenKind::LBrace, "", line, col); continue;
                case '}': emit(TokenKind::RBrace, "", line, col); continue;
                case '[': emit(TokenKind::LBracket, "", line, col); continue;
                case ']': emit(TokenKind::RBracket, "", line, col); continue;
                case '=': emit(TokenKind::Equals, "", line, col); continue;
                case ';': emit(TokenKind::Semicolon, "", line, col); continue;
                case ',': emit(TokenKind::Comma, "", line, col); continue;
                case ':': emit(TokenKind::Colon, "", line, col); continue;
                default: break;
            }
            auto nc = peek();
            if (ch == '-' && nc == std::optional<int>('>')) {
                advance();
                emit(TokenKind::Arrow, "", line, col);
            } else if (ch == '-' && nc == std::optional<int>('-')) {
                advance();
                emit(TokenKind::DashDash, "", line, col);
            } else if (ch == '-' && nc && (is_digit(*nc) || *nc == '.')) {
                emit(TokenKind::Id, scan_numeral(ch), line, col);
            } else if (ch == '.' && nc && is_digit(*nc)) {
                emit(TokenKind::Id, scan_numeral(ch), line, col);
            } else if (is_digit(ch)) {
                emit(TokenKind::Id, scan_numeral(ch), line, col);
            } else if (ch == '"') {
                emit(TokenKind::Id, scan_quoted_string(line, col), line, col);
            } else if (ch == '<') {
                emit(TokenKind::Id, scan_html_string(line, col), line, col);
            } else if (is_alpha(ch) || ch == '_' || ch >= 0x80) {
                std::string word = scan_unquoted_id(ch);
                TokenKind kind = keyword_or_id(word);
                emit(kind,
                     kind == TokenKind::Id ? std::move(word) : std::string(),
                     line, col);
            } else {
                char buf[64];
                std::snprintf(buf, sizeof buf,
                              "unexpected character '%c' (0x%02x)",
                              (ch >= 32 && ch < 127) ? ch : '?',
                              static_cast<unsigned>(ch));
                error_at(buf, line, col);
            }
        }
    }
};

}  // namespace detail

// tokenise — tokenise a DOT source string into tokens plus recoverable errors.
inline LexResult tokenise(const std::string& source) {
    return detail::Lexer(source).run();
}

}  // namespace dot
}  // namespace ca

#endif  // CA_DOT_LEXER_HPP

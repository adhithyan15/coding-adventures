// Tests for the C++ lisp-lexer library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>
#include <vector>

#include "lisp_lexer.hpp"

namespace ll = ca::lisp_lexer;
using ll::TokenType;

// Token types of `src`, excluding the trailing Eof.
static std::vector<TokenType> types(const std::string& src) {
    std::vector<TokenType> out;
    for (const auto& t : ll::tokenize(src))
        if (t.type != TokenType::Eof) out.push_back(t.type);
    return out;
}

// Token values of `src`, excluding the trailing Eof.
static std::vector<std::string> values(const std::string& src) {
    std::vector<std::string> out;
    for (const auto& t : ll::tokenize(src))
        if (t.type != TokenType::Eof) out.push_back(t.value);
    return out;
}

int main() {
    using T = TokenType;

    // ── basic atoms ──────────────────────────────────────────────────────────
    ISO_CHECK((types("42") == std::vector<T>{T::Number}));
    ISO_CHECK((values("42") == std::vector<std::string>{"42"}));
    ISO_CHECK((types("-7") == std::vector<T>{T::Number}));
    ISO_CHECK((values("-7") == std::vector<std::string>{"-7"}));
    ISO_CHECK((types("0") == std::vector<T>{T::Number}));
    ISO_CHECK((types("define") == std::vector<T>{T::Symbol}));
    ISO_CHECK((values("define") == std::vector<std::string>{"define"}));
    ISO_CHECK((types("\"hello world\"") == std::vector<T>{T::String}));
    ISO_CHECK((types("\"hello \\\"world\\\"\"") == std::vector<T>{T::String}));

    // ── operator symbols ─────────────────────────────────────────────────────
    ISO_CHECK((types("+") == std::vector<T>{T::Symbol}));
    ISO_CHECK((values("+") == std::vector<std::string>{"+"}));
    ISO_CHECK((types("(- 3 1)") ==
               std::vector<T>{T::LParen, T::Symbol, T::Number, T::Number,
                              T::RParen}));
    ISO_CHECK((types("*") == std::vector<T>{T::Symbol}));
    ISO_CHECK((types("/") == std::vector<T>{T::Symbol}));
    ISO_CHECK((types("=") == std::vector<T>{T::Symbol}));
    ISO_CHECK((types("< > <= >=") ==
               std::vector<T>{T::Symbol, T::Symbol, T::Symbol, T::Symbol}));
    ISO_CHECK((types("set!") == std::vector<T>{T::Symbol}));
    ISO_CHECK((values("set!") == std::vector<std::string>{"set!"}));
    ISO_CHECK((types("null?") == std::vector<T>{T::Symbol}));
    ISO_CHECK((values("null?") == std::vector<std::string>{"null?"}));

    // ── delimiters ───────────────────────────────────────────────────────────
    ISO_CHECK((types("()") == std::vector<T>{T::LParen, T::RParen}));
    ISO_CHECK((types("'x") == std::vector<T>{T::Quote, T::Symbol}));
    ISO_CHECK((types("(a . b)") ==
               std::vector<T>{T::LParen, T::Symbol, T::Dot, T::Symbol,
                              T::RParen}));

    // ── whitespace and comments ──────────────────────────────────────────────
    ISO_CHECK((types("  42  ") == std::vector<T>{T::Number}));
    ISO_CHECK((types("a\tb") == std::vector<T>{T::Symbol, T::Symbol}));
    ISO_CHECK((types("a\nb") == std::vector<T>{T::Symbol, T::Symbol}));
    ISO_CHECK((types("; this is a comment\n42") == std::vector<T>{T::Number}));
    ISO_CHECK((types("(+ 1 2) ; add them") ==
               std::vector<T>{T::LParen, T::Symbol, T::Number, T::Number,
                              T::RParen}));

    // ── full expressions ─────────────────────────────────────────────────────
    ISO_CHECK((types("(+ 1 2)") ==
               std::vector<T>{T::LParen, T::Symbol, T::Number, T::Number,
                              T::RParen}));
    ISO_CHECK((types("(+ (* 2 3) 4)") ==
               std::vector<T>{T::LParen, T::Symbol, T::LParen, T::Symbol,
                              T::Number, T::Number, T::RParen, T::Number,
                              T::RParen}));
    ISO_CHECK((types("(define x 42)") ==
               std::vector<T>{T::LParen, T::Symbol, T::Symbol, T::Number,
                              T::RParen}));
    ISO_CHECK((types("(lambda (x) (* x x))") ==
               std::vector<T>{T::LParen, T::Symbol, T::LParen, T::Symbol,
                              T::RParen, T::LParen, T::Symbol, T::Symbol,
                              T::Symbol, T::RParen, T::RParen}));
    ISO_CHECK((types("'foo") == std::vector<T>{T::Quote, T::Symbol}));
    ISO_CHECK((types("'(1 2 3)") ==
               std::vector<T>{T::Quote, T::LParen, T::Number, T::Number,
                              T::Number, T::RParen}));
    ISO_CHECK((types("(1 . 2)") ==
               std::vector<T>{T::LParen, T::Number, T::Dot, T::Number,
                              T::RParen}));
    ISO_CHECK((types("(cond ((eq x 0) 1) (t x))") ==
               std::vector<T>{T::LParen, T::Symbol, T::LParen, T::LParen,
                              T::Symbol, T::Symbol, T::Number, T::RParen,
                              T::Number, T::RParen, T::LParen, T::Symbol,
                              T::Symbol, T::RParen, T::RParen}));
    {  // factorial
        std::string src = R"(
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1))))))))";
        std::vector<ll::Token> toks;
        for (const auto& t : ll::tokenize(src))
            if (t.type != TokenType::Eof) toks.push_back(t);
        ISO_CHECK(toks.size() > 20);
        ISO_CHECK(toks[0].type == T::LParen);
        ISO_CHECK(toks[1].value == "define");
        ISO_CHECK(toks[2].value == "factorial");
    }

    // ── EOF / empties ────────────────────────────────────────────────────────
    {
        auto t = ll::tokenize("");
        ISO_CHECK(t.size() == 1 && t[0].type == T::Eof);
    }
    {
        auto t = ll::tokenize("; just a comment\n; another one");
        ISO_CHECK(t.size() == 1 && t[0].type == T::Eof);
    }
    {
        auto t = ll::tokenize("(+ 1 2)");
        ISO_CHECK(t.back().type == T::Eof);
    }

    // ── number vs symbol disambiguation ──────────────────────────────────────
    ISO_CHECK((types("-42") == std::vector<T>{T::Number}));
    ISO_CHECK((values("-42") == std::vector<std::string>{"-42"}));
    ISO_CHECK((values("(- 3 1)") ==
               std::vector<std::string>{"(", "-", "3", "1", ")"}));

    // ── error cases ──────────────────────────────────────────────────────────
    {
        bool threw = false;
        try {
            ll::tokenize("\"hello");
        } catch (const ll::LexerError& e) {
            threw = true;
            ISO_CHECK(std::string(e.what()).find("Unterminated string") !=
                      std::string::npos);
        }
        ISO_CHECK(threw);
    }
    {
        bool threw = false;
        try {
            ll::tokenize("@");
        } catch (const ll::LexerError& e) {
            threw = true;
            ISO_CHECK(std::string(e.what()).find("Unexpected character") !=
                      std::string::npos);
        }
        ISO_CHECK(threw);
    }

    // ── token type names ─────────────────────────────────────────────────────
    ISO_CHECK(std::string(ll::token_type_name(T::Number)) == "NUMBER");
    ISO_CHECK(std::string(ll::token_type_name(T::Symbol)) == "SYMBOL");
    ISO_CHECK(std::string(ll::token_type_name(T::String)) == "STRING");
    ISO_CHECK(std::string(ll::token_type_name(T::LParen)) == "LPAREN");
    ISO_CHECK(std::string(ll::token_type_name(T::RParen)) == "RPAREN");
    ISO_CHECK(std::string(ll::token_type_name(T::Quote)) == "QUOTE");
    ISO_CHECK(std::string(ll::token_type_name(T::Dot)) == "DOT");
    ISO_CHECK(std::string(ll::token_type_name(T::Eof)) == "EOF");

    return ISO_TEST_RESULT();
}

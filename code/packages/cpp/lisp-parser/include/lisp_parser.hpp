// lisp_parser.hpp — parse token streams into S-expression ASTs.
// ==============================================================
//
// A faithful, header-only port of the Rust `lisp-parser` crate (namespace
// `ca::lisp_parser`). It sits on top of the sibling header-only `lisp-lexer`:
// tokens in, a tree of `SExpr` nodes out. Lisp's grammar is tiny (6 rules), so
// this is a small recursive-descent parser.
//
//   program = { sexpr }
//   sexpr   = atom | list | quoted
//   atom    = NUMBER | SYMBOL | STRING
//   list    = '(' { sexpr } ')'         (may end '. sexpr' → a dotted pair)
//   quoted  = "'" sexpr                 ('x is sugar for (quote x))
//
// Rust's `Result` becomes a thrown `ParseError`. An `SExpr` is move-only (it
// owns its children via `std::unique_ptr` for the single-child forms).
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef LISP_PARSER_HPP
#define LISP_PARSER_HPP

#include <cstddef>
#include <memory>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

#include "lisp_lexer.hpp"

namespace ca::lisp_parser {

// The kind of atom (terminal value) in an S-expression.
enum class AtomKind { Number, Symbol, String };

// The kind of S-expression node.
enum class SExprKind { Atom, List, DottedPair, Quoted };

class SExpr;

namespace detail {
struct Atom {
    AtomKind kind;
    std::string value;
};
struct List {
    std::vector<SExpr> items;
};
struct DottedPair {
    std::vector<SExpr> elements;
    std::unique_ptr<SExpr> last;
};
struct Quoted {
    std::unique_ptr<SExpr> inner;
};
}  // namespace detail

// An S-expression: an atom, a list, a dotted pair, or a quoted form. Move-only.
class SExpr {
   public:
    std::variant<detail::Atom, detail::List, detail::DottedPair, detail::Quoted>
        node;

    SExprKind kind() const {
        switch (node.index()) {
            case 0: return SExprKind::Atom;
            case 1: return SExprKind::List;
            case 2: return SExprKind::DottedPair;
            default: return SExprKind::Quoted;
        }
    }

    // For an Atom node: its kind and source text.
    AtomKind atom_kind() const { return std::get<detail::Atom>(node).kind; }
    const std::string& atom_value() const {
        return std::get<detail::Atom>(node).value;
    }

    // Recursively collect every atom value in this tree.
    std::vector<std::string> find_atoms() const {
        std::vector<std::string> out;
        collect_atoms(out);
        return out;
    }

    // Number of List / DottedPair nodes in this tree.
    std::size_t count_lists() const {
        if (const auto* l = std::get_if<detail::List>(&node)) {
            std::size_t total = 1;
            for (const auto& c : l->items) total += c.count_lists();
            return total;
        }
        if (const auto* d = std::get_if<detail::DottedPair>(&node)) {
            std::size_t total = 1;
            for (const auto& c : d->elements) total += c.count_lists();
            return total + d->last->count_lists();
        }
        if (const auto* q = std::get_if<detail::Quoted>(&node))
            return q->inner->count_lists();
        return 0;  // Atom
    }

    // Number of Quoted nodes in this tree.
    std::size_t count_quoted() const {
        if (const auto* q = std::get_if<detail::Quoted>(&node))
            return 1 + q->inner->count_quoted();
        if (const auto* l = std::get_if<detail::List>(&node)) {
            std::size_t total = 0;
            for (const auto& c : l->items) total += c.count_quoted();
            return total;
        }
        if (const auto* d = std::get_if<detail::DottedPair>(&node)) {
            std::size_t total = 0;
            for (const auto& c : d->elements) total += c.count_quoted();
            return total + d->last->count_quoted();
        }
        return 0;  // Atom
    }

   private:
    void collect_atoms(std::vector<std::string>& out) const {
        if (const auto* a = std::get_if<detail::Atom>(&node)) {
            out.push_back(a->value);
        } else if (const auto* l = std::get_if<detail::List>(&node)) {
            for (const auto& c : l->items) c.collect_atoms(out);
        } else if (const auto* d = std::get_if<detail::DottedPair>(&node)) {
            for (const auto& c : d->elements) c.collect_atoms(out);
            d->last->collect_atoms(out);
        } else {
            std::get<detail::Quoted>(node).inner->collect_atoms(out);
        }
    }
};

// Thrown on a syntax error (or a wrapped lexer error).
class ParseError : public std::runtime_error {
   public:
    explicit ParseError(const std::string& message)
        : std::runtime_error(message) {}
};

namespace detail {

class Parser {
   public:
    explicit Parser(std::vector<lisp_lexer::Token> tokens)
        : tokens_(std::move(tokens)) {}

    std::vector<SExpr> parse_program() {
        std::vector<SExpr> out;
        while (peek_type() != lisp_lexer::TokenType::Eof)
            out.push_back(parse_sexpr());
        return out;
    }

   private:
    lisp_lexer::TokenType peek_type() const {
        return pos_ < tokens_.size() ? tokens_[pos_].type
                                     : lisp_lexer::TokenType::Eof;
    }
    std::string peek_value() const {
        return pos_ < tokens_.size() ? tokens_[pos_].value : std::string();
    }
    void advance() {
        if (pos_ < tokens_.size()) ++pos_;
    }

    SExpr parse_sexpr() {
        using lisp_lexer::TokenType;
        switch (peek_type()) {
            case TokenType::LParen:
                return parse_list();
            case TokenType::Quote: {
                advance();
                auto inner = std::make_unique<SExpr>(parse_sexpr());
                return SExpr{Quoted{std::move(inner)}};
            }
            case TokenType::Number: {
                std::string v = peek_value();
                advance();
                return SExpr{Atom{AtomKind::Number, std::move(v)}};
            }
            case TokenType::Symbol: {
                std::string v = peek_value();
                advance();
                return SExpr{Atom{AtomKind::Symbol, std::move(v)}};
            }
            case TokenType::String: {
                std::string v = peek_value();
                advance();
                return SExpr{Atom{AtomKind::String, std::move(v)}};
            }
            default:
                throw ParseError("ParseError: Unexpected token");
        }
    }

    SExpr parse_list() {
        using lisp_lexer::TokenType;
        advance();  // consume '('
        std::vector<SExpr> elements;
        bool dotted = false;
        std::unique_ptr<SExpr> dot_value;

        while (peek_type() != TokenType::RParen &&
               peek_type() != TokenType::Eof) {
            if (peek_type() == TokenType::Dot) {
                advance();
                dotted = true;
                dot_value = std::make_unique<SExpr>(parse_sexpr());
                break;
            }
            elements.push_back(parse_sexpr());
        }

        if (peek_type() != TokenType::RParen)
            throw ParseError("ParseError: Expected RParen");
        advance();  // consume ')'

        if (dotted)
            return SExpr{DottedPair{std::move(elements), std::move(dot_value)}};
        return SExpr{List{std::move(elements)}};
    }

    std::vector<lisp_lexer::Token> tokens_;
    std::size_t pos_ = 0;
};

}  // namespace detail

// Parse a pre-tokenized stream into S-expressions.
inline std::vector<SExpr> parse_tokens(std::vector<lisp_lexer::Token> tokens) {
    detail::Parser parser(std::move(tokens));
    return parser.parse_program();
}

// Parse Lisp `source` into a vector of top-level S-expressions. Throws
// `ParseError` on a lexer or syntax error.
inline std::vector<SExpr> parse(const std::string& source) {
    try {
        return parse_tokens(lisp_lexer::tokenize(source));
    } catch (const lisp_lexer::LexerError& e) {
        throw ParseError(std::string("ParseError: Lexer error: ") + e.what());
    }
}

}  // namespace ca::lisp_parser

#endif  // LISP_PARSER_HPP

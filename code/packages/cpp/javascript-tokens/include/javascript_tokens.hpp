// javascript_tokens.hpp — shared JavaScript/TypeScript token vocabulary, in pure
// ISO C++17, header-only, in namespace ca::jstokens. A faithful port of the Rust
// `javascript-tokens` crate.
// ===========================================================================
//
// The shared vocabulary every layer of a JS toolchain talks in — the lexer,
// parser, AST, and tooling — without depending on any layer above:
//
//   - EsVersion : the ECMAScript editions with a grammar (ES1..ES2025), with
//                 string round-tripping and a chronological order.
//   - Span      : a half-open [start, end) byte range within one source file.
//   - TokenKind : the broad classification of a token, plus an `Other` variant
//                 carrying a grammar-specific token name.
//
// DIVERGENCE. `EsVersion::parse` throws `UnknownEsVersion`; `try_parse` returns
// `std::optional` instead. `EsVersion` relational order is the enum's underlying
// (declaration = chronological) order.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_JAVASCRIPT_TOKENS_HPP
#define CA_JAVASCRIPT_TOKENS_HPP

#include <array>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>

namespace ca {
namespace jstokens {

// ── EsVersion ────────────────────────────────────────────────────────────────

// The ECMAScript editions with a grammar file, in chronological (= underlying)
// order, so `Es1 < Es2025` holds via the enum's built-in relational operators.
enum class EsVersion {
    Es1,
    Es3,
    Es5,
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    Es2023,
    Es2024,
    Es2025
};

// The grammar-file basename: "es1" … "es2025".
inline const char* as_str(EsVersion v) {
    switch (v) {
        case EsVersion::Es1: return "es1";
        case EsVersion::Es3: return "es3";
        case EsVersion::Es5: return "es5";
        case EsVersion::Es2015: return "es2015";
        case EsVersion::Es2016: return "es2016";
        case EsVersion::Es2017: return "es2017";
        case EsVersion::Es2018: return "es2018";
        case EsVersion::Es2019: return "es2019";
        case EsVersion::Es2020: return "es2020";
        case EsVersion::Es2021: return "es2021";
        case EsVersion::Es2022: return "es2022";
        case EsVersion::Es2023: return "es2023";
        case EsVersion::Es2024: return "es2024";
        case EsVersion::Es2025: return "es2025";
    }
    return "";
}

// Every version in chronological order.
inline const std::array<EsVersion, 14>& es_version_all() {
    static const std::array<EsVersion, 14> all = {
        EsVersion::Es1,    EsVersion::Es3,    EsVersion::Es5,
        EsVersion::Es2015, EsVersion::Es2016, EsVersion::Es2017,
        EsVersion::Es2018, EsVersion::Es2019, EsVersion::Es2020,
        EsVersion::Es2021, EsVersion::Es2022, EsVersion::Es2023,
        EsVersion::Es2024, EsVersion::Es2025};
    return all;
}

// The most recent edition (ES2025) — also the default.
constexpr EsVersion es_version_latest() { return EsVersion::Es2025; }
constexpr EsVersion es_version_default() { return es_version_latest(); }

// Error thrown by EsVersion::parse when the input names no known edition.
class UnknownEsVersion : public std::runtime_error {
public:
    explicit UnknownEsVersion(const std::string& bad)
        : std::runtime_error(build_message(bad)), value_(bad) {}
    const std::string& value() const { return value_; }

private:
    std::string value_;
    static std::string build_message(const std::string& bad) {
        std::string msg = "unknown ECMAScript version \"" + bad +
                          "\"; valid values are ";
        bool first = true;
        for (EsVersion v : es_version_all()) {
            if (!first) msg += ", ";
            first = false;
            msg += '"';
            msg += as_str(v);
            msg += '"';
        }
        return msg;
    }
};

// Non-throwing parse from the same strings `as_str` emits (exact; empty string
// rejected). std::nullopt if unrecognized.
inline std::optional<EsVersion> es_version_try_parse(const std::string& s) {
    for (EsVersion v : es_version_all()) {
        if (s == as_str(v)) return v;
    }
    return std::nullopt;
}

// Throwing parse: throws UnknownEsVersion on an unrecognized string.
inline EsVersion es_version_parse(const std::string& s) {
    if (auto v = es_version_try_parse(s)) return *v;
    throw UnknownEsVersion(s);
}

// ── Span ─────────────────────────────────────────────────────────────────────

// A half-open [start, end) byte range within one source file. `start <= end` is
// the caller's invariant (not enforced), matching the Rust type.
struct Span {
    std::uint32_t start;
    std::uint32_t end;

    static constexpr Span make(std::uint32_t start, std::uint32_t end) {
        return Span{start, end};
    }
    constexpr std::uint32_t len() const { return end - start; }
    constexpr bool is_empty() const { return start == end; }

    constexpr bool operator==(const Span& o) const {
        return start == o.start && end == o.end;
    }
    constexpr bool operator!=(const Span& o) const { return !(*this == o); }
    // Lexicographic: start first, then end.
    constexpr bool operator<(const Span& o) const {
        return start != o.start ? start < o.start : end < o.end;
    }
    constexpr bool operator>(const Span& o) const { return o < *this; }
    constexpr bool operator<=(const Span& o) const { return !(o < *this); }
    constexpr bool operator>=(const Span& o) const { return !(*this < o); }
};

// ── TokenKind ────────────────────────────────────────────────────────────────

// The broad token classification. `Other` carries a grammar-specific token name.
class TokenKind {
public:
    enum class Tag {
        Name,
        Number,
        String,
        Regex,
        TemplateNoSub,
        TemplateHead,
        TemplateMiddle,
        TemplateTail,
        BigInt,
        PrivateName,
        Keyword,
        Operator,
        Punctuation,
        Comment,
        Whitespace,
        Newline,
        Hashbang,
        Error,
        Eof,
        Other
    };

    // A non-Other token kind.
    static TokenKind of(Tag tag) { return TokenKind(tag, std::string()); }
    // An Other token kind wrapping the grammar token `name`.
    static TokenKind other(std::string name) {
        return TokenKind(Tag::Other, std::move(name));
    }

    Tag tag() const { return tag_; }
    const std::string& other_name() const { return other_; }

    // Trivia — Comment / Whitespace / Newline (a hint, not a hard rule).
    bool is_trivia() const {
        return tag_ == Tag::Comment || tag_ == Tag::Whitespace ||
               tag_ == Tag::Newline;
    }
    bool is_eof() const { return tag_ == Tag::Eof; }

    bool operator==(const TokenKind& o) const {
        if (tag_ != o.tag_) return false;
        return tag_ != Tag::Other || other_ == o.other_;
    }
    bool operator!=(const TokenKind& o) const { return !(*this == o); }
    // A total order (tag, then Other name) — usable as an associative-map key.
    bool operator<(const TokenKind& o) const {
        if (tag_ != o.tag_) return tag_ < o.tag_;
        return other_ < o.other_;
    }

private:
    TokenKind(Tag tag, std::string other)
        : tag_(tag), other_(std::move(other)) {}
    Tag tag_;
    std::string other_;
};

}  // namespace jstokens
}  // namespace ca

#endif  // CA_JAVASCRIPT_TOKENS_HPP

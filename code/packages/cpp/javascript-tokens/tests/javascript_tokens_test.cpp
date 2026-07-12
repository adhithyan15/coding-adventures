// Tests for the C++ javascript-tokens vocabulary, using the header-only
// iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <map>
#include <optional>
#include <string>

#include "javascript_tokens.hpp"

namespace jst = ca::jstokens;
using jst::EsVersion;
using jst::Span;
using TokenKind = jst::TokenKind;
using Tag = jst::TokenKind::Tag;

int main() {
    // ── EsVersion: latest / default / as_str / ALL ───────────────────────
    ISO_CHECK(jst::es_version_latest() == EsVersion::Es2025);
    ISO_CHECK(jst::es_version_default() == jst::es_version_latest());
    ISO_CHECK_STR_EQ(jst::as_str(EsVersion::Es2020), "es2020");
    {
        const char* expected[] = {"es1",    "es3",    "es5",    "es2015",
                                  "es2016", "es2017", "es2018", "es2019",
                                  "es2020", "es2021", "es2022", "es2023",
                                  "es2024", "es2025"};
        const auto& all = jst::es_version_all();
        ISO_CHECK_EQ_UINT(all.size(), 14u);
        for (std::size_t i = 0; i < all.size(); i++) {
            ISO_CHECK_STR_EQ(jst::as_str(all[i]), expected[i]);
        }
    }

    // ── round-trip through strings ───────────────────────────────────────
    for (EsVersion v : jst::es_version_all()) {
        auto parsed = jst::es_version_try_parse(jst::as_str(v));
        ISO_CHECK(parsed.has_value() && *parsed == v);
    }

    // ── empty string and unknowns are rejected ───────────────────────────
    {
        ISO_CHECK(!jst::es_version_try_parse("").has_value());
        const char* bad[] = {"es2", "es5.1", "latest", "ES2025", "es2026", " es2025"};
        for (const char* b : bad) {
            ISO_CHECK(!jst::es_version_try_parse(b).has_value());
            bool threw = false;
            try {
                (void)jst::es_version_parse(b);
            } catch (const jst::UnknownEsVersion&) {
                threw = true;
            }
            ISO_CHECK(threw);
        }
    }

    // ── unknown-version message names input and valid set ────────────────
    {
        bool threw = false;
        try {
            (void)jst::es_version_parse("nope");
        } catch (const jst::UnknownEsVersion& e) {
            threw = true;
            std::string msg = e.what();
            ISO_CHECK(msg.find("\"nope\"") != std::string::npos);
            ISO_CHECK(msg.find("\"es2025\"") != std::string::npos);
            ISO_CHECK(e.value() == "nope");
        }
        ISO_CHECK(threw);
    }

    // ── ordering is chronological ────────────────────────────────────────
    ISO_CHECK(EsVersion::Es1 < EsVersion::Es3);
    ISO_CHECK(EsVersion::Es5 < EsVersion::Es2015);
    ISO_CHECK(EsVersion::Es2015 < EsVersion::Es2025);

    // ── Span: construction, len, is_empty ────────────────────────────────
    {
        Span s = Span::make(10, 20);
        ISO_CHECK_EQ_UINT(s.start, 10u);
        ISO_CHECK_EQ_UINT(s.end, 20u);
        ISO_CHECK_EQ_UINT(s.len(), 10u);
        ISO_CHECK_EQ_UINT(Span::make(0, 1).len(), 1u);
        ISO_CHECK_EQ_UINT(Span::make(42, 42).len(), 0u);
        ISO_CHECK(Span::make(0, 0).is_empty());
        ISO_CHECK(!Span::make(0, 1).is_empty());
    }

    // ── Span: equality and lexicographic ordering ────────────────────────
    ISO_CHECK(Span::make(3, 7) == Span::make(3, 7));
    ISO_CHECK(Span::make(3, 7) != Span::make(3, 8));
    ISO_CHECK(Span::make(0, 5) < Span::make(0, 6));
    ISO_CHECK(Span::make(0, 5) < Span::make(1, 2));
    ISO_CHECK(Span::make(5, 10) > Span::make(5, 5));

    // Compile-time (const) construction / len / is_empty.
    {
        constexpr Span S = Span::make(2, 5);
        static_assert(S.len() == 3, "const len");
        static_assert(!S.is_empty(), "const is_empty");
    }

    // ── TokenKind: is_trivia exhaustively ────────────────────────────────
    {
        struct {
            Tag tag;
            bool trivia;
        } cases[] = {
            {Tag::Name, false},          {Tag::Number, false},
            {Tag::String, false},        {Tag::Regex, false},
            {Tag::TemplateNoSub, false}, {Tag::TemplateHead, false},
            {Tag::TemplateMiddle, false}, {Tag::TemplateTail, false},
            {Tag::BigInt, false},        {Tag::PrivateName, false},
            {Tag::Keyword, false},       {Tag::Operator, false},
            {Tag::Punctuation, false},   {Tag::Comment, true},
            {Tag::Whitespace, true},     {Tag::Newline, true},
            {Tag::Hashbang, false},      {Tag::Error, false},
            {Tag::Eof, false},
        };
        for (auto& c : cases) {
            ISO_CHECK(TokenKind::of(c.tag).is_trivia() == c.trivia);
        }
        ISO_CHECK(!TokenKind::other("anything").is_trivia());
    }

    // ── TokenKind: is_eof only for Eof ───────────────────────────────────
    ISO_CHECK(TokenKind::of(Tag::Eof).is_eof());
    ISO_CHECK(!TokenKind::of(Tag::Name).is_eof());
    ISO_CHECK(!TokenKind::of(Tag::Newline).is_eof());
    ISO_CHECK(!TokenKind::other("EOF").is_eof());

    // ── TokenKind: equality (including Other by name) ────────────────────
    ISO_CHECK(TokenKind::of(Tag::Name) == TokenKind::of(Tag::Name));
    ISO_CHECK(TokenKind::of(Tag::Name) != TokenKind::of(Tag::Number));
    ISO_CHECK(TokenKind::other("X") == TokenKind::other("X"));
    ISO_CHECK(TokenKind::other("X") != TokenKind::other("Y"));

    // ── TokenKind: usable as an associative-map key ──────────────────────
    {
        std::map<TokenKind, unsigned> counts;
        counts[TokenKind::of(Tag::Name)] += 1;
        counts[TokenKind::of(Tag::Name)] += 1;
        counts[TokenKind::of(Tag::Number)] += 1;
        counts[TokenKind::other("FOO")] += 5;
        ISO_CHECK_EQ_UINT(counts[TokenKind::of(Tag::Name)], 2u);
        ISO_CHECK_EQ_UINT(counts[TokenKind::of(Tag::Number)], 1u);
        ISO_CHECK_EQ_UINT(counts[TokenKind::other("FOO")], 5u);
        ISO_CHECK(counts.find(TokenKind::other("BAR")) == counts.end());
    }

    return ISO_TEST_RESULT();
}

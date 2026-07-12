// Tests for the C++ dot-lexer, using the header-only iso_test.h harness (pure
// ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>

#include "dot_lexer.hpp"

namespace dot = ca::dot;
using K = dot::TokenKind;

static void chk(const dot::LexResult& r, std::size_t i, K kind,
                const std::string& value) {
    ISO_CHECK(i < r.tokens.size());
    if (i < r.tokens.size()) {
        ISO_CHECK(r.tokens[i].kind == kind);
        ISO_CHECK(r.tokens[i].value == value);
    }
}

int main() {
    // ── doc example ──────────────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("digraph G { A -> B }");
        ISO_CHECK(r.errors.empty());
        ISO_CHECK_EQ_UINT(r.tokens.size(), 8u);
        chk(r, 0, K::Digraph, "");
        chk(r, 1, K::Id, "G");
        chk(r, 2, K::LBrace, "");
        chk(r, 3, K::Id, "A");
        chk(r, 4, K::Arrow, "");
        chk(r, 5, K::Id, "B");
        chk(r, 6, K::RBrace, "");
        chk(r, 7, K::Eof, "");
        ISO_CHECK(r.tokens[0].line == 1 && r.tokens[0].col == 1);
    }

    // ── case-insensitive keywords ────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("STRICT Graph node EDGE SubGraph");
        chk(r, 0, K::Strict, "");
        chk(r, 1, K::Graph, "");
        chk(r, 2, K::Node, "");
        chk(r, 3, K::Edge, "");
        chk(r, 4, K::Subgraph, "");
    }

    // ── punctuation and -- ───────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("{}[]=;,: A -- B");
        chk(r, 0, K::LBrace, "");
        chk(r, 7, K::Colon, "");
        chk(r, 9, K::DashDash, "");
    }

    // ── numerals ─────────────────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("-42 3.14 .5");
        chk(r, 0, K::Id, "-42");
        chk(r, 1, K::Id, "3.14");
        chk(r, 2, K::Id, ".5");
        ISO_CHECK(r.errors.empty());
    }

    // ── quoted strings with escapes ──────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("\"hello world\" \"a\\\"b\" \"x\\ny\"");
        chk(r, 0, K::Id, "hello world");
        chk(r, 1, K::Id, "a\"b");
        chk(r, 2, K::Id, "x\ny");
    }

    // ── HTML string ──────────────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("<hello> <<b>x</b>>");
        chk(r, 0, K::Id, "hello");
        chk(r, 1, K::Id, "<b>x</b>");
    }

    // ── comments skipped ─────────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("a // line\nb /* block */ c");
        chk(r, 0, K::Id, "a");
        chk(r, 1, K::Id, "b");
        chk(r, 2, K::Id, "c");
    }

    // ── error recovery ───────────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("a @ b");
        ISO_CHECK_EQ_UINT(r.errors.size(), 1u);
        chk(r, 0, K::Id, "a");
        chk(r, 1, K::Id, "b");
    }
    {
        dot::LexResult r = dot::tokenise("\"unterminated");
        ISO_CHECK_EQ_UINT(r.errors.size(), 1u);
        chk(r, 0, K::Id, "unterminated");
    }

    // ── empty input ──────────────────────────────────────────────────────
    {
        dot::LexResult r = dot::tokenise("");
        ISO_CHECK_EQ_UINT(r.tokens.size(), 1u);
        chk(r, 0, K::Eof, "");
    }

    return ISO_TEST_RESULT();
}

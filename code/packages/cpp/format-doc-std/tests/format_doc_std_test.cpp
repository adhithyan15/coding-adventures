// Tests for the C++ format-doc-std library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>
#include <vector>

#include "format_doc.hpp"
#include "format_doc_std.hpp"

namespace fds = ca::format_doc_std;
namespace fd = ca::format_doc;
using fd::Doc;
using fd::text;

static std::string render(Doc doc, std::size_t width) {
    fd::LayoutOptions o;
    o.print_width = width;
    return fd::render_text(fd::layout_doc(doc, o));
}
static bool starts_with(const std::string& s, const std::string& p) {
    return s.rfind(p, 0) == 0;
}
static bool ends_with(const std::string& s, const std::string& p) {
    return s.size() >= p.size() && s.compare(s.size() - p.size(), p.size(), p) == 0;
}
static bool contains(const std::string& s, const std::string& p) {
    return s.find(p) != std::string::npos;
}

int main() {
    using fds::TrailingSeparator;

    // ── VERSION ───────────────────────────────────────────────────────────────
    ISO_CHECK(std::string(fds::kVersion) == "0.1.0");

    // ── delimited_list ────────────────────────────────────────────────────────
    ISO_CHECK(render(fds::delimited_list(text("["), {}, text("]")), 80) == "[]");
    {
        fds::DelimitedListConfig cfg;
        cfg.empty_spacing = true;
        ISO_CHECK(render(fds::delimited_list_with(text("["), {}, text("]"), cfg),
                         80) == "[ ]");
    }
    ISO_CHECK(render(fds::delimited_list(text("["),
                                         {text("a"), text("b"), text("c")},
                                         text("]")),
                     80) == "[a, b, c]");
    {
        auto s = render(
            fds::delimited_list(
                text("["),
                {text("aaaaaaaaaa"), text("bbbbbbbbbb"), text("cccccccccc")},
                text("]")),
            12);
        ISO_CHECK(contains(s, "\n") && starts_with(s, "[\n") && ends_with(s, "]"));
    }
    {
        fds::DelimitedListConfig cfg;
        cfg.separator = text(";");
        ISO_CHECK(render(fds::delimited_list_with(
                             text("("), {text("a"), text("b"), text("c")},
                             text(")"), cfg),
                         80) == "(a; b; c)");
    }
    {
        fds::DelimitedListConfig cfg;
        cfg.trailing_separator = TrailingSeparator::Always;
        ISO_CHECK(render(fds::delimited_list_with(text("["),
                                                  {text("a"), text("b")},
                                                  text("]"), cfg),
                         80) == "[a, b,]");
    }
    {
        fds::DelimitedListConfig cfg;
        cfg.trailing_separator = TrailingSeparator::IfBreak;
        ISO_CHECK(render(fds::delimited_list_with(text("["),
                                                  {text("a"), text("b")},
                                                  text("]"), cfg),
                         80) == "[a, b]");
    }
    {
        fds::DelimitedListConfig cfg;
        cfg.trailing_separator = TrailingSeparator::IfBreak;
        auto s = render(
            fds::delimited_list_with(
                text("["),
                {text("aaaaaaaa"), text("bbbbbbbb"), text("cccccccc")},
                text("]"), cfg),
            10);
        ISO_CHECK(contains(s, "\n") && contains(s, ",\n]"));
    }

    // ── call_like ─────────────────────────────────────────────────────────────
    ISO_CHECK(render(fds::call_like(text("print"),
                                    {text("a"), text("b"), text("c")},
                                    fds::CallLikeConfig{}),
                     80) == "print(a, b, c)");
    ISO_CHECK(render(fds::call_like(text("now"), {}, fds::CallLikeConfig{}),
                     80) == "now()");
    {
        auto s = render(fds::call_like(text("very_long_function_name"),
                                       {text("first_argument"),
                                        text("second_argument"),
                                        text("third_argument")},
                                       fds::CallLikeConfig{}),
                        30);
        ISO_CHECK(contains(s, "\n") &&
                  starts_with(s, "very_long_function_name("));
    }
    {
        fds::CallLikeConfig cfg;
        cfg.open = text("[");
        cfg.close = text("]");
        ISO_CHECK(render(fds::call_like(text("idx"), {text("0")}, cfg), 80) ==
                  "idx[0]");
    }

    // ── block_like ────────────────────────────────────────────────────────────
    ISO_CHECK(render(fds::block_like(text("{"), fd::nil(), text("}")), 80) ==
              "{ }");
    {
        fds::BlockLikeConfig cfg;
        cfg.empty_spacing = false;
        ISO_CHECK(render(fds::block_like_with(text("{"), fd::nil(), text("}"),
                                              cfg),
                         80) == "{}");
    }
    ISO_CHECK(render(fds::block_like(text("{"), text("body"), text("}")), 80) ==
              "{ body }");
    {
        auto s = render(
            fds::block_like(text("{"),
                            text("body_that_exceeds_print_width_to_force_break"),
                            text("}")),
            20);
        ISO_CHECK(contains(s, "\n") && starts_with(s, "{\n") &&
                  ends_with(s, "\n}"));
    }

    // ── infix_chain ───────────────────────────────────────────────────────────
    ISO_CHECK(render(fds::infix_chain({}, {}, fds::InfixChainConfig{}), 80) ==
              "");
    ISO_CHECK(render(fds::infix_chain({text("x")}, {}, fds::InfixChainConfig{}),
                     80) == "x");
    ISO_CHECK(render(fds::infix_chain({text("a"), text("b"), text("c")},
                                      {text("+"), text("-")},
                                      fds::InfixChainConfig{}),
                     80) == "a + b - c");
    {
        auto s = render(fds::infix_chain(
                            {text("aaaaaaaa"), text("bbbbbbbb"), text("cccccccc")},
                            {text("+"), text("-")}, fds::InfixChainConfig{}),
                        12);
        ISO_CHECK(contains(s, "\n"));
        auto nl = s.find('\n');
        ISO_CHECK(nl != std::string::npos && nl > 0 && s[nl - 1] == '+');
    }
    {
        fds::InfixChainConfig cfg;
        cfg.break_before_operators = true;
        auto s = render(fds::infix_chain(
                            {text("aaaaaaaa"), text("bbbbbbbb"), text("cccccccc")},
                            {text("+"), text("-")}, cfg),
                        12);
        auto nl = s.find('\n');
        ISO_CHECK(nl != std::string::npos);
        std::size_t i = nl + 1;
        while (i < s.size() && s[i] == ' ') ++i;
        ISO_CHECK(i < s.size() && s[i] == '+');
    }
    {  // arity mismatch throws (Rust panics)
        bool threw = false;
        try {
            fds::infix_chain({text("a"), text("b"), text("c")}, {text("+")},
                             fds::InfixChainConfig{});
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── composability ─────────────────────────────────────────────────────────
    {
        Doc sum = fds::infix_chain({text("x"), text("y")}, {text("+")},
                                   fds::InfixChainConfig{});
        Doc call = fds::call_like(text("print"), {sum, text("z")},
                                  fds::CallLikeConfig{});
        ISO_CHECK(render(call, 80) == "print(x + y, z)");
    }
    {
        auto inner = [](std::vector<Doc> xs) {
            return fds::delimited_list(text("["), std::move(xs), text("]"));
        };
        Doc outer = fds::delimited_list(
            text("["),
            {inner({text("a"), text("b")}), inner({text("c"), text("d")}),
             inner({text("e"), text("f")})},
            text("]"));
        ISO_CHECK(render(outer, 80) == "[[a, b], [c, d], [e, f]]");
    }

    return ISO_TEST_RESULT();
}

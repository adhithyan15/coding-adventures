// Tests for the C++ format-doc library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <string>
#include <vector>

#include "format_doc.hpp"

namespace fd = ca::format_doc;
using fd::Doc;

// Layout `doc` at `width` and render to a string.
static std::string render(const Doc& doc, std::size_t width) {
    fd::LayoutOptions o;
    o.print_width = width;
    return fd::render_text(fd::layout_doc(doc, o));
}

int main() {
    // ── builders ───────────────────────────────────────────────────────────
    ISO_CHECK(render(fd::nil(), 80) == "");
    ISO_CHECK(render(fd::text("hello"), 80) == "hello");
    ISO_CHECK(render(fd::text(""), 80) == "");  // empty collapses to nil
    ISO_CHECK(render(fd::concat({fd::text("a"),
                                 fd::concat({fd::text("b"), fd::text("c")}),
                                 fd::text("d")}),
                     80) == "abcd");
    ISO_CHECK(render(fd::concat({fd::text("a"), fd::nil(), fd::text("b"),
                                 fd::nil()}),
                     80) == "ab");
    ISO_CHECK(render(fd::concat({fd::text("only")}), 80) == "only");
    ISO_CHECK(render(fd::concat({}), 80) == "");

    ISO_CHECK(render(fd::join(fd::text(", "),
                              {fd::text("a"), fd::text("b"), fd::text("c")}),
                     80) == "a, b, c");
    ISO_CHECK(render(fd::join(fd::text(", "), {}), 80) == "");
    ISO_CHECK(render(fd::join(fd::text(", "), {fd::text("only")}), 80) == "only");
    ISO_CHECK(render(fd::indent(fd::text("x"), 0), 80) == "x");

    // ── line primitives inside groups ───────────────────────────────────────
    ISO_CHECK(render(fd::group(fd::concat({fd::text("a"), fd::line(),
                                           fd::text("b")})),
                     80) == "a b");
    ISO_CHECK(render(fd::group(fd::concat({fd::text("aaaaa"), fd::line(),
                                           fd::text("bbbbb")})),
                     5) == "aaaaa\nbbbbb");
    ISO_CHECK(render(fd::group(fd::concat({fd::text("a"), fd::softline(),
                                           fd::text("b")})),
                     80) == "ab");
    ISO_CHECK(render(fd::group(fd::concat({fd::text("aaaaa"), fd::softline(),
                                           fd::text("bbbbb")})),
                     5) == "aaaaa\nbbbbb");
    ISO_CHECK(render(fd::group(fd::concat({fd::text("a"), fd::hardline(),
                                           fd::text("b")})),
                     80) == "a\nb");

    // ── group flat / broken ─────────────────────────────────────────────────
    ISO_CHECK(render(fd::group(fd::concat({fd::text("("), fd::softline(),
                                           fd::text("x"), fd::softline(),
                                           fd::text(")")})),
                     80) == "(x)");
    ISO_CHECK(render(fd::group(fd::concat(
                         {fd::text("foo("),
                          fd::indent(fd::concat({fd::softline(), fd::text("bar,"),
                                                 fd::line(), fd::text("baz")}),
                                     1),
                          fd::softline(), fd::text(")")})),
                     8) == "foo(\n  bar,\n  baz\n)");
    {  // group broken uses indent_width setting (4)
        Doc doc = fd::group(fd::concat(
            {fd::text("a"),
             fd::indent(fd::concat({fd::hardline(), fd::text("b")}), 2)}));
        fd::LayoutOptions o;
        o.indent_width = 4;
        ISO_CHECK(fd::render_text(fd::layout_doc(doc, o)) == "a\n        b");
    }

    // ── if_break ─────────────────────────────────────────────────────────────
    ISO_CHECK(render(fd::group(fd::concat(
                         {fd::text("a"),
                          fd::if_break(fd::text("BROKEN"), fd::text("FLAT")),
                          fd::text("b")})),
                     80) == "aFLATb");
    ISO_CHECK(render(fd::group(fd::concat(
                         {fd::text("aaaaa"), fd::line(),
                          fd::if_break(fd::text("BROKEN"), fd::text("FLAT"))})),
                     5) == "aaaaa\nBROKEN");

    // ── annotations ──────────────────────────────────────────────────────────
    {
        Doc doc = fd::annotate(fd::ann_str("kw"), fd::text("if"));
        auto t = fd::layout_doc(doc, fd::LayoutOptions{});
        ISO_CHECK(t.lines.size() == 1 && t.lines[0].spans.size() == 1);
        std::vector<fd::DocAnnotation> want{fd::ann_str("kw")};
        ISO_CHECK((t.lines[0].spans[0].annotations == want));
    }
    {  // nested annotations accumulate outer-first
        Doc doc = fd::annotate(
            fd::ann_str("statement"),
            fd::annotate(fd::ann_str("keyword"), fd::text("if")));
        auto t = fd::layout_doc(doc, fd::LayoutOptions{});
        std::vector<fd::DocAnnotation> want{fd::ann_str("statement"),
                                            fd::ann_str("keyword")};
        ISO_CHECK((t.lines[0].spans[0].annotations == want));
    }
    {  // annotations do not change layout
        std::string plain = render(fd::text("hi there"), 80);
        std::string annot =
            render(fd::annotate(fd::ann_bool(true), fd::text("hi there")), 80);
        ISO_CHECK(plain == annot);
    }
    {  // span coalescing merges only when annotations match
        Doc same = fd::concat({fd::annotate(fd::ann_int(1), fd::text("foo")),
                               fd::annotate(fd::ann_int(1), fd::text("bar"))});
        auto t = fd::layout_doc(same, fd::LayoutOptions{});
        ISO_CHECK(t.lines[0].spans.size() == 1);
        ISO_CHECK(t.lines[0].spans[0].text == "foobar");

        Doc diff = fd::concat({fd::annotate(fd::ann_int(1), fd::text("foo")),
                               fd::annotate(fd::ann_int(2), fd::text("bar"))});
        auto t2 = fd::layout_doc(diff, fd::LayoutOptions{});
        ISO_CHECK(t2.lines[0].spans.size() == 2);
    }

    // ── layout metadata ──────────────────────────────────────────────────────
    {
        auto t = fd::layout_doc(fd::text("abc"), fd::LayoutOptions{});
        ISO_CHECK(t.print_width == 80);
        ISO_CHECK(t.indent_width == fd::kDefaultIndentWidth);
        ISO_CHECK(t.line_height == fd::kDefaultLineHeight);
        ISO_CHECK(t.width == 3);
        ISO_CHECK(t.height == 1);
    }
    {  // line_height multiplies height
        fd::LayoutOptions o;
        o.line_height = 3;
        auto t = fd::layout_doc(
            fd::concat({fd::text("a"), fd::hardline(), fd::text("b")}), o);
        ISO_CHECK(t.height == 6);
    }
    {  // lines have correct widths
        auto t = fd::layout_doc(
            fd::concat({fd::text("hello"), fd::hardline(), fd::text("world!")}),
            fd::LayoutOptions{});
        ISO_CHECK(t.lines[0].width == 5);
        ISO_CHECK(t.lines[1].width == 6);
        ISO_CHECK(t.width == 6);
    }
    ISO_CHECK(render(fd::concat({fd::text("a"),
                                 fd::indent(fd::concat({fd::hardline(),
                                                        fd::text("b")}),
                                            1)}),
                     80) == "a\n  b");
    ISO_CHECK(render(fd::concat({fd::hardline(), fd::hardline()}), 80) == "\n\n");

    // ── fits look-ahead ──────────────────────────────────────────────────────
    ISO_CHECK(render(fd::group(fd::concat({fd::text("a"), fd::line(),
                                           fd::text("b")})),
                     80) == "a b");
    ISO_CHECK(render(fd::group(fd::concat({fd::text("a"), fd::hardline(),
                                           fd::text("b")})),
                     1000) == "a\nb");
    {  // spec example: narrow breaks, wide stays flat
        Doc doc = fd::group(fd::concat(
            {fd::text("foo("),
             fd::indent(fd::concat({fd::softline(), fd::text("bar,"), fd::line(),
                                    fd::text("baz")}),
                        1),
             fd::softline(), fd::text(")")}));
        ISO_CHECK(render(doc, 8) == "foo(\n  bar,\n  baz\n)");
        ISO_CHECK(render(doc, 80) == "foo(bar, baz)");
    }

    // ── print_width == 0 throws ──────────────────────────────────────────────
    {
        bool threw = false;
        try {
            fd::LayoutOptions o;
            o.print_width = 0;
            (void)fd::layout_doc(fd::text("x"), o);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── newline normalisation ────────────────────────────────────────────────
    {
        auto t = fd::layout_doc(fd::text("a\nb"), fd::LayoutOptions{});
        ISO_CHECK(t.lines.size() == 2);
        ISO_CHECK(t.lines[0].spans[0].text == "a");
        ISO_CHECK(t.lines[0].spans[0].text.find('\n') == std::string::npos);
        ISO_CHECK(t.lines[1].spans[0].text == "b");
    }
    {
        auto t = fd::layout_doc(fd::text("a\r\nb\rc"), fd::LayoutOptions{});
        ISO_CHECK(t.lines.size() == 3);
        ISO_CHECK(t.lines[0].spans[0].text == "a");
        ISO_CHECK(t.lines[1].spans[0].text == "b");
        ISO_CHECK(t.lines[2].spans[0].text == "c");
    }
    {
        auto t = fd::layout_doc(fd::text("\n\n"), fd::LayoutOptions{});
        ISO_CHECK(t.lines.size() == 3);
        for (const auto& l : t.lines) ISO_CHECK(l.spans.empty());
    }

    // ── linearity of the borrowed-stack fits() ───────────────────────────────
    {  // 1000 nested groups
        Doc doc = fd::text("x");
        for (int i = 0; i < 1000; ++i) doc = fd::group(doc);
        ISO_CHECK(render(doc, 80) == "x");
    }
    {  // nested groups with concat siblings — just complete
        Doc doc = fd::concat({fd::text("end")});
        for (int i = 0; i < 500; ++i)
            doc = fd::group(fd::concat(
                {fd::text("L" + std::to_string(i) + "("), doc, fd::text(")")}));
        auto t = fd::layout_doc(doc, fd::LayoutOptions{});
        ISO_CHECK(!t.lines.empty());
    }
    {  // same doc + same options -> identical rendering
        Doc doc = fd::group(fd::concat(
            {fd::text("foo("),
             fd::indent(fd::concat({fd::softline(), fd::text("bar,"), fd::line(),
                                    fd::text("baz")}),
                        1),
             fd::softline(), fd::text(")")}));
        fd::LayoutOptions o;
        o.print_width = 8;
        ISO_CHECK(fd::render_text(fd::layout_doc(doc, o)) ==
                  fd::render_text(fd::layout_doc(doc, o)));
    }

    return ISO_TEST_RESULT();
}

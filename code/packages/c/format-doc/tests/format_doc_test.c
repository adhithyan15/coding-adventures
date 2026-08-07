/*
 * Tests for format-doc, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests: builder behaviour is checked by
 * rendering (the `FdDoc` tree is opaque), and layout metadata is checked by
 * inspecting the returned `FdLayoutTree`.
 */
#include "iso_test.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* free */
#include <string.h> /* strchr */

#include "format_doc.h"

/* Layout `doc` at `width`, render to a fresh string, and release both the
 * layout and the doc (builders/layout own their memory). */
static char *render(FdDoc *doc, size_t width) {
    FdLayoutOptions o = fd_layout_options_default();
    o.print_width = width;
    FdLayoutTree t = fd_layout_doc(doc, &o);
    char *s = fd_render_text(&t);
    fd_layout_free(&t);
    fd_free(doc);
    return s;
}

int main(void) {
    /* ── builders ──────────────────────────────────────────────────────────*/
    {
        char *s = render(fd_nil(), 80);
        ISO_CHECK_STR_EQ(s, "");
        free(s);
    }
    {
        char *s = render(fd_text("hello"), 80);
        ISO_CHECK_STR_EQ(s, "hello");
        free(s);
    }
    {
        char *s = render(fd_text(""), 80); /* empty text collapses to nil */
        ISO_CHECK_STR_EQ(s, "");
        free(s);
    }
    { /* concat flattens nested */
        FdDoc *inner[2] = {fd_text("b"), fd_text("c")};
        FdDoc *outer[3] = {fd_text("a"), fd_concat(inner, 2), fd_text("d")};
        char *s = render(fd_concat(outer, 3), 80);
        ISO_CHECK_STR_EQ(s, "abcd");
        free(s);
    }
    { /* concat drops nil */
        FdDoc *p[4] = {fd_text("a"), fd_nil(), fd_text("b"), fd_nil()};
        char *s = render(fd_concat(p, 4), 80);
        ISO_CHECK_STR_EQ(s, "ab");
        free(s);
    }
    { /* concat singleton unwraps */
        FdDoc *p[1] = {fd_text("only")};
        char *s = render(fd_concat(p, 1), 80);
        ISO_CHECK_STR_EQ(s, "only");
        free(s);
    }
    { /* concat empty is nil */
        char *s = render(fd_concat(NULL, 0), 80);
        ISO_CHECK_STR_EQ(s, "");
        free(s);
    }
    { /* join basic */
        FdDoc *p[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
        char *s = render(fd_join(fd_text(", "), p, 3), 80);
        ISO_CHECK_STR_EQ(s, "a, b, c");
        free(s);
    }
    { /* join empty is nil */
        char *s = render(fd_join(fd_text(", "), NULL, 0), 80);
        ISO_CHECK_STR_EQ(s, "");
        free(s);
    }
    { /* join singleton: no separator */
        FdDoc *p[1] = {fd_text("only")};
        char *s = render(fd_join(fd_text(", "), p, 1), 80);
        ISO_CHECK_STR_EQ(s, "only");
        free(s);
    }
    { /* indent 0 levels is a no-op */
        char *s = render(fd_indent(fd_text("x"), 0), 80);
        ISO_CHECK_STR_EQ(s, "x");
        free(s);
    }

    /* ── line primitives inside groups ─────────────────────────────────────*/
    { /* line flat -> space */
        FdDoc *p[3] = {fd_text("a"), fd_line(), fd_text("b")};
        char *s = render(fd_group(fd_concat(p, 3)), 80);
        ISO_CHECK_STR_EQ(s, "a b");
        free(s);
    }
    { /* line broken -> newline + indent */
        FdDoc *p[3] = {fd_text("aaaaa"), fd_line(), fd_text("bbbbb")};
        char *s = render(fd_group(fd_concat(p, 3)), 5);
        ISO_CHECK_STR_EQ(s, "aaaaa\nbbbbb");
        free(s);
    }
    { /* softline flat -> empty */
        FdDoc *p[3] = {fd_text("a"), fd_softline(), fd_text("b")};
        char *s = render(fd_group(fd_concat(p, 3)), 80);
        ISO_CHECK_STR_EQ(s, "ab");
        free(s);
    }
    { /* softline broken -> newline */
        FdDoc *p[3] = {fd_text("aaaaa"), fd_softline(), fd_text("bbbbb")};
        char *s = render(fd_group(fd_concat(p, 3)), 5);
        ISO_CHECK_STR_EQ(s, "aaaaa\nbbbbb");
        free(s);
    }
    { /* hardline always breaks */
        FdDoc *p[3] = {fd_text("a"), fd_hardline(), fd_text("b")};
        char *s = render(fd_group(fd_concat(p, 3)), 80);
        ISO_CHECK_STR_EQ(s, "a\nb");
        free(s);
    }

    /* ── group flat / broken decisions ─────────────────────────────────────*/
    { /* group flat when it fits */
        FdDoc *p[5] = {fd_text("("), fd_softline(), fd_text("x"),
                       fd_softline(), fd_text(")")};
        char *s = render(fd_group(fd_concat(p, 5)), 80);
        ISO_CHECK_STR_EQ(s, "(x)");
        free(s);
    }
    { /* group broken when it does not fit */
        FdDoc *inner[4] = {fd_softline(), fd_text("bar,"), fd_line(),
                           fd_text("baz")};
        FdDoc *p[4] = {fd_text("foo("), fd_indent(fd_concat(inner, 4), 1),
                       fd_softline(), fd_text(")")};
        char *s = render(fd_group(fd_concat(p, 4)), 8);
        ISO_CHECK_STR_EQ(s, "foo(\n  bar,\n  baz\n)");
        free(s);
    }
    { /* group broken uses indent_width setting (4) */
        FdDoc *inner[2] = {fd_hardline(), fd_text("b")};
        FdDoc *p[2] = {fd_text("a"), fd_indent(fd_concat(inner, 2), 2)};
        FdLayoutOptions o = fd_layout_options_default();
        o.indent_width = 4;
        FdDoc *doc = fd_group(fd_concat(p, 2));
        FdLayoutTree t = fd_layout_doc(doc, &o);
        char *s = fd_render_text(&t);
        ISO_CHECK_STR_EQ(s, "a\n        b");
        free(s);
        fd_layout_free(&t);
        fd_free(doc);
    }

    /* ── if_break ──────────────────────────────────────────────────────────*/
    { /* if_break picks flat when flat */
        FdDoc *p[3] = {fd_text("a"),
                       fd_if_break(fd_text("BROKEN"), fd_text("FLAT")),
                       fd_text("b")};
        char *s = render(fd_group(fd_concat(p, 3)), 80);
        ISO_CHECK_STR_EQ(s, "aFLATb");
        free(s);
    }
    { /* if_break picks broken when broken */
        FdDoc *p[3] = {fd_text("aaaaa"), fd_line(),
                       fd_if_break(fd_text("BROKEN"), fd_text("FLAT"))};
        char *s = render(fd_group(fd_concat(p, 3)), 5);
        ISO_CHECK_STR_EQ(s, "aaaaa\nBROKEN");
        free(s);
    }

    /* ── annotations ───────────────────────────────────────────────────────*/
    { /* annotations attach to emitted spans */
        FdDoc *doc = fd_annotate(fd_ann_str("kw"), fd_text("if"));
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.n_lines == 1 && t.lines[0].n_spans == 1);
        ISO_CHECK(t.lines[0].spans[0].n_annotations == 1);
        FdAnnotation want = fd_ann_str("kw");
        ISO_CHECK(fd_ann_equal(&t.lines[0].spans[0].annotations[0], &want));
        fd_ann_free(&want);
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* nested annotations accumulate outer-first */
        FdDoc *doc =
            fd_annotate(fd_ann_str("statement"),
                        fd_annotate(fd_ann_str("keyword"), fd_text("if")));
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.lines[0].spans[0].n_annotations == 2);
        FdAnnotation outer = fd_ann_str("statement");
        FdAnnotation inner = fd_ann_str("keyword");
        ISO_CHECK(fd_ann_equal(&t.lines[0].spans[0].annotations[0], &outer));
        ISO_CHECK(fd_ann_equal(&t.lines[0].spans[0].annotations[1], &inner));
        fd_ann_free(&outer);
        fd_ann_free(&inner);
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* annotations do not change layout */
        char *plain = render(fd_text("hi there"), 80);
        char *annot =
            render(fd_annotate(fd_ann_bool(1), fd_text("hi there")), 80);
        ISO_CHECK_STR_EQ(annot, plain);
        free(plain);
        free(annot);
    }
    { /* span coalescing merges only when annotations match */
        FdDoc *same[2] = {fd_annotate(fd_ann_int(1), fd_text("foo")),
                          fd_annotate(fd_ann_int(1), fd_text("bar"))};
        FdDoc *doc = fd_concat(same, 2);
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.lines[0].n_spans == 1);
        ISO_CHECK_STR_EQ(t.lines[0].spans[0].text, "foobar");
        fd_layout_free(&t);
        fd_free(doc);

        FdDoc *diff[2] = {fd_annotate(fd_ann_int(1), fd_text("foo")),
                          fd_annotate(fd_ann_int(2), fd_text("bar"))};
        FdDoc *doc2 = fd_concat(diff, 2);
        FdLayoutTree t2 = fd_layout_doc(doc2, &o);
        ISO_CHECK(t2.lines[0].n_spans == 2);
        fd_layout_free(&t2);
        fd_free(doc2);
    }

    /* ── layout metadata ───────────────────────────────────────────────────*/
    { /* records print_width and dimensions */
        FdLayoutOptions o = fd_layout_options_default();
        FdDoc *doc = fd_text("abc");
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.print_width == 80);
        ISO_CHECK(t.indent_width == FD_DEFAULT_INDENT_WIDTH);
        ISO_CHECK(t.line_height == FD_DEFAULT_LINE_HEIGHT);
        ISO_CHECK(t.width == 3);
        ISO_CHECK(t.height == 1);
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* line_height multiplies height */
        FdDoc *p[3] = {fd_text("a"), fd_hardline(), fd_text("b")};
        FdDoc *doc = fd_concat(p, 3);
        FdLayoutOptions o = {80, 2, 3};
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.height == 6);
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* lines have correct widths */
        FdDoc *p[3] = {fd_text("hello"), fd_hardline(), fd_text("world!")};
        FdDoc *doc = fd_concat(p, 3);
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.lines[0].width == 5);
        ISO_CHECK(t.lines[1].width == 6);
        ISO_CHECK(t.width == 6);
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* render_text handles indented lines */
        FdDoc *inner[2] = {fd_hardline(), fd_text("b")};
        FdDoc *p[2] = {fd_text("a"), fd_indent(fd_concat(inner, 2), 1)};
        char *s = render(fd_concat(p, 2), 80);
        ISO_CHECK_STR_EQ(s, "a\n  b");
        free(s);
    }
    { /* render_text handles blank lines */
        FdDoc *p[2] = {fd_hardline(), fd_hardline()};
        char *s = render(fd_concat(p, 2), 80);
        ISO_CHECK_STR_EQ(s, "\n\n");
        free(s);
    }

    /* ── fits look-ahead ───────────────────────────────────────────────────*/
    { /* fits succeeds when content fits */
        FdDoc *p[3] = {fd_text("a"), fd_line(), fd_text("b")};
        char *s = render(fd_group(fd_concat(p, 3)), 80);
        ISO_CHECK_STR_EQ(s, "a b");
        free(s);
    }
    { /* fits fails on hardline inside group */
        FdDoc *p[3] = {fd_text("a"), fd_hardline(), fd_text("b")};
        char *s = render(fd_group(fd_concat(p, 3)), 1000);
        ISO_CHECK_STR_EQ(s, "a\nb");
        free(s);
    }
    { /* spec example: narrow breaks, wide stays flat */
        FdDoc *inner[4] = {fd_softline(), fd_text("bar,"), fd_line(),
                           fd_text("baz")};
        FdDoc *p[4] = {fd_text("foo("), fd_indent(fd_concat(inner, 4), 1),
                       fd_softline(), fd_text(")")};
        FdDoc *doc = fd_group(fd_concat(p, 4));
        FdLayoutOptions narrow = fd_layout_options_default();
        narrow.print_width = 8;
        FdLayoutTree tn = fd_layout_doc(doc, &narrow);
        char *sn = fd_render_text(&tn);
        ISO_CHECK_STR_EQ(sn, "foo(\n  bar,\n  baz\n)");
        free(sn);
        fd_layout_free(&tn);

        FdLayoutOptions wide = fd_layout_options_default();
        FdLayoutTree tw = fd_layout_doc(doc, &wide);
        char *sw = fd_render_text(&tw);
        ISO_CHECK_STR_EQ(sw, "foo(bar, baz)");
        free(sw);
        fd_layout_free(&tw);
        fd_free(doc);
    }

    /* ── newline normalisation in text ─────────────────────────────────────*/
    { /* LF auto-splits on hardline; spans stay single-line */
        FdDoc *doc = fd_text("a\nb");
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.n_lines == 2);
        ISO_CHECK_STR_EQ(t.lines[0].spans[0].text, "a");
        ISO_CHECK(strchr(t.lines[0].spans[0].text, '\n') == NULL);
        ISO_CHECK_STR_EQ(t.lines[1].spans[0].text, "b");
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* CRLF and CR both normalise to LF */
        FdDoc *doc = fd_text("a\r\nb\rc");
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.n_lines == 3);
        ISO_CHECK_STR_EQ(t.lines[0].spans[0].text, "a");
        ISO_CHECK_STR_EQ(t.lines[1].spans[0].text, "b");
        ISO_CHECK_STR_EQ(t.lines[2].spans[0].text, "c");
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* text of only newlines produces blank lines */
        FdDoc *doc = fd_text("\n\n");
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.n_lines == 3);
        for (size_t i = 0; i < t.n_lines; i++)
            ISO_CHECK(t.lines[i].n_spans == 0);
        fd_layout_free(&t);
        fd_free(doc);
    }

    /* ── stack-safety / linearity of the borrowed-stack fits() ─────────────*/
    { /* 1000 nested groups render to "x" (was O(N^2) with the naive fits) */
        FdDoc *doc = fd_text("x");
        for (int i = 0; i < 1000; i++) doc = fd_group(doc);
        char *s = render(doc, 80);
        ISO_CHECK_STR_EQ(s, "x");
        free(s);
    }
    { /* nested groups with concat siblings: complete without blowing up */
        FdDoc *inner[1] = {fd_text("end")};
        FdDoc *doc = fd_concat(inner, 1);
        for (int i = 0; i < 500; i++) {
            char label[32];
            snprintf(label, sizeof label, "L%d(", i);
            FdDoc *p[3] = {fd_text(label), doc, fd_text(")")};
            doc = fd_group(fd_concat(p, 3));
        }
        FdLayoutOptions o = fd_layout_options_default();
        FdLayoutTree t = fd_layout_doc(doc, &o);
        ISO_CHECK(t.n_lines >= 1);
        fd_layout_free(&t);
        fd_free(doc);
    }
    { /* same doc + same options -> identical rendering (determinism) */
        FdDoc *inner[4] = {fd_softline(), fd_text("bar,"), fd_line(),
                           fd_text("baz")};
        FdDoc *p[4] = {fd_text("foo("), fd_indent(fd_concat(inner, 4), 1),
                       fd_softline(), fd_text(")")};
        FdDoc *doc = fd_group(fd_concat(p, 4));
        FdLayoutOptions o = fd_layout_options_default();
        o.print_width = 8;
        FdLayoutTree a = fd_layout_doc(doc, &o);
        FdLayoutTree b = fd_layout_doc(doc, &o);
        char *sa = fd_render_text(&a);
        char *sb = fd_render_text(&b);
        ISO_CHECK_STR_EQ(sa, sb);
        free(sa);
        free(sb);
        fd_layout_free(&a);
        fd_layout_free(&b);
        fd_free(doc);
    }

    return ISO_TEST_RESULT();
}

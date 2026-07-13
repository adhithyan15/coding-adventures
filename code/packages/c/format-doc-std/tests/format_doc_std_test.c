/*
 * Tests for format-doc-std, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests. Each template is laid out and
 * rendered through format-doc to check the resulting string.
 */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* strcmp, strstr, strncmp, strlen, strchr */

#include "format_doc.h"
#include "format_doc_std.h"

/* Lay out `doc` at `width` and render to a fresh string; frees the doc. */
static char *render(FdDoc *doc, size_t width) {
    FdLayoutOptions o = fd_layout_options_default();
    o.print_width = width;
    FdLayoutTree t = fd_layout_doc(doc, &o);
    char *s = fd_render_text(&t);
    fd_layout_free(&t);
    fd_free(doc);
    return s;
}

static int starts_with(const char *s, const char *pre) {
    return strncmp(s, pre, strlen(pre)) == 0;
}
static int ends_with(const char *s, const char *suf) {
    size_t ls = strlen(s), lf = strlen(suf);
    return ls >= lf && strcmp(s + ls - lf, suf) == 0;
}

int main(void) {
    /* ── VERSION ──────────────────────────────────────────────────────────────*/
    ISO_CHECK_STR_EQ(fds_version, "0.1.0");

    /* ── delimited_list ───────────────────────────────────────────────────────*/
    { /* empty, default */
        char *s = render(fds_delimited_list(fd_text("["), NULL, 0, fd_text("]")),
                         80);
        ISO_CHECK_STR_EQ(s, "[]");
        free(s);
    }
    { /* empty, with spacing */
        FdDoc *sep = fd_text(",");
        FdsDelimitedListConfig cfg = {sep, FDS_TRAILING_NEVER, 1};
        char *s = render(fds_delimited_list_with(fd_text("["), NULL, 0,
                                                 fd_text("]"), &cfg),
                         80);
        ISO_CHECK_STR_EQ(s, "[ ]");
        free(s);
        fd_free(sep);
    }
    { /* flat when it fits */
        FdDoc *items[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
        char *s = render(
            fds_delimited_list(fd_text("["), items, 3, fd_text("]")), 80);
        ISO_CHECK_STR_EQ(s, "[a, b, c]");
        free(s);
    }
    { /* broken when it does not fit */
        FdDoc *items[3] = {fd_text("aaaaaaaaaa"), fd_text("bbbbbbbbbb"),
                           fd_text("cccccccccc")};
        char *s = render(
            fds_delimited_list(fd_text("["), items, 3, fd_text("]")), 12);
        ISO_CHECK(strchr(s, '\n') != NULL);
        ISO_CHECK(starts_with(s, "[\n"));
        ISO_CHECK(ends_with(s, "]"));
        free(s);
    }
    { /* custom separator */
        FdDoc *sep = fd_text(";");
        FdsDelimitedListConfig cfg = {sep, FDS_TRAILING_NEVER, 0};
        FdDoc *items[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
        char *s = render(fds_delimited_list_with(fd_text("("), items, 3,
                                                 fd_text(")"), &cfg),
                         80);
        ISO_CHECK_STR_EQ(s, "(a; b; c)");
        free(s);
        fd_free(sep);
    }
    { /* trailing Always, flat */
        FdDoc *sep = fd_text(",");
        FdsDelimitedListConfig cfg = {sep, FDS_TRAILING_ALWAYS, 0};
        FdDoc *items[2] = {fd_text("a"), fd_text("b")};
        char *s = render(fds_delimited_list_with(fd_text("["), items, 2,
                                                 fd_text("]"), &cfg),
                         80);
        ISO_CHECK_STR_EQ(s, "[a, b,]");
        free(s);
        fd_free(sep);
    }
    { /* trailing IfBreak, flat omits */
        FdDoc *sep = fd_text(",");
        FdsDelimitedListConfig cfg = {sep, FDS_TRAILING_IF_BREAK, 0};
        FdDoc *items[2] = {fd_text("a"), fd_text("b")};
        char *s = render(fds_delimited_list_with(fd_text("["), items, 2,
                                                 fd_text("]"), &cfg),
                         80);
        ISO_CHECK_STR_EQ(s, "[a, b]");
        free(s);
        fd_free(sep);
    }
    { /* trailing IfBreak, broken emits */
        FdDoc *sep = fd_text(",");
        FdsDelimitedListConfig cfg = {sep, FDS_TRAILING_IF_BREAK, 0};
        FdDoc *items[3] = {fd_text("aaaaaaaa"), fd_text("bbbbbbbb"),
                           fd_text("cccccccc")};
        char *s = render(fds_delimited_list_with(fd_text("["), items, 3,
                                                 fd_text("]"), &cfg),
                         10);
        ISO_CHECK(strchr(s, '\n') != NULL);
        ISO_CHECK(strstr(s, ",\n]") != NULL);
        free(s);
        fd_free(sep);
    }

    /* ── call_like ────────────────────────────────────────────────────────────*/
    { /* default parens and commas */
        FdDoc *o = fd_text("("), *c = fd_text(")"), *sp = fd_text(",");
        FdsCallLikeConfig cfg = {o, c, sp, FDS_TRAILING_NEVER};
        FdDoc *args[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
        char *s = render(fds_call_like(fd_text("print"), args, 3, &cfg), 80);
        ISO_CHECK_STR_EQ(s, "print(a, b, c)");
        free(s);
        fd_free(o);
        fd_free(c);
        fd_free(sp);
    }
    { /* empty args */
        FdDoc *o = fd_text("("), *c = fd_text(")"), *sp = fd_text(",");
        FdsCallLikeConfig cfg = {o, c, sp, FDS_TRAILING_NEVER};
        char *s = render(fds_call_like(fd_text("now"), NULL, 0, &cfg), 80);
        ISO_CHECK_STR_EQ(s, "now()");
        free(s);
        fd_free(o);
        fd_free(c);
        fd_free(sp);
    }
    { /* breaks when args too long */
        FdDoc *o = fd_text("("), *c = fd_text(")"), *sp = fd_text(",");
        FdsCallLikeConfig cfg = {o, c, sp, FDS_TRAILING_NEVER};
        FdDoc *args[3] = {fd_text("first_argument"), fd_text("second_argument"),
                          fd_text("third_argument")};
        char *s = render(
            fds_call_like(fd_text("very_long_function_name"), args, 3, &cfg),
            30);
        ISO_CHECK(strchr(s, '\n') != NULL);
        ISO_CHECK(starts_with(s, "very_long_function_name("));
        free(s);
        fd_free(o);
        fd_free(c);
        fd_free(sp);
    }
    { /* custom brackets */
        FdDoc *o = fd_text("["), *c = fd_text("]"), *sp = fd_text(",");
        FdsCallLikeConfig cfg = {o, c, sp, FDS_TRAILING_NEVER};
        FdDoc *args[1] = {fd_text("0")};
        char *s = render(fds_call_like(fd_text("idx"), args, 1, &cfg), 80);
        ISO_CHECK_STR_EQ(s, "idx[0]");
        free(s);
        fd_free(o);
        fd_free(c);
        fd_free(sp);
    }

    /* ── block_like ───────────────────────────────────────────────────────────*/
    { /* default empty spacing */
        char *s = render(fds_block_like(fd_text("{"), fd_nil(), fd_text("}")),
                         80);
        ISO_CHECK_STR_EQ(s, "{ }");
        free(s);
    }
    { /* empty, no spacing */
        FdsBlockLikeConfig cfg = {0};
        char *s = render(
            fds_block_like_with(fd_text("{"), fd_nil(), fd_text("}"), &cfg), 80);
        ISO_CHECK_STR_EQ(s, "{}");
        free(s);
    }
    { /* inline when it fits */
        char *s = render(
            fds_block_like(fd_text("{"), fd_text("body"), fd_text("}")), 80);
        ISO_CHECK_STR_EQ(s, "{ body }");
        free(s);
    }
    { /* breaks when body too long */
        char *s = render(
            fds_block_like(
                fd_text("{"),
                fd_text("body_that_exceeds_print_width_to_force_break"),
                fd_text("}")),
            20);
        ISO_CHECK(strchr(s, '\n') != NULL);
        ISO_CHECK(starts_with(s, "{\n"));
        ISO_CHECK(ends_with(s, "\n}"));
        free(s);
    }

    /* ── infix_chain ──────────────────────────────────────────────────────────*/
    { /* empty is nil */
        FdsInfixChainConfig cfg = fds_infix_chain_config_default();
        char *s = render(fds_infix_chain(NULL, 0, NULL, 0, &cfg), 80);
        ISO_CHECK_STR_EQ(s, "");
        free(s);
    }
    { /* single operand unchanged */
        FdsInfixChainConfig cfg = fds_infix_chain_config_default();
        FdDoc *ops[1] = {fd_text("x")};
        char *s = render(fds_infix_chain(ops, 1, NULL, 0, &cfg), 80);
        ISO_CHECK_STR_EQ(s, "x");
        free(s);
    }
    { /* break-after operators (default), flat */
        FdsInfixChainConfig cfg = fds_infix_chain_config_default();
        FdDoc *operands[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
        FdDoc *operators[2] = {fd_text("+"), fd_text("-")};
        char *s = render(fds_infix_chain(operands, 3, operators, 2, &cfg), 80);
        ISO_CHECK_STR_EQ(s, "a + b - c");
        free(s);
    }
    { /* break-after, broken: first line ends with '+' */
        FdsInfixChainConfig cfg = fds_infix_chain_config_default();
        FdDoc *operands[3] = {fd_text("aaaaaaaa"), fd_text("bbbbbbbb"),
                              fd_text("cccccccc")};
        FdDoc *operators[2] = {fd_text("+"), fd_text("-")};
        char *s = render(fds_infix_chain(operands, 3, operators, 2, &cfg), 12);
        ISO_CHECK(strchr(s, '\n') != NULL);
        char *nl = strchr(s, '\n');
        ISO_CHECK(nl != NULL && nl > s && *(nl - 1) == '+');
        free(s);
    }
    { /* break-before operators: line 2 (after indent) starts with '+' */
        FdsInfixChainConfig cfg = {1};
        FdDoc *operands[3] = {fd_text("aaaaaaaa"), fd_text("bbbbbbbb"),
                              fd_text("cccccccc")};
        FdDoc *operators[2] = {fd_text("+"), fd_text("-")};
        char *s = render(fds_infix_chain(operands, 3, operators, 2, &cfg), 12);
        char *nl = strchr(s, '\n');
        ISO_CHECK(nl != NULL);
        const char *line2 = nl + 1;
        while (*line2 == ' ') line2++;
        ISO_CHECK(*line2 == '+');
        free(s);
    }
    { /* arity mismatch → NULL (Rust panics; the C port fails closed) */
        FdsInfixChainConfig cfg = fds_infix_chain_config_default();
        FdDoc *operands[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
        FdDoc *operators[1] = {fd_text("+")};
        ISO_CHECK(fds_infix_chain(operands, 3, operators, 1, &cfg) == NULL);
    }

    /* ── composability ────────────────────────────────────────────────────────*/
    { /* print(x + y, z) */
        FdsInfixChainConfig icfg = fds_infix_chain_config_default();
        FdDoc *operands[2] = {fd_text("x"), fd_text("y")};
        FdDoc *operators[1] = {fd_text("+")};
        FdDoc *sum = fds_infix_chain(operands, 2, operators, 1, &icfg);
        FdDoc *o = fd_text("("), *c = fd_text(")"), *sp = fd_text(",");
        FdsCallLikeConfig ccfg = {o, c, sp, FDS_TRAILING_NEVER};
        FdDoc *args[2] = {sum, fd_text("z")};
        char *s = render(fds_call_like(fd_text("print"), args, 2, &ccfg), 80);
        ISO_CHECK_STR_EQ(s, "print(x + y, z)");
        free(s);
        fd_free(o);
        fd_free(c);
        fd_free(sp);
    }
    { /* nested delimited lists fit flat */
        FdDoc *l1[2] = {fd_text("a"), fd_text("b")};
        FdDoc *l2[2] = {fd_text("c"), fd_text("d")};
        FdDoc *l3[2] = {fd_text("e"), fd_text("f")};
        FdDoc *inner[3] = {
            fds_delimited_list(fd_text("["), l1, 2, fd_text("]")),
            fds_delimited_list(fd_text("["), l2, 2, fd_text("]")),
            fds_delimited_list(fd_text("["), l3, 2, fd_text("]"))};
        char *s = render(
            fds_delimited_list(fd_text("["), inner, 3, fd_text("]")), 80);
        ISO_CHECK_STR_EQ(s, "[[a, b], [c, d], [e, f]]");
        free(s);
    }

    return ISO_TEST_RESULT();
}

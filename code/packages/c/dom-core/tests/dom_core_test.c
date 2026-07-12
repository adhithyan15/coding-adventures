/*
 * Tests for the C dom-core tree model, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own unit test, extended to cover
 * every node kind, namespaced elements, nested children, and the accessors.
 */
#include "iso_test.h"

#include <stddef.h>

#include "dom_core.h"

int main(void) {
    /* ── a document holding element, text, and comment nodes ────────────── */
    {
        DomDocument *doc = dom_document_new();
        ISO_CHECK(doc != NULL);

        DomAttribute attrs[] = {{"class", "intro"}};
        ISO_CHECK(dom_document_push_child(doc, dom_element("p", attrs, 1)) == 0);
        ISO_CHECK(dom_document_push_child(doc, dom_text("hello")) == 0);
        ISO_CHECK(dom_document_push_child(doc, dom_comment("note")) == 0);

        size_t n = 0;
        DomNode *const *kids = dom_document_children(doc, &n);
        ISO_CHECK_EQ_UINT(n, 3u);

        ISO_CHECK(dom_node_kind(kids[0]) == DOM_ELEMENT);
        ISO_CHECK_STR_EQ(dom_element_name(kids[0]), "p");
        ISO_CHECK_STR_EQ(dom_element_attribute(kids[0], "class"), "intro");
        ISO_CHECK(dom_element_attribute(kids[0], "id") == NULL); /* absent */

        ISO_CHECK(dom_node_kind(kids[1]) == DOM_TEXT);
        ISO_CHECK_STR_EQ(dom_text_data(kids[1]), "hello");

        ISO_CHECK(dom_node_kind(kids[2]) == DOM_COMMENT);
        ISO_CHECK_STR_EQ(dom_comment_data(kids[2]), "note");

        dom_document_free(doc);
    }

    /* ── nested elements via append_child, and children() ───────────────── */
    {
        DomNode *div = dom_element("div", NULL, 0);
        ISO_CHECK(dom_element_append_child(div, dom_element("span", NULL, 0)) == 0);
        ISO_CHECK(dom_element_append_child(div, dom_text("inner")) == 0);

        size_t n = 0;
        DomNode *const *kids = dom_node_children(div, &n);
        ISO_CHECK(kids != NULL);
        ISO_CHECK_EQ_UINT(n, 2u);
        ISO_CHECK_STR_EQ(dom_element_name(kids[0]), "span");
        ISO_CHECK(dom_node_kind(kids[1]) == DOM_TEXT);
        ISO_CHECK_STR_EQ(dom_text_data(kids[1]), "inner");

        dom_node_free(div);
    }

    /* ── children() on a non-element is NULL (Rust: Option::None) ───────── */
    {
        DomNode *t = dom_text("x");
        size_t n = 123;
        ISO_CHECK(dom_node_children(t, &n) == NULL);
        ISO_CHECK_EQ_UINT(n, 0u);
        /* append_child to a non-element fails without consuming the caller's
         * detached child, which we then free ourselves. */
        DomNode *orphan = dom_comment("c");
        ISO_CHECK(dom_element_append_child(t, orphan) == -1);
        dom_node_free(orphan);
        dom_node_free(t);
    }

    /* ── namespaced element ─────────────────────────────────────────────── */
    {
        DomAttribute a[] = {{"viewBox", "0 0 10 10"}};
        DomNode *svg =
            dom_namespaced_element("http://www.w3.org/2000/svg", "svg", a, 1);
        ISO_CHECK_STR_EQ(dom_element_namespace(svg),
                         "http://www.w3.org/2000/svg");
        ISO_CHECK_STR_EQ(dom_element_name(svg), "svg");
        ISO_CHECK_STR_EQ(dom_element_attribute(svg, "viewBox"), "0 0 10 10");
        /* A plain element has no namespace. */
        DomNode *p = dom_element("p", NULL, 0);
        ISO_CHECK(dom_element_namespace(p) == NULL);
        dom_node_free(svg);
        dom_node_free(p);
    }

    /* ── doctype, with optional fields present and absent ───────────────── */
    {
        DomNode *dt = dom_doctype("html", NULL, NULL, 0);
        ISO_CHECK(dom_node_kind(dt) == DOM_DOCUMENT_TYPE);
        ISO_CHECK_STR_EQ(dom_doctype_name(dt), "html");
        ISO_CHECK(dom_doctype_public_id(dt) == NULL);
        ISO_CHECK(dom_doctype_system_id(dt) == NULL);
        ISO_CHECK_EQ_INT(dom_doctype_force_quirks(dt), 0);
        dom_node_free(dt);

        DomNode *legacy = dom_doctype("html", "-//W3C//DTD HTML 4.01//EN",
                                      "http://www.w3.org/TR/html4/strict.dtd", 1);
        ISO_CHECK_STR_EQ(dom_doctype_public_id(legacy),
                         "-//W3C//DTD HTML 4.01//EN");
        ISO_CHECK_STR_EQ(dom_doctype_system_id(legacy),
                         "http://www.w3.org/TR/html4/strict.dtd");
        ISO_CHECK_EQ_INT(dom_doctype_force_quirks(legacy), 1);
        dom_node_free(legacy);
    }

    /* ── accessors return NULL for the wrong node kind ──────────────────── */
    {
        DomNode *e = dom_element("p", NULL, 0);
        ISO_CHECK(dom_text_data(e) == NULL);
        ISO_CHECK(dom_comment_data(e) == NULL);
        ISO_CHECK(dom_doctype_name(e) == NULL);
        DomNode *t = dom_text("hi");
        ISO_CHECK(dom_element_name(t) == NULL);
        ISO_CHECK(dom_element_attribute(t, "x") == NULL);
        dom_node_free(e);
        dom_node_free(t);
    }

    return ISO_TEST_RESULT();
}

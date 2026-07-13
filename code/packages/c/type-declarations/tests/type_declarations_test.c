/*
 * Tests for the C type-declarations library, using the header-only iso_test.h
 * harness (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "type_declarations.h"

int main(void) {
    /* ── KindDecl::to_iir_hint ───────────────────────────────────────────── */
    {
        TdKind k;
        k = td_kind_int();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "i64");
        k = td_kind_bool();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "bool");
        k = td_kind_str();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "str");
        k = td_kind_function(1);
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "closure");
        k = td_kind_function(0);
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "closure");
        /* All non-concrete kinds map to "any". */
        k = td_kind_any();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "any");
        k = td_kind_nil();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "any");
        k = td_kind_symbol();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "any");
        k = td_kind_list();
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "any");
        ISO_CHECK(td_kind_named("Foo", &k) == 0);
        ISO_CHECK_STR_EQ(td_kind_to_iir_hint(&k), "any");
        td_kind_free(&k);
    }

    /* ── is_concrete_hint ────────────────────────────────────────────────── */
    {
        TdKind i = td_kind_int(), b = td_kind_bool(), f = td_kind_function(2);
        TdKind a = td_kind_any(), n = td_kind_nil();
        ISO_CHECK(td_kind_is_concrete_hint(&i));
        ISO_CHECK(td_kind_is_concrete_hint(&b));
        ISO_CHECK(td_kind_is_concrete_hint(&f));
        ISO_CHECK(!td_kind_is_concrete_hint(&a));
        ISO_CHECK(!td_kind_is_concrete_hint(&n));
    }

    /* ── AnnotatedNode::iir_hint ─────────────────────────────────────────── */
    {
        TdKind k = td_kind_int();
        TdAnnotatedNode node;
        ISO_CHECK(td_annotated_node_init(&node, "atom", &k) == 0);
        td_annotated_node_set_position(&node, 1, 1, 1, 2);
        ISO_CHECK_STR_EQ(td_annotated_node_iir_hint(&node), "i64");
        size_t line, col;
        td_annotated_node_position(&node, &line, &col);
        ISO_CHECK_EQ_UINT(line, 1u);
        ISO_CHECK_EQ_UINT(col, 1u);
        td_annotated_node_free(&node);
    }

    /* ── resolve: alias chain, cycle, record-stays-named, passthrough ────── */
    {
        TypeDeclarations *d;
        ISO_CHECK(td_new(&d, "twig") == 0);
        ISO_CHECK_STR_EQ(td_language(d), "twig");

        /* Nat -> Int (alias). */
        TdKind int_kind = td_kind_int();
        TdNamedType alias;
        ISO_CHECK(td_named_alias(&int_kind, &alias) == 0);
        ISO_CHECK(td_insert_named_type(d, "Nat", alias) == 0);

        TdKind nat, resolved;
        ISO_CHECK(td_kind_named("Nat", &nat) == 0);
        ISO_CHECK(td_resolve(d, &nat, &resolved) == 0);
        ISO_CHECK(resolved.tag == TD_INT);
        td_kind_free(&resolved);
        td_kind_free(&nat);

        /* Non-Named kinds pass through unchanged. */
        TdKind boolk = td_kind_bool(), rb;
        ISO_CHECK(td_resolve(d, &boolk, &rb) == 0);
        ISO_CHECK(rb.tag == TD_BOOL);
        td_kind_free(&rb);

        td_free(d);
    }

    /* ── resolve on a cycle returns Any (depth guard) ────────────────────── */
    {
        TypeDeclarations *d;
        ISO_CHECK(td_new(&d, "twig") == 0);
        /* A -> Named("A") (direct cycle). */
        TdKind a_named;
        ISO_CHECK(td_kind_named("A", &a_named) == 0);
        TdNamedType alias;
        ISO_CHECK(td_named_alias(&a_named, &alias) == 0);
        td_kind_free(&a_named);
        ISO_CHECK(td_insert_named_type(d, "A", alias) == 0);

        TdKind query, resolved;
        ISO_CHECK(td_kind_named("A", &query) == 0);
        ISO_CHECK(td_resolve(d, &query, &resolved) == 0);
        ISO_CHECK(resolved.tag == TD_ANY); /* did not loop forever */
        td_kind_free(&resolved);
        td_kind_free(&query);
        td_free(d);
    }

    /* ── a Named record stays Named (not an alias) ───────────────────────── */
    {
        TypeDeclarations *d;
        ISO_CHECK(td_new(&d, "twig") == 0);
        TdNamedType rec;
        ISO_CHECK(td_named_record(NULL, 0, &rec) == 0);
        ISO_CHECK(td_insert_named_type(d, "Point", rec) == 0);

        TdKind point, resolved;
        ISO_CHECK(td_kind_named("Point", &point) == 0);
        ISO_CHECK(td_resolve(d, &point, &resolved) == 0);
        ISO_CHECK(resolved.tag == TD_NAMED);
        ISO_CHECK_STR_EQ(resolved.named, "Point");
        td_kind_free(&resolved);
        td_kind_free(&point);
        td_free(d);
    }

    /* ── union_variants lookup ───────────────────────────────────────────── */
    {
        TypeDeclarations *d;
        ISO_CHECK(td_new(&d, "twig") == 0);
        /* Shape = Union(Circle, Rect) with no fields. */
        TdVariant vs[2];
        ISO_CHECK(td_variant_init(&vs[0], "Circle", NULL, 0) == 0);
        ISO_CHECK(td_variant_init(&vs[1], "Rect", NULL, 0) == 0);
        TdNamedType un;
        ISO_CHECK(td_named_union(vs, 2, &un) == 0);
        td_variant_free(&vs[0]);
        td_variant_free(&vs[1]);
        ISO_CHECK(td_insert_named_type(d, "Shape", un) == 0);

        char **names = NULL;
        size_t count = 0;
        ISO_CHECK(td_union_variants(d, "Shape", &names, &count) == 1);
        ISO_CHECK_EQ_UINT(count, 2u);
        ISO_CHECK_STR_EQ(names[0], "Circle");
        ISO_CHECK_STR_EQ(names[1], "Rect");
        td_string_array_free(names, count);

        /* Unknown -> not a union (return 0). */
        char **none = NULL;
        size_t nc = 0;
        ISO_CHECK(td_union_variants(d, "Unknown", &none, &nc) == 0);
        ISO_CHECK(none == NULL);
        td_free(d);
    }

    /* ── new is empty; typed mode + globals + a record with fields ───────── */
    {
        TypeDeclarations *d;
        ISO_CHECK(td_new(&d, "twig") == 0);
        ISO_CHECK_EQ_UINT(td_named_type_count(d), 0u);
        ISO_CHECK_EQ_UINT(td_global_count(d), 0u);
        ISO_CHECK(!td_has_typed_mode(d));
        td_set_typed_mode(d, TD_MODE_STRICT);
        ISO_CHECK(td_has_typed_mode(d));
        ISO_CHECK(td_typed_mode(d) == TD_MODE_STRICT);

        /* A global binding and a record with two Int fields. */
        ISO_CHECK(td_insert_global(d, "origin", td_kind_int()) == 0);
        ISO_CHECK_EQ_UINT(td_global_count(d), 1u);

        TdField fields[2];
        TdKind ik1 = td_kind_int(), ik2 = td_kind_int();
        ISO_CHECK(td_field_init(&fields[0], "x", &ik1) == 0);
        ISO_CHECK(td_field_init(&fields[1], "y", &ik2) == 0);
        TdNamedType rec;
        ISO_CHECK(td_named_record(fields, 2, &rec) == 0);
        td_field_free(&fields[0]);
        td_field_free(&fields[1]);
        ISO_CHECK(td_insert_named_type(d, "Point", rec) == 0);
        ISO_CHECK_EQ_UINT(td_named_type_count(d), 1u);
        td_free(d);
    }

    /* ── AnnotatedNode children: child_node / node_children ──────────────── */
    {
        TdKind any = td_kind_any();
        TdAnnotatedNode root;
        ISO_CHECK(td_annotated_node_init(&root, "program", &any) == 0);

        /* Add a token leaf and two child nodes. */
        ISO_CHECK(td_annotated_node_add_token(&root, "(", 1, 1) == 0);
        TdKind ik = td_kind_int();
        TdAnnotatedNode atom;
        ISO_CHECK(td_annotated_node_init(&atom, "atom", &ik) == 0);
        ISO_CHECK(td_annotated_node_add_child_node(&root, atom) == 0);
        TdKind bk = td_kind_bool();
        TdAnnotatedNode cond;
        ISO_CHECK(td_annotated_node_init(&cond, "cond", &bk) == 0);
        ISO_CHECK(td_annotated_node_add_child_node(&root, cond) == 0);

        /* child_node finds the first node with a matching rule. */
        const TdAnnotatedNode *found =
            td_annotated_node_child_node(&root, "atom");
        ISO_CHECK(found != NULL);
        if (found) {
            ISO_CHECK_STR_EQ(td_annotated_node_iir_hint(found), "i64");
        }
        ISO_CHECK(td_annotated_node_child_node(&root, "missing") == NULL);

        /* node_children excludes the token leaf. */
        const TdAnnotatedNode **kids = NULL;
        size_t count = 0;
        ISO_CHECK(td_annotated_node_node_children(&root, &kids, &count) == 0);
        ISO_CHECK_EQ_UINT(count, 2u);
        free((void *)kids);

        /* Position falls back to (0, 0) when unset. */
        size_t line, col;
        td_annotated_node_position(&root, &line, &col);
        ISO_CHECK_EQ_UINT(line, 0u);
        ISO_CHECK_EQ_UINT(col, 0u);

        td_annotated_node_free(&root);
    }

    return ISO_TEST_RESULT();
}

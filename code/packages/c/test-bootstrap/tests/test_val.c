/*
 * test_val.c -- tagged values, deep clone / equality / weight, and the total
 * canonical renderer (VER05i sections 4.1 and 7.2).
 */
#include <stdio.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "test_util.h"

static void render_is(const pbt_val *v, const char *want)
{
    pbt_str s;
    pbt_str_init(&s);
    ISO_CHECK(pbt_canon(v, &s) == 0);
    if (s.s == NULL || strcmp(s.s, want) != 0) {
        printf("  got  %s\n  want %s\n", s.s == NULL ? "(null)" : s.s, want);
    }
    ISO_CHECK(s.s != NULL && strcmp(s.s, want) == 0);
    pbt_str_free(&s);
}

static void test_scalars_and_render(void)
{
    pbt_val v;
    pbt_val l;
    uint8_t raw[2] = {0x00, 0xff};
    pbt_val_nil(&v);
    render_is(&v, "nil");
    pbt_val_set_bool(&v, 1);
    render_is(&v, "true");
    pbt_val_set_bool(&v, 0);
    render_is(&v, "false");
    pbt_val_set_i64(&v, -12);
    render_is(&v, "-12");
    pbt_val_set_u64(&v, UINT64_MAX);
    render_is(&v, "18446744073709551615");
    ISO_CHECK(pbt_val_set_bytes(&v, raw, 2) == 0);
    render_is(&v, "0x00ff");
    pbt_val_free(&v);
    ISO_CHECK(pbt_val_set_bytes(&v, NULL, 0) == 0);
    render_is(&v, "0x");
    pbt_val_free(&v);
    pbt_val_set_ptr(&v, &raw);
    render_is(&v, "<ptr>");

    /* Strings are quoted and every control, quote, backslash and high byte is escaped. */
    ISO_CHECK(pbt_val_set_str(&v, "a\"b\\c\n\r\t\x01\x7f\xc3\xa9", 12) == 0);
    render_is(&v, "\"a\\\"b\\\\c\\n\\r\\t\\x01\\x7f\\xc3\\xa9\"");
    pbt_val_free(&v);

    /* Lists and tuples. */
    ISO_CHECK(pbt_val_set_list(&l, PBT_V_LIST, 3) == 0);
    pbt_val_set_i64(&l.u.list.items[0], 1);
    pbt_val_set_bool(&l.u.list.items[1], 1);
    ISO_CHECK(pbt_val_set_bytes(&l.u.list.items[2], raw, 1) == 0);
    render_is(&l, "[1,true,0x00]");
    l.kind = PBT_V_TUPLE;
    render_is(&l, "(1,true,0x00)");
    pbt_val_free(&l);
    ISO_CHECK(pbt_val_set_list(&l, PBT_V_LIST, 0) == 0);
    render_is(&l, "[]");
    ISO_CHECK(l.u.list.items == NULL); /* an empty list never allocates */
    pbt_val_free(&l);
}

static void test_clone_equal_weight(void)
{
    pbt_val a;
    pbt_val c;
    uint8_t raw[3] = {1, 2, 3};
    pbt_val_nil(&a);
    ISO_CHECK(pbt_val_set_list(&a, PBT_V_LIST, 2) == 0);
    ISO_CHECK(pbt_val_set_bytes(&a.u.list.items[0], raw, 3) == 0);
    pbt_val_set_i64(&a.u.list.items[1], -77);
    ISO_CHECK(pbt_val_clone(&c, &a) == 0);
    ISO_CHECK(pbt_val_equal(&a, &c));
    /* The clone is DEEP: changing it leaves the original alone. */
    c.u.list.items[0].u.bytes.p[0] = 9;
    ISO_CHECK(!pbt_val_equal(&a, &c));
    ISO_CHECK(a.u.list.items[0].u.bytes.p[0] == 1);
    pbt_val_free(&c);
    ISO_CHECK(c.kind == PBT_V_NIL);
    pbt_val_free(&c); /* freeing twice is harmless */

    /* Equality is kind sensitive: true is not 1, bytes are not a string, a list is not a tuple. */
    {
        pbt_val b;
        pbt_val i;
        pbt_val_set_bool(&b, 1);
        pbt_val_set_i64(&i, 1);
        ISO_CHECK(!pbt_val_equal(&b, &i));
        ISO_CHECK(pbt_val_equal(&i, &i));
        pbt_val_free(&b);
    }
    {
        pbt_val s1;
        pbt_val s2;
        ISO_CHECK(pbt_val_set_bytes(&s1, raw, 3) == 0);
        ISO_CHECK(pbt_val_set_str(&s2, "\x01\x02\x03", 3) == 0);
        ISO_CHECK(!pbt_val_equal(&s1, &s2));
        pbt_val_free(&s1);
        pbt_val_free(&s2);
    }

    /* Weight: collection length plus the length of each child collection. */
    ISO_CHECK_EQ_UINT(pbt_val_weight(&a), 2 + 3);
    {
        pbt_val one;
        pbt_val_set_i64(&one, 5);
        ISO_CHECK_EQ_UINT(pbt_val_weight(&one), 0);
    }
    pbt_val_free(&a);
}

static void test_limits(void)
{
    pbt_val v;
    pbt_val deep;
    pbt_val cur;
    pbt_val clone;
    int i;
    /* Sizes above the cap are refused, leaving a valid nil. */
    ISO_CHECK(pbt_val_set_list(&v, PBT_V_LIST, PBT_MAX_LIST_LEN + 1) == PBT_E_LIMIT);
    ISO_CHECK(v.kind == PBT_V_NIL);
    ISO_CHECK(pbt_val_set_bytes(&v, NULL, PBT_MAX_INPUT_BYTES + 1) == PBT_E_LIMIT);
    ISO_CHECK(v.kind == PBT_V_NIL);

    /* A value nested deeper than PBT_MAX_VAL_DEPTH is refused by clone; free copes with it. */
    ISO_CHECK(pbt_val_set_list(&deep, PBT_V_LIST, 1) == 0);
    cur = deep;
    {
        pbt_val *slot = &deep;
        for (i = 0; i < PBT_MAX_VAL_DEPTH + 10; i++) {
            ISO_CHECK(pbt_val_set_list(&slot->u.list.items[0], PBT_V_LIST, 1) == 0);
            slot = &slot->u.list.items[0];
        }
    }
    (void)cur;
    ISO_CHECK(pbt_val_clone(&clone, &deep) == PBT_E_LIMIT);
    ISO_CHECK(clone.kind == PBT_V_NIL);
    ISO_CHECK(!pbt_val_equal(&deep, &deep)); /* deeper than the bound compares unequal */
    pbt_val_free(&deep);
}

static void test_canon_caps(void)
{
    pbt_val v;
    pbt_str s;
    size_t i;
    pbt_val deep;
    pbt_val *slot;

    /* More than PBT_CANON_MAX_NODES nodes truncates with "...". */
    ISO_CHECK(pbt_val_set_list(&v, PBT_V_LIST, 30000) == 0);
    for (i = 0; i < 30000; i++) {
        pbt_val_set_i64(&v.u.list.items[i], 7);
    }
    pbt_str_init(&s);
    ISO_CHECK(pbt_canon(&v, &s) == 0);
    ISO_CHECK(s.s != NULL && strstr(s.s, "...") != NULL);
    ISO_CHECK(s.n < 25000); /* far below the 60k a full rendering needs */
    pbt_str_free(&s);
    pbt_val_free(&v);

    /* A 1 MiB bytes value renders within the output cap and ends with the marker. */
    ISO_CHECK(pbt_val_set_bytes(&v, NULL, PBT_MAX_INPUT_BYTES) == 0);
    pbt_str_init(&s);
    ISO_CHECK(pbt_canon(&v, &s) == 0);
    ISO_CHECK(s.n <= PBT_CANON_MAX_BYTES);
    ISO_CHECK(s.n > PBT_CANON_MAX_BYTES - 16);
    ISO_CHECK(s.s != NULL && strcmp(s.s + s.n - 3, "...") == 0);
    pbt_str_free(&s);
    pbt_val_free(&v);

    /* Nesting deeper than PBT_CANON_MAX_DEPTH renders as "...". */
    ISO_CHECK(pbt_val_set_list(&deep, PBT_V_LIST, 1) == 0);
    slot = &deep;
    for (i = 0; i < 30; i++) {
        ISO_CHECK(pbt_val_set_list(&slot->u.list.items[0], PBT_V_LIST, 1) == 0);
        slot = &slot->u.list.items[0];
    }
    pbt_val_free(slot);
    pbt_val_set_i64(slot, 1);
    pbt_str_init(&s);
    ISO_CHECK(pbt_canon(&deep, &s) == 0);
    ISO_CHECK(s.s != NULL && strstr(s.s, "[[[[[[[[[[[[[[[[...") != NULL);
    ISO_CHECK(strchr(s.s, '1') == NULL);
    pbt_str_free(&s);
    pbt_val_free(&deep);

    /* A value of an unknown kind is "<unrenderable>", never a crash. */
    pbt_val_nil(&v);
    v.kind = (pbt_kind)99;
    render_is(&v, "<unrenderable>");
}

static void test_canon_oom(void)
{
    pbt_val v;
    pbt_str s;
    long baseline = pbt_hooks.live_allocs;
    int rc;
    ISO_CHECK(pbt_val_set_bytes(&v, NULL, 200) == 0);
    pbt_hooks.fail_after = 1;
    pbt_str_init(&s);
    rc = pbt_canon(&v, &s);
    ISO_CHECK(rc == PBT_E_NOMEM);
    pbt_str_free(&s);
    pbt_hooks.fail_after = 0;
    pbt_val_free(&v);
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
}

int main(void)
{
    pbt_hooks_reset();
    test_scalars_and_render();
    test_clone_equal_weight();
    test_limits();
    test_canon_caps();
    test_canon_oom();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}

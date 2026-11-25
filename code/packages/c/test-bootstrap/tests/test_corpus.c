/*
 * test_corpus.c -- replay the committed pbt-v1 corpus and prove the run CAN FAIL.
 *
 * A conformance run that cannot fail proves nothing (the LH-003 false pass this
 * repo has been burned by).  So after the positive run (25/25, exact outputs,
 * minimals AND step counts) this file:
 *
 *   - corrupts EVERY real case, per op, in each way the op defines, and asserts
 *     the run fails, that a problem NAMES that case, and that no OTHER case is
 *     named (a single-case negative control proves only one path);
 *   - drops and truncates the case list, adds unknown ops, extra and missing
 *     fields, duplicate ids, a wrong schema version;
 *   - sabotages the engine through the pbt_hooks seams and requires the run to
 *     notice.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "test_util.h"

static int evaluate(pbt_json *doc, pbt_corpus_report *rep)
{
    pbt_corpus_report_init(rep);
    return pbt_corpus_evaluate(doc, rep);
}

static void test_positive(pbt_json *doc)
{
    pbt_corpus_report rep;
    size_t i;
    ISO_CHECK(evaluate(doc, &rep) == 0);
    for (i = 0; i < rep.nstored; i++) {
        printf("  problem: %s\n", rep.problems[i]);
    }
    ISO_CHECK_EQ_UINT(rep.nproblems, 0);
    ISO_CHECK_EQ_UINT(rep.evaluated, 25);
    ISO_CHECK_EQ_UINT(pbt_json_get(doc, "cases")->n, 25);
    pbt_corpus_report_free(&rep);
}

/* ---- per-op corruptions ------------------------------------------------------------ */

static const char *op_of(pbt_json *cs)
{
    return pbt_json_get(cs, "op")->s;
}

/* Replace an int object's decimal with something guaranteed different and canonical. */
static void bump_int(pbt_json *intobj)
{
    pbt_json *v = pbt_json_get(intobj, "value");
    tu_set_str(v, strcmp(v->s, "424242") == 0 ? "424243" : "424242");
}

static void steps_plus_one(pbt_json *cs)
{
    pbt_json *v = pbt_json_get(pbt_json_get(cs, "expected_steps"), "value");
    pbt_big b = tu_big(v->s);
    pbt_big one;
    char buf[PBT_BIG_DEC_BUF];
    pbt_big_set_u64(&one, 1);
    (void)pbt_big_add(&b, &b, &one);
    (void)pbt_big_to_dec(&b, buf, sizeof buf);
    tu_set_str(v, buf);
}

/* Number of corruption variants for a case; apply variant v (0-based). */
static int variants(pbt_json *cs)
{
    const char *op = op_of(cs);
    if (strncmp(op, "gen_", 4) == 0) {
        return 2;
    }
    if (strncmp(op, "shrink_", 7) == 0 || strcmp(op, "replay_consistency") == 0 || strcmp(op, "filter_exhausted") == 0) {
        return 2;
    }
    return 1;
}

static void corrupt(pbt_json *cs, int v)
{
    const char *op = op_of(cs);
    if (strncmp(op, "gen_", 4) == 0) {
        pbt_json *outs = pbt_json_get(cs, "outputs");
        if (v == 0) {
            pbt_json *first = &outs->items[0];
            if (first->type == PBT_J_BOOL) {
                first->b = !first->b; /* flipped output */
            } else if (pbt_json_get(first, "hex") != NULL) {
                pbt_json *hex = pbt_json_get(first, "hex");
                tu_set_str(hex, hex->slen == 0 ? "00" : (hex->s[0] == '0' ? "11" : "00"));
            } else {
                bump_int(first);
            }
        } else {
            /* count no longer equals the number of outputs: refused before generating */
            pbt_json *cv = pbt_json_get(pbt_json_get(cs, "count"), "value");
            tu_set_str(cv, strcmp(cv->s, "9") == 0 ? "10" : "9");
            (void)outs;
        }
    } else if (strcmp(op, "shrink_int") == 0 || strcmp(op, "shrink_refinement") == 0) {
        if (v == 0) {
            steps_plus_one(cs);
        } else {
            bump_int(pbt_json_get(cs, "expected_minimal"));
        }
    } else if (strcmp(op, "shrink_list") == 0) {
        if (v == 0) {
            steps_plus_one(cs);
        } else {
            bump_int(&pbt_json_get(cs, "expected_minimal")->items[0]);
        }
    } else if (strcmp(op, "shrink_bytes") == 0) {
        if (v == 0) {
            steps_plus_one(cs);
        } else {
            tu_set_str(pbt_json_get(pbt_json_get(cs, "expected_minimal"), "hex"), "ff");
        }
    } else if (strcmp(op, "replay_consistency") == 0) {
        if (v == 0) {
            bump_int(pbt_json_get(cs, "initial_fail"));
        } else {
            bump_int(pbt_json_get(cs, "expected_minimal"));
        }
    } else if (strcmp(op, "filter_exhausted") == 0) {
        if (v == 0) {
            tu_set_str(pbt_json_get(cs, "failure_id"), "property-panic"); /* wrong failure id */
        } else {
            tu_set_str(pbt_json_get(cs, "verdict"), "pass");
        }
    } else if (strcmp(op, "failure_id_valid") == 0) {
        pbt_json *valid = pbt_json_get(cs, "valid");
        valid->b = !valid->b; /* flipped validity */
    } else if (strcmp(op, "corpus_reject") == 0) {
        tu_set_str(pbt_json_get(cs, "text"), "tfv 1\nrec R1 Kop S706274\n"); /* a VALID tfv in place of a reject text */
    }
}

static void test_every_case_corrupted(pbt_json *doc)
{
    size_t ncases = pbt_json_get(doc, "cases")->n;
    size_t i;
    int v;
    for (i = 0; i < ncases; i++) {
        int nv = variants(tu_case(doc, i));
        for (v = 0; v < nv; v++) {
            pbt_json *copy = pbt_json_clone(doc);
            pbt_corpus_report rep;
            const char *id;
            ISO_CHECK(copy != NULL);
            if (copy == NULL) {
                return;
            }
            id = tu_case_id(copy, i);
            corrupt(tu_case(copy, i), v);
            ISO_CHECK(evaluate(copy, &rep) == 0);
            if (rep.nproblems == 0 || !tu_report_names(&rep, id) || tu_report_names_other(&rep, copy, id)) {
                printf("  case %s variant %d: nproblems=%lu names=%d other=%d\n", id, v,
                       (unsigned long)rep.nproblems, tu_report_names(&rep, id), tu_report_names_other(&rep, copy, id));
            }
            ISO_CHECK(rep.nproblems > 0);
            ISO_CHECK(tu_report_names(&rep, id));
            ISO_CHECK(!tu_report_names_other(&rep, copy, id));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
        }
    }
}

/* ---- structural corruptions ----------------------------------------------------------- */

static pbt_json *cases_of(pbt_json *doc)
{
    return pbt_json_get(doc, "cases");
}

static void test_structural(pbt_json *doc)
{
    pbt_json *copy;
    pbt_corpus_report rep;
    size_t n = cases_of(doc)->n;

    /* A dropped case: the case-count gate fires. */
    copy = pbt_json_clone(doc);
    cases_of(copy)->n = n - 1;
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    ISO_CHECK_EQ_UINT(rep.evaluated, 24);
    pbt_corpus_report_free(&rep);
    cases_of(copy)->n = n; /* restore so free sees every item */
    pbt_json_free(copy);

    /* A truncated corpus (half the cases). */
    copy = pbt_json_clone(doc);
    cases_of(copy)->n = n / 2;
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
    cases_of(copy)->n = n;
    pbt_json_free(copy);

    /* An empty corpus. */
    copy = pbt_json_clone(doc);
    cases_of(copy)->n = 0;
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    ISO_CHECK_EQ_UINT(rep.evaluated, 0);
    pbt_corpus_report_free(&rep);
    cases_of(copy)->n = n;
    pbt_json_free(copy);

    /* An unknown op is a problem naming the case, never a skip. */
    copy = pbt_json_clone(doc);
    tu_set_str(pbt_json_get(tu_case(copy, 3), "op"), "no_such_op");
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, 3)));
    pbt_corpus_report_free(&rep);
    pbt_json_free(copy);

    /* A non-string id and a non-string op are problems, not crashes. */
    copy = pbt_json_clone(doc);
    pbt_json_get(tu_case(copy, 0), "id")->type = PBT_J_BOOL;
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
    pbt_json_get(tu_case(copy, 0), "id")->type = PBT_J_STR;
    pbt_json_get(tu_case(copy, 1), "op")->type = PBT_J_NUM;
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
    pbt_json_get(tu_case(copy, 1), "op")->type = PBT_J_STR;
    pbt_json_free(copy);

    /* A duplicate id. */
    copy = pbt_json_clone(doc);
    tu_set_str(pbt_json_get(tu_case(copy, 1), "id"), tu_case_id(copy, 0));
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
    pbt_json_free(copy);

    /* An extra field and a missing field (the member count changes: shrink n to drop one). */
    copy = pbt_json_clone(doc);
    {
        pbt_json *cs = tu_case(copy, 5);
        char *saved = cs->keys[cs->n - 1];
        size_t saved_len = cs->keylens[cs->n - 1];
        cs->keys[cs->n - 1] = (char *)pbt_alloc(6, 1);
        memcpy(cs->keys[cs->n - 1], "bogus", 6);
        cs->keylens[cs->n - 1] = 5; /* the real last field is now called "bogus": one missing, one extra */
        ISO_CHECK(evaluate(copy, &rep) == 0);
        ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, 5)));
        pbt_corpus_report_free(&rep);
        pbt_free(cs->keys[cs->n - 1]);
        cs->keys[cs->n - 1] = saved;
        cs->keylens[cs->n - 1] = saved_len;
    }
    pbt_json_free(copy);

    /* A wrong schema version and a document with the wrong shape. */
    copy = pbt_json_clone(doc);
    tu_set_str(pbt_json_get(copy, "schema_version"), "2");
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
    pbt_json_free(copy);

    ISO_CHECK(evaluate(cases_of(doc), &rep) == 0); /* the bare array is not a document */
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
}

/* ---- hostile sizes: refused, not run ------------------------------------------------------ */

static void test_hostile_sizes(pbt_json *doc)
{
    pbt_json *copy;
    pbt_corpus_report rep;
    size_t i;
    size_t ncases = cases_of(doc)->n;

    /* A count above the cap in a gen case. */
    for (i = 0; i < ncases; i++) {
        if (strcmp(pbt_json_get(tu_case(doc, i), "op")->s, "gen_bytes") == 0) {
            copy = pbt_json_clone(doc);
            tu_set_int_field(tu_case(copy, i), "count", "99999999");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);

            /* count x max_len above the total budget even though each is under the cap. */
            copy = pbt_json_clone(doc);
            tu_set_int_field(tu_case(copy, i), "count", "10000");
            tu_set_int_field(tu_case(copy, i), "max_len", "10000");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            break;
        }
    }

    /* A non-canonical int (leading zero), a too-wide int, a negative zero. */
    for (i = 0; i < ncases; i++) {
        if (strcmp(pbt_json_get(tu_case(doc, i), "op")->s, "shrink_int") == 0) {
            static const char *const bad[] = {"007", "-0", "", "-", "12a", "1e3"};
            size_t k;
            char huge[PBT_BIG_MAX_DIGITS + 8];
            for (k = 0; k < sizeof bad / sizeof bad[0]; k++) {
                copy = pbt_json_clone(doc);
                tu_set_int_field(tu_case(copy, i), "initial", bad[k]);
                ISO_CHECK(evaluate(copy, &rep) == 0);
                ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
                pbt_corpus_report_free(&rep);
                pbt_json_free(copy);
            }
            memset(huge, '9', sizeof huge - 1);
            huge[sizeof huge - 1] = '\0';
            copy = pbt_json_clone(doc);
            tu_set_int_field(tu_case(copy, i), "initial", huge);
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            break;
        }
    }

    /* Uppercase hex bytes are refused. */
    for (i = 0; i < ncases; i++) {
        if (strcmp(pbt_json_get(tu_case(doc, i), "op")->s, "shrink_bytes") == 0) {
            copy = pbt_json_clone(doc);
            tu_set_str(pbt_json_get(pbt_json_get(tu_case(copy, i), "initial"), "hex"), "0A");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            break;
        }
    }

    /* A string with an embedded NUL is not the literal it starts with ("bool\0x" is not "bool"). */
    for (i = 0; i < ncases; i++) {
        if (strcmp(pbt_json_get(tu_case(doc, i), "op")->s, "gen_primitive") == 0) {
            pbt_json *type;
            copy = pbt_json_clone(doc);
            type = pbt_json_get(tu_case(copy, i), "type");
            tu_set_str(type, "bool");
            type->s[4] = '\0';
            pbt_free(type->s);
            type->s = (char *)pbt_alloc(7, 1);
            memcpy(type->s, "bool\0x", 7);
            type->slen = 6;
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            break;
        }
    }

    /* An unknown property name is a problem. */
    for (i = 0; i < ncases; i++) {
        if (strcmp(pbt_json_get(tu_case(doc, i), "op")->s, "shrink_list") == 0) {
            copy = pbt_json_clone(doc);
            tu_set_str(pbt_json_get(tu_case(copy, i), "property"), "no_such_property");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            break;
        }
    }
}

/* Replace a member with a null / an empty object: every op must refuse a field of the wrong type, by case id. */
static void replace_member(pbt_json *obj, size_t member, pbt_jtype type)
{
    pbt_json *holder = (pbt_json *)pbt_alloc(1, sizeof *holder);
    *holder = obj->items[member];
    pbt_json_free(holder); /* frees the old subtree and the holder */
    memset(&obj->items[member], 0, sizeof obj->items[member]);
    obj->items[member].type = type;
}

static void test_every_field_of_every_case_must_be_the_right_type(pbt_json *doc)
{
    size_t ncases = cases_of(doc)->n;
    size_t i;
    for (i = 0; i < ncases; i++) {
        size_t nfields = tu_case(doc, i)->n;
        size_t f;
        for (f = 0; f < nfields; f++) {
            int variant;
            const char *key = tu_case(doc, i)->keys[f];
            if (strcmp(key, "id") == 0) {
                continue;
            }
            for (variant = 0; variant < 2; variant++) {
                pbt_json *copy = pbt_json_clone(doc);
                pbt_corpus_report rep;
                const char *id = tu_case_id(copy, i);
                replace_member(tu_case(copy, i), f, variant == 0 ? PBT_J_NULL : PBT_J_OBJ);
                ISO_CHECK(evaluate(copy, &rep) == 0);
                if (!tu_report_names(&rep, id)) {
                    printf("  case %s field %s variant %d: no problem named the case\n", id, key, variant);
                }
                ISO_CHECK(tu_report_names(&rep, id));
                ISO_CHECK(!tu_report_names_other(&rep, copy, id));
                pbt_corpus_report_free(&rep);
                pbt_json_free(copy);
            }
        }
    }
}

/*
 * The corpus names three properties it never uses (int_is_even, int_abs_le_10, bytes_no_zero_byte).
 * Expected values come from the independent Python reference, not from this engine.
 */
static void test_properties_the_committed_corpus_does_not_use(void)
{
    static const char *const text =
        "{\"schema_version\":1,\"cases\":["
        "{\"id\":\"syn/even\",\"op\":\"shrink_int\",\"initial\":{\"kind\":\"int\",\"value\":\"15\"},"
        "\"target\":{\"kind\":\"int\",\"value\":\"0\"},\"min\":{\"kind\":\"int\",\"value\":\"-100\"},"
        "\"max\":{\"kind\":\"int\",\"value\":\"100\"},\"property\":\"int_is_even\","
        "\"expected_minimal\":{\"kind\":\"int\",\"value\":\"15\"},\"expected_steps\":{\"kind\":\"int\",\"value\":\"0\"}},"
        "{\"id\":\"syn/abs\",\"op\":\"shrink_int\",\"initial\":{\"kind\":\"int\",\"value\":\"-77\"},"
        "\"target\":{\"kind\":\"int\",\"value\":\"0\"},\"min\":{\"kind\":\"int\",\"value\":\"-100\"},"
        "\"max\":{\"kind\":\"int\",\"value\":\"100\"},\"property\":\"int_abs_le_10\","
        "\"expected_minimal\":{\"kind\":\"int\",\"value\":\"-11\"},\"expected_steps\":{\"kind\":\"int\",\"value\":\"10\"}},"
        "{\"id\":\"syn/zero\",\"op\":\"shrink_bytes\",\"initial\":{\"kind\":\"bytes\",\"hex\":\"0500070009\"},"
        "\"property\":\"bytes_no_zero_byte\",\"expected_minimal\":{\"kind\":\"bytes\",\"hex\":\"00\"},"
        "\"expected_steps\":{\"kind\":\"int\",\"value\":\"2\"}},"
        "{\"id\":\"syn/dup\",\"op\":\"shrink_list\",\"initial\":[{\"kind\":\"int\",\"value\":\"1\"},{\"kind\":\"int\",\"value\":\"2\"},"
        "{\"kind\":\"int\",\"value\":\"3\"},{\"kind\":\"int\",\"value\":\"3\"}],\"elem_target\":{\"kind\":\"int\",\"value\":\"0\"},"
        "\"property\":\"list_no_consecutive_duplicates\",\"expected_minimal\":[{\"kind\":\"int\",\"value\":\"3\"},{\"kind\":\"int\",\"value\":\"3\"}],"
        "\"expected_steps\":{\"kind\":\"int\",\"value\":\"1\"}}"
        "]}";
    pbt_json *doc = NULL;
    pbt_corpus_report rep;
    char err[100];
    ISO_CHECK(pbt_json_parse(text, strlen(text), &doc, err, sizeof err) == 0);
    if (doc == NULL) {
        return;
    }
    ISO_CHECK(evaluate(doc, &rep) == 0);
    ISO_CHECK_EQ_UINT(rep.evaluated, 4);
    ISO_CHECK(!tu_report_names(&rep, "syn/even"));
    ISO_CHECK(!tu_report_names(&rep, "syn/abs"));
    ISO_CHECK(!tu_report_names(&rep, "syn/zero"));
    ISO_CHECK(!tu_report_names(&rep, "syn/dup"));
    /* Only the document-level gates remain (case count, ops never exercised, failure ids). */
    ISO_CHECK(rep.nproblems > 0);
    pbt_corpus_report_free(&rep);
    /* And a wrong expectation IS caught: corrupt the even case's steps. */
    tu_set_int_field(&pbt_json_get(doc, "cases")->items[0], "expected_steps", "3");
    ISO_CHECK(evaluate(doc, &rep) == 0);
    ISO_CHECK(tu_report_names(&rep, "syn/even"));
    pbt_corpus_report_free(&rep);
    pbt_json_free(doc);
}

/* Decode failures: wrong kinds, a negative size, too-wide ints, over-long hex, a list past the cap, values nested too deep. */
static void test_decoder_rejections(pbt_json *doc)
{
    size_t ncases = cases_of(doc)->n;
    size_t i;
    for (i = 0; i < ncases; i++) {
        const char *op = pbt_json_get(tu_case(doc, i), "op")->s;
        pbt_json *copy;
        pbt_corpus_report rep;
        if (strcmp(op, "gen_bytes") == 0) {
            /* min_len above max_len, a negative size, and a size past the cap */
            static const char *const bad[] = {"-1", "10001", "99999999999999999999"};
            size_t k;
            for (k = 0; k < 3; k++) {
                copy = pbt_json_clone(doc);
                tu_set_int_field(tu_case(copy, i), "min_len", bad[k]);
                ISO_CHECK(evaluate(copy, &rep) == 0);
                ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
                pbt_corpus_report_free(&rep);
                pbt_json_free(copy);
            }
            copy = pbt_json_clone(doc);
            tu_set_int_field(tu_case(copy, i), "min_len", "9");
            tu_set_int_field(tu_case(copy, i), "max_len", "4");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            /* a wrong kind on an int */
            copy = pbt_json_clone(doc);
            tu_set_str(pbt_json_get(pbt_json_get(tu_case(copy, i), "seed"), "kind"), "bytes");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            /* an output that is not a value at all (a bare number) */
            copy = pbt_json_clone(doc);
            {
                pbt_json *out0 = &pbt_json_get(tu_case(copy, i), "outputs")->items[0];
                pbt_json *holder = (pbt_json *)pbt_alloc(1, sizeof *holder);
                *holder = *out0;
                pbt_json_free(holder);
                memset(out0, 0, sizeof *out0);
                out0->type = PBT_J_NUM;
                out0->s = (char *)pbt_alloc(2, 1);
                out0->s[0] = '5';
                out0->s[1] = '\0';
                out0->slen = 1;
            }
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            /* odd-length hex in an output */
            copy = pbt_json_clone(doc);
            tu_set_str(pbt_json_get(&pbt_json_get(tu_case(copy, i), "outputs")->items[0], "hex"), "abc");
            ISO_CHECK(evaluate(copy, &rep) == 0);
            ISO_CHECK(tu_report_names(&rep, tu_case_id(copy, i)));
            pbt_corpus_report_free(&rep);
            pbt_json_free(copy);
            return;
        }
    }
}

/* More cases than the cap are refused up front: the duplicate-id check is pairwise, so 100k cases must not cost O(n^2). */
static void test_case_count_is_capped(void)
{
    size_t n = PBT_CORPUS_MAX_SIZE + 1;
    size_t cap = 64 + n * 12;
    char *text = (char *)pbt_alloc(cap, 1);
    size_t pos = 0;
    size_t i;
    pbt_json *doc = NULL;
    pbt_corpus_report rep;
    char err[100];
    ISO_CHECK(text != NULL);
    if (text == NULL) {
        return;
    }
    pos += (size_t)snprintf(text + pos, cap - pos, "{\"schema_version\":1,\"cases\":[");
    for (i = 0; i < n; i++) {
        pos += (size_t)snprintf(text + pos, cap - pos, "%s{}", i == 0 ? "" : ",");
    }
    pos += (size_t)snprintf(text + pos, cap - pos, "]}");
    ISO_CHECK(pbt_json_parse(text, pos, &doc, err, sizeof err) == 0);
    if (doc != NULL) {
        ISO_CHECK(evaluate(doc, &rep) == 0);
        ISO_CHECK_EQ_UINT(rep.evaluated, 0);
        ISO_CHECK_EQ_UINT(rep.nproblems, 1); /* refused outright, not 10,001 per-case problems */
        ISO_CHECK(rep.nstored == 1 && strstr(rep.problems[0], "cap") != NULL);
        pbt_corpus_report_free(&rep);
        pbt_json_free(doc);
    }
    pbt_free(text);
}

/* ---- bounded, clipped reports --------------------------------------------------------------- */

static void test_clipped_reports(pbt_json *doc)
{
    pbt_json *copy = pbt_json_clone(doc);
    pbt_corpus_report rep;
    size_t i;
    char *longid = (char *)pbt_alloc(5000, 1);
    memset(longid, 'x', 4999);
    longid[4999] = '\0';
    tu_set_str(pbt_json_get(tu_case(copy, 0), "op"), longid);
    tu_set_str(pbt_json_get(tu_case(copy, 1), "id"), longid);
    ISO_CHECK(evaluate(copy, &rep) == 0);
    ISO_CHECK(rep.nproblems > 0);
    for (i = 0; i < rep.nstored; i++) {
        ISO_CHECK(strlen(rep.problems[i]) <= 400);
    }
    pbt_corpus_report_free(&rep);
    pbt_json_free(copy);
    pbt_free(longid);
}

/* ---- engine sabotage ------------------------------------------------------------------------ */

/* A unit step that moves TWO toward the target. */
static void unit_two(pbt_big *out, const pbt_big *cur, const pbt_big *diff)
{
    pbt_big two;
    pbt_big_set_i64(&two, 2 * pbt_big_sign(diff));
    (void)pbt_big_sub(out, cur, &two);
}

/* A bisection that takes ceil instead of floor of the half. */
static void bisect_ceil(pbt_big *out, const pbt_big *cur, const pbt_big *diff)
{
    pbt_big half;
    pbt_big one;
    pbt_big_set_u64(&one, 1);
    (void)pbt_big_add(&half, diff, &one);
    pbt_big_floor_half(&half, &half);
    (void)pbt_big_sub(out, cur, &half);
}

static uint64_t seed_plus_one(uint64_t s)
{
    return s + 1;
}

static void expect_sabotage_caught(pbt_json *doc, const char *what)
{
    pbt_corpus_report rep;
    ISO_CHECK(evaluate(doc, &rep) == 0);
    if (rep.nproblems == 0) {
        printf("  sabotage NOT caught: %s\n", what);
    }
    ISO_CHECK_MSG(rep.nproblems > 0, what);
    pbt_corpus_report_free(&rep);
    pbt_hooks_reset();
}

static void test_sabotage(pbt_json *doc)
{
    pbt_hooks.unit = unit_two;
    expect_sabotage_caught(doc, "a changed unit step");
    pbt_hooks.bisect = bisect_ceil;
    expect_sabotage_caught(doc, "a changed bisection");
    pbt_hooks.skip_deletion = 1;
    expect_sabotage_caught(doc, "a skipped deletion phase");
    pbt_hooks.seed_map = seed_plus_one;
    expect_sabotage_caught(doc, "a shifted RNG");
}

int main(void)
{
    pbt_json *doc = tu_load_corpus();
    ISO_CHECK_MSG(doc != NULL, "the committed pbt-v1 corpus must load (code/specs/fixtures/pbt-v1/cases.json)");
    if (doc == NULL) {
        return ISO_TEST_RESULT();
    }
    pbt_hooks_reset();
    test_positive(doc);
    test_every_case_corrupted(doc);
    test_structural(doc);
    test_hostile_sizes(doc);
    test_properties_the_committed_corpus_does_not_use();
    test_decoder_rejections(doc);
    test_every_field_of_every_case_must_be_the_right_type(doc);
    test_clipped_reports(doc);
    test_case_count_is_capped();
    test_sabotage(doc);
    pbt_json_free(doc);
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}

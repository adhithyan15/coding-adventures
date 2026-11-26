/*
 * test_fuzz_corpus.c -- the fuzz-v1 conformance corpus through the C engine (VER06j section 11).
 *
 *   - all 25 cases, exact, nothing skipped;
 *   - NEGATIVE CONTROLS: corrupt each case's expected value in turn and require the runner to fail AND to name
 *     exactly that case (a runner that cannot fail proves nothing);
 *   - dropped and truncated corpora;
 *   - sabotage of every engine entry point through fz_hooks;
 *   - the aggregate mutate_sequence work cap, zeroed on a breach;
 *   - a hostile document: unknown op, extra field, duplicate id, wide integer.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "iso_test.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "test_util.h"

#define CORPUS "code/specs/fixtures/fuzz-v1/cases.json"

static pbt_json *load(void)
{
    size_t len = 0;
    char *text = tu_read_repo_file(CORPUS, &len);
    pbt_json *doc = NULL;
    char err[200];
    if (text == NULL) {
        return NULL;
    }
    if (pbt_json_parse(text, len, &doc, err, sizeof err) != 0) {
        doc = NULL;
    }
    free(text);
    return doc;
}

static void eval(pbt_json *doc, pbt_corpus_report *r)
{
    pbt_corpus_report_init(r);
    ISO_CHECK_EQ_INT(fz_corpus_evaluate(doc, r), 0);
}

static pbt_json *field(pbt_json *cs, const char *name)
{
    return pbt_json_get(cs, name);
}

/* Corrupt the expected value of case `i` according to its op.  Returns 0 if the op is unknown. */
static int corrupt(pbt_json *cs)
{
    const char *op = pbt_json_get(cs, "op")->s;
    if (strncmp(op, "mutate_", 7) == 0) {
        pbt_json *hex = pbt_json_get(field(cs, "output"), "hex");
        if (hex->slen == 0) {
            tu_set_str(hex, "00");
        } else {
            char t[8];
            t[0] = hex->s[0] == '0' ? '1' : '0';
            t[1] = '\0';
            hex->s[0] = t[0];
        }
        return 1;
    }
    if (strcmp(op, "corpus_admit") == 0) {
        pbt_json *b = field(cs, "admitted");
        b->b = !b->b;
        return 1;
    }
    if (strcmp(op, "crash_dedup") == 0) {
        pbt_json *b = field(cs, "is_duplicate");
        b->b = !b->b;
        return 1;
    }
    if (strcmp(op, "failure_id_valid") == 0) {
        pbt_json *b = field(cs, "valid");
        b->b = !b->b;
        return 1;
    }
    if (strcmp(op, "private_reporting") == 0) {
        tu_set_str(field(cs, "tap_line"), "not ok 1 - forged [fuzz-crash:00000000]");
        return 1;
    }
    if (strcmp(op, "corpus_reject") == 0) {
        tu_set_str(field(cs, "text"), "tfv 1\n");
        return 1;
    }
    return 0;
}

static void test_conforms(void)
{
    pbt_json *doc = load();
    pbt_corpus_report r;
    ISO_CHECK_MSG(doc != NULL, "fuzz-v1 cases.json must load");
    if (doc == NULL) {
        return;
    }
    eval(doc, &r);
    ISO_CHECK_EQ_INT(r.nproblems, 0);
    ISO_CHECK_EQ_INT(r.evaluated, 25);
    ISO_CHECK_EQ_INT(pbt_json_get(doc, "cases")->n, FZ_V1_CASE_COUNT);
    pbt_corpus_report_free(&r);
    pbt_json_free(doc);
}

static void test_per_case_corruption(void)
{
    size_t n;
    size_t i;
    pbt_json *probe = load();
    if (probe == NULL) {
        ISO_CHECK_MSG(0, "corpus must load");
        return;
    }
    n = pbt_json_get(probe, "cases")->n;
    pbt_json_free(probe);
    for (i = 0; i < n; i++) {
        pbt_json *doc = load();
        pbt_corpus_report r;
        char id[200];
        ISO_CHECK(doc != NULL);
        if (doc == NULL) {
            return;
        }
        (void)snprintf(id, sizeof id, "%s", tu_case_id(doc, i));
        ISO_CHECK_MSG(corrupt(tu_case(doc, i)), "every op has a corruption");
        eval(doc, &r);
        if (!tu_report_names(&r, id)) {
            printf("  corrupting %s was not reported against it\n", id);
        }
        ISO_CHECK(r.nproblems > 0);
        ISO_CHECK(tu_report_names(&r, id));
        if (tu_report_names_other(&r, doc, id)) {
            printf("  corrupting %s also named another case\n", id);
        }
        ISO_CHECK(!tu_report_names_other(&r, doc, id));
        pbt_corpus_report_free(&r);
        pbt_json_free(doc);
    }
}

static void test_dropped_and_truncated(void)
{
    size_t n;
    size_t i;
    size_t keep;
    pbt_json *probe = load();
    if (probe == NULL) {
        ISO_CHECK_MSG(0, "corpus must load");
        return;
    }
    n = pbt_json_get(probe, "cases")->n;
    pbt_json_free(probe);
    /* dropping any one case is a problem (the count gate, and an op gate when it was the only case of its op) */
    for (i = 0; i < n; i++) {
        pbt_json *doc = load();
        pbt_json *cases = pbt_json_get(doc, "cases");
        pbt_corpus_report r;
        size_t k;
        pbt_json dropped = cases->items[i];
        for (k = i; k + 1 < cases->n; k++) {
            cases->items[k] = cases->items[k + 1];
        }
        cases->items[cases->n - 1] = dropped; /* keep it owned so pbt_json_free releases it */
        cases->n--;
        eval(doc, &r);
        ISO_CHECK(r.nproblems > 0);
        pbt_corpus_report_free(&r);
        cases->n++;
        pbt_json_free(doc);
    }
    /* truncation to any prefix is a problem */
    for (keep = 0; keep < n; keep += 3) {
        pbt_json *doc = load();
        pbt_json *cases = pbt_json_get(doc, "cases");
        size_t full = cases->n;
        pbt_corpus_report r;
        cases->n = keep;
        eval(doc, &r);
        ISO_CHECK(r.nproblems > 0);
        pbt_corpus_report_free(&r);
        cases->n = full;
        pbt_json_free(doc);
    }
    /* an empty corpus cannot conform */
    {
        pbt_json *doc = load();
        pbt_json *cases = pbt_json_get(doc, "cases");
        size_t full = cases->n;
        pbt_corpus_report r;
        cases->n = 0;
        eval(doc, &r);
        ISO_CHECK(r.nproblems >= 14); /* one "never exercised" per op, plus the gates */
        pbt_corpus_report_free(&r);
        cases->n = full;
        pbt_json_free(doc);
    }
}

/* ---- sabotage through fz_hooks ------------------------------------------------------------------------------- */

static int bad_bit_flip(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *a, const pbt_big *b)
{
    int rc = fz_bit_flip_big(o, d, n, a, b);
    if (rc == FZ_OK && o->n > 0) {
        o->p[0] ^= 1;
    }
    return rc;
}
static int bad_byte_flip(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *a, const pbt_big *b)
{
    int rc = fz_byte_flip_big(o, d, n, a, b);
    if (rc == FZ_OK && o->n > 0) {
        o->p[0] ^= 1;
    }
    return rc;
}
static int bad_arith(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *a, const pbt_big *b, int w, fz_endian e)
{
    int rc = fz_byte_arithmetic_big(o, d, n, a, b, w, e == FZ_BIG ? FZ_LITTLE : FZ_BIG);
    return rc;
}
static int bad_boundary(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *a, int w, const pbt_big *v,
                        fz_endian e)
{
    return fz_boundary_insert_big(o, d, n, a, w, v, e == FZ_BIG ? FZ_LITTLE : FZ_BIG);
}
static int bad_delete(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *a, const pbt_big *b)
{
    return fz_block_delete_big(o, d, n, b, a); /* offset and length swapped */
}
static int bad_insert(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *a, const uint8_t *i, size_t in)
{
    pbt_big z;
    pbt_big_zero(&z);
    (void)a;
    return fz_block_insert_big(o, d, n, &z, i, in);
}
static int bad_dup(fz_bytes *o, const uint8_t *d, size_t n, const pbt_big *s, const pbt_big *l, const pbt_big *t)
{
    return fz_block_duplicate_big(o, d, n, t, l, s);
}
static int bad_splice(fz_bytes *o, const uint8_t *t, size_t tn, const uint8_t *dn, size_t dl, const pbt_big *a,
                      const pbt_big *b)
{
    return fz_splice_big(o, dn, dl, t, tn, a, b); /* target and donor swapped */
}
static int bad_sequence(fz_bytes *o, const uint8_t *d, size_t n, uint64_t seed, size_t steps, size_t max_len,
                        uint64_t cap, uint64_t *done)
{
    return fz_mutate_sequence_budget(o, d, n, seed + 1, steps, max_len, cap, done);
}
static int bad_admit(const uint64_t *e, size_t ne, const uint64_t *t, size_t nt, int *a, size_t *n)
{
    (void)e;
    (void)ne;
    (void)t;
    (void)nt;
    *a = 1;
    *n = 1;
    return FZ_OK;
}
static int bad_dup_crash(fz_str ka, const fz_str *fa, size_t na, fz_str kb, const fz_str *fb, size_t nb)
{
    (void)ka;
    (void)fa;
    (void)na;
    (void)kb;
    (void)fb;
    (void)nb;
    return 1;
}
static int bad_fid(const char *s, size_t n)
{
    (void)s;
    (void)n;
    return 1;
}
static int bad_tap(char *buf, size_t cap, int num, const char *t, size_t tl, const char *f, size_t fl, const char *s,
                   size_t sl)
{
    int rc = fz_format_tap_failure(buf, cap, num + 1, t, tl, f, fl, s, sl);
    return rc;
}
static int bad_tfv(const char *t, size_t n, char *e, size_t c)
{
    (void)t;
    (void)n;
    (void)e;
    (void)c;
    return 0;
}

static void sabotaged(const char *name, void (*arm)(void))
{
    pbt_json *doc = load();
    pbt_corpus_report r;
    ISO_CHECK(doc != NULL);
    if (doc == NULL) {
        return;
    }
    arm();
    eval(doc, &r);
    fz_hooks_reset();
    if (r.nproblems == 0) {
        printf("  sabotage %s went unnoticed\n", name);
    }
    ISO_CHECK(r.nproblems > 0);
    pbt_corpus_report_free(&r);
    pbt_json_free(doc);
}

static void arm_bit_flip(void) { fz_hooks.bit_flip = bad_bit_flip; }
static void arm_byte_flip(void) { fz_hooks.byte_flip = bad_byte_flip; }
static void arm_arith(void) { fz_hooks.byte_arithmetic = bad_arith; }
static void arm_boundary(void) { fz_hooks.boundary_insert = bad_boundary; }
static void arm_delete(void) { fz_hooks.block_delete = bad_delete; }
static void arm_insert(void) { fz_hooks.block_insert = bad_insert; }
static void arm_dup(void) { fz_hooks.block_duplicate = bad_dup; }
static void arm_splice(void) { fz_hooks.splice = bad_splice; }
static void arm_sequence(void) { fz_hooks.mutate_sequence = bad_sequence; }
static void arm_admit(void) { fz_hooks.corpus_admit = bad_admit; }
static void arm_dupcrash(void) { fz_hooks.is_duplicate = bad_dup_crash; }
static void arm_fid(void) { fz_hooks.is_failure_id = bad_fid; }
static void arm_tap(void) { fz_hooks.format_tap = bad_tap; }
static void arm_tfv(void) { fz_hooks.tfv_validate = bad_tfv; }

static void test_sabotage(void)
{
    sabotaged("bit_flip", arm_bit_flip);
    sabotaged("byte_flip", arm_byte_flip);
    sabotaged("byte_arithmetic", arm_arith);
    sabotaged("boundary_insert", arm_boundary);
    sabotaged("block_delete", arm_delete);
    sabotaged("block_insert", arm_insert);
    sabotaged("block_duplicate", arm_dup);
    sabotaged("splice", arm_splice);
    sabotaged("mutate_sequence", arm_sequence);
    sabotaged("corpus_admit", arm_admit);
    sabotaged("is_duplicate", arm_dupcrash);
    sabotaged("is_failure_id", arm_fid);
    sabotaged("format_tap", arm_tap);
    sabotaged("tfv_validate", arm_tfv);
}

/* ---- hostile documents ------------------------------------------------------------------------------------- */

static void eval_text(const char *text, pbt_corpus_report *r)
{
    pbt_json *doc = NULL;
    char err[200];
    ISO_CHECK(pbt_json_parse(text, strlen(text), &doc, err, sizeof err) == 0);
    eval(doc, r);
    pbt_json_free(doc);
}

static int any_problem_contains(const pbt_corpus_report *r, const char *needle)
{
    size_t i;
    for (i = 0; i < r->nstored; i++) {
        if (strstr(r->problems[i], needle) != NULL) {
            return 1;
        }
    }
    return 0;
}

static void test_hostile_documents(void)
{
    pbt_corpus_report r;
    eval_text("{\"schema_version\":2,\"cases\":[]}", &r);
    ISO_CHECK(any_problem_contains(&r, "schema_version"));
    pbt_corpus_report_free(&r);
    eval_text("{\"cases\":[]}", &r);
    ISO_CHECK(r.nproblems > 0);
    pbt_corpus_report_free(&r);
    eval_text("[]", &r);
    ISO_CHECK(r.nproblems > 0);
    pbt_corpus_report_free(&r);
    /* unknown op, extra field, missing id, non-object case, duplicate id */
    eval_text("{\"schema_version\":1,\"cases\":[{\"id\":\"a\",\"op\":\"nope\"},{\"id\":\"b\",\"op\":\"corpus_reject\","
              "\"text\":\"x\",\"extra\":1},7,{\"op\":\"corpus_reject\",\"text\":\"x\"},"
              "{\"id\":\"d\",\"op\":\"corpus_reject\",\"text\":\"bad\"},{\"id\":\"d\",\"op\":\"corpus_reject\",\"text\":\"bad\"}]}",
              &r);
    ISO_CHECK(any_problem_contains(&r, "a: unknown op"));
    ISO_CHECK(any_problem_contains(&r, "b: fields do not match"));
    ISO_CHECK(any_problem_contains(&r, "case #3 is missing a valid string id"));
    ISO_CHECK(any_problem_contains(&r, "d: duplicate case id"));
    pbt_corpus_report_free(&r);
    /* a non-canonical integer, odd hex, uppercase hex and a too-wide integer are problems, never a skip */
    eval_text("{\"schema_version\":1,\"cases\":[{\"id\":\"i1\",\"op\":\"mutate_bit_flip\",\"input\":{\"kind\":\"bytes\","
              "\"hex\":\"0\"},\"bit_offset\":{\"kind\":\"int\",\"value\":\"007\"},\"count\":{\"kind\":\"int\","
              "\"value\":\"1\"},\"output\":{\"kind\":\"bytes\",\"hex\":\"\"}},"
              "{\"id\":\"i2\",\"op\":\"mutate_bit_flip\",\"input\":{\"kind\":\"bytes\",\"hex\":\"AB\"},"
              "\"bit_offset\":{\"kind\":\"int\",\"value\":\"1\"},\"count\":{\"kind\":\"int\",\"value\":\"1\"},"
              "\"output\":{\"kind\":\"bytes\",\"hex\":\"\"}}]}",
              &r);
    ISO_CHECK(any_problem_contains(&r, "i1: could not be evaluated"));
    ISO_CHECK(any_problem_contains(&r, "i2: could not be evaluated"));
    ISO_CHECK_EQ_INT(r.evaluated, 0);
    pbt_corpus_report_free(&r);
    /* case ids are escaped in problem lines: a control byte in an id shows as '?' */
    eval_text("{\"schema_version\":1,\"cases\":[{\"id\":\"x\\ny\\u0001\",\"op\":\"nope\"}]}", &r);
    ISO_CHECK(any_problem_contains(&r, "x?y?: unknown op"));
    {
        size_t i;
        for (i = 0; i < r.nstored; i++) {
            ISO_CHECK(strchr(r.problems[i], '\n') == NULL);
        }
    }
    pbt_corpus_report_free(&r);
}

static int g_calls;
static uint64_t g_report_done;
static int g_report_rc;

/* A mutate_sequence stand-in that reports a chosen amount of work and a chosen status. */
static int fake_sequence(fz_bytes *o, const uint8_t *d, size_t n, uint64_t seed, size_t steps, size_t max_len,
                         uint64_t cap, uint64_t *done)
{
    int rc = fz_mutate_sequence_budget(o, d, n, seed, steps, max_len, cap, done);
    g_calls++;
    if (g_report_rc != FZ_OK) {
        fz_bytes_free(o);
        *done = g_report_done;
        return g_report_rc;
    }
    *done = g_report_done == (uint64_t)-1 ? cap : g_report_done;
    return rc;
}

#define SEQ_CASE(id)                                                                                                 \
    "{\"id\":\"" id "\",\"op\":\"mutate_sequence\",\"input\":{\"kind\":\"bytes\",\"hex\":\"00\"},"            \
    "\"seed\":{\"kind\":\"int\",\"value\":\"1\"},\"steps\":{\"kind\":\"int\",\"value\":\"0\"},"              \
    "\"output\":{\"kind\":\"bytes\",\"hex\":\"00\"}}"

/* The aggregate mutate_sequence work cap: a call that hits its cap (FZ_E_WORK, with LESS work than the whole
 * budget) zeroes the aggregate, so every later case fails at once WITHOUT running; a call that uses exactly what
 * was left does too.  The work of a failed call is charged. */
static void test_work_cap_zeroed(void)
{
    pbt_corpus_report r;
    const char *three = "{\"schema_version\":1,\"cases\":[" SEQ_CASE("one") "," SEQ_CASE("two") "," SEQ_CASE("three") "]}";
    /* breach with work below the cap */
    g_calls = 0;
    g_report_rc = FZ_E_WORK;
    g_report_done = 5;
    fz_hooks.mutate_sequence = fake_sequence;
    eval_text(three, &r);
    fz_hooks_reset();
    ISO_CHECK(any_problem_contains(&r, "one: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK(any_problem_contains(&r, "two: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK(any_problem_contains(&r, "three: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK_EQ_INT(g_calls, 1); /* the later cases never ran */
    pbt_corpus_report_free(&r);
    /* a successful call that consumed the whole budget */
    g_calls = 0;
    g_report_rc = FZ_OK;
    g_report_done = (uint64_t)-1;
    fz_hooks.mutate_sequence = fake_sequence;
    eval_text(three, &r);
    fz_hooks_reset();
    ISO_CHECK(!any_problem_contains(&r, "one:"));
    ISO_CHECK(any_problem_contains(&r, "two: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK(any_problem_contains(&r, "three: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK_EQ_INT(g_calls, 1);
    pbt_corpus_report_free(&r);
    /* partial charges add up: 3 calls of 1/3 of the budget exhaust it by the fourth */
    g_calls = 0;
    g_report_rc = FZ_OK;
    g_report_done = FZ_CORPUS_MAX_SEQUENCE_WORK / 3 + 1;
    fz_hooks.mutate_sequence = fake_sequence;
    eval_text("{\"schema_version\":1,\"cases\":[" SEQ_CASE("a") "," SEQ_CASE("b") "," SEQ_CASE("c") "," SEQ_CASE("d") "]}",
              &r);
    fz_hooks_reset();
    ISO_CHECK(!any_problem_contains(&r, "a:"));
    ISO_CHECK(!any_problem_contains(&r, "b:"));
    ISO_CHECK(!any_problem_contains(&r, "c:"));
    ISO_CHECK(any_problem_contains(&r, "d: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK_EQ_INT(g_calls, 3);
    pbt_corpus_report_free(&r);
    /* the work of a FAILED call (any failure, not only a breach) is charged too: two refused calls of just over
     * half the budget each exhaust it, so the third case fails without running */
    g_calls = 0;
    g_report_rc = FZ_E_LIMIT;
    g_report_done = FZ_CORPUS_MAX_SEQUENCE_WORK / 2 + 1;
    fz_hooks.mutate_sequence = fake_sequence;
    eval_text(three, &r);
    fz_hooks_reset();
    ISO_CHECK(any_problem_contains(&r, "one: could not be evaluated: the engine refused"));
    ISO_CHECK(any_problem_contains(&r, "two: could not be evaluated: the engine refused"));
    ISO_CHECK(any_problem_contains(&r, "three: could not be evaluated: mutate_sequence work budget"));
    ISO_CHECK_EQ_INT(g_calls, 2);
    pbt_corpus_report_free(&r);
    g_report_rc = FZ_OK;
    /* the same case with the real engine and no hook is fine */
    eval_text("{\"schema_version\":1,\"cases\":[" SEQ_CASE("solo") "]}", &r);
    ISO_CHECK(!any_problem_contains(&r, "solo:"));
    pbt_corpus_report_free(&r);
}

int main(void)
{
    test_conforms();
    test_per_case_corruption();
    test_dropped_and_truncated();
    test_sabotage();
    test_hostile_documents();
    test_work_cap_zeroed();
    fz_hooks_reset();
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    return ISO_TEST_RESULT();
}

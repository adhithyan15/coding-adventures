/*
 * pbt_corpus.c -- evaluator for the shared pbt-v1 conformance corpus
 * (VER05 section 6, VER05i section 8).
 *
 * The corpus pins what every language lane's engine must reproduce: exact
 * generator outputs for a seed, exact shrink minimals AND step counts, the
 * replay of a failure, the exhausted-filter verdict, the failure vocabulary and
 * strict-TFV rejections.  This evaluator runs every case through the engine's
 * OWN public functions (never a shortcut) and reports PROBLEMS, one string each,
 * starting with the case id.  No problems means "conforms".
 *
 * It is deliberately paranoid about its input, because the input is a file:
 *   - exactly the fields of each op, nothing missing, nothing extra;
 *   - integers are canonical decimal strings, bytes are lowercase even hex;
 *   - an unknown op / property / malformed value is a PROBLEM, never a skip;
 *   - sizes (counts, lengths, retries) are capped at 10,000 and the TOTAL work
 *     of a gen case at 2^20 bytes: a hostile fixture is refused, not run;
 *   - every fixture fragment copied into a problem is clipped (200 / 400 bytes);
 *   - recursion is bounded to 64 levels.
 */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "pbt.h"
#include "pbt_internal.h"
#include "pbt_tfv.h"

#define CLIP_FRAGMENT 200
#define CLIP_PROBLEM 400
#define MAX_TOTAL_GEN ((uint64_t)1 << 20)

/* ---- the report ----------------------------------------------------------------- */

void pbt_corpus_report_init(pbt_corpus_report *r)
{
    memset(r, 0, sizeof *r);
}

void pbt_corpus_report_free(pbt_corpus_report *r)
{
    size_t i;
    for (i = 0; i < r->nstored; i++) {
        pbt_free(r->problems[i]);
    }
    pbt_free(r->problems);
    pbt_corpus_report_init(r);
}

/* Copy up to CLIP_FRAGMENT bytes of fixture text, replacing anything unprintable, and mark truncation. */
static void clip(char *dst, const char *s, size_t n)
{
    size_t m = n < CLIP_FRAGMENT ? n : CLIP_FRAGMENT;
    size_t i;
    for (i = 0; i < m; i++) {
        unsigned char ch = (unsigned char)s[i];
        dst[i] = (ch >= 0x20 && ch < 0x7f) ? (char)ch : '?';
    }
    if (n > m) {
        dst[m - 3] = '.';
        dst[m - 2] = '.';
        dst[m - 1] = '.';
    }
    dst[m] = '\0';
}

/* Record "<id>: <text>".  id may be NULL (a document-level problem). */
static void problem(pbt_corpus_report *r, const char *id, const char *fmt, ...)
{
    char body[CLIP_PROBLEM + 8];
    char line[CLIP_PROBLEM + CLIP_FRAGMENT + 16];
    char idc[CLIP_FRAGMENT + 1];
    va_list ap;
    size_t len;
    char *copy;
    va_start(ap, fmt);
    (void)vsnprintf(body, sizeof body, fmt, ap);
    va_end(ap);
    if (id != NULL) {
        clip(idc, id, strlen(id));
        (void)snprintf(line, sizeof line, "%s: %s", idc, body);
    } else {
        (void)snprintf(line, sizeof line, "%s", body);
    }
    len = strlen(line);
    if (len > CLIP_PROBLEM) {
        line[CLIP_PROBLEM - 3] = '.';
        line[CLIP_PROBLEM - 2] = '.';
        line[CLIP_PROBLEM - 1] = '.';
        line[CLIP_PROBLEM] = '\0';
        len = CLIP_PROBLEM;
    }
    r->nproblems++;
    if (r->nstored >= PBT_CORPUS_MAX_PROBLEMS) {
        return;
    }
    if (r->problems == NULL) {
        r->problems = (char **)pbt_alloc(PBT_CORPUS_MAX_PROBLEMS, sizeof(char *));
        if (r->problems == NULL) {
            return;
        }
    }
    copy = (char *)pbt_alloc(len + 1, 1);
    if (copy == NULL) {
        return;
    }
    memcpy(copy, line, len + 1);
    r->problems[r->nstored++] = copy;
}

/* ---- strict decoding --------------------------------------------------------------- */

/* A short reason a decode failed, in a static buffer owned by the caller. */
typedef struct why {
    char text[160];
} why;

static int fail_why(why *w, const char *m)
{
    (void)snprintf(w->text, sizeof w->text, "%s", m);
    return 1;
}

static int is_str(const pbt_json *n)
{
    return n != NULL && n->type == PBT_J_STR;
}

/* The node is a string equal to the literal, embedded NULs included (JSON strings may contain \u0000). */
static int str_is(const pbt_json *n, const char *lit)
{
    return is_str(n) && n->slen == strlen(lit) && memcmp(n->s, lit, n->slen) == 0;
}

/* The object has exactly the listed members (no more, no fewer). */
static int has_exact_fields(const pbt_json *obj, const char *const *names, size_t n)
{
    size_t i;
    if (obj->type != PBT_J_OBJ || obj->n != n) {
        return 0;
    }
    for (i = 0; i < n; i++) {
        if (pbt_json_get(obj, names[i]) == NULL) {
            return 0;
        }
    }
    return 1;
}

/* A canonical decimal integer: -?(0|[1-9][0-9]*), never "-0", at most PBT_BIG_MAX_DIGITS digits. */
static int canonical_int(const char *s, size_t n)
{
    size_t i = 0;
    size_t digits;
    if (n > 0 && s[0] == '-') {
        i = 1;
    }
    digits = n - i;
    if (digits == 0) {
        return 0;
    }
    for (; i < n; i++) {
        if (s[i] < '0' || s[i] > '9') {
            return 0;
        }
    }
    i = n - digits;
    if (s[i] == '0' && (digits > 1 || i == 1)) {
        return 0;
    }
    return digits <= PBT_BIG_MAX_DIGITS;
}

/* {"kind":"int","value":"<canonical decimal>"} */
static int dec_int(const pbt_json *n, pbt_big *out, why *w)
{
    static const char *const names[] = {"kind", "value"};
    const pbt_json *kind;
    const pbt_json *val;
    pbt_big_zero(out);
    if (n == NULL || !has_exact_fields(n, names, 2)) {
        return fail_why(w, "an int must be {kind, value}");
    }
    kind = pbt_json_get(n, "kind");
    val = pbt_json_get(n, "value");
    if (!str_is(kind, "int")) {
        return fail_why(w, "an int needs kind \"int\"");
    }
    if (!is_str(val) || !canonical_int(val->s, val->slen)) {
        return fail_why(w, "an int value must be a canonical decimal string within the digit cap");
    }
    if (pbt_big_parse_dec(out, val->s, val->slen) != 0) {
        return fail_why(w, "an int value is too wide for this lane");
    }
    return 0;
}

/* A small non-negative int (count, length, retry...) in [lo, hi]. */
static int dec_small(const pbt_json *n, uint64_t lo, uint64_t hi, uint64_t *out, why *w)
{
    pbt_big b;
    *out = 0;
    if (dec_int(n, &b, w)) {
        return 1;
    }
    if (pbt_big_sign(&b) < 0 || !pbt_big_fits_u64(&b)) {
        return fail_why(w, "a size must be a small non-negative int");
    }
    *out = pbt_big_low_u64(&b);
    if (*out < lo || *out > hi) {
        return fail_why(w, "a size is outside its cap");
    }
    return 0;
}

static int hexval(char c)
{
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    return -1;
}

/* {"kind":"bytes","hex":"<even-length lowercase hex>"} into a BYTES value. */
static int dec_bytes(const pbt_json *n, pbt_val *out, why *w)
{
    static const char *const names[] = {"kind", "hex"};
    const pbt_json *kind;
    const pbt_json *hex;
    size_t i;
    int rc;
    pbt_val_nil(out);
    if (n == NULL || !has_exact_fields(n, names, 2)) {
        return fail_why(w, "bytes must be {kind, hex}");
    }
    kind = pbt_json_get(n, "kind");
    hex = pbt_json_get(n, "hex");
    if (!str_is(kind, "bytes") || !is_str(hex)) {
        return fail_why(w, "bytes need kind \"bytes\" and a hex string");
    }
    if (hex->slen % 2 != 0 || hex->slen / 2 > PBT_CORPUS_MAX_SIZE) {
        return fail_why(w, "bytes hex must be even length and within the cap");
    }
    rc = pbt_val_set_bytes(out, NULL, hex->slen / 2);
    if (rc != 0) {
        return fail_why(w, "out of memory");
    }
    for (i = 0; i < hex->slen; i += 2) {
        int h = hexval(hex->s[i]);
        int l = hexval(hex->s[i + 1]);
        if (h < 0 || l < 0) {
            pbt_val_free(out);
            return fail_why(w, "bytes hex must be lowercase");
        }
        out->u.bytes.p[i / 2] = (uint8_t)(h * 16 + l);
    }
    return 0;
}

static int dec_value(const pbt_json *n, int depth, pbt_val *out, why *w);

/* A JSON array of values into a LIST value. */
static int dec_list(const pbt_json *n, int depth, pbt_val *out, why *w)
{
    size_t i;
    pbt_val_nil(out);
    if (n == NULL || n->type != PBT_J_ARR) {
        return fail_why(w, "expected a list");
    }
    if (n->n > PBT_CORPUS_MAX_SIZE) {
        return fail_why(w, "a list is longer than the cap");
    }
    if (pbt_val_set_list(out, PBT_V_LIST, n->n) != 0) {
        return fail_why(w, "out of memory");
    }
    for (i = 0; i < n->n; i++) {
        if (dec_value(&n->items[i], depth + 1, &out->u.list.items[i], w)) {
            pbt_val_free(out);
            return 1;
        }
    }
    return 0;
}

/* bool | int | bytes | list, bounded to 64 levels. */
static int dec_value(const pbt_json *n, int depth, pbt_val *out, why *w)
{
    pbt_big b;
    pbt_val_nil(out);
    if (depth > PBT_MAX_VAL_DEPTH) {
        return fail_why(w, "values nest too deeply");
    }
    if (n->type == PBT_J_BOOL) {
        pbt_val_set_bool(out, n->b);
        return 0;
    }
    if (n->type == PBT_J_ARR) {
        return dec_list(n, depth, out, w);
    }
    if (n->type == PBT_J_OBJ) {
        const pbt_json *kind = pbt_json_get(n, "kind");
        if (str_is(kind, "bytes")) {
            return dec_bytes(n, out, w);
        }
        if (dec_int(n, &b, w)) {
            return 1;
        }
        pbt_val_set_int(out, &b);
        return 0;
    }
    return fail_why(w, "unrecognised value");
}

/* ---- the named properties -------------------------------------------------------------- */
/* Each returns 1 when the property HOLDS, 0 when it fails, negative on error. */

static int cmp_i64(const pbt_big *x, int64_t v)
{
    pbt_big b;
    pbt_big_set_i64(&b, v);
    return pbt_big_cmp(x, &b);
}

static int p_int_le_100(const pbt_val *v)
{
    return cmp_i64(&v->u.i, 100) <= 0;
}

static int p_int_in_range_neg10_50(const pbt_val *v)
{
    return cmp_i64(&v->u.i, -10) >= 0 && cmp_i64(&v->u.i, 50) <= 0;
}

static int p_int_is_even(const pbt_val *v)
{
    return v->u.i.n == 0 || (v->u.i.d[0] & 1u) == 0;
}

static int p_int_abs_le_10(const pbt_val *v)
{
    pbt_big a;
    pbt_big_abs(&a, &v->u.i);
    return cmp_i64(&a, 10) <= 0;
}

static int p_refinement_x_le_50(const pbt_val *v)
{
    return cmp_i64(&v->u.i, 50) <= 0;
}

static int p_list_no_consecutive_duplicates(const pbt_val *v)
{
    size_t i;
    for (i = 0; i + 1 < v->u.list.n; i++) {
        if (v->u.list.items[i].kind != PBT_V_INT || v->u.list.items[i + 1].kind != PBT_V_INT) {
            return -1;
        }
        if (pbt_big_cmp(&v->u.list.items[i].u.i, &v->u.list.items[i + 1].u.i) == 0) {
            return 0;
        }
    }
    return 1;
}

static int p_list_sum_le_20(const pbt_val *v)
{
    pbt_big sum;
    size_t i;
    pbt_big_zero(&sum);
    for (i = 0; i < v->u.list.n; i++) {
        if (v->u.list.items[i].kind != PBT_V_INT || pbt_big_add(&sum, &sum, &v->u.list.items[i].u.i) != 0) {
            return -1;
        }
    }
    return cmp_i64(&sum, 20) <= 0;
}

static int p_bytes_len_lt_5(const pbt_val *v)
{
    return v->u.bytes.n < 5;
}

static int p_bytes_no_zero_byte(const pbt_val *v)
{
    size_t i;
    for (i = 0; i < v->u.bytes.n; i++) {
        if (v->u.bytes.p[i] == 0) {
            return 0;
        }
    }
    return 1;
}

typedef int (*cprop_fn)(const pbt_val *v);
typedef struct cprop {
    const char *name;
    pbt_kind kind;
    cprop_fn fn;
} cprop;

static const cprop PROPS[] = {
    {"int_le_100", PBT_V_INT, p_int_le_100},
    {"int_in_range_neg10_50", PBT_V_INT, p_int_in_range_neg10_50},
    {"int_is_even", PBT_V_INT, p_int_is_even},
    {"int_abs_le_10", PBT_V_INT, p_int_abs_le_10},
    {"list_no_consecutive_duplicates", PBT_V_LIST, p_list_no_consecutive_duplicates},
    {"list_sum_le_20", PBT_V_LIST, p_list_sum_le_20},
    {"bytes_len_lt_5", PBT_V_BYTES, p_bytes_len_lt_5},
    {"bytes_no_zero_byte", PBT_V_BYTES, p_bytes_no_zero_byte},
    {"refinement_x_le_50", PBT_V_INT, p_refinement_x_le_50},
};

static const cprop *find_prop(const pbt_json *n, pbt_kind kind)
{
    size_t i;
    if (!is_str(n)) {
        return NULL;
    }
    for (i = 0; i < sizeof PROPS / sizeof PROPS[0]; i++) {
        if (strcmp(PROPS[i].name, n->s) == 0 && n->slen == strlen(PROPS[i].name)) {
            return PROPS[i].kind == kind ? &PROPS[i] : NULL;
        }
    }
    return NULL;
}

static int corpus_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    const cprop *p = (const cprop *)user;
    (void)scope;
    if (v->kind != p->kind) {
        return -1;
    }
    return p->fn(v);
}

static pbt_prop as_prop(const cprop *p)
{
    pbt_prop pr;
    pr.fn = corpus_prop;
    pr.user = (void *)p;
    return pr;
}

/* ---- the per-op evaluators ---------------------------------------------------------------- */

typedef struct ctx {
    pbt_corpus_report *rep;
    const char *id;
    why w;
} ctx;

#define PROBLEM(c, ...) problem((c)->rep, (c)->id, __VA_ARGS__)

static const pbt_json *fld(const pbt_json *c, const char *name)
{
    return pbt_json_get(c, name);
}

/* Generate `count` values of `gen` from `seed` and compare with the decoded `outputs`. */
static void compare_gen(ctx *c, pbt_gen *gen, uint64_t seed, uint64_t count, const pbt_json *outputs, const char *what)
{
    pbt_run run;
    pbt_val expected;
    size_t i;
    if (gen == NULL) {
        PROBLEM(c, "%s: could not build the generator", what);
        return;
    }
    if (outputs == NULL || outputs->type != PBT_J_ARR || outputs->n != count) {
        PROBLEM(c, "%s: outputs must be a list of exactly count (%lu) values", what, (unsigned long)count);
        return;
    }
    if (dec_list(outputs, 0, &expected, &c->w)) {
        PROBLEM(c, "%s: outputs: %s", what, c->w.text);
        return;
    }
    pbt_rng_init(&run.rng, seed);
    run.budget = NULL;
    run.depth = 0;
    for (i = 0; i < count; i++) {
        pbt_val got;
        int rc = pbt_gen_sample(&run, gen, &got);
        if (rc != 0) {
            PROBLEM(c, "%s: generation failed at index %lu (status %d)", what, (unsigned long)i, rc);
            break;
        }
        if (!pbt_val_equal(&got, &expected.u.list.items[i])) {
            PROBLEM(c, "%s outputs mismatch at index %lu", what, (unsigned long)i);
            pbt_val_free(&got);
            break;
        }
        pbt_val_free(&got);
    }
    pbt_val_free(&expected);
}

static void eval_gen_primitive(ctx *c, const pbt_json *cs)
{
    const pbt_json *type = fld(cs, "type");
    pbt_big seed;
    uint64_t count;
    pbt_gen *gen = NULL;
    if (!is_str(type)) {
        PROBLEM(c, "gen_primitive: type must be a string");
        return;
    }
    if (dec_int(fld(cs, "seed"), &seed, &c->w) || dec_small(fld(cs, "count"), 0, PBT_CORPUS_MAX_SIZE, &count, &c->w)) {
        PROBLEM(c, "gen_primitive: %s", c->w.text);
        return;
    }
    if (str_is(type, "bool")) {
        gen = pbt_gen_bool();
    } else if (str_is(type, "u8")) {
        gen = pbt_gen_u8();
    } else if (str_is(type, "u64")) {
        gen = pbt_gen_u64();
    } else if (str_is(type, "i128")) {
        gen = pbt_gen_i128();
    } else {
        char frag[CLIP_FRAGMENT + 1];
        clip(frag, type->s, type->slen);
        PROBLEM(c, "gen_primitive: unknown primitive type \"%s\"", frag);
        return;
    }
    compare_gen(c, gen, pbt_big_mod_u64(&seed), count, fld(cs, "outputs"), "gen_primitive");
    pbt_gen_release(gen);
}

static void eval_gen_bounded(ctx *c, const pbt_json *cs)
{
    pbt_big lo;
    pbt_big hi;
    pbt_big seed;
    pbt_big span;
    uint64_t count;
    uint64_t width;
    pbt_gen *gen;
    if (dec_int(fld(cs, "min"), &lo, &c->w) || dec_int(fld(cs, "max"), &hi, &c->w) ||
        dec_int(fld(cs, "seed"), &seed, &c->w) || dec_small(fld(cs, "count"), 0, PBT_CORPUS_MAX_SIZE, &count, &c->w)) {
        PROBLEM(c, "gen_bounded: %s", c->w.text);
        return;
    }
    if (pbt_big_cmp(&lo, &hi) > 0) {
        PROBLEM(c, "gen_bounded: min exceeds max");
        return;
    }
    (void)pbt_big_sub(&span, &hi, &lo);
    width = (pbt_big_bitlen(&span) + 7u) / 8u;
    if (width < 1) {
        width = 1;
    }
    /* count x width is the total-generation budget: both are small, so no overflow. */
    if (count * width > MAX_TOTAL_GEN) {
        PROBLEM(c, "gen_bounded: count x width exceeds the total generation budget");
        return;
    }
    gen = pbt_gen_int(&lo, &hi);
    compare_gen(c, gen, pbt_big_mod_u64(&seed), count, fld(cs, "outputs"), "gen_bounded");
    pbt_gen_release(gen);
}

static void eval_gen_bytes(ctx *c, const pbt_json *cs)
{
    pbt_big seed;
    uint64_t lo;
    uint64_t hi;
    uint64_t count;
    pbt_gen *gen;
    if (dec_small(fld(cs, "min_len"), 0, PBT_CORPUS_MAX_SIZE, &lo, &c->w) ||
        dec_small(fld(cs, "max_len"), 0, PBT_CORPUS_MAX_SIZE, &hi, &c->w) || dec_int(fld(cs, "seed"), &seed, &c->w) ||
        dec_small(fld(cs, "count"), 0, PBT_CORPUS_MAX_SIZE, &count, &c->w)) {
        PROBLEM(c, "gen_bytes: %s", c->w.text);
        return;
    }
    if (lo > hi) {
        PROBLEM(c, "gen_bytes: min_len exceeds max_len");
        return;
    }
    if (hi > 0 && count * hi > MAX_TOTAL_GEN) {
        PROBLEM(c, "gen_bytes: count x max_len exceeds the total generation budget");
        return;
    }
    gen = pbt_gen_bytes((size_t)lo, (size_t)hi);
    compare_gen(c, gen, pbt_big_mod_u64(&seed), count, fld(cs, "outputs"), "gen_bytes");
    pbt_gen_release(gen);
}

static void eval_shrink_int(ctx *c, const pbt_json *cs, const char *op)
{
    pbt_big initial;
    pbt_big target;
    pbt_big lo;
    pbt_big hi;
    pbt_big expected;
    pbt_big minimal;
    uint64_t exp_steps;
    int steps = 0;
    int rc;
    const cprop *p = find_prop(fld(cs, "property"), PBT_V_INT);
    if (p == NULL) {
        PROBLEM(c, "%s: unknown or mistyped property", op);
        return;
    }
    if (dec_int(fld(cs, "initial"), &initial, &c->w) || dec_int(fld(cs, "target"), &target, &c->w) ||
        dec_int(fld(cs, "min"), &lo, &c->w) || dec_int(fld(cs, "max"), &hi, &c->w) ||
        dec_int(fld(cs, "expected_minimal"), &expected, &c->w) ||
        dec_small(fld(cs, "expected_steps"), 0, PBT_MAX_SHRINK_STEPS, &exp_steps, &c->w)) {
        PROBLEM(c, "%s: %s", op, c->w.text);
        return;
    }
    rc = pbt_shrink_int(NULL, &initial, &target, &lo, &hi, as_prop(p), 1000, &minimal, &steps);
    if (rc != 0) {
        PROBLEM(c, "%s: shrinking failed (status %d)", op, rc);
        return;
    }
    if (pbt_big_cmp(&minimal, &expected) != 0) {
        PROBLEM(c, "%s minimal mismatch", op);
    }
    if ((uint64_t)steps != exp_steps) {
        PROBLEM(c, "%s step count mismatch: got %d, want %lu", op, steps, (unsigned long)exp_steps);
    }
}

static void eval_shrink_list(ctx *c, const pbt_json *cs)
{
    pbt_val list;
    pbt_val expected;
    pbt_big target;
    uint64_t exp_steps;
    int steps = 0;
    int rc;
    size_t i;
    const cprop *p = find_prop(fld(cs, "property"), PBT_V_LIST);
    if (p == NULL) {
        PROBLEM(c, "shrink_list: unknown or mistyped property");
        return;
    }
    if (dec_list(fld(cs, "initial"), 0, &list, &c->w)) {
        PROBLEM(c, "shrink_list: initial: %s", c->w.text);
        return;
    }
    if (dec_list(fld(cs, "expected_minimal"), 0, &expected, &c->w)) {
        PROBLEM(c, "shrink_list: expected_minimal: %s", c->w.text);
        pbt_val_free(&list);
        return;
    }
    for (i = 0; i < list.u.list.n; i++) {
        if (list.u.list.items[i].kind != PBT_V_INT) {
            PROBLEM(c, "shrink_list: initial must be a list of ints");
            goto out;
        }
    }
    if (dec_int(fld(cs, "elem_target"), &target, &c->w) ||
        dec_small(fld(cs, "expected_steps"), 0, PBT_MAX_SHRINK_STEPS, &exp_steps, &c->w)) {
        PROBLEM(c, "shrink_list: %s", c->w.text);
        goto out;
    }
    rc = pbt_shrink_list_ints(NULL, &list, &target, as_prop(p), 1000, &steps);
    if (rc != 0) {
        PROBLEM(c, "shrink_list: shrinking failed (status %d)", rc);
        goto out;
    }
    if (!pbt_val_equal(&list, &expected)) {
        PROBLEM(c, "shrink_list minimal mismatch");
    }
    if ((uint64_t)steps != exp_steps) {
        PROBLEM(c, "shrink_list steps mismatch: got %d, want %lu", steps, (unsigned long)exp_steps);
    }
out:
    pbt_val_free(&list);
    pbt_val_free(&expected);
}

static void eval_shrink_bytes(ctx *c, const pbt_json *cs)
{
    pbt_val bytes;
    pbt_val expected;
    uint64_t exp_steps;
    int steps = 0;
    int rc;
    const cprop *p = find_prop(fld(cs, "property"), PBT_V_BYTES);
    if (p == NULL) {
        PROBLEM(c, "shrink_bytes: unknown or mistyped property");
        return;
    }
    if (dec_bytes(fld(cs, "initial"), &bytes, &c->w)) {
        PROBLEM(c, "shrink_bytes: initial: %s", c->w.text);
        return;
    }
    if (dec_bytes(fld(cs, "expected_minimal"), &expected, &c->w)) {
        PROBLEM(c, "shrink_bytes: expected_minimal: %s", c->w.text);
        pbt_val_free(&bytes);
        return;
    }
    if (dec_small(fld(cs, "expected_steps"), 0, PBT_MAX_SHRINK_STEPS, &exp_steps, &c->w)) {
        PROBLEM(c, "shrink_bytes: %s", c->w.text);
        goto out;
    }
    rc = pbt_shrink_bytes(NULL, &bytes, as_prop(p), 1000, &steps);
    if (rc != 0) {
        PROBLEM(c, "shrink_bytes: shrinking failed (status %d)", rc);
        goto out;
    }
    if (!pbt_val_equal(&bytes, &expected)) {
        PROBLEM(c, "shrink_bytes minimal mismatch");
    }
    if ((uint64_t)steps != exp_steps) {
        PROBLEM(c, "shrink_bytes steps mismatch: got %d, want %lu", steps, (unsigned long)exp_steps);
    }
out:
    pbt_val_free(&bytes);
    pbt_val_free(&expected);
}

static void eval_replay(ctx *c, const pbt_json *cs)
{
    pbt_big seed;
    pbt_big want_initial;
    pbt_big want_min;
    pbt_big got_initial;
    pbt_big got_min;
    int rc;
    const cprop *p = find_prop(fld(cs, "property"), PBT_V_INT);
    if (p == NULL) {
        PROBLEM(c, "replay_consistency: unknown or mistyped property");
        return;
    }
    if (dec_int(fld(cs, "seed"), &seed, &c->w) || dec_int(fld(cs, "initial_fail"), &want_initial, &c->w) ||
        dec_int(fld(cs, "expected_minimal"), &want_min, &c->w)) {
        PROBLEM(c, "replay_consistency: %s", c->w.text);
        return;
    }
    rc = pbt_simulate_replay(NULL, pbt_big_mod_u64(&seed), as_prop(p), &got_initial, &got_min);
    if (rc != 0) {
        PROBLEM(c, "replay_consistency: replay failed (status %d)", rc);
        return;
    }
    if (pbt_big_cmp(&got_initial, &want_initial) != 0) {
        PROBLEM(c, "replay initial_fail mismatch");
    }
    if (pbt_big_cmp(&got_min, &want_min) != 0) {
        PROBLEM(c, "replay minimal mismatch");
    }
}

/* The corpus filter: a precondition that nothing satisfies, counting its own calls. */
static int never(void *self, pbt_budget *b, const pbt_val *v)
{
    (void)b;
    (void)v;
    ++*(unsigned long *)self;
    return 0;
}

static int always_holds(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)user;
    (void)scope;
    (void)v;
    return 1;
}

static void eval_filter_exhausted(ctx *c, const pbt_json *cs)
{
    uint64_t retries;
    const pbt_json *verdict = fld(cs, "verdict");
    const pbt_json *fid = fld(cs, "failure_id");
    unsigned long calls = 0;
    pbt_gen *base;
    pbt_gen *filtered;
    pbt_verdict v;
    pbt_options o;
    pbt_prop prop;
    if (dec_small(fld(cs, "max_retries"), 1, PBT_CORPUS_MAX_SIZE, &retries, &c->w)) {
        PROBLEM(c, "filter_exhausted: %s", c->w.text);
        return;
    }
    if (!is_str(verdict) || !is_str(fid)) {
        PROBLEM(c, "filter_exhausted: verdict and failure_id must be strings");
        return;
    }
    base = pbt_gen_u8();
    filtered = pbt_gen_filter(base, never, &calls, (int)retries);
    pbt_gen_release(base);
    if (filtered == NULL) {
        PROBLEM(c, "filter_exhausted: could not build the filter");
        return;
    }
    o.num_tests = 1;
    o.max_shrink_steps = 1000;
    o.ci = PBT_CI_OFF;
    prop.fn = always_holds;
    prop.user = NULL;
    (void)pbt_check(NULL, 1, filtered, prop, &o, NULL, &v);
    /* The end-to-end run: the verdict must be fail / property-exhausted after exactly max_retries predicate calls. */
    if (v.passed || v.failure_id == NULL || strcmp(v.failure_id, PBT_FAIL_EXHAUSTED) != 0) {
        PROBLEM(c, "filter_exhausted: the run did not report property-exhausted");
    }
    if (calls != (unsigned long)retries) {
        PROBLEM(c, "filter_exhausted: predicate called %lu times, want %lu", calls, (unsigned long)retries);
    }
    if (!str_is(verdict, "fail") || v.failure_id == NULL || !str_is(fid, v.failure_id)) {
        PROBLEM(c, "filter_exhausted: expected verdict/failure_id disagree with the engine");
    }
    pbt_verdict_free(&v);
    pbt_gen_release(filtered);
}

typedef struct seen {
    int ids[4];
    int rejected;
} seen;

static const char *const FAILURE_IDS[4] = {PBT_FAIL_COUNTEREXAMPLE, PBT_FAIL_EXHAUSTED, PBT_FAIL_TIMEOUT,
                                           PBT_FAIL_PANIC};

static void eval_failure_id(ctx *c, const pbt_json *cs, seen *s)
{
    const pbt_json *in = fld(cs, "input");
    const pbt_json *valid = fld(cs, "valid");
    int got;
    int i;
    if (!is_str(in) || valid == NULL || valid->type != PBT_J_BOOL) {
        PROBLEM(c, "failure_id_valid: input must be a string and valid a boolean");
        return;
    }
    got = pbt_is_failure_id(in->s) && strlen(in->s) == in->slen;
    if (got != valid->b) {
        PROBLEM(c, "failure_id_valid membership disagrees");
    }
    if (got) {
        for (i = 0; i < 4; i++) {
            if (strcmp(in->s, FAILURE_IDS[i]) == 0) {
                s->ids[i] = 1;
            }
        }
    } else {
        s->rejected = 1;
    }
}

static void eval_corpus_reject(ctx *c, const pbt_json *cs)
{
    const pbt_json *text = fld(cs, "text");
    char err[160];
    if (!is_str(text)) {
        PROBLEM(c, "corpus_reject: text must be a string");
        return;
    }
    if (pbt_tfv_validate(text->s, text->slen, err, sizeof err) == 0) {
        PROBLEM(c, "corpus_reject: text was accepted by the strict TFV validator");
    }
}

/* ---- shapes and the document ------------------------------------------------------------------ */

typedef struct shape {
    const char *op;
    const char *const *fields; /* besides id and op */
    size_t n;
} shape;

static const char *const F_GEN_PRIMITIVE[] = {"type", "seed", "count", "outputs"};
static const char *const F_GEN_BOUNDED[] = {"min", "max", "seed", "count", "outputs"};
static const char *const F_GEN_BYTES[] = {"min_len", "max_len", "seed", "count", "outputs"};
static const char *const F_SHRINK_INT[] = {"initial", "target", "min", "max", "property", "expected_minimal",
                                           "expected_steps"};
static const char *const F_SHRINK_LIST[] = {"initial", "elem_target", "property", "expected_minimal", "expected_steps"};
static const char *const F_SHRINK_BYTES[] = {"initial", "property", "expected_minimal", "expected_steps"};
static const char *const F_REPLAY[] = {"seed", "property", "initial_fail", "expected_minimal"};
static const char *const F_FILTER[] = {"max_retries", "verdict", "failure_id"};
static const char *const F_FAILURE_ID[] = {"input", "valid"};
static const char *const F_REJECT[] = {"text"};

#define SHAPE(name, arr) {name, arr, sizeof arr / sizeof arr[0]}
static const shape SHAPES[] = {
    SHAPE("gen_primitive", F_GEN_PRIMITIVE),
    SHAPE("gen_bounded", F_GEN_BOUNDED),
    SHAPE("gen_bytes", F_GEN_BYTES),
    SHAPE("shrink_int", F_SHRINK_INT),
    SHAPE("shrink_refinement", F_SHRINK_INT),
    SHAPE("shrink_list", F_SHRINK_LIST),
    SHAPE("shrink_bytes", F_SHRINK_BYTES),
    SHAPE("replay_consistency", F_REPLAY),
    SHAPE("filter_exhausted", F_FILTER),
    SHAPE("failure_id_valid", F_FAILURE_ID),
    SHAPE("corpus_reject", F_REJECT),
};
#define N_SHAPES (sizeof SHAPES / sizeof SHAPES[0])

/* The case has exactly id, op and the op's fields. */
static int shape_matches(const pbt_json *cs, const shape *sh)
{
    size_t i;
    if (cs->type != PBT_J_OBJ || cs->n != sh->n + 2) {
        return 0;
    }
    if (pbt_json_get(cs, "id") == NULL || pbt_json_get(cs, "op") == NULL) {
        return 0;
    }
    for (i = 0; i < sh->n; i++) {
        if (pbt_json_get(cs, sh->fields[i]) == NULL) {
            return 0;
        }
    }
    return 1;
}

/* Duplicate case ids: compare each id with the rest.  Case counts are tiny (the corpus has 25), and the document is capped. */
static int id_seen_before(const pbt_json *cases, size_t upto, const pbt_json *id)
{
    size_t i;
    for (i = 0; i < upto; i++) {
        const pbt_json *other = pbt_json_get(&cases->items[i], "id");
        if (is_str(other) && other->slen == id->slen && memcmp(other->s, id->s, id->slen) == 0) {
            return 1;
        }
    }
    return 0;
}

int pbt_corpus_evaluate(const pbt_json *document, pbt_corpus_report *rep)
{
    static const char *const doc_fields[] = {"schema_version", "cases"};
    const pbt_json *cases;
    const pbt_json *ver;
    int ops_seen[N_SHAPES];
    seen sn;
    size_t i;
    size_t k;
    memset(ops_seen, 0, sizeof ops_seen);
    memset(&sn, 0, sizeof sn);

    if (document == NULL || !has_exact_fields(document, doc_fields, 2)) {
        problem(rep, NULL, "pbt-v1: the document must be exactly {schema_version, cases}");
        return 0;
    }
    ver = pbt_json_get(document, "schema_version");
    cases = pbt_json_get(document, "cases");
    if (ver->type != PBT_J_NUM || ver->slen != 1 || ver->s[0] != '1') {
        problem(rep, NULL, "pbt-v1: schema_version must be 1");
        return 0;
    }
    if (cases->type != PBT_J_ARR) {
        problem(rep, NULL, "pbt-v1: cases must be a list");
        return 0;
    }

    /* The duplicate-id check compares ids pairwise, so the case count is capped (pbt-v1 has 25). */
    if (cases->n > PBT_CORPUS_MAX_SIZE) {
        problem(rep, NULL, "pbt-v1: %lu cases exceed the cap of %d", (unsigned long)cases->n, PBT_CORPUS_MAX_SIZE);
        return 0;
    }
    for (i = 0; i < cases->n; i++) {
        const pbt_json *cs = &cases->items[i];
        const pbt_json *id = pbt_json_get(cs, "id");
        const pbt_json *op = pbt_json_get(cs, "op");
        const shape *sh = NULL;
        ctx c;
        c.rep = rep;
        c.w.text[0] = '\0';
        if (!is_str(id) || id->slen == 0) {
            problem(rep, NULL, "pbt-v1: case #%lu is missing a valid string id", (unsigned long)i);
            continue;
        }
        c.id = id->s;
        if (id_seen_before(cases, i, id)) {
            problem(rep, c.id, "duplicate case id");
        }
        if (!is_str(op)) {
            problem(rep, c.id, "the op must be a string");
            continue;
        }
        for (k = 0; k < N_SHAPES; k++) {
            if (strcmp(SHAPES[k].op, op->s) == 0 && strlen(SHAPES[k].op) == op->slen) {
                sh = &SHAPES[k];
                ops_seen[k] = 1;
                break;
            }
        }
        if (sh == NULL) {
            char frag[CLIP_FRAGMENT + 1];
            clip(frag, op->s, op->slen);
            problem(rep, c.id, "unknown op \"%s\"", frag);
            continue;
        }
        if (!shape_matches(cs, sh)) {
            problem(rep, c.id, "fields do not match the %s shape exactly", sh->op);
            continue;
        }
        rep->evaluated++;
        if (strcmp(sh->op, "gen_primitive") == 0) {
            eval_gen_primitive(&c, cs);
        } else if (strcmp(sh->op, "gen_bounded") == 0) {
            eval_gen_bounded(&c, cs);
        } else if (strcmp(sh->op, "gen_bytes") == 0) {
            eval_gen_bytes(&c, cs);
        } else if (strcmp(sh->op, "shrink_int") == 0 || strcmp(sh->op, "shrink_refinement") == 0) {
            eval_shrink_int(&c, cs, sh->op);
        } else if (strcmp(sh->op, "shrink_list") == 0) {
            eval_shrink_list(&c, cs);
        } else if (strcmp(sh->op, "shrink_bytes") == 0) {
            eval_shrink_bytes(&c, cs);
        } else if (strcmp(sh->op, "replay_consistency") == 0) {
            eval_replay(&c, cs);
        } else if (strcmp(sh->op, "filter_exhausted") == 0) {
            eval_filter_exhausted(&c, cs);
        } else if (strcmp(sh->op, "failure_id_valid") == 0) {
            eval_failure_id(&c, cs, &sn);
        } else {
            eval_corpus_reject(&c, cs);
        }
    }

    /* Coverage gates: a corpus that silently stopped exercising something is a failure. */
    if (rep->evaluated != PBT_V1_CASE_COUNT) {
        problem(rep, NULL, "pbt-v1: evaluated %lu cases, the corpus has %d", (unsigned long)rep->evaluated,
                PBT_V1_CASE_COUNT);
    }
    for (k = 0; k < N_SHAPES; k++) {
        if (!ops_seen[k]) {
            problem(rep, NULL, "pbt-v1: operation %s never exercised in the corpus", SHAPES[k].op);
        }
    }
    for (k = 0; k < 4; k++) {
        if (!sn.ids[k]) {
            problem(rep, NULL, "pbt-v1: registered failure id %s missing", FAILURE_IDS[k]);
        }
    }
    if (!sn.rejected) {
        problem(rep, NULL, "pbt-v1: failure_id_valid needs at least one unregistered-id rejection");
    }
    return 0;
}

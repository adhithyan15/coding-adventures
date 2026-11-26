/*
 * fuzz_corpus.c -- evaluator for the shared fuzz-v1 conformance corpus (VER06 section 8, VER06j section 9).
 *
 * The corpus pins what every language lane's fuzz engine must reproduce: the exact output of each mutator, the
 * corpus-admission rule, crash de-duplication, the failure-id vocabulary, the opaque TAP line and a strict-TFV
 * rejection.  This evaluator runs every case through the engine's OWN public functions (never a shortcut) and
 * reports PROBLEMS, one string each, starting with the case id.  No problems means "conforms".
 *
 * It is deliberately paranoid about its input, because the input is a file:
 *   - exactly the fields of each op, nothing missing, nothing extra;
 *   - integers are canonical decimal strings, bytes are lowercase even hex;
 *   - an unknown op / malformed value is a PROBLEM, never a skip;
 *   - every size is capped at 10,000, and the aggregate mutate_sequence work at 2^24 BYTES ACTUALLY COPIED;
 *   - every fixture fragment copied into a problem is clipped (200 / 400 bytes) and made printable;
 *   - an allocation failure is a problem too: a run that lost memory can never claim conformance.
 *
 * The engine entry points are looked up through fz_hooks so the tests can sabotage each one and watch this
 * runner notice (a conformance run that cannot fail proves nothing).
 */
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "pbt_internal.h"
#include "pbt_tfv.h"

#define CLIP_FRAGMENT 200
#define CLIP_PROBLEM 400

/* ---- the report --------------------------------------------------------------------------------------- */

/* Copy up to CLIP_FRAGMENT bytes of fixture text, showing anything unprintable as '?', and mark truncation. */
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

/* Record "<id>: <text>".  id may be NULL (a document-level problem).  nproblems counts even when storing fails. */
static void problem(pbt_corpus_report *r, const char *id, size_t idlen, const char *fmt, ...)
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
        clip(idc, id, idlen);
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

/* ---- the per-case context ---------------------------------------------------------------------------- */

typedef struct ctx {
    pbt_corpus_report *rep;
    const char *id;
    size_t idlen;
    int ok;           /* 0 once the case could not be evaluated */
    int accepted;     /* failure_id_valid: the engine accepted the id */
    uint64_t *work;   /* aggregate mutate_sequence work left */
    char why[160];
} ctx;

static void bad(ctx *c, const char *fmt, ...)
{
    char body[CLIP_PROBLEM];
    va_list ap;
    va_start(ap, fmt);
    (void)vsnprintf(body, sizeof body, fmt, ap);
    va_end(ap);
    c->ok = 0;
    problem(c->rep, c->id, c->idlen, "could not be evaluated: %s", body);
}

static void flag(ctx *c, const char *fmt, ...)
{
    char body[CLIP_PROBLEM];
    va_list ap;
    va_start(ap, fmt);
    (void)vsnprintf(body, sizeof body, fmt, ap);
    va_end(ap);
    problem(c->rep, c->id, c->idlen, "%s", body);
}

/* ---- strict decoding ----------------------------------------------------------------------------------- */

static int is_str(const pbt_json *n)
{
    return n != NULL && n->type == PBT_J_STR;
}

static int str_is(const pbt_json *n, const char *lit)
{
    return is_str(n) && n->slen == strlen(lit) && memcmp(n->s, lit, n->slen) == 0;
}

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

/* -?(0|[1-9][0-9]*), never "-0", at most PBT_BIG_MAX_DIGITS digits. */
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
static int dec_int(ctx *c, const pbt_json *n, pbt_big *out, const char *what)
{
    static const char *const names[] = {"kind", "value"};
    const pbt_json *val;
    pbt_big_zero(out);
    if (n == NULL || !has_exact_fields(n, names, 2) || !str_is(pbt_json_get(n, "kind"), "int")) {
        bad(c, "%s must be {kind: int, value}", what);
        return 1;
    }
    val = pbt_json_get(n, "value");
    if (!is_str(val) || !canonical_int(val->s, val->slen)) {
        bad(c, "%s must be a canonical decimal string within the digit cap", what);
        return 1;
    }
    if (pbt_big_parse_dec(out, val->s, val->slen) != 0) {
        bad(c, "%s is too wide for this lane", what);
        return 1;
    }
    return 0;
}

/* A size (steps...) in [0, FZ_CORPUS_MAX_SIZE]. */
static int dec_size(ctx *c, const pbt_json *n, size_t *out, const char *what)
{
    pbt_big b;
    *out = 0;
    if (dec_int(c, n, &b, what)) {
        return 1;
    }
    if (pbt_big_sign(&b) < 0 || !pbt_big_fits_u64(&b) || pbt_big_low_u64(&b) > FZ_CORPUS_MAX_SIZE) {
        bad(c, "%s is outside [0, %d]", what, FZ_CORPUS_MAX_SIZE);
        return 1;
    }
    *out = (size_t)pbt_big_low_u64(&b);
    return 0;
}

/* A width: anything outside [0, 64] becomes -1 ("not one of 1, 2, 4, 8": input unchanged). */
static int dec_width(ctx *c, const pbt_json *n, int *out)
{
    pbt_big b;
    if (dec_int(c, n, &b, "width")) {
        return 1;
    }
    *out = (pbt_big_fits_i64(&b) && pbt_big_to_i64(&b) >= 0 && pbt_big_to_i64(&b) <= 64) ? (int)pbt_big_to_i64(&b) : -1;
    return 0;
}

static int hexval(char ch)
{
    if (ch >= '0' && ch <= '9') {
        return ch - '0';
    }
    if (ch >= 'a' && ch <= 'f') {
        return ch - 'a' + 10;
    }
    return -1;
}

/* {"kind":"bytes","hex":"<even-length lowercase hex>"} into an owned buffer (free with fz_bytes_free). */
static int dec_bytes(ctx *c, const pbt_json *n, fz_bytes *out, const char *what)
{
    static const char *const names[] = {"kind", "hex"};
    const pbt_json *hex;
    size_t i;
    out->p = NULL;
    out->n = 0;
    if (n == NULL || !has_exact_fields(n, names, 2) || !str_is(pbt_json_get(n, "kind"), "bytes") ||
        !is_str(pbt_json_get(n, "hex"))) {
        bad(c, "%s must be {kind: bytes, hex}", what);
        return 1;
    }
    hex = pbt_json_get(n, "hex");
    if (hex->slen % 2 != 0 || hex->slen / 2 > FZ_CORPUS_MAX_SIZE) {
        bad(c, "%s hex must be even length and within the cap", what);
        return 1;
    }
    out->p = (uint8_t *)pbt_alloc(hex->slen / 2, 1);
    if (out->p == NULL) {
        bad(c, "out of memory");
        return 1;
    }
    out->n = hex->slen / 2;
    for (i = 0; i < hex->slen; i += 2) {
        int h = hexval(hex->s[i]);
        int l = hexval(hex->s[i + 1]);
        if (h < 0 || l < 0) {
            fz_bytes_free(out);
            bad(c, "%s hex must be lowercase", what);
            return 1;
        }
        out->p[i / 2] = (uint8_t)(h * 16 + l);
    }
    return 0;
}

static int dec_str(ctx *c, const pbt_json *n, fz_str *out, const char *what)
{
    out->p = "";
    out->n = 0;
    if (!is_str(n)) {
        bad(c, "%s must be a string", what);
        return 1;
    }
    out->p = n->s;
    out->n = n->slen;
    return 0;
}

static int dec_bool(ctx *c, const pbt_json *n, int *out, const char *what)
{
    *out = 0;
    if (n == NULL || n->type != PBT_J_BOOL) {
        bad(c, "%s must be a bool", what);
        return 1;
    }
    *out = n->b != 0;
    return 0;
}

static int dec_endian(ctx *c, const pbt_json *n, fz_endian *out)
{
    *out = FZ_LITTLE;
    if (str_is(n, "little")) {
        return 0;
    }
    if (str_is(n, "big")) {
        *out = FZ_BIG;
        return 0;
    }
    bad(c, "endian must be 'little' or 'big'");
    return 1;
}

/* A list of strings into an owned fz_str array (the texts stay owned by the JSON tree). */
static int dec_strs(ctx *c, const pbt_json *n, fz_str **out, size_t *count, const char *what)
{
    size_t i;
    fz_str *v;
    *out = NULL;
    *count = 0;
    if (n == NULL || n->type != PBT_J_ARR || n->n > FZ_CORPUS_MAX_SIZE) {
        bad(c, "%s must be a list of strings within the cap", what);
        return 1;
    }
    v = (fz_str *)pbt_alloc(n->n, sizeof *v);
    if (v == NULL) {
        bad(c, "out of memory");
        return 1;
    }
    for (i = 0; i < n->n; i++) {
        if (!is_str(&n->items[i])) {
            pbt_free(v);
            bad(c, "%s must be a list of strings", what);
            return 1;
        }
        v[i].p = n->items[i].s;
        v[i].n = n->items[i].slen;
    }
    *out = v;
    *count = n->n;
    return 0;
}

/* A list of edge ids, each in [0, 2^64). */
static int dec_edges(ctx *c, const pbt_json *n, uint64_t **out, size_t *count, const char *what)
{
    size_t i;
    uint64_t *v;
    *out = NULL;
    *count = 0;
    if (n == NULL || n->type != PBT_J_ARR || n->n > FZ_CORPUS_MAX_SIZE) {
        bad(c, "%s must be a list of ints within the cap", what);
        return 1;
    }
    v = (uint64_t *)pbt_alloc(n->n, sizeof *v);
    if (v == NULL) {
        bad(c, "out of memory");
        return 1;
    }
    for (i = 0; i < n->n; i++) {
        pbt_big b;
        if (dec_int(c, &n->items[i], &b, what)) {
            pbt_free(v);
            return 1;
        }
        if (pbt_big_sign(&b) < 0 || !pbt_big_fits_u64(&b)) {
            pbt_free(v);
            bad(c, "%s element is outside [0, 2^64)", what);
            return 1;
        }
        v[i] = pbt_big_low_u64(&b);
    }
    *out = v;
    *count = n->n;
    return 0;
}

/* ---- comparison ---------------------------------------------------------------------------------------- */

static void hex_clip(char *dst, const uint8_t *p, size_t n)
{
    static const char digits[] = "0123456789abcdef";
    size_t m = n < CLIP_FRAGMENT / 2 ? n : CLIP_FRAGMENT / 2;
    size_t i;
    for (i = 0; i < m; i++) {
        dst[2 * i] = digits[p[i] >> 4];
        dst[2 * i + 1] = digits[p[i] & 15];
    }
    dst[2 * m] = '\0';
    if (n > m) {
        dst[2 * m - 1] = '.';
        dst[2 * m - 2] = '.';
    }
}

static void compare_output(ctx *c, const char *op, const fz_bytes *got, const pbt_json *want_node)
{
    fz_bytes want;
    char gh[CLIP_FRAGMENT + 1];
    char wh[CLIP_FRAGMENT + 1];
    int saved = c->ok;
    if (dec_bytes(c, want_node, &want, "output")) {
        return;
    }
    c->ok = saved;
    if (want.n != got->n || (want.n > 0 && memcmp(want.p, got->p, want.n) != 0)) {
        hex_clip(wh, want.p, want.n);
        hex_clip(gh, got->p, got->n);
        flag(c, "%s expected %s, got %s", op, wh, gh);
    }
    fz_bytes_free(&want);
}

/* ---- per-op evaluation ------------------------------------------------------------------------------------ */

/* Clamp a bignum split into [0, n]. */
static size_t clamp_big(const pbt_big *v, size_t n)
{
    if (pbt_big_sign(v) <= 0) {
        return 0;
    }
    if (!pbt_big_fits_u64(v) || pbt_big_low_u64(v) > (uint64_t)n) {
        return n;
    }
    return (size_t)pbt_big_low_u64(v);
}

static void run_status(ctx *c, int rc)
{
    if (rc == FZ_E_NOMEM) {
        bad(c, "out of memory");
    } else if (rc == FZ_E_WORK) {
        *c->work = 0;
        bad(c, "mutate_sequence work budget of %lu bytes exhausted", (unsigned long)FZ_CORPUS_MAX_SEQUENCE_WORK);
    } else if (rc != FZ_OK) {
        bad(c, "the engine refused the case (status %d)", rc);
    }
}

#define F(name) pbt_json_get(cs, name)

static void eval_mutator(ctx *c, const char *op, const pbt_json *cs)
{
    fz_bytes in = {NULL, 0};
    fz_bytes got = {NULL, 0};
    int rc = FZ_OK;
    int check_len = 0;
    pbt_big a;
    pbt_big b;
    pbt_big d;

    if (strcmp(op, "mutate_splice") == 0) {
        fz_bytes tgt = {NULL, 0};
        fz_bytes donor = {NULL, 0};
        if (!dec_bytes(c, F("target"), &tgt, "target") && !dec_bytes(c, F("donor"), &donor, "donor") &&
            !dec_int(c, F("target_split"), &a, "target_split") && !dec_int(c, F("donor_split"), &b, "donor_split")) {
            rc = (fz_hooks.splice != NULL ? fz_hooks.splice : fz_splice_big)(&got, tgt.p, tgt.n, donor.p, donor.n, &a, &b);
            if (rc == FZ_OK) {
                size_t want_len = clamp_big(&a, tgt.n) + donor.n - clamp_big(&b, donor.n);
                compare_output(c, op, &got, F("output"));
                if (got.n != want_len) {
                    flag(c, "splice length is not prefix + suffix");
                }
            } else {
                run_status(c, rc);
            }
        }
        fz_bytes_free(&tgt);
        fz_bytes_free(&donor);
        fz_bytes_free(&got);
        return;
    }
    if (dec_bytes(c, F("input"), &in, "input")) {
        return;
    }
    if (strcmp(op, "mutate_bit_flip") == 0 || strcmp(op, "mutate_byte_flip") == 0) {
        int bit = strcmp(op, "mutate_bit_flip") == 0;
        if (dec_int(c, F(bit ? "bit_offset" : "byte_offset"), &a, "offset") || dec_int(c, F("count"), &b, "count")) {
            goto done;
        }
        if (bit) {
            rc = (fz_hooks.bit_flip != NULL ? fz_hooks.bit_flip : fz_bit_flip_big)(&got, in.p, in.n, &a, &b);
        } else {
            rc = (fz_hooks.byte_flip != NULL ? fz_hooks.byte_flip : fz_byte_flip_big)(&got, in.p, in.n, &a, &b);
        }
        check_len = 1;
    } else if (strcmp(op, "mutate_arithmetic") == 0) {
        int w;
        fz_endian e;
        if (dec_int(c, F("byte_offset"), &a, "byte_offset") || dec_int(c, F("delta"), &b, "delta") ||
            dec_width(c, F("width"), &w) || dec_endian(c, F("endian"), &e)) {
            goto done;
        }
        rc = (fz_hooks.byte_arithmetic != NULL ? fz_hooks.byte_arithmetic : fz_byte_arithmetic_big)(&got, in.p, in.n, &a,
                                                                                                   &b, w, e);
        check_len = 1;
    } else if (strcmp(op, "mutate_boundary") == 0) {
        int w;
        fz_endian e;
        if (dec_int(c, F("byte_offset"), &a, "byte_offset") || dec_width(c, F("width"), &w) ||
            dec_int(c, F("val"), &b, "val") || dec_endian(c, F("endian"), &e)) {
            goto done;
        }
        rc = (fz_hooks.boundary_insert != NULL ? fz_hooks.boundary_insert : fz_boundary_insert_big)(&got, in.p, in.n, &a,
                                                                                                  w, &b, e);
        check_len = 1;
    } else if (strcmp(op, "mutate_block_delete") == 0) {
        if (dec_int(c, F("offset"), &a, "offset") || dec_int(c, F("length"), &b, "length")) {
            goto done;
        }
        rc = (fz_hooks.block_delete != NULL ? fz_hooks.block_delete : fz_block_delete_big)(&got, in.p, in.n, &a, &b);
    } else if (strcmp(op, "mutate_block_insert") == 0) {
        fz_bytes ins = {NULL, 0};
        if (dec_int(c, F("offset"), &a, "offset") || dec_bytes(c, F("insert_bytes"), &ins, "insert_bytes")) {
            goto done;
        }
        rc = (fz_hooks.block_insert != NULL ? fz_hooks.block_insert : fz_block_insert_big)(&got, in.p, in.n, &a, ins.p,
                                                                                         ins.n);
        fz_bytes_free(&ins);
    } else if (strcmp(op, "mutate_block_duplicate") == 0) {
        if (dec_int(c, F("src_offset"), &a, "src_offset") || dec_int(c, F("length"), &b, "length") ||
            dec_int(c, F("dst_offset"), &d, "dst_offset")) {
            goto done;
        }
        rc = (fz_hooks.block_duplicate != NULL ? fz_hooks.block_duplicate : fz_block_duplicate_big)(&got, in.p, in.n,
                                                                                                  &a, &b, &d);
    } else { /* mutate_sequence */
        size_t steps;
        uint64_t done_work = 0;
        if (dec_int(c, F("seed"), &a, "seed") || dec_size(c, F("steps"), &steps, "steps")) {
            goto done;
        }
        if (*c->work == 0) {
            bad(c, "mutate_sequence work budget of %lu bytes exhausted", (unsigned long)FZ_CORPUS_MAX_SEQUENCE_WORK);
            goto done;
        }
        rc = (fz_hooks.mutate_sequence != NULL ? fz_hooks.mutate_sequence : fz_mutate_sequence_budget)(
            &got, in.p, in.n, pbt_big_mod_u64(&a), steps, FZ_MAX_INPUT_BYTES, *c->work, &done_work);
        /* charge the work really done, INCLUDING a call that hit its cap (then the whole budget is gone) */
        *c->work = done_work >= *c->work ? 0 : *c->work - done_work;
    }
    if (rc != FZ_OK) {
        run_status(c, rc);
        goto done;
    }
    compare_output(c, op, &got, F("output"));
    if (check_len && got.n != in.n) {
        flag(c, "%s changed the length of its input", op);
    }
done:
    fz_bytes_free(&in);
    fz_bytes_free(&got);
}

/* Independent set-difference check: is any trace edge absent from existing? */
static int cmp_u64(const void *x, const void *y)
{
    uint64_t a = *(const uint64_t *)x;
    uint64_t b = *(const uint64_t *)y;
    return a < b ? -1 : (a > b ? 1 : 0);
}

static int has_novel(const uint64_t *existing, size_t ne, const uint64_t *trace, size_t nt, int *novel)
{
    uint64_t *have = (uint64_t *)pbt_alloc(ne, sizeof(uint64_t));
    size_t i;
    *novel = 0;
    if (have == NULL) {
        return 1;
    }
    if (ne > 0) {
        memcpy(have, existing, ne * sizeof(uint64_t));
        qsort(have, ne, sizeof(uint64_t), cmp_u64);
    }
    for (i = 0; i < nt && !*novel; i++) {
        *novel = ne == 0 || bsearch(&trace[i], have, ne, sizeof(uint64_t), cmp_u64) == NULL;
    }
    pbt_free(have);
    return 0;
}

static void eval_corpus_admit(ctx *c, const pbt_json *cs)
{
    uint64_t *existing = NULL;
    uint64_t *trace = NULL;
    size_t ne = 0;
    size_t nt = 0;
    int admitted;
    pbt_big new_edges;
    int got_admitted = 0;
    size_t got_new = 0;
    int rc;
    int novel;
    if (dec_edges(c, F("existing_edges"), &existing, &ne, "existing_edges") ||
        dec_edges(c, F("trace_edges"), &trace, &nt, "trace_edges") || dec_bool(c, F("admitted"), &admitted, "admitted") ||
        dec_int(c, F("new_edges"), &new_edges, "new_edges")) {
        goto done;
    }
    rc = (fz_hooks.corpus_admit != NULL ? fz_hooks.corpus_admit : fz_corpus_admit)(existing, ne, trace, nt, &got_admitted,
                                                                                  &got_new);
    if (rc != FZ_OK) {
        run_status(c, rc);
        goto done;
    }
    if ((got_admitted != 0) != admitted) {
        flag(c, "corpus_admit admitted %d, corpus %d", got_admitted != 0, admitted);
    }
    {
        pbt_big g;
        pbt_big_set_u64(&g, (uint64_t)got_new);
        if (pbt_big_cmp(&g, &new_edges) != 0) {
            flag(c, "corpus_admit new_edges %lu, corpus differs", (unsigned long)got_new);
        }
    }
    if (has_novel(existing, ne, trace, nt, &novel)) {
        bad(c, "out of memory");
        goto done;
    }
    if ((got_admitted != 0) != (novel != 0)) {
        flag(c, "corpus_admit disagrees with set difference");
    }
done:
    pbt_free(existing);
    pbt_free(trace);
}

static void eval_crash_dedup(ctx *c, const pbt_json *cs)
{
    fz_str ka;
    fz_str kb;
    fz_str *fa = NULL;
    fz_str *fb = NULL;
    size_t na = 0;
    size_t nb = 0;
    int want;
    if (dec_str(c, F("crash_a_kind"), &ka, "crash_a_kind") ||
        dec_strs(c, F("crash_a_frames"), &fa, &na, "crash_a_frames") ||
        dec_str(c, F("crash_b_kind"), &kb, "crash_b_kind") ||
        dec_strs(c, F("crash_b_frames"), &fb, &nb, "crash_b_frames") || dec_bool(c, F("is_duplicate"), &want, "is_duplicate")) {
        goto done;
    }
    if (((fz_hooks.is_duplicate != NULL ? fz_hooks.is_duplicate : fz_is_duplicate_crash)(ka, fa, na, kb, fb, nb) != 0) !=
        (want != 0)) {
        flag(c, "crash_dedup is_duplicate disagrees with the corpus (%d)", want);
    }
done:
    pbt_free(fa);
    pbt_free(fb);
}

static void eval_failure_id(ctx *c, const pbt_json *cs)
{
    fz_str in;
    int valid;
    int got;
    if (dec_str(c, F("input"), &in, "input") || dec_bool(c, F("valid"), &valid, "valid")) {
        return;
    }
    got = (fz_hooks.is_failure_id != NULL ? fz_hooks.is_failure_id : fz_is_failure_id)(in.p, in.n) != 0;
    c->accepted = got;
    if (got != valid) {
        char frag[CLIP_FRAGMENT + 1];
        clip(frag, in.p, in.n);
        flag(c, "failure id membership disagrees for \"%s\"", frag);
    }
}

static void eval_private_reporting(ctx *c, const pbt_json *cs)
{
    fz_bytes raw = {NULL, 0};
    fz_str target;
    fz_str fid;
    fz_str sig;
    fz_str tap;
    int want_leak;
    char line[1024];
    int rc;
    if (dec_str(c, F("target"), &target, "target") || dec_str(c, F("failure_id"), &fid, "failure_id") ||
        dec_bytes(c, F("raw_payload"), &raw, "raw_payload") || dec_str(c, F("sig_hash"), &sig, "sig_hash") ||
        dec_str(c, F("tap_line"), &tap, "tap_line") ||
        dec_bool(c, F("contains_raw_payload"), &want_leak, "contains_raw_payload")) {
        goto done;
    }
    rc = (fz_hooks.format_tap != NULL ? fz_hooks.format_tap : fz_format_tap_failure)(line, sizeof line, 1, target.p,
                                                                                    target.n, fid.p, fid.n, sig.p, sig.n);
    if (rc != FZ_OK) {
        bad(c, "format_tap_failure refused (status %d)", rc);
        goto done;
    }
    if (strlen(line) != tap.n || memcmp(line, tap.p, tap.n) != 0) {
        char gf[CLIP_FRAGMENT + 1];
        char wf[CLIP_FRAGMENT + 1];
        clip(wf, tap.p, tap.n);
        clip(gf, line, strlen(line));
        flag(c, "tap_line expected %s, got %s", wf, gf);
    }
    {
        int leaked = 0;
        if (raw.n > 0) {
            char *hx = (char *)pbt_alloc(raw.n * 2 + 1, 1);
            size_t i;
            if (hx == NULL) {
                bad(c, "out of memory");
                goto done;
            }
            for (i = 0; i < raw.n; i++) {
                (void)snprintf(hx + 2 * i, 3, "%02x", (unsigned)raw.p[i]);
            }
            leaked = strstr(line, hx) != NULL;
            pbt_free(hx);
        }
        if ((leaked != 0) != (want_leak != 0)) {
            flag(c, "privacy verdict wrong, payload leaked: %d", leaked);
        }
    }
    if (strpbrk(line, "\r\n") != NULL) {
        flag(c, "failure line spans more than one line");
    }
done:
    fz_bytes_free(&raw);
}

static void eval_corpus_reject(ctx *c, const pbt_json *cs)
{
    fz_str text;
    char err[200];
    if (dec_str(c, F("text"), &text, "text")) {
        return;
    }
    if ((fz_hooks.tfv_validate != NULL ? fz_hooks.tfv_validate : pbt_tfv_validate)(text.p, text.n, err, sizeof err) == 0) {
        flag(c, "corpus_reject text was accepted by the strict parser");
    }
}

/* ---- the document ---------------------------------------------------------------------------------------- */

typedef struct shape {
    const char *op;
    const char *fields[7];
    size_t n;
} shape;

static const shape SHAPES[] = {
    {"mutate_bit_flip", {"input", "bit_offset", "count", "output"}, 4},
    {"mutate_byte_flip", {"input", "byte_offset", "count", "output"}, 4},
    {"mutate_arithmetic", {"input", "byte_offset", "delta", "width", "endian", "output"}, 6},
    {"mutate_boundary", {"input", "byte_offset", "width", "val", "endian", "output"}, 6},
    {"mutate_block_delete", {"input", "offset", "length", "output"}, 4},
    {"mutate_block_insert", {"input", "offset", "insert_bytes", "output"}, 4},
    {"mutate_block_duplicate", {"input", "src_offset", "length", "dst_offset", "output"}, 5},
    {"mutate_splice", {"target", "donor", "target_split", "donor_split", "output"}, 5},
    {"mutate_sequence", {"input", "seed", "steps", "output"}, 4},
    {"corpus_admit", {"existing_edges", "trace_edges", "admitted", "new_edges"}, 4},
    {"crash_dedup", {"crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate"}, 5},
    {"failure_id_valid", {"input", "valid"}, 2},
    {"private_reporting", {"target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload"}, 6},
    {"corpus_reject", {"text"}, 1},
};
#define N_SHAPES (sizeof SHAPES / sizeof SHAPES[0])

static const char *const FAILURE_IDS[5] = {FZ_FID_CRASH, FZ_FID_TIMEOUT, FZ_FID_OOM, FZ_FID_LEAK, FZ_FID_PANIC};

static int fields_match(const pbt_json *cs, const shape *sh)
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

int fz_corpus_evaluate(const pbt_json *document, pbt_corpus_report *rep)
{
    static const char *const doc_fields[] = {"schema_version", "cases"};
    const pbt_json *cases;
    const pbt_json *ver;
    int ops_seen[N_SHAPES];
    int accepted_seen = 0;
    int rejected_seen = 0;
    uint64_t work = FZ_CORPUS_MAX_SEQUENCE_WORK;
    size_t i;
    size_t k;
    memset(ops_seen, 0, sizeof ops_seen);

    if (document == NULL || !has_exact_fields(document, doc_fields, 2)) {
        problem(rep, NULL, 0, "fuzz-v1: the document must be exactly {schema_version, cases}");
        return 0;
    }
    ver = pbt_json_get(document, "schema_version");
    cases = pbt_json_get(document, "cases");
    if (ver->type != PBT_J_NUM || ver->slen != 1 || ver->s[0] != '1') {
        problem(rep, NULL, 0, "fuzz-v1: schema_version must be 1");
        return 0;
    }
    if (cases->type != PBT_J_ARR) {
        problem(rep, NULL, 0, "fuzz-v1: cases must be a list");
        return 0;
    }
    /* The duplicate-id check compares ids pairwise, so the case count is capped. */
    if (cases->n > FZ_CORPUS_MAX_SIZE) {
        problem(rep, NULL, 0, "fuzz-v1: %lu cases exceed the cap of %d", (unsigned long)cases->n, FZ_CORPUS_MAX_SIZE);
        return 0;
    }
    for (i = 0; i < cases->n; i++) {
        const pbt_json *cs = &cases->items[i];
        const pbt_json *id = pbt_json_get(cs, "id");
        const pbt_json *op = pbt_json_get(cs, "op");
        const shape *sh = NULL;
        size_t shape_index = 0;
        ctx c;
        memset(&c, 0, sizeof c);
        c.rep = rep;
        c.ok = 1;
        c.work = &work;
        if (!is_str(id) || id->slen == 0) {
            problem(rep, NULL, 0, "fuzz-v1: case #%lu is missing a valid string id", (unsigned long)i);
            continue;
        }
        c.id = id->s;
        c.idlen = id->slen;
        if (id_seen_before(cases, i, id)) {
            problem(rep, c.id, c.idlen, "duplicate case id");
        }
        if (!is_str(op)) {
            problem(rep, c.id, c.idlen, "the op must be a string");
            continue;
        }
        for (k = 0; k < N_SHAPES; k++) {
            if (strlen(SHAPES[k].op) == op->slen && memcmp(SHAPES[k].op, op->s, op->slen) == 0) {
                sh = &SHAPES[k];
                shape_index = k;
                break;
            }
        }
        if (sh == NULL) {
            char frag[CLIP_FRAGMENT + 1];
            clip(frag, op->s, op->slen);
            problem(rep, c.id, c.idlen, "unknown op \"%s\"", frag);
            continue;
        }
        if (!fields_match(cs, sh)) {
            problem(rep, c.id, c.idlen, "fields do not match the %s shape exactly", sh->op);
            continue;
        }
        if (strcmp(sh->op, "corpus_admit") == 0) {
            eval_corpus_admit(&c, cs);
        } else if (strcmp(sh->op, "crash_dedup") == 0) {
            eval_crash_dedup(&c, cs);
        } else if (strcmp(sh->op, "failure_id_valid") == 0) {
            eval_failure_id(&c, cs);
        } else if (strcmp(sh->op, "private_reporting") == 0) {
            eval_private_reporting(&c, cs);
        } else if (strcmp(sh->op, "corpus_reject") == 0) {
            eval_corpus_reject(&c, cs);
        } else {
            eval_mutator(&c, sh->op, cs);
        }
        if (c.ok) {
            rep->evaluated++;
            ops_seen[shape_index] = 1;
            if (strcmp(sh->op, "failure_id_valid") == 0) {
                accepted_seen = accepted_seen || c.accepted;
                rejected_seen = rejected_seen || !c.accepted;
            }
        }
    }

    /* Coverage gates: a corpus that silently stopped exercising something is a failure. */
    if (cases->n != FZ_V1_CASE_COUNT) {
        problem(rep, NULL, 0, "fuzz-v1: expected %d cases, got %lu", FZ_V1_CASE_COUNT, (unsigned long)cases->n);
    }
    if (rep->evaluated != FZ_V1_CASE_COUNT) {
        problem(rep, NULL, 0, "fuzz-v1: evaluated %lu cases, the corpus has %d", (unsigned long)rep->evaluated,
                FZ_V1_CASE_COUNT);
    }
    for (k = 0; k < N_SHAPES; k++) {
        if (!ops_seen[k]) {
            problem(rep, NULL, 0, "fuzz-v1: operation %s never exercised in the corpus", SHAPES[k].op);
        }
    }
    if (!accepted_seen) {
        problem(rep, NULL, 0, "fuzz-v1: failure_id_valid needs a registered-id acceptance");
    }
    if (!rejected_seen) {
        problem(rep, NULL, 0, "fuzz-v1: failure_id_valid needs an unregistered-id rejection");
    }
    for (k = 0; k < 5; k++) {
        if (!(fz_hooks.is_failure_id != NULL ? fz_hooks.is_failure_id : fz_is_failure_id)(FAILURE_IDS[k],
                                                                                          strlen(FAILURE_IDS[k]))) {
            problem(rep, NULL, 0, "fuzz-v1: the engine rejects the registered id %s", FAILURE_IDS[k]);
        }
    }
    return 0;
}

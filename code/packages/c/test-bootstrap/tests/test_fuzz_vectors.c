/*
 * test_fuzz_vectors.c -- the 820 differential vectors of the Go lane (VER06j section 11).
 *
 * code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go is a generated table: seeded random inputs run
 * through the independent Python oracle (scripts/fuzz_conformance.py).  The fuzz-v1 corpus alone cannot pin these
 * (its splice has equal splits, its mutate_sequence does not pin every constant), so this test reads the Go
 * file straight from its committed path (there is no copy) and checks EVERY row through BOTH the int64 and the
 * pbt_big entry points.
 *
 * A row looks like:   {"splice", []string{"efdd...", "0162...", "8", "39"}, "efdd..."},
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

#define MAXARGS 6

typedef struct row {
    char op[24];
    char args[MAXARGS][96];
    size_t nargs;
    char out[400];
} row;

/* Copy the quoted string starting at *pp (which points at the opening quote); advance past the closing quote. */
static int take_quoted(const char **pp, char *dst, size_t cap)
{
    const char *p = *pp;
    size_t n = 0;
    if (*p != '"') {
        return 1;
    }
    p++;
    while (*p != '"' && *p != '\0') {
        if (n + 1 >= cap) {
            return 1;
        }
        dst[n++] = *p++;
    }
    if (*p != '"') {
        return 1;
    }
    dst[n] = '\0';
    *pp = p + 1;
    return 0;
}

static int parse_row(const char *line, row *r)
{
    const char *p = line;
    memset(r, 0, sizeof *r);
    if (strncmp(p, "\t{", 2) != 0) {
        return 1;
    }
    p += 2;
    if (take_quoted(&p, r->op, sizeof r->op) || strncmp(p, ", []string{", 11) != 0) {
        return 1;
    }
    p += 11;
    while (*p == '"') {
        if (r->nargs >= MAXARGS || take_quoted(&p, r->args[r->nargs], sizeof r->args[0])) {
            return 1;
        }
        r->nargs++;
        if (*p == ',') {
            p += 2; /* ", " */
        }
    }
    if (strncmp(p, "}, ", 3) != 0) {
        return 1;
    }
    p += 3;
    if (take_quoted(&p, r->out, sizeof r->out) || strcmp(p, "},") != 0) {
        return 1;
    }
    return 0;
}

static int hexval(char c)
{
    return c >= '0' && c <= '9' ? c - '0' : (c >= 'a' && c <= 'f' ? c - 'a' + 10 : -1);
}

static fz_bytes unhex(const char *h)
{
    fz_bytes b;
    size_t n = strlen(h) / 2;
    size_t i;
    b.p = (uint8_t *)pbt_alloc(n, 1);
    b.n = n;
    for (i = 0; i < n; i++) {
        b.p[i] = (uint8_t)(hexval(h[2 * i]) * 16 + hexval(h[2 * i + 1]));
    }
    return b;
}

static int same(const fz_bytes *got, const fz_bytes *want)
{
    return got->n == want->n && (want->n == 0 || memcmp(got->p, want->p, want->n) == 0);
}

static pbt_big big(const char *dec)
{
    pbt_big b;
    ISO_CHECK(pbt_big_parse_dec(&b, dec, strlen(dec)) == 0);
    return b;
}

static int fits64(const char *dec)
{
    pbt_big b = big(dec);
    return pbt_big_fits_i64(&b);
}

static int64_t i64(const char *dec)
{
    pbt_big b = big(dec);
    return pbt_big_to_i64(&b);
}

static fz_endian endian_of(const char *s)
{
    return strcmp(s, "big") == 0 ? FZ_BIG : FZ_LITTLE;
}

static void check_pair(size_t index, const row *r, int rc_big, fz_bytes *got_big, int rc_small, fz_bytes *got_small,
                       int have_small, const fz_bytes *want)
{
    if (rc_big != FZ_OK || !same(got_big, want)) {
        printf("  vector %lu (%s) failed on the pbt_big path\n", (unsigned long)index, r->op);
    }
    ISO_CHECK(rc_big == FZ_OK);
    ISO_CHECK(same(got_big, want));
    if (have_small) {
        if (rc_small != FZ_OK || !same(got_small, want)) {
            printf("  vector %lu (%s) failed on the int64 path\n", (unsigned long)index, r->op);
        }
        ISO_CHECK(rc_small == FZ_OK);
        ISO_CHECK(same(got_small, want));
    }
    fz_bytes_free(got_big);
    fz_bytes_free(got_small);
}

static void run_row(size_t index, const row *r)
{
    fz_bytes in = {NULL, 0};
    fz_bytes want = unhex(r->out);
    fz_bytes gb = {NULL, 0};
    fz_bytes gs = {NULL, 0};
    int rb = FZ_OK;
    int rs = FZ_OK;
    int have_small = 0;
    const char *op = r->op;
    if (strcmp(op, "splice") != 0) {
        in = unhex(r->args[0]);
    }
    if (strcmp(op, "bit_flip") == 0 || strcmp(op, "byte_flip") == 0) {
        int bit = strcmp(op, "bit_flip") == 0;
        pbt_big o = big(r->args[1]);
        pbt_big c = big(r->args[2]);
        rb = (bit ? fz_bit_flip_big : fz_byte_flip_big)(&gb, in.p, in.n, &o, &c);
        have_small = fits64(r->args[1]) && fits64(r->args[2]);
        if (have_small) {
            rs = (bit ? fz_bit_flip : fz_byte_flip)(&gs, in.p, in.n, i64(r->args[1]), i64(r->args[2]));
        }
    } else if (strcmp(op, "arithmetic") == 0) {
        pbt_big o = big(r->args[1]);
        pbt_big d = big(r->args[2]);
        int w = atoi(r->args[3]);
        fz_endian e = endian_of(r->args[4]);
        rb = fz_byte_arithmetic_big(&gb, in.p, in.n, &o, &d, w, e);
        have_small = fits64(r->args[1]) && fits64(r->args[2]);
        if (have_small) {
            rs = fz_byte_arithmetic(&gs, in.p, in.n, i64(r->args[1]), i64(r->args[2]), w, e);
        }
    } else if (strcmp(op, "boundary") == 0) {
        pbt_big o = big(r->args[1]);
        pbt_big v = big(r->args[3]);
        int w = atoi(r->args[2]);
        fz_endian e = endian_of(r->args[4]);
        rb = fz_boundary_insert_big(&gb, in.p, in.n, &o, w, &v, e);
        /* the int64 form takes an unsigned value: use it when the value is a non-negative 64-bit number */
        have_small = fits64(r->args[1]) && pbt_big_sign(&v) >= 0 && pbt_big_fits_u64(&v);
        if (have_small) {
            rs = fz_boundary_insert(&gs, in.p, in.n, i64(r->args[1]), w, pbt_big_low_u64(&v), e);
        }
    } else if (strcmp(op, "block_delete") == 0) {
        pbt_big o = big(r->args[1]);
        pbt_big l = big(r->args[2]);
        rb = fz_block_delete_big(&gb, in.p, in.n, &o, &l);
        have_small = fits64(r->args[1]) && fits64(r->args[2]);
        if (have_small) {
            rs = fz_block_delete(&gs, in.p, in.n, i64(r->args[1]), i64(r->args[2]));
        }
    } else if (strcmp(op, "block_insert") == 0) {
        pbt_big o = big(r->args[1]);
        fz_bytes ins = unhex(r->args[2]);
        rb = fz_block_insert_big(&gb, in.p, in.n, &o, ins.p, ins.n);
        have_small = fits64(r->args[1]);
        if (have_small) {
            rs = fz_block_insert(&gs, in.p, in.n, i64(r->args[1]), ins.p, ins.n);
        }
        fz_bytes_free(&ins);
    } else if (strcmp(op, "block_duplicate") == 0) {
        pbt_big a = big(r->args[1]);
        pbt_big l = big(r->args[2]);
        pbt_big d = big(r->args[3]);
        rb = fz_block_duplicate_big(&gb, in.p, in.n, &a, &l, &d);
        have_small = fits64(r->args[1]) && fits64(r->args[2]) && fits64(r->args[3]);
        if (have_small) {
            rs = fz_block_duplicate(&gs, in.p, in.n, i64(r->args[1]), i64(r->args[2]), i64(r->args[3]));
        }
    } else if (strcmp(op, "splice") == 0) {
        fz_bytes t = unhex(r->args[0]);
        fz_bytes d = unhex(r->args[1]);
        pbt_big a = big(r->args[2]);
        pbt_big b = big(r->args[3]);
        rb = fz_splice_big(&gb, t.p, t.n, d.p, d.n, &a, &b);
        have_small = fits64(r->args[2]) && fits64(r->args[3]);
        if (have_small) {
            rs = fz_splice(&gs, t.p, t.n, d.p, d.n, i64(r->args[2]), i64(r->args[3]));
        }
        fz_bytes_free(&t);
        fz_bytes_free(&d);
    } else if (strcmp(op, "sequence") == 0) {
        pbt_big seed = big(r->args[1]);
        rb = fz_mutate_sequence(&gb, in.p, in.n, pbt_big_mod_u64(&seed), (size_t)atoi(r->args[2]),
                                (size_t)atol(r->args[3]));
    } else {
        printf("  unknown vector op %s\n", op);
        ISO_CHECK(0);
        fz_bytes_free(&in);
        fz_bytes_free(&want);
        return;
    }
    check_pair(index, r, rb, &gb, rs, &gs, have_small, &want);
    fz_bytes_free(&in);
    fz_bytes_free(&want);
}

int main(void)
{
    size_t len = 0;
    char *text = tu_read_repo_file("code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go", &len);
    const char *p;
    size_t count = 0;
    size_t counts[9] = {0};
    static const char *const OPS[9] = {"bit_flip",      "byte_flip", "arithmetic", "boundary", "block_delete",
                                       "block_insert", "block_duplicate", "splice", "sequence"};
    int in_table = 0;
    size_t k;
    ISO_CHECK_MSG(text != NULL, "the Go reference vector table must be found");
    if (text == NULL) {
        return ISO_TEST_RESULT();
    }
    for (p = text; *p != '\0';) {
        char line[1024];
        size_t n = 0;
        while (p[n] != '\0' && p[n] != '\n') {
            n++;
        }
        if (n >= sizeof line) {
            printf("  a line of the table is unexpectedly long\n");
            ISO_CHECK(0);
            break;
        }
        memcpy(line, p, n);
        line[n] = '\0';
        p += n + (p[n] == '\n' ? 1 : 0);
        if (!in_table) {
            in_table = strcmp(line, "var referenceVectors = []refVector{") == 0;
            continue;
        }
        if (strcmp(line, "}") == 0) {
            break;
        }
        {
            row r;
            if (parse_row(line, &r) != 0) {
                printf("  unparseable vector row %lu: %s\n", (unsigned long)(count + 1), line);
                ISO_CHECK(0);
                continue;
            }
            for (k = 0; k < 9; k++) {
                if (strcmp(OPS[k], r.op) == 0) {
                    counts[k]++;
                }
            }
            run_row(count, &r);
            count++;
        }
    }
    ISO_CHECK_EQ_INT(count, 820);
    for (k = 0; k < 9; k++) {
        if (counts[k] < 50) {
            printf("  only %lu vectors for %s\n", (unsigned long)counts[k], OPS[k]);
        }
        ISO_CHECK(counts[k] >= 50);
    }
    ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, 0);
    free(text);
    return ISO_TEST_RESULT();
}

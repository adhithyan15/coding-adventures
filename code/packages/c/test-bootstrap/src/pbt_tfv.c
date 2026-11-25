/*
 * pbt_tfv.c -- strict TFV v1 validator (VER05i section 8; TF01 section 2).
 *
 * The format in one screen:
 *
 *     tfv 1
 *     # comment
 *     <id> R<n> K<field> <value> K<field> <value> ...
 *
 *     value := N | T | F          null, true, false
 *            | I<int>             canonical decimal integer, <= 1000 digits
 *            | D<16 hex>          IEEE-754 bit pattern, lowercase
 *            | X<hex> | S<hex>    bytes / UTF-8 text as even-length lowercase hex
 *            | L<n> v1..vn        list
 *            | M<n> k1 v1..kn vn  map; no two keys deep-equal
 *
 * Worked examples.  "tfv 1\nr R1 Kop S706274 Kdata XFF\n" is rejected: XFF is
 * uppercase hex (and odd length).  "bad header\n" is rejected: the first line
 * must be exactly "tfv 1".
 *
 * RESOURCE DISCIPLINE.  The validator reserves no memory from a COUNT in the
 * input (a count only bounds a loop that consumes tokens, and the token total is
 * capped at 200,000).  Deep-equality key strings are built only where a map
 * needs them, charged against a byte budget, and registered so they are freed on
 * every exit path.  Duplicate detection sorts, so it is O(n log n).
 */
#include "pbt_tfv.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "pbt_internal.h"

#define MAX_KEY_BYTES ((size_t)8 * 1024 * 1024)

typedef struct strbuf {
    char *p;
    size_t n;
    size_t cap;
} strbuf;

typedef struct ref {
    const char *p;
    size_t n;
} ref;

typedef struct tctx {
    const char *cur; /* cursor inside the current line */
    const char *lend;
    size_t tokens;
    size_t keybytes;
    char **garbage;
    size_t ngarbage;
    size_t capgarbage;
    char msg[96];
    int oom;
} tctx;

static int bad(tctx *c, const char *m)
{
    if (c->msg[0] == '\0') {
        (void)snprintf(c->msg, sizeof c->msg, "%s", m);
    }
    return 1;
}

/*
 * Register a heap string so it is freed when validation ends.
 *
 * OWNERSHIP RULE (one rule, every caller): on success (0) the garbage list owns
 * `s`; on failure (1) the caller STILL owns `s` and must free it.  keep() never
 * frees, so no path can free a block twice.
 */
static int keep(tctx *c, char *s)
{
    if (c->ngarbage == c->capgarbage) {
        size_t ncap = c->capgarbage == 0 ? 16 : c->capgarbage * 2;
        char **g = (char **)pbt_realloc(c->garbage, ncap, sizeof *g);
        if (g == NULL) {
            c->oom = 1;
            return 1;
        }
        c->garbage = g;
        c->capgarbage = ncap;
    }
    c->garbage[c->ngarbage++] = s;
    return 0;
}

/* A fresh NUL-terminated copy, charged against the key budget and registered. */
static char *dup_charged(tctx *c, const char *s, size_t n)
{
    char *d;
    if (n > MAX_KEY_BYTES - c->keybytes) {
        (void)bad(c, "deep-equality keys exceed the size budget");
        return NULL;
    }
    d = (char *)pbt_alloc(n + 1, 1);
    if (d == NULL) {
        c->oom = 1;
        return NULL;
    }
    memcpy(d, s, n);
    d[n] = '\0';
    c->keybytes += n + 1;
    if (keep(c, d)) {
        pbt_free(d); /* keep() left ownership with us */
        return NULL;
    }
    return d;
}

static int sb_add(tctx *c, strbuf *b, const char *s, size_t n)
{
    if (n > MAX_KEY_BYTES || b->n > MAX_KEY_BYTES - n || c->keybytes + n > MAX_KEY_BYTES) {
        return bad(c, "deep-equality keys exceed the size budget");
    }
    if (b->n + n + 1 > b->cap) {
        size_t cap = b->cap == 0 ? 64 : b->cap;
        char *np;
        while (cap < b->n + n + 1) {
            cap *= 2;
        }
        np = (char *)pbt_realloc(b->p, cap, 1);
        if (np == NULL) {
            c->oom = 1;
            return 1;
        }
        b->p = np;
        b->cap = cap;
    }
    memcpy(b->p + b->n, s, n);
    b->n += n;
    b->p[b->n] = '\0';
    return 0;
}

/*
 * Hand a finished buffer to the garbage list (charging its size) and return it.
 * On NULL the buffer is STILL OWNED BY THE CALLER (b->p), who frees it.
 */
static char *sb_finish(tctx *c, strbuf *b)
{
    if (b->p == NULL && sb_add(c, b, "", 0)) {
        return NULL;
    }
    c->keybytes += b->n + 1;
    if (keep(c, b->p)) {
        return NULL;
    }
    return b->p;
}

/* The next space-separated token of the current line (lines were pre-checked: no empty tokens). */
static int next_token(tctx *c, const char **tok, size_t *tl)
{
    const char *p = c->cur;
    if (c->cur >= c->lend) {
        return bad(c, "record ends early");
    }
    while (p < c->lend && *p != ' ') {
        p++;
    }
    *tok = c->cur;
    *tl = (size_t)(p - c->cur);
    c->cur = p < c->lend ? p + 1 : p;
    /* A line ending right after a separator would leave cur == lend with a token pending; pre-checked. */
    if (++c->tokens > PBT_TFV_MAX_TOKENS) {
        return bad(c, "too many tokens");
    }
    return 0;
}

static int more_tokens(const tctx *c)
{
    return c->cur < c->lend;
}

static int is_id_byte(unsigned char ch)
{
    return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch == '.' ||
           ch == '_' || ch == '/' || ch == '-';
}

static int valid_field(const char *s, size_t n)
{
    size_t i;
    if (n == 0 || s[0] < 'a' || s[0] > 'z') {
        return 0;
    }
    for (i = 1; i < n; i++) {
        char ch = s[i];
        if (!((ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9') || ch == '_')) {
            return 0;
        }
    }
    return 1;
}

/* 0 or digits without a leading zero, at most 9 digits; result in *out. */
static int canonical_nat(tctx *c, const char *s, size_t n, size_t *out)
{
    size_t i;
    size_t v = 0;
    if (n == 0) {
        return bad(c, "empty count");
    }
    for (i = 0; i < n; i++) {
        if (s[i] < '0' || s[i] > '9') {
            return bad(c, "non-digit in count");
        }
    }
    if (n > 1 && s[0] == '0') {
        return bad(c, "leading zero in count");
    }
    if (n > 9) {
        return bad(c, "count too large");
    }
    for (i = 0; i < n; i++) {
        v = v * 10 + (size_t)(s[i] - '0');
    }
    *out = v;
    return 0;
}

static int hexv(char ch)
{
    if (ch >= '0' && ch <= '9') {
        return ch - '0';
    }
    if (ch >= 'a' && ch <= 'f') {
        return ch - 'a' + 10;
    }
    return -1;
}

/* Even-length lowercase hex: validates (and optionally decodes into out, which must hold n/2 bytes). */
static int lower_hex(const char *s, size_t n, unsigned char *out)
{
    size_t i;
    if (n % 2 != 0) {
        return 0;
    }
    for (i = 0; i < n; i += 2) {
        int h = hexv(s[i]);
        int l = hexv(s[i + 1]);
        if (h < 0 || l < 0) {
            return 0;
        }
        if (out != NULL) {
            out[i / 2] = (unsigned char)(h * 16 + l);
        }
    }
    return 1;
}

static int valid_int(tctx *c, const char *body, size_t n)
{
    size_t i = 0;
    int neg = 0;
    size_t digits;
    if (n > 0 && body[0] == '-') {
        neg = 1;
        i = 1;
    }
    digits = n - i;
    if (digits == 0) {
        return bad(c, "empty integer");
    }
    for (; i < n; i++) {
        if (body[i] < '0' || body[i] > '9') {
            return bad(c, "non-digit in integer");
        }
    }
    if (body[neg] == '0' && (digits > 1 || neg)) {
        return bad(c, "non-canonical integer");
    }
    if (digits > PBT_TFV_MAX_INT_DIGITS) {
        return bad(c, "integer has too many digits");
    }
    return 0;
}

static int cmp_str(const void *a, const void *b)
{
    return strcmp(*(char *const *)a, *(char *const *)b);
}

static int cmp_ref(const void *a, const void *b)
{
    const ref *x = (const ref *)a;
    const ref *y = (const ref *)b;
    size_t m = x->n < y->n ? x->n : y->n;
    int r = m == 0 ? 0 : memcmp(x->p, y->p, m);
    if (r != 0) {
        return r;
    }
    return x->n < y->n ? -1 : (x->n > y->n ? 1 : 0);
}

/* True when two refs are equal.  Sorts `refs` in place. */
static int has_dup_ref(ref *refs, size_t n)
{
    size_t i;
    if (n < 2) {
        return 0;
    }
    qsort(refs, n, sizeof *refs, cmp_ref);
    for (i = 1; i < n; i++) {
        if (cmp_ref(&refs[i - 1], &refs[i]) == 0) {
            return 1;
        }
    }
    return 0;
}

/*
 * Validate one value.  When need_key is set, *key receives the deep-equality
 * key: a string equal for two values exactly when TF01 section 3.1 calls them
 * deep-equal (every NaN equals every NaN, -0 and +0 stay distinct, map entries
 * are order independent).
 */
static int value(tctx *c, int depth, int need_key, const char **key)
{
    const char *tok;
    size_t tl;
    char tag;
    const char *body;
    size_t bl;
    *key = NULL;
    if (next_token(c, &tok, &tl)) {
        return 1;
    }
    tag = tok[0];
    body = tok + 1;
    bl = tl - 1;
    switch (tag) {
    case 'N':
    case 'T':
    case 'F':
        if (bl != 0) {
            return bad(c, "unexpected body after N, T or F");
        }
        if (need_key) {
            *key = dup_charged(c, &tag, 1);
            return *key == NULL;
        }
        return 0;
    case 'I':
        if (valid_int(c, body, bl)) {
            return 1;
        }
        break;
    case 'D': {
        unsigned char raw[8];
        if (bl != 16 || !lower_hex(body, bl, raw)) {
            return bad(c, "float needs exactly 16 lowercase hex digits");
        }
        if (need_key) {
            unsigned exp = (unsigned)(raw[0] & 0x7f) << 4 | (unsigned)(raw[1] >> 4);
            int mant_zero = (raw[1] & 0x0f) == 0;
            int i;
            for (i = 2; i < 8; i++) {
                mant_zero = mant_zero && raw[i] == 0;
            }
            if (exp == 0x7ff && !mant_zero) {
                *key = dup_charged(c, "Dnan", 4);
                return *key == NULL;
            }
        }
        break;
    }
    case 'X':
        if (!lower_hex(body, bl, NULL)) {
            return bad(c, "bytes need even-length lowercase hex");
        }
        break;
    case 'S': {
        unsigned char *raw;
        size_t i = 0;
        int ok = 1;
        if (!lower_hex(body, bl, NULL)) {
            return bad(c, "text needs even-length lowercase hex");
        }
        raw = (unsigned char *)pbt_alloc(bl / 2 + 1, 1);
        if (raw == NULL) {
            c->oom = 1;
            return 1;
        }
        (void)lower_hex(body, bl, raw);
        while (i < bl / 2) {
            size_t l = pbt_utf8_len(raw + i, bl / 2 - i);
            if (l == 0) {
                ok = 0;
                break;
            }
            i += l;
        }
        pbt_free(raw);
        if (!ok) {
            return bad(c, "text is not strict UTF-8");
        }
        break;
    }
    case 'L':
    case 'M': {
        size_t count;
        size_t i;
        if (depth + 1 > PBT_TFV_MAX_DEPTH) {
            return bad(c, "nesting too deep");
        }
        if (canonical_nat(c, body, bl, &count)) {
            return 1;
        }
        if (tag == 'L') {
            strbuf sb;
            sb.p = NULL;
            sb.n = 0;
            sb.cap = 0;
            if (need_key && sb_add(c, &sb, "L[", 2)) {
                pbt_free(sb.p);
                return 1;
            }
            for (i = 0; i < count; i++) {
                const char *k;
                if (value(c, depth + 1, need_key, &k)) {
                    pbt_free(sb.p);
                    return 1;
                }
                if (need_key && (sb_add(c, &sb, k, strlen(k)) || sb_add(c, &sb, ",", 1))) {
                    pbt_free(sb.p);
                    return 1;
                }
            }
            if (need_key) {
                if (sb_add(c, &sb, "]", 1)) {
                    pbt_free(sb.p);
                    return 1;
                }
                *key = sb_finish(c, &sb);
                if (*key == NULL) {
                    pbt_free(sb.p);
                    return 1;
                }
            }
            return 0;
        } else {
            /* Map: collect key strings (always needed, for the duplicate check) and entries (only when this map is a key). */
            char **keys = NULL;
            char **entries = NULL;
            size_t cap = 0;
            size_t nk = 0;
            int rc = 0;
            for (i = 0; i < count && !rc; i++) {
                const char *k;
                const char *v;
                if (nk == cap) {
                    size_t ncap = cap == 0 ? 8 : cap * 2;
                    char **nkeys = (char **)pbt_realloc(keys, ncap, sizeof *nkeys);
                    char **nent;
                    if (nkeys == NULL) {
                        c->oom = 1;
                        rc = 1;
                        break;
                    }
                    keys = nkeys;
                    nent = (char **)pbt_realloc(entries, ncap, sizeof *nent);
                    if (nent == NULL) {
                        c->oom = 1;
                        rc = 1;
                        break;
                    }
                    entries = nent;
                    cap = ncap;
                }
                if (value(c, depth + 1, 1, &k) || value(c, depth + 1, need_key, &v)) {
                    rc = 1;
                    break;
                }
                keys[nk] = (char *)k;
                if (need_key) {
                    strbuf e;
                    e.p = NULL;
                    e.n = 0;
                    e.cap = 0;
                    if (sb_add(c, &e, "(", 1) || sb_add(c, &e, k, strlen(k)) || sb_add(c, &e, "=>", 2) ||
                        sb_add(c, &e, v, strlen(v)) || sb_add(c, &e, ")", 1)) {
                        pbt_free(e.p);
                        rc = 1;
                        break;
                    }
                    entries[nk] = sb_finish(c, &e);
                    if (entries[nk] == NULL) {
                        pbt_free(e.p);
                        rc = 1;
                        break;
                    }
                }
                nk++;
            }
            if (!rc && nk > 1) {
                size_t j;
                char **sorted = (char **)pbt_alloc(nk, sizeof *sorted);
                if (sorted == NULL) {
                    c->oom = 1;
                    rc = 1;
                } else {
                    memcpy(sorted, keys, nk * sizeof *sorted);
                    qsort(sorted, nk, sizeof *sorted, cmp_str);
                    for (j = 1; j < nk; j++) {
                        if (strcmp(sorted[j - 1], sorted[j]) == 0) {
                            rc = bad(c, "duplicate map key");
                            break;
                        }
                    }
                    pbt_free(sorted);
                }
            }
            if (!rc && need_key) {
                strbuf sb;
                size_t j;
                sb.p = NULL;
                sb.n = 0;
                sb.cap = 0;
                if (nk > 1) {
                    qsort(entries, nk, sizeof *entries, cmp_str);
                }
                rc = sb_add(c, &sb, "M{", 2);
                for (j = 0; j < nk && !rc; j++) {
                    rc = sb_add(c, &sb, entries[j], strlen(entries[j]));
                }
                if (!rc) {
                    rc = sb_add(c, &sb, "}", 1);
                }
                if (!rc) {
                    *key = sb_finish(c, &sb);
                    rc = *key == NULL;
                }
                if (rc) {
                    pbt_free(sb.p);
                }
            }
            pbt_free(keys);
            pbt_free(entries);
            return rc;
        }
    }
    default:
        return bad(c, "unknown tag");
    }
    /* Scalars I, D, X, S: the token itself is the key. */
    if (need_key) {
        *key = dup_charged(c, tok, tl);
        return *key == NULL;
    }
    return 0;
}

/* One record line.  On success *id / *idlen name the record id (inside the input). */
static int record(tctx *c, const char *line, size_t len, const char **id, size_t *idlen)
{
    const char *tok;
    size_t tl;
    size_t count;
    size_t i;
    ref *fields = NULL;
    size_t nf = 0;
    size_t capf = 0;
    int rc = 0;

    if (memchr(line, '\r', len) != NULL) {
        return bad(c, "carriage return in a record line");
    }
    /* No leading, trailing or doubled space: every token is then non-empty. */
    if (line[0] == ' ' || line[len - 1] == ' ') {
        return bad(c, "leading, trailing or doubled space");
    }
    for (i = 1; i < len; i++) {
        if (line[i] == ' ' && line[i - 1] == ' ') {
            return bad(c, "leading, trailing or doubled space");
        }
    }
    c->cur = line;
    c->lend = line + len;
    if (next_token(c, id, idlen)) {
        return 1;
    }
    for (i = 0; i < *idlen; i++) {
        if (!is_id_byte((unsigned char)(*id)[i])) {
            return bad(c, "bad record id");
        }
    }
    if (next_token(c, &tok, &tl)) {
        return bad(c, "a record needs an id and R<n>");
    }
    if (tok[0] != 'R') {
        return bad(c, "expected R<n>");
    }
    if (canonical_nat(c, tok + 1, tl - 1, &count)) {
        return 1;
    }
    for (i = 0; i < count && !rc; i++) {
        const char *k;
        if (next_token(c, &tok, &tl)) {
            rc = 1;
            break;
        }
        if (tok[0] != 'K' || !valid_field(tok + 1, tl - 1)) {
            rc = bad(c, "bad field key");
            break;
        }
        if (nf == capf) {
            size_t ncap = capf == 0 ? 8 : capf * 2;
            ref *nr = (ref *)pbt_realloc(fields, ncap, sizeof *nr);
            if (nr == NULL) {
                c->oom = 1;
                rc = 1;
                break;
            }
            fields = nr;
            capf = ncap;
        }
        fields[nf].p = tok + 1;
        fields[nf].n = tl - 1;
        nf++;
        if (value(c, 0, 0, &k)) {
            rc = 1;
            break;
        }
    }
    if (!rc && has_dup_ref(fields, nf)) {
        rc = bad(c, "duplicate field");
    }
    if (!rc && more_tokens(c)) {
        rc = bad(c, "trailing tokens after the last field");
    }
    pbt_free(fields);
    return rc;
}

int pbt_tfv_validate(const char *text, size_t len, char *err, size_t errcap)
{
    tctx c;
    size_t pos = 0;
    size_t i;
    size_t lineno = 0;
    ref *ids = NULL;
    size_t nids = 0;
    size_t capids = 0;
    int rc = 0;
    char where[160];

    memset(&c, 0, sizeof c);
    if (len > PBT_TFV_MAX_BYTES) {
        rc = bad(&c, "input is too large");
        goto done;
    }
    for (i = 0; i < len;) {
        size_t l = pbt_utf8_len((const unsigned char *)text + i, len - i);
        if (l == 0) {
            rc = bad(&c, "document is not valid UTF-8");
            goto done;
        }
        i += l;
    }
    /* The first line must be exactly "tfv 1" (the final newline is optional). */
    {
        size_t e = 0;
        while (e < len && text[e] != '\n') {
            e++;
        }
        if (e != 5 || memcmp(text, "tfv 1", 5) != 0) {
            rc = bad(&c, "the first line must be exactly \"tfv 1\"");
            goto done;
        }
        pos = e < len ? e + 1 : len;
        lineno = 1;
    }
    while (pos < len) {
        size_t e = pos;
        const char *id;
        size_t idlen;
        while (e < len && text[e] != '\n') {
            e++;
        }
        lineno++;
        if (e == pos || text[pos] == '#') { /* blank and comment lines are skipped whole */
            pos = e < len ? e + 1 : len;
            continue;
        }
        if (record(&c, text + pos, e - pos, &id, &idlen)) {
            rc = 1;
            (void)snprintf(where, sizeof where, "line %lu: %s", (unsigned long)lineno, c.msg);
            if (err != NULL && errcap > 0) {
                (void)snprintf(err, errcap, "%s", where);
            }
            goto cleanup;
        }
        if (nids == capids) {
            size_t ncap = capids == 0 ? 16 : capids * 2;
            ref *nr = (ref *)pbt_realloc(ids, ncap, sizeof *nr);
            if (nr == NULL) {
                c.oom = 1;
                rc = 1;
                goto done;
            }
            ids = nr;
            capids = ncap;
        }
        ids[nids].p = id;
        ids[nids].n = idlen;
        nids++;
        pos = e < len ? e + 1 : len;
    }
    if (has_dup_ref(ids, nids)) {
        rc = bad(&c, "duplicate record id");
    }

done:
    if (rc && err != NULL && errcap > 0) {
        (void)snprintf(err, errcap, "%s", c.oom ? "out of memory" : c.msg);
    }
cleanup:
    for (i = 0; i < c.ngarbage; i++) {
        pbt_free(c.garbage[i]);
    }
    pbt_free(c.garbage);
    pbt_free(ids);
    return rc;
}

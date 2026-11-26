#include "test_util.h"

#include "pbt_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char *tu_read_file(const char *path, size_t *len)
{
    FILE *f = fopen(path, "rb");
    long size;
    char *buf;
    size_t got;
    if (f == NULL) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0 || (size = ftell(f)) < 0 || fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return NULL;
    }
    buf = (char *)malloc((size_t)size + 1);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    buf[got] = '\0';
    if (len != NULL) {
        *len = got;
    }
    return buf;
}

char *tu_read_corpus_text(size_t *len)
{
    static const char *const prefixes[] = {"", "../", "../../", "../../../", "../../../../", "../../../../../",
                                           "../../../../../../", "../../../../../../../"};
    size_t i;
    for (i = 0; i < sizeof prefixes / sizeof prefixes[0]; i++) {
        char path[256];
        char *text;
        (void)snprintf(path, sizeof path, "%scode/specs/fixtures/pbt-v1/cases.json", prefixes[i]);
        text = tu_read_file(path, len);
        if (text != NULL) {
            return text;
        }
    }
    return NULL;
}

char *tu_read_repo_file(const char *relpath, size_t *len)
{
    static const char *const prefixes[] = {"", "../", "../../", "../../../", "../../../../", "../../../../../",
                                           "../../../../../../", "../../../../../../../"};
    size_t i;
    for (i = 0; i < sizeof prefixes / sizeof prefixes[0]; i++) {
        char path[320];
        char *text;
        (void)snprintf(path, sizeof path, "%s%s", prefixes[i], relpath);
        text = tu_read_file(path, len);
        if (text != NULL) {
            return text;
        }
    }
    return NULL;
}

pbt_json *tu_load_corpus(void)
{
    size_t len = 0;
    char *text = tu_read_corpus_text(&len);
    pbt_json *doc = NULL;
    char err[200];
    if (text == NULL) {
        return NULL;
    }
    if (pbt_json_parse(text, len, &doc, err, sizeof err) != 0) {
        free(text);
        return NULL;
    }
    free(text);
    return doc;
}

pbt_big tu_big(const char *dec)
{
    pbt_big b;
    (void)pbt_big_parse_dec(&b, dec, strlen(dec));
    return b;
}

typedef struct fn_holder {
    tu_val_fn fn;
} fn_holder;

static int holder_prop(void *user, pbt_budget *scope, const pbt_val *v)
{
    (void)scope;
    return ((fn_holder *)user)->fn(v);
}

/* Function pointers cannot be smuggled through void *, so a small static pool carries them. */
pbt_prop tu_prop(tu_val_fn fn)
{
    static fn_holder pool[64];
    static size_t used;
    pbt_prop p;
    size_t i;
    for (i = 0; i < used; i++) {
        if (pool[i].fn == fn) {
            break;
        }
    }
    if (i == used) {
        i = used++ % 64;
        pool[i].fn = fn;
    }
    p.fn = holder_prop;
    p.user = &pool[i];
    return p;
}

static uint64_t inv_xorshift(uint64_t z, unsigned shift)
{
    /* x ^ (x >> s) is inverted by folding in the shifted copies until the shift leaves the word. */
    uint64_t x = z;
    unsigned s;
    for (s = shift; s < 64; s += shift) {
        x ^= z >> s;
    }
    return x;
}

static uint64_t inv_odd(uint64_t c)
{
    /* Newton iteration for the multiplicative inverse of an odd number modulo 2^64. */
    uint64_t x = c;
    int i;
    for (i = 0; i < 6; i++) {
        x *= (uint64_t)2 - c * x;
    }
    return x;
}

uint64_t tu_state_for_output(uint64_t target)
{
    uint64_t z = target;
    z = inv_xorshift(z, 31);
    z *= inv_odd(0x94d049bb133111ebu);
    z = inv_xorshift(z, 27);
    z *= inv_odd(0xbf58476d1ce4e5b9u);
    z = inv_xorshift(z, 30);
    /* z is the state AFTER the gamma step; the state before it is z - gamma. */
    return z - 0x9e3779b97f4a7c15u;
}

void tu_set_str(pbt_json *node, const char *text)
{
    size_t n = strlen(text);
    pbt_free(node->s);
    node->s = (char *)pbt_alloc(n + 1, 1);
    memcpy(node->s, text, n + 1);
    node->slen = n;
}

pbt_json *tu_case(pbt_json *doc, size_t index)
{
    return &pbt_json_get(doc, "cases")->items[index];
}

const char *tu_case_id(pbt_json *doc, size_t index)
{
    return pbt_json_get(tu_case(doc, index), "id")->s;
}

void tu_set_int_field(pbt_json *cs, const char *field, const char *dec)
{
    tu_set_str(pbt_json_get(pbt_json_get(cs, field), "value"), dec);
}

int tu_report_names(const pbt_corpus_report *r, const char *id)
{
    size_t i;
    size_t n = strlen(id);
    for (i = 0; i < r->nstored; i++) {
        if (strncmp(r->problems[i], id, n) == 0 && r->problems[i][n] == ':') {
            return 1;
        }
    }
    return 0;
}

int tu_report_names_other(const pbt_corpus_report *r, pbt_json *doc, const char *id)
{
    size_t ncases = pbt_json_get(doc, "cases")->n;
    size_t k;
    for (k = 0; k < ncases; k++) {
        const char *other = tu_case_id(doc, k);
        if (strcmp(other, id) != 0 && tu_report_names(r, other)) {
            return 1;
        }
    }
    return 0;
}

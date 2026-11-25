/*
 * pbt_json.h -- strict JSON reader (VER05i section 8).
 * ===========================================================================
 *
 * The pbt-v1 corpus is JSON, and a conformance harness must refuse a corrupted
 * fixture instead of "making sense" of it.  This reader is deliberately
 * stricter than most:
 *
 *   - nesting depth at most PBT_JSON_MAX_DEPTH (64)
 *   - document at most PBT_JSON_MAX_BYTES (8 MiB)
 *   - at most PBT_JSON_MAX_NODES (200,000) values in total
 *   - duplicate object keys are an error (JSON itself leaves them undefined)
 *   - invalid UTF-8 is an error: overlong forms, surrogates, > U+10FFFF
 *   - numbers follow the JSON grammar exactly (no leading zeros, no "+", no
 *     bare "." ...) and are KEPT AS TEXT: no number is ever converted to a
 *     double, so the corpus' integers (carried as decimal strings) cannot lose
 *     precision
 *   - lone or reversed surrogate escapes (\ud800) are an error
 *   - unescaped control characters (< 0x20) inside strings are an error
 *   - trailing data after the document is an error
 *
 * Strings are stored decoded, NUL terminated, with their length (an embedded
 * \u0000 is legal, so always use `slen`).
 */
#ifndef PBT_JSON_H
#define PBT_JSON_H

#include <stddef.h>

#define PBT_JSON_MAX_DEPTH 64
#define PBT_JSON_MAX_BYTES ((size_t)8 * 1024 * 1024)
#define PBT_JSON_MAX_NODES 200000

typedef enum pbt_jtype {
    PBT_J_NULL,
    PBT_J_BOOL,
    PBT_J_NUM,
    PBT_J_STR,
    PBT_J_ARR,
    PBT_J_OBJ
} pbt_jtype;

typedef struct pbt_json pbt_json;
struct pbt_json {
    pbt_jtype type;
    int b;                 /* PBT_J_BOOL */
    char *s;               /* PBT_J_STR decoded text, PBT_J_NUM literal text (NUL terminated) */
    size_t slen;
    size_t n;              /* element / member count for PBT_J_ARR / PBT_J_OBJ */
    pbt_json *items;       /* n values */
    char **keys;           /* n keys (PBT_J_OBJ only) */
    size_t *keylens;
};

/*
 * Parse text[0..len).  Returns 0 and a heap tree in *out, or nonzero and a
 * short message in err (when errcap > 0).  *out is NULL on failure.
 */
int pbt_json_parse(const char *text, size_t len, pbt_json **out, char *err, size_t errcap);
void pbt_json_free(pbt_json *root);
/* Deep copy (bounded by the parser's depth); NULL on out-of-memory. */
pbt_json *pbt_json_clone(const pbt_json *root);
/* Object member lookup by NUL-terminated key; NULL if absent or not an object. */
pbt_json *pbt_json_get(const pbt_json *obj, const char *key);

#endif /* PBT_JSON_H */

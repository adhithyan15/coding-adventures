/*
 * canonical_cbor.c — implementation of the pure-ISO C canonical CBOR codec.
 * ========================================================================
 *
 * Structure mirrors the Rust crate: a recursive `CborValue` tree, an encoder
 * that emits canonical bytes by construction (smallest-form integers,
 * length-first map key ordering), and a strict decoder that rejects every
 * non-canonical or hostile input (expanded integers, indefinite lengths,
 * non-minimal forms, over-long declared lengths, and over-deep nesting).
 */
#include "canonical_cbor.h"

#include <stdlib.h> /* malloc, realloc, free, calloc */
#include <string.h> /* memcpy, memcmp */

/* ── Value construction / destruction ─────────────────────────────────────*/

static CborValue *alloc_value(CborType type) {
    CborValue *v = (CborValue *)calloc(1, sizeof(CborValue));
    if (v != NULL) v->type = type;
    return v;
}

CborValue *cbor_unsigned(uint64_t n) {
    CborValue *v = alloc_value(CBOR_UNSIGNED);
    if (v != NULL) v->as.u = n;
    return v;
}

CborValue *cbor_negative(uint64_t n) {
    CborValue *v = alloc_value(CBOR_NEGATIVE);
    if (v != NULL) v->as.u = n;
    return v;
}

CborValue *cbor_bool(int b) {
    CborValue *v = alloc_value(CBOR_BOOL);
    if (v != NULL) v->as.boolean = b ? 1 : 0;
    return v;
}

CborValue *cbor_null(void) { return alloc_value(CBOR_NULL); }

/* Shared body for Bytes and Text (both are a copied byte block). */
static CborValue *make_blob(CborType type, const uint8_t *data, size_t len) {
    CborValue *v = alloc_value(type);
    if (v == NULL) return NULL;
    if (len > 0) {
        v->as.bytes.data = (uint8_t *)malloc(len);
        if (v->as.bytes.data == NULL) {
            free(v);
            return NULL;
        }
        memcpy(v->as.bytes.data, data, len);
    }
    v->as.bytes.len = len;
    return v;
}

CborValue *cbor_bytes(const uint8_t *data, size_t len) {
    return make_blob(CBOR_BYTES, data, len);
}

CborValue *cbor_text(const char *utf8, size_t len) {
    return make_blob(CBOR_TEXT, (const uint8_t *)utf8, len);
}

CborValue *cbor_array(void) { return alloc_value(CBOR_ARRAY); }
CborValue *cbor_map(void) { return alloc_value(CBOR_MAP); }

CborValue *cbor_tag(uint64_t number, CborValue *inner) {
    CborValue *v = alloc_value(CBOR_TAG);
    if (v == NULL) {
        cbor_free(inner); /* take ownership even on failure */
        return NULL;
    }
    v->as.tag.number = number;
    v->as.tag.inner = inner;
    return v;
}

/* Grow a pointer/entry array to hold at least `need` elements of `elem` bytes.
 * Doubling is capped so `nc * elem` cannot overflow size_t. */
static int grow_array(void **base, size_t *cap, size_t need, size_t elem) {
    if (need <= *cap) return 1;
    size_t nc = *cap ? *cap : 4;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2 / elem) {
            nc = need; /* last resort: exact fit (need is a real element count) */
            break;
        }
        nc *= 2;
    }
    if (nc > ((size_t)-1) / elem) return 0; /* size overflow guard */
    void *p = realloc(*base, nc * elem);
    if (p == NULL) return 0;
    *base = p;
    *cap = nc;
    return 1;
}

CborStatus cbor_array_push(CborValue *array, CborValue *item) {
    void *base = array->as.array.items;
    if (!grow_array(&base, &array->as.array.cap, array->as.array.len + 1,
                    sizeof(CborValue *))) {
        cbor_free(item);
        return CBOR_ERR_ALLOC;
    }
    array->as.array.items = (CborValue **)base;
    array->as.array.items[array->as.array.len++] = item;
    return CBOR_OK;
}

CborStatus cbor_map_push(CborValue *map, CborValue *key, CborValue *val) {
    void *base = map->as.map.entries;
    if (!grow_array(&base, &map->as.map.cap, map->as.map.len + 1,
                    sizeof(CborPair))) {
        cbor_free(key);
        cbor_free(val);
        return CBOR_ERR_ALLOC;
    }
    map->as.map.entries = (CborPair *)base;
    map->as.map.entries[map->as.map.len].key = key;
    map->as.map.entries[map->as.map.len].val = val;
    map->as.map.len++;
    return CBOR_OK;
}

void cbor_free(CborValue *v) {
    if (v == NULL) return;
    switch (v->type) {
        case CBOR_BYTES:
        case CBOR_TEXT:
            free(v->as.bytes.data);
            break;
        case CBOR_ARRAY:
            for (size_t i = 0; i < v->as.array.len; i++)
                cbor_free(v->as.array.items[i]);
            free(v->as.array.items);
            break;
        case CBOR_MAP:
            for (size_t i = 0; i < v->as.map.len; i++) {
                cbor_free(v->as.map.entries[i].key);
                cbor_free(v->as.map.entries[i].val);
            }
            free(v->as.map.entries);
            break;
        case CBOR_TAG:
            cbor_free(v->as.tag.inner);
            break;
        default:
            break; /* Unsigned / Negative / Bool / Null own nothing */
    }
    free(v);
}

int cbor_equal(const CborValue *a, const CborValue *b) {
    if (a == NULL || b == NULL) return a == b;
    if (a->type != b->type) return 0;
    switch (a->type) {
        case CBOR_UNSIGNED:
        case CBOR_NEGATIVE:
            return a->as.u == b->as.u;
        case CBOR_BOOL:
            return a->as.boolean == b->as.boolean;
        case CBOR_NULL:
            return 1;
        case CBOR_BYTES:
        case CBOR_TEXT:
            return a->as.bytes.len == b->as.bytes.len &&
                   (a->as.bytes.len == 0 ||
                    memcmp(a->as.bytes.data, b->as.bytes.data,
                           a->as.bytes.len) == 0);
        case CBOR_ARRAY:
            if (a->as.array.len != b->as.array.len) return 0;
            for (size_t i = 0; i < a->as.array.len; i++)
                if (!cbor_equal(a->as.array.items[i], b->as.array.items[i]))
                    return 0;
            return 1;
        case CBOR_MAP:
            if (a->as.map.len != b->as.map.len) return 0;
            for (size_t i = 0; i < a->as.map.len; i++)
                if (!cbor_equal(a->as.map.entries[i].key,
                                b->as.map.entries[i].key) ||
                    !cbor_equal(a->as.map.entries[i].val,
                                b->as.map.entries[i].val))
                    return 0;
            return 1;
        case CBOR_TAG:
            return a->as.tag.number == b->as.tag.number &&
                   cbor_equal(a->as.tag.inner, b->as.tag.inner);
    }
    return 0;
}

const char *cbor_status_message(CborStatus status) {
    switch (status) {
        case CBOR_OK: return "canonical-cbor: ok";
        case CBOR_ERR_UNEXPECTED_EOF: return "canonical-cbor: unexpected end of input";
        case CBOR_ERR_TRAILING_BYTES: return "canonical-cbor: trailing bytes after decoded item";
        case CBOR_ERR_RESERVED: return "canonical-cbor: reserved additional-info value";
        case CBOR_ERR_INDEFINITE: return "canonical-cbor: indefinite-length item rejected";
        case CBOR_ERR_NON_MINIMAL_INTEGER: return "canonical-cbor: argument is not smallest-form";
        case CBOR_ERR_INVALID_UTF8: return "canonical-cbor: text is not valid UTF-8";
        case CBOR_ERR_NON_CANONICAL_MAP_ORDER: return "canonical-cbor: map order is not canonical";
        case CBOR_ERR_UNSUPPORTED_SIMPLE: return "canonical-cbor: unsupported simple value";
        case CBOR_ERR_FLOAT_NOT_SUPPORTED: return "canonical-cbor: floats are not supported";
        case CBOR_ERR_TOO_DEEP: return "canonical-cbor: decode nesting depth exceeded";
        case CBOR_ERR_LENGTH_TOO_LARGE: return "canonical-cbor: declared length is too large";
        case CBOR_ERR_DUPLICATE_MAP_KEY: return "canonical-cbor: duplicate canonical map key";
        case CBOR_ERR_ENCODE_TOO_DEEP: return "canonical-cbor: encode nesting depth exceeded";
        case CBOR_ERR_ENCODE_TOO_LARGE: return "canonical-cbor: encoded item is too large";
        case CBOR_ERR_ALLOC: return "canonical-cbor: allocation failed";
    }
    return "canonical-cbor: unknown status";
}

/* ── Encoder ──────────────────────────────────────────────────────────────*/

/* A growable byte buffer used to build encoded output. */
typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
} ByteBuf;

static CborStatus bb_reserve(ByteBuf *b, size_t extra) {
    if (extra > ((size_t)-1) - b->len) return CBOR_ERR_ENCODE_TOO_LARGE;
    size_t need = b->len + extra;
    if (need > CBOR_MAX_ENCODED_SIZE) return CBOR_ERR_ENCODE_TOO_LARGE;
    if (need <= b->cap) return CBOR_OK;
    size_t nc = b->cap ? b->cap : 16;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    uint8_t *p = (uint8_t *)realloc(b->data, nc);
    if (p == NULL) return CBOR_ERR_ALLOC;
    b->data = p;
    b->cap = nc;
    return CBOR_OK;
}

static CborStatus bb_push(ByteBuf *b, uint8_t byte) {
    CborStatus status = bb_reserve(b, 1);
    if (status != CBOR_OK) return status;
    b->data[b->len++] = byte;
    return CBOR_OK;
}

static CborStatus bb_extend(ByteBuf *b, const uint8_t *src, size_t n) {
    CborStatus status;
    if (n == 0) return CBOR_OK;
    status = bb_reserve(b, n);
    if (status != CBOR_OK) return status;
    memcpy(b->data + b->len, src, n);
    b->len += n;
    return CBOR_OK;
}

/* Append `nbytes` of `arg` in big-endian order. */
static CborStatus bb_push_be(ByteBuf *b, uint64_t arg, int nbytes) {
    for (int i = nbytes - 1; i >= 0; i--) {
        CborStatus status = bb_push(b, (uint8_t)((arg >> (8 * i)) & 0xFF));
        if (status != CBOR_OK) return status;
    }
    return CBOR_OK;
}

/* Write a header byte plus its big-endian argument in the shortest form. */
static CborStatus write_type_and_argument(ByteBuf *b, uint8_t major, uint64_t arg) {
    uint8_t mt = (uint8_t)(major << 5);
    if (arg <= 23) return bb_push(b, (uint8_t)(mt | (uint8_t)arg));
    {
        uint8_t info = arg <= 0xFF ? 24 : (arg <= 0xFFFF ? 25 : (arg <= 0xFFFFFFFFu ? 26 : 27));
        int bytes = info == 24 ? 1 : (info == 25 ? 2 : (info == 26 ? 4 : 8));
        CborStatus status = bb_push(b, (uint8_t)(mt | info));
        return status == CBOR_OK ? bb_push_be(b, arg, bytes) : status;
    }
}

static CborStatus encode_into(const CborValue *v, ByteBuf *out, size_t depth);

/* One encoded map key plus a borrowed reference to its value. */
typedef struct {
    ByteBuf key; /* encoded key bytes */
    const CborValue *val;
} EncEntry;

/* Stable length-first-then-bytewise `<` on encoded key blobs. */
static int enc_key_less(const EncEntry *a, const EncEntry *b) {
    if (a->key.len != b->key.len) return a->key.len < b->key.len;
    size_t n = a->key.len;
    int cmp = n == 0 ? 0 : memcmp(a->key.data, b->key.data, n);
    return cmp < 0;
}

/* Encode a map: encode each key, stable-sort length-first, emit. Returns 1 on
 * success, 0 on OOM (all temporary key buffers are freed either way). */
static int enc_key_equal(const EncEntry *a, const EncEntry *b) {
    return a->key.len == b->key.len &&
           (a->key.len == 0 || memcmp(a->key.data, b->key.data, a->key.len) == 0);
}

static CborStatus encode_map(const CborValue *v, ByteBuf *out, size_t depth) {
    size_t n = v->as.map.len;
    EncEntry *ents = NULL;
    if (n > 0) {
        if (n > ((size_t)-1) / sizeof(EncEntry)) return CBOR_ERR_ALLOC;
        ents = (EncEntry *)calloc(n, sizeof(EncEntry));
        if (ents == NULL) return CBOR_ERR_ALLOC;
    }
    CborStatus status = CBOR_OK;
    for (size_t i = 0; i < n && status == CBOR_OK; i++) {
        ents[i].val = v->as.map.entries[i].val;
        status = encode_into(v->as.map.entries[i].key, &ents[i].key, depth + 1);
    }
    /* Insertion sort is stable, matching Rust's stable sort_by. Maps are small,
     * and stable ordering makes adjacent duplicate detection deterministic. */
    if (status == CBOR_OK) {
        for (size_t i = 1; i < n; i++) {
            EncEntry cur = ents[i];
            size_t j = i;
            while (j > 0 && enc_key_less(&cur, &ents[j - 1])) {
                ents[j] = ents[j - 1];
                j--;
            }
            ents[j] = cur;
        }
        for (size_t i = 1; i < n; i++) {
            if (enc_key_equal(&ents[i - 1], &ents[i])) {
                status = CBOR_ERR_DUPLICATE_MAP_KEY;
                break;
            }
        }
        if (status == CBOR_OK)
            status = write_type_and_argument(out, 5, (uint64_t)n);
        for (size_t i = 0; i < n && status == CBOR_OK; i++) {
            status = bb_extend(out, ents[i].key.data, ents[i].key.len);
            if (status == CBOR_OK)
                status = encode_into(ents[i].val, out, depth + 1);
        }
    }
    for (size_t i = 0; i < n; i++) free(ents[i].key.data);
    free(ents);
    return status;
}

static CborStatus encode_into(const CborValue *v, ByteBuf *out, size_t depth) {
    CborStatus status;
    if (depth > CBOR_MAX_ENCODE_DEPTH) return CBOR_ERR_ENCODE_TOO_DEEP;
    switch (v->type) {
        case CBOR_UNSIGNED:
            return write_type_and_argument(out, 0, v->as.u);
        case CBOR_NEGATIVE:
            return write_type_and_argument(out, 1, v->as.u);
        case CBOR_BYTES:
            status = write_type_and_argument(out, 2, (uint64_t)v->as.bytes.len);
            return status == CBOR_OK
                       ? bb_extend(out, v->as.bytes.data, v->as.bytes.len)
                       : status;
        case CBOR_TEXT:
            status = write_type_and_argument(out, 3, (uint64_t)v->as.bytes.len);
            return status == CBOR_OK
                       ? bb_extend(out, v->as.bytes.data, v->as.bytes.len)
                       : status;
        case CBOR_ARRAY:
            status = write_type_and_argument(out, 4, (uint64_t)v->as.array.len);
            for (size_t i = 0; i < v->as.array.len && status == CBOR_OK; i++)
                status = encode_into(v->as.array.items[i], out, depth + 1);
            return status;
        case CBOR_MAP:
            return encode_map(v, out, depth);
        case CBOR_TAG:
            status = write_type_and_argument(out, 6, v->as.tag.number);
            return status == CBOR_OK
                       ? encode_into(v->as.tag.inner, out, depth + 1)
                       : status;
        case CBOR_BOOL:
            return bb_push(out, v->as.boolean ? 0xF5 : 0xF4);
        case CBOR_NULL:
            return bb_push(out, 0xF6);
    }
    return CBOR_ERR_ALLOC;
}

CborStatus cbor_encode(const CborValue *v, uint8_t **out, size_t *out_len) {
    ByteBuf b = {NULL, 0, 0};
    *out = NULL;
    *out_len = 0;
    CborStatus status = encode_into(v, &b, 0);
    if (status != CBOR_OK) {
        free(b.data);
        return status;
    }
    *out = b.data;
    *out_len = b.len;
    return CBOR_OK;
}

/* ── Decoder ──────────────────────────────────────────────────────────────*/

typedef struct {
    const uint8_t *bytes;
    size_t len;
    size_t pos;
} Cursor;

static CborStatus cur_read_u8(Cursor *c, uint8_t *out) {
    if (c->pos >= c->len) return CBOR_ERR_UNEXPECTED_EOF;
    *out = c->bytes[c->pos++];
    return CBOR_OK;
}

/* Read exactly `n` bytes, returning a pointer into the input. Checked add so a
 * hostile `n` cannot wrap pos+n past the bounds check. */
static CborStatus cur_read_n(Cursor *c, size_t n, const uint8_t **out) {
    if (n > ((size_t)-1) - c->pos) return CBOR_ERR_LENGTH_TOO_LARGE;
    size_t end = c->pos + n;
    if (end > c->len) return CBOR_ERR_UNEXPECTED_EOF;
    *out = c->bytes + c->pos;
    c->pos = end;
    return CBOR_OK;
}

static size_t cur_remaining(const Cursor *c) { return c->len - c->pos; }

/* Reject a declared length larger than the remaining input (each declared unit
 * costs at least `min_per_unit` wire bytes) before allocating anything. */
static CborStatus length_within_remaining(uint64_t declared, size_t remaining,
                                          size_t min_per_unit, size_t *out) {
    if (declared > (uint64_t)((size_t)-1)) return CBOR_ERR_LENGTH_TOO_LARGE;
    size_t d = (size_t)declared;
    if (d != 0 && min_per_unit > ((size_t)-1) / d)
        return CBOR_ERR_LENGTH_TOO_LARGE; /* d*min_per_unit overflow */
    if (d * min_per_unit > remaining) return CBOR_ERR_LENGTH_TOO_LARGE;
    *out = d;
    return CBOR_OK;
}

/* Read a header byte + argument, enforcing smallest-form for major types 0..6.
 * Writes major (0..7), info (0..31 as read), and the unsigned argument. */
static CborStatus read_header(Cursor *c, uint8_t *major, uint8_t *info,
                             uint64_t *arg) {
    uint8_t b;
    CborStatus st = cur_read_u8(c, &b);
    if (st != CBOR_OK) return st;
    *major = (uint8_t)(b >> 5);
    *info = (uint8_t)(b & 0x1F);
    int enforce_minimal = (*major != 7);
    if (*info <= 23) {
        *arg = *info;
        return CBOR_OK;
    }
    if (*info == 24) {
        uint8_t v;
        st = cur_read_u8(c, &v);
        if (st != CBOR_OK) return st;
        if (enforce_minimal && v <= 23) return CBOR_ERR_NON_MINIMAL_INTEGER;
        *arg = v;
        return CBOR_OK;
    }
    if (*info == 25 || *info == 26 || *info == 27) {
        int nbytes = *info == 25 ? 2 : (*info == 26 ? 4 : 8);
        const uint8_t *bs;
        st = cur_read_n(c, (size_t)nbytes, &bs);
        if (st != CBOR_OK) return st;
        uint64_t v = 0;
        for (int i = 0; i < nbytes; i++) v = (v << 8) | bs[i];
        uint64_t threshold =
            *info == 25 ? 0xFFu : (*info == 26 ? 0xFFFFu : 0xFFFFFFFFu);
        if (enforce_minimal && v <= threshold)
            return CBOR_ERR_NON_MINIMAL_INTEGER;
        *arg = v;
        return CBOR_OK;
    }
    if (*info <= 30) return CBOR_ERR_RESERVED; /* 28, 29, 30 */
    return CBOR_ERR_INDEFINITE;                /* 31 */
}

/* Minimal UTF-8 validator (same acceptance set as Rust std::str::from_utf8):
 * rejects overlong encodings, surrogates (U+D800..DFFF), and > U+10FFFF. */
static int utf8_valid(const uint8_t *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        uint8_t b0 = s[i];
        if (b0 < 0x80) {
            i += 1;
        } else if (b0 >= 0xC2 && b0 <= 0xDF) {
            if (i + 1 >= n || (s[i + 1] & 0xC0) != 0x80) return 0;
            i += 2;
        } else if (b0 == 0xE0) {
            if (i + 2 >= n || s[i + 1] < 0xA0 || s[i + 1] > 0xBF ||
                (s[i + 2] & 0xC0) != 0x80)
                return 0;
            i += 3;
        } else if (b0 >= 0xE1 && b0 <= 0xEC) {
            if (i + 2 >= n || (s[i + 1] & 0xC0) != 0x80 ||
                (s[i + 2] & 0xC0) != 0x80)
                return 0;
            i += 3;
        } else if (b0 == 0xED) {
            if (i + 2 >= n || s[i + 1] < 0x80 || s[i + 1] > 0x9F ||
                (s[i + 2] & 0xC0) != 0x80)
                return 0; /* excludes surrogates */
            i += 3;
        } else if (b0 >= 0xEE && b0 <= 0xEF) {
            if (i + 2 >= n || (s[i + 1] & 0xC0) != 0x80 ||
                (s[i + 2] & 0xC0) != 0x80)
                return 0;
            i += 3;
        } else if (b0 == 0xF0) {
            if (i + 3 >= n || s[i + 1] < 0x90 || s[i + 1] > 0xBF ||
                (s[i + 2] & 0xC0) != 0x80 || (s[i + 3] & 0xC0) != 0x80)
                return 0;
            i += 4;
        } else if (b0 >= 0xF1 && b0 <= 0xF3) {
            if (i + 3 >= n || (s[i + 1] & 0xC0) != 0x80 ||
                (s[i + 2] & 0xC0) != 0x80 || (s[i + 3] & 0xC0) != 0x80)
                return 0;
            i += 4;
        } else if (b0 == 0xF4) {
            if (i + 3 >= n || s[i + 1] < 0x80 || s[i + 1] > 0x8F ||
                (s[i + 2] & 0xC0) != 0x80 || (s[i + 3] & 0xC0) != 0x80)
                return 0; /* caps at U+10FFFF */
            i += 4;
        } else {
            return 0; /* 0x80..0xC1 and 0xF5..0xFF are never valid leads */
        }
    }
    return 1;
}

static CborStatus read_value(Cursor *c, size_t depth, CborValue **out);

/* Strict length-first-then-bytewise `<` on key encodings; equal => false, so
 * duplicate keys are rejected as non-canonical. */
static int key_strictly_less(const uint8_t *a, size_t alen, const uint8_t *b,
                             size_t blen) {
    if (alen != blen) return alen < blen;
    return alen != 0 && memcmp(a, b, alen) < 0;
}

static CborStatus read_array(Cursor *c, size_t depth, uint64_t arg,
                            CborValue **out) {
    size_t count;
    CborStatus st = length_within_remaining(arg, cur_remaining(c), 1, &count);
    if (st != CBOR_OK) return st;
    CborValue *arr = cbor_array();
    if (arr == NULL) return CBOR_ERR_ALLOC;
    for (size_t i = 0; i < count; i++) {
        CborValue *item;
        st = read_value(c, depth + 1, &item);
        if (st != CBOR_OK) {
            cbor_free(arr);
            return st;
        }
        st = cbor_array_push(arr, item); /* frees item on failure */
        if (st != CBOR_OK) {
            cbor_free(arr);
            return st;
        }
    }
    *out = arr;
    return CBOR_OK;
}

static CborStatus read_map(Cursor *c, size_t depth, uint64_t arg,
                          CborValue **out) {
    size_t count;
    CborStatus st = length_within_remaining(arg, cur_remaining(c), 2, &count);
    if (st != CBOR_OK) return st;
    CborValue *map = cbor_map();
    if (map == NULL) return CBOR_ERR_ALLOC;
    size_t prev_start = 0, prev_end = 0;
    int have_prev = 0;
    for (size_t i = 0; i < count; i++) {
        size_t key_start = c->pos;
        CborValue *k;
        st = read_value(c, depth + 1, &k);
        if (st != CBOR_OK) {
            cbor_free(map);
            return st;
        }
        size_t key_end = c->pos;
        CborValue *val;
        st = read_value(c, depth + 1, &val);
        if (st != CBOR_OK) {
            cbor_free(k);
            cbor_free(map);
            return st;
        }
        if (have_prev &&
            !key_strictly_less(c->bytes + prev_start, prev_end - prev_start,
                               c->bytes + key_start, key_end - key_start)) {
            cbor_free(k);
            cbor_free(val);
            cbor_free(map);
            return CBOR_ERR_NON_CANONICAL_MAP_ORDER;
        }
        prev_start = key_start;
        prev_end = key_end;
        have_prev = 1;
        st = cbor_map_push(map, k, val); /* frees k,val on failure */
        if (st != CBOR_OK) {
            cbor_free(map);
            return st;
        }
    }
    *out = map;
    return CBOR_OK;
}

static CborStatus read_value(Cursor *c, size_t depth, CborValue **out) {
    if (depth > CBOR_MAX_DECODE_DEPTH) return CBOR_ERR_TOO_DEEP;
    uint8_t major, info;
    uint64_t arg;
    CborStatus st = read_header(c, &major, &info, &arg);
    if (st != CBOR_OK) return st;
    switch (major) {
        case 0:
            *out = cbor_unsigned(arg);
            return *out ? CBOR_OK : CBOR_ERR_ALLOC;
        case 1:
            *out = cbor_negative(arg);
            return *out ? CBOR_OK : CBOR_ERR_ALLOC;
        case 2: {
            size_t len;
            st = length_within_remaining(arg, cur_remaining(c), 1, &len);
            if (st != CBOR_OK) return st;
            const uint8_t *s;
            st = cur_read_n(c, len, &s);
            if (st != CBOR_OK) return st;
            *out = cbor_bytes(s, len);
            return *out ? CBOR_OK : CBOR_ERR_ALLOC;
        }
        case 3: {
            size_t len;
            st = length_within_remaining(arg, cur_remaining(c), 1, &len);
            if (st != CBOR_OK) return st;
            const uint8_t *s;
            st = cur_read_n(c, len, &s);
            if (st != CBOR_OK) return st;
            if (!utf8_valid(s, len)) return CBOR_ERR_INVALID_UTF8;
            *out = cbor_text((const char *)s, len);
            return *out ? CBOR_OK : CBOR_ERR_ALLOC;
        }
        case 4:
            return read_array(c, depth, arg, out);
        case 5:
            return read_map(c, depth, arg, out);
        case 6: {
            CborValue *inner;
            st = read_value(c, depth + 1, &inner);
            if (st != CBOR_OK) return st;
            *out = cbor_tag(arg, inner); /* takes ownership of inner */
            return *out ? CBOR_OK : CBOR_ERR_ALLOC;
        }
        case 7:
            switch (info) {
                case 20:
                    *out = cbor_bool(0);
                    return *out ? CBOR_OK : CBOR_ERR_ALLOC;
                case 21:
                    *out = cbor_bool(1);
                    return *out ? CBOR_OK : CBOR_ERR_ALLOC;
                case 22:
                    *out = cbor_null();
                    return *out ? CBOR_OK : CBOR_ERR_ALLOC;
                case 25:
                case 26:
                case 27:
                    return CBOR_ERR_FLOAT_NOT_SUPPORTED;
                default:
                    return CBOR_ERR_UNSUPPORTED_SIMPLE;
            }
        default:
            return CBOR_ERR_UNSUPPORTED_SIMPLE; /* unreachable: major is 3 bits */
    }
}

CborStatus cbor_decode(const uint8_t *bytes, size_t len, CborValue **out) {
    *out = NULL;
    Cursor c = {bytes, len, 0};
    CborValue *v;
    CborStatus st = read_value(&c, 0, &v);
    if (st != CBOR_OK) return st;
    if (c.pos != c.len) {
        cbor_free(v);
        return CBOR_ERR_TRAILING_BYTES;
    }
    *out = v;
    return CBOR_OK;
}

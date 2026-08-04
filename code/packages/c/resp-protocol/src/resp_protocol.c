/*
 * resp_protocol.c — implementation of RESP v2 (see resp_protocol.h). A faithful
 * port of the Rust `resp-protocol` crate (types.rs + encoder.rs + decoder.rs).
 */
#include "resp_protocol.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, memmove, memcmp, strlen, strchr */

/* ===================================================================== *
 *  Small helpers
 * ===================================================================== */

static char *dup_cstr(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    if (p) {
        memcpy(p, s, n);
    }
    return p;
}

static int is_ascii_ws(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f';
}

/* A validating UTF-8 scan (the crate rejects non-UTF-8 in text frames). */
static int utf8_valid(const unsigned char *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        unsigned char c = s[i];
        size_t extra, k;
        unsigned long min_cp, cp;
        if (c < 0x80) {
            i++;
            continue;
        } else if ((c & 0xE0) == 0xC0) {
            extra = 1;
            min_cp = 0x80;
            cp = c & 0x1Fu;
        } else if ((c & 0xF0) == 0xE0) {
            extra = 2;
            min_cp = 0x800;
            cp = c & 0x0Fu;
        } else if ((c & 0xF8) == 0xF0) {
            extra = 3;
            min_cp = 0x10000;
            cp = c & 0x07u;
        } else {
            return 0;
        }
        if (extra >= n - i) {
            return 0; /* truncated sequence */
        }
        for (k = 1; k <= extra; k++) {
            unsigned char cc = s[i + k];
            if ((cc & 0xC0) != 0x80) {
                return 0;
            }
            cp = (cp << 6) | (cc & 0x3Fu);
        }
        if (cp < min_cp || cp > 0x10FFFFuL || (cp >= 0xD800uL && cp <= 0xDFFFuL)) {
            return 0; /* overlong, out of range, or surrogate */
        }
        i += extra + 1;
    }
    return 1;
}

/* Strict signed-decimal parse matching Rust's i64/isize parse: optional sign,
 * at least one digit, digits only, no overflow. */
static int parse_i64(const unsigned char *s, size_t n, long long *out) {
    size_t i = 0;
    int neg = 0;
    unsigned long long acc = 0;
    if (n == 0) {
        return 0;
    }
    if (s[0] == '+' || s[0] == '-') {
        neg = (s[0] == '-');
        i = 1;
        if (i == n) {
            return 0;
        }
    }
    for (; i < n; i++) {
        unsigned int d;
        if (s[i] < '0' || s[i] > '9') {
            return 0;
        }
        d = (unsigned int)(s[i] - '0');
        if (acc > (18446744073709551615ULL - d) / 10ULL) {
            return 0; /* would overflow u64 */
        }
        acc = acc * 10ULL + d;
    }
    if (neg) {
        if (acc > 9223372036854775808ULL) {
            return 0;
        }
        *out = (acc == 9223372036854775808ULL) ? (-9223372036854775807LL - 1)
                                               : -(long long)acc;
    } else {
        if (acc > 9223372036854775807ULL) {
            return 0;
        }
        *out = (long long)acc;
    }
    return 1;
}

/* ===================================================================== *
 *  Value constructors, destructor, equality
 * ===================================================================== */

RespValue *resp_simple_string(const char *s) {
    RespValue *v = malloc(sizeof *v);
    if (!v) {
        return NULL;
    }
    v->type = RESP_SIMPLE_STRING;
    v->as.simple = dup_cstr(s);
    if (!v->as.simple) {
        free(v);
        return NULL;
    }
    return v;
}

RespValue *resp_error(const char *message) {
    RespValue *v = malloc(sizeof *v);
    const char *space;
    if (!v) {
        return NULL;
    }
    v->type = RESP_ERROR;
    v->as.error.message = NULL;
    v->as.error.error_type = NULL;
    v->as.error.detail = NULL;
    v->as.error.message = dup_cstr(message);
    if (!v->as.error.message) {
        resp_free(v);
        return NULL;
    }
    space = strchr(message, ' ');
    if (space) {
        size_t tlen = (size_t)(space - message);
        v->as.error.error_type = malloc(tlen + 1);
        if (!v->as.error.error_type) {
            resp_free(v);
            return NULL;
        }
        memcpy(v->as.error.error_type, message, tlen);
        v->as.error.error_type[tlen] = '\0';
        v->as.error.detail = dup_cstr(space + 1);
    } else {
        v->as.error.error_type = dup_cstr(message);
        v->as.error.detail = dup_cstr("");
    }
    if (!v->as.error.error_type || !v->as.error.detail) {
        resp_free(v);
        return NULL;
    }
    return v;
}

RespValue *resp_integer(long long value) {
    RespValue *v = malloc(sizeof *v);
    if (!v) {
        return NULL;
    }
    v->type = RESP_INTEGER;
    v->as.integer = value;
    return v;
}

RespValue *resp_bulk_string(const unsigned char *data, size_t len) {
    RespValue *v = malloc(sizeof *v);
    if (!v) {
        return NULL;
    }
    v->type = RESP_BULK_STRING;
    v->as.bulk.is_null = 0;
    v->as.bulk.len = len;
    v->as.bulk.data = malloc(len ? len : 1); /* never malloc(0); non-NULL */
    if (!v->as.bulk.data) {
        free(v);
        return NULL;
    }
    if (len) {
        memcpy(v->as.bulk.data, data, len);
    }
    return v;
}

RespValue *resp_bulk_null(void) {
    RespValue *v = malloc(sizeof *v);
    if (!v) {
        return NULL;
    }
    v->type = RESP_BULK_STRING;
    v->as.bulk.is_null = 1;
    v->as.bulk.data = NULL;
    v->as.bulk.len = 0;
    return v;
}

RespValue *resp_array(RespValue **items, size_t count) {
    RespValue *v = malloc(sizeof *v);
    if (!v) {
        size_t i;
        for (i = 0; i < count; i++) {
            resp_free(items[i]);
        }
        free(items);
        return NULL;
    }
    v->type = RESP_ARRAY;
    v->as.array.is_null = 0;
    v->as.array.items = items;
    v->as.array.count = count;
    return v;
}

RespValue *resp_array_null(void) {
    RespValue *v = malloc(sizeof *v);
    if (!v) {
        return NULL;
    }
    v->type = RESP_ARRAY;
    v->as.array.is_null = 1;
    v->as.array.items = NULL;
    v->as.array.count = 0;
    return v;
}

void resp_free(RespValue *v) {
    if (!v) {
        return;
    }
    switch (v->type) {
        case RESP_SIMPLE_STRING:
            free(v->as.simple);
            break;
        case RESP_ERROR:
            free(v->as.error.message);
            free(v->as.error.error_type);
            free(v->as.error.detail);
            break;
        case RESP_INTEGER:
            break;
        case RESP_BULK_STRING:
            free(v->as.bulk.data);
            break;
        case RESP_ARRAY:
            if (v->as.array.items) {
                size_t i;
                for (i = 0; i < v->as.array.count; i++) {
                    resp_free(v->as.array.items[i]);
                }
                free(v->as.array.items);
            }
            break;
    }
    free(v);
}

int resp_equal(const RespValue *a, const RespValue *b) {
    if (a == b) {
        return 1;
    }
    if (!a || !b || a->type != b->type) {
        return 0;
    }
    switch (a->type) {
        case RESP_SIMPLE_STRING:
            return strcmp(a->as.simple, b->as.simple) == 0;
        case RESP_ERROR:
            return strcmp(a->as.error.message, b->as.error.message) == 0;
        case RESP_INTEGER:
            return a->as.integer == b->as.integer;
        case RESP_BULK_STRING:
            if (a->as.bulk.is_null || b->as.bulk.is_null) {
                return a->as.bulk.is_null == b->as.bulk.is_null;
            }
            if (a->as.bulk.len != b->as.bulk.len) {
                return 0;
            }
            return memcmp(a->as.bulk.data, b->as.bulk.data, a->as.bulk.len) == 0;
        case RESP_ARRAY: {
            size_t i;
            if (a->as.array.is_null || b->as.array.is_null) {
                return a->as.array.is_null == b->as.array.is_null;
            }
            if (a->as.array.count != b->as.array.count) {
                return 0;
            }
            for (i = 0; i < a->as.array.count; i++) {
                if (!resp_equal(a->as.array.items[i], b->as.array.items[i])) {
                    return 0;
                }
            }
            return 1;
        }
    }
    return 0;
}

const char *resp_error_type(const RespValue *v) {
    return v->as.error.error_type;
}
const char *resp_error_detail(const RespValue *v) {
    return v->as.error.detail;
}

/* ===================================================================== *
 *  Growable byte buffer (encode) and pointer vector (decode)
 * ===================================================================== */

typedef struct {
    unsigned char *data;
    size_t len, cap;
    int ok;
} ByteBuf;

static void bb_init(ByteBuf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->ok = 1;
}
static void bb_free(ByteBuf *b) {
    free(b->data);
    b->data = NULL;
    b->len = b->cap = 0;
}
static int bb_reserve(ByteBuf *b, size_t extra) {
    size_t need, ncap;
    if (!b->ok) {
        return 0;
    }
    if (extra > SIZE_MAX - b->len) {
        b->ok = 0;
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) {
        return 1;
    }
    ncap = b->cap ? b->cap : 16;
    while (ncap < need) {
        if (ncap > SIZE_MAX / 2) {
            ncap = need;
            break;
        }
        ncap *= 2;
    }
    {
        unsigned char *nd = realloc(b->data, ncap);
        if (!nd) {
            b->ok = 0;
            return 0;
        }
        b->data = nd;
        b->cap = ncap;
    }
    return 1;
}
static void bb_push(ByteBuf *b, unsigned char c) {
    if (bb_reserve(b, 1)) {
        b->data[b->len++] = c;
    }
}
static void bb_append(ByteBuf *b, const unsigned char *s, size_t n) {
    if (n && bb_reserve(b, n)) {
        memcpy(b->data + b->len, s, n);
        b->len += n;
    }
}
static void bb_append_cstr(ByteBuf *b, const char *s) {
    bb_append(b, (const unsigned char *)s, strlen(s));
}
static void bb_append_i64(ByteBuf *b, long long v) {
    char tmp[24];
    int i = 0;
    int neg = (v < 0);
    unsigned long long mag =
        neg ? (unsigned long long)(-(v + 1)) + 1ULL : (unsigned long long)v;
    if (mag == 0) {
        tmp[i++] = '0';
    }
    while (mag) {
        tmp[i++] = (char)('0' + (mag % 10ULL));
        mag /= 10ULL;
    }
    if (neg) {
        tmp[i++] = '-';
    }
    while (i > 0) {
        bb_push(b, (unsigned char)tmp[--i]);
    }
}

typedef struct {
    RespValue **items;
    size_t count, cap;
    int ok;
} PtrVec;

static void pv_init(PtrVec *v) {
    v->items = NULL;
    v->count = 0;
    v->cap = 0;
    v->ok = 1;
}
/* Take ownership of `item`; on any failure free it and latch !ok. */
static void pv_push(PtrVec *v, RespValue *item) {
    if (!v->ok || !item) {
        v->ok = 0;
        resp_free(item);
        return;
    }
    if (v->count == v->cap) {
        size_t ncap = v->cap ? v->cap * 2 : 4;
        RespValue **ni;
        if (v->cap > (SIZE_MAX / sizeof(RespValue *)) / 2) {
            v->ok = 0;
            resp_free(item);
            return;
        }
        ni = realloc(v->items, ncap * sizeof *ni);
        if (!ni) {
            v->ok = 0;
            resp_free(item);
            return;
        }
        v->items = ni;
        v->cap = ncap;
    }
    v->items[v->count++] = item;
}
static void pv_free_all(PtrVec *v) {
    size_t i;
    for (i = 0; i < v->count; i++) {
        resp_free(v->items[i]);
    }
    free(v->items);
    v->items = NULL;
    v->count = v->cap = 0;
}

/* ===================================================================== *
 *  Encoder
 * ===================================================================== */

static void encode_into(const RespValue *v, ByteBuf *b,
                        RespEncodeStatus *status) {
    if (*status != RESP_ENCODE_OK) {
        return;
    }
    switch (v->type) {
        case RESP_SIMPLE_STRING:
            if (strchr(v->as.simple, '\r') || strchr(v->as.simple, '\n')) {
                *status = RESP_ENCODE_ERR_SIMPLE_NEWLINE;
                return;
            }
            bb_push(b, '+');
            bb_append_cstr(b, v->as.simple);
            bb_append_cstr(b, "\r\n");
            break;
        case RESP_ERROR:
            bb_push(b, '-');
            bb_append_cstr(b, v->as.error.message);
            bb_append_cstr(b, "\r\n");
            break;
        case RESP_INTEGER:
            bb_push(b, ':');
            bb_append_i64(b, v->as.integer);
            bb_append_cstr(b, "\r\n");
            break;
        case RESP_BULK_STRING:
            if (v->as.bulk.is_null) {
                bb_append_cstr(b, "$-1\r\n");
            } else {
                bb_push(b, '$');
                bb_append_i64(b, (long long)v->as.bulk.len);
                bb_append_cstr(b, "\r\n");
                bb_append(b, v->as.bulk.data, v->as.bulk.len);
                bb_append_cstr(b, "\r\n");
            }
            break;
        case RESP_ARRAY:
            if (v->as.array.is_null) {
                bb_append_cstr(b, "*-1\r\n");
            } else {
                size_t i;
                bb_push(b, '*');
                bb_append_i64(b, (long long)v->as.array.count);
                bb_append_cstr(b, "\r\n");
                for (i = 0; i < v->as.array.count; i++) {
                    encode_into(v->as.array.items[i], b, status);
                    if (*status != RESP_ENCODE_OK) {
                        return;
                    }
                }
            }
            break;
    }
    if (!b->ok) {
        *status = RESP_ENCODE_ERR_ALLOC;
    }
}

RespEncodeStatus resp_encode(const RespValue *value, unsigned char **out,
                             size_t *out_len) {
    ByteBuf b;
    RespEncodeStatus status = RESP_ENCODE_OK;
    *out = NULL;
    *out_len = 0;
    bb_init(&b);
    encode_into(value, &b, &status);
    if (status == RESP_ENCODE_OK && !b.ok) {
        status = RESP_ENCODE_ERR_ALLOC;
    }
    if (status != RESP_ENCODE_OK) {
        bb_free(&b);
        return status;
    }
    *out = b.data;
    *out_len = b.len;
    return RESP_ENCODE_OK;
}

/* ===================================================================== *
 *  Decoder
 * ===================================================================== */

/* Find the first CRLF in buf[0..len]; on success set *line_len (bytes before
 * CRLF) and *consumed (line_len + 2). */
static int read_line(const unsigned char *buf, size_t len, size_t *line_len,
                     size_t *consumed) {
    size_t i;
    if (len < 2) {
        return 0;
    }
    for (i = 0; i + 1 < len; i++) {
        if (buf[i] == '\r' && buf[i + 1] == '\n') {
            *line_len = i;
            *consumed = i + 2;
            return 1;
        }
    }
    return 0;
}

static RespDecodeStatus decode_one(const unsigned char *buf, size_t len,
                                   RespValue **out, size_t *consumed);

static RespDecodeStatus decode_bulk(const unsigned char *buf, size_t len,
                                    RespValue **out, size_t *consumed) {
    size_t line_len, cons, body_start, body_end, tail_end, blen;
    long long length;
    if (!read_line(buf + 1, len - 1, &line_len, &cons)) {
        return RESP_DECODE_INCOMPLETE;
    }
    if (!utf8_valid(buf + 1, line_len) ||
        !parse_i64(buf + 1, line_len, &length)) {
        return RESP_DECODE_ERROR;
    }
    if (length == -1) {
        *out = resp_bulk_null();
        if (!*out) {
            return RESP_DECODE_ERROR;
        }
        *consumed = cons + 1;
        return RESP_DECODE_OK;
    }
    if (length < -1) {
        return RESP_DECODE_ERROR;
    }
    blen = (size_t)length;
    body_start = 1 + cons;
    if (blen > SIZE_MAX - body_start) {
        return RESP_DECODE_INCOMPLETE; /* cannot possibly fit */
    }
    body_end = body_start + blen;
    if (body_end > SIZE_MAX - 2) {
        return RESP_DECODE_INCOMPLETE;
    }
    tail_end = body_end + 2;
    if (len < tail_end) {
        return RESP_DECODE_INCOMPLETE;
    }
    if (!(buf[body_end] == '\r' && buf[body_end + 1] == '\n')) {
        return RESP_DECODE_ERROR;
    }
    *out = resp_bulk_string(buf + body_start, blen);
    if (!*out) {
        return RESP_DECODE_ERROR;
    }
    *consumed = tail_end;
    return RESP_DECODE_OK;
}

static RespDecodeStatus decode_array(const unsigned char *buf, size_t len,
                                     RespValue **out, size_t *consumed) {
    size_t line_len, cons, offset, i, n;
    long long count;
    PtrVec pv;
    if (!read_line(buf + 1, len - 1, &line_len, &cons)) {
        return RESP_DECODE_INCOMPLETE;
    }
    if (!utf8_valid(buf + 1, line_len) ||
        !parse_i64(buf + 1, line_len, &count)) {
        return RESP_DECODE_ERROR;
    }
    if (count == -1) {
        *out = resp_array_null();
        if (!*out) {
            return RESP_DECODE_ERROR;
        }
        *consumed = cons + 1;
        return RESP_DECODE_OK;
    }
    if (count < -1) {
        return RESP_DECODE_ERROR;
    }
    n = (size_t)count;
    offset = cons + 1;
    /* Grow the children incrementally rather than pre-allocating `n` slots: a
     * hostile header like "*100000000\r\n" must not force a huge allocation
     * before a single child has been shown to exist. */
    pv_init(&pv);
    for (i = 0; i < n; i++) {
        RespValue *child = NULL;
        size_t used;
        RespDecodeStatus st =
            decode_one(buf + offset, len - offset, &child, &used);
        if (st == RESP_DECODE_OK) {
            pv_push(&pv, child); /* takes ownership; latches !ok on failure */
            if (!pv.ok) {
                pv_free_all(&pv);
                return RESP_DECODE_ERROR;
            }
            offset += used;
        } else {
            pv_free_all(&pv);
            return st; /* INCOMPLETE or ERROR, propagated */
        }
    }
    *out = resp_array(pv.items, pv.count); /* takes ownership of the buffer */
    if (!*out) {
        return RESP_DECODE_ERROR;
    }
    *consumed = offset;
    return RESP_DECODE_OK;
}

static RespDecodeStatus decode_inline(const unsigned char *buf, size_t len,
                                      RespValue **out, size_t *consumed) {
    size_t line_len, cons, i = 0;
    PtrVec pv;
    pv_init(&pv);
    if (!read_line(buf, len, &line_len, &cons)) {
        return RESP_DECODE_INCOMPLETE;
    }
    while (i < line_len) {
        size_t start;
        RespValue *tok;
        while (i < line_len && is_ascii_ws(buf[i])) {
            i++;
        }
        if (i >= line_len) {
            break;
        }
        start = i;
        while (i < line_len && !is_ascii_ws(buf[i])) {
            i++;
        }
        tok = resp_bulk_string(buf + start, i - start);
        pv_push(&pv, tok); /* takes ownership; latches !ok on failure */
        if (!pv.ok) {
            pv_free_all(&pv);
            return RESP_DECODE_ERROR;
        }
    }
    *out = resp_array(pv.items, pv.count); /* transfers ownership of pv.items */
    if (!*out) {
        return RESP_DECODE_ERROR;
    }
    *consumed = cons;
    return RESP_DECODE_OK;
}

static RespDecodeStatus decode_one(const unsigned char *buf, size_t len,
                                   RespValue **out, size_t *consumed) {
    unsigned char prefix;
    *out = NULL;
    *consumed = 0;
    if (len == 0) {
        return RESP_DECODE_INCOMPLETE;
    }
    prefix = buf[0];
    if (prefix == '+' || prefix == '-' || prefix == ':') {
        size_t line_len, cons;
        const unsigned char *line = buf + 1;
        if (!read_line(buf + 1, len - 1, &line_len, &cons)) {
            return RESP_DECODE_INCOMPLETE;
        }
        if (!utf8_valid(line, line_len)) {
            return RESP_DECODE_ERROR;
        }
        if (prefix == ':') {
            long long iv;
            if (!parse_i64(line, line_len, &iv)) {
                return RESP_DECODE_ERROR;
            }
            *out = resp_integer(iv);
        } else {
            char *s = malloc(line_len + 1);
            if (!s) {
                return RESP_DECODE_ERROR;
            }
            memcpy(s, line, line_len);
            s[line_len] = '\0';
            *out = (prefix == '+') ? resp_simple_string(s) : resp_error(s);
            free(s);
        }
        if (!*out) {
            return RESP_DECODE_ERROR;
        }
        *consumed = cons + 1;
        return RESP_DECODE_OK;
    }
    if (prefix == '$') {
        return decode_bulk(buf, len, out, consumed);
    }
    if (prefix == '*') {
        return decode_array(buf, len, out, consumed);
    }
    return decode_inline(buf, len, out, consumed);
}

RespDecodeStatus resp_decode(const unsigned char *buffer, size_t len,
                             RespValue **out, size_t *consumed) {
    return decode_one(buffer, len, out, consumed);
}

RespDecodeStatus resp_decode_all(const unsigned char *buffer, size_t len,
                                 RespValue ***out_items, size_t *out_count,
                                 size_t *consumed) {
    PtrVec pv;
    size_t offset = 0;
    pv_init(&pv);
    *out_items = NULL;
    *out_count = 0;
    *consumed = 0;
    while (offset < len) {
        RespValue *v = NULL;
        size_t used;
        RespDecodeStatus st =
            decode_one(buffer + offset, len - offset, &v, &used);
        if (st == RESP_DECODE_OK) {
            pv_push(&pv, v);
            if (!pv.ok) {
                pv_free_all(&pv);
                return RESP_DECODE_ERROR;
            }
            offset += used;
        } else if (st == RESP_DECODE_INCOMPLETE) {
            break;
        } else {
            pv_free_all(&pv);
            return RESP_DECODE_ERROR;
        }
    }
    *out_items = pv.items;
    *out_count = pv.count;
    *consumed = offset;
    return RESP_DECODE_OK;
}

/* ===================================================================== *
 *  Streaming decoder
 * ===================================================================== */

struct RespDecoder {
    ByteBuf buffer; /* accumulated, not-yet-decoded bytes */
    PtrVec queue;   /* decoded messages, read from `head` upward */
    size_t head;
    int error;
};

RespDecoder *resp_decoder_new(void) {
    RespDecoder *d = malloc(sizeof *d);
    if (!d) {
        return NULL;
    }
    bb_init(&d->buffer);
    pv_init(&d->queue);
    d->head = 0;
    d->error = 0;
    return d;
}

void resp_decoder_free(RespDecoder *d) {
    if (!d) {
        return;
    }
    {
        size_t i; /* free only the still-owned (unread) queued messages */
        for (i = d->head; i < d->queue.count; i++) {
            resp_free(d->queue.items[i]);
        }
        free(d->queue.items);
    }
    bb_free(&d->buffer);
    free(d);
}

static void decoder_drain(RespDecoder *d) {
    if (d->error) {
        return;
    }
    for (;;) {
        RespValue *v = NULL;
        size_t used;
        RespDecodeStatus st =
            decode_one(d->buffer.data, d->buffer.len, &v, &used);
        if (st == RESP_DECODE_OK) {
            pv_push(&d->queue, v);
            if (!d->queue.ok) {
                d->error = 1;
                return;
            }
            memmove(d->buffer.data, d->buffer.data + used,
                    d->buffer.len - used);
            d->buffer.len -= used;
        } else if (st == RESP_DECODE_INCOMPLETE) {
            break;
        } else {
            d->error = 1;
            break;
        }
    }
}

void resp_decoder_feed(RespDecoder *d, const unsigned char *data, size_t len) {
    if (!d) {
        return;
    }
    bb_append(&d->buffer, data, len);
    if (!d->buffer.ok) {
        d->error = 1;
        return;
    }
    decoder_drain(d);
}

int resp_decoder_has_message(const RespDecoder *d) {
    return d && (d->head < d->queue.count);
}

int resp_decoder_has_error(const RespDecoder *d) { return d && d->error; }

int resp_decoder_get_message(RespDecoder *d, RespValue **out) {
    *out = NULL;
    if (!d || d->error || d->head >= d->queue.count) {
        return 0;
    }
    *out = d->queue.items[d->head++];
    if (d->head >= d->queue.count) { /* fully drained — compact */
        d->queue.count = 0;
        d->head = 0;
    }
    return 1;
}

int resp_decoder_decode_all(RespDecoder *d, const unsigned char *data,
                            size_t len, RespValue ***out_items,
                            size_t *out_count) {
    size_t avail;
    *out_items = NULL;
    *out_count = 0;
    if (!d) {
        return 0;
    }
    resp_decoder_feed(d, data, len);
    if (d->error) {
        return 0;
    }
    avail = d->queue.count - d->head;
    if (avail > 0) {
        RespValue **arr;
        if (avail > SIZE_MAX / sizeof(RespValue *)) {
            return 0;
        }
        arr = malloc(avail * sizeof *arr);
        if (!arr) {
            return 0;
        }
        memcpy(arr, d->queue.items + d->head, avail * sizeof *arr);
        *out_items = arr;
        *out_count = avail;
    }
    /* ownership of the unread messages transferred out; reset the queue */
    d->queue.count = 0;
    d->head = 0;
    return 1;
}

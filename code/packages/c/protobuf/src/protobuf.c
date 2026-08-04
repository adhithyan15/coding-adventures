/*
 * protobuf.c — implementation of the pure-ISO C protobuf wire-format codec.
 * ========================================================================
 *
 * See protobuf.h for the format overview. The Writer is a growable byte buffer
 * (malloc-owned); the Reader is a cursor that borrows the caller's buffer and
 * never allocates. Every routine mirrors the Rust crate's logic exactly.
 */
#include "protobuf.h"

#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, strlen */

/* ── Errors ────────────────────────────────────────────────────────────────*/

const char *pb_error_message(PbError err) {
    switch (err) {
        case PB_OK: return "ok";
        case PB_ERR_TRUNCATED_VARINT: return "truncated or over-long varint";
        case PB_ERR_UNEXPECTED_EOF: return "unexpected end of protobuf buffer";
        case PB_ERR_UNKNOWN_WIRE_TYPE: return "unknown protobuf wire type";
        case PB_ERR_ZERO_FIELD_NUMBER:
            return "protobuf field number 0 is illegal";
    }
    return "unknown error";
}

/* ── Writer ────────────────────────────────────────────────────────────────*/

void pb_writer_init(PbWriter *w) {
    w->buf = NULL;
    w->len = 0;
    w->cap = 0;
    w->oom = 0;
}

void pb_writer_free(PbWriter *w) {
    free(w->buf);
    w->buf = NULL;
    w->len = 0;
    w->cap = 0;
    w->oom = 0;
}

const uint8_t *pb_writer_bytes(const PbWriter *w) { return w->buf; }
size_t pb_writer_len(const PbWriter *w) { return w->len; }

uint8_t *pb_writer_take(PbWriter *w, size_t *out_len) {
    uint8_t *buf;
    if (w->oom || w->buf == NULL) {
        *out_len = 0;
        return NULL;
    }
    buf = w->buf;
    *out_len = w->len;
    w->buf = NULL;
    w->len = 0;
    w->cap = 0;
    return buf;
}

/* Ensure room for `extra` more bytes; on failure latch `oom`. Growth doubles
 * capacity, capped so the multiply and the `len + extra` sum can't overflow. */
static int pb_reserve(PbWriter *w, size_t extra) {
    size_t need, nc;
    uint8_t *nb;
    if (w->oom) return 0;
    if (extra > (size_t)-1 - w->len) { /* len + extra would overflow */
        w->oom = 1;
        return 0;
    }
    need = w->len + extra;
    if (need <= w->cap) return 1;
    nc = w->cap ? w->cap : 16;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2) {
            nc = need; /* can't double any more; take exactly what's needed */
            break;
        }
        nc *= 2;
    }
    nb = (uint8_t *)realloc(w->buf, nc);
    if (nb == NULL) {
        w->oom = 1;
        return 0;
    }
    w->buf = nb;
    w->cap = nc;
    return 1;
}

static void pb_push(PbWriter *w, uint8_t byte) {
    if (!pb_reserve(w, 1)) return;
    w->buf[w->len++] = byte;
}
static void pb_push_slice(PbWriter *w, const uint8_t *data, size_t len) {
    if (!pb_reserve(w, len)) return;
    if (len > 0) memcpy(w->buf + w->len, data, len);
    w->len += len;
}

void pb_write_varint(PbWriter *w, uint64_t value) {
    /* Emit 7 bits per byte, continuation flag on all but the last. */
    for (;;) {
        uint8_t byte = (uint8_t)(value & 0x7f);
        value >>= 7;
        if (value == 0) {
            pb_push(w, byte);
            break;
        }
        pb_push(w, (uint8_t)(byte | 0x80));
    }
}

static void pb_write_tag(PbWriter *w, uint32_t field, PbWireType wire) {
    pb_write_varint(w, ((uint64_t)field << 3) | (uint64_t)wire);
}

void pb_varint(PbWriter *w, uint32_t field, uint64_t value) {
    pb_write_tag(w, field, PB_WIRE_VARINT);
    pb_write_varint(w, value);
}

void pb_bytes(PbWriter *w, uint32_t field, const uint8_t *value, size_t len) {
    pb_write_tag(w, field, PB_WIRE_LENGTH_DELIMITED);
    pb_write_varint(w, (uint64_t)len);
    pb_push_slice(w, value, len);
}

void pb_string(PbWriter *w, uint32_t field, const char *value) {
    pb_bytes(w, field, (const uint8_t *)value, strlen(value));
}

void pb_message(PbWriter *w, uint32_t field, const uint8_t *encoded,
                size_t len) {
    pb_bytes(w, field, encoded, len);
}

/* Little-endian byte emission (the crate uses `to_le_bytes`). */
void pb_fixed32(PbWriter *w, uint32_t field, uint32_t value) {
    pb_write_tag(w, field, PB_WIRE_FIXED32);
    pb_push(w, (uint8_t)(value & 0xff));
    pb_push(w, (uint8_t)((value >> 8) & 0xff));
    pb_push(w, (uint8_t)((value >> 16) & 0xff));
    pb_push(w, (uint8_t)((value >> 24) & 0xff));
}

void pb_fixed64(PbWriter *w, uint32_t field, uint64_t value) {
    int i;
    pb_write_tag(w, field, PB_WIRE_FIXED64);
    for (i = 0; i < 8; i++) pb_push(w, (uint8_t)((value >> (i * 8)) & 0xff));
}

/* ── Reader ────────────────────────────────────────────────────────────────*/

int pb_value_as_varint(const PbValue *v, uint64_t *out) {
    if (v->kind == PB_WIRE_VARINT) {
        *out = v->varint;
        return 1;
    }
    return 0;
}
int pb_value_as_bytes(const PbValue *v, const uint8_t **out, size_t *out_len) {
    if (v->kind == PB_WIRE_LENGTH_DELIMITED) {
        *out = v->bytes;
        *out_len = v->bytes_len;
        return 1;
    }
    return 0;
}

void pb_reader_init(PbReader *r, const uint8_t *data, size_t len) {
    r->data = data;
    r->len = len;
    r->pos = 0;
}

int pb_reader_is_empty(const PbReader *r) { return r->pos >= r->len; }

static PbError pb_read_varint(PbReader *r, uint64_t *out) {
    uint64_t result = 0;
    int shift;
    /* A u64 needs at most ceil(64/7) = 10 varint bytes; more means overflow. */
    for (shift = 0; shift < 64; shift += 7) {
        uint8_t byte;
        if (r->pos >= r->len) return PB_ERR_TRUNCATED_VARINT;
        byte = r->data[r->pos];
        r->pos += 1;
        result |= (uint64_t)(byte & 0x7f) << shift;
        if ((byte & 0x80) == 0) {
            *out = result;
            return PB_OK;
        }
    }
    return PB_ERR_TRUNCATED_VARINT;
}

/* Borrow `len` bytes at the cursor, advancing past them. */
static PbError pb_read_slice(PbReader *r, size_t len, const uint8_t **out) {
    size_t end;
    if (len > r->len - r->pos) /* r->pos <= r->len always, so no underflow */
        return PB_ERR_UNEXPECTED_EOF;
    end = r->pos + len;
    *out = r->data + r->pos;
    r->pos = end;
    return PB_OK;
}

static uint64_t load_u64_le(const uint8_t *p) {
    uint64_t r = 0;
    int i;
    for (i = 0; i < 8; i++) r |= (uint64_t)p[i] << (i * 8);
    return r;
}
static uint32_t load_u32_le(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
}

PbError pb_reader_next_field(PbReader *r, PbField *out, int *has_field) {
    uint64_t tag;
    uint32_t number;
    uint64_t wire_bits;
    PbError err;

    *has_field = 0;
    if (pb_reader_is_empty(r)) return PB_OK; /* clean end */

    err = pb_read_varint(r, &tag);
    if (err != PB_OK) return err;

    number = (uint32_t)(tag >> 3);
    if (number == 0) return PB_ERR_ZERO_FIELD_NUMBER;

    out->number = number;
    wire_bits = tag & 0x7;
    switch (wire_bits) {
        case 0: { /* Varint */
            uint64_t v;
            err = pb_read_varint(r, &v);
            if (err != PB_OK) return err;
            out->value.kind = PB_WIRE_VARINT;
            out->value.varint = v;
            break;
        }
        case 1: { /* Fixed64 */
            const uint8_t *b;
            err = pb_read_slice(r, 8, &b);
            if (err != PB_OK) return err;
            out->value.kind = PB_WIRE_FIXED64;
            out->value.fixed64 = load_u64_le(b);
            break;
        }
        case 2: { /* LengthDelimited */
            uint64_t vlen;
            const uint8_t *b;
            err = pb_read_varint(r, &vlen);
            if (err != PB_OK) return err;
            /* A length that doesn't fit size_t can't be in the buffer. */
            if (vlen > (uint64_t)(r->len - r->pos)) return PB_ERR_UNEXPECTED_EOF;
            err = pb_read_slice(r, (size_t)vlen, &b);
            if (err != PB_OK) return err;
            out->value.kind = PB_WIRE_LENGTH_DELIMITED;
            out->value.bytes = b;
            out->value.bytes_len = (size_t)vlen;
            break;
        }
        case 5: { /* Fixed32 */
            const uint8_t *b;
            err = pb_read_slice(r, 4, &b);
            if (err != PB_OK) return err;
            out->value.kind = PB_WIRE_FIXED32;
            out->value.fixed32 = load_u32_le(b);
            break;
        }
        default: return PB_ERR_UNKNOWN_WIRE_TYPE;
    }

    *has_field = 1;
    return PB_OK;
}

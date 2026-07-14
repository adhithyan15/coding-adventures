/*
 * sqlite_file.c — implementation of the zero-dependency SQLite-file reader.
 *
 * A faithful C port of the Rust `sqlite-file` crate.  Every routine mirrors its
 * Rust counterpart's control flow and bounds checks; see sqlite_file.h for the
 * surface documentation.  All multi-byte integers on the wire are big-endian.
 *
 * Untrusted input discipline: every read is bounds-checked, every allocation is
 * size-overflow-checked, b-tree walks use an explicit stack (no recursion) with
 * a visited-page bitmap (cycle detection) and a running byte cap (amplification
 * DoS), and every error path frees all intermediates.
 */
#include "sqlite_file.h"

#include <stdlib.h>
#include <string.h>

/* ================================================================== */
/* varint                                                             */
/* ================================================================== */

int sf_varint_read(const uint8_t *buf, size_t len, int64_t *value, size_t *consumed) {
    uint64_t result = 0;
    size_t i;
    for (i = 0; i < 8; ++i) {
        uint8_t byte;
        if (i >= len) {
            return 0;
        }
        byte = buf[i];
        result = (result << 7) | (uint64_t)(byte & 0x7f);
        if ((byte & 0x80) == 0) {
            *value = (int64_t)result;
            *consumed = i + 1;
            return 1;
        }
    }
    if (len < 9) {
        return 0;
    }
    result = (result << 8) | (uint64_t)buf[8];
    *value = (int64_t)result;
    *consumed = 9;
    return 1;
}

size_t sf_varint_write(int64_t value, uint8_t out[9]) {
    uint64_t v = (uint64_t)value;
    size_t len;
    int shift;
    size_t i;
    if (v > 0x00ffffffffffffffULL) {
        size_t idx = 0;
        for (shift = 57; shift >= 8; shift -= 7) {
            out[idx++] = (uint8_t)(0x80 | ((v >> shift) & 0x7f));
        }
        out[idx++] = (uint8_t)(v & 0xff);
        return 9;
    }
    len = 1;
    shift = 7;
    while (shift < 63 && (v >> shift) != 0) {
        ++len;
        shift += 7;
    }
    for (i = len; i-- > 0;) {
        uint8_t group = (uint8_t)((v >> (i * 7)) & 0x7f);
        uint8_t cont = (i == 0) ? 0 : 0x80;
        out[len - 1 - i] = (uint8_t)(cont | group);
    }
    return len;
}

/* ================================================================== */
/* UTF-8 lossy (mirrors Rust String::from_utf8_lossy)                 */
/* ================================================================== */

/* Copy valid UTF-8 verbatim; replace each maximal invalid subpart with U+FFFD
 * (EF BF BD).  Returns a malloc'd buffer and sets *out_len; NULL on OOM. */
static uint8_t *utf8_lossy(const uint8_t *s, size_t len, size_t *out_len) {
    /* Worst case: every byte becomes a 3-byte replacement. Guard the multiply. */
    size_t cap = (len > (size_t)-1 / 3) ? (size_t)-1 : len * 3;
    uint8_t *out = (uint8_t *)malloc(cap == 0 ? 1 : cap);
    size_t o = 0;
    size_t i = 0;
    if (out == NULL) {
        return NULL;
    }
    while (i < len) {
        uint8_t b0 = s[i];
        size_t seq_len;
        unsigned lo, hi;
        if (b0 < 0x80) {
            out[o++] = b0;
            i += 1;
            continue;
        } else if (b0 >= 0xC2 && b0 <= 0xDF) {
            seq_len = 2; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xE0) {
            seq_len = 3; lo = 0xA0; hi = 0xBF;
        } else if (b0 >= 0xE1 && b0 <= 0xEC) {
            seq_len = 3; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xED) {
            seq_len = 3; lo = 0x80; hi = 0x9F;
        } else if (b0 >= 0xEE && b0 <= 0xEF) {
            seq_len = 3; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xF0) {
            seq_len = 4; lo = 0x90; hi = 0xBF;
        } else if (b0 >= 0xF1 && b0 <= 0xF3) {
            seq_len = 4; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xF4) {
            seq_len = 4; lo = 0x80; hi = 0x8F;
        } else {
            out[o++] = 0xEF; out[o++] = 0xBF; out[o++] = 0xBD;
            i += 1;
            continue;
        }
        {
            int ok = 1;
            size_t consumed = 1;
            size_t j;
            for (j = 1; j < seq_len; ++j) {
                unsigned blo = (j == 1) ? lo : 0x80u;
                unsigned bhi = (j == 1) ? hi : 0xBFu;
                uint8_t bj;
                if (i + j >= len) { ok = 0; break; }
                bj = s[i + j];
                if (bj < blo || bj > bhi) { ok = 0; break; }
                ++consumed;
            }
            if (ok) {
                for (j = 0; j < seq_len; ++j) {
                    out[o++] = s[i + j];
                }
                i += seq_len;
            } else {
                out[o++] = 0xEF; out[o++] = 0xBF; out[o++] = 0xBD;
                i += consumed;
            }
        }
    }
    *out_len = o;
    return out;
}

/* ================================================================== */
/* record                                                             */
/* ================================================================== */

void sf_row_free(sf_row_t *row) {
    size_t i;
    if (row == NULL || row->items == NULL) {
        return;
    }
    for (i = 0; i < row->len; ++i) {
        free(row->items[i].bytes);
    }
    free(row->items);
    row->items = NULL;
    row->len = 0;
}

static size_t content_size(uint64_t serial) {
    switch (serial) {
    case 0: case 8: case 9: case 10: case 11: return 0;
    case 1: return 1;
    case 2: return 2;
    case 3: return 3;
    case 4: return 4;
    case 5: return 6;
    case 6: case 7: return 8;
    default: return (size_t)((serial - 12) / 2);
    }
}

static int64_t read_int_be(const uint8_t *bytes, size_t len) {
    uint64_t v = 0;
    size_t i;
    size_t bits;
    for (i = 0; i < len; ++i) {
        v = (v << 8) | (uint64_t)bytes[i];
    }
    bits = len * 8;
    if (bits < 64 && (v & ((uint64_t)1 << (bits - 1))) != 0) {
        v |= ~(((uint64_t)1 << bits) - 1);
    }
    return (int64_t)v;
}

static sf_error_t encode_value_layout(const sf_value_t *value, uint64_t *serial,
                                      size_t *width) {
    switch (value->type) {
    case SF_VAL_NULL:
        *serial = 0;
        *width = 0;
        return SF_OK;
    case SF_VAL_INT: {
        int64_t v = value->int_val;
        if (v == 0) {
            *serial = 8;
            *width = 0;
        } else if (v == 1) {
            *serial = 9;
            *width = 0;
        } else if (v >= -((int64_t)1 << 7) && v < ((int64_t)1 << 7)) {
            *serial = 1;
            *width = 1;
        } else if (v >= -((int64_t)1 << 15) && v < ((int64_t)1 << 15)) {
            *serial = 2;
            *width = 2;
        } else if (v >= -((int64_t)1 << 23) && v < ((int64_t)1 << 23)) {
            *serial = 3;
            *width = 3;
        } else if (v >= -((int64_t)1 << 31) && v < ((int64_t)1 << 31)) {
            *serial = 4;
            *width = 4;
        } else if (v >= -((int64_t)1 << 47) && v < ((int64_t)1 << 47)) {
            *serial = 5;
            *width = 6;
        } else {
            *serial = 6;
            *width = 8;
        }
        return SF_OK;
    }
    case SF_VAL_REAL:
        *serial = 7;
        *width = 8;
        return SF_OK;
    case SF_VAL_TEXT:
        if ((value->bytes == NULL && value->bytes_len != 0) ||
            value->bytes_len > (size_t)(INT64_MAX - 13) / 2) {
            return SF_ERR_CORRUPT;
        }
        *serial = 13 + 2 * (uint64_t)value->bytes_len;
        *width = value->bytes_len;
        return SF_OK;
    case SF_VAL_BLOB:
        if ((value->bytes == NULL && value->bytes_len != 0) ||
            value->bytes_len > (size_t)(INT64_MAX - 12) / 2) {
            return SF_ERR_CORRUPT;
        }
        *serial = 12 + 2 * (uint64_t)value->bytes_len;
        *width = value->bytes_len;
        return SF_OK;
    default:
        return SF_ERR_CORRUPT;
    }
}

static void write_low_bytes_be(uint64_t value, size_t width, uint8_t *out) {
    size_t i;
    for (i = 0; i < width; ++i) {
        out[i] = (uint8_t)(value >> ((width - 1 - i) * 8));
    }
}

sf_error_t sf_record_encode(const sf_value_t *values, size_t count,
                            uint8_t **out_record, size_t *out_len) {
    uint8_t *serials = NULL;
    uint8_t *payload = NULL;
    uint8_t *record = NULL;
    size_t serial_len = 0;
    size_t payload_len = 0;
    size_t payload_off = 0;
    size_t i;
    size_t assumed_header_width = 1;
    uint8_t header_varint[9];
    size_t header_width;
    size_t header_len;
    size_t total_len;

    if (out_record == NULL || out_len == NULL || (values == NULL && count != 0)) {
        return SF_ERR_CORRUPT;
    }
    *out_record = NULL;
    *out_len = 0;
    if (count > (size_t)-1 / 9) {
        return SF_ERR_ALLOC;
    }
    serials = (uint8_t *)malloc(count == 0 ? 1 : count * 9);
    if (serials == NULL) {
        return SF_ERR_ALLOC;
    }

    for (i = 0; i < count; ++i) {
        uint64_t serial;
        size_t width;
        sf_error_t err = encode_value_layout(&values[i], &serial, &width);
        if (err != SF_OK) {
            free(serials);
            return err;
        }
        if (width > (size_t)-1 - payload_len) {
            free(serials);
            return SF_ERR_ALLOC;
        }
        payload_len += width;
        serial_len += sf_varint_write((int64_t)serial, serials + serial_len);
    }

    payload = (uint8_t *)malloc(payload_len == 0 ? 1 : payload_len);
    if (payload == NULL) {
        free(serials);
        return SF_ERR_ALLOC;
    }
    for (i = 0; i < count; ++i) {
        uint64_t serial;
        size_t width;
        (void)encode_value_layout(&values[i], &serial, &width);
        switch (values[i].type) {
        case SF_VAL_INT:
            if (width != 0) {
                write_low_bytes_be((uint64_t)values[i].int_val, width, payload + payload_off);
            }
            break;
        case SF_VAL_REAL: {
            uint64_t bits;
            memcpy(&bits, &values[i].real_val, sizeof bits);
            write_low_bytes_be(bits, 8, payload + payload_off);
            break;
        }
        case SF_VAL_TEXT:
        case SF_VAL_BLOB:
            if (width != 0) {
                memcpy(payload + payload_off, values[i].bytes, width);
            }
            break;
        default:
            break;
        }
        payload_off += width;
    }

    for (;;) {
        if (serial_len > (size_t)INT64_MAX - assumed_header_width) {
            free(serials);
            free(payload);
            return SF_ERR_ALLOC;
        }
        header_len = assumed_header_width + serial_len;
        header_width = sf_varint_write((int64_t)header_len, header_varint);
        if (header_width == assumed_header_width) {
            break;
        }
        assumed_header_width = header_width;
    }
    if (header_len > (size_t)-1 - payload_len) {
        free(serials);
        free(payload);
        return SF_ERR_ALLOC;
    }
    total_len = header_len + payload_len;
    record = (uint8_t *)malloc(total_len == 0 ? 1 : total_len);
    if (record == NULL) {
        free(serials);
        free(payload);
        return SF_ERR_ALLOC;
    }
    memcpy(record, header_varint, header_width);
    memcpy(record + header_width, serials, serial_len);
    memcpy(record + header_len, payload, payload_len);
    free(serials);
    free(payload);
    *out_record = record;
    *out_len = total_len;
    return SF_OK;
}

void sf_record_free(uint8_t *record) { free(record); }

/* Fill *out with the decoded value; returns SF_OK, SF_ERR_CORRUPT, or
 * SF_ERR_ALLOC.  Allocates out->bytes for TEXT/BLOB. */
static sf_error_t decode_value(uint64_t serial, const uint8_t *content, size_t content_len,
                               sf_value_t *out) {
    out->bytes = NULL;
    out->bytes_len = 0;
    out->int_val = 0;
    out->real_val = 0.0;
    switch (serial) {
    case 0:
        out->type = SF_VAL_NULL;
        return SF_OK;
    case 1: case 2: case 3: case 4: case 5: case 6:
        out->type = SF_VAL_INT;
        out->int_val = read_int_be(content, content_len);
        return SF_OK;
    case 7: {
        uint64_t bits = 0;
        double d;
        size_t i;
        if (content_len != 8) {
            return SF_ERR_CORRUPT;
        }
        for (i = 0; i < 8; ++i) {
            bits = (bits << 8) | (uint64_t)content[i];
        }
        memcpy(&d, &bits, sizeof d);
        out->type = SF_VAL_REAL;
        out->real_val = d;
        return SF_OK;
    }
    case 8:
        out->type = SF_VAL_INT;
        out->int_val = 0;
        return SF_OK;
    case 9:
        out->type = SF_VAL_INT;
        out->int_val = 1;
        return SF_OK;
    case 10: case 11:
        return SF_ERR_CORRUPT;
    default:
        if (serial % 2 == 0) {
            uint8_t *b = (uint8_t *)malloc(content_len == 0 ? 1 : content_len);
            if (b == NULL) {
                return SF_ERR_ALLOC;
            }
            if (content_len != 0) {
                memcpy(b, content, content_len);
            }
            out->type = SF_VAL_BLOB;
            out->bytes = b;
            out->bytes_len = content_len;
            return SF_OK;
        } else {
            size_t tlen = 0;
            uint8_t *t = utf8_lossy(content, content_len, &tlen);
            if (t == NULL) {
                return SF_ERR_ALLOC;
            }
            out->type = SF_VAL_TEXT;
            out->bytes = t;
            out->bytes_len = tlen;
            return SF_OK;
        }
    }
}

/* Growable sf_value_t array. */
static sf_error_t values_push(sf_value_t **items, size_t *len, size_t *cap, sf_value_t v) {
    if (*len == *cap) {
        size_t nc = (*cap == 0) ? 4 : *cap * 2;
        sf_value_t *ni;
        if (*cap > (size_t)-1 / 2 / sizeof(sf_value_t)) {
            return SF_ERR_ALLOC;
        }
        ni = (sf_value_t *)realloc(*items, nc * sizeof(sf_value_t));
        if (ni == NULL) {
            return SF_ERR_ALLOC;
        }
        *items = ni;
        *cap = nc;
    }
    (*items)[(*len)++] = v;
    return SF_OK;
}

sf_error_t sf_record_decode(const uint8_t *record, size_t len, sf_row_t *out) {
    int64_t header_len_raw;
    size_t header_off;
    size_t header_len;
    size_t payload_off;
    sf_value_t *items = NULL;
    size_t vlen = 0;
    size_t vcap = 0;

    out->items = NULL;
    out->len = 0;

    if (!sf_varint_read(record, len, &header_len_raw, &header_off)) {
        return SF_ERR_CORRUPT;
    }
    if (header_len_raw < 0) {
        return SF_ERR_CORRUPT;
    }
    header_len = (size_t)header_len_raw;
    if (header_len > len) {
        return SF_ERR_CORRUPT;
    }
    payload_off = header_len;

    while (header_off < header_len) {
        int64_t serial_raw;
        size_t n;
        uint64_t serial;
        size_t size;
        sf_value_t v;
        sf_error_t err;
        if (!sf_varint_read(record + header_off, len - header_off, &serial_raw, &n)) {
            goto corrupt;
        }
        header_off += n;
        if (serial_raw < 0) {
            goto corrupt;
        }
        serial = (uint64_t)serial_raw;
        size = content_size(serial);
        if (size > len - payload_off) { /* overflow-safe: payload_off <= len */
            goto corrupt;
        }
        err = decode_value(serial, record + payload_off, size, &v);
        if (err != SF_OK) {
            /* err is SF_ERR_CORRUPT (reserved serial / bad float) or SF_ERR_ALLOC. */
            sf_row_t tmp;
            tmp.items = items;
            tmp.len = vlen;
            sf_row_free(&tmp);
            return err;
        }
        payload_off += size;
        err = values_push(&items, &vlen, &vcap, v);
        if (err != SF_OK) {
            sf_row_t tmp;
            free(v.bytes);
            tmp.items = items;
            tmp.len = vlen;
            sf_row_free(&tmp);
            return err;
        }
    }

    out->items = items;
    out->len = vlen;
    return SF_OK;

corrupt:
    { sf_row_t tmp; tmp.items = items; tmp.len = vlen; sf_row_free(&tmp); }
    return SF_ERR_CORRUPT;
}

/* ================================================================== */
/* header                                                             */
/* ================================================================== */

static const uint8_t MAGIC[16] = {'S', 'Q', 'L', 'i', 't', 'e', ' ', 'f',
                                  'o', 'r', 'm', 'a', 't', ' ', '3', '\0'};

static uint32_t be_u32(const uint8_t *buf, size_t off) {
    return ((uint32_t)buf[off] << 24) | ((uint32_t)buf[off + 1] << 16) |
           ((uint32_t)buf[off + 2] << 8) | (uint32_t)buf[off + 3];
}

static int is_power_of_two(uint32_t x) { return x != 0 && (x & (x - 1)) == 0; }

sf_error_t sf_header_parse(const uint8_t *buf, size_t len, sf_header_t *out) {
    uint16_t raw;
    uint32_t page_size;
    uint8_t reserved;
    if (len < 100) {
        return SF_ERR_TRUNCATED;
    }
    if (memcmp(buf, MAGIC, 16) != 0) {
        return SF_ERR_BAD_MAGIC;
    }
    raw = (uint16_t)(((uint16_t)buf[16] << 8) | (uint16_t)buf[17]);
    page_size = (raw == 1) ? 65536u : (uint32_t)raw;
    if (page_size < 512 || !is_power_of_two(page_size)) {
        return SF_ERR_BAD_PAGE_SIZE;
    }
    reserved = buf[20];
    if ((uint32_t)reserved >= page_size) {
        return SF_ERR_BAD_PAGE_SIZE;
    }
    switch (be_u32(buf, 56)) {
    case 1: out->text_encoding = SF_UTF8; break;
    case 2: out->text_encoding = SF_UTF16LE; break;
    case 3: out->text_encoding = SF_UTF16BE; break;
    default: return SF_ERR_UNSUPPORTED;
    }
    out->page_size = page_size;
    out->reserved_space = reserved;
    out->page_count = be_u32(buf, 28);
    out->change_counter = be_u32(buf, 24);
    out->freelist_trunk = be_u32(buf, 32);
    out->freelist_count = be_u32(buf, 36);
    out->schema_cookie = be_u32(buf, 40);
    out->schema_format = be_u32(buf, 44);
    return SF_OK;
}

uint32_t sf_header_usable_size(const sf_header_t *h) {
    return h->page_size - (uint32_t)h->reserved_space;
}

/* ================================================================== */
/* pager                                                              */
/* ================================================================== */

sf_error_t sf_pager_open(const uint8_t *data, size_t len, sf_header_t *out_header,
                         sf_pager_t *out_pager) {
    sf_error_t err = sf_header_parse(data, len, out_header);
    if (err != SF_OK) {
        return err;
    }
    out_pager->data = data;
    out_pager->len = len;
    out_pager->page_size = (size_t)out_header->page_size;
    return SF_OK;
}

sf_error_t sf_pager_page(const sf_pager_t *p, uint32_t page_no, const uint8_t **out,
                         size_t *out_len) {
    size_t index;
    size_t start;
    size_t end;
    if (page_no == 0) {
        return SF_ERR_BAD_PAGE_NUMBER;
    }
    index = (size_t)(page_no - 1);
    if (p->page_size != 0 && index > ((size_t)-1) / p->page_size) {
        return SF_ERR_BAD_PAGE_NUMBER;
    }
    start = index * p->page_size;
    if (start > ((size_t)-1) - p->page_size) {
        return SF_ERR_BAD_PAGE_NUMBER;
    }
    end = start + p->page_size;
    if (end > p->len) {
        return SF_ERR_BAD_PAGE_NUMBER;
    }
    *out = p->data + start;
    *out_len = p->page_size;
    return SF_OK;
}

size_t sf_pager_page_count(const sf_pager_t *p) {
    return p->page_size == 0 ? 0 : p->len / p->page_size;
}

/* ================================================================== */
/* btree helpers                                                      */
/* ================================================================== */

static int page_be_u16(const uint8_t *page, size_t page_len, size_t off, uint16_t *out) {
    if (off + 1 >= page_len) {
        return 0;
    }
    *out = (uint16_t)(((uint16_t)page[off] << 8) | (uint16_t)page[off + 1]);
    return 1;
}

static int page_be_u32(const uint8_t *page, size_t page_len, size_t off, uint32_t *out) {
    if (off > page_len || page_len - off < 4) {
        return 0;
    }
    *out = ((uint32_t)page[off] << 24) | ((uint32_t)page[off + 1] << 16) |
           ((uint32_t)page[off + 2] << 8) | (uint32_t)page[off + 3];
    return 1;
}

/* i-th cell offset from the cell-pointer array; SF_ERR_CORRUPT on trouble. */
static sf_error_t cell_pointer(const uint8_t *page, size_t page_len, size_t ptr_array, size_t i,
                               size_t *out) {
    size_t entry;
    uint16_t off;
    if (i > ((size_t)-1 - ptr_array) / 2) {
        return SF_ERR_CORRUPT;
    }
    entry = ptr_array + i * 2;
    if (!page_be_u16(page, page_len, entry, &off)) {
        return SF_ERR_CORRUPT;
    }
    *out = (size_t)off;
    return SF_OK;
}

/* Growable uint32 stack. */
typedef struct { uint32_t *data; size_t len; size_t cap; } u32stack;

static sf_error_t stack_push(u32stack *s, uint32_t v) {
    if (s->len == s->cap) {
        size_t nc = (s->cap == 0) ? 8 : s->cap * 2;
        uint32_t *nd;
        if (s->cap > (size_t)-1 / 2 / sizeof(uint32_t)) {
            return SF_ERR_ALLOC;
        }
        nd = (uint32_t *)realloc(s->data, nc * sizeof(uint32_t));
        if (nd == NULL) {
            return SF_ERR_ALLOC;
        }
        s->data = nd;
        s->cap = nc;
    }
    s->data[s->len++] = v;
    return SF_OK;
}

/* Growable byte buffer for reassembling records. */
typedef struct { uint8_t *data; size_t len; size_t cap; } bytebuf;

static sf_error_t buf_reserve(bytebuf *b, size_t extra) {
    if (extra > (size_t)-1 - b->len) {
        return SF_ERR_ALLOC;
    }
    if (b->len + extra > b->cap) {
        size_t nc = b->cap == 0 ? 64 : b->cap;
        uint8_t *nd;
        while (nc < b->len + extra) {
            if (nc > (size_t)-1 / 2) {
                nc = b->len + extra;
                break;
            }
            nc *= 2;
        }
        nd = (uint8_t *)realloc(b->data, nc == 0 ? 1 : nc);
        if (nd == NULL) {
            return SF_ERR_ALLOC;
        }
        b->data = nd;
        b->cap = nc;
    }
    return SF_OK;
}

static sf_error_t buf_append(bytebuf *b, const uint8_t *src, size_t n) {
    sf_error_t err = buf_reserve(b, n);
    if (err != SF_OK) {
        return err;
    }
    if (n != 0) {
        memcpy(b->data + b->len, src, n);
    }
    b->len += n;
    return SF_OK;
}

/* Follow the overflow-page chain, appending onto `rec` until it holds
 * payload_len bytes.  `visited` is a bitmap sized page_count+1. */
static sf_error_t follow_overflow(const sf_pager_t *p, uint32_t first_page, size_t payload_len,
                                  size_t usable, size_t file_bytes, size_t page_count,
                                  uint8_t *visited, bytebuf *rec) {
    uint32_t next = first_page;
    while (rec->len < payload_len) {
        const uint8_t *page;
        size_t page_len;
        uint32_t next_ptr;
        size_t content_len;
        size_t still_needed;
        size_t take;
        sf_error_t err;
        if (next == 0) {
            return SF_ERR_CORRUPT;
        }
        if (next <= page_count) {
            if (visited[next]) {
                return SF_ERR_CORRUPT;
            }
            visited[next] = 1;
        }
        err = sf_pager_page(p, next, &page, &page_len);
        if (err != SF_OK) {
            return err;
        }
        if (!page_be_u32(page, page_len, 0, &next_ptr)) {
            return SF_ERR_CORRUPT;
        }
        if (usable > page_len || usable < 4) {
            return SF_ERR_CORRUPT;
        }
        content_len = usable - 4;
        still_needed = payload_len - rec->len;
        take = still_needed < content_len ? still_needed : content_len;
        err = buf_append(rec, page + 4, take);
        if (err != SF_OK) {
            return err;
        }
        if (rec->len > file_bytes) {
            return SF_ERR_CORRUPT;
        }
        next = next_ptr;
    }
    return SF_OK;
}

/* Inline split + overflow reassembly for one leaf cell payload → *out (owned). */
static sf_error_t split_and_reassemble(const sf_pager_t *p, const uint8_t *payload,
                                       size_t payload_avail, size_t payload_len, size_t usable,
                                       size_t max_local, size_t file_bytes, size_t page_count,
                                       uint8_t *ov_visited, uint8_t **out, size_t *out_len) {
    *out = NULL;
    if (payload_len <= max_local) {
        uint8_t *b;
        if (payload_len > payload_avail) {
            return SF_ERR_CORRUPT;
        }
        b = (uint8_t *)malloc(payload_len == 0 ? 1 : payload_len);
        if (b == NULL) {
            return SF_ERR_ALLOC;
        }
        if (payload_len != 0) {
            memcpy(b, payload, payload_len);
        }
        *out = b;
        *out_len = payload_len;
        return SF_OK;
    }
    if (payload_len > file_bytes) {
        return SF_ERR_CORRUPT;
    }
    {
        size_t a = usable > 12 ? usable - 12 : 0;
        size_t m = (a * 32) / 255;
        size_t min_local = m > 23 ? m - 23 : 0;
        size_t span;
        size_t k;
        size_t inline_len;
        uint32_t first_overflow;
        bytebuf rec;
        sf_error_t err;
        /* clear the overflow visited bitmap for this record. */
        memset(ov_visited, 0, page_count + 1);
        if (usable < 4 || usable - 4 == 0) {
            return SF_ERR_CORRUPT;
        }
        span = usable - 4;
        k = min_local + (payload_len - min_local) % span;
        inline_len = (k <= max_local) ? k : min_local;
        if (inline_len > payload_avail || payload_avail - inline_len < 4) {
            return SF_ERR_CORRUPT;
        }
        first_overflow = ((uint32_t)payload[inline_len] << 24) |
                         ((uint32_t)payload[inline_len + 1] << 16) |
                         ((uint32_t)payload[inline_len + 2] << 8) |
                         (uint32_t)payload[inline_len + 3];
        rec.data = NULL;
        rec.len = 0;
        rec.cap = 0;
        err = buf_reserve(&rec, payload_len);
        if (err != SF_OK) {
            return err;
        }
        err = buf_append(&rec, payload, inline_len);
        if (err != SF_OK) {
            free(rec.data);
            return err;
        }
        err = follow_overflow(p, first_overflow, payload_len, usable, file_bytes, page_count,
                              ov_visited, &rec);
        if (err != SF_OK) {
            free(rec.data);
            return err;
        }
        *out = rec.data;
        *out_len = rec.len;
        return SF_OK;
    }
}

/* ================================================================== */
/* btree walks                                                        */
/* ================================================================== */

#define LEAF_TABLE 0x0D
#define INTERIOR_TABLE 0x05
#define LEAF_INDEX 0x0A
#define INTERIOR_INDEX 0x02

void sf_table_rows_free(sf_table_rows_t *r) {
    size_t i;
    if (r == NULL || r->rows == NULL) {
        return;
    }
    for (i = 0; i < r->len; ++i) {
        free(r->rows[i].bytes);
    }
    free(r->rows);
    r->rows = NULL;
    r->len = 0;
}

void sf_records_free(sf_records_t *r) {
    size_t i;
    if (r == NULL || r->records == NULL) {
        return;
    }
    for (i = 0; i < r->len; ++i) {
        free(r->records[i].bytes);
    }
    free(r->records);
    r->records = NULL;
    r->len = 0;
}

static sf_error_t table_rows_push(sf_table_rows_t *rows, size_t *cap, int64_t rowid,
                                  uint8_t *bytes, size_t len) {
    if (rows->len == *cap) {
        size_t nc = (*cap == 0) ? 8 : *cap * 2;
        sf_table_row_t *nr;
        if (*cap > (size_t)-1 / 2 / sizeof(sf_table_row_t)) {
            return SF_ERR_ALLOC;
        }
        nr = (sf_table_row_t *)realloc(rows->rows, nc * sizeof(sf_table_row_t));
        if (nr == NULL) {
            return SF_ERR_ALLOC;
        }
        rows->rows = nr;
        *cap = nc;
    }
    rows->rows[rows->len].rowid = rowid;
    rows->rows[rows->len].bytes = bytes;
    rows->rows[rows->len].len = len;
    rows->len++;
    return SF_OK;
}

static int cmp_rows(const void *a, const void *b) {
    int64_t ra = ((const sf_table_row_t *)a)->rowid;
    int64_t rb = ((const sf_table_row_t *)b)->rowid;
    if (ra < rb) return -1;
    if (ra > rb) return 1;
    return 0;
}

sf_error_t sf_walk_table(const sf_pager_t *p, const sf_header_t *h, uint32_t root_page,
                         sf_table_rows_t *out) {
    size_t usable = (size_t)sf_header_usable_size(h);
    size_t max_local = usable > 35 ? usable - 35 : 0;
    size_t page_count = sf_pager_page_count(p);
    size_t file_bytes = page_count * p->page_size;
    size_t emitted = 0;
    sf_error_t err = SF_OK;
    u32stack stack;
    uint8_t *visited = NULL;
    uint8_t *ov_visited = NULL;
    size_t rows_cap = 0;

    out->rows = NULL;
    out->len = 0;
    stack.data = NULL;
    stack.len = 0;
    stack.cap = 0;

    visited = (uint8_t *)calloc(page_count + 1, 1);
    ov_visited = (uint8_t *)malloc(page_count + 1);
    if (visited == NULL || ov_visited == NULL) {
        err = SF_ERR_ALLOC;
        goto cleanup;
    }
    err = stack_push(&stack, root_page);
    if (err != SF_OK) {
        goto cleanup;
    }

    while (stack.len > 0) {
        uint32_t page_no = stack.data[--stack.len];
        const uint8_t *page;
        size_t page_len;
        size_t header_off;
        uint8_t page_type;
        uint16_t cc;
        size_t cell_count;
        size_t i;

        if (page_no <= page_count) {
            if (visited[page_no]) {
                err = SF_ERR_CORRUPT;
                goto cleanup;
            }
            visited[page_no] = 1;
        }
        err = sf_pager_page(p, page_no, &page, &page_len);
        if (err != SF_OK) {
            goto cleanup;
        }
        header_off = (page_no == 1) ? 100 : 0;
        if (header_off >= page_len) {
            err = SF_ERR_TRUNCATED;
            goto cleanup;
        }
        page_type = page[header_off];
        if (!page_be_u16(page, page_len, header_off + 3, &cc)) {
            err = SF_ERR_TRUNCATED;
            goto cleanup;
        }
        cell_count = cc;

        if (page_type == LEAF_TABLE) {
            size_t ptr_array = header_off + 8;
            for (i = 0; i < cell_count; ++i) {
                size_t cell_off;
                const uint8_t *cell;
                size_t cell_avail;
                int64_t payload_len_raw;
                size_t n1;
                int64_t rowid;
                size_t n2;
                size_t poff;
                uint8_t *rec = NULL;
                size_t rec_len = 0;
                err = cell_pointer(page, page_len, ptr_array, i, &cell_off);
                if (err != SF_OK) {
                    goto cleanup;
                }
                if (cell_off > page_len) {
                    err = SF_ERR_CORRUPT;
                    goto cleanup;
                }
                cell = page + cell_off;
                cell_avail = page_len - cell_off;
                if (!sf_varint_read(cell, cell_avail, &payload_len_raw, &n1) ||
                    payload_len_raw < 0) {
                    err = SF_ERR_CORRUPT;
                    goto cleanup;
                }
                if (!sf_varint_read(cell + n1, cell_avail - n1, &rowid, &n2)) {
                    err = SF_ERR_CORRUPT;
                    goto cleanup;
                }
                poff = n1 + n2;
                err = split_and_reassemble(p, cell + poff, cell_avail - poff,
                                           (size_t)payload_len_raw, usable, max_local, file_bytes,
                                           page_count, ov_visited, &rec, &rec_len);
                if (err != SF_OK) {
                    goto cleanup;
                }
                if (rec_len > (size_t)-1 - emitted || emitted + rec_len > file_bytes) {
                    free(rec);
                    err = SF_ERR_CORRUPT;
                    goto cleanup;
                }
                emitted += rec_len;
                err = table_rows_push(out, &rows_cap, rowid, rec, rec_len);
                if (err != SF_OK) {
                    free(rec);
                    goto cleanup;
                }
            }
        } else if (page_type == INTERIOR_TABLE) {
            size_t ptr_array = header_off + 12;
            uint32_t rightmost;
            for (i = 0; i < cell_count; ++i) {
                size_t cell_off;
                uint32_t child;
                err = cell_pointer(page, page_len, ptr_array, i, &cell_off);
                if (err != SF_OK) {
                    goto cleanup;
                }
                if (!page_be_u32(page, page_len, cell_off, &child)) {
                    err = SF_ERR_CORRUPT;
                    goto cleanup;
                }
                err = stack_push(&stack, child);
                if (err != SF_OK) {
                    goto cleanup;
                }
            }
            if (!page_be_u32(page, page_len, header_off + 8, &rightmost)) {
                err = SF_ERR_TRUNCATED;
                goto cleanup;
            }
            err = stack_push(&stack, rightmost);
            if (err != SF_OK) {
                goto cleanup;
            }
        } else {
            err = SF_ERR_CORRUPT;
            goto cleanup;
        }
    }

    if (out->len > 1) {
        qsort(out->rows, out->len, sizeof(sf_table_row_t), cmp_rows);
    }
    err = SF_OK;

cleanup:
    free(stack.data);
    free(visited);
    free(ov_visited);
    if (err != SF_OK) {
        sf_table_rows_free(out);
    }
    return err;
}

static sf_error_t records_push(sf_records_t *recs, size_t *cap, uint8_t *bytes, size_t len) {
    if (recs->len == *cap) {
        size_t nc = (*cap == 0) ? 8 : *cap * 2;
        sf_blob_t *nr;
        if (*cap > (size_t)-1 / 2 / sizeof(sf_blob_t)) {
            return SF_ERR_ALLOC;
        }
        nr = (sf_blob_t *)realloc(recs->records, nc * sizeof(sf_blob_t));
        if (nr == NULL) {
            return SF_ERR_ALLOC;
        }
        recs->records = nr;
        *cap = nc;
    }
    recs->records[recs->len].bytes = bytes;
    recs->records[recs->len].len = len;
    recs->len++;
    return SF_OK;
}

static size_t index_max_local(size_t usable) {
    size_t a = usable > 12 ? usable - 12 : 0;
    size_t v = (a * 64) / 255;
    return v > 23 ? v - 23 : 0;
}

sf_error_t sf_walk_index(const sf_pager_t *p, const sf_header_t *h, uint32_t root_page,
                         sf_records_t *out) {
    size_t usable = (size_t)sf_header_usable_size(h);
    size_t max_local = index_max_local(usable);
    size_t page_count = sf_pager_page_count(p);
    size_t file_bytes = page_count * p->page_size;
    size_t emitted = 0;
    sf_error_t err = SF_OK;
    u32stack stack;
    uint8_t *visited = NULL;
    uint8_t *ov_visited = NULL;
    size_t recs_cap = 0;

    out->records = NULL;
    out->len = 0;
    stack.data = NULL;
    stack.len = 0;
    stack.cap = 0;

    visited = (uint8_t *)calloc(page_count + 1, 1);
    ov_visited = (uint8_t *)malloc(page_count + 1);
    if (visited == NULL || ov_visited == NULL) {
        err = SF_ERR_ALLOC;
        goto cleanup;
    }
    err = stack_push(&stack, root_page);
    if (err != SF_OK) {
        goto cleanup;
    }

    while (stack.len > 0) {
        uint32_t page_no = stack.data[--stack.len];
        const uint8_t *page;
        size_t page_len;
        size_t header_off;
        uint8_t page_type;
        uint16_t cc;
        size_t cell_count;
        size_t ptr_array;
        size_t payload_skip;
        size_t i;

        if (page_no <= page_count) {
            if (visited[page_no]) {
                err = SF_ERR_CORRUPT;
                goto cleanup;
            }
            visited[page_no] = 1;
        }
        err = sf_pager_page(p, page_no, &page, &page_len);
        if (err != SF_OK) {
            goto cleanup;
        }
        header_off = (page_no == 1) ? 100 : 0;
        if (header_off >= page_len) {
            err = SF_ERR_TRUNCATED;
            goto cleanup;
        }
        page_type = page[header_off];
        if (!page_be_u16(page, page_len, header_off + 3, &cc)) {
            err = SF_ERR_TRUNCATED;
            goto cleanup;
        }
        cell_count = cc;

        if (page_type == LEAF_INDEX) {
            ptr_array = header_off + 8;
            payload_skip = 0;
        } else if (page_type == INTERIOR_INDEX) {
            ptr_array = header_off + 12;
            payload_skip = 4;
        } else {
            err = SF_ERR_CORRUPT;
            goto cleanup;
        }

        for (i = 0; i < cell_count; ++i) {
            size_t cell_off;
            const uint8_t *payload;
            size_t avail;
            int64_t payload_len_raw;
            size_t n1;
            uint8_t *rec = NULL;
            size_t rec_len = 0;
            err = cell_pointer(page, page_len, ptr_array, i, &cell_off);
            if (err != SF_OK) {
                goto cleanup;
            }
            if (payload_skip == 4) {
                uint32_t child;
                if (!page_be_u32(page, page_len, cell_off, &child)) {
                    err = SF_ERR_CORRUPT;
                    goto cleanup;
                }
                err = stack_push(&stack, child);
                if (err != SF_OK) {
                    goto cleanup;
                }
            }
            if (cell_off > page_len || page_len - cell_off < payload_skip) {
                err = SF_ERR_CORRUPT;
                goto cleanup;
            }
            payload = page + cell_off + payload_skip;
            avail = page_len - cell_off - payload_skip;
            if (!sf_varint_read(payload, avail, &payload_len_raw, &n1) || payload_len_raw < 0) {
                err = SF_ERR_CORRUPT;
                goto cleanup;
            }
            err = split_and_reassemble(p, payload + n1, avail - n1, (size_t)payload_len_raw, usable,
                                       max_local, file_bytes, page_count, ov_visited, &rec,
                                       &rec_len);
            if (err != SF_OK) {
                goto cleanup;
            }
            if (rec_len > (size_t)-1 - emitted || emitted + rec_len > file_bytes) {
                free(rec);
                err = SF_ERR_CORRUPT;
                goto cleanup;
            }
            emitted += rec_len;
            err = records_push(out, &recs_cap, rec, rec_len);
            if (err != SF_OK) {
                free(rec);
                goto cleanup;
            }
        }

        if (page_type == INTERIOR_INDEX) {
            uint32_t rightmost;
            if (!page_be_u32(page, page_len, header_off + 8, &rightmost)) {
                err = SF_ERR_TRUNCATED;
                goto cleanup;
            }
            err = stack_push(&stack, rightmost);
            if (err != SF_OK) {
                goto cleanup;
            }
        }
    }
    err = SF_OK;

cleanup:
    free(stack.data);
    free(visited);
    free(ov_visited);
    if (err != SF_OK) {
        sf_records_free(out);
    }
    return err;
}

/* ================================================================== */
/* schema                                                             */
/* ================================================================== */

void sf_schema_free(sf_schema_t *s) {
    size_t i;
    if (s == NULL || s->entries == NULL) {
        return;
    }
    for (i = 0; i < s->len; ++i) {
        free(s->entries[i].object_type);
        free(s->entries[i].name);
        free(s->entries[i].table_name);
        free(s->entries[i].sql);
    }
    free(s->entries);
    s->entries = NULL;
    s->len = 0;
}

void sf_named_rows_free(sf_named_rows_t *r) {
    size_t i;
    if (r == NULL || r->rows == NULL) {
        return;
    }
    for (i = 0; i < r->len; ++i) {
        sf_row_free(&r->rows[i].columns);
    }
    free(r->rows);
    r->rows = NULL;
    r->len = 0;
}

void sf_rows_free(sf_rows_t *r) {
    size_t i;
    if (r == NULL || r->rows == NULL) {
        return;
    }
    for (i = 0; i < r->len; ++i) {
        sf_row_free(&r->rows[i]);
    }
    free(r->rows);
    r->rows = NULL;
    r->len = 0;
}

/* Copy a TEXT value's bytes into a fresh NUL-terminated C string. */
static char *dup_text(const sf_value_t *v) {
    char *s = (char *)malloc(v->bytes_len + 1);
    if (s == NULL) {
        return NULL;
    }
    if (v->bytes_len != 0) {
        memcpy(s, v->bytes, v->bytes_len);
    }
    s[v->bytes_len] = '\0';
    return s;
}

static sf_error_t decode_schema_row(const uint8_t *rec, size_t rec_len, sf_schema_entry_t *out) {
    sf_row_t row;
    sf_error_t err;
    out->object_type = NULL;
    out->name = NULL;
    out->table_name = NULL;
    out->has_root_page = 0;
    out->root_page = 0;
    out->has_sql = 0;
    out->sql = NULL;

    err = sf_record_decode(rec, rec_len, &row);
    if (err != SF_OK) {
        return err == SF_ERR_ALLOC ? SF_ERR_ALLOC : SF_ERR_CORRUPT;
    }
    if (row.len != 5) {
        sf_row_free(&row);
        return SF_ERR_CORRUPT;
    }
    if (row.items[0].type != SF_VAL_TEXT || row.items[1].type != SF_VAL_TEXT ||
        row.items[2].type != SF_VAL_TEXT) {
        sf_row_free(&row);
        return SF_ERR_CORRUPT;
    }
    out->object_type = dup_text(&row.items[0]);
    out->name = dup_text(&row.items[1]);
    out->table_name = dup_text(&row.items[2]);
    if (out->object_type == NULL || out->name == NULL || out->table_name == NULL) {
        sf_row_free(&row);
        return SF_ERR_ALLOC;
    }
    /* root_page column */
    if (row.items[3].type == SF_VAL_NULL) {
        out->has_root_page = 0;
    } else if (row.items[3].type == SF_VAL_INT) {
        int64_t n = row.items[3].int_val;
        if (n == 0) {
            out->has_root_page = 0;
        } else if (n < 0 || n > 0xffffffffLL) {
            sf_row_free(&row);
            return SF_ERR_CORRUPT;
        } else {
            out->has_root_page = 1;
            out->root_page = (uint32_t)n;
        }
    } else {
        sf_row_free(&row);
        return SF_ERR_CORRUPT;
    }
    /* sql column */
    if (row.items[4].type == SF_VAL_NULL) {
        out->has_sql = 0;
    } else if (row.items[4].type == SF_VAL_TEXT) {
        out->has_sql = 1;
        out->sql = dup_text(&row.items[4]);
        if (out->sql == NULL) {
            sf_row_free(&row);
            return SF_ERR_ALLOC;
        }
    } else {
        sf_row_free(&row);
        return SF_ERR_CORRUPT;
    }
    sf_row_free(&row);
    return SF_OK;
}

static sf_error_t read_schema_from(const sf_pager_t *p, const sf_header_t *h, sf_schema_t *out) {
    sf_table_rows_t rows;
    sf_error_t err;
    size_t i;
    out->entries = NULL;
    out->len = 0;
    err = sf_walk_table(p, h, 1, &rows);
    if (err != SF_OK) {
        return err;
    }
    if (rows.len != 0) {
        out->entries = (sf_schema_entry_t *)calloc(rows.len, sizeof(sf_schema_entry_t));
        if (out->entries == NULL) {
            sf_table_rows_free(&rows);
            return SF_ERR_ALLOC;
        }
    }
    for (i = 0; i < rows.len; ++i) {
        sf_schema_entry_t e;
        err = decode_schema_row(rows.rows[i].bytes, rows.rows[i].len, &e);
        if (err != SF_OK) {
            sf_table_rows_free(&rows);
            sf_schema_free(out);
            return err;
        }
        out->entries[out->len++] = e;
    }
    sf_table_rows_free(&rows);
    return SF_OK;
}

sf_error_t sf_read_schema(const uint8_t *data, size_t len, sf_schema_t *out) {
    sf_header_t h;
    sf_pager_t p;
    sf_error_t err;
    out->entries = NULL;
    out->len = 0;
    err = sf_pager_open(data, len, &h, &p);
    if (err != SF_OK) {
        return err;
    }
    return read_schema_from(&p, &h, out);
}

static sf_error_t table_root_page_from(const sf_pager_t *p, const sf_header_t *h, const char *name,
                                       uint32_t *out) {
    sf_schema_t schema;
    sf_error_t err;
    size_t i;
    err = read_schema_from(p, h, &schema);
    if (err != SF_OK) {
        return err;
    }
    for (i = 0; i < schema.len; ++i) {
        sf_schema_entry_t *e = &schema.entries[i];
        if (e->object_type != NULL && strcmp(e->object_type, "table") == 0 && e->name != NULL &&
            strcmp(e->name, name) == 0) {
            if (!e->has_root_page) {
                sf_schema_free(&schema);
                return SF_ERR_CORRUPT;
            }
            *out = e->root_page;
            sf_schema_free(&schema);
            return SF_OK;
        }
    }
    sf_schema_free(&schema);
    return SF_ERR_NO_SUCH_TABLE;
}

sf_error_t sf_table_root_page(const uint8_t *data, size_t len, const char *name, uint32_t *out) {
    sf_header_t h;
    sf_pager_t p;
    sf_error_t err = sf_pager_open(data, len, &h, &p);
    if (err != SF_OK) {
        return err;
    }
    return table_root_page_from(&p, &h, name, out);
}

sf_error_t sf_read_table(const uint8_t *data, size_t len, const char *name, sf_named_rows_t *out) {
    sf_header_t h;
    sf_pager_t p;
    sf_error_t err;
    uint32_t root;
    sf_table_rows_t rows;
    size_t i;

    out->rows = NULL;
    out->len = 0;
    err = sf_pager_open(data, len, &h, &p);
    if (err != SF_OK) {
        return err;
    }
    err = table_root_page_from(&p, &h, name, &root);
    if (err != SF_OK) {
        return err;
    }
    err = sf_walk_table(&p, &h, root, &rows);
    if (err != SF_OK) {
        return err;
    }
    if (rows.len != 0) {
        out->rows = (sf_named_row_t *)calloc(rows.len, sizeof(sf_named_row_t));
        if (out->rows == NULL) {
            sf_table_rows_free(&rows);
            return SF_ERR_ALLOC;
        }
    }
    for (i = 0; i < rows.len; ++i) {
        sf_row_t cols;
        err = sf_record_decode(rows.rows[i].bytes, rows.rows[i].len, &cols);
        if (err != SF_OK) {
            sf_table_rows_free(&rows);
            sf_named_rows_free(out);
            return err == SF_ERR_ALLOC ? SF_ERR_ALLOC : SF_ERR_CORRUPT;
        }
        out->rows[out->len].rowid = rows.rows[i].rowid;
        out->rows[out->len].columns = cols;
        out->len++;
    }
    sf_table_rows_free(&rows);
    return SF_OK;
}

sf_error_t sf_read_without_rowid_table(const uint8_t *data, size_t len, const char *name,
                                       sf_rows_t *out) {
    sf_header_t h;
    sf_pager_t p;
    sf_error_t err;
    uint32_t root;
    sf_records_t recs;
    size_t i;

    out->rows = NULL;
    out->len = 0;
    err = sf_pager_open(data, len, &h, &p);
    if (err != SF_OK) {
        return err;
    }
    err = table_root_page_from(&p, &h, name, &root);
    if (err != SF_OK) {
        return err;
    }
    err = sf_walk_index(&p, &h, root, &recs);
    if (err != SF_OK) {
        return err;
    }
    if (recs.len != 0) {
        out->rows = (sf_row_t *)calloc(recs.len, sizeof(sf_row_t));
        if (out->rows == NULL) {
            sf_records_free(&recs);
            return SF_ERR_ALLOC;
        }
    }
    for (i = 0; i < recs.len; ++i) {
        sf_row_t cols;
        err = sf_record_decode(recs.records[i].bytes, recs.records[i].len, &cols);
        if (err != SF_OK) {
            sf_records_free(&recs);
            sf_rows_free(out);
            return err == SF_ERR_ALLOC ? SF_ERR_ALLOC : SF_ERR_CORRUPT;
        }
        out->rows[out->len++] = cols;
    }
    sf_records_free(&recs);
    return SF_OK;
}

/*
 * intel_8008_packager.c — implementation of the pure-ISO C Intel HEX codec.
 * ========================================================================
 *
 * `pak_encode_hex` walks the binary in 16-byte chunks, emitting one data record
 * per chunk followed by the fixed EOF record. `pak_decode_hex` parses records
 * line by line, verifying the leading ':', hex validity, length, and checksum,
 * collecting non-overlapping data segments and requiring the EOF sentinel, then
 * assembles the payload span-checked against the 8008's 16 KB address space.
 */
#include "intel_8008_packager.h"

#include <stdlib.h> /* malloc, realloc, free, calloc */
#include <string.h> /* memcpy */

/* ── Constants ────────────────────────────────────────────────────────────*/

#define BYTES_PER_RECORD 16u
#define RECORD_TYPE_DATA 0x00u
#define RECORD_TYPE_EOF 0x01u
#define MAX_HEX_LINE_LEN 1024u

const char *pak_error_message(PakStatus status) {
    switch (status) {
        case PAK_OK: return "ok";
        case PAK_ERR_EMPTY_BINARY: return "binary must be non-empty";
        case PAK_ERR_ORIGIN_TOO_LARGE: return "origin must be 0-65535";
        case PAK_ERR_IMAGE_OVERFLOW:
            return "image overflows 16-bit address space";
        case PAK_ERR_MISSING_COLON: return "expected ':' at start of record";
        case PAK_ERR_INVALID_HEX: return "invalid hex data";
        case PAK_ERR_RECORD_TOO_SHORT: return "record too short";
        case PAK_ERR_BAD_CHECKSUM: return "checksum mismatch";
        case PAK_ERR_UNSUPPORTED_TYPE: return "unsupported record type";
        case PAK_ERR_IMAGE_TOO_LARGE: return "decoded image too large";
        case PAK_ERR_MISSING_EOF: return "missing EOF record";
        case PAK_ERR_OVERLAP: return "record overlaps another record";
        case PAK_ERR_LINE_TOO_LONG: return "line too long";
        case PAK_ERR_ALLOC: return "out of memory";
    }
    return "unknown error";
}

void pak_decoded_free(PakDecoded *d) {
    if (d == NULL) return;
    free(d->binary);
    d->binary = NULL;
    d->binary_len = 0;
    d->origin = 0;
}

/* Intel HEX checksum: two's complement of the byte-sum, mod 256. */
static uint8_t checksum(const uint8_t *fields, size_t n) {
    uint32_t total = 0;
    for (size_t i = 0; i < n; i++) total += fields[i];
    return (uint8_t)((0x100u - (total % 0x100u)) % 0x100u);
}

/* ── Encode ───────────────────────────────────────────────────────────────*/

/* A growable char buffer for building the output string. */
typedef struct {
    char *data;
    size_t len;
    size_t cap;
} StrBuf;

static int sb_reserve(StrBuf *b, size_t extra) {
    if (extra + 1 > ((size_t)-1) - b->len) return 0; /* +1 for NUL headroom */
    size_t need = b->len + extra + 1;
    if (need <= b->cap) return 1;
    size_t nc = b->cap ? b->cap : 32;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    char *pnew = (char *)realloc(b->data, nc);
    if (pnew == NULL) return 0;
    b->data = pnew;
    b->cap = nc;
    return 1;
}

static int sb_putc(StrBuf *b, char c) {
    if (!sb_reserve(b, 1)) return 0;
    b->data[b->len++] = c;
    return 1;
}

static int sb_puts(StrBuf *b, const char *s) {
    while (*s)
        if (!sb_putc(b, *s++)) return 0;
    return 1;
}

/* Append `byte` as two uppercase hex digits. */
static int sb_hex(StrBuf *b, uint8_t byte) {
    static const char DIG[] = "0123456789ABCDEF";
    return sb_putc(b, DIG[(byte >> 4) & 0xF]) && sb_putc(b, DIG[byte & 0xF]);
}

/* Append a single data record for `chunk[0..chunk_len)` loaded at `address`. */
static int emit_data_record(StrBuf *b, size_t address, const uint8_t *chunk,
                            size_t chunk_len) {
    uint8_t n = (uint8_t)chunk_len;
    uint8_t addr_hi = (uint8_t)((address >> 8) & 0xFF);
    uint8_t addr_lo = (uint8_t)(address & 0xFF);

    /* Checksum over [n, addr_hi, addr_lo, type, data...]; chunk_len <= 16. */
    uint8_t fields[4 + BYTES_PER_RECORD];
    fields[0] = n;
    fields[1] = addr_hi;
    fields[2] = addr_lo;
    fields[3] = RECORD_TYPE_DATA;
    memcpy(fields + 4, chunk, chunk_len);
    uint8_t cs = checksum(fields, 4 + chunk_len);

    if (!sb_putc(b, ':') || !sb_hex(b, n) || !sb_hex(b, addr_hi) ||
        !sb_hex(b, addr_lo) || !sb_hex(b, RECORD_TYPE_DATA))
        return 0;
    for (size_t i = 0; i < chunk_len; i++)
        if (!sb_hex(b, chunk[i])) return 0;
    return sb_hex(b, cs) && sb_putc(b, '\n');
}

PakStatus pak_encode_hex(const uint8_t *binary, size_t len, size_t origin,
                         char **out, size_t *out_len) {
    *out = NULL;
    *out_len = 0;
    if (len == 0) return PAK_ERR_EMPTY_BINARY;
    if (origin > 0xFFFF) return PAK_ERR_ORIGIN_TOO_LARGE;
    /* origin <= 0xFFFF so 0x10000-origin >= 1 (no underflow); len > it means
     * origin+len > 0x10000. */
    if (len > 0x10000 - origin) return PAK_ERR_IMAGE_OVERFLOW;

    StrBuf b = {NULL, 0, 0};
    size_t offset = 0;
    while (offset < len) {
        size_t end = offset + BYTES_PER_RECORD;
        if (end > len) end = len;
        if (!emit_data_record(&b, origin + offset, binary + offset,
                              end - offset)) {
            free(b.data);
            return PAK_ERR_ALLOC;
        }
        offset = end;
    }
    if (!sb_puts(&b, ":00000001FF\n")) {
        free(b.data);
        return PAK_ERR_ALLOC;
    }
    b.data[b.len] = '\0'; /* headroom reserved by sb_reserve */
    *out = b.data;
    *out_len = b.len;
    return PAK_OK;
}

/* ── Decode ───────────────────────────────────────────────────────────────*/

/* One collected data segment (owns its bytes). */
typedef struct {
    size_t address;
    uint8_t *data;
    size_t len;
} Segment;

typedef struct {
    Segment *items;
    size_t len;
    size_t cap;
} SegList;

static void seglist_free(SegList *s) {
    for (size_t i = 0; i < s->len; i++) free(s->items[i].data);
    free(s->items);
    s->items = NULL;
    s->len = 0;
    s->cap = 0;
}

static int seglist_push(SegList *s, size_t address, uint8_t *data, size_t len) {
    if (s->len == s->cap) {
        size_t nc = s->cap ? s->cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(Segment)) return 0;
        nc *= 2;
        Segment *pnew = (Segment *)realloc(s->items, nc * sizeof(Segment));
        if (pnew == NULL) return 0;
        s->items = pnew;
        s->cap = nc;
    }
    s->items[s->len].address = address;
    s->items[s->len].data = data;
    s->items[s->len].len = len;
    s->len++;
    return 1;
}

/* Hex-digit value, or -1 if not a hex character. */
static int hex_val(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/* Parse `slen` hex chars from `s` into `out` (capacity >= slen/2). Writes the
 * decoded byte count via *out_n; returns 0 on success, -1 on odd length /
 * non-hex. */
static int parse_hex_bytes(const char *s, size_t slen, uint8_t *out,
                           size_t *out_n) {
    if (slen % 2 != 0) return -1;
    size_t n = 0;
    for (size_t i = 0; i < slen; i += 2) {
        int hi = hex_val(s[i]);
        int lo = hex_val(s[i + 1]);
        if (hi < 0 || lo < 0) return -1;
        out[n++] = (uint8_t)(hi * 16 + lo);
    }
    *out_n = n;
    return 0;
}

/* True if [a, a+alen) and [b, b+blen) intersect. */
static int ranges_overlap(size_t a, size_t alen, size_t b, size_t blen) {
    size_t a_end = a + alen;
    size_t b_end = b + blen;
    return a < b_end && b < a_end;
}

PakStatus pak_decode_hex(const char *text, PakDecoded *out) {
    out->origin = 0;
    out->binary = NULL;
    out->binary_len = 0;

    SegList segs = {NULL, 0, 0};
    int found_eof = 0;
    PakStatus st = PAK_OK;

    const char *p = text;
    while (*p != '\0') {
        /* Slice out the next line [line, line_end). */
        const char *line = p;
        while (*p != '\0' && *p != '\n') p++;
        const char *line_end = p;
        if (*p == '\n') p++;

        /* Trim leading/trailing ASCII whitespace. */
        while (line < line_end && (unsigned char)*line <= ' ') line++;
        while (line_end > line && (unsigned char)line_end[-1] <= ' ') line_end--;
        size_t line_len = (size_t)(line_end - line);
        if (line_len == 0) continue;

        if (line_len > MAX_HEX_LINE_LEN) { st = PAK_ERR_LINE_TOO_LONG; goto done; }
        if (line[0] != ':') { st = PAK_ERR_MISSING_COLON; goto done; }

        /* Parse the hex body after ':'. body_len <= MAX_HEX_LINE_LEN-1, so
         * rec_stack (MAX_HEX_LINE_LEN/2 bytes) always has room. */
        size_t body_len = line_len - 1;
        uint8_t rec_stack[MAX_HEX_LINE_LEN / 2];
        size_t rec_n = 0;
        if (parse_hex_bytes(line + 1, body_len, rec_stack, &rec_n) != 0) {
            st = PAK_ERR_INVALID_HEX;
            goto done;
        }

        if (rec_n < 5) { st = PAK_ERR_RECORD_TOO_SHORT; goto done; }
        size_t byte_count = rec_stack[0];
        size_t address = ((size_t)rec_stack[1] << 8) | rec_stack[2];
        uint8_t rec_type = rec_stack[3];

        size_t expected = 4 + byte_count + 1;
        if (rec_n < expected) { st = PAK_ERR_RECORD_TOO_SHORT; goto done; }

        uint8_t stored_cs = rec_stack[4 + byte_count];
        uint8_t computed_cs = checksum(rec_stack, 4 + byte_count);
        if (computed_cs != stored_cs) { st = PAK_ERR_BAD_CHECKSUM; goto done; }

        if (rec_type == RECORD_TYPE_EOF) {
            found_eof = 1;
            break;
        }
        if (rec_type != RECORD_TYPE_DATA) {
            st = PAK_ERR_UNSUPPORTED_TYPE;
            goto done;
        }

        /* Reject any overlap or duplicate with an already-collected record. */
        for (size_t i = 0; i < segs.len; i++) {
            if (ranges_overlap(segs.items[i].address, segs.items[i].len, address,
                               byte_count)) {
                st = PAK_ERR_OVERLAP;
                goto done;
            }
        }

        /* Copy the data bytes into an owned segment. */
        uint8_t *data = NULL;
        if (byte_count > 0) {
            data = (uint8_t *)malloc(byte_count);
            if (data == NULL) { st = PAK_ERR_ALLOC; goto done; }
            memcpy(data, rec_stack + 4, byte_count);
        }
        if (!seglist_push(&segs, address, data, byte_count)) {
            free(data);
            st = PAK_ERR_ALLOC;
            goto done;
        }
    }

    if (!found_eof) { st = PAK_ERR_MISSING_EOF; goto done; }

    if (segs.len == 0) { /* EOF-only file -> empty image */
        seglist_free(&segs);
        return PAK_OK;
    }

    /* Compute the image span [origin, end). */
    {
        size_t origin = segs.items[0].address;
        size_t end = 0;
        for (size_t i = 0; i < segs.len; i++) {
            if (segs.items[i].address < origin) origin = segs.items[i].address;
            size_t seg_end = segs.items[i].address + segs.items[i].len;
            if (seg_end > end) end = seg_end;
        }
        size_t span = end > origin ? end - origin : 0;
        if (span > PAK_MAX_IMAGE_SIZE) { st = PAK_ERR_IMAGE_TOO_LARGE; goto done; }

        uint8_t *buffer = NULL;
        if (span > 0) {
            buffer = (uint8_t *)calloc(span, 1);
            if (buffer == NULL) { st = PAK_ERR_ALLOC; goto done; }
            for (size_t i = 0; i < segs.len; i++) {
                size_t start = segs.items[i].address - origin;
                if (segs.items[i].len > 0)
                    memcpy(buffer + start, segs.items[i].data, segs.items[i].len);
            }
        }
        seglist_free(&segs);
        out->origin = origin;
        out->binary = buffer;
        out->binary_len = span;
        return PAK_OK;
    }

done:
    seglist_free(&segs);
    return st;
}

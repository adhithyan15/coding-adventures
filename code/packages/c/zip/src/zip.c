/*
 * zip.c — implementation of ZIP (see zip.h). A faithful port of the Rust
 * `zip` crate's writer/reader design, built directly on `c/deflate` (RFC
 * 1951) for compression rather than re-implementing DEFLATE framing here
 * (see the "NOTE ON REPO-WIDE PRECEDENT" comment in zip.h for why this
 * package is allowed to do that when most sibling `zip` ports cannot).
 */
#include "zip.h"

#include "deflate.h"

#include <stdint.h>  /* uint16_t, uint32_t, uint64_t, SIZE_MAX */
#include <stdlib.h>  /* malloc, realloc, free */
#include <string.h>  /* memcpy, memcmp, strlen, strcmp */

/* =========================================================================
 * Growable byte buffer — the writer accumulates the whole archive (Local
 * Headers + data, then Central Directory, then EOCD) into one of these.
 * Same overflow-guarded doubling-growth pattern as the sibling `c/lzss` and
 * `c/deflate` packages' internal buffers.
 * ========================================================================= */

typedef struct {
    unsigned char *data;
    size_t len, cap;
    int ok; /* 0 once an allocation has failed; further pushes become no-ops */
} ByteBuf;

static void bb_init(ByteBuf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->ok = 1;
}

static int bb_reserve(ByteBuf *b, size_t extra) {
    size_t need, nc;
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
    nc = b->cap ? b->cap : 64;
    while (nc < need) {
        if (nc > SIZE_MAX / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    {
        unsigned char *nd = (unsigned char *)realloc(b->data, nc);
        if (!nd) {
            b->ok = 0;
            return 0;
        }
        b->data = nd;
        b->cap = nc;
    }
    return 1;
}

static void bb_push(ByteBuf *b, unsigned char c) {
    if (bb_reserve(b, 1)) {
        b->data[b->len++] = c;
    }
}

static void bb_push_bytes(ByteBuf *b, const unsigned char *data, size_t len) {
    if (len == 0) {
        return;
    }
    if (bb_reserve(b, len)) {
        memcpy(b->data + b->len, data, len);
        b->len += len;
    }
}

/* All ZIP integers are little-endian on the wire regardless of host order. */
static void bb_push_u16le(ByteBuf *b, uint16_t v) {
    bb_push(b, (unsigned char)(v & 0xFFu));
    bb_push(b, (unsigned char)((v >> 8) & 0xFFu));
}

static void bb_push_u32le(ByteBuf *b, uint32_t v) {
    bb_push(b, (unsigned char)(v & 0xFFu));
    bb_push(b, (unsigned char)((v >> 8) & 0xFFu));
    bb_push(b, (unsigned char)((v >> 16) & 0xFFu));
    bb_push(b, (unsigned char)((v >> 24) & 0xFFu));
}

/* =========================================================================
 * CRC-32 (polynomial 0xEDB88320, reflected)
 * ========================================================================= */

static uint32_t crc_table[256];
static int crc_table_ready = 0;

/* Builds the standard reflected CRC-32 table the first time it's needed. Not
 * thread-safe (matches the rest of this single-threaded test/education
 * codebase); a second call is a fast no-op via crc_table_ready. */
static void crc_table_init(void) {
    unsigned n;
    if (crc_table_ready) {
        return;
    }
    for (n = 0; n < 256; n++) {
        uint32_t c = n;
        int k;
        for (k = 0; k < 8; k++) {
            if (c & 1u) {
                c = 0xEDB88320u ^ (c >> 1);
            } else {
                c = c >> 1;
            }
        }
        crc_table[n] = c;
    }
    crc_table_ready = 1;
}

uint32_t zip_crc32(const unsigned char *data, size_t len, uint32_t initial) {
    uint32_t crc;
    size_t i;
    crc_table_init();
    crc = initial ^ 0xFFFFFFFFu;
    for (i = 0; i < len; i++) {
        crc = crc_table[(crc ^ data[i]) & 0xFFu] ^ (crc >> 8);
    }
    return crc ^ 0xFFFFFFFFu;
}

/* =========================================================================
 * MS-DOS date/time
 * ========================================================================= */

uint32_t zip_dos_datetime(unsigned year, unsigned month, unsigned day,
                          unsigned hour, unsigned minute, unsigned second) {
    unsigned yoff = (year >= 1980u) ? (year - 1980u) : 0u;
    uint16_t t, d;
    if (yoff > 127u) { /* DOS date field's year offset is 7 bits (max 2107) */
        yoff = 127u;
    }
    t = (uint16_t)(((hour & 0x1Fu) << 11) | ((minute & 0x3Fu) << 5) |
                   ((second / 2u) & 0x1Fu));
    d = (uint16_t)(((yoff & 0x7Fu) << 9) | ((month & 0x0Fu) << 5) |
                   (day & 0x1Fu));
    return ((uint32_t)d << 16) | (uint32_t)t;
}

/* =========================================================================
 * Bounds-checked little-endian reads
 * -------------------------------------------------------------------------
 * Every multi-byte field read out of an (untrusted) archive goes through
 * these. `off > len` is checked BEFORE any subtraction, so `len - off` below
 * it can never underflow; there is no addition of `off` at all, so these
 * cannot overflow regardless of how large `off` is.
 * ========================================================================= */

static int read_u16(const unsigned char *data, size_t len, size_t off,
                    uint16_t *out) {
    if (off > len || len - off < 2) {
        return 0;
    }
    *out = (uint16_t)((uint16_t)data[off] | ((uint16_t)data[off + 1] << 8));
    return 1;
}

static int read_u32(const unsigned char *data, size_t len, size_t off,
                    uint32_t *out) {
    if (off > len || len - off < 4) {
        return 0;
    }
    *out = (uint32_t)data[off] | ((uint32_t)data[off + 1] << 8) |
           ((uint32_t)data[off + 2] << 16) | ((uint32_t)data[off + 3] << 24);
    return 1;
}

/* =========================================================================
 * ZipWriter
 * ========================================================================= */

/* Metadata recorded per entry as it's written, used to build the Central
 * Directory in zip_writer_finish. `name` is a malloc'd copy — the caller's
 * `name` pointer is not required to outlive the add_* call. */
typedef struct {
    char *name;
    size_t name_len;
    uint16_t method;
    uint32_t dos_datetime;
    uint32_t crc;
    uint32_t compressed_size;
    uint32_t uncompressed_size;
    uint32_t local_offset;
    uint32_t external_attrs;
} CdRecord;

typedef struct {
    CdRecord *data;
    size_t count, cap;
    int ok;
} CdBuf;

static void cb_init(CdBuf *c) {
    c->data = NULL;
    c->count = 0;
    c->cap = 0;
    c->ok = 1;
}

static int cb_push(CdBuf *c, CdRecord rec) {
    if (!c->ok) {
        return 0;
    }
    if (c->count == c->cap) {
        size_t nc = c->cap ? c->cap * 2 : 8;
        CdRecord *nd;
        if (c->cap > (SIZE_MAX / sizeof(CdRecord)) / 2) {
            c->ok = 0;
            return 0;
        }
        nd = (CdRecord *)realloc(c->data, nc * sizeof *nd);
        if (!nd) {
            c->ok = 0;
            return 0;
        }
        c->data = nd;
        c->cap = nc;
    }
    c->data[c->count++] = rec;
    return 1;
}

static void cb_free(CdBuf *c) {
    size_t i;
    for (i = 0; i < c->count; i++) {
        free(c->data[i].name);
    }
    free(c->data);
}

struct ZipWriter {
    ByteBuf buf; /* accumulates Local Headers + data, then CD + EOCD */
    CdBuf entries;
    int finished;
};

ZipStatus zip_writer_new(ZipWriter **out) {
    ZipWriter *w = (ZipWriter *)malloc(sizeof *w);
    *out = NULL;
    if (!w) {
        return ZIP_ERR_ALLOC;
    }
    bb_init(&w->buf);
    cb_init(&w->entries);
    w->finished = 0;
    *out = w;
    return ZIP_OK;
}

/* add_entry — the shared implementation behind zip_writer_add_file and
 * zip_writer_add_directory: write the Local File Header + data now, and
 * record a CdRecord for the Central Directory that zip_writer_finish()
 * writes later.
 *
 * Auto-compression policy (matches the reference Rust `zip` crate): try
 * DEFLATE when `compress` is set and there is data; use it only if the
 * compressed form is STRICTLY smaller than the original, else fall back to
 * Stored. A DEFLATE_ERR_ALLOC from the compressor also falls back to Stored
 * rather than failing the whole entry — Stored never needs to allocate a
 * second copy of the data (it's appended directly), so it can succeed even
 * when the compressor couldn't.
 */
static ZipStatus add_entry(ZipWriter *w, const char *name,
                           const unsigned char *data, size_t len, int compress,
                           uint32_t unix_mode) {
    size_t name_len;
    uint32_t crc;
    uint16_t method;
    uint16_t version_needed;
    const unsigned char *file_data;
    size_t file_data_len;
    unsigned char *deflate_out = NULL;
    size_t local_offset_val;
    CdRecord rec;

    if (!w || w->finished) {
        return ZIP_ERR_MALFORMED; /* misuse: writing after finish() */
    }
    if (!w->buf.ok || !w->entries.ok) {
        return ZIP_ERR_ALLOC;
    }
    if (w->entries.count >= ZIP_MAX_ENTRIES) {
        return ZIP_ERR_TOO_LARGE;
    }
    /* This package implements the core (non-ZIP64) format: every 32-bit
     * size/offset field must fit as-is. Reject rather than silently produce
     * a corrupt archive by truncating a >4GB entry or a >4GB running offset. */
    if (len > 0xFFFFFFFFu || w->buf.len > 0xFFFFFFFFu) {
        return ZIP_ERR_TOO_LARGE;
    }

    name_len = strlen(name);
    if (name_len > 0xFFFFu) {
        return ZIP_ERR_TOO_LARGE; /* name_len is a 16-bit field */
    }

    crc = zip_crc32(data, len, 0);
    method = 0;
    file_data = data;
    file_data_len = len;

    if (compress && len > 0) {
        unsigned char *comp = NULL;
        size_t comp_len = 0;
        DeflateStatus ds = deflate_compress(data, len, &comp, &comp_len);
        if (ds == DEFLATE_OK && comp_len < len) {
            method = 8;
            file_data = comp;
            file_data_len = comp_len;
            deflate_out = comp; /* freed below, after it's copied into buf */
        } else if (ds == DEFLATE_OK) {
            deflate_free(comp); /* compressing didn't help; use Stored */
        }
        /* ds == DEFLATE_ERR_ALLOC: silently fall back to Stored (see doc
         * comment above); comp/comp_len are already NULL/0 in that case. */
    }

    version_needed = (method == 8) ? 20 : 10;
    local_offset_val = w->buf.len;

    /* ---- Local File Header (30 + name_len + 0 bytes of extra) ---- */
    bb_push_u32le(&w->buf, 0x04034B50u);
    bb_push_u16le(&w->buf, version_needed);
    bb_push_u16le(&w->buf, 0x0800u); /* GP flag bit 11: UTF-8 filename */
    bb_push_u16le(&w->buf, method);
    bb_push_u16le(&w->buf, (uint16_t)(ZIP_DOS_EPOCH & 0xFFFFu));
    bb_push_u16le(&w->buf, (uint16_t)((ZIP_DOS_EPOCH >> 16) & 0xFFFFu));
    bb_push_u32le(&w->buf, crc);
    bb_push_u32le(&w->buf, (uint32_t)file_data_len);
    bb_push_u32le(&w->buf, (uint32_t)len);
    bb_push_u16le(&w->buf, (uint16_t)name_len);
    bb_push_u16le(&w->buf, 0); /* extra_len */
    bb_push_bytes(&w->buf, (const unsigned char *)name, name_len);
    bb_push_bytes(&w->buf, file_data, file_data_len);

    if (deflate_out) {
        deflate_free(deflate_out);
    }

    if (!w->buf.ok) {
        return ZIP_ERR_ALLOC;
    }

    /* ---- Record for the Central Directory (written in finish()) ---- */
    rec.name = (char *)malloc(name_len + 1u);
    if (!rec.name) {
        return ZIP_ERR_ALLOC;
    }
    if (name_len > 0) {
        memcpy(rec.name, name, name_len);
    }
    rec.name[name_len] = '\0';
    rec.name_len = name_len;
    rec.method = method;
    rec.dos_datetime = (uint32_t)ZIP_DOS_EPOCH;
    rec.crc = crc;
    rec.compressed_size = (uint32_t)file_data_len;
    rec.uncompressed_size = (uint32_t)len;
    rec.local_offset = (uint32_t)local_offset_val;
    rec.external_attrs = unix_mode << 16;

    if (!cb_push(&w->entries, rec)) {
        free(rec.name);
        return ZIP_ERR_ALLOC;
    }
    return ZIP_OK;
}

ZipStatus zip_writer_add_file(ZipWriter *w, const char *name,
                              const unsigned char *data, size_t len,
                              int compress) {
    return add_entry(w, name, data, len, compress, 0100644u);
}

ZipStatus zip_writer_add_directory(ZipWriter *w, const char *name) {
    /* Directories carry no data; "" is a valid non-NULL zero-length source
     * (never dereferenced, since add_entry only reads `data` when len > 0). */
    return add_entry(w, name, (const unsigned char *)"", 0, 0, 0040755u);
}

ZipStatus zip_writer_finish(ZipWriter *w, unsigned char **out, size_t *out_len) {
    size_t cd_start, i;
    uint32_t cd_offset, cd_size;
    uint16_t num_entries;

    *out = NULL;
    *out_len = 0;
    if (!w) {
        return ZIP_ERR_MALFORMED;
    }
    if (w->finished) {
        return ZIP_ERR_MALFORMED; /* misuse: finish() called twice */
    }
    if (!w->buf.ok || !w->entries.ok) {
        return ZIP_ERR_ALLOC;
    }
    if (w->buf.len > 0xFFFFFFFFu) {
        return ZIP_ERR_TOO_LARGE; /* cd_offset must fit in u32 */
    }
    cd_offset = (uint32_t)w->buf.len;
    cd_start = w->buf.len;

    /* ---- Central Directory: one 46-byte(+name) header per entry ---- */
    for (i = 0; i < w->entries.count; i++) {
        CdRecord *e = &w->entries.data[i];
        uint16_t version_needed = (e->method == 8) ? 20 : 10;
        bb_push_u32le(&w->buf, 0x02014B50u);
        bb_push_u16le(&w->buf, 0x031Eu); /* version_made_by: Unix, v30 (4.5) */
        bb_push_u16le(&w->buf, version_needed);
        bb_push_u16le(&w->buf, 0x0800u); /* GP flag: UTF-8 filename */
        bb_push_u16le(&w->buf, e->method);
        bb_push_u16le(&w->buf, (uint16_t)(e->dos_datetime & 0xFFFFu));
        bb_push_u16le(&w->buf, (uint16_t)((e->dos_datetime >> 16) & 0xFFFFu));
        bb_push_u32le(&w->buf, e->crc);
        bb_push_u32le(&w->buf, e->compressed_size);
        bb_push_u32le(&w->buf, e->uncompressed_size);
        bb_push_u16le(&w->buf, (uint16_t)e->name_len);
        bb_push_u16le(&w->buf, 0); /* extra_len */
        bb_push_u16le(&w->buf, 0); /* comment_len */
        bb_push_u16le(&w->buf, 0); /* disk_start */
        bb_push_u16le(&w->buf, 0); /* internal_attrs */
        bb_push_u32le(&w->buf, e->external_attrs);
        bb_push_u32le(&w->buf, e->local_offset);
        bb_push_bytes(&w->buf, (const unsigned char *)e->name, e->name_len);
    }
    if (!w->buf.ok) {
        return ZIP_ERR_ALLOC;
    }
    if (w->buf.len - cd_start > 0xFFFFFFFFu) {
        return ZIP_ERR_TOO_LARGE; /* cd_size must fit in u32 */
    }
    cd_size = (uint32_t)(w->buf.len - cd_start);
    /* entries.count is already capped below ZIP_MAX_ENTRIES (65535) in
     * add_entry, so this narrowing to u16 is exact, never a truncation. */
    num_entries = (uint16_t)w->entries.count;

    /* ---- End of Central Directory Record (fixed 22 bytes) ---- */
    bb_push_u32le(&w->buf, 0x06054B50u);
    bb_push_u16le(&w->buf, 0); /* disk_number */
    bb_push_u16le(&w->buf, 0); /* cd_disk */
    bb_push_u16le(&w->buf, num_entries);
    bb_push_u16le(&w->buf, num_entries);
    bb_push_u32le(&w->buf, cd_size);
    bb_push_u32le(&w->buf, cd_offset);
    bb_push_u16le(&w->buf, 0); /* comment_len */
    if (!w->buf.ok) {
        return ZIP_ERR_ALLOC;
    }

    *out = w->buf.data;
    *out_len = w->buf.len;
    /* Ownership of the buffer transferred to the caller; leave the writer's
     * copy empty so zip_writer_free() below won't double-free it. */
    w->buf.data = NULL;
    w->buf.len = 0;
    w->buf.cap = 0;
    w->finished = 1;
    return ZIP_OK;
}

void zip_writer_free(ZipWriter *w) {
    if (!w) {
        return;
    }
    free(w->buf.data); /* NULL if finish() already transferred ownership */
    cb_free(&w->entries);
    free(w);
}

/* =========================================================================
 * ZipReader
 * ========================================================================= */

typedef struct {
    ZipEntry *data;
    size_t count, cap;
    int ok;
} EntryBuf;

static void eb_init(EntryBuf *e) {
    e->data = NULL;
    e->count = 0;
    e->cap = 0;
    e->ok = 1;
}

static int eb_push(EntryBuf *e, ZipEntry entry) {
    if (!e->ok) {
        return 0;
    }
    if (e->count == e->cap) {
        size_t nc = e->cap ? e->cap * 2 : 8;
        ZipEntry *nd;
        if (e->cap > (SIZE_MAX / sizeof(ZipEntry)) / 2) {
            e->ok = 0;
            return 0;
        }
        nd = (ZipEntry *)realloc(e->data, nc * sizeof *nd);
        if (!nd) {
            e->ok = 0;
            return 0;
        }
        e->data = nd;
        e->cap = nc;
    }
    e->data[e->count++] = entry;
    return 1;
}

static void eb_free(EntryBuf *e) {
    size_t i;
    for (i = 0; i < e->count; i++) {
        free(e->data[i].name);
    }
    free(e->data);
}

struct ZipReader {
    const unsigned char *data; /* BORROWED — caller must outlive the reader */
    size_t len;
    ZipEntry *entries;
    size_t entry_count;
    size_t max_total_uncompressed; /* aggregate decompression-bomb budget */
    size_t total_uncompressed;     /* running total handed back so far */
};

/* find_eocd — scan backward from the end of `data` for the EOCD signature
 * 0x06054B50, bounded to the last (22 + 65535) bytes (the maximum possible
 * comment length) — never an unbounded scan over attacker-controlled input.
 * A hit is only accepted if its declared comment_len exactly explains the
 * remaining bytes to EOF, the same disambiguation the reference Rust reader
 * uses to reject a signature that merely happens to appear inside file data
 * or a comment. */
static int find_eocd(const unsigned char *data, size_t len, size_t *out_pos) {
    const uint32_t sig_wanted = 0x06054B50u;
    const size_t eocd_min = 22;
    const size_t max_comment = 65535;
    size_t scan_start, i;

    if (len < eocd_min) {
        return 0;
    }
    scan_start = (len > eocd_min + max_comment) ? (len - eocd_min - max_comment)
                                                 : 0;
    i = len - eocd_min;
    for (;;) {
        uint32_t sig;
        if (read_u32(data, len, i, &sig) && sig == sig_wanted) {
            uint16_t comment_len;
            if (read_u16(data, len, i + 20, &comment_len) &&
                i + eocd_min + (size_t)comment_len == len) {
                *out_pos = i;
                return 1;
            }
        }
        if (i <= scan_start) {
            break;
        }
        i--;
    }
    return 0;
}

ZipStatus zip_reader_new_with_budget(const unsigned char *data, size_t len,
                                     size_t max_total_uncompressed,
                                     ZipReader **out) {
    size_t eocd_pos;
    uint32_t cd_offset32, cd_size32;
    uint64_t data_len64, cd_offset64, cd_size64, cd_end64;
    size_t cd_offset, cd_end, pos;
    EntryBuf entries;
    unsigned count = 0;
    ZipReader *r;

    *out = NULL;
    if (!find_eocd(data, len, &eocd_pos)) {
        return ZIP_ERR_MALFORMED;
    }
    if (!read_u32(data, len, eocd_pos + 16, &cd_offset32) ||
        !read_u32(data, len, eocd_pos + 12, &cd_size32)) {
        return ZIP_ERR_MALFORMED;
    }

    /* Widen BEFORE combining untrusted 32-bit offset/size fields, so a
     * maliciously large pair (e.g. both near UINT32_MAX) cannot wrap a
     * 32-bit size_t and slip past the bounds check below. uint32 + uint32
     * always fits in uint64 without overflow. */
    data_len64 = (uint64_t)len;
    cd_offset64 = (uint64_t)cd_offset32;
    cd_size64 = (uint64_t)cd_size32;
    cd_end64 = cd_offset64 + cd_size64;
    if (cd_offset64 > data_len64 || cd_end64 > data_len64) {
        return ZIP_ERR_MALFORMED;
    }
    cd_offset = (size_t)cd_offset64; /* safe: <= data_len64 <= len <= SIZE_MAX */
    cd_end = (size_t)cd_end64;

    eb_init(&entries);
    pos = cd_offset;
    while (pos + 4 <= cd_end && count < ZIP_MAX_ENTRIES) {
        uint32_t sig, crc, comp_size, uncomp_size, local_offset;
        uint16_t method, name_len, extra_len, comment_len;
        uint64_t name_start64, name_end64, next_pos64;
        size_t name_start;
        ZipEntry entry;

        if (!read_u32(data, len, pos, &sig) || sig != 0x02014B50u) {
            break; /* end of CD (or trailing padding) — not an error */
        }
        if (pos + 46 > cd_end) {
            eb_free(&entries);
            return ZIP_ERR_MALFORMED; /* fixed-size header doesn't fit */
        }
        if (!read_u16(data, len, pos + 10, &method) ||
            !read_u32(data, len, pos + 16, &crc) ||
            !read_u32(data, len, pos + 20, &comp_size) ||
            !read_u32(data, len, pos + 24, &uncomp_size) ||
            !read_u16(data, len, pos + 28, &name_len) ||
            !read_u16(data, len, pos + 30, &extra_len) ||
            !read_u16(data, len, pos + 32, &comment_len) ||
            !read_u32(data, len, pos + 42, &local_offset)) {
            eb_free(&entries);
            return ZIP_ERR_MALFORMED;
        }

        name_start64 = (uint64_t)pos + 46u;
        name_end64 = name_start64 + (uint64_t)name_len;
        if (name_end64 > data_len64) {
            eb_free(&entries);
            return ZIP_ERR_MALFORMED; /* name runs past end of archive */
        }
        name_start = (size_t)name_start64;

        entry.name = (char *)malloc((size_t)name_len + 1u);
        if (!entry.name) {
            eb_free(&entries);
            return ZIP_ERR_ALLOC;
        }
        if (name_len > 0) {
            memcpy(entry.name, data + name_start, name_len);
        }
        entry.name[name_len] = '\0';
        entry.name_len = name_len;
        entry.size = uncomp_size;
        entry.compressed_size = comp_size;
        entry.method = method;
        entry.crc32 = crc;
        entry.is_directory = (name_len > 0 && entry.name[name_len - 1] == '/');
        entry.local_offset = local_offset;

        if (!eb_push(&entries, entry)) {
            free(entry.name);
            eb_free(&entries);
            return ZIP_ERR_ALLOC;
        }

        next_pos64 = name_end64 + (uint64_t)extra_len + (uint64_t)comment_len;
        if (next_pos64 > data_len64) {
            eb_free(&entries);
            return ZIP_ERR_MALFORMED;
        }
        pos = (size_t)next_pos64;
        count++;
    }

    r = (ZipReader *)malloc(sizeof *r);
    if (!r) {
        eb_free(&entries);
        return ZIP_ERR_ALLOC;
    }
    r->data = data;
    r->len = len;
    r->entries = entries.data;
    r->entry_count = entries.count;
    r->max_total_uncompressed = max_total_uncompressed;
    r->total_uncompressed = 0;
    *out = r;
    return ZIP_OK;
}

ZipStatus zip_reader_new(const unsigned char *data, size_t len,
                         ZipReader **out) {
    return zip_reader_new_with_budget(data, len,
                                      ZIP_DEFAULT_MAX_TOTAL_UNCOMPRESSED, out);
}

size_t zip_reader_entry_count(const ZipReader *r) {
    return r ? r->entry_count : 0;
}

const ZipEntry *zip_reader_entry(const ZipReader *r, size_t index) {
    if (!r || index >= r->entry_count) {
        return NULL;
    }
    return &r->entries[index];
}

ZipStatus zip_reader_read(ZipReader *r, const ZipEntry *entry,
                          unsigned char **out, size_t *out_len) {
    size_t lh_off;
    uint16_t flags, lh_name_len, lh_extra_len;
    uint64_t data_start64, data_end64;
    size_t data_start, data_end, compressed_len;
    const unsigned char *compressed;
    unsigned char *decompressed = NULL;
    size_t decompressed_len = 0;

    *out = NULL;
    *out_len = 0;
    if (!r || !entry) {
        return ZIP_ERR_MALFORMED;
    }
    if (entry->is_directory) {
        return ZIP_OK; /* no data; out/out_len already NULL/0 */
    }

    /* Aggregate decompression-bomb budget, checked against the DECLARED size
     * before spending any CPU decompressing. Actual output is trimmed to
     * this size below (never expanded past it), so the declared size is a
     * valid upper bound for this check. See zip.h "Security". */
    {
        uint64_t projected =
            (uint64_t)r->total_uncompressed + (uint64_t)entry->size;
        if (projected > (uint64_t)r->max_total_uncompressed) {
            return ZIP_ERR_TOO_LARGE;
        }
    }

    lh_off = (size_t)entry->local_offset;
    if (lh_off > r->len || r->len - lh_off < 30) {
        return ZIP_ERR_MALFORMED;
    }
    if (!read_u16(r->data, r->len, lh_off + 6, &flags)) {
        return ZIP_ERR_MALFORMED;
    }
    if (flags & 0x0001u) {
        return ZIP_ERR_ENCRYPTED;
    }
    if (!read_u16(r->data, r->len, lh_off + 26, &lh_name_len) ||
        !read_u16(r->data, r->len, lh_off + 28, &lh_extra_len)) {
        return ZIP_ERR_MALFORMED;
    }

    /* The Local Header's own name/extra lengths (which may legitimately
     * differ from the Central Directory copy) determine where data starts;
     * the Central Directory's compressed_size determines where it ends —
     * the Central Directory is authoritative for sizes (see zip.h). */
    data_start64 = (uint64_t)lh_off + 30u + (uint64_t)lh_name_len +
                   (uint64_t)lh_extra_len;
    if (data_start64 > (uint64_t)r->len) {
        return ZIP_ERR_MALFORMED;
    }
    data_end64 = data_start64 + (uint64_t)entry->compressed_size;
    if (data_end64 > (uint64_t)r->len) {
        return ZIP_ERR_MALFORMED;
    }
    data_start = (size_t)data_start64;
    data_end = (size_t)data_end64;
    compressed = r->data + data_start;
    compressed_len = data_end - data_start;

    switch (entry->method) {
        case 0: /* Stored */
            if (compressed_len > 0) {
                decompressed = (unsigned char *)malloc(compressed_len);
                if (!decompressed) {
                    return ZIP_ERR_ALLOC;
                }
                memcpy(decompressed, compressed, compressed_len);
            }
            decompressed_len = compressed_len;
            break;
        case 8: { /* DEFLATE — c/deflate decodes stored/fixed/dynamic blocks,
                   * and caps output at DEFLATE_MAX_OUTPUT (per-entry bomb
                   * guard) independently of the aggregate check above. */
            DeflateStatus ds = deflate_decompress(compressed, compressed_len,
                                                  &decompressed,
                                                  &decompressed_len);
            if (ds == DEFLATE_ERR_ALLOC) {
                return ZIP_ERR_ALLOC;
            }
            if (ds != DEFLATE_OK) {
                return ZIP_ERR_MALFORMED;
            }
            break;
        }
        default:
            return ZIP_ERR_UNSUPPORTED_METHOD;
    }

    /* Trim to the declared uncompressed size (guards against a decompressor
     * that produced more bytes than promised); matches the reference Rust
     * implementation's behaviour exactly. */
    if (decompressed_len > (size_t)entry->size) {
        decompressed_len = (size_t)entry->size;
    }

    {
        uint32_t actual_crc = zip_crc32(decompressed, decompressed_len, 0);
        if (actual_crc != entry->crc32) {
            free(decompressed);
            return ZIP_ERR_CRC_MISMATCH;
        }
    }

    r->total_uncompressed += decompressed_len;
    *out = decompressed;
    *out_len = decompressed_len;
    return ZIP_OK;
}

ZipStatus zip_reader_read_by_name(ZipReader *r, const char *name,
                                  unsigned char **out, size_t *out_len) {
    size_t i;
    *out = NULL;
    *out_len = 0;
    if (!r || !name) {
        return ZIP_ERR_NOT_FOUND;
    }
    for (i = 0; i < r->entry_count; i++) {
        if (strcmp(r->entries[i].name, name) == 0) {
            return zip_reader_read(r, &r->entries[i], out, out_len);
        }
    }
    return ZIP_ERR_NOT_FOUND;
}

void zip_reader_free(ZipReader *r) {
    size_t i;
    if (!r) {
        return;
    }
    for (i = 0; i < r->entry_count; i++) {
        free(r->entries[i].name);
    }
    free(r->entries);
    free(r);
}

/* =========================================================================
 * One-shot convenience API
 * ========================================================================= */

ZipStatus zip_bytes(const ZipFile *files, size_t count, unsigned char **out,
                    size_t *out_len) {
    ZipWriter *w = NULL;
    ZipStatus st;
    size_t i;

    *out = NULL;
    *out_len = 0;
    st = zip_writer_new(&w);
    if (st != ZIP_OK) {
        return st;
    }
    for (i = 0; i < count; i++) {
        st = zip_writer_add_file(w, files[i].name, files[i].data,
                                 files[i].len, 1);
        if (st != ZIP_OK) {
            zip_writer_free(w);
            return st;
        }
    }
    st = zip_writer_finish(w, out, out_len);
    zip_writer_free(w);
    return st;
}

static void free_zip_files(ZipFile *files, size_t n) {
    size_t i;
    if (!files) {
        return;
    }
    for (i = 0; i < n; i++) {
        free(files[i].name);
        free(files[i].data);
    }
    free(files);
}

ZipStatus zip_unzip(const unsigned char *data, size_t len, ZipFile **out_files,
                    size_t *out_count) {
    ZipReader *r = NULL;
    ZipStatus st;
    ZipFile *files = NULL;
    size_t cap = 0, n = 0, i;

    *out_files = NULL;
    *out_count = 0;
    st = zip_reader_new(data, len, &r);
    if (st != ZIP_OK) {
        return st;
    }

    for (i = 0; i < zip_reader_entry_count(r); i++) {
        const ZipEntry *e = zip_reader_entry(r, i);
        unsigned char *fdata;
        size_t flen;

        if (e->is_directory) {
            continue;
        }
        st = zip_reader_read(r, e, &fdata, &flen);
        if (st != ZIP_OK) {
            free_zip_files(files, n);
            zip_reader_free(r);
            return st;
        }

        if (n == cap) {
            size_t nc = cap ? cap * 2 : 8;
            ZipFile *nf;
            if (cap > (SIZE_MAX / sizeof(ZipFile)) / 2) {
                free(fdata);
                free_zip_files(files, n);
                zip_reader_free(r);
                return ZIP_ERR_ALLOC;
            }
            nf = (ZipFile *)realloc(files, nc * sizeof *nf);
            if (!nf) {
                free(fdata);
                free_zip_files(files, n);
                zip_reader_free(r);
                return ZIP_ERR_ALLOC;
            }
            files = nf;
            cap = nc;
        }

        files[n].name = (char *)malloc(e->name_len + 1u);
        if (!files[n].name) {
            free(fdata);
            free_zip_files(files, n);
            zip_reader_free(r);
            return ZIP_ERR_ALLOC;
        }
        memcpy(files[n].name, e->name, e->name_len + 1u); /* includes NUL */
        files[n].name_len = e->name_len;
        files[n].data = fdata;
        files[n].len = flen;
        n++;
    }

    zip_reader_free(r);
    *out_files = files;
    *out_count = n;
    return ZIP_OK;
}

void zip_files_free(ZipFile *files, size_t count) {
    free_zip_files(files, count);
}

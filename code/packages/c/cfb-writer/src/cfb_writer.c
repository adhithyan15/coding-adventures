/*
 * cfb_writer.c — implementation of the pure-ISO C Compound File Binary writer.
 * ==========================================================================
 *
 * See cfb_writer.h. The layout is fully deterministic. `cfb_writer_finish`
 * builds a `Layout` (a handful of owned byte buffers plus the FAT array) and
 * concatenates them. Every allocation is checked; on OOM the whole thing unwinds
 * and returns NULL.
 */
#include "cfb_writer.h"

#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, memset, strlen */

/* ── Constants (mirroring MS-CFB / the reader) ─────────────────────────────*/

static const uint8_t SIGNATURE[8] = {0xD0, 0xCF, 0x11, 0xE0,
                                     0xA1, 0xB1, 0x1A, 0xE1};
#define FREESECT 0xFFFFFFFFu
#define ENDOFCHAIN 0xFFFFFFFEu
#define FATSECT 0xFFFFFFFDu
#define NOSTREAM 0xFFFFFFFFu

#define HEADER_LEN 512
#define SECTOR_SIZE 512
#define MINI_SECTOR_SIZE 64
#define MINI_CUTOFF 4096u
#define DIR_ENTRY_SIZE 128
#define HEADER_DIFAT_COUNT 109
#define HEADER_DIFAT_OFFSET 76
#define FAT_ENTRIES_PER_SECTOR (SECTOR_SIZE / 4)
#define MINIFAT_ENTRIES_PER_SECTOR (SECTOR_SIZE / 4)
#define MAX_NAME_UNITS 31
#define OBJ_STREAM 0x02
#define OBJ_ROOT 0x05
#define COLOR_BLACK 0x01

/* ── Growable byte buffer ──────────────────────────────────────────────────*/

typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
    int oom;
} Buf;

static void buf_init(Buf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->oom = 0;
}
static void buf_free(Buf *b) {
    free(b->data);
    buf_init(b);
}
static int buf_reserve(Buf *b, size_t extra) {
    size_t need, nc;
    uint8_t *nd;
    if (b->oom) return 0;
    if (extra > (size_t)-1 - b->len) {
        b->oom = 1;
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) return 1;
    nc = b->cap ? b->cap : 512;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    nd = (uint8_t *)realloc(b->data, nc);
    if (nd == NULL) {
        b->oom = 1;
        return 0;
    }
    b->data = nd;
    b->cap = nc;
    return 1;
}
static void buf_extend(Buf *b, const uint8_t *p, size_t n) {
    if (!buf_reserve(b, n)) return;
    if (n > 0) memcpy(b->data + b->len, p, n);
    b->len += n;
}
static void buf_zeros(Buf *b, size_t n) {
    if (!buf_reserve(b, n)) return;
    if (n > 0) memset(b->data + b->len, 0, n);
    b->len += n;
}

/* Little-endian patch writers into an already-sized buffer. */
static void put_u16(uint8_t *buf, size_t off, uint16_t v) {
    buf[off] = (uint8_t)(v & 0xff);
    buf[off + 1] = (uint8_t)((v >> 8) & 0xff);
}
static void put_u32(uint8_t *buf, size_t off, uint32_t v) {
    buf[off] = (uint8_t)(v & 0xff);
    buf[off + 1] = (uint8_t)((v >> 8) & 0xff);
    buf[off + 2] = (uint8_t)((v >> 16) & 0xff);
    buf[off + 3] = (uint8_t)((v >> 24) & 0xff);
}
static void put_u64(uint8_t *buf, size_t off, uint64_t v) {
    int i;
    for (i = 0; i < 8; i++) buf[off + (size_t)i] = (uint8_t)((v >> (i * 8)) & 0xff);
}

/* Round `n` up to a whole number of `unit`s (unit != 0), overflow-safe. */
static uint64_t div_round_up(uint64_t n, uint64_t unit) {
    return n == 0 ? 0 : (n - 1) / unit + 1;
}

/* ── Growable u32 array (the FAT / mini-FAT chains) ────────────────────────*/

typedef struct {
    uint32_t *data;
    size_t len;
    size_t cap;
    int oom;
} U32Vec;

static void u32_init(U32Vec *v) {
    v->data = NULL;
    v->len = 0;
    v->cap = 0;
    v->oom = 0;
}
static void u32_free(U32Vec *v) {
    free(v->data);
    u32_init(v);
}
static int u32_push(U32Vec *v, uint32_t x) {
    if (v->oom) return 0;
    if (v->len == v->cap) {
        size_t nc = v->cap ? v->cap : 16;
        uint32_t *nd;
        if (nc > ((size_t)-1) / 2 / sizeof(uint32_t)) {
            v->oom = 1;
            return 0;
        }
        nc *= 2;
        nd = (uint32_t *)realloc(v->data, nc * sizeof(uint32_t));
        if (nd == NULL) {
            v->oom = 1;
            return 0;
        }
        v->data = nd;
        v->cap = nc;
    }
    v->data[v->len++] = x;
    return 1;
}
/* Resize to `n` entries, filling any newly-exposed slots with `fill`. */
static int u32_resize_fill(U32Vec *v, size_t n, uint32_t fill) {
    if (v->oom) return 0;
    if (n > v->cap) {
        uint32_t *nd;
        /* Guard the multiply BEFORE allocating so realloc never sees a wrapped
         * size (and we never drop the old pointer on the overflow path). */
        if (n > ((size_t)-1) / sizeof(uint32_t)) {
            v->oom = 1;
            return 0;
        }
        nd = (uint32_t *)realloc(v->data, n * sizeof(uint32_t));
        if (nd == NULL) {
            v->oom = 1;
            return 0;
        }
        v->data = nd;
        v->cap = n;
    }
    if (n > v->len) {
        size_t i;
        for (i = v->len; i < n; i++) v->data[i] = fill;
    }
    v->len = n;
    return 1;
}

/* ── UTF-8 → UTF-16 (for the on-disk name field) ──────────────────────────*/

/* Decode `name` (UTF-8) into UTF-16 units, truncated to MAX_NAME_UNITS. Writes
 * up to MAX_NAME_UNITS units into `out` and returns the count. Invalid bytes are
 * replaced with U+FFFD (defensive; C callers should pass valid UTF-8). */
static size_t utf8_to_utf16_truncated(const char *name, uint16_t *out) {
    const unsigned char *s = (const unsigned char *)name;
    size_t n_units = 0;
    while (*s != '\0' && n_units < MAX_NAME_UNITS) {
        uint32_t cp;
        unsigned char b0 = s[0];
        int extra, ok = 1, k;
        if (b0 < 0x80) {
            cp = b0;
            extra = 0;
        } else if ((b0 & 0xE0) == 0xC0) {
            cp = b0 & 0x1Fu;
            extra = 1;
        } else if ((b0 & 0xF0) == 0xE0) {
            cp = b0 & 0x0Fu;
            extra = 2;
        } else if ((b0 & 0xF8) == 0xF0) {
            cp = b0 & 0x07u;
            extra = 3;
        } else {
            cp = 0xFFFD;
            extra = 0;
            ok = 0;
        }
        for (k = 1; ok && k <= extra; k++) {
            if ((s[k] & 0xC0) != 0x80) {
                ok = 0;
                cp = 0xFFFD;
                extra = 0;
                break;
            }
            cp = (cp << 6) | (uint32_t)(s[k] & 0x3F);
        }
        s += 1 + (size_t)(ok ? extra : 0);

        if (cp <= 0xFFFF) {
            out[n_units++] = (uint16_t)cp;
        } else {
            /* astral plane → surrogate pair (needs room for both units) */
            uint32_t c = cp - 0x10000;
            if (n_units + 2 > MAX_NAME_UNITS) break;
            out[n_units++] = (uint16_t)(0xD800 + (c >> 10));
            out[n_units++] = (uint16_t)(0xDC00 + (c & 0x3FF));
        }
    }
    return n_units;
}

/* ── Streams / the writer ──────────────────────────────────────────────────*/

typedef struct {
    char *name; /* owned UTF-8 copy */
    uint8_t *data;
    size_t data_len;
} Stream;

struct CfbWriter {
    Stream *streams;
    size_t n;
    size_t cap;
};

CfbWriter *cfb_writer_new(void) {
    CfbWriter *w = (CfbWriter *)calloc(1, sizeof(CfbWriter));
    return w;
}

void cfb_writer_free(CfbWriter *w) {
    size_t i;
    if (w == NULL) return;
    for (i = 0; i < w->n; i++) {
        free(w->streams[i].name);
        free(w->streams[i].data);
    }
    free(w->streams);
    free(w);
}

int cfb_writer_add_stream(CfbWriter *w, const char *name, const uint8_t *data,
                          size_t data_len) {
    char *name_copy;
    uint8_t *data_copy = NULL;
    size_t name_len = strlen(name);

    if (w->n == w->cap) {
        size_t nc = w->cap ? w->cap : 4;
        Stream *ns;
        if (nc > ((size_t)-1) / 2 / sizeof(Stream)) return 0;
        nc *= 2;
        ns = (Stream *)realloc(w->streams, nc * sizeof(Stream));
        if (ns == NULL) return 0;
        w->streams = ns;
        w->cap = nc;
    }
    name_copy = (char *)malloc(name_len + 1);
    if (name_copy == NULL) return 0;
    memcpy(name_copy, name, name_len + 1);
    if (data_len > 0) {
        data_copy = (uint8_t *)malloc(data_len);
        if (data_copy == NULL) {
            free(name_copy);
            return 0;
        }
        memcpy(data_copy, data, data_len);
    }
    w->streams[w->n].name = name_copy;
    w->streams[w->n].data = data_copy;
    w->streams[w->n].data_len = data_len;
    w->n++;
    return 1;
}

/* ── Directory entry encoding ──────────────────────────────────────────────*/

typedef struct {
    const char *name; /* borrowed UTF-8 */
    uint8_t object_type;
    uint32_t right;
    uint32_t child;
    uint32_t start_sector;
    uint64_t size;
} DirEntry;

static void encode_dir_entry(const DirEntry *e, uint8_t out[DIR_ENTRY_SIZE]) {
    uint16_t units[MAX_NAME_UNITS];
    size_t n_units, i;
    memset(out, 0, DIR_ENTRY_SIZE);
    n_units = utf8_to_utf16_truncated(e->name, units);
    for (i = 0; i < n_units; i++) put_u16(out, i * 2, units[i]);
    /* out[2*n_units..2*n_units+2] stays zero — the NUL terminator. */
    put_u16(out, 64, (uint16_t)((n_units + 1) * 2)); /* name length incl NUL */
    out[66] = e->object_type;
    out[67] = COLOR_BLACK;
    put_u32(out, 68, NOSTREAM); /* left: unused */
    put_u32(out, 72, e->right);
    put_u32(out, 76, e->child);
    /* CLSID / flags / times (80..116): zero for determinism */
    put_u32(out, 116, e->start_sector);
    put_u64(out, 120, e->size);
}

/* Encode all entries + pad to a whole sector with "unused" (type-0) entries. */
static void encode_directory(const DirEntry *dir, size_t n_dir, Buf *out) {
    size_t i, rem;
    uint8_t entry[DIR_ENTRY_SIZE];
    for (i = 0; i < n_dir; i++) {
        encode_dir_entry(&dir[i], entry);
        buf_extend(out, entry, DIR_ENTRY_SIZE);
    }
    rem = out->len % SECTOR_SIZE;
    if (rem != 0) {
        size_t pad_bytes = SECTOR_SIZE - rem;
        size_t n_entries = pad_bytes / DIR_ENTRY_SIZE;
        for (i = 0; i < n_entries; i++) {
            memset(entry, 0, DIR_ENTRY_SIZE);
            put_u32(entry, 68, NOSTREAM);
            put_u32(entry, 72, NOSTREAM);
            put_u32(entry, 76, NOSTREAM);
            buf_extend(out, entry, DIR_ENTRY_SIZE);
        }
    }
}

/* Encode a FAT-like u32 array, padded to whole sectors, trailing = FREESECT. */
static void encode_fat_like(const uint32_t *entries, size_t n,
                            size_t entries_per_sector, Buf *out) {
    size_t sectors = (size_t)div_round_up(n, entries_per_sector);
    size_t total_slots = sectors * entries_per_sector;
    size_t i;
    if (n == 0) return; /* empty stays empty */
    if (!buf_reserve(out, total_slots * 4)) return;
    for (i = 0; i < n; i++) {
        put_u32(out->data + out->len, 0, entries[i]);
        out->len += 4;
    }
    for (i = n; i < total_slots; i++) {
        put_u32(out->data + out->len, 0, FREESECT);
        out->len += 4;
    }
}

static void pad_to_sector(Buf *b) {
    size_t rem = b->len % SECTOR_SIZE;
    if (rem != 0) buf_zeros(b, SECTOR_SIZE - rem);
}

/* ── The full build + serialise ────────────────────────────────────────────*/

/* Link `count` consecutive sectors from `start` into a FAT chain. */
static void chain(U32Vec *fat, uint32_t start, size_t count) {
    size_t k;
    for (k = 0; k < count; k++) {
        size_t s = (size_t)start + k;
        fat->data[s] = (k + 1 < count) ? (start + (uint32_t)k + 1) : ENDOFCHAIN;
    }
}

uint8_t *cfb_writer_finish(CfbWriter *w, size_t *out_len) {
    size_t n = w->n, i;
    uint8_t *result = NULL;

    /* Per-stream placement: 0=empty, 1=mini, 2=large; plus bucket index. */
    int *place = NULL;
    size_t *bucket = NULL;
    U32Vec mini_start_of; /* mini-sector start per mini bucket */
    U32Vec large_start_of; /* regular sector start per large bucket */
    Buf mini_stream, minifat_bytes, directory, out;
    U32Vec minifat, fat;
    DirEntry *dir = NULL;
    uint64_t mini_stream_size = 0;
    size_t mini_bucket = 0, large_bucket = 0;
    uint32_t next_sector = 0;
    uint32_t first_dir_sector, first_minifat_sector = ENDOFCHAIN;
    uint32_t num_minifat_sectors = 0, mini_stream_start = ENDOFCHAIN;
    size_t dir_sector_count, minifat_sector_count = 0;
    size_t mini_stream_sector_count = 0;
    size_t data_sectors, total_sectors, num_fat_sectors = 0;
    size_t k;
    int oom = 0;

    u32_init(&mini_start_of);
    u32_init(&large_start_of);
    u32_init(&minifat);
    u32_init(&fat);
    buf_init(&mini_stream);
    buf_init(&minifat_bytes);
    buf_init(&directory);
    buf_init(&out);

    place = (int *)calloc(n ? n : 1, sizeof(int));
    bucket = (size_t *)calloc(n ? n : 1, sizeof(size_t));
    dir = (DirEntry *)calloc(n + 1, sizeof(DirEntry));
    if (place == NULL || bucket == NULL || dir == NULL) {
        oom = 1;
        goto cleanup;
    }

    /* 1. Partition + 2. build mini-stream and mini-FAT. */
    for (i = 0; i < n; i++) {
        const Stream *s = &w->streams[i];
        if (s->data_len == 0) {
            place[i] = 0;
        } else if ((uint64_t)s->data_len < (uint64_t)MINI_CUTOFF) {
            uint32_t start_mini = (uint32_t)minifat.len;
            size_t n_mini =
                (size_t)div_round_up(s->data_len, MINI_SECTOR_SIZE);
            size_t j;
            place[i] = 1;
            bucket[i] = mini_bucket;
            if (!u32_push(&mini_start_of, start_mini)) {
                oom = 1;
                goto cleanup;
            }
            buf_extend(&mini_stream, s->data, s->data_len);
            buf_zeros(&mini_stream, n_mini * MINI_SECTOR_SIZE - s->data_len);
            for (j = 0; j < n_mini; j++) {
                uint32_t nextv =
                    (j + 1 < n_mini) ? (start_mini + (uint32_t)j + 1)
                                     : ENDOFCHAIN;
                if (!u32_push(&minifat, nextv)) {
                    oom = 1;
                    goto cleanup;
                }
            }
            mini_bucket++;
        } else {
            place[i] = 2;
            bucket[i] = large_bucket;
            large_bucket++;
        }
    }

    mini_stream_size = (uint64_t)mini_stream.len;
    pad_to_sector(&mini_stream);
    encode_fat_like(minifat.data, minifat.len, MINIFAT_ENTRIES_PER_SECTOR,
                    &minifat_bytes);

    /* 3. Build directory entries (root + one per stream). */
    dir[0].name = "Root Entry";
    dir[0].object_type = OBJ_ROOT;
    dir[0].right = NOSTREAM;
    dir[0].child = (n == 0) ? NOSTREAM : 1u;
    dir[0].start_sector = ENDOFCHAIN; /* patched to mini-stream start */
    dir[0].size = mini_stream_size;
    for (i = 0; i < n; i++) {
        DirEntry *e = &dir[i + 1];
        e->name = w->streams[i].name;
        e->object_type = OBJ_STREAM;
        e->right = (i + 1 < n) ? (uint32_t)(i + 2) : NOSTREAM;
        e->child = NOSTREAM;
        e->size = (uint64_t)w->streams[i].data_len;
        if (place[i] == 0)
            e->start_sector = ENDOFCHAIN;
        else if (place[i] == 1)
            e->start_sector = mini_start_of.data[bucket[i]]; /* mini index */
        else
            e->start_sector = 0; /* patched below */
    }

    /* 4. Assign regular sectors. */
    dir_sector_count =
        (size_t)div_round_up((uint64_t)(n + 1) * DIR_ENTRY_SIZE, SECTOR_SIZE);
    first_dir_sector = next_sector;
    next_sector += (uint32_t)dir_sector_count;

    minifat_sector_count = minifat_bytes.len / SECTOR_SIZE;
    if (minifat_sector_count == 0) {
        first_minifat_sector = ENDOFCHAIN;
        num_minifat_sectors = 0;
    } else {
        first_minifat_sector = next_sector;
        next_sector += (uint32_t)minifat_sector_count;
        num_minifat_sectors = (uint32_t)minifat_sector_count;
    }

    mini_stream_sector_count = mini_stream.len / SECTOR_SIZE;
    if (mini_stream_sector_count == 0) {
        mini_stream_start = ENDOFCHAIN;
    } else {
        mini_stream_start = next_sector;
        next_sector += (uint32_t)mini_stream_sector_count;
    }

    for (i = 0; i < n; i++) {
        if (place[i] == 2) {
            size_t sc = (size_t)div_round_up(w->streams[i].data_len, SECTOR_SIZE);
            if (!u32_push(&large_start_of, next_sector)) {
                oom = 1;
                goto cleanup;
            }
            next_sector += (uint32_t)sc;
        }
    }
    data_sectors = next_sector;

    /* 4b. FAT chains for the data sectors. */
    if (!u32_resize_fill(&fat, data_sectors, FREESECT)) {
        oom = 1;
        goto cleanup;
    }
    chain(&fat, first_dir_sector, dir_sector_count);
    if (num_minifat_sectors > 0)
        chain(&fat, first_minifat_sector, minifat_sector_count);
    if (mini_stream_sector_count > 0)
        chain(&fat, mini_stream_start, mini_stream_sector_count);
    for (i = 0; i < n; i++) {
        if (place[i] == 2) {
            size_t sc = (size_t)div_round_up(w->streams[i].data_len, SECTOR_SIZE);
            chain(&fat, large_start_of.data[bucket[i]], sc);
            dir[i + 1].start_sector = large_start_of.data[bucket[i]];
        }
    }
    dir[0].start_sector = mini_stream_start;

    /* Encode the directory bytes now that start sectors are final. */
    encode_directory(dir, n + 1, &directory);

    /* 5. Fixed-point the FAT-sector count. */
    for (;;) {
        size_t total = data_sectors + num_fat_sectors;
        size_t needed = (size_t)div_round_up(total, FAT_ENTRIES_PER_SECTOR);
        if (needed == num_fat_sectors) break;
        num_fat_sectors = needed;
    }
    total_sectors = data_sectors + num_fat_sectors;
    if (!u32_resize_fill(&fat, total_sectors, FREESECT)) {
        oom = 1;
        goto cleanup;
    }
    for (k = 0; k < num_fat_sectors; k++)
        fat.data[data_sectors + k] = FATSECT;

    /* Propagate any buffer OOM before serialising. */
    if (mini_stream.oom || minifat_bytes.oom || directory.oom || minifat.oom ||
        fat.oom) {
        oom = 1;
        goto cleanup;
    }

    /* ── Serialise ─────────────────────────────────────────────────────────*/
    buf_zeros(&out, HEADER_LEN);
    if (out.oom) {
        oom = 1;
        goto cleanup;
    }
    memcpy(out.data, SIGNATURE, 8);
    put_u16(out.data, 24, 0x003E); /* minor version */
    put_u16(out.data, 26, 0x0003); /* major version (v3) */
    put_u16(out.data, 28, 0xFFFE); /* little-endian BOM */
    put_u16(out.data, 30, 0x0009); /* sector shift → 512 */
    put_u16(out.data, 32, 0x0006); /* mini sector shift → 64 */
    put_u32(out.data, 40, 0);      /* directory sector count (0 for v3) */
    put_u32(out.data, 44, (uint32_t)num_fat_sectors);
    put_u32(out.data, 48, first_dir_sector);
    put_u32(out.data, 52, 0);
    put_u32(out.data, 56, MINI_CUTOFF);
    put_u32(out.data, 60, first_minifat_sector);
    put_u32(out.data, 64, num_minifat_sectors);
    put_u32(out.data, 68, ENDOFCHAIN); /* first DIFAT sector (none) */
    put_u32(out.data, 72, 0);          /* DIFAT sector count */
    /* DIFAT: first 109 FAT-sector ids, then FREESECT. */
    for (i = 0; i < HEADER_DIFAT_COUNT; i++) {
        uint32_t v = (i < num_fat_sectors) ? (uint32_t)(data_sectors + i)
                                           : FREESECT;
        put_u32(out.data, HEADER_DIFAT_OFFSET + i * 4, v);
    }

    /* Data sectors in numbering order: directory, mini-FAT, mini-stream, large. */
    buf_extend(&out, directory.data, directory.len);
    buf_extend(&out, minifat_bytes.data, minifat_bytes.len);
    buf_extend(&out, mini_stream.data, mini_stream.len);
    for (i = 0; i < n; i++) {
        if (place[i] == 2) {
            size_t dl = w->streams[i].data_len;
            size_t sc = (size_t)div_round_up(dl, SECTOR_SIZE);
            buf_extend(&out, w->streams[i].data, dl);
            buf_zeros(&out, sc * SECTOR_SIZE - dl);
        }
    }

    /* FAT sectors. */
    {
        Buf fat_bytes;
        buf_init(&fat_bytes);
        encode_fat_like(fat.data, fat.len, FAT_ENTRIES_PER_SECTOR, &fat_bytes);
        buf_extend(&out, fat_bytes.data, fat_bytes.len);
        if (fat_bytes.oom) out.oom = 1;
        buf_free(&fat_bytes);
    }

    if (out.oom) {
        oom = 1;
        goto cleanup;
    }
    result = out.data; /* transfer ownership */
    *out_len = out.len;
    buf_init(&out); /* prevent free below */

cleanup:
    free(place);
    free(bucket);
    free(dir);
    u32_free(&mini_start_of);
    u32_free(&large_start_of);
    u32_free(&minifat);
    u32_free(&fat);
    buf_free(&mini_stream);
    buf_free(&minifat_bytes);
    buf_free(&directory);
    buf_free(&out);
    cfb_writer_free(w); /* the writer is consumed */
    if (oom) {
        *out_len = 0;
        return NULL;
    }
    return result;
}

uint8_t *cfb_write(const char *const *names, const uint8_t *const *data,
                   const size_t *data_len, size_t n, size_t *out_len) {
    CfbWriter *w = cfb_writer_new();
    size_t i;
    if (w == NULL) {
        *out_len = 0;
        return NULL;
    }
    for (i = 0; i < n; i++) {
        if (!cfb_writer_add_stream(w, names[i], data[i], data_len[i])) {
            cfb_writer_free(w);
            *out_len = 0;
            return NULL;
        }
    }
    return cfb_writer_finish(w, out_len);
}

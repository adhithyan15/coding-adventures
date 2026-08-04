/*
 * cfb.c — OLE2 / Compound File Binary Format reader, pure ISO C17.
 * =====================================================================
 *
 * See cfb.h. A faithful port of the Rust `cfb` crate. `cfb_open` validates the
 * header, assembles the FAT / mini-FAT / directory / mini-stream up front, and
 * flattens the directory tree; later reads walk a bounds- and cycle-guarded
 * sector chain.
 *
 * Cycle detection: the Rust uses a `HashSet` visited-set. Here, sector-chain
 * walks use the equivalent step cap (`steps >= fat_len + 1`) — a valid acyclic
 * chain can never exceed the number of FAT slots, so exceeding it proves a
 * cycle (same `CFB_CYCLE_DETECTED` result, bounded work). The directory-tree
 * walk uses an explicit `visited` bool array sized to the directory.
 */
#include "cfb.h"

#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, memcmp */

/* ── Constants ──────────────────────────────────────────────────────────────*/
static const uint8_t SIGNATURE[8] = {0xD0, 0xCF, 0x11, 0xE0,
                                     0xA1, 0xB1, 0x1A, 0xE1};
#define FREESECT 0xFFFFFFFFu
#define ENDOFCHAIN 0xFFFFFFFEu
#define FATSECT 0xFFFFFFFDu
#define DIFSECT 0xFFFFFFFCu
#define NOSTREAM 0xFFFFFFFFu

#define HEADER_LEN 512
#define DIR_ENTRY_SIZE 128
#define HEADER_DIFAT_COUNT 109
#define HEADER_DIFAT_OFFSET 76
#define MAX_OUTPUT ((uint64_t)256 * 1024 * 1024)

typedef struct {
    char name[CFB_NAME_CAP];
    uint8_t object_type;
    uint32_t left, right, child, start_sector;
    uint64_t size;
} DirEntry;

struct CompoundFile {
    uint8_t *data;
    size_t data_len;
    size_t sector_size;
    size_t mini_sector_size;
    uint32_t mini_cutoff;
    uint32_t *fat;
    size_t fat_len;
    uint32_t *mini_fat;
    size_t mini_fat_len;
    DirEntry *dir;
    size_t dir_len;
    uint8_t *mini_stream;
    size_t mini_stream_len;
    CfbEntry *entries;
    size_t entries_len;
};

const char *cfb_error_str(CfbError e) {
    switch (e) {
    case CFB_OK:
        return "ok";
    case CFB_BAD_SIGNATURE:
        return "not a Compound File (bad signature)";
    case CFB_TRUNCATED:
        return "input truncated";
    case CFB_UNSUPPORTED_SECTOR_SIZE:
        return "unsupported sector shift";
    case CFB_BAD_SECTOR_CHAIN:
        return "sector chain out of bounds";
    case CFB_CYCLE_DETECTED:
        return "cycle detected in sector chain";
    case CFB_OUTPUT_TOO_LARGE:
        return "assembled output exceeds safety cap";
    case CFB_BAD_DIRECTORY:
        return "malformed directory";
    case CFB_NOT_A_STREAM:
        return "directory entry is not a stream";
    }
    return "unknown error";
}

/* ── Little-endian readers (bounds-checked, overflow-safe) ──────────────────*/
static int rd_u16(const uint8_t *b, size_t len, size_t off, uint16_t *out) {
    if (off > len || len - off < 2) {
        return 0;
    }
    *out = (uint16_t)((uint16_t)b[off] | ((uint16_t)b[off + 1] << 8));
    return 1;
}
static int rd_u32(const uint8_t *b, size_t len, size_t off, uint32_t *out) {
    if (off > len || len - off < 4) {
        return 0;
    }
    *out = (uint32_t)b[off] | ((uint32_t)b[off + 1] << 8) |
           ((uint32_t)b[off + 2] << 16) | ((uint32_t)b[off + 3] << 24);
    return 1;
}
static int rd_u64(const uint8_t *b, size_t len, size_t off, uint64_t *out) {
    uint32_t lo, hi;
    if (!rd_u32(b, len, off, &lo) || !rd_u32(b, len, off + 4, &hi)) {
        return 0;
    }
    *out = (uint64_t)lo | ((uint64_t)hi << 32);
    return 1;
}

/* ── Growable buffers with size_t-overflow-guarded doubling ─────────────────*/
static int u32vec_push(uint32_t **p, size_t *len, size_t *cap, uint32_t v) {
    if (*len == *cap) {
        size_t nc = *cap ? *cap : 8;
        uint32_t *nd;
        if (nc > (size_t)-1 / 2) {
            return 0;
        }
        nc *= 2;
        if (nc > (size_t)-1 / sizeof(uint32_t)) {
            return 0;
        }
        nd = (uint32_t *)realloc(*p, nc * sizeof(uint32_t));
        if (!nd) {
            return 0;
        }
        *p = nd;
        *cap = nc;
    }
    (*p)[(*len)++] = v;
    return 1;
}
static int bytevec_extend(uint8_t **p, size_t *len, size_t *cap,
                          const uint8_t *src, size_t n) {
    if (n > (size_t)-1 - *len) {
        return 0;
    }
    if (*len + n > *cap) {
        size_t nc = *cap ? *cap : 64;
        uint8_t *nd;
        while (nc < *len + n) {
            if (nc > (size_t)-1 / 2) {
                nc = *len + n;
                break;
            }
            nc *= 2;
        }
        nd = (uint8_t *)realloc(*p, nc ? nc : 1);
        if (!nd) {
            return 0;
        }
        *p = nd;
        *cap = nc;
    }
    if (n) {
        memcpy(*p + *len, src, n);
    }
    *len += n;
    return 1;
}

/* ── Name decoding (UTF-16 LE → UTF-8) ──────────────────────────────────────*/
static void utf8_put(char *out, size_t cap, size_t *pos, uint32_t cp) {
    if (cp < 0x80) {
        if (*pos + 1 < cap) {
            out[(*pos)++] = (char)cp;
        }
    } else if (cp < 0x800) {
        if (*pos + 2 < cap) {
            out[(*pos)++] = (char)(0xC0 | (cp >> 6));
            out[(*pos)++] = (char)(0x80 | (cp & 0x3F));
        }
    } else if (cp < 0x10000) {
        if (*pos + 3 < cap) {
            out[(*pos)++] = (char)(0xE0 | (cp >> 12));
            out[(*pos)++] = (char)(0x80 | ((cp >> 6) & 0x3F));
            out[(*pos)++] = (char)(0x80 | (cp & 0x3F));
        }
    } else {
        if (*pos + 4 < cap) {
            out[(*pos)++] = (char)(0xF0 | (cp >> 18));
            out[(*pos)++] = (char)(0x80 | ((cp >> 12) & 0x3F));
            out[(*pos)++] = (char)(0x80 | ((cp >> 6) & 0x3F));
            out[(*pos)++] = (char)(0x80 | (cp & 0x3F));
        }
    }
}

/* `name_len` is the UTF-16 byte length including the 2-byte NUL terminator;
 * lossy (unpaired surrogates → U+FFFD). Reads only the 64-byte name field. */
static void decode_utf16le_name(const uint8_t *field, size_t name_len,
                                char *out, size_t cap) {
    size_t usable = name_len < 64 ? name_len : 64;
    size_t chars = usable >= 2 ? usable - 2 : 0; /* strip trailing NUL */
    size_t i = 0, pos = 0;
    while (i + 2 <= 64 && i < chars) {
        uint32_t u = (uint32_t)field[i] | ((uint32_t)field[i + 1] << 8);
        i += 2;
        if (u >= 0xD800 && u <= 0xDBFF) {
            if (i + 2 <= 64 && i < chars) {
                uint32_t lo = (uint32_t)field[i] | ((uint32_t)field[i + 1] << 8);
                if (lo >= 0xDC00 && lo <= 0xDFFF) {
                    i += 2;
                    utf8_put(out, cap, &pos,
                             0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00));
                    continue;
                }
            }
            utf8_put(out, cap, &pos, 0xFFFD);
        } else if (u >= 0xDC00 && u <= 0xDFFF) {
            utf8_put(out, cap, &pos, 0xFFFD);
        } else {
            utf8_put(out, cap, &pos, u);
        }
    }
    out[pos] = '\0';
}

/* ── Sector range (bounds-checked, overflow-safe) ───────────────────────────*/
static CfbError sector_range(const CompoundFile *cf, uint32_t n, size_t *start,
                             size_t *end) {
    size_t s, e;
    if (n >= FREESECT - 4) {
        return CFB_BAD_SECTOR_CHAIN;
    }
    if ((size_t)n > ((size_t)-1) / cf->sector_size) {
        return CFB_BAD_SECTOR_CHAIN;
    }
    s = (size_t)n * cf->sector_size;
    if (s > (size_t)-1 - HEADER_LEN) {
        return CFB_BAD_SECTOR_CHAIN;
    }
    s += HEADER_LEN;
    if (s > (size_t)-1 - cf->sector_size) {
        return CFB_BAD_SECTOR_CHAIN;
    }
    e = s + cf->sector_size;
    if (e > cf->data_len) {
        return CFB_BAD_SECTOR_CHAIN;
    }
    *start = s;
    *end = e;
    return CFB_OK;
}

/* ── FAT sector-chain walk (bounds- + cycle-guarded) ────────────────────────*/
static CfbError read_fat_chain(const CompoundFile *cf, uint32_t start,
                               int has_hint, uint64_t hint, uint8_t **out,
                               size_t *out_len) {
    uint8_t *buf = NULL;
    size_t len = 0, cap = 0;
    uint32_t current = start;
    size_t cap_steps = (cf->fat_len ? cf->fat_len : 1) + 1;
    size_t steps = 0;

    while (current != ENDOFCHAIN) {
        size_t s, e;
        CfbError err;
        if (current == FREESECT || current == FATSECT || current == DIFSECT) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        if (steps >= cap_steps) {
            free(buf);
            return CFB_CYCLE_DETECTED;
        }
        steps++;
        err = sector_range(cf, current, &s, &e);
        if (err != CFB_OK) {
            free(buf);
            return err;
        }
        if (!bytevec_extend(&buf, &len, &cap, cf->data + s, e - s)) {
            free(buf);
            return CFB_OUTPUT_TOO_LARGE;
        }
        if ((uint64_t)len > MAX_OUTPUT) {
            free(buf);
            return CFB_OUTPUT_TOO_LARGE;
        }
        if (has_hint && (uint64_t)len >= hint) {
            break;
        }
        if ((size_t)current >= cf->fat_len) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        current = cf->fat[current];
    }
    *out = buf;
    *out_len = len;
    return CFB_OK;
}

/* ── mini-FAT chain walk over the assembled mini-stream ─────────────────────*/
static CfbError read_mini_chain(const CompoundFile *cf, uint32_t start,
                                uint64_t size, uint8_t **out, size_t *out_len) {
    uint8_t *buf = NULL;
    size_t len = 0, cap = 0;
    uint32_t current = start;
    size_t cap_steps = (cf->mini_fat_len ? cf->mini_fat_len : 1) + 1;
    size_t steps = 0;

    if (size > MAX_OUTPUT) {
        return CFB_OUTPUT_TOO_LARGE;
    }
    while (current != ENDOFCHAIN) {
        size_t off, end;
        if (current == FREESECT || current == FATSECT || current == DIFSECT) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        if (steps >= cap_steps) {
            free(buf);
            return CFB_CYCLE_DETECTED;
        }
        steps++;
        if ((size_t)current > ((size_t)-1) / cf->mini_sector_size) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        off = (size_t)current * cf->mini_sector_size;
        if (off > (size_t)-1 - cf->mini_sector_size) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        end = off + cf->mini_sector_size;
        if (end > cf->mini_stream_len) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        if (!bytevec_extend(&buf, &len, &cap, cf->mini_stream + off,
                            cf->mini_sector_size)) {
            free(buf);
            return CFB_OUTPUT_TOO_LARGE;
        }
        if ((uint64_t)len > MAX_OUTPUT) {
            free(buf);
            return CFB_OUTPUT_TOO_LARGE;
        }
        if ((uint64_t)len >= size) {
            break;
        }
        if ((size_t)current >= cf->mini_fat_len) {
            free(buf);
            return CFB_BAD_SECTOR_CHAIN;
        }
        current = cf->mini_fat[current];
    }
    *out = buf;
    *out_len = len;
    return CFB_OK;
}

/* ── DIFAT collection → list of FAT-sector ids ──────────────────────────────*/
static CfbError collect_difat(const CompoundFile *cf, uint32_t first_difat,
                              uint32_t num_difat, uint32_t num_fat_sectors,
                              size_t total_sectors, uint32_t **out,
                              size_t *out_len) {
    uint32_t *ids = NULL;
    size_t len = 0, cap = 0;
    int i;

    for (i = 0; i < HEADER_DIFAT_COUNT; i++) {
        uint32_t v;
        if (!rd_u32(cf->data, cf->data_len,
                    (size_t)HEADER_DIFAT_OFFSET + (size_t)i * 4, &v)) {
            free(ids);
            return CFB_TRUNCATED;
        }
        if (v == FREESECT) {
            continue;
        }
        if (!u32vec_push(&ids, &len, &cap, v)) {
            free(ids);
            return CFB_OUTPUT_TOO_LARGE;
        }
    }

    if (first_difat != ENDOFCHAIN && num_difat > 0) {
        size_t per_sector = cf->sector_size / 4;
        uint32_t current = first_difat;
        size_t bound = total_sectors > 1 ? total_sectors : 1;
        size_t cap_steps = ((size_t)num_difat < bound ? (size_t)num_difat
                                                      : bound) +
                           1;
        size_t steps = 0;
        while (current != ENDOFCHAIN && current != FREESECT) {
            size_t s, e, k;
            CfbError err;
            if (steps >= cap_steps || steps > total_sectors) {
                free(ids);
                return CFB_CYCLE_DETECTED;
            }
            steps++;
            err = sector_range(cf, current, &s, &e);
            if (err != CFB_OK) {
                free(ids);
                return err;
            }
            for (k = 0; k + 1 < per_sector; k++) {
                uint32_t v;
                if (!rd_u32(cf->data + s, e - s, k * 4, &v)) {
                    free(ids);
                    return CFB_TRUNCATED;
                }
                if (v != FREESECT) {
                    if (!u32vec_push(&ids, &len, &cap, v)) {
                        free(ids);
                        return CFB_OUTPUT_TOO_LARGE;
                    }
                }
            }
            if (!rd_u32(cf->data + s, e - s, (per_sector - 1) * 4, &current)) {
                free(ids);
                return CFB_TRUNCATED;
            }
        }
    }

    if ((size_t)num_fat_sectors < len) {
        len = num_fat_sectors;
    }
    *out = ids;
    *out_len = len;
    return CFB_OK;
}

/* ── Assemble the flat FAT ──────────────────────────────────────────────────*/
static CfbError assemble_fat(CompoundFile *cf, const uint32_t *ids,
                             size_t ids_len, size_t total_sectors) {
    size_t per_sector = cf->sector_size / 4;
    size_t i;
    uint32_t *fat = NULL;
    size_t len = 0, cap = 0;

    if (ids_len > total_sectors + 1) {
        return CFB_BAD_SECTOR_CHAIN;
    }
    for (i = 0; i < ids_len; i++) {
        size_t s, e, k;
        CfbError err = sector_range(cf, ids[i], &s, &e);
        if (err != CFB_OK) {
            free(fat);
            return err;
        }
        for (k = 0; k < per_sector; k++) {
            uint32_t v;
            if (!rd_u32(cf->data + s, e - s, k * 4, &v)) {
                free(fat);
                return CFB_TRUNCATED;
            }
            if (!u32vec_push(&fat, &len, &cap, v)) {
                free(fat);
                return CFB_OUTPUT_TOO_LARGE;
            }
        }
    }
    cf->fat = fat;
    cf->fat_len = len;
    return CFB_OK;
}

/* ── Directory parsing ──────────────────────────────────────────────────────*/
static CfbError parse_dir_entry(const uint8_t *e, DirEntry *out) {
    uint16_t name_len;
    if (!rd_u16(e, DIR_ENTRY_SIZE, 64, &name_len)) {
        return CFB_TRUNCATED;
    }
    out->object_type = e[66];
    if (!rd_u32(e, DIR_ENTRY_SIZE, 68, &out->left) ||
        !rd_u32(e, DIR_ENTRY_SIZE, 72, &out->right) ||
        !rd_u32(e, DIR_ENTRY_SIZE, 76, &out->child) ||
        !rd_u32(e, DIR_ENTRY_SIZE, 116, &out->start_sector) ||
        !rd_u64(e, DIR_ENTRY_SIZE, 120, &out->size)) {
        return CFB_TRUNCATED;
    }
    decode_utf16le_name(e, name_len, out->name, CFB_NAME_CAP);
    return CFB_OK;
}

static CfbError read_directory(CompoundFile *cf, uint32_t first_dir_sector) {
    uint8_t *raw = NULL;
    size_t raw_len = 0, count, i;
    CfbError err = read_fat_chain(cf, first_dir_sector, 0, 0, &raw, &raw_len);
    if (err != CFB_OK) {
        return err;
    }
    if (raw_len == 0) {
        free(raw);
        return CFB_BAD_DIRECTORY;
    }
    count = raw_len / DIR_ENTRY_SIZE;
    if (count == 0) {
        free(raw);
        return CFB_BAD_DIRECTORY;
    }
    cf->dir = (DirEntry *)calloc(count, sizeof(DirEntry));
    if (!cf->dir) {
        free(raw);
        return CFB_OUTPUT_TOO_LARGE;
    }
    for (i = 0; i < count; i++) {
        err = parse_dir_entry(raw + i * DIR_ENTRY_SIZE, &cf->dir[i]);
        if (err != CFB_OK) {
            free(raw);
            return err;
        }
    }
    cf->dir_len = count;
    free(raw);
    return CFB_OK;
}

/* ── Flatten the red-black directory tree (explicit stack + visited) ────────*/
static CfbError push_entry(CompoundFile *cf, size_t *cap, const DirEntry *de,
                           CfbEntryKind kind, uint32_t id) {
    if (cf->entries_len == *cap) {
        size_t nc = *cap ? *cap : 8;
        CfbEntry *nd;
        if (nc > (size_t)-1 / 2) {
            return CFB_OUTPUT_TOO_LARGE;
        }
        nc *= 2;
        if (nc > (size_t)-1 / sizeof(CfbEntry)) {
            return CFB_OUTPUT_TOO_LARGE;
        }
        nd = (CfbEntry *)realloc(cf->entries, nc * sizeof(CfbEntry));
        if (!nd) {
            return CFB_OUTPUT_TOO_LARGE;
        }
        cf->entries = nd;
        *cap = nc;
    }
    memcpy(cf->entries[cf->entries_len].name, de->name, CFB_NAME_CAP);
    cf->entries[cf->entries_len].size = de->size;
    cf->entries[cf->entries_len].kind = kind;
    cf->entries[cf->entries_len].id = id;
    cf->entries_len++;
    return CFB_OK;
}

static CfbError walk_tree(CompoundFile *cf, uint32_t start, uint8_t *visited,
                          size_t *ent_cap) {
    uint32_t *stack = NULL;
    size_t sp = 0, scap = 0;
    CfbError err = CFB_OK;

    if (!u32vec_push(&stack, &sp, &scap, start)) {
        return CFB_OUTPUT_TOO_LARGE;
    }
    while (sp > 0) {
        uint32_t id = stack[--sp];
        const DirEntry *de;
        CfbEntryKind kind = CFB_ENTRY_STREAM;
        int have_kind = 1;
        if (id == NOSTREAM) {
            continue;
        }
        if ((size_t)id >= cf->dir_len) {
            err = CFB_BAD_DIRECTORY;
            break;
        }
        if (visited[id]) {
            err = CFB_CYCLE_DETECTED;
            break;
        }
        visited[id] = 1;
        de = &cf->dir[id];
        switch (de->object_type) {
        case 1:
            kind = CFB_ENTRY_STORAGE;
            break;
        case 2:
            kind = CFB_ENTRY_STREAM;
            break;
        case 5:
            kind = CFB_ENTRY_ROOT_STORAGE;
            break;
        default:
            have_kind = 0;
            break;
        }
        if (have_kind) {
            err = push_entry(cf, ent_cap, de, kind, id);
            if (err != CFB_OK) {
                break;
            }
            if (kind == CFB_ENTRY_STORAGE) {
                if (!u32vec_push(&stack, &sp, &scap, de->child)) {
                    err = CFB_OUTPUT_TOO_LARGE;
                    break;
                }
            }
        }
        if (!u32vec_push(&stack, &sp, &scap, de->left) ||
            !u32vec_push(&stack, &sp, &scap, de->right)) {
            err = CFB_OUTPUT_TOO_LARGE;
            break;
        }
    }
    free(stack);
    return err;
}

static CfbError enumerate_entries(CompoundFile *cf) {
    size_t ent_cap = 0;
    uint8_t *visited;
    CfbError err;
    const DirEntry *root = &cf->dir[0];

    err = push_entry(cf, &ent_cap, root, CFB_ENTRY_ROOT_STORAGE, 0);
    if (err != CFB_OK) {
        return err;
    }
    if (root->child == NOSTREAM) {
        return CFB_OK;
    }
    visited = (uint8_t *)calloc(cf->dir_len, 1);
    if (!visited) {
        return CFB_OUTPUT_TOO_LARGE;
    }
    err = walk_tree(cf, root->child, visited, &ent_cap);
    free(visited);
    return err;
}

/* ── open ───────────────────────────────────────────────────────────────────*/
CfbError cfb_open(const uint8_t *bytes, size_t len, CompoundFile **out) {
    CompoundFile *cf;
    CfbError err;
    uint16_t sector_shift, mini_sector_shift;
    uint32_t num_fat_sectors, first_dir_sector, mini_cutoff;
    uint32_t first_minifat_sector, num_minifat_sectors;
    uint32_t first_difat_sector, num_difat_sectors;
    size_t total_sectors;
    uint32_t *fat_ids = NULL;
    size_t fat_ids_len = 0;

    *out = NULL;
    if (len < HEADER_LEN) {
        return CFB_TRUNCATED;
    }
    if (memcmp(bytes, SIGNATURE, 8) != 0) {
        return CFB_BAD_SIGNATURE;
    }
    if (!rd_u16(bytes, len, 30, &sector_shift)) {
        return CFB_TRUNCATED;
    }
    cf = (CompoundFile *)calloc(1, sizeof(CompoundFile));
    if (!cf) {
        return CFB_OUTPUT_TOO_LARGE;
    }
    if (sector_shift == 0x0009) {
        cf->sector_size = 512;
    } else if (sector_shift == 0x000C) {
        cf->sector_size = 4096;
    } else {
        free(cf);
        return CFB_UNSUPPORTED_SECTOR_SIZE;
    }
    if (!rd_u16(bytes, len, 32, &mini_sector_shift)) {
        free(cf);
        return CFB_TRUNCATED;
    }
    if (mini_sector_shift != 0x0006) {
        free(cf);
        return CFB_UNSUPPORTED_SECTOR_SIZE;
    }
    cf->mini_sector_size = (size_t)1 << mini_sector_shift;

    if (!rd_u32(bytes, len, 44, &num_fat_sectors) ||
        !rd_u32(bytes, len, 48, &first_dir_sector) ||
        !rd_u32(bytes, len, 56, &mini_cutoff) ||
        !rd_u32(bytes, len, 60, &first_minifat_sector) ||
        !rd_u32(bytes, len, 64, &num_minifat_sectors) ||
        !rd_u32(bytes, len, 68, &first_difat_sector) ||
        !rd_u32(bytes, len, 72, &num_difat_sectors)) {
        free(cf);
        return CFB_TRUNCATED;
    }
    cf->mini_cutoff = mini_cutoff;
    total_sectors = (len - HEADER_LEN) / cf->sector_size;

    cf->data = (uint8_t *)malloc(len);
    if (!cf->data) {
        free(cf);
        return CFB_OUTPUT_TOO_LARGE;
    }
    memcpy(cf->data, bytes, len);
    cf->data_len = len;

    err = collect_difat(cf, first_difat_sector, num_difat_sectors,
                        num_fat_sectors, total_sectors, &fat_ids, &fat_ids_len);
    if (err != CFB_OK) {
        cfb_free(cf);
        return err;
    }
    err = assemble_fat(cf, fat_ids, fat_ids_len, total_sectors);
    free(fat_ids);
    if (err != CFB_OK) {
        cfb_free(cf);
        return err;
    }

    /* mini-FAT (a regular-FAT stream reinterpreted as an array of u32s). */
    if (first_minifat_sector != ENDOFCHAIN && num_minifat_sectors != 0) {
        uint8_t *raw = NULL;
        size_t raw_len = 0, k, mcap = 0;
        err = read_fat_chain(cf, first_minifat_sector, 0, 0, &raw, &raw_len);
        if (err != CFB_OK) {
            cfb_free(cf);
            return err;
        }
        for (k = 0; k + 4 <= raw_len; k += 4) {
            uint32_t v;
            (void)rd_u32(raw, raw_len, k, &v);
            if (!u32vec_push(&cf->mini_fat, &cf->mini_fat_len, &mcap, v)) {
                free(raw);
                cfb_free(cf);
                return CFB_OUTPUT_TOO_LARGE;
            }
        }
        free(raw);
    }

    err = read_directory(cf, first_dir_sector);
    if (err != CFB_OK) {
        cfb_free(cf);
        return err;
    }
    if (cf->dir_len == 0) {
        cfb_free(cf);
        return CFB_BAD_DIRECTORY;
    }

    if (cf->dir[0].object_type != 5) {
        cfb_free(cf);
        return CFB_BAD_DIRECTORY;
    }
    if (cf->dir[0].size != 0) {
        uint8_t *ms = NULL;
        size_t ms_len = 0, want;
        err = read_fat_chain(cf, cf->dir[0].start_sector, 0, 0, &ms, &ms_len);
        if (err != CFB_OK) {
            cfb_free(cf);
            return err;
        }
        want = (size_t)cf->dir[0].size;
        if ((uint64_t)want != cf->dir[0].size || want > ms_len) {
            free(ms);
            cfb_free(cf);
            return CFB_BAD_DIRECTORY;
        }
        cf->mini_stream = ms;
        cf->mini_stream_len = want;
    }

    err = enumerate_entries(cf);
    if (err != CFB_OK) {
        cfb_free(cf);
        return err;
    }

    *out = cf;
    return CFB_OK;
}

void cfb_free(CompoundFile *cf) {
    if (!cf) {
        return;
    }
    free(cf->data);
    free(cf->fat);
    free(cf->mini_fat);
    free(cf->dir);
    free(cf->mini_stream);
    free(cf->entries);
    free(cf);
}

size_t cfb_sector_size(const CompoundFile *cf) { return cf->sector_size; }
size_t cfb_entry_count(const CompoundFile *cf) { return cf->entries_len; }
const CfbEntry *cfb_entry(const CompoundFile *cf, size_t i) {
    return i < cf->entries_len ? &cf->entries[i] : NULL;
}

CfbError cfb_read_stream_by_id(const CompoundFile *cf, uint32_t id,
                               uint8_t **out_data, size_t *out_len) {
    const DirEntry *entry;
    uint64_t size;
    uint8_t *bytes = NULL;
    size_t bytes_len = 0, want;
    CfbError err;

    *out_data = NULL;
    *out_len = 0;
    if ((size_t)id >= cf->dir_len) {
        return CFB_BAD_DIRECTORY;
    }
    entry = &cf->dir[id];
    if (entry->object_type != 2) {
        return CFB_NOT_A_STREAM;
    }
    size = entry->size;
    if (size > MAX_OUTPUT) {
        return CFB_OUTPUT_TOO_LARGE;
    }
    if (size == 0) {
        return CFB_OK; /* empty stream: *out_data NULL, *out_len 0 */
    }
    if (size < (uint64_t)cf->mini_cutoff) {
        err = read_mini_chain(cf, entry->start_sector, size, &bytes, &bytes_len);
    } else {
        err = read_fat_chain(cf, entry->start_sector, 1, size, &bytes,
                             &bytes_len);
    }
    if (err != CFB_OK) {
        return err;
    }
    want = (size_t)size;
    if ((uint64_t)want != size || want > bytes_len) {
        free(bytes);
        return CFB_BAD_SECTOR_CHAIN;
    }
    *out_data = bytes; /* over-allocated tail is harmless; out_len is exact */
    *out_len = want;
    return CFB_OK;
}

/* ASCII case-insensitive equality. */
static int ascii_ci_eq(const char *a, const char *b) {
    while (*a && *b) {
        unsigned char ca = (unsigned char)*a, cb = (unsigned char)*b;
        if (ca >= 'A' && ca <= 'Z') {
            ca = (unsigned char)(ca - 'A' + 'a');
        }
        if (cb >= 'A' && cb <= 'Z') {
            cb = (unsigned char)(cb - 'A' + 'a');
        }
        if (ca != cb) {
            return 0;
        }
        a++;
        b++;
    }
    return *a == '\0' && *b == '\0';
}

int cfb_read_stream(const CompoundFile *cf, const char *name,
                    uint8_t **out_data, size_t *out_len) {
    size_t i;
    *out_data = NULL;
    *out_len = 0;
    for (i = 0; i < cf->entries_len; i++) {
        if (cf->entries[i].kind == CFB_ENTRY_STREAM &&
            ascii_ci_eq(cf->entries[i].name, name)) {
            if (cfb_read_stream_by_id(cf, cf->entries[i].id, out_data,
                                      out_len) == CFB_OK) {
                return 1;
            }
            return 0;
        }
    }
    return 0;
}

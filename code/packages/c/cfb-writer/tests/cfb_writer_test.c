/* Tests for cfb-writer, using the header-only iso_test.h harness (pure ISO).
 *
 * The sibling `cfb` reader crate is not ported, so this test file carries a
 * compact CFB reader of its own (`extract_stream`) to prove the writer's output
 * is valid: we write streams, read them back by walking the FAT / mini-FAT
 * chains, and assert byte-for-byte equality — the same round-trip proof the
 * Rust crate's tests use. */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "cfb_writer.h"

#define SECTOR 512
#define HEADER 512
#define MINI_SECTOR 64
#define ENDOFCHAIN 0xFFFFFFFEu

static uint16_t rd_u16(const uint8_t *b, size_t o) {
    return (uint16_t)(b[o] | ((uint16_t)b[o + 1] << 8));
}
static uint32_t rd_u32(const uint8_t *b, size_t o) {
    return (uint32_t)b[o] | ((uint32_t)b[o + 1] << 8) |
           ((uint32_t)b[o + 2] << 16) | ((uint32_t)b[o + 3] << 24);
}
static uint64_t rd_u64(const uint8_t *b, size_t o) {
    uint64_t v = 0;
    int i;
    for (i = 0; i < 8; i++) v |= (uint64_t)b[o + (size_t)i] << (i * 8);
    return v;
}
/* Pointer to sector `sid` (sectors are numbered after the 512-byte header). */
static const uint8_t *sector(const uint8_t *b, uint32_t sid) {
    return b + HEADER + (size_t)sid * SECTOR;
}
/* fat[g] via the header DIFAT (<=109 FAT sectors, no DIFAT chain needed). */
static uint32_t fat_at(const uint8_t *b, uint32_t g) {
    uint32_t fat_sid = rd_u32(b, 76 + (size_t)(g / 128) * 4);
    return rd_u32(sector(b, fat_sid), (size_t)(g % 128) * 4);
}
/* Collect a regular-sector chain from `start` into a malloc'd buffer. */
static uint8_t *collect_chain(const uint8_t *b, uint32_t start, size_t *out_n) {
    uint8_t *acc = NULL;
    size_t n = 0;
    uint32_t s = start;
    while (s != ENDOFCHAIN && s < 0xFFFFFFF0u) {
        uint8_t *na = (uint8_t *)realloc(acc, n + SECTOR);
        if (na == NULL) {
            free(acc);
            return NULL;
        }
        acc = na;
        memcpy(acc + n, sector(b, s), SECTOR);
        n += SECTOR;
        s = fat_at(b, s);
    }
    *out_n = n;
    return acc;
}

/* Reconstruct the stream at directory index `idx`; returns malloc'd data. */
static uint8_t *extract_stream(const uint8_t *b, size_t idx, size_t *out_len) {
    uint32_t first_dir = rd_u32(b, 48);
    uint32_t mini_cutoff = rd_u32(b, 56);
    uint32_t first_minifat = rd_u32(b, 60);
    size_t dir_n = 0;
    uint8_t *dir = collect_chain(b, first_dir, &dir_n);
    const uint8_t *e;
    uint32_t start;
    uint64_t size;
    uint8_t *result;

    if (dir == NULL) return NULL;
    e = dir + idx * 128;
    start = rd_u32(e, 116);
    size = rd_u64(e, 120);

    if (size == 0) {
        free(dir);
        *out_len = 0;
        return (uint8_t *)malloc(1); /* non-NULL empty */
    }
    if (size >= (uint64_t)mini_cutoff) {
        /* large: a regular-sector chain */
        size_t chain_n = 0;
        uint8_t *chain = collect_chain(b, start, &chain_n);
        free(dir);
        if (chain == NULL) return NULL;
        result = (uint8_t *)malloc((size_t)size);
        if (result != NULL) memcpy(result, chain, (size_t)size);
        free(chain);
        *out_len = (size_t)size;
        return result;
    } else {
        /* mini: walk the mini-FAT over the root-owned mini-stream */
        uint32_t root_start = rd_u32(dir + 0 * 128, 116); /* mini-stream */
        size_t mini_n = 0, minifat_n = 0, got = 0;
        uint8_t *mini_stream = collect_chain(b, root_start, &mini_n);
        uint8_t *minifat = collect_chain(b, first_minifat, &minifat_n);
        uint32_t mi = start;
        free(dir);
        if (mini_stream == NULL || minifat == NULL) {
            free(mini_stream);
            free(minifat);
            return NULL;
        }
        result = (uint8_t *)malloc((size_t)size);
        if (result == NULL) {
            free(mini_stream);
            free(minifat);
            return NULL;
        }
        while (mi != ENDOFCHAIN && got < (size_t)size) {
            size_t take = (size_t)size - got;
            if (take > MINI_SECTOR) take = MINI_SECTOR;
            memcpy(result + got, mini_stream + (size_t)mi * MINI_SECTOR, take);
            got += take;
            mi = rd_u32(minifat, (size_t)mi * 4);
        }
        free(mini_stream);
        free(minifat);
        *out_len = (size_t)size;
        return result;
    }
}

/* Assert stream at dir index `idx` reconstructs to exactly `data[len]`. */
static void check_roundtrip(const uint8_t *cfb, size_t idx,
                            const uint8_t *data, size_t len) {
    size_t got = 0;
    uint8_t *out = extract_stream(cfb, idx, &got);
    ISO_CHECK_MSG(out != NULL, "extract_stream failed");
    if (out != NULL) {
        ISO_CHECK(got == len);
        if (got == len && len > 0) ISO_CHECK_MEM_EQ(out, data, len);
        free(out);
    }
}

int main(void) {
    static const uint8_t SIG[8] = {0xD0, 0xCF, 0x11, 0xE0,
                                   0xA1, 0xB1, 0x1A, 0xE1};

    /* ── mixed small + large round-trip ────────────────────────────────────*/
    {
        uint8_t *workbook = (uint8_t *)malloc(5000);
        uint8_t *another = (uint8_t *)malloc(100);
        const char *names[3] = {"Workbook", "SmallStream", "Another"};
        const uint8_t *datas[3];
        size_t lens[3] = {5000, 17, 100};
        uint8_t *cfb;
        size_t clen = 0;
        memset(workbook, 0xAB, 5000);
        memset(another, 0x01, 100);
        datas[0] = workbook;
        datas[1] = (const uint8_t *)"hello mini-stream";
        datas[2] = another;
        cfb = cfb_write(names, datas, lens, 3, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            ISO_CHECK_MEM_EQ(cfb, SIG, 8);
            ISO_CHECK((clen - HEADER) % SECTOR == 0);
            check_roundtrip(cfb, 1, workbook, 5000);
            check_roundtrip(cfb, 2, (const uint8_t *)"hello mini-stream", 17);
            check_roundtrip(cfb, 3, another, 100);
            free(cfb);
        }
        free(workbook);
        free(another);
    }

    /* ── header fields ─────────────────────────────────────────────────────*/
    {
        const char *names[1] = {"Only"};
        const uint8_t *datas[1] = {(const uint8_t *)"x"};
        size_t lens[1] = {1};
        size_t clen = 0;
        uint8_t *cfb = cfb_write(names, datas, lens, 1, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            ISO_CHECK_EQ_UINT(rd_u16(cfb, 26), 0x0003u); /* major v3 */
            ISO_CHECK_EQ_UINT(rd_u16(cfb, 30), 0x0009u); /* sector shift */
            ISO_CHECK_EQ_UINT(rd_u16(cfb, 32), 0x0006u); /* mini shift */
            ISO_CHECK_EQ_UINT(rd_u16(cfb, 28), 0xFFFEu); /* BOM */
            ISO_CHECK_EQ_UINT(rd_u32(cfb, 56), 4096u);   /* mini cutoff */
            ISO_CHECK((clen - HEADER) % SECTOR == 0);
            free(cfb);
        }
    }

    /* ── empty stream + a real one ─────────────────────────────────────────*/
    {
        const char *names[2] = {"Nothing", "Something"};
        const uint8_t *datas[2] = {(const uint8_t *)"", (const uint8_t *)"data"};
        size_t lens[2] = {0, 4};
        size_t clen = 0;
        uint8_t *cfb = cfb_write(names, datas, lens, 2, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            check_roundtrip(cfb, 1, NULL, 0);
            check_roundtrip(cfb, 2, (const uint8_t *)"data", 4);
            free(cfb);
        }
    }

    /* ── no streams: valid minimal CFB ─────────────────────────────────────*/
    {
        CfbWriter *w = cfb_writer_new();
        size_t clen = 0;
        uint8_t *cfb;
        ISO_CHECK(w != NULL);
        cfb = cfb_writer_finish(w, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            ISO_CHECK_MEM_EQ(cfb, SIG, 8);
            /* root entry (dir index 0) is OBJ_ROOT (0x05) at offset 66 */
            {
                uint32_t first_dir = rd_u32(cfb, 48);
                const uint8_t *root = sector(cfb, first_dir);
                ISO_CHECK(root[66] == 0x05);
            }
            free(cfb);
        }
    }

    /* ── exactly-cutoff is large; one under is mini ────────────────────────*/
    {
        uint8_t *at = (uint8_t *)malloc(4096);
        uint8_t *under = (uint8_t *)malloc(4095);
        const char *names[1];
        const uint8_t *datas[1];
        size_t lens[1];
        size_t clen = 0;
        uint8_t *cfb;
        memset(at, 0x7E, 4096);
        memset(under, 0x7E, 4095);
        names[0] = "AtCutoff";
        datas[0] = at;
        lens[0] = 4096;
        cfb = cfb_write(names, datas, lens, 1, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            check_roundtrip(cfb, 1, at, 4096);
            free(cfb);
        }
        names[0] = "JustUnder";
        datas[0] = under;
        lens[0] = 4095;
        cfb = cfb_write(names, datas, lens, 1, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            check_roundtrip(cfb, 1, under, 4095);
            free(cfb);
        }
        free(at);
        free(under);
    }

    /* ── many small streams spanning many mini-sectors ─────────────────────*/
    {
        CfbWriter *w = cfb_writer_new();
        uint8_t payloads[50][200];
        size_t plens[50];
        char nbuf[50][8];
        size_t i;
        uint8_t *cfb;
        size_t clen = 0;
        ISO_CHECK(w != NULL);
        for (i = 0; i < 50; i++) {
            size_t len = (i % 200) + 1;
            plens[i] = len;
            memset(payloads[i], (int)(i & 0xFF), len);
            nbuf[i][0] = 's';
            /* small integer to string */
            if (i < 10) {
                nbuf[i][1] = (char)('0' + (int)i);
                nbuf[i][2] = '\0';
            } else {
                nbuf[i][1] = (char)('0' + (int)(i / 10));
                nbuf[i][2] = (char)('0' + (int)(i % 10));
                nbuf[i][3] = '\0';
            }
            cfb_writer_add_stream(w, nbuf[i], payloads[i], len);
        }
        cfb = cfb_writer_finish(w, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            for (i = 0; i < 50; i++)
                check_roundtrip(cfb, i + 1, payloads[i], plens[i]);
            free(cfb);
        }
    }

    /* ── a huge stream needs > 1 FAT sector (fixed-point) ──────────────────*/
    {
        size_t big_len = 300u * 1024u;
        uint8_t *big = (uint8_t *)malloc(big_len);
        const char *names[1] = {"Huge"};
        const uint8_t *datas[1];
        size_t lens[1];
        size_t clen = 0, i;
        uint8_t *cfb;
        for (i = 0; i < big_len; i++) big[i] = (uint8_t)(i & 0xFF);
        datas[0] = big;
        lens[0] = big_len;
        cfb = cfb_write(names, datas, lens, 1, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            ISO_CHECK(rd_u32(cfb, 44) > 1); /* num FAT sectors */
            check_roundtrip(cfb, 1, big, big_len);
            free(cfb);
        }
        free(big);
    }

    /* ── overlong name truncated to 31 UTF-16 units ────────────────────────*/
    {
        char longname[101];
        const char *names[1];
        const uint8_t *datas[1] = {(const uint8_t *)"payload"};
        size_t lens[1] = {7};
        size_t clen = 0;
        uint8_t *cfb;
        memset(longname, 'A', 100);
        longname[100] = '\0';
        names[0] = longname;
        cfb = cfb_write(names, datas, lens, 1, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            uint32_t first_dir = rd_u32(cfb, 48);
            const uint8_t *entry = sector(cfb, first_dir) + 128; /* index 1 */
            /* name length incl NUL = (31 + 1) * 2 = 64 */
            ISO_CHECK_EQ_UINT(rd_u16(entry, 64), 64u);
            check_roundtrip(cfb, 1, (const uint8_t *)"payload", 7);
            free(cfb);
        }
    }

    /* ── UTF-8 name transcoded to UTF-16LE (café-Ω) ────────────────────────*/
    {
        /* "café-Ω" = c a f é(U+00E9) -(U+002D) Ω(U+03A9): 6 UTF-16 units */
        const char *names[1] = {"caf\xC3\xA9-\xCE\xA9"};
        const uint8_t *datas[1] = {(const uint8_t *)"unicode"};
        size_t lens[1] = {7};
        size_t clen = 0;
        uint8_t *cfb = cfb_write(names, datas, lens, 1, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            uint32_t first_dir = rd_u32(cfb, 48);
            const uint8_t *e = sector(cfb, first_dir) + 128; /* index 1 */
            static const uint8_t expect_name[14] = {
                'c', 0,    'a', 0,    'f',  0,    0xE9,
                0,   '-',  0,   0xA9, 0x03, 0,    0}; /* incl NUL */
            ISO_CHECK_EQ_UINT(rd_u16(e, 64), 14u); /* (6+1)*2 */
            ISO_CHECK_MEM_EQ(e, expect_name, 14);
            check_roundtrip(cfb, 1, (const uint8_t *)"unicode", 7);
            free(cfb);
        }
    }

    /* ── determinism: identical inputs → identical bytes ───────────────────*/
    {
        uint8_t *a5000 = (uint8_t *)malloc(5000);
        const char *names[2] = {"A", "B"};
        const uint8_t *datas[2];
        size_t lens[2] = {5000, 4};
        size_t l1 = 0, l2 = 0;
        uint8_t *c1, *c2;
        memset(a5000, 9, 5000);
        datas[0] = a5000;
        datas[1] = (const uint8_t *)"tiny";
        c1 = cfb_write(names, datas, lens, 2, &l1);
        c2 = cfb_write(names, datas, lens, 2, &l2);
        ISO_CHECK(c1 != NULL && c2 != NULL);
        if (c1 != NULL && c2 != NULL) {
            ISO_CHECK(l1 == l2);
            if (l1 == l2) ISO_CHECK_MEM_EQ(c1, c2, l1);
        }
        free(c1);
        free(c2);
        free(a5000);
    }

    /* ── mini-stream spanning multiple 512-byte sectors ────────────────────*/
    {
        CfbWriter *w = cfb_writer_new();
        uint8_t payloads[20][200];
        char nbuf[20][8];
        size_t i;
        uint8_t *cfb;
        size_t clen = 0;
        ISO_CHECK(w != NULL);
        for (i = 0; i < 20; i++) {
            memset(payloads[i], (int)((i + 1) & 0xFF), 200);
            nbuf[i][0] = 'm';
            if (i < 10) {
                nbuf[i][1] = (char)('0' + (int)i);
                nbuf[i][2] = '\0';
            } else {
                nbuf[i][1] = (char)('0' + (int)(i / 10));
                nbuf[i][2] = (char)('0' + (int)(i % 10));
                nbuf[i][3] = '\0';
            }
            cfb_writer_add_stream(w, nbuf[i], payloads[i], 200);
        }
        cfb = cfb_writer_finish(w, &clen);
        ISO_CHECK(cfb != NULL);
        if (cfb != NULL) {
            for (i = 0; i < 20; i++)
                check_roundtrip(cfb, i + 1, payloads[i], 200);
            free(cfb);
        }
    }

    return ISO_TEST_RESULT();
}

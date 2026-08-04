/*
 * Tests for cfb. The Rust crate's tests use a real embedded .xls fixture plus
 * crafted in-memory CFBs; we port the crafted builders (self-contained) so the
 * full read path — header, FAT, directory tree, mini-FAT / mini-stream, cycle
 * and bounds guards — is exercised without an external fixture. Uses the
 * header-only iso_test.h harness (pure ISO C17).
 */
#include "iso_test.h"

#include "cfb.h"

#include <stdlib.h>
#include <string.h>

#define HEADER_LEN 512
#define SECTOR 512
#define DIR_ENTRY_SIZE 128
#define FREESECT 0xFFFFFFFFu
#define ENDOFCHAIN 0xFFFFFFFEu
#define FATSECT 0xFFFFFFFDu
#define NOSTREAM 0xFFFFFFFFu
#define HEADER_DIFAT_OFFSET 76
#define HEADER_DIFAT_COUNT 109

static const uint8_t SIG[8] = {0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1};

static void wu16(uint8_t *b, size_t o, uint16_t v) {
    b[o] = (uint8_t)v;
    b[o + 1] = (uint8_t)(v >> 8);
}
static void wu32(uint8_t *b, size_t o, uint32_t v) {
    b[o] = (uint8_t)v;
    b[o + 1] = (uint8_t)(v >> 8);
    b[o + 2] = (uint8_t)(v >> 16);
    b[o + 3] = (uint8_t)(v >> 24);
}
static void wu64(uint8_t *b, size_t o, uint64_t v) {
    wu32(b, o, (uint32_t)v);
    wu32(b, o + 4, (uint32_t)(v >> 32));
}
/* Write a UTF-16LE name into a 128-byte entry starting at entry_off, and set
 * the name-length field (bytes incl. NUL) at entry_off+64. */
static void wname(uint8_t *b, size_t entry_off, const char *name) {
    size_t i = 0, n = 0;
    for (i = 0; name[i]; i++) {
        wu16(b, entry_off + i * 2, (uint16_t)(unsigned char)name[i]);
        n++;
    }
    wu16(b, entry_off + 64, (uint16_t)((n + 1) * 2));
}

/* Build a minimal valid CFB: sectors 0=FAT, 1=directory, 2=mini-FAT,
 * 3=mini-stream, with a "Tiny" 8-byte mini stream. Options:
 *   second   — add a second (empty) stream "Two" chained via left sibling
 *   cycle    — make "Two".left point back to entry 1 (directory-tree cycle)
 * Returns total length in *len (buffer must be >= HEADER_LEN + 4*SECTOR). */
static void craft_mini(uint8_t *buf, size_t *len, int second, int cycle) {
    size_t fat_off = HEADER_LEN, dir_off = HEADER_LEN + SECTOR;
    size_t mf_off = HEADER_LEN + 2 * SECTOR, ms_off = HEADER_LEN + 3 * SECTOR;
    size_t total = HEADER_LEN + 4 * SECTOR;
    size_t root, st, i;
    static const uint8_t payload[8] = {0xDE, 0xAD, 0xBE, 0xEF,
                                       0x01, 0x02, 0x03, 0x04};

    memset(buf, 0, total);
    memcpy(buf, SIG, 8);
    buf[30] = 0x09; /* sector shift -> 512 */
    buf[32] = 0x06; /* mini sector shift -> 64 */
    wu32(buf, 44, 1);          /* 1 FAT sector */
    wu32(buf, 48, 1);          /* dir @ sector 1 */
    wu32(buf, 56, 4096);       /* mini cutoff */
    wu32(buf, 60, 2);          /* mini-FAT @ sector 2 */
    wu32(buf, 64, 1);          /* 1 mini-FAT sector */
    wu32(buf, 68, ENDOFCHAIN); /* first DIFAT */
    wu32(buf, 72, 0);          /* num DIFAT sectors */
    wu32(buf, HEADER_DIFAT_OFFSET, 0); /* DIFAT[0] = FAT sector 0 */
    for (i = 1; i < HEADER_DIFAT_COUNT; i++) {
        wu32(buf, HEADER_DIFAT_OFFSET + i * 4, FREESECT);
    }

    /* FAT (sector 0). */
    for (i = 0; i < SECTOR / 4; i++) {
        wu32(buf, fat_off + i * 4, FREESECT);
    }
    wu32(buf, fat_off + 0 * 4, FATSECT);    /* sector 0 is the FAT */
    wu32(buf, fat_off + 1 * 4, ENDOFCHAIN); /* directory: 1 sector */
    wu32(buf, fat_off + 2 * 4, ENDOFCHAIN); /* mini-FAT: 1 sector */
    wu32(buf, fat_off + 3 * 4, ENDOFCHAIN); /* mini-stream: 1 sector */

    /* Directory (sector 1): root (id0) + stream "Tiny" (id1) [+ "Two" (id2)]. */
    root = dir_off;
    wname(buf, root, "Root Entry");
    buf[root + 66] = 5; /* root storage */
    wu32(buf, root + 68, NOSTREAM);
    wu32(buf, root + 72, NOSTREAM);
    wu32(buf, root + 76, 1); /* child = entry 1 */
    wu32(buf, root + 116, 3); /* mini-stream @ sector 3 */
    wu64(buf, root + 120, 64); /* mini-stream size */

    st = dir_off + DIR_ENTRY_SIZE;
    wname(buf, st, "Tiny");
    buf[st + 66] = 2; /* stream */
    wu32(buf, st + 68, second ? 2u : NOSTREAM); /* left -> "Two" if present */
    wu32(buf, st + 72, NOSTREAM);
    wu32(buf, st + 76, NOSTREAM);
    wu32(buf, st + 116, 0); /* mini-sector 0 */
    wu64(buf, st + 120, 8); /* 8 bytes */

    if (second) {
        size_t st2 = dir_off + 2 * DIR_ENTRY_SIZE;
        wname(buf, st2, "Two");
        buf[st2 + 66] = 2; /* stream */
        wu32(buf, st2 + 68, cycle ? 1u : NOSTREAM); /* left cycles back to 1 */
        wu32(buf, st2 + 72, NOSTREAM);
        wu32(buf, st2 + 76, NOSTREAM);
        wu32(buf, st2 + 116, 0);
        wu64(buf, st2 + 120, 0); /* empty */
    }

    /* mini-FAT (sector 2): mini-sector 0 is the whole stream. */
    for (i = 0; i < SECTOR / 4; i++) {
        wu32(buf, mf_off + i * 4, FREESECT);
    }
    wu32(buf, mf_off + 0 * 4, ENDOFCHAIN);

    /* mini-stream (sector 3): payload at mini-sector 0. */
    memcpy(buf + ms_off, payload, 8);

    *len = total;
}

/* Build a CFB whose directory sector-chain self-loops (FAT[1] = 1). */
static void craft_fat_cycle(uint8_t *buf, size_t *len) {
    size_t fat_off = HEADER_LEN, total = HEADER_LEN + 2 * SECTOR, i;
    memset(buf, 0, total);
    memcpy(buf, SIG, 8);
    buf[30] = 0x09;
    buf[32] = 0x06;
    wu32(buf, 44, 1); /* 1 FAT sector */
    wu32(buf, 48, 1); /* dir @ sector 1 */
    wu32(buf, 56, 4096);
    wu32(buf, 60, ENDOFCHAIN);
    wu32(buf, 64, 0);
    wu32(buf, 68, ENDOFCHAIN);
    wu32(buf, 72, 0);
    wu32(buf, HEADER_DIFAT_OFFSET, 0);
    for (i = 1; i < HEADER_DIFAT_COUNT; i++) {
        wu32(buf, HEADER_DIFAT_OFFSET + i * 4, FREESECT);
    }
    for (i = 0; i < SECTOR / 4; i++) {
        wu32(buf, fat_off + i * 4, FREESECT);
    }
    wu32(buf, fat_off + 0 * 4, FATSECT);
    wu32(buf, fat_off + 1 * 4, 1); /* POISON: directory sector 1 -> itself */
    *len = total;
}

int main(void) {
    uint8_t buf[HEADER_LEN + 4 * SECTOR];
    size_t len;

    /* ── mini-stream round-trip ───────────────────────────────────────────── */
    {
        CompoundFile *cf = NULL;
        uint8_t *data = NULL;
        size_t dlen = 0;
        static const uint8_t want[8] = {0xDE, 0xAD, 0xBE, 0xEF,
                                        0x01, 0x02, 0x03, 0x04};
        craft_mini(buf, &len, 0, 0);
        ISO_CHECK_EQ_INT(cfb_open(buf, len, &cf), CFB_OK);
        ISO_CHECK(cf != NULL);
        ISO_CHECK_EQ_UINT(cfb_sector_size(cf), 512);
        /* entries include the root storage */
        {
            size_t i, roots = 0, tinys = 0;
            for (i = 0; i < cfb_entry_count(cf); i++) {
                const CfbEntry *e = cfb_entry(cf, i);
                if (e->kind == CFB_ENTRY_ROOT_STORAGE) {
                    roots++;
                }
                if (e->kind == CFB_ENTRY_STREAM && strcmp(e->name, "Tiny") == 0) {
                    tinys++;
                }
            }
            ISO_CHECK(roots == 1);
            ISO_CHECK(tinys == 1);
        }
        ISO_CHECK(cfb_read_stream(cf, "Tiny", &data, &dlen));
        ISO_CHECK_EQ_UINT(dlen, 8);
        ISO_CHECK_MEM_EQ(data, want, 8);
        free(data);
        /* case-insensitive */
        ISO_CHECK(cfb_read_stream(cf, "TINY", &data, &dlen));
        free(data);
        ISO_CHECK(cfb_read_stream(cf, "tiny", &data, &dlen));
        free(data);
        ISO_CHECK(!cfb_read_stream(cf, "does-not-exist", &data, &dlen));
        /* read_stream_by_id on the root storage is NotAStream */
        ISO_CHECK_EQ_INT(cfb_read_stream_by_id(cf, 0, &data, &dlen),
                         CFB_NOT_A_STREAM);
        cfb_free(cf);
    }

    /* ── multi-entry flatten ──────────────────────────────────────────────── */
    {
        CompoundFile *cf = NULL;
        size_t i, streams = 0;
        craft_mini(buf, &len, 1, 0); /* add second stream "Two" */
        ISO_CHECK_EQ_INT(cfb_open(buf, len, &cf), CFB_OK);
        for (i = 0; i < cfb_entry_count(cf); i++) {
            if (cfb_entry(cf, i)->kind == CFB_ENTRY_STREAM) {
                streams++;
            }
        }
        ISO_CHECK_EQ_UINT(streams, 2); /* Tiny + Two */
        cfb_free(cf);
    }

    /* ── directory-tree cycle is detected, not hung ───────────────────────── */
    {
        CompoundFile *cf = NULL;
        craft_mini(buf, &len, 1, 1); /* "Two".left cycles back to entry 1 */
        ISO_CHECK_EQ_INT(cfb_open(buf, len, &cf), CFB_CYCLE_DETECTED);
        ISO_CHECK(cf == NULL);
    }

    /* ── FAT sector-chain cycle is detected, not hung ─────────────────────── */
    {
        CompoundFile *cf = NULL;
        CfbError err;
        craft_fat_cycle(buf, &len);
        err = cfb_open(buf, len, &cf);
        ISO_CHECK(err == CFB_CYCLE_DETECTED || err == CFB_BAD_SECTOR_CHAIN);
        ISO_CHECK(cf == NULL);
    }

    /* ── error paths ──────────────────────────────────────────────────────── */
    {
        CompoundFile *cf = NULL;
        uint8_t empty[1] = {0};
        ISO_CHECK_EQ_INT(cfb_open(empty, 0, &cf), CFB_TRUNCATED);

        /* valid signature but far too short for a header */
        {
            uint8_t sh[18];
            memcpy(sh, SIG, 8);
            memset(sh + 8, 0, 10);
            ISO_CHECK_EQ_INT(cfb_open(sh, sizeof sh, &cf), CFB_TRUNCATED);
        }

        /* bad signature */
        craft_mini(buf, &len, 0, 0);
        buf[0] = 0x00;
        ISO_CHECK_EQ_INT(cfb_open(buf, len, &cf), CFB_BAD_SIGNATURE);
        {
            uint8_t blob[600];
            memset(blob, 0, sizeof blob);
            ISO_CHECK_EQ_INT(cfb_open(blob, sizeof blob, &cf),
                             CFB_BAD_SIGNATURE);
        }

        /* unsupported sector shift */
        craft_mini(buf, &len, 0, 0);
        buf[30] = 0x0A;
        buf[31] = 0x00;
        ISO_CHECK_EQ_INT(cfb_open(buf, len, &cf),
                         CFB_UNSUPPORTED_SECTOR_SIZE);

        /* whole header present but body dropped: FAT/dir sectors point past EOF */
        craft_mini(buf, &len, 0, 0);
        {
            CompoundFile *c2 = NULL;
            CfbError err = cfb_open(buf, HEADER_LEN, &c2);
            ISO_CHECK(err != CFB_OK);
            ISO_CHECK(c2 == NULL);
        }
    }

    /* ── truncation fuzz: every prefix parses to an error or a clean file ──── */
    {
        size_t n;
        craft_mini(buf, &len, 1, 0);
        for (n = 0; n <= len; n++) {
            CompoundFile *cf = NULL;
            if (cfb_open(buf, n, &cf) == CFB_OK) {
                uint8_t *d = NULL;
                size_t dl = 0;
                (void)cfb_read_stream(cf, "Tiny", &d, &dl);
                free(d);
                cfb_free(cf);
            }
        }
        ISO_CHECK(1); /* survived every prefix */
    }

    return ISO_TEST_RESULT();
}

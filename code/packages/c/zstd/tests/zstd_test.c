/* Tests for the C zstd port (CMP07), using the iso_test.h harness. Covers
 * the 10 mandatory conformance cases from code/specs/CMP07-zstd.md plus
 * extra round-trip and robustness coverage.
 *
 * TC-9 (real `zstd` CLI interop, both directions) is the test that matters
 * most: see zstd.h's top-of-file warning and lessons.md Lesson 96 for why a
 * codec can pass every OTHER test here and still be silently non-conformant
 * to the real RFC 8878 wire format. It degrades gracefully (prints a notice
 * and returns) rather than failing when no `zstd` binary is on PATH. */
#include "iso_test.h"

#include <stdint.h> /* uint32_t */
#include <stdio.h>  /* FILE, fopen, fread, fwrite, fseek, ftell, remove,
                       snprintf, printf */
#include <stdlib.h> /* malloc, free, system */
#include <string.h> /* memcmp, memcpy, memset, strlen */

#include "zstd.h"

#if defined(_WIN32)
#define ZSTD_TEST_NULL_DEVICE "NUL"
#else
#define ZSTD_TEST_NULL_DEVICE "/dev/null"
#endif

/* ---- small helpers ------------------------------------------------------ */

static void check_round_trip(const unsigned char *data, size_t len) {
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0;
    ISO_CHECK_EQ_INT((int)zstd_compress(data, len, &comp, &comp_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_INT((int)zstd_decompress(comp, comp_len, &back, &back_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(back_len, len);
    if (back_len == len) {
        ISO_CHECK(len == 0 || memcmp(back, data, len) == 0);
    }
    free(comp);
    free(back);
}

/* count_blocks_in_our_frame — walk block headers in a frame WE produced.
 * Only valid for our own compress() output, which always uses a fixed
 * 13-byte header (magic 4 + FHD 1 + 8-byte FCS) — real-world frames (e.g.
 * from the `zstd` CLI) may use a different header layout and must not be
 * walked with this shortcut. Used by TC-7 to confirm multi-block splitting
 * actually happened, not just that the round trip succeeded. */
static size_t count_blocks_in_our_frame(const unsigned char *data,
                                         size_t len) {
    size_t pos = 13, blocks = 0;
    for (;;) {
        uint32_t hdr;
        unsigned btype;
        size_t bsize;
        int last;
        if (pos + 3 > len) {
            break;
        }
        hdr = (uint32_t)data[pos] | ((uint32_t)data[pos + 1] << 8) |
              ((uint32_t)data[pos + 2] << 16);
        pos += 3;
        last = (int)(hdr & 1u);
        btype = (unsigned)((hdr >> 1) & 3u);
        bsize = (size_t)(hdr >> 3);
        blocks++;
        pos += (btype == 1) ? 1 : bsize; /* RLE: 1 byte; Raw/Compressed: bsize */
        if (last) {
            break;
        }
    }
    return blocks;
}

/* ---- TC-1..TC-8: format-level round trips -------------------------------- */

static void tc1_empty(void) { check_round_trip((const unsigned char *)"", 0); }

static void tc2_single(void) {
    unsigned char b = 0x42;
    check_round_trip(&b, 1);
}

static void tc3_all_bytes(void) {
    unsigned char input[256];
    size_t i;
    for (i = 0; i < 256; i++) {
        input[i] = (unsigned char)i;
    }
    check_round_trip(input, 256);
}

static void tc4_rle(void) {
    unsigned char *input = (unsigned char *)malloc(1024);
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0;
    memset(input, 'A', 1024);
    ISO_CHECK_EQ_INT((int)zstd_compress(input, 1024, &comp, &comp_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_INT((int)zstd_decompress(comp, comp_len, &back, &back_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(back_len, 1024u);
    if (back && back_len == 1024) {
        ISO_CHECK(memcmp(back, input, 1024) == 0);
    }
    ISO_CHECK_MSG(comp_len < 30, "RLE of 1024 bytes should compress to < 30 bytes");
    free(input);
    free(comp);
    free(back);
}

static void tc5_prose(void) {
    const char *phrase = "the quick brown fox jumps over the lazy dog ";
    size_t one = strlen(phrase);
    size_t total = one * 25, i;
    unsigned char *input = (unsigned char *)malloc(total);
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0, threshold;

    for (i = 0; i < 25; i++) {
        memcpy(input + i * one, phrase, one);
    }

    ISO_CHECK_EQ_INT((int)zstd_compress(input, total, &comp, &comp_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_INT((int)zstd_decompress(comp, comp_len, &back, &back_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(back_len, total);
    if (back && back_len == total) {
        ISO_CHECK(memcmp(back, input, total) == 0);
    }
    threshold = total * 80 / 100;
    ISO_CHECK_MSG(comp_len < threshold,
                  "prose must compress to < 80% of input size");

    free(input);
    free(comp);
    free(back);
}

static void tc6_random(void) {
    unsigned char input[512];
    uint32_t seed = 42;
    size_t i;
    /* LCG: seed = seed*1664525 + 1013904223 (mod 2^32, via uint32_t
     * wraparound — well-defined unsigned overflow in C). */
    for (i = 0; i < 512; i++) {
        seed = seed * 1664525u + 1013904223u;
        input[i] = (unsigned char)(seed & 0xFFu);
    }
    check_round_trip(input, 512);
}

static void tc7_multiblock(void) {
    size_t total = 200 * 1024;
    unsigned char *input = (unsigned char *)malloc(total);
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0;
    memset(input, 'x', total);

    ISO_CHECK_EQ_INT((int)zstd_compress(input, total, &comp, &comp_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_INT((int)zstd_decompress(comp, comp_len, &back, &back_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(back_len, total);
    if (back && back_len == total) {
        ISO_CHECK(memcmp(back, input, total) == 0);
    }
    ISO_CHECK_MSG(count_blocks_in_our_frame(comp, comp_len) >= 2,
                  "200 KB input (> 128 KB block cap) must split into >= 2 blocks");

    free(input);
    free(comp);
    free(back);
}

static void tc8_repeat_offset(void) {
    const char *pattern = "ABCDEFGH";
    size_t pattern_len = 8, total = pattern_len + (128 + pattern_len) * 10;
    unsigned char *input = (unsigned char *)malloc(total);
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0, pos = 0, r, threshold;

    memcpy(input + pos, pattern, pattern_len);
    pos += pattern_len;
    for (r = 0; r < 10; r++) {
        memset(input + pos, 'X', 128);
        pos += 128;
        memcpy(input + pos, pattern, pattern_len);
        pos += pattern_len;
    }
    ISO_CHECK_EQ_UINT(pos, total);

    ISO_CHECK_EQ_INT((int)zstd_compress(input, total, &comp, &comp_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_INT((int)zstd_decompress(comp, comp_len, &back, &back_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(back_len, total);
    if (back && back_len == total) {
        ISO_CHECK(memcmp(back, input, total) == 0);
    }
    threshold = total * 70 / 100;
    ISO_CHECK_MSG(comp_len < threshold,
                  "repeat-offset pattern must compress to < 70% of input size");

    free(input);
    free(comp);
    free(back);
}

/* ---- TC-9: real `zstd` CLI interop --------------------------------------- */
/*
 * This is the test that actually proves the wire format is real RFC 8878,
 * not merely self-consistent. See zstd.h's top-of-file warning: a codec
 * whose encoder and decoder always agree with each other can still be
 * silently, systematically wrong — three such bugs (a fabricated FSE
 * table-spread, a wrong per-sequence field order, and a missing
 * last-sequence update skip) survived every other port's ENTIRE history
 * because this exact test didn't exist. It uses `system()` + temp files
 * rather than popen()/fork()+exec() so the same code compiles unmodified
 * under MSVC (no POSIX-only APIs) while still exercising a genuine external
 * process — pure ISO C's system() is standard (C17 §7.22.4.8).
 */

static int zstd_cli_available(void) {
    char cmd[256];
    int rc;
    (void)snprintf(cmd, sizeof cmd, "zstd --version >%s 2>&1",
                   ZSTD_TEST_NULL_DEVICE);
    rc = system(cmd);
    return rc == 0;
}

static int write_file(const char *path, const unsigned char *data,
                       size_t len) {
    FILE *f = fopen(path, "wb");
    size_t written;
    if (!f) {
        return 0;
    }
    written = (len == 0) ? 0 : fwrite(data, 1, len, f);
    fclose(f);
    return written == len;
}

/* read_file — read an entire file into a malloc'd buffer. Returns NULL for
 * both "couldn't open/read" and "empty file"; callers distinguish via
 * *out_len (0 either way, but non-NULL data is only ever produced for a
 * non-empty, successfully-read file, which is all these tests need). */
static unsigned char *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    long size;
    unsigned char *buf;
    size_t n;
    *out_len = 0;
    if (!f) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    size = ftell(f);
    if (size <= 0 || fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return NULL;
    }
    buf = (unsigned char *)malloc((size_t)size);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    n = fread(buf, 1, (size_t)size, f);
    fclose(f);
    if (n != (size_t)size) {
        free(buf);
        return NULL;
    }
    *out_len = (size_t)size;
    return buf;
}

/* run_cli_interop — shared body for TC-9 and the high-sequence-count
 * regression test below: build `original` (`len` bytes), then check both
 * interop directions against the real `zstd` CLI. `tag` disambiguates temp
 * file names between the two callers. */
static void run_cli_interop(const unsigned char *original, size_t len,
                             const char *tag) {
    char path_ours[64], path_input[64], path_theirs[64], path_decoded[64];
    char cmd[512];
    int rc;

    (void)snprintf(path_ours, sizeof path_ours, "zstd_test_%s_ours.zst", tag);
    (void)snprintf(path_input, sizeof path_input, "zstd_test_%s_input.bin",
                    tag);
    (void)snprintf(path_theirs, sizeof path_theirs, "zstd_test_%s_theirs.zst",
                    tag);
    (void)snprintf(path_decoded, sizeof path_decoded,
                    "zstd_test_%s_decoded.bin", tag);

    /* Direction 1: compress with ours, decompress with real `zstd -d`. */
    {
        unsigned char *comp = NULL, *decoded = NULL;
        size_t comp_len = 0, decoded_len = 0;
        ISO_CHECK_EQ_INT((int)zstd_compress(original, len, &comp, &comp_len),
                         (int)ZSTD_OK);
        ISO_CHECK(write_file(path_ours, comp, comp_len));
        free(comp);

        remove(path_decoded);
        (void)snprintf(cmd, sizeof cmd, "zstd -d -q -f -o %s %s >%s 2>&1",
                        path_decoded, path_ours, ZSTD_TEST_NULL_DEVICE);
        rc = system(cmd);
        ISO_CHECK_MSG(rc == 0,
                      "real `zstd -d` failed to decode our compressed output");

        decoded = read_file(path_decoded, &decoded_len);
        ISO_CHECK_EQ_UINT(decoded_len, len);
        if (decoded && decoded_len == len) {
            ISO_CHECK(memcmp(decoded, original, len) == 0);
        }
        free(decoded);
        remove(path_ours);
        remove(path_decoded);
    }

    /* Direction 2: compress with real `zstd`, decompress with ours. */
    {
        unsigned char *their_comp = NULL, *ours_decoded = NULL;
        size_t their_comp_len = 0, ours_decoded_len = 0;

        ISO_CHECK(write_file(path_input, original, len));
        remove(path_theirs);
        (void)snprintf(cmd, sizeof cmd, "zstd -q -f -o %s %s >%s 2>&1",
                        path_theirs, path_input, ZSTD_TEST_NULL_DEVICE);
        rc = system(cmd);
        ISO_CHECK_MSG(rc == 0, "real `zstd` CLI failed to compress our input");

        their_comp = read_file(path_theirs, &their_comp_len);
        ISO_CHECK(their_comp != NULL);
        if (their_comp) {
            ZstdStatus s = zstd_decompress(their_comp, their_comp_len,
                                            &ours_decoded, &ours_decoded_len);
            ISO_CHECK_EQ_INT((int)s, (int)ZSTD_OK);
            ISO_CHECK_EQ_UINT(ours_decoded_len, len);
            if (ours_decoded && ours_decoded_len == len) {
                ISO_CHECK(memcmp(ours_decoded, original, len) == 0);
            }
            free(ours_decoded);
        }
        free(their_comp);
        remove(path_input);
        remove(path_theirs);
    }
}

static void tc9_cli_interop(void) {
    const char *phrase = "the quick brown fox jumps over the lazy dog ";
    size_t one = strlen(phrase), total = one * 25, i;
    unsigned char *original;

    if (!zstd_cli_available()) {
        printf("  zstd CLI not found on PATH -- skipping TC-9 interop test\n");
        return;
    }

    original = (unsigned char *)malloc(total);
    for (i = 0; i < 25; i++) {
        memcpy(original + i * one, phrase, one);
    }

    run_cli_interop(original, total, "tc9");

    free(original);
}

/* Extra regression coverage (not one of the spec's 10 numbered cases): real
 * CLI interop on an input large enough to push a single block's sequence
 * count past 128 — exactly where encode_seq_count's wire encoding switches
 * from its 1-byte form to its 2-byte form. The marker-byte-order fix
 * documented on encode_seq_count round-trips fine against ITSELF but
 * silently produces a non-conformant frame without it; only a real
 * cross-implementation check like this one catches that class of bug. */
static void rt_cli_interop_high_sequence_count(void) {
    const char *src = "ABCDEF";
    size_t src_len = 6, total = 9000, i;
    unsigned char *original;

    if (!zstd_cli_available()) {
        printf("  zstd CLI not found on PATH -- skipping high-seq-count interop test\n");
        return;
    }

    original = (unsigned char *)malloc(total);
    for (i = 0; i < total; i++) {
        original[i] = (unsigned char)src[i % src_len];
    }

    run_cli_interop(original, total, "hiseq");

    free(original);
}

/* ---- TC-10: manual minimal raw-block frame ------------------------------- */

static void tc10_wire_format(void) {
    /* Magic + FHD(Single_Segment=1, FCS 1-byte=5, no checksum/dict) + FCS +
     * Block(Last=1, Raw, Size=5) + "hello". See code/specs/CMP07-zstd.md
     * TC-10 and the matching hand-derivation in the Rust reference's test. */
    unsigned char frame[] = {
        0x28, 0xB5, 0x2F, 0xFD, /* magic */
        0x20,                    /* FHD: Single_Segment=1, FCS_flag=00(1 byte) */
        0x05,                    /* FCS = 5 */
        0x29, 0x00, 0x00,        /* block header: last=1, Raw, size=5 */
        'h',  'e',  'l',  'l',  'o',
    };
    unsigned char *out = NULL;
    size_t out_len = 0;
    ISO_CHECK_EQ_INT((int)zstd_decompress(frame, sizeof frame, &out, &out_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(out_len, 5u);
    if (out && out_len == 5) {
        ISO_CHECK(memcmp(out, "hello", 5) == 0);
    }
    free(out);
}

/* ---- extra round-trip coverage ------------------------------------------- */

static void rt_binary_data(void) {
    unsigned char input[300];
    size_t i;
    for (i = 0; i < 300; i++) {
        input[i] = (unsigned char)(i % 256);
    }
    check_round_trip(input, 300);
}

static void rt_all_zeros(void) {
    unsigned char input[1000];
    memset(input, 0, sizeof input);
    check_round_trip(input, sizeof input);
}

static void rt_all_ff(void) {
    unsigned char input[1000];
    memset(input, 0xFF, sizeof input);
    check_round_trip(input, sizeof input);
}

static void rt_determinism(void) {
    /* Compressing the same data twice must produce byte-identical output —
     * required for reproducible builds and cache invalidation. */
    const char *phrase = "hello, ZStd world! ";
    size_t one = strlen(phrase), total = one * 50, i;
    unsigned char *input = (unsigned char *)malloc(total);
    unsigned char *comp1 = NULL, *comp2 = NULL;
    size_t comp1_len = 0, comp2_len = 0;

    for (i = 0; i < 50; i++) {
        memcpy(input + i * one, phrase, one);
    }
    ISO_CHECK_EQ_INT((int)zstd_compress(input, total, &comp1, &comp1_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_INT((int)zstd_compress(input, total, &comp2, &comp2_len),
                     (int)ZSTD_OK);
    ISO_CHECK_EQ_UINT(comp1_len, comp2_len);
    if (comp1_len == comp2_len) {
        ISO_CHECK(memcmp(comp1, comp2, comp1_len) == 0);
    }

    free(input);
    free(comp1);
    free(comp2);
}

/* ---- robustness: malformed / adversarial decompress input ---------------- */

static void rt_malformed_decompress_safety(void) {
    unsigned char *out;
    size_t out_len;

    /* Too short to even hold a magic number. */
    {
        unsigned char tiny[] = {0x28, 0xB5};
        out = NULL;
        out_len = 99;
        ISO_CHECK_EQ_INT((int)zstd_decompress(tiny, sizeof tiny, &out, &out_len),
                         (int)ZSTD_ERR_FORMAT);
        ISO_CHECK(out == NULL);
        ISO_CHECK_EQ_UINT(out_len, 0u);
    }

    /* Bad magic number entirely. */
    {
        unsigned char bad_magic[] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
        out = NULL;
        ISO_CHECK_EQ_INT(
            (int)zstd_decompress(bad_magic, sizeof bad_magic, &out, &out_len),
            (int)ZSTD_ERR_FORMAT);
        ISO_CHECK(out == NULL);
    }

    /* Valid magic + FHD but a block claiming a size far beyond the buffer:
     * must be rejected, not read out of bounds. */
    {
        unsigned char truncated[] = {
            0x28, 0xB5, 0x2F, 0xFD, /* magic */
            0x20,                   /* FHD: Single_Segment, FCS 1-byte */
            0xFF,                   /* FCS = 255 (irrelevant, untrusted) */
            0xF9, 0xFF, 0xFF,       /* block hdr: huge bogus size */
        };
        out = NULL;
        ISO_CHECK_EQ_INT(
            (int)zstd_decompress(truncated, sizeof truncated, &out, &out_len),
            (int)ZSTD_ERR_FORMAT);
        ISO_CHECK(out == NULL);
    }

    /* A block claiming Block_Size just over the 128 KB security cap, even
     * though the buffer conveniently doesn't back it — the cap check must
     * fire before any bounds arithmetic on the (adversarial) size. */
    {
        unsigned char oversized_claim[] = {
            0x28, 0xB5, 0x2F, 0xFD, /* magic */
            0x20,                   /* FHD */
            0x00,                   /* FCS = 0 */
            /* header encodes bsize = (1<<17)+1 = 131073, last=1,
             * type=Raw(00): hdr = (131073 << 3) | (0 << 1) | 1 = 0x100009 */
            0x09, 0x00, 0x10,
        };
        out = NULL;
        ISO_CHECK_EQ_INT((int)zstd_decompress(oversized_claim,
                                               sizeof oversized_claim, &out,
                                               &out_len),
                         (int)ZSTD_ERR_FORMAT);
        ISO_CHECK(out == NULL);
    }

    /* Reserved block type (11) must be rejected. */
    {
        unsigned char reserved_type[] = {
            0x28, 0xB5, 0x2F, 0xFD, /* magic */
            0x20,                   /* FHD */
            0x00,                   /* FCS = 0 */
            0x07, 0x00, 0x00,       /* hdr: last=1, type=11(reserved), size=0 */
        };
        out = NULL;
        ISO_CHECK_EQ_INT(
            (int)zstd_decompress(reserved_type, sizeof reserved_type, &out,
                                  &out_len),
            (int)ZSTD_ERR_FORMAT);
        ISO_CHECK(out == NULL);
    }
}

int main(void) {
    tc1_empty();
    tc2_single();
    tc3_all_bytes();
    tc4_rle();
    tc5_prose();
    tc6_random();
    tc7_multiblock();
    tc8_repeat_offset();
    tc9_cli_interop();
    tc10_wire_format();

    rt_cli_interop_high_sequence_count();
    rt_binary_data();
    rt_all_zeros();
    rt_all_ff();
    rt_determinism();
    rt_malformed_decompress_safety();

    return ISO_TEST_RESULT();
}

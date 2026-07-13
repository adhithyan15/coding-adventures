/*
 * Tests for the C intel-8008-packager library, using the header-only iso_test.h
 * harness (pure ISO). Byte vectors and error expectations mirror the Rust
 * crate's own unit tests one-for-one.
 */
#include "iso_test.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* strcmp, strncmp, strstr, strlen */

#include "intel_8008_packager.h"

/* Encode helper: returns the malloc'd string (caller frees), or NULL on error. */
static char *enc(const uint8_t *bin, size_t n, size_t origin) {
    char *out = NULL;
    size_t len = 0;
    if (pak_encode_hex(bin, n, origin, &out, &len) != PAK_OK) return NULL;
    return out;
}

/* Count '\n'-terminated lines in `s`. */
static size_t count_lines(const char *s) {
    size_t c = 0;
    for (const char *p = s; *p; p++)
        if (*p == '\n') c++;
    return c;
}

/* Decode helper: status only, freeing any result. */
static PakStatus dec_status(const char *text) {
    PakDecoded d;
    PakStatus st = pak_decode_hex(text, &d);
    pak_decoded_free(&d);
    return st;
}

int main(void) {
    /* ── encode: exact small vectors ──────────────────────────────────── */
    {
        uint8_t b1[1] = {0xFF};
        char *s = enc(b1, 1, 0);
        ISO_CHECK(s != NULL);
        ISO_CHECK_STR_EQ(s, ":01000000FF00\n:00000001FF\n");
        free(s);

        uint8_t b3[3] = {0x06, 0x00, 0xFF}; /* MVI B,0; HLT */
        s = enc(b3, 3, 0);
        ISO_CHECK(s != NULL);
        ISO_CHECK_STR_EQ(s, ":030000000600FFF8\n:00000001FF\n");
        free(s);
    }

    /* ── encode: structural checks ────────────────────────────────────── */
    {
        uint8_t b3[3] = {0x01, 0x02, 0x03};
        char *s = enc(b3, 3, 0);
        ISO_CHECK(s != NULL);
        ISO_CHECK(s[0] == ':'); /* every record starts with ':' */
        ISO_CHECK(count_lines(s) == 2);
        ISO_CHECK(strstr(s, ":00000001FF\n") != NULL); /* EOF present */
        free(s);

        /* three-byte program field layout */
        uint8_t prog[3] = {0x06, 0x00, 0xFF};
        s = enc(prog, 3, 0);
        ISO_CHECK(strncmp(s + 1, "03", 2) == 0);     /* byte count */
        ISO_CHECK(strncmp(s + 3, "0000", 4) == 0);   /* address */
        ISO_CHECK(strncmp(s + 7, "00", 2) == 0);     /* record type DATA */
        ISO_CHECK(strncmp(s + 9, "0600FF", 6) == 0); /* data bytes */
        free(s);

        /* 16 bytes -> one data record + EOF; byte count "10" */
        uint8_t b16[16];
        for (int i = 0; i < 16; i++) b16[i] = (uint8_t)i;
        s = enc(b16, 16, 0);
        ISO_CHECK(count_lines(s) == 2 && strncmp(s + 1, "10", 2) == 0);
        free(s);

        /* 17 bytes -> two data records (16 + 1) + EOF */
        uint8_t b17[17];
        for (int i = 0; i < 17; i++) b17[i] = (uint8_t)i;
        s = enc(b17, 17, 0);
        ISO_CHECK(count_lines(s) == 3 && strncmp(s + 1, "10", 2) == 0);
        {
            const char *l2 = strchr(s, '\n') + 1;
            ISO_CHECK(strncmp(l2 + 1, "01", 2) == 0); /* second record: 1 byte */
        }
        free(s);

        /* address increments by 16 for the second record */
        uint8_t z32[32] = {0};
        s = enc(z32, 32, 0);
        ISO_CHECK(strncmp(s + 3, "0000", 4) == 0);
        {
            const char *l2 = strchr(s, '\n') + 1;
            ISO_CHECK(strncmp(l2 + 3, "0010", 4) == 0);
        }
        free(s);

        /* non-zero origin, large origin */
        uint8_t b4[4] = {0x7C, 0x03, 0x00, 0xFF};
        s = enc(b4, 4, 0x0100);
        ISO_CHECK(strncmp(s + 3, "0100", 4) == 0);
        free(s);
        uint8_t z4[4] = {0};
        s = enc(z4, 4, 0x2000);
        ISO_CHECK(strncmp(s + 3, "2000", 4) == 0);
        free(s);
    }

    /* ── encode: checksum property (all record bytes sum to 0 mod 256) ── */
    {
        uint8_t b3[3] = {0x06, 0x00, 0xFF};
        /* Each non-EOF record's byte-sum (checksum included) must be 0 mod 256. */
        char *s = enc(b3, 3, 0);
        for (const char *line = s; *line;) {
            const char *nl = strchr(line, '\n');
            size_t linelen = nl ? (size_t)(nl - line) : strlen(line);
            if (!(linelen == 11 && strncmp(line, ":00000001FF", 11) == 0)) {
                unsigned sum = 0;
                for (size_t i = 1; i + 1 < linelen; i += 2) {
                    char pair[3] = {line[i], line[i + 1], 0};
                    sum += (unsigned)strtoul(pair, NULL, 16);
                }
                ISO_CHECK(sum % 256 == 0);
            }
            line = nl ? nl + 1 : line + linelen;
        }
        free(s);
    }

    /* ── encode: error cases ──────────────────────────────────────────── */
    {
        char *out = NULL;
        size_t len = 0;
        ISO_CHECK(pak_encode_hex(NULL, 0, 0, &out, &len) == PAK_ERR_EMPTY_BINARY);
        uint8_t two[2] = {1, 2};
        ISO_CHECK(pak_encode_hex(two, 2, 0xFFFF, &out, &len) == PAK_ERR_IMAGE_OVERFLOW);
        uint8_t one[1] = {1};
        ISO_CHECK(pak_encode_hex(one, 1, 0x10000, &out, &len) == PAK_ERR_ORIGIN_TOO_LARGE);
    }

    /* ── decode: round trips ──────────────────────────────────────────── */
    {
        struct { size_t n; size_t origin; } cases[] = {
            {1, 0}, {3, 0}, {17, 0}, {4, 0x0100}, {16, 0x3FF0}};
        for (size_t k = 0; k < sizeof(cases) / sizeof(cases[0]); k++) {
            size_t n = cases[k].n;
            uint8_t *bin = (uint8_t *)malloc(n);
            for (size_t i = 0; i < n; i++) bin[i] = (uint8_t)(i * 7 + 1);
            char *hex = enc(bin, n, cases[k].origin);
            ISO_CHECK(hex != NULL);
            PakDecoded d;
            ISO_CHECK(pak_decode_hex(hex, &d) == PAK_OK);
            ISO_CHECK(d.origin == cases[k].origin);
            ISO_CHECK(d.binary_len == n && memcmp(d.binary, bin, n) == 0);
            pak_decoded_free(&d);
            free(hex);
            free(bin);
        }

        /* full 16 KB round trip (span exactly PAK_MAX_IMAGE_SIZE, allowed) */
        size_t n = PAK_MAX_IMAGE_SIZE;
        uint8_t *bin = (uint8_t *)malloc(n);
        memset(bin, 0xFF, n);
        char *hex = enc(bin, n, 0);
        ISO_CHECK(hex != NULL);
        PakDecoded d;
        ISO_CHECK(pak_decode_hex(hex, &d) == PAK_OK);
        ISO_CHECK(d.binary_len == n);
        pak_decoded_free(&d);
        free(hex);
        free(bin);
    }

    /* ── decode: error cases ──────────────────────────────────────────── */
    {
        ISO_CHECK(dec_status("03000000060000F7\n:00000001FF\n") == PAK_ERR_MISSING_COLON);
        ISO_CHECK(dec_status(":0ZZZZ000060000F7\n:00000001FF\n") == PAK_ERR_INVALID_HEX);
        ISO_CHECK(dec_status(":100\n:00000001FF\n") == PAK_ERR_INVALID_HEX); /* odd body */
        ISO_CHECK(dec_status(":020000020000FC\n:00000001FF\n") == PAK_ERR_UNSUPPORTED_TYPE);
        ISO_CHECK(dec_status(":050000000102\n:00000001FF\n") == PAK_ERR_RECORD_TOO_SHORT);
        ISO_CHECK(dec_status(":00000001FF\n") == PAK_OK);          /* EOF-only */
        ISO_CHECK(dec_status(":0100000000FF\n") == PAK_ERR_MISSING_EOF);

        /* bad checksum: encode [1,2,3], corrupt the first record's checksum */
        {
            uint8_t b[3] = {1, 2, 3};
            char *s = enc(b, 3, 0);
            char *nl = strchr(s, '\n');
            nl[-1] = '0';
            nl[-2] = '0';
            ISO_CHECK(dec_status(s) == PAK_ERR_BAD_CHECKSUM);
            free(s);
        }

        /* image too large: records at 0x0000 and 0x4001 -> span 0x4002 */
        ISO_CHECK(dec_status(":0100000000FF\n:01400100FFBF\n:00000001FF\n") ==
                  PAK_ERR_IMAGE_TOO_LARGE);

        /* overlapping: 16-byte record at 0 + 1-byte record inside it */
        {
            uint8_t b16[16] = {0};
            char *hexa = enc(b16, 16, 0);
            char *nl = strchr(hexa, '\n');
            size_t reclen = (size_t)(nl - hexa);
            char *combined = (char *)malloc(reclen + 64);
            memcpy(combined, hexa, reclen);
            strcpy(combined + reclen, "\n:0100050000FA\n:00000001FF\n");
            ISO_CHECK(dec_status(combined) == PAK_ERR_OVERLAP);
            free(combined);
            free(hexa);
        }

        /* duplicate address */
        ISO_CHECK(dec_status(":0100000042BD\n:0100000042BD\n:00000001FF\n") ==
                  PAK_ERR_OVERLAP);

        /* out-of-order overlapping (record at 0x0005 before 16-byte at 0) */
        {
            uint8_t b16[16] = {0};
            char *hexa = enc(b16, 16, 0);
            char *nl = strchr(hexa, '\n');
            size_t reclen = (size_t)(nl - hexa);
            char *combined = (char *)malloc(reclen + 64);
            strcpy(combined, ":0100050000FA\n");
            size_t pos = strlen(combined);
            memcpy(combined + pos, hexa, reclen);
            strcpy(combined + pos + reclen, "\n:00000001FF\n");
            ISO_CHECK(dec_status(combined) == PAK_ERR_OVERLAP);
            free(combined);
            free(hexa);
        }

        /* line too long: ':' followed by 600 'AA' pairs = 1201 chars */
        {
            size_t body = 1200;
            char *ll = (char *)malloc(1 + body + 32);
            ll[0] = ':';
            for (size_t i = 0; i < body; i++) ll[1 + i] = 'A';
            strcpy(ll + 1 + body, "\n:00000001FF\n");
            ISO_CHECK(dec_status(ll) == PAK_ERR_LINE_TOO_LONG);
            free(ll);
        }
    }

    /* ── error-message keywords match the Rust PackagerError text ─────── */
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_EMPTY_BINARY), "non-empty") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_MISSING_COLON), "':'") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_BAD_CHECKSUM), "checksum") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_UNSUPPORTED_TYPE), "unsupported") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_IMAGE_TOO_LARGE), "large") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_MISSING_EOF), "EOF") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_OVERLAP), "overlap") != NULL);
    ISO_CHECK(strstr(pak_error_message(PAK_ERR_LINE_TOO_LONG), "long") != NULL);

    return ISO_TEST_RESULT();
}

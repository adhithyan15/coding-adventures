/* Tests for the C argon2id, using the header-only iso_test.h harness (pure ISO).
 * The primary vector is RFC 9106 §5.3 (Argon2id), matching the Rust crate. */
#include "iso_test.h"

#include <string.h> /* memcmp, memset */

#include "argon2id.h"

/* Decode a lowercase hex string into `out`. */
static void from_hex(const char *hex, uint8_t *out) {
    size_t i;
    for (i = 0; hex[i] && hex[i + 1]; i += 2) {
        int hi = hex[i] <= '9' ? hex[i] - '0' : (hex[i] | 0x20) - 'a' + 10;
        int lo = hex[i + 1] <= '9' ? hex[i + 1] - '0'
                                   : (hex[i + 1] | 0x20) - 'a' + 10;
        out[i / 2] = (uint8_t)((hi << 4) | lo);
    }
}

int main(void) {
    /* ── RFC 9106 §5.3 Argon2id known-answer ─────────────────────────────── */
    {
        uint8_t password[32];
        uint8_t salt[16];
        uint8_t key[8];
        uint8_t ad[12];
        uint8_t tag[32];
        uint8_t want[32];
        Argon2idOptions opts;
        memset(password, 0x01, 32);
        memset(salt, 0x02, 16);
        memset(key, 0x03, 8);
        memset(ad, 0x04, 12);
        opts.key = key;
        opts.key_len = 8;
        opts.associated_data = ad;
        opts.ad_len = 12;
        opts.version = 0; /* default 0x13 */
        ISO_CHECK_EQ_INT(
            (int)argon2id(password, 32, salt, 16, 3, 32, 4, 32, &opts, tag),
            (int)ARGON2ID_OK);
        from_hex("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659",
                 want);
        ISO_CHECK_MEM_EQ(tag, want, 32);
    }

    /* ── determinism, and sensitivity to inputs ─────────────────────────── */
    {
        uint8_t a[32];
        uint8_t b[32];
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"password", 8,
                                      (const uint8_t *)"somesalt", 8, 1, 8, 1, 32,
                                      NULL, a),
                         (int)ARGON2ID_OK);
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"password", 8,
                                      (const uint8_t *)"somesalt", 8, 1, 8, 1, 32,
                                      NULL, b),
                         (int)ARGON2ID_OK);
        ISO_CHECK_MEM_EQ(a, b, 32); /* deterministic */

        argon2id((const uint8_t *)"password1", 9, (const uint8_t *)"somesalt", 8,
                1, 8, 1, 32, NULL, a);
        argon2id((const uint8_t *)"password2", 9, (const uint8_t *)"somesalt", 8,
                1, 8, 1, 32, NULL, b);
        ISO_CHECK(memcmp(a, b, 32) != 0); /* different passwords */

        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsalt", 8, 1,
                8, 1, 32, NULL, a);
        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsal2", 8, 1,
                8, 1, 32, NULL, b);
        ISO_CHECK(memcmp(a, b, 32) != 0); /* different salts */

        /* more passes change the output. */
        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsalt", 8, 1,
                8, 1, 32, NULL, a);
        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsalt", 8, 2,
                8, 1, 32, NULL, b);
        ISO_CHECK(memcmp(a, b, 32) != 0);
    }

    /* ── key and associated data bind the output ────────────────────────── */
    {
        uint8_t base[32];
        uint8_t withkey[32];
        uint8_t withad[32];
        Argon2idOptions ko;
        Argon2idOptions ao;
        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsalt", 8, 1,
                8, 1, 32, NULL, base);
        ko.key = (const uint8_t *)"secret!!";
        ko.key_len = 8;
        ko.associated_data = NULL;
        ko.ad_len = 0;
        ko.version = 0;
        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsalt", 8, 1,
                8, 1, 32, &ko, withkey);
        ISO_CHECK(memcmp(base, withkey, 32) != 0);
        ao.key = NULL;
        ao.key_len = 0;
        ao.associated_data = (const uint8_t *)"ad";
        ao.ad_len = 2;
        ao.version = 0;
        argon2id((const uint8_t *)"password", 8, (const uint8_t *)"saltsalt", 8, 1,
                8, 1, 32, &ao, withad);
        ISO_CHECK(memcmp(base, withad, 32) != 0);
    }

    /* ── tag length variants (including > 64, exercising H') ─────────────── */
    {
        uint32_t lens[6];
        size_t li;
        uint8_t buf[128];
        lens[0] = 4;
        lens[1] = 16;
        lens[2] = 32;
        lens[3] = 64;
        lens[4] = 65;
        lens[5] = 128;
        for (li = 0; li < 6; li++) {
            ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"password", 8,
                                          (const uint8_t *)"saltsalt", 8, 1, 8, 1,
                                          lens[li], NULL, buf),
                             (int)ARGON2ID_OK);
        }
    }

    /* ── multi-lane parameters produce a 32-byte tag ────────────────────── */
    {
        uint8_t password[32];
        uint8_t salt[16];
        uint8_t tag[32];
        memset(password, 0x01, 32);
        memset(salt, 0x02, 16);
        ISO_CHECK_EQ_INT(
            (int)argon2id(password, 32, salt, 16, 3, 32, 4, 32, NULL, tag),
            (int)ARGON2ID_OK);
    }

    /* ── parameter validation (mirrors the Rust unit tests) ─────────────── */
    {
        uint8_t tag[32];
        Argon2idOptions badver;
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"pw", 2,
                                      (const uint8_t *)"short", 5, 1, 8, 1, 32,
                                      NULL, tag),
                         (int)ARGON2ID_SALT_TOO_SHORT);
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"pw", 2,
                                      (const uint8_t *)"saltsalt", 8, 1, 8, 1, 3,
                                      NULL, tag),
                         (int)ARGON2ID_TAG_TOO_SMALL);
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"pw", 2,
                                      (const uint8_t *)"saltsalt", 8, 1, 1, 1, 32,
                                      NULL, tag),
                         (int)ARGON2ID_MEMORY_TOO_SMALL);
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"pw", 2,
                                      (const uint8_t *)"saltsalt", 8, 0, 8, 1, 32,
                                      NULL, tag),
                         (int)ARGON2ID_TIME_COST_ZERO);
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"pw", 2,
                                      (const uint8_t *)"saltsalt", 8, 1, 8, 0, 32,
                                      NULL, tag),
                         (int)ARGON2ID_INVALID_PARALLELISM);
        badver.key = NULL;
        badver.key_len = 0;
        badver.associated_data = NULL;
        badver.ad_len = 0;
        badver.version = 0x10;
        ISO_CHECK_EQ_INT((int)argon2id((const uint8_t *)"pw", 2,
                                      (const uint8_t *)"saltsalt", 8, 1, 8, 1, 32,
                                      &badver, tag),
                         (int)ARGON2ID_UNSUPPORTED_VERSION);
    }

    return ISO_TEST_RESULT();
}

/*
 * Tests for intel-8008-assembler, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's unit tests.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "intel_8008_assembler.h"

/* Encode one instruction and compare against `expect` (length `n`). */
static int enc_eq(const char *m, const char *const *ops, size_t nops,
                  const Intel8008Symbols *syms, size_t pc,
                  const uint8_t *expect, size_t n) {
    uint8_t *out = NULL;
    size_t len = 0;
    char err[128];
    int ok;
    if (intel8008_encode_instruction(m, ops, nops, syms, pc, &out, &len, err,
                                     sizeof err) != INTEL8008_OK) {
        return 0;
    }
    ok = (len == n) && (n == 0 || memcmp(out, expect, n) == 0);
    free(out);
    return ok;
}

/* Assemble `src` and compare bytes against `expect` (length `n`). */
static int asm_eq(const char *src, const uint8_t *expect, size_t n) {
    uint8_t *out = NULL;
    size_t len = 0;
    char err[128];
    int ok;
    if (intel8008_assemble(src, &out, &len, err, sizeof err) != INTEL8008_OK) {
        return 0;
    }
    ok = (len == n) && (n == 0 || memcmp(out, expect, n) == 0);
    free(out);
    return ok;
}

int main(void) {
    char err[128];

    /* ── instruction sizes ─────────────────────────────────────────────────*/
    {
        const char *one[] = {"HLT", "RFC", "RET", "RLC", "RRC", "RAL", "RAR",
                             "RFZ", "RTC", "ADD", "ADC", "SUB", "SBB", "ANA",
                             "XRA", "ORA", "CMP"};
        const char *two[] = {"MVI", "ADI", "ACI", "SUI", "SBI",
                             "ANI", "XRI", "ORI", "CPI"};
        const char *three[] = {"JMP", "CAL", "JFC", "JTC", "JFZ", "JTZ",
                               "JFP", "JTP", "CFC", "CTC", "CFZ", "CTZ"};
        size_t i, sz;
        for (i = 0; i < sizeof one / sizeof one[0]; i++) {
            ISO_CHECK_EQ_INT(
                intel8008_instruction_size(one[i], &sz, err, sizeof err),
                INTEL8008_OK);
            ISO_CHECK_EQ_UINT(sz, 1u);
        }
        for (i = 0; i < sizeof two / sizeof two[0]; i++) {
            ISO_CHECK_EQ_INT(
                intel8008_instruction_size(two[i], &sz, err, sizeof err),
                INTEL8008_OK);
            ISO_CHECK_EQ_UINT(sz, 2u);
        }
        for (i = 0; i < sizeof three / sizeof three[0]; i++) {
            ISO_CHECK_EQ_INT(
                intel8008_instruction_size(three[i], &sz, err, sizeof err),
                INTEL8008_OK);
            ISO_CHECK_EQ_UINT(sz, 3u);
        }
        ISO_CHECK_EQ_INT(intel8008_instruction_size("ORG", &sz, err, sizeof err),
                         INTEL8008_OK);
        ISO_CHECK_EQ_UINT(sz, 0u);
        ISO_CHECK_EQ_INT(
            intel8008_instruction_size("BOGUS", &sz, err, sizeof err),
            INTEL8008_ERR);
    }

    /* ── encoder vectors ───────────────────────────────────────────────────*/
    {
        static const uint8_t hlt[] = {0xFF};
        static const uint8_t rfc[] = {0x03};
        static const uint8_t mvi_b_42[] = {0x06, 0x2A};
        static const uint8_t mvi_h_20[] = {0x26, 0x20};
        static const uint8_t mov_a_b[] = {0x78};
        static const uint8_t add_c[] = {0x81};
        static const uint8_t jmp_a[] = {0x7C, 0x0A, 0x00};
        static const uint8_t cal_100[] = {0x7E, 0x00, 0x01};
        static const uint8_t adi_5[] = {0xC4, 0x05};
        static const uint8_t in_2[] = {0x51};
        static const uint8_t out_17[] = {0x22};
        static const uint8_t inr_d[] = {0x10};
        static const uint8_t dcr_c[] = {0x09};
        static const uint8_t rst_3[] = {0x1D};
        const char *mvi_b[] = {"B", "42"};
        const char *mvi_h[] = {"H", "0x20"};
        const char *mov_ab[] = {"A", "B"};
        const char *one_c[] = {"C"};
        const char *jmp_op[] = {"0x000A"};
        const char *cal_op[] = {"0x0100"};
        const char *adi_op[] = {"5"};
        const char *in_op[] = {"2"};
        const char *out_op[] = {"17"};
        const char *inr_op[] = {"D"};
        const char *rst_op[] = {"3"};

        ISO_CHECK(enc_eq("HLT", NULL, 0, NULL, 0, hlt, 1));
        ISO_CHECK(enc_eq("RFC", NULL, 0, NULL, 0, rfc, 1));
        ISO_CHECK(enc_eq("RET", NULL, 0, NULL, 0, rfc, 1));
        ISO_CHECK(enc_eq("MVI", mvi_b, 2, NULL, 0, mvi_b_42, 2));
        ISO_CHECK(enc_eq("MVI", mvi_h, 2, NULL, 0, mvi_h_20, 2));
        ISO_CHECK(enc_eq("MOV", mov_ab, 2, NULL, 0, mov_a_b, 1));
        ISO_CHECK(enc_eq("ADD", one_c, 1, NULL, 0, add_c, 1));
        ISO_CHECK(enc_eq("JMP", jmp_op, 1, NULL, 0, jmp_a, 3));
        ISO_CHECK(enc_eq("CAL", cal_op, 1, NULL, 0, cal_100, 3));
        ISO_CHECK(enc_eq("ADI", adi_op, 1, NULL, 0, adi_5, 2));
        ISO_CHECK(enc_eq("IN", in_op, 1, NULL, 0, in_2, 1));
        ISO_CHECK(enc_eq("OUT", out_op, 1, NULL, 0, out_17, 1));
        ISO_CHECK(enc_eq("INR", inr_op, 1, NULL, 0, inr_d, 1));
        ISO_CHECK(enc_eq("DCR", one_c, 1, NULL, 0, dcr_c, 1));
        ISO_CHECK(enc_eq("RST", rst_op, 1, NULL, 0, rst_3, 1));
    }

    /* ── label / hi / lo resolution ────────────────────────────────────────*/
    {
        Intel8008Symbols *s = intel8008_symbols_new();
        static const uint8_t jtz[] = {0x4C, 0x10, 0x00};
        static const uint8_t mvi_hi[] = {0x26, 0x20};
        static const uint8_t mvi_lo[] = {0x2E, 0x00};
        const char *jtz_op[] = {"loop_end"};
        const char *hi_op[] = {"H", "hi(counter)"};
        const char *lo_op[] = {"L", "lo(counter)"};
        intel8008_symbols_set(s, "loop_end", 0x0010);
        ISO_CHECK(enc_eq("JTZ", jtz_op, 1, s, 0, jtz, 3));
        intel8008_symbols_set(s, "counter", 0x2000);
        ISO_CHECK(enc_eq("MVI", hi_op, 2, s, 0, mvi_hi, 2));
        ISO_CHECK(enc_eq("MVI", lo_op, 2, s, 0, mvi_lo, 2));
        intel8008_symbols_free(s);
    }

    /* ── full two-pass assembly ────────────────────────────────────────────*/
    {
        static const uint8_t halt[] = {0xFF};
        static const uint8_t mvi_halt[] = {0x06, 0x00, 0xFF};
        static const uint8_t fwd[] = {0x7C, 0x03, 0x00, 0xFF};
        static const uint8_t jmp_dollar[] = {0x7C, 0x00, 0x00};
        static const uint8_t pad[] = {0x06, 0x01, 0xFF, 0xFF, 0xFF, 0xFF};
        ISO_CHECK(asm_eq("    ORG 0x0000\n_start:\n    HLT\n", halt, 1));
        ISO_CHECK(asm_eq("    ORG 0x0000\n_start:\n    MVI  B, 0\n    HLT\n",
                         mvi_halt, 3));
        ISO_CHECK(asm_eq("\n    ORG 0x0000\n_start:\n    JMP loop_end\n"
                         "loop_end:\n    HLT\n",
                         fwd, 4));
        ISO_CHECK(asm_eq("    ORG 0x0000\n    JMP $\n", jmp_dollar, 3));
        ISO_CHECK(asm_eq("\n    ORG 0x0000\n    MVI  B, 1\n    ORG 0x0005\n"
                         "    HLT\n",
                         pad, 6));
    }

    /* ── error paths ───────────────────────────────────────────────────────*/
    {
        uint8_t *out = NULL;
        size_t len = 0;
        ISO_CHECK_EQ_INT(
            intel8008_assemble("    BOGUS\n", &out, &len, err, sizeof err),
            INTEL8008_ERR);
        ISO_CHECK(strstr(err, "BOGUS") != NULL);
        ISO_CHECK(out == NULL);

        ISO_CHECK_EQ_INT(intel8008_assemble("    JMP undefined_label\n", &out,
                                            &len, err, sizeof err),
                         INTEL8008_ERR);

        ISO_CHECK_EQ_INT(
            intel8008_assemble("    MVI B, 256\n", &out, &len, err, sizeof err),
            INTEL8008_ERR);
        ISO_CHECK(strstr(err, "256") != NULL || strstr(err, "range") != NULL);
    }

    return ISO_TEST_RESULT();
}

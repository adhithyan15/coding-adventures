/*
 * Tests for aarch64-encoder, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's unit tests (known-good ARM64 encodings).
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>

#include "aarch64_encoder.h"

/* Finish `a` and return word at byte offset `off` (0 on unexpected length). */
static uint32_t word_at(A64Assembler *a, size_t off, size_t expect_len) {
    uint8_t *b = NULL;
    size_t len = 0;
    uint32_t w = 0;
    if (a64_finish(a, &b, &len) != A64_OK) {
        a64_free(a);
        return 0xDEADBEEFu;
    }
    if (expect_len != 0 && len != expect_len) {
        free(b);
        a64_free(a);
        return 0xBADBAD00u;
    }
    if (off + 4 <= len) {
        w = (uint32_t)b[off] | ((uint32_t)b[off + 1] << 8) |
            ((uint32_t)b[off + 2] << 16) | ((uint32_t)b[off + 3] << 24);
    }
    free(b);
    a64_free(a);
    return w;
}

/* Single-instruction word (asserts the stream is exactly one word). */
static uint32_t one_word(A64Assembler *a) { return word_at(a, 0, 4); }

int main(void) {
    /* ── moves ─────────────────────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_movz(a, A64_X0, 0, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xD2800000u); }
    { A64Assembler *a = a64_new(); a64_movz(a, A64_X1, 5, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xD28000A1u); }
    { A64Assembler *a = a64_new(); a64_movz(a, A64_X0, 0x1234, 1); ISO_CHECK_EQ_UINT(one_word(a), 0xD2A24680u); }
    { A64Assembler *a = a64_new(); a64_mov_imm64(a, A64_X0, 5); ISO_CHECK_EQ_UINT(one_word(a), 0xD28000A0u); }
    {
        A64Assembler *a = a64_new();
        uint8_t *b = NULL; size_t len = 0;
        a64_mov_imm64(a, A64_X0, 0x12345678u);
        ISO_CHECK_EQ_INT(a64_finish(a, &b, &len), A64_OK);
        ISO_CHECK_EQ_UINT(len, 8u);
        free(b); a64_free(a);
    }

    /* ── arithmetic (register) ─────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_add(a, A64_X0, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x8B010000u); }
    { A64Assembler *a = a64_new(); a64_sub(a, A64_X2, A64_X3, A64_X4); ISO_CHECK_EQ_UINT(one_word(a), 0xCB040062u); }
    { A64Assembler *a = a64_new(); a64_mul(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9B027C20u); }

    /* ── arithmetic (immediate) ────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_add_imm(a, A64_X0, A64_X0, 1); ISO_CHECK_EQ_UINT(one_word(a), 0x91000400u); }
    {
        A64Assembler *a = a64_new();
        a64_add_imm(a, A64_X0, A64_X0, 1u << 12);
        ISO_CHECK_EQ_INT(a64_error(a), A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        a64_free(a);
    }

    /* ── compare ───────────────────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_cmp_imm(a, A64_X0, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xF100001Fu); }
    { A64Assembler *a = a64_new(); a64_cmp(a, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0xEB01001Fu); }

    /* ── memory ────────────────────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_ldr(a, A64_X0, A64_SP, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xF94003E0u); }
    { A64Assembler *a = a64_new(); a64_str(a, A64_X0, A64_SP, 8); ISO_CHECK_EQ_UINT(one_word(a), 0xF90007E0u); }
    {
        A64Assembler *a = a64_new();
        a64_ldr(a, A64_X0, A64_SP, 7);
        ISO_CHECK_EQ_INT(a64_error(a), A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        a64_free(a);
    }

    /* ── scalar FP ─────────────────────────────────────────────────────────*/
    {
        A64Assembler *a = a64_new();
        a64_ldr_d(a, A64_X0, A64_SP, 8);
        a64_str_d(a, A64_X0, A64_SP, 8);
        uint8_t *b = NULL; size_t len = 0;
        ISO_CHECK_EQ_INT(a64_finish(a, &b, &len), A64_OK);
        ISO_CHECK_EQ_UINT((uint32_t)b[0] | ((uint32_t)b[1] << 8) | ((uint32_t)b[2] << 16) | ((uint32_t)b[3] << 24), 0xFD4007E0u);
        ISO_CHECK_EQ_UINT((uint32_t)b[4] | ((uint32_t)b[5] << 8) | ((uint32_t)b[6] << 16) | ((uint32_t)b[7] << 24), 0xFD0007E0u);
        free(b); a64_free(a);
    }
    { A64Assembler *a = a64_new(); a64_fadd(a, A64_X0, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E612800u); }
    { A64Assembler *a = a64_new(); a64_fsub(a, A64_X0, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E613800u); }
    { A64Assembler *a = a64_new(); a64_fmul(a, A64_X0, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E610800u); }
    { A64Assembler *a = a64_new(); a64_fdiv(a, A64_X0, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E611800u); }
    { A64Assembler *a = a64_new(); a64_fcmp(a, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E612000u); }
    { A64Assembler *a = a64_new(); a64_fsqrt(a, A64_X0, A64_X0); ISO_CHECK_EQ_UINT(one_word(a), 0x1E61C000u); }
    { A64Assembler *a = a64_new(); a64_scvtf(a, A64_X0, A64_X3); ISO_CHECK_EQ_UINT(one_word(a), 0x9E620060u); }
    { A64Assembler *a = a64_new(); a64_fcvtzs(a, A64_X2, A64_X0); ISO_CHECK_EQ_UINT(one_word(a), 0x9E780002u); }
    { A64Assembler *a = a64_new(); a64_frintm(a, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E654020u); }

    /* ── STP / LDP ─────────────────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_stp_pre(a, A64_FP, A64_LR, A64_SP, -16); ISO_CHECK_EQ_UINT(one_word(a), 0xA9BF7BFDu); }
    { A64Assembler *a = a64_new(); a64_ldp_post(a, A64_FP, A64_LR, A64_SP, 16); ISO_CHECK_EQ_UINT(one_word(a), 0xA8C17BFDu); }

    /* ── branches ──────────────────────────────────────────────────────────*/
    {
        A64Assembler *a = a64_new();
        A64Label l2 = a64_create_label(a);
        a64_movz(a, A64_X0, 1, 0);
        a64_b(a, l2);
        a64_movz(a, A64_X0, 2, 0);
        a64_bind(a, l2);
        a64_ret(a);
        ISO_CHECK_EQ_UINT(word_at(a, 4, 16), 0x14000002u);
    }
    {
        A64Assembler *a = a64_new();
        A64Label lp = a64_create_label(a);
        a64_bind(a, lp);
        a64_nop(a);
        a64_b(a, lp);
        ISO_CHECK_EQ_UINT(word_at(a, 4, 8), 0x14000000u | 0x03FFFFFFu);
    }
    {
        A64Assembler *a = a64_new();
        A64Label l = a64_create_label(a);
        a64_cmp_imm(a, A64_X0, 0);
        a64_b_cond(a, A64_EQ, l);
        a64_nop(a);
        a64_bind(a, l);
        a64_ret(a);
        ISO_CHECK_EQ_UINT(word_at(a, 4, 16), 0x54000000u | (2u << 5) | (uint32_t)A64_EQ);
    }
    {
        A64Assembler *a = a64_new();
        uint8_t *b = NULL; size_t len = 0;
        A64Label l = a64_create_label(a);
        a64_b(a, l);
        ISO_CHECK_EQ_INT(a64_finish(a, &b, &len), A64_ERR_UNBOUND_LABEL);
        a64_free(a);
    }
    {
        A64Assembler *a = a64_new();
        A64Label l = a64_create_label(a);
        a64_bind(a, l);
        a64_bind(a, l);
        ISO_CHECK_EQ_INT(a64_error(a), A64_ERR_LABEL_ALREADY_BOUND);
        a64_free(a);
    }
    {
        A64Assembler *a = a64_new();
        A64Label l = a64_create_label(a);
        a64_bl(a, l);
        a64_bind(a, l);
        a64_ret(a);
        ISO_CHECK_EQ_UINT(word_at(a, 0, 8), 0x94000001u);
    }

    /* ── indirect / return / misc ──────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_blr(a, A64_X0); ISO_CHECK_EQ_UINT(one_word(a), 0xD63F0000u); }
    { A64Assembler *a = a64_new(); a64_ret(a); ISO_CHECK_EQ_UINT(one_word(a), 0xD65F03C0u); }
    { A64Assembler *a = a64_new(); a64_nop(a); ISO_CHECK_EQ_UINT(one_word(a), 0xD503201Fu); }
    { A64Assembler *a = a64_new(); a64_udf(a, 0); ISO_CHECK_EQ_UINT(one_word(a), 0x00000000u); }
    { A64Assembler *a = a64_new(); a64_svc(a, 0x80); ISO_CHECK_EQ_UINT(one_word(a), 0xD4001001u); }
    { A64Assembler *a = a64_new(); a64_cset(a, A64_X0, A64_EQ); ISO_CHECK_EQ_UINT(one_word(a), 0x9A9F17E0u); }
    {
        A64Assembler *a = a64_new();
        A64Label l = a64_create_label(a);
        a64_cbz(a, A64_X0, l);
        a64_nop(a);
        a64_bind(a, l);
        a64_ret(a);
        ISO_CHECK_EQ_UINT(word_at(a, 0, 12), 0xB4000000u | (2u << 5));
    }
    {
        A64Assembler *a = a64_new();
        A64Label l = a64_create_label(a);
        a64_cbnz(a, A64_X1, l);
        a64_bind(a, l);
        a64_ret(a);
        ISO_CHECK_EQ_UINT(word_at(a, 0, 8), 0xB5000000u | (1u << 5) | 1u);
    }

    /* ── division / logical / shifts / unary ───────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_sdiv(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC20C20u); }
    { A64Assembler *a = a64_new(); a64_udiv(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC20820u); }
    { A64Assembler *a = a64_new(); a64_msub(a, A64_X0, A64_X1, A64_X2, A64_X3); ISO_CHECK_EQ_UINT(one_word(a), 0x9B028C20u); }
    { A64Assembler *a = a64_new(); a64_and(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x8A020020u); }
    { A64Assembler *a = a64_new(); a64_orr(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0xAA020020u); }
    { A64Assembler *a = a64_new(); a64_eor(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0xCA020020u); }
    { A64Assembler *a = a64_new(); a64_mvn(a, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0xAA2103E0u); }
    { A64Assembler *a = a64_new(); a64_lsl_reg(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC22020u); }
    { A64Assembler *a = a64_new(); a64_lsr_reg(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC22420u); }
    { A64Assembler *a = a64_new(); a64_asr_reg(a, A64_X0, A64_X1, A64_X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC22820u); }
    { A64Assembler *a = a64_new(); a64_neg(a, A64_X0, A64_X1); ISO_CHECK_EQ_UINT(one_word(a), 0xCB0103E0u); }

    /* ── adrp placeholder ──────────────────────────────────────────────────*/
    {
        A64Assembler *a = a64_new();
        uint8_t *b = NULL; size_t len = 0;
        size_t i0 = a64_adrp_placeholder(a, A64_X0);
        size_t i1 = a64_adrp_placeholder(a, A64_X1);
        ISO_CHECK_EQ_INT(a64_finish(a, &b, &len), A64_OK);
        ISO_CHECK_EQ_UINT((uint32_t)b[0] | ((uint32_t)b[1] << 8) | ((uint32_t)b[2] << 16) | ((uint32_t)b[3] << 24), 0x90000000u);
        ISO_CHECK_EQ_UINT((uint32_t)b[4] | ((uint32_t)b[5] << 8) | ((uint32_t)b[6] << 16) | ((uint32_t)b[7] << 24), 0x90000001u);
        ISO_CHECK_EQ_UINT(i0, 0u);
        ISO_CHECK_EQ_UINT(i1, 1u);
        free(b); a64_free(a);
    }

    /* ── pre-indexed byte store ────────────────────────────────────────────*/
    { A64Assembler *a = a64_new(); a64_strb_pre_neg1(a, A64_X4, A64_X5); ISO_CHECK_EQ_UINT(one_word(a), 0x381FFCA4u); }
    { A64Assembler *a = a64_new(); a64_strb_pre_neg1(a, A64_X0, A64_X0); ISO_CHECK_EQ_UINT(one_word(a), 0x381FFC00u); }

    /* ── composite prologue/epilogue ───────────────────────────────────────*/
    {
        A64Assembler *a = a64_new();
        uint8_t *b = NULL; size_t len = 0;
        a64_stp_pre(a, A64_FP, A64_LR, A64_SP, -16);
        a64_add_imm(a, A64_FP, A64_SP, 0);
        a64_movz(a, A64_X0, 42, 0);
        a64_ldp_post(a, A64_FP, A64_LR, A64_SP, 16);
        a64_ret(a);
        ISO_CHECK_EQ_INT(a64_finish(a, &b, &len), A64_OK);
        ISO_CHECK_EQ_UINT(len, 20u);
        free(b); a64_free(a);
    }

    return ISO_TEST_RESULT();
}

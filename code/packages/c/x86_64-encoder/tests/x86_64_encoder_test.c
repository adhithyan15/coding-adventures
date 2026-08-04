/*
 * Tests for x86_64-encoder, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's unit tests (byte-exact x86-64 encodings).
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "x86_64_encoder.h"

/* Finish `a`, compare bytes to `exp` (len `n`), free everything. Returns 1 on
 * an exact match. */
static int eq(X64Assembler *a, const uint8_t *exp, size_t n) {
    uint8_t *b = NULL;
    size_t len = 0;
    int ok;
    if (x64_finish(a, &b, &len) != X64_OK) {
        x64_free(a);
        return 0;
    }
    ok = (len == n) && (n == 0 || memcmp(b, exp, n) == 0);
    free(b);
    x64_free(a);
    return ok;
}

#define EXP(...) \
    ((const uint8_t[]){__VA_ARGS__}), sizeof((const uint8_t[]){__VA_ARGS__})

int main(void) {
    /* ── MOV ────────────────────────────────────────────────────────────────*/
    { X64Assembler *a = x64_new(); x64_mov_r64_r64(a, X64_RAX, X64_RDI); ISO_CHECK(eq(a, EXP(0x48, 0x89, 0xF8))); }
    { X64Assembler *a = x64_new(); x64_mov_r64_r64(a, X64_R15, X64_R8); ISO_CHECK(eq(a, EXP(0x4D, 0x89, 0xC7))); }
    { X64Assembler *a = x64_new(); x64_mov_r64_imm32(a, X64_RAX, 42); ISO_CHECK(eq(a, EXP(0x48, 0xC7, 0xC0, 0x2A, 0x00, 0x00, 0x00))); }
    { X64Assembler *a = x64_new(); x64_mov_r64_imm32(a, X64_RAX, -1); ISO_CHECK(eq(a, EXP(0x48, 0xC7, 0xC0, 0xFF, 0xFF, 0xFF, 0xFF))); }
    { X64Assembler *a = x64_new(); x64_mov_r64_imm64(a, X64_RAX, 0x1234567890ABCDEFull); ISO_CHECK(eq(a, EXP(0x48, 0xB8, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12))); }
    { X64Assembler *a = x64_new(); x64_mov_r64_imm64(a, X64_R10, 0xDEADBEEFCAFEBABEull); ISO_CHECK(eq(a, EXP(0x49, 0xBA, 0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE))); }

    /* ── memory ─────────────────────────────────────────────────────────────*/
    { X64Assembler *a = x64_new(); x64_mov_r64_mem(a, X64_RAX, X64_RBP, -8); ISO_CHECK(eq(a, EXP(0x48, 0x8B, 0x85, 0xF8, 0xFF, 0xFF, 0xFF))); }
    { X64Assembler *a = x64_new(); x64_mov_mem_r64(a, X64_RBP, -8, X64_RDI); ISO_CHECK(eq(a, EXP(0x48, 0x89, 0xBD, 0xF8, 0xFF, 0xFF, 0xFF))); }
    { X64Assembler *a = x64_new(); x64_mov_r64_mem(a, X64_RAX, X64_RSP, -8); ISO_CHECK(eq(a, EXP(0x48, 0x8B, 0x84, 0x24, 0xF8, 0xFF, 0xFF, 0xFF))); }

    /* ── arithmetic ─────────────────────────────────────────────────────────*/
    { X64Assembler *a = x64_new(); x64_add(a, X64_RAX, X64_RCX); ISO_CHECK(eq(a, EXP(0x48, 0x01, 0xC8))); }
    { X64Assembler *a = x64_new(); x64_sub(a, X64_RAX, X64_RCX); ISO_CHECK(eq(a, EXP(0x48, 0x29, 0xC8))); }
    { X64Assembler *a = x64_new(); x64_imul(a, X64_RAX, X64_RCX); ISO_CHECK(eq(a, EXP(0x48, 0x0F, 0xAF, 0xC1))); }
    { X64Assembler *a = x64_new(); x64_add_imm32(a, X64_RAX, 1000); ISO_CHECK(eq(a, EXP(0x48, 0x81, 0xC0, 0xE8, 0x03, 0x00, 0x00))); }
    { X64Assembler *a = x64_new(); x64_neg(a, X64_RAX); ISO_CHECK(eq(a, EXP(0x48, 0xF7, 0xD8))); }
    { X64Assembler *a = x64_new(); x64_idiv(a, X64_RCX); ISO_CHECK(eq(a, EXP(0x48, 0xF7, 0xF9))); }
    { X64Assembler *a = x64_new(); x64_div(a, X64_RCX); ISO_CHECK(eq(a, EXP(0x48, 0xF7, 0xF1))); }
    { X64Assembler *a = x64_new(); x64_cqo(a); ISO_CHECK(eq(a, EXP(0x48, 0x99))); }

    /* ── logical ────────────────────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        x64_and(a, X64_RAX, X64_RCX);
        x64_or(a, X64_RAX, X64_RCX);
        x64_xor(a, X64_RAX, X64_RCX);
        x64_test(a, X64_RAX, X64_RCX);
        x64_not(a, X64_RAX);
        ISO_CHECK(eq(a, EXP(0x48, 0x21, 0xC8, 0x48, 0x09, 0xC8, 0x48, 0x31,
                            0xC8, 0x48, 0x85, 0xC8, 0x48, 0xF7, 0xD0)));
    }

    /* ── shifts ─────────────────────────────────────────────────────────────*/
    { X64Assembler *a = x64_new(); x64_shl_cl(a, X64_RAX); ISO_CHECK(eq(a, EXP(0x48, 0xD3, 0xE0))); }
    { X64Assembler *a = x64_new(); x64_shr_cl(a, X64_RAX); ISO_CHECK(eq(a, EXP(0x48, 0xD3, 0xE8))); }
    { X64Assembler *a = x64_new(); x64_sar_cl(a, X64_RAX); ISO_CHECK(eq(a, EXP(0x48, 0xD3, 0xF8))); }
    { X64Assembler *a = x64_new(); x64_shl_imm8(a, X64_RAX, 3); ISO_CHECK(eq(a, EXP(0x48, 0xC1, 0xE0, 0x03))); }

    /* ── compare + set ──────────────────────────────────────────────────────*/
    { X64Assembler *a = x64_new(); x64_cmp(a, X64_RAX, X64_RCX); ISO_CHECK(eq(a, EXP(0x48, 0x39, 0xC8))); }
    { X64Assembler *a = x64_new(); x64_setcc(a, X64_E, X64_RAX); ISO_CHECK(eq(a, EXP(0x40, 0x0F, 0x94, 0xC0))); }
    { X64Assembler *a = x64_new(); x64_movzx_r64_r8(a, X64_RAX, X64_RAX); ISO_CHECK(eq(a, EXP(0x48, 0x0F, 0xB6, 0xC0))); }

    /* ── SSE2 ───────────────────────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        x64_movsd_load(a, X64_RAX, X64_RBP, 8);
        x64_movsd_store(a, X64_RBP, 8, X64_RAX);
        ISO_CHECK(eq(a, EXP(0xF2, 0x0F, 0x10, 0x85, 0x08, 0x00, 0x00, 0x00,
                            0xF2, 0x0F, 0x11, 0x85, 0x08, 0x00, 0x00, 0x00)));
    }
    {
        X64Assembler *a = x64_new();
        x64_addsd(a, X64_RAX, X64_RCX);
        x64_subsd(a, X64_RAX, X64_RCX);
        x64_mulsd(a, X64_RAX, X64_RCX);
        x64_divsd(a, X64_RAX, X64_RCX);
        ISO_CHECK(eq(a, EXP(0xF2, 0x0F, 0x58, 0xC1, 0xF2, 0x0F, 0x5C, 0xC1,
                            0xF2, 0x0F, 0x59, 0xC1, 0xF2, 0x0F, 0x5E, 0xC1)));
    }
    { X64Assembler *a = x64_new(); x64_ucomisd(a, X64_RAX, X64_RCX); ISO_CHECK(eq(a, EXP(0x66, 0x0F, 0x2E, 0xC1))); }
    { X64Assembler *a = x64_new(); x64_sqrtsd(a, X64_RAX, X64_RAX); ISO_CHECK(eq(a, EXP(0xF2, 0x0F, 0x51, 0xC0))); }

    /* ── int ⇄ real conversions ─────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        x64_cvtsi2sd(a, X64_RAX, X64_RAX);
        x64_cvttsd2si(a, X64_RAX, X64_RAX);
        x64_roundsd(a, X64_RAX, X64_RAX, 1);
        ISO_CHECK(eq(a, EXP(0xF2, 0x48, 0x0F, 0x2A, 0xC0, 0xF2, 0x48, 0x0F,
                            0x2C, 0xC0, 0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x01)));
    }
    {
        X64Assembler *a = x64_new();
        x64_cvtsi2sd(a, X64_RCX, X64_R8);
        x64_cvttsd2si(a, X64_R8, X64_RCX);
        ISO_CHECK(eq(a, EXP(0xF2, 0x49, 0x0F, 0x2A, 0xC8, 0xF2, 0x4C, 0x0F,
                            0x2C, 0xC1)));
    }

    /* ── stack ──────────────────────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        x64_push(a, X64_RBP);
        x64_pop(a, X64_RBP);
        ISO_CHECK(eq(a, EXP(0x55, 0x5D)));
    }
    { X64Assembler *a = x64_new(); x64_push(a, X64_R15); ISO_CHECK(eq(a, EXP(0x41, 0x57))); }

    /* ── control flow ───────────────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        X64Label fwd = x64_create_label(a);
        x64_jmp(a, fwd);
        x64_nop(a);
        x64_nop(a);
        x64_bind(a, fwd);
        x64_ret(a);
        ISO_CHECK(eq(a, EXP(0xE9, 0x02, 0x00, 0x00, 0x00, 0x90, 0x90, 0xC3)));
    }
    {
        X64Assembler *a = x64_new();
        X64Label top = x64_create_label(a);
        uint8_t *b = NULL;
        size_t len = 0;
        int32_t disp;
        x64_bind(a, top);
        x64_nop(a);
        x64_jcc(a, X64_NE, top);
        ISO_CHECK_EQ_INT(x64_finish(a, &b, &len), X64_OK);
        ISO_CHECK(b[0] == 0x90 && b[1] == 0x0F && b[2] == 0x85);
        disp = (int32_t)((uint32_t)b[3] | ((uint32_t)b[4] << 8) |
                         ((uint32_t)b[5] << 16) | ((uint32_t)b[6] << 24));
        ISO_CHECK_EQ_INT(disp, -7);
        free(b);
        x64_free(a);
    }
    {
        X64Assembler *a = x64_new();
        uint8_t *b = NULL;
        size_t len = 0;
        X64Label l = x64_create_label(a);
        x64_jmp(a, l);
        ISO_CHECK_EQ_INT(x64_finish(a, &b, &len), X64_ERR_UNBOUND_LABEL);
        x64_free(a);
    }
    {
        X64Assembler *a = x64_new();
        X64Label l = x64_create_label(a);
        x64_bind(a, l);
        x64_bind(a, l);
        ISO_CHECK_EQ_INT(x64_error(a), X64_ERR_LABEL_ALREADY_BOUND);
        x64_free(a);
    }

    /* ── calls / relocs ─────────────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        uint8_t *b = NULL;
        size_t len = 0, po = 0;
        const char *sym = NULL;
        X64RelocKind kind;
        int32_t addend = 0;
        x64_call_rel32(a, "__twig_print_i64", X64_RELOC_PLT_REL32);
        ISO_CHECK_EQ_UINT(x64_external_reloc_count(a), 1u);
        ISO_CHECK(x64_external_reloc(a, 0, &po, &sym, &kind, &addend));
        ISO_CHECK_STR_EQ(sym, "__twig_print_i64");
        ISO_CHECK_EQ_INT(kind, X64_RELOC_PLT_REL32);
        ISO_CHECK_EQ_INT(addend, -4);
        ISO_CHECK_EQ_UINT(po, 1u);
        ISO_CHECK_EQ_INT(x64_finish(a, &b, &len), X64_OK);
        ISO_CHECK(len == 5 && b[0] == 0xE8 && b[1] == 0 && b[4] == 0);
        free(b);
        x64_free(a);
    }
    {
        X64Assembler *a = x64_new();
        X64Label top = x64_create_label(a);
        uint8_t *b = NULL;
        size_t len = 0;
        int32_t disp;
        x64_bind(a, top);
        x64_nop(a);
        x64_call_label(a, top);
        ISO_CHECK_EQ_INT(x64_finish(a, &b, &len), X64_OK);
        ISO_CHECK(b[0] == 0x90 && b[1] == 0xE8);
        disp = (int32_t)((uint32_t)b[2] | ((uint32_t)b[3] << 8) |
                         ((uint32_t)b[4] << 16) | ((uint32_t)b[5] << 24));
        ISO_CHECK_EQ_INT(disp, -6);
        free(b);
        x64_free(a);
    }
    { X64Assembler *a = x64_new(); x64_call_r64(a, X64_RAX); ISO_CHECK(eq(a, EXP(0x40, 0xFF, 0xD0))); }
    { X64Assembler *a = x64_new(); x64_ret(a); ISO_CHECK(eq(a, EXP(0xC3))); }
    { X64Assembler *a = x64_new(); x64_ud2(a); ISO_CHECK(eq(a, EXP(0x0F, 0x0B))); }
    {
        X64Assembler *a = x64_new();
        uint8_t *b = NULL;
        size_t len = 0, po = 0;
        const char *sym = NULL;
        x64_lea_rip_rel(a, X64_RAX, "_twig_globals", X64_RELOC_PC_REL32);
        ISO_CHECK(x64_external_reloc(a, 0, &po, &sym, NULL, NULL));
        ISO_CHECK_STR_EQ(sym, "_twig_globals");
        ISO_CHECK_EQ_UINT(po, 3u);
        ISO_CHECK_EQ_INT(x64_finish(a, &b, &len), X64_OK);
        ISO_CHECK(len == 7 && b[0] == 0x48 && b[1] == 0x8D && b[2] == 0x05);
        free(b);
        x64_free(a);
    }

    /* ── worked examples ────────────────────────────────────────────────────*/
    {
        X64Assembler *a = x64_new();
        x64_mov_r64_r64(a, X64_RAX, X64_RDI);
        x64_add(a, X64_RAX, X64_RSI);
        x64_ret(a);
        ISO_CHECK(eq(a, EXP(0x48, 0x89, 0xF8, 0x48, 0x01, 0xF0, 0xC3)));
    }
    {
        X64Assembler *a = x64_new();
        x64_mov_r64_r64(a, X64_RAX, X64_RCX);
        x64_add(a, X64_RAX, X64_RDX);
        x64_ret(a);
        ISO_CHECK(eq(a, EXP(0x48, 0x89, 0xC8, 0x48, 0x01, 0xD0, 0xC3)));
    }

    return ISO_TEST_RESULT();
}

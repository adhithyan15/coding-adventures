/*
 * aarch64_encoder.c — AArch64 (ARM64) instruction encoder, pure ISO C17.
 * =====================================================================
 *
 * See aarch64_encoder.h. Instruction words accumulate in a growable u32 vector;
 * branches record a fix-up that `a64_finish` patches once every label's word
 * index is known. Each instruction word is a base opcode OR'd with register /
 * immediate bit-fields (DDI 0487 layout).
 */
#include "aarch64_encoder.h"

#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* (none needed beyond size_t) */

typedef struct {
    int bound;
    size_t idx;
} LabelSlot;
typedef struct {
    size_t word_idx;
    A64Label target;
    int imm19; /* 0 = Imm26, 1 = Imm19 */
} Fixup;

struct A64Assembler {
    uint32_t *code;
    size_t code_len, code_cap;
    LabelSlot *labels;
    size_t labels_len, labels_cap;
    Fixup *fixups;
    size_t fixups_len, fixups_cap;
    A64Status err; /* sticky: first error wins */
};

static int grow(void **data, size_t *cap, size_t need, size_t elem) {
    size_t nc;
    void *nd;
    if (need <= *cap) {
        return 1;
    }
    nc = *cap ? *cap : 8;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    if (nc > (size_t)-1 / elem) {
        return 0;
    }
    nd = realloc(*data, nc * elem);
    if (!nd) {
        return 0;
    }
    *data = nd;
    *cap = nc;
    return 1;
}

static void latch(A64Assembler *a, A64Status st) {
    if (a->err == A64_OK) {
        a->err = st;
    }
}

static uint32_t ridx(A64Reg r) { return (uint32_t)r; }

A64Assembler *a64_new(void) {
    return (A64Assembler *)calloc(1, sizeof(A64Assembler));
}
void a64_free(A64Assembler *a) {
    if (!a) {
        return;
    }
    free(a->code);
    free(a->labels);
    free(a->fixups);
    free(a);
}
A64Status a64_error(const A64Assembler *a) { return a ? a->err : A64_ERR_OUT_OF_MEMORY; }
size_t a64_len_words(const A64Assembler *a) { return a ? a->code_len : 0; }

/* ── Emit primitives ────────────────────────────────────────────────────────*/
static void emit(A64Assembler *a, uint32_t word) {
    if (a->err == A64_ERR_OUT_OF_MEMORY) {
        return;
    }
    if (!grow((void **)&a->code, &a->code_cap, a->code_len + 1,
              sizeof(uint32_t))) {
        latch(a, A64_ERR_OUT_OF_MEMORY);
        return;
    }
    a->code[a->code_len++] = word;
}
static void emit_r(A64Assembler *a, uint32_t base, A64Reg rd, A64Reg rn,
                   A64Reg rm) {
    emit(a, base | (ridx(rm) << 16) | (ridx(rn) << 5) | ridx(rd));
}
static void emit_branch(A64Assembler *a, A64Label target, int imm19,
                        uint32_t base) {
    size_t word_idx = a->code_len;
    emit(a, base);
    if (a->err == A64_ERR_OUT_OF_MEMORY) {
        return;
    }
    if (!grow((void **)&a->fixups, &a->fixups_cap, a->fixups_len + 1,
              sizeof(Fixup))) {
        latch(a, A64_ERR_OUT_OF_MEMORY);
        return;
    }
    a->fixups[a->fixups_len].word_idx = word_idx;
    a->fixups[a->fixups_len].target = target;
    a->fixups[a->fixups_len].imm19 = imm19;
    a->fixups_len++;
}

/* ── Labels ─────────────────────────────────────────────────────────────────*/
A64Label a64_create_label(A64Assembler *a) {
    A64Label id = (A64Label)a->labels_len;
    if (!grow((void **)&a->labels, &a->labels_cap, a->labels_len + 1,
              sizeof(LabelSlot))) {
        latch(a, A64_ERR_OUT_OF_MEMORY);
        return id;
    }
    a->labels[a->labels_len].bound = 0;
    a->labels[a->labels_len].idx = 0;
    a->labels_len++;
    return id;
}
void a64_bind(A64Assembler *a, A64Label label) {
    if ((size_t)label >= a->labels_len) {
        return;
    }
    if (a->labels[label].bound) {
        latch(a, A64_ERR_LABEL_ALREADY_BOUND);
        return;
    }
    a->labels[label].bound = 1;
    a->labels[label].idx = a->code_len;
}

/* ── Move-immediate ─────────────────────────────────────────────────────────*/
void a64_movz(A64Assembler *a, A64Reg rd, uint16_t imm16, uint8_t hw) {
    emit(a, 0xD2800000u | ((uint32_t)(hw & 3) << 21) |
                ((uint32_t)imm16 << 5) | ridx(rd));
}
void a64_movk(A64Assembler *a, A64Reg rd, uint16_t imm16, uint8_t hw) {
    emit(a, 0xF2800000u | ((uint32_t)(hw & 3) << 21) |
                ((uint32_t)imm16 << 5) | ridx(rd));
}
void a64_mov_imm64(A64Assembler *a, A64Reg rd, uint64_t imm) {
    int i, emitted = 0;
    if (imm == 0) {
        a64_movz(a, rd, 0, 0);
        return;
    }
    for (i = 0; i < 4; i++) {
        uint16_t c = (uint16_t)((imm >> (16 * i)) & 0xFFFF);
        if (c == 0) {
            continue;
        }
        if (!emitted) {
            a64_movz(a, rd, c, (uint8_t)i);
            emitted = 1;
        } else {
            a64_movk(a, rd, c, (uint8_t)i);
        }
    }
}

/* ── Arithmetic / division / logical / shifts / unary (register) ────────────*/
void a64_add(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x8B000000u, rd, rn, rm);
}
void a64_sub(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0xCB000000u, rd, rn, rm);
}
void a64_mul(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit(a, 0x9B000000u | (ridx(rm) << 16) | (0x1Fu << 10) | (ridx(rn) << 5) |
                ridx(rd));
}
void a64_sdiv(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x9AC00C00u, rd, rn, rm);
}
void a64_udiv(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x9AC00800u, rd, rn, rm);
}
void a64_msub(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm, A64Reg ra) {
    emit(a, 0x9B008000u | (ridx(rm) << 16) | (ridx(ra) << 10) | (ridx(rn) << 5) |
                ridx(rd));
}
void a64_and(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x8A000000u, rd, rn, rm);
}
void a64_orr(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0xAA000000u, rd, rn, rm);
}
void a64_eor(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0xCA000000u, rd, rn, rm);
}
void a64_mvn(A64Assembler *a, A64Reg rd, A64Reg rm) {
    emit(a, 0xAA200000u | (ridx(rm) << 16) | (0x1Fu << 5) | ridx(rd));
}
void a64_lsl_reg(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x9AC02000u, rd, rn, rm);
}
void a64_lsr_reg(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x9AC02400u, rd, rn, rm);
}
void a64_asr_reg(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm) {
    emit_r(a, 0x9AC02800u, rd, rn, rm);
}
void a64_neg(A64Assembler *a, A64Reg rd, A64Reg rm) {
    emit(a, 0xCB000000u | (ridx(rm) << 16) | (0x1Fu << 5) | ridx(rd));
}

size_t a64_adrp_placeholder(A64Assembler *a, A64Reg rd) {
    size_t word_idx = a->code_len;
    emit(a, 0x90000000u | ridx(rd));
    return word_idx;
}

/* ── Arithmetic (immediate) / compare ───────────────────────────────────────*/
void a64_add_imm(A64Assembler *a, A64Reg rd, A64Reg rn, uint32_t imm12) {
    if (imm12 >= (1u << 12)) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    emit(a, 0x91000000u | (imm12 << 10) | (ridx(rn) << 5) | ridx(rd));
}
void a64_sub_imm(A64Assembler *a, A64Reg rd, A64Reg rn, uint32_t imm12) {
    if (imm12 >= (1u << 12)) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    emit(a, 0xD1000000u | (imm12 << 10) | (ridx(rn) << 5) | ridx(rd));
}
void a64_cmp(A64Assembler *a, A64Reg rn, A64Reg rm) {
    emit(a, 0xEB000000u | (ridx(rm) << 16) | (ridx(rn) << 5) | 0x1Fu);
}
void a64_cmp_imm(A64Assembler *a, A64Reg rn, uint32_t imm12) {
    if (imm12 >= (1u << 12)) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    emit(a, 0xF1000000u | (imm12 << 10) | (ridx(rn) << 5) | 0x1Fu);
}

/* ── Memory (scaled unsigned offset) ────────────────────────────────────────*/
static void mem_scaled8(A64Assembler *a, uint32_t base, A64Reg rt, A64Reg rn,
                        uint32_t imm) {
    uint32_t imm12;
    if (imm % 8 != 0 || imm > 0x7FF8) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    imm12 = imm / 8;
    emit(a, base | (imm12 << 10) | (ridx(rn) << 5) | ridx(rt));
}
void a64_ldr(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm) {
    mem_scaled8(a, 0xF9400000u, rt, rn, imm);
}
void a64_str(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm) {
    mem_scaled8(a, 0xF9000000u, rt, rn, imm);
}
void a64_ldr_d(A64Assembler *a, A64Reg dt, A64Reg rn, uint32_t imm) {
    mem_scaled8(a, 0xFD400000u, dt, rn, imm);
}
void a64_str_d(A64Assembler *a, A64Reg dt, A64Reg rn, uint32_t imm) {
    mem_scaled8(a, 0xFD000000u, dt, rn, imm);
}
void a64_ldrb(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm) {
    if (imm > 0xFFF) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    emit(a, 0x39400000u | (imm << 10) | (ridx(rn) << 5) | ridx(rt));
}
void a64_strb(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm) {
    if (imm > 0xFFF) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    emit(a, 0x39000000u | (imm << 10) | (ridx(rn) << 5) | ridx(rt));
}
void a64_strb_pre_neg1(A64Assembler *a, A64Reg wt, A64Reg rn) {
    emit(a, 0x381FFC00u | (ridx(rn) << 5) | ridx(wt));
}

/* ── Scalar FP + conversions ────────────────────────────────────────────────*/
void a64_fadd(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm) {
    emit_r(a, 0x1E602800u, dd, dn, dm);
}
void a64_fsub(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm) {
    emit_r(a, 0x1E603800u, dd, dn, dm);
}
void a64_fmul(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm) {
    emit_r(a, 0x1E600800u, dd, dn, dm);
}
void a64_fdiv(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm) {
    emit_r(a, 0x1E601800u, dd, dn, dm);
}
void a64_fcmp(A64Assembler *a, A64Reg dn, A64Reg dm) {
    emit(a, 0x1E602000u | (ridx(dm) << 16) | (ridx(dn) << 5));
}
void a64_scvtf(A64Assembler *a, A64Reg dd, A64Reg xn) {
    emit(a, 0x9E620000u | (ridx(xn) << 5) | ridx(dd));
}
void a64_fcvtzs(A64Assembler *a, A64Reg xd, A64Reg dn) {
    emit(a, 0x9E780000u | (ridx(dn) << 5) | ridx(xd));
}
void a64_frintm(A64Assembler *a, A64Reg dd, A64Reg dn) {
    emit(a, 0x1E654000u | (ridx(dn) << 5) | ridx(dd));
}
void a64_fsqrt(A64Assembler *a, A64Reg dd, A64Reg dn) {
    emit(a, 0x1E61C000u | (ridx(dn) << 5) | ridx(dd));
}

/* ── STP / LDP ──────────────────────────────────────────────────────────────*/
static void pair_imm7(A64Assembler *a, uint32_t base, A64Reg rt1, A64Reg rt2,
                      A64Reg rn, int32_t imm) {
    uint32_t imm7;
    if (imm % 8 != 0 || imm < -512 || imm > 504) {
        latch(a, A64_ERR_IMMEDIATE_OUT_OF_RANGE);
        return;
    }
    imm7 = (uint32_t)(imm / 8) & 0x7Fu;
    emit(a, base | (imm7 << 15) | (ridx(rt2) << 10) | (ridx(rn) << 5) |
                ridx(rt1));
}
void a64_stp_pre(A64Assembler *a, A64Reg rt1, A64Reg rt2, A64Reg rn,
                 int32_t imm) {
    pair_imm7(a, 0xA9800000u, rt1, rt2, rn, imm);
}
void a64_ldp_post(A64Assembler *a, A64Reg rt1, A64Reg rt2, A64Reg rn,
                  int32_t imm) {
    pair_imm7(a, 0xA8C00000u, rt1, rt2, rn, imm);
}

/* ── Branches / misc ────────────────────────────────────────────────────────*/
void a64_b(A64Assembler *a, A64Label target) {
    emit_branch(a, target, 0, 0x14000000u);
}
void a64_bl(A64Assembler *a, A64Label target) {
    emit_branch(a, target, 0, 0x94000000u);
}
void a64_b_cond(A64Assembler *a, A64Cond cond, A64Label target) {
    emit_branch(a, target, 1, 0x54000000u | (uint32_t)cond);
}
void a64_blr(A64Assembler *a, A64Reg rn) {
    emit(a, 0xD63F0000u | (ridx(rn) << 5));
}
void a64_ret(A64Assembler *a) { emit(a, 0xD65F0000u | (ridx(A64_LR) << 5)); }
void a64_cset(A64Assembler *a, A64Reg rd, A64Cond cond) {
    uint32_t inv = (uint32_t)cond ^ 1u;
    emit(a, 0x9A800400u | (0x1Fu << 16) | (inv << 12) | (0x1Fu << 5) |
                ridx(rd));
}
void a64_cbz(A64Assembler *a, A64Reg rt, A64Label target) {
    emit_branch(a, target, 1, 0xB4000000u | ridx(rt));
}
void a64_cbnz(A64Assembler *a, A64Reg rt, A64Label target) {
    emit_branch(a, target, 1, 0xB5000000u | ridx(rt));
}
void a64_nop(A64Assembler *a) { emit(a, 0xD503201Fu); }
void a64_udf(A64Assembler *a, uint16_t imm) { emit(a, (uint32_t)imm); }
void a64_svc(A64Assembler *a, uint16_t imm) {
    emit(a, 0xD4000001u | ((uint32_t)imm << 5));
}

/* ── Finalisation ───────────────────────────────────────────────────────────*/
A64Status a64_finish(A64Assembler *a, uint8_t **out_bytes, size_t *out_len) {
    size_t i;
    uint8_t *bytes;
    *out_bytes = NULL;
    *out_len = 0;
    if (!a) {
        return A64_ERR_OUT_OF_MEMORY;
    }
    if (a->err != A64_OK) {
        return a->err;
    }
    for (i = 0; i < a->fixups_len; i++) {
        Fixup *f = &a->fixups[i];
        int64_t delta;
        uint32_t *word;
        if ((size_t)f->target >= a->labels_len || !a->labels[f->target].bound) {
            return A64_ERR_UNBOUND_LABEL;
        }
        delta = (int64_t)a->labels[f->target].idx - (int64_t)f->word_idx;
        word = &a->code[f->word_idx];
        if (!f->imm19) { /* Imm26 */
            if (delta < -(1 << 25) || delta >= (1 << 25)) {
                return A64_ERR_BRANCH_OUT_OF_RANGE;
            }
            *word = (*word & ~0x03FFFFFFu) | ((uint32_t)delta & 0x03FFFFFFu);
        } else { /* Imm19 */
            if (delta < -(1 << 18) || delta >= (1 << 18)) {
                return A64_ERR_BRANCH_OUT_OF_RANGE;
            }
            *word = (*word & ~(0x0007FFFFu << 5)) |
                    (((uint32_t)delta & 0x0007FFFFu) << 5);
        }
    }
    if (a->code_len == 0) {
        return A64_OK; /* empty stream, *out_bytes stays NULL */
    }
    bytes = (uint8_t *)malloc(a->code_len * 4);
    if (!bytes) {
        return A64_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < a->code_len; i++) {
        uint32_t w = a->code[i];
        bytes[i * 4 + 0] = (uint8_t)(w & 0xFF);
        bytes[i * 4 + 1] = (uint8_t)((w >> 8) & 0xFF);
        bytes[i * 4 + 2] = (uint8_t)((w >> 16) & 0xFF);
        bytes[i * 4 + 3] = (uint8_t)((w >> 24) & 0xFF);
    }
    *out_bytes = bytes;
    *out_len = a->code_len * 4;
    return A64_OK;
}

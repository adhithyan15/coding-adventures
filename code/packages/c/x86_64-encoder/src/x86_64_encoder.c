/*
 * x86_64_encoder.c — x86-64 (AMD64) instruction encoder, pure ISO C17.
 * ===================================================================
 *
 * See x86_64_encoder.h. Instruction bytes accumulate in a growable buffer;
 * branches record a fix-up that x64_finish patches with the rel32 displacement
 * once every label's byte offset is known. Each instruction is a REX prefix +
 * opcode(s) + ModR/M (+ SIB) + disp/imm.
 */
#include "x86_64_encoder.h"

#include <limits.h> /* INT32_MIN/MAX via stdint */
#include <stdint.h>
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, strlen */

typedef struct {
    int bound;
    size_t off;
} LabelSlot;
typedef struct {
    size_t slot_offset;
    size_t instr_end_offset;
    X64Label target;
} Fixup;
typedef struct {
    size_t patch_offset;
    char *symbol; /* owned */
    X64RelocKind kind;
    int32_t addend;
} Reloc;

struct X64Assembler {
    uint8_t *code;
    size_t code_len, code_cap;
    LabelSlot *labels;
    size_t labels_len, labels_cap;
    Fixup *fixups;
    size_t fixups_len, fixups_cap;
    Reloc *relocs;
    size_t relocs_len, relocs_cap;
    X64Status err;
};

static int grow(void **data, size_t *cap, size_t need, size_t elem) {
    size_t nc;
    void *nd;
    if (need <= *cap) {
        return 1;
    }
    nc = *cap ? *cap : 16;
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

static void latch(X64Assembler *a, X64Status st) {
    if (a->err == X64_OK) {
        a->err = st;
    }
}

/* Register field helpers. */
static uint8_t rlow3(X64Reg r) { return (uint8_t)r & 0x7; }
static int rhigh1(X64Reg r) { return (uint8_t)r >= 8; }

X64Assembler *x64_new(void) {
    return (X64Assembler *)calloc(1, sizeof(X64Assembler));
}
void x64_free(X64Assembler *a) {
    size_t i;
    if (!a) {
        return;
    }
    free(a->code);
    free(a->labels);
    free(a->fixups);
    for (i = 0; i < a->relocs_len; i++) {
        free(a->relocs[i].symbol);
    }
    free(a->relocs);
    free(a);
}
X64Status x64_error(const X64Assembler *a) {
    return a ? a->err : X64_ERR_OUT_OF_MEMORY;
}
size_t x64_len(const X64Assembler *a) { return a ? a->code_len : 0; }

/* ── Byte emission ──────────────────────────────────────────────────────────*/
static void e8(X64Assembler *a, uint8_t b) {
    if (a->err == X64_ERR_OUT_OF_MEMORY) {
        return;
    }
    if (!grow((void **)&a->code, &a->code_cap, a->code_len + 1, 1)) {
        latch(a, X64_ERR_OUT_OF_MEMORY);
        return;
    }
    a->code[a->code_len++] = b;
}
static void e32(X64Assembler *a, uint32_t w) {
    e8(a, (uint8_t)(w & 0xFF));
    e8(a, (uint8_t)((w >> 8) & 0xFF));
    e8(a, (uint8_t)((w >> 16) & 0xFF));
    e8(a, (uint8_t)((w >> 24) & 0xFF));
}
static void e64(X64Assembler *a, uint64_t w) {
    int i;
    for (i = 0; i < 8; i++) {
        e8(a, (uint8_t)((w >> (8 * i)) & 0xFF));
    }
}
static uint8_t rex(int w, int r, int x, int b) {
    return (uint8_t)(0x40 | ((w ? 1 : 0) << 3) | ((r ? 1 : 0) << 2) |
                     ((x ? 1 : 0) << 1) | (b ? 1 : 0));
}
static uint8_t modrm(uint8_t mode, uint8_t reg, uint8_t rm) {
    return (uint8_t)((mode << 6) | (reg << 3) | rm);
}

static void e_rr(X64Assembler *a, uint8_t opcode, X64Reg reg_src,
                 X64Reg rm_dst) {
    e8(a, rex(1, rhigh1(reg_src), 0, rhigh1(rm_dst)));
    e8(a, opcode);
    e8(a, modrm(0x3, rlow3(reg_src), rlow3(rm_dst)));
}
static void e_ri32(X64Assembler *a, uint8_t opcode, uint8_t ext, X64Reg dst,
                   int32_t imm) {
    e8(a, rex(1, 0, 0, rhigh1(dst)));
    e8(a, opcode);
    e8(a, modrm(0x3, ext, rlow3(dst)));
    e32(a, (uint32_t)imm);
}
static void e_unary_f7(X64Assembler *a, X64Reg dst, uint8_t ext) {
    e8(a, rex(1, 0, 0, rhigh1(dst)));
    e8(a, 0xF7);
    e8(a, modrm(0x3, ext, rlow3(dst)));
}
static void e_load_store(X64Assembler *a, uint8_t opcode, X64Reg reg,
                         X64Reg base, int32_t disp) {
    int needs_sib = rlow3(base) == 4;
    e8(a, rex(1, rhigh1(reg), 0, rhigh1(base)));
    e8(a, opcode);
    if (needs_sib) {
        e8(a, modrm(0x2, rlow3(reg), 0x4));
        e8(a, (uint8_t)((0x4 << 3) | rlow3(base)));
    } else {
        e8(a, modrm(0x2, rlow3(reg), rlow3(base)));
    }
    e32(a, (uint32_t)disp);
}
static void e_shift_cl(X64Assembler *a, X64Reg dst, uint8_t ext) {
    e8(a, rex(1, 0, 0, rhigh1(dst)));
    e8(a, 0xD3);
    e8(a, modrm(0x3, ext, rlow3(dst)));
}
static void e_shift_imm(X64Assembler *a, X64Reg dst, uint8_t ext, uint8_t imm) {
    e8(a, rex(1, 0, 0, rhigh1(dst)));
    e8(a, 0xC1);
    e8(a, modrm(0x3, ext, rlow3(dst)));
    e8(a, imm);
}
static void e_sse_rr(X64Assembler *a, uint8_t prefix, uint8_t opcode, X64Reg dst,
                     X64Reg src) {
    e8(a, prefix);
    if (rhigh1(dst) || rhigh1(src)) {
        e8(a, rex(0, rhigh1(dst), 0, rhigh1(src)));
    }
    e8(a, 0x0F);
    e8(a, opcode);
    e8(a, modrm(0x3, rlow3(dst), rlow3(src)));
}
static void e_sse_mem(X64Assembler *a, uint8_t prefix, uint8_t opcode,
                      X64Reg xmm, X64Reg base, int32_t disp) {
    int needs_sib;
    e8(a, prefix);
    if (rhigh1(xmm) || rhigh1(base)) {
        e8(a, rex(0, rhigh1(xmm), 0, rhigh1(base)));
    }
    e8(a, 0x0F);
    e8(a, opcode);
    needs_sib = rlow3(base) == 4;
    if (needs_sib) {
        e8(a, modrm(0x2, rlow3(xmm), 0x4));
        e8(a, (uint8_t)((0x4 << 3) | rlow3(base)));
    } else {
        e8(a, modrm(0x2, rlow3(xmm), rlow3(base)));
    }
    e32(a, (uint32_t)disp);
}
static void e_sse_rr_w(X64Assembler *a, uint8_t prefix, uint8_t opcode,
                       X64Reg reg, X64Reg rm) {
    e8(a, prefix);
    e8(a, rex(1, rhigh1(reg), 0, rhigh1(rm)));
    e8(a, 0x0F);
    e8(a, opcode);
    e8(a, modrm(0x3, rlow3(reg), rlow3(rm)));
}
static void e_sse_rri_0f3a(X64Assembler *a, uint8_t opcode, X64Reg reg,
                           X64Reg rm, uint8_t imm8) {
    e8(a, 0x66);
    if (rhigh1(reg) || rhigh1(rm)) {
        e8(a, rex(0, rhigh1(reg), 0, rhigh1(rm)));
    }
    e8(a, 0x0F);
    e8(a, 0x3A);
    e8(a, opcode);
    e8(a, modrm(0x3, rlow3(reg), rlow3(rm)));
    e8(a, imm8);
}

/* Record a rel32 branch fix-up slot at the current position. */
static void branch_slot(X64Assembler *a, X64Label target) {
    size_t slot = a->code_len;
    e32(a, 0);
    if (a->err == X64_ERR_OUT_OF_MEMORY) {
        return;
    }
    if (!grow((void **)&a->fixups, &a->fixups_cap, a->fixups_len + 1,
              sizeof(Fixup))) {
        latch(a, X64_ERR_OUT_OF_MEMORY);
        return;
    }
    a->fixups[a->fixups_len].slot_offset = slot;
    a->fixups[a->fixups_len].instr_end_offset = a->code_len;
    a->fixups[a->fixups_len].target = target;
    a->fixups_len++;
}

static void add_reloc(X64Assembler *a, size_t patch_offset, const char *symbol,
                      X64RelocKind kind, int32_t addend) {
    char *sym;
    size_t n;
    if (a->err == X64_ERR_OUT_OF_MEMORY) {
        return;
    }
    if (!grow((void **)&a->relocs, &a->relocs_cap, a->relocs_len + 1,
              sizeof(Reloc))) {
        latch(a, X64_ERR_OUT_OF_MEMORY);
        return;
    }
    n = strlen(symbol) + 1;
    sym = (char *)malloc(n);
    if (!sym) {
        latch(a, X64_ERR_OUT_OF_MEMORY);
        return;
    }
    memcpy(sym, symbol, n);
    a->relocs[a->relocs_len].patch_offset = patch_offset;
    a->relocs[a->relocs_len].symbol = sym;
    a->relocs[a->relocs_len].kind = kind;
    a->relocs[a->relocs_len].addend = addend;
    a->relocs_len++;
}

/* ── Labels ─────────────────────────────────────────────────────────────────*/
X64Label x64_create_label(X64Assembler *a) {
    X64Label id = (X64Label)a->labels_len;
    if (!grow((void **)&a->labels, &a->labels_cap, a->labels_len + 1,
              sizeof(LabelSlot))) {
        latch(a, X64_ERR_OUT_OF_MEMORY);
        return id;
    }
    a->labels[a->labels_len].bound = 0;
    a->labels[a->labels_len].off = 0;
    a->labels_len++;
    return id;
}
void x64_bind(X64Assembler *a, X64Label label) {
    if ((size_t)label >= a->labels_len) {
        return;
    }
    if (a->labels[label].bound) {
        latch(a, X64_ERR_LABEL_ALREADY_BOUND);
        return;
    }
    a->labels[label].bound = 1;
    a->labels[label].off = a->code_len;
}

/* ── MOV family ─────────────────────────────────────────────────────────────*/
void x64_mov_r64_r64(X64Assembler *a, X64Reg dst, X64Reg src) {
    e_rr(a, 0x89, src, dst);
}
void x64_mov_r64_imm32(X64Assembler *a, X64Reg dst, int32_t imm) {
    e8(a, rex(1, 0, 0, rhigh1(dst)));
    e8(a, 0xC7);
    e8(a, modrm(0x3, 0, rlow3(dst)));
    e32(a, (uint32_t)imm);
}
void x64_mov_r64_imm64(X64Assembler *a, X64Reg dst, uint64_t imm) {
    e8(a, rex(1, 0, 0, rhigh1(dst)));
    e8(a, (uint8_t)(0xB8 + rlow3(dst)));
    e64(a, imm);
}
void x64_mov_r64_mem(X64Assembler *a, X64Reg dst, X64Reg base, int32_t disp) {
    e_load_store(a, 0x8B, dst, base, disp);
}
void x64_mov_mem_r64(X64Assembler *a, X64Reg base, int32_t disp, X64Reg src) {
    e_load_store(a, 0x89, src, base, disp);
}
void x64_lea_rip_rel(X64Assembler *a, X64Reg dst, const char *symbol,
                     X64RelocKind kind) {
    size_t patch;
    e8(a, rex(1, rhigh1(dst), 0, 0));
    e8(a, 0x8D);
    e8(a, modrm(0x0, rlow3(dst), 0x5));
    patch = a->code_len;
    e32(a, 0);
    add_reloc(a, patch, symbol, kind, -4);
}

/* ── Arithmetic ─────────────────────────────────────────────────────────────*/
void x64_add(X64Assembler *a, X64Reg dst, X64Reg src) { e_rr(a, 0x01, src, dst); }
void x64_sub(X64Assembler *a, X64Reg dst, X64Reg src) { e_rr(a, 0x29, src, dst); }
void x64_imul(X64Assembler *a, X64Reg dst, X64Reg src) {
    e8(a, rex(1, rhigh1(dst), 0, rhigh1(src)));
    e8(a, 0x0F);
    e8(a, 0xAF);
    e8(a, modrm(0x3, rlow3(dst), rlow3(src)));
}
void x64_idiv(X64Assembler *a, X64Reg divisor) { e_unary_f7(a, divisor, 7); }
void x64_div(X64Assembler *a, X64Reg divisor) { e_unary_f7(a, divisor, 6); }
void x64_cqo(X64Assembler *a) {
    e8(a, 0x48);
    e8(a, 0x99);
}
void x64_add_imm32(X64Assembler *a, X64Reg dst, int32_t imm) {
    e_ri32(a, 0x81, 0, dst, imm);
}
void x64_sub_imm32(X64Assembler *a, X64Reg dst, int32_t imm) {
    e_ri32(a, 0x81, 5, dst, imm);
}
void x64_neg(X64Assembler *a, X64Reg dst) { e_unary_f7(a, dst, 3); }

/* ── Logical ────────────────────────────────────────────────────────────────*/
void x64_and(X64Assembler *a, X64Reg dst, X64Reg src) { e_rr(a, 0x21, src, dst); }
void x64_or(X64Assembler *a, X64Reg dst, X64Reg src) { e_rr(a, 0x09, src, dst); }
void x64_xor(X64Assembler *a, X64Reg dst, X64Reg src) { e_rr(a, 0x31, src, dst); }
void x64_test(X64Assembler *a, X64Reg lhs, X64Reg rhs) {
    e_rr(a, 0x85, rhs, lhs);
}
void x64_not(X64Assembler *a, X64Reg dst) { e_unary_f7(a, dst, 2); }

/* ── Shifts ─────────────────────────────────────────────────────────────────*/
void x64_shl_cl(X64Assembler *a, X64Reg dst) { e_shift_cl(a, dst, 4); }
void x64_shr_cl(X64Assembler *a, X64Reg dst) { e_shift_cl(a, dst, 5); }
void x64_sar_cl(X64Assembler *a, X64Reg dst) { e_shift_cl(a, dst, 7); }
void x64_shl_imm8(X64Assembler *a, X64Reg dst, uint8_t imm) {
    e_shift_imm(a, dst, 4, imm);
}
void x64_shr_imm8(X64Assembler *a, X64Reg dst, uint8_t imm) {
    e_shift_imm(a, dst, 5, imm);
}
void x64_sar_imm8(X64Assembler *a, X64Reg dst, uint8_t imm) {
    e_shift_imm(a, dst, 7, imm);
}

/* ── Compare + set ──────────────────────────────────────────────────────────*/
void x64_cmp(X64Assembler *a, X64Reg lhs, X64Reg rhs) {
    e_rr(a, 0x39, rhs, lhs);
}
void x64_cmp_imm32(X64Assembler *a, X64Reg lhs, int32_t imm) {
    e_ri32(a, 0x81, 7, lhs, imm);
}
void x64_setcc(X64Assembler *a, X64Cond cond, X64Reg dst) {
    e8(a, rex(0, 0, 0, rhigh1(dst)));
    e8(a, 0x0F);
    e8(a, (uint8_t)(0x90 | (uint8_t)cond));
    e8(a, modrm(0x3, 0, rlow3(dst)));
}
void x64_movzx_r64_r8(X64Assembler *a, X64Reg dst, X64Reg src) {
    e8(a, rex(1, rhigh1(dst), 0, rhigh1(src)));
    e8(a, 0x0F);
    e8(a, 0xB6);
    e8(a, modrm(0x3, rlow3(dst), rlow3(src)));
}
void x64_movzx_r64_byte_at(X64Assembler *a, X64Reg dst, X64Reg base) {
    e8(a, rex(1, rhigh1(dst), 0, rhigh1(base)));
    e8(a, 0x0F);
    e8(a, 0xB6);
    e8(a, modrm(0x0, rlow3(dst), rlow3(base)));
}
void x64_mov_byte_at_r8(X64Assembler *a, X64Reg base, X64Reg src) {
    e8(a, rex(0, rhigh1(src), 0, rhigh1(base)));
    e8(a, 0x88);
    e8(a, modrm(0x0, rlow3(src), rlow3(base)));
}

/* ── SSE2 ───────────────────────────────────────────────────────────────────*/
void x64_movsd_load(X64Assembler *a, X64Reg dst_xmm, X64Reg base, int32_t disp) {
    e_sse_mem(a, 0xF2, 0x10, dst_xmm, base, disp);
}
void x64_movsd_store(X64Assembler *a, X64Reg base, int32_t disp,
                     X64Reg src_xmm) {
    e_sse_mem(a, 0xF2, 0x11, src_xmm, base, disp);
}
void x64_addsd(X64Assembler *a, X64Reg dst, X64Reg src) {
    e_sse_rr(a, 0xF2, 0x58, dst, src);
}
void x64_subsd(X64Assembler *a, X64Reg dst, X64Reg src) {
    e_sse_rr(a, 0xF2, 0x5C, dst, src);
}
void x64_mulsd(X64Assembler *a, X64Reg dst, X64Reg src) {
    e_sse_rr(a, 0xF2, 0x59, dst, src);
}
void x64_divsd(X64Assembler *a, X64Reg dst, X64Reg src) {
    e_sse_rr(a, 0xF2, 0x5E, dst, src);
}
void x64_ucomisd(X64Assembler *a, X64Reg lhs, X64Reg rhs) {
    e_sse_rr(a, 0x66, 0x2E, lhs, rhs);
}
void x64_cvtsi2sd(X64Assembler *a, X64Reg xmm_dst, X64Reg gpr_src) {
    e_sse_rr_w(a, 0xF2, 0x2A, xmm_dst, gpr_src);
}
void x64_cvttsd2si(X64Assembler *a, X64Reg gpr_dst, X64Reg xmm_src) {
    e_sse_rr_w(a, 0xF2, 0x2C, gpr_dst, xmm_src);
}
void x64_roundsd(X64Assembler *a, X64Reg xmm_dst, X64Reg xmm_src, uint8_t imm8) {
    e_sse_rri_0f3a(a, 0x0B, xmm_dst, xmm_src, imm8);
}
void x64_sqrtsd(X64Assembler *a, X64Reg xmm_dst, X64Reg xmm_src) {
    e_sse_rr(a, 0xF2, 0x51, xmm_dst, xmm_src);
}

/* ── Stack ──────────────────────────────────────────────────────────────────*/
void x64_push(X64Assembler *a, X64Reg src) {
    if (rhigh1(src)) {
        e8(a, rex(0, 0, 0, 1));
    }
    e8(a, (uint8_t)(0x50 + rlow3(src)));
}
void x64_pop(X64Assembler *a, X64Reg dst) {
    if (rhigh1(dst)) {
        e8(a, rex(0, 0, 0, 1));
    }
    e8(a, (uint8_t)(0x58 + rlow3(dst)));
}

/* ── Control flow / misc ────────────────────────────────────────────────────*/
void x64_jmp(X64Assembler *a, X64Label target) {
    e8(a, 0xE9);
    branch_slot(a, target);
}
void x64_jcc(X64Assembler *a, X64Cond cond, X64Label target) {
    e8(a, 0x0F);
    e8(a, (uint8_t)(0x80 | (uint8_t)cond));
    branch_slot(a, target);
}
void x64_call_rel32(X64Assembler *a, const char *symbol, X64RelocKind kind) {
    size_t patch;
    e8(a, 0xE8);
    patch = a->code_len;
    e32(a, 0);
    add_reloc(a, patch, symbol, kind, -4);
}
void x64_call_label(X64Assembler *a, X64Label target) {
    e8(a, 0xE8);
    branch_slot(a, target);
}
void x64_call_r64(X64Assembler *a, X64Reg target) {
    e8(a, rex(0, 0, 0, rhigh1(target)));
    e8(a, 0xFF);
    e8(a, modrm(0x3, 2, rlow3(target)));
}
void x64_ret(X64Assembler *a) { e8(a, 0xC3); }
void x64_nop(X64Assembler *a) { e8(a, 0x90); }
void x64_int3(X64Assembler *a) { e8(a, 0xCC); }
void x64_ud2(X64Assembler *a) {
    e8(a, 0x0F);
    e8(a, 0x0B);
}

/* ── External relocations ───────────────────────────────────────────────────*/
size_t x64_external_reloc_count(const X64Assembler *a) {
    return a ? a->relocs_len : 0;
}
int x64_external_reloc(const X64Assembler *a, size_t i, size_t *patch_offset,
                       const char **symbol, X64RelocKind *kind,
                       int32_t *addend) {
    if (!a || i >= a->relocs_len) {
        return 0;
    }
    if (patch_offset) *patch_offset = a->relocs[i].patch_offset;
    if (symbol) *symbol = a->relocs[i].symbol;
    if (kind) *kind = a->relocs[i].kind;
    if (addend) *addend = a->relocs[i].addend;
    return 1;
}

/* ── Finalisation ───────────────────────────────────────────────────────────*/
X64Status x64_finish(X64Assembler *a, uint8_t **out_bytes, size_t *out_len) {
    size_t i;
    uint8_t *bytes;
    *out_bytes = NULL;
    *out_len = 0;
    if (!a) {
        return X64_ERR_OUT_OF_MEMORY;
    }
    if (a->err != X64_OK) {
        return a->err;
    }
    for (i = 0; i < a->fixups_len; i++) {
        Fixup *f = &a->fixups[i];
        int64_t delta;
        uint32_t d;
        if ((size_t)f->target >= a->labels_len || !a->labels[f->target].bound) {
            return X64_ERR_UNBOUND_LABEL;
        }
        delta = (int64_t)a->labels[f->target].off -
                (int64_t)f->instr_end_offset;
        if (delta < INT32_MIN || delta > INT32_MAX) {
            return X64_ERR_BRANCH_OUT_OF_RANGE;
        }
        d = (uint32_t)(int32_t)delta;
        a->code[f->slot_offset + 0] = (uint8_t)(d & 0xFF);
        a->code[f->slot_offset + 1] = (uint8_t)((d >> 8) & 0xFF);
        a->code[f->slot_offset + 2] = (uint8_t)((d >> 16) & 0xFF);
        a->code[f->slot_offset + 3] = (uint8_t)((d >> 24) & 0xFF);
    }
    if (a->code_len == 0) {
        return X64_OK;
    }
    bytes = (uint8_t *)malloc(a->code_len);
    if (!bytes) {
        return X64_ERR_OUT_OF_MEMORY;
    }
    memcpy(bytes, a->code, a->code_len);
    *out_bytes = bytes;
    *out_len = a->code_len;
    return X64_OK;
}

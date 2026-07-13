/*
 * Tests for the C assembler library, using the header-only iso_test.h harness
 * (pure ISO). Cases mirror the Rust crate's own unit tests: register/immediate
 * parsing, instruction parsing, exact binary encoding, and error handling.
 */
#include "iso_test.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* free */
#include <string.h> /* strcmp */

#include "assembler.h"

/* Parse `src`, returning the instruction array (caller frees) or NULL. */
static ArmInstruction *parse(Assembler *a, const char *src, size_t *n) {
    ArmInstruction *out = NULL;
    AsmError err;
    if (asm_parse(a, src, &out, n, &err) != ASM_OK) return NULL;
    return out;
}

/* Encode `src` into a single word (assumes exactly one instruction word). */
static uint32_t encode_one(const char *src) {
    Assembler a;
    asm_init(&a);
    size_t n = 0;
    ArmInstruction *instrs = parse(&a, src, &n);
    uint32_t *words = NULL;
    size_t wlen = 0;
    asm_encode(instrs, n, &words, &wlen);
    uint32_t w = (wlen > 0) ? words[0] : 0;
    free(words);
    asm_instructions_free(instrs, n);
    asm_free(&a);
    return w;
}

int main(void) {
    Assembler a;

    /* ── register parsing (via MOV's rd) ──────────────────────────────── */
    {
        /* Valid registers 0..15, SP/LR/PC parse; invalid ones error. */
        struct { const char *reg; uint32_t val; } ok[] = {
            {"R0", 0}, {"R15", 15}, {"SP", 13}, {"LR", 14}, {"PC", 15}, {"r0", 0}, {"sp", 13}};
        for (size_t i = 0; i < sizeof(ok) / sizeof(ok[0]); i++) {
            char src[32];
            snprintf(src, sizeof(src), "MOV %s, #0", ok[i].reg);
            asm_init(&a);
            size_t n = 0;
            ArmInstruction *ins = parse(&a, src, &n);
            ISO_CHECK(ins != NULL && n == 1);
            ISO_CHECK(ins[0].has_rd && ins[0].rd == ok[i].val);
            asm_instructions_free(ins, n);
            asm_free(&a);
        }
        /* R16 / X0 are invalid registers */
        asm_init(&a);
        size_t n = 0;
        ArmInstruction *ins = NULL;
        AsmError err;
        ISO_CHECK(asm_parse(&a, "MOV R16, #0", &ins, &n, &err) == ASM_ERR_INVALID_REGISTER);
        ISO_CHECK(asm_parse(&a, "MOV X0, #1", &ins, &n, &err) == ASM_ERR_INVALID_REGISTER);
        ISO_CHECK(strcmp(err.message, "Invalid register: X0") == 0);
        asm_free(&a);
    }

    /* ── immediate parsing (via MOV's operand2; operand2 requires a '#'
     *    prefix — a bare number reaches parse_register and errors, matching
     *    the Rust parse_operand2, so it is not covered here). ──────────── */
    {
        struct { const char *imm; uint32_t val; } cases[] = {
            {"#42", 42}, {"#0", 0}, {"#255", 255}, {"#0xFF", 255}, {"#0x10", 16}};
        for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
            char src[32];
            snprintf(src, sizeof(src), "MOV R0, %s", cases[i].imm);
            asm_init(&a);
            size_t n = 0;
            ArmInstruction *ins = parse(&a, src, &n);
            ISO_CHECK(ins != NULL && n == 1);
            ISO_CHECK(ins[0].operand2.kind == ASM_OPERAND2_IMMEDIATE &&
                      ins[0].operand2.value == cases[i].val);
            asm_instructions_free(ins, n);
            asm_free(&a);
        }
    }

    /* ── instruction parsing ──────────────────────────────────────────── */
    {
        asm_init(&a);
        size_t n = 0;
        ArmInstruction *ins = parse(&a, "MOV R0, #42", &n);
        ISO_CHECK(n == 1 && ins[0].kind == ASM_INSTR_DATA_PROCESSING);
        ISO_CHECK(ins[0].opcode == ARM_MOV && ins[0].rd == 0);
        ISO_CHECK(ins[0].operand2.kind == ASM_OPERAND2_IMMEDIATE && ins[0].operand2.value == 42);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "ADD R2, R0, R1", &n);
        ISO_CHECK(ins[0].opcode == ARM_ADD && ins[0].rd == 2 && ins[0].has_rn && ins[0].rn == 0);
        ISO_CHECK(ins[0].operand2.kind == ASM_OPERAND2_REGISTER && ins[0].operand2.value == 1);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "SUB R3, R1, R2", &n);
        ISO_CHECK(ins[0].opcode == ARM_SUB);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "CMP R0, R1", &n);
        ISO_CHECK(ins[0].opcode == ARM_CMP && !ins[0].has_rd && ins[0].set_flags);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "LDR R0, [R1]", &n);
        ISO_CHECK(ins[0].kind == ASM_INSTR_LOAD && ins[0].rd == 0 && ins[0].rn == 1);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "STR R0, [R1]", &n);
        ISO_CHECK(ins[0].kind == ASM_INSTR_STORE && ins[0].rd == 0 && ins[0].rn == 1);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "NOP", &n);
        ISO_CHECK(ins[0].kind == ASM_INSTR_NOP);
        asm_instructions_free(ins, n);
        asm_free(&a);

        /* label: records address 0 and produces a Label instruction */
        asm_init(&a);
        ins = parse(&a, "loop:", &n);
        ISO_CHECK(ins[0].kind == ASM_INSTR_LABEL && strcmp(ins[0].label, "loop") == 0);
        size_t addr = 999;
        ISO_CHECK(asm_label_lookup(&a, "loop", &addr) && addr == 0);
        asm_instructions_free(ins, n);
        asm_free(&a);

        /* comments stripped, empty lines skipped, multiple instructions */
        asm_init(&a);
        ins = parse(&a, "MOV R0, #1 ; load one", &n);
        ISO_CHECK(n == 1);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "\n\nMOV R0, #1\n\n", &n);
        ISO_CHECK(n == 1);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a, "MOV R0, #10\nMOV R1, #20\nADD R2, R0, R1", &n);
        ISO_CHECK(n == 3);
        asm_instructions_free(ins, n);
        asm_free(&a);
    }

    /* ── binary encoding (exact words) ────────────────────────────────── */
    {
        /* MOV R0, #42: cond=E, I=1, opcode=D, S=0, Rn=0, Rd=0, imm=42 */
        uint32_t w = encode_one("MOV R0, #42");
        ISO_CHECK(((w >> 28) & 0xF) == 0xE);
        ISO_CHECK(((w >> 25) & 0x1) == 1);
        ISO_CHECK(((w >> 21) & 0xF) == 0xD);
        ISO_CHECK(((w >> 12) & 0xF) == 0);
        ISO_CHECK((w & 0xFFF) == 42);
        ISO_CHECK(w == 0xE3A0002Au); /* full word */

        /* ADD R2, R0, R1 */
        w = encode_one("ADD R2, R0, R1");
        ISO_CHECK(((w >> 25) & 0x1) == 0);   /* register operand */
        ISO_CHECK(((w >> 21) & 0xF) == 0x4); /* ADD */
        ISO_CHECK(((w >> 16) & 0xF) == 0);   /* Rn = R0 */
        ISO_CHECK(((w >> 12) & 0xF) == 2);   /* Rd = R2 */
        ISO_CHECK((w & 0xF) == 1);           /* Rm = R1 */
        ISO_CHECK(w == 0xE0802001u);

        /* NOP */
        ISO_CHECK(encode_one("NOP") == 0xE1A00000u);

        /* LDR sets bit 20; STR clears it */
        ISO_CHECK(((encode_one("LDR R0, [R1]") >> 20) & 0x1) == 1);
        ISO_CHECK(((encode_one("STR R0, [R1]") >> 20) & 0x1) == 0);
        ISO_CHECK(encode_one("LDR R0, [R1]") == 0xE5910000u);
        ISO_CHECK(encode_one("STR R0, [R1]") == 0xE5810000u);
    }

    /* ── labels produce no binary; full program length ────────────────── */
    {
        asm_init(&a);
        size_t n = 0;
        ArmInstruction *ins = parse(&a, "start:\nMOV R0, #1", &n);
        uint32_t *words = NULL;
        size_t wlen = 0;
        asm_encode(ins, n, &words, &wlen);
        ISO_CHECK(wlen == 1); /* only the MOV emits a word */
        free(words);
        asm_instructions_free(ins, n);
        asm_free(&a);

        asm_init(&a);
        ins = parse(&a,
                    "MOV R0, #10\nMOV R1, #20\nADD R2, R0, R1\nSTR R2, [R3]", &n);
        asm_encode(ins, n, &words, &wlen);
        ISO_CHECK(wlen == 4);
        free(words);
        asm_instructions_free(ins, n);
        asm_free(&a);
    }

    /* ── error cases ──────────────────────────────────────────────────── */
    {
        asm_init(&a);
        size_t n = 0;
        ArmInstruction *ins = NULL;
        AsmError err;
        ISO_CHECK(asm_parse(&a, "BLAH R0, R1", &ins, &n, &err) == ASM_ERR_UNKNOWN_MNEMONIC);
        ISO_CHECK(strcmp(err.message, "Unknown mnemonic: BLAH") == 0);
        ISO_CHECK(asm_parse(&a, "MOV X0, #1", &ins, &n, &err) == ASM_ERR_INVALID_REGISTER);
        ISO_CHECK(asm_parse(&a, "ADD R0, R1", &ins, &n, &err) == ASM_ERR_INVALID_OPERAND_COUNT);
        ISO_CHECK(strcmp(err.message, "ADD: expected 3 operands, got 2") == 0);
        asm_free(&a);
    }

    return ISO_TEST_RESULT();
}

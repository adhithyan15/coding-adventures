/*
 * assembler.c — implementation of the pure-ISO C ARM assembler.
 * ============================================================
 *
 * Two passes, exactly like the Rust crate: `asm_parse` turns source text into
 * a flat array of `ArmInstruction`s (recording labels in the assembler), and
 * `asm_encode` turns each instruction into its 32-bit ARM word.
 */
#include "assembler.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, strcmp, strlen, strchr */

/* ── Label table ──────────────────────────────────────────────────────────*/

struct AsmLabel {
    char *name;
    size_t address;
};

void asm_init(Assembler *a) {
    a->labels = NULL;
    a->num_labels = 0;
    a->cap_labels = 0;
}

void asm_free(Assembler *a) {
    if (a == NULL) return;
    for (size_t i = 0; i < a->num_labels; i++) free(a->labels[i].name);
    free(a->labels);
    a->labels = NULL;
    a->num_labels = 0;
    a->cap_labels = 0;
}

int asm_label_lookup(const Assembler *a, const char *name, size_t *out_addr) {
    for (size_t i = 0; i < a->num_labels; i++)
        if (strcmp(a->labels[i].name, name) == 0) {
            *out_addr = a->labels[i].address;
            return 1;
        }
    return 0;
}

/* Insert or update a label; returns 0 on OOM. Takes a copy of `name`. */
static int label_insert(Assembler *a, const char *name, size_t address) {
    for (size_t i = 0; i < a->num_labels; i++)
        if (strcmp(a->labels[i].name, name) == 0) {
            a->labels[i].address = address; /* HashMap overwrite semantics */
            return 1;
        }
    if (a->num_labels == a->cap_labels) {
        size_t nc = a->cap_labels ? a->cap_labels : 4;
        if (nc > ((size_t)-1) / 2 / sizeof(struct AsmLabel)) return 0;
        nc *= 2;
        struct AsmLabel *p =
            (struct AsmLabel *)realloc(a->labels, nc * sizeof(struct AsmLabel));
        if (p == NULL) return 0;
        a->labels = p;
        a->cap_labels = nc;
    }
    size_t len = strlen(name);
    char *copy = (char *)malloc(len + 1);
    if (copy == NULL) return 0;
    memcpy(copy, name, len + 1);
    a->labels[a->num_labels].name = copy;
    a->labels[a->num_labels].address = address;
    a->num_labels++;
    return 1;
}

/* ── Small string helpers ─────────────────────────────────────────────────*/

static int is_ws(char c) {
    return c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\v' ||
           c == '\f';
}

static char up(char c) { return (c >= 'a' && c <= 'z') ? (char)(c - 32) : c; }

/* Copy s[0..len) into buf (NUL-terminated) uppercased; returns 0 if it does not
 * fit (bufsz includes the terminator). */
static int copy_upper(const char *s, size_t len, char *buf, size_t bufsz) {
    if (len + 1 > bufsz) return 0;
    for (size_t i = 0; i < len; i++) buf[i] = up(s[i]);
    buf[len] = '\0';
    return 1;
}

/* Trim a mutable string in place: skip leading whitespace, NUL-terminate after
 * the last non-whitespace char. */
static char *trim_inplace(char *s) {
    while (is_ws(*s)) s++;
    size_t len = strlen(s);
    while (len > 0 && is_ws(s[len - 1])) s[--len] = '\0';
    return s;
}

/* ── Register / immediate parsing ─────────────────────────────────────────*/

/* Parse a register name (already trimmed). Returns 1 and writes *out on
 * success. Accepts R0..R15, SP, LR, PC (case-insensitive). */
static int parse_register(const char *s, uint32_t *out) {
    char buf[16];
    size_t len = strlen(s);
    if (!copy_upper(s, len, buf, sizeof(buf))) return 0; /* too long -> invalid */
    if (strcmp(buf, "SP") == 0) { *out = 13; return 1; }
    if (strcmp(buf, "LR") == 0) { *out = 14; return 1; }
    if (strcmp(buf, "PC") == 0) { *out = 15; return 1; }
    if (buf[0] != 'R' || buf[1] == '\0') return 0;
    uint32_t n = 0;
    for (size_t i = 1; buf[i] != '\0'; i++) {
        if (buf[i] < '0' || buf[i] > '9') return 0;
        n = n * 10 + (uint32_t)(buf[i] - '0');
        if (n > 15) return 0;
    }
    *out = n;
    return 1;
}

/* Parse an immediate (already trimmed): optional leading '#', then decimal or
 * 0x-hex. Returns 1 and writes *out on success. */
static int parse_immediate(const char *s, uint32_t *out) {
    if (*s == '#') {
        s++;
        while (is_ws(*s)) s++;
    }
    int hex = 0;
    if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) {
        hex = 1;
        s += 2;
    }
    if (*s == '\0') return 0;
    uint32_t n = 0;
    for (; *s != '\0'; s++) {
        int d;
        if (*s >= '0' && *s <= '9') d = *s - '0';
        else if (hex && *s >= 'a' && *s <= 'f') d = *s - 'a' + 10;
        else if (hex && *s >= 'A' && *s <= 'F') d = *s - 'A' + 10;
        else return 0;
        n = n * (hex ? 16u : 10u) + (uint32_t)d;
    }
    *out = n;
    return 1;
}

/* ── Parsing ──────────────────────────────────────────────────────────────*/

/* Grow an instruction array by one slot (overflow-guarded). Returns 0 on OOM. */
static int instrs_push(ArmInstruction **arr, size_t *len, size_t *cap,
                       ArmInstruction v) {
    if (*len == *cap) {
        size_t nc = *cap ? *cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(ArmInstruction)) return 0;
        nc *= 2;
        ArmInstruction *p =
            (ArmInstruction *)realloc(*arr, nc * sizeof(ArmInstruction));
        if (p == NULL) return 0;
        *arr = p;
        *cap = nc;
    }
    (*arr)[(*len)++] = v;
    return 1;
}

void asm_instructions_free(ArmInstruction *instrs, size_t n) {
    if (instrs == NULL) return;
    for (size_t i = 0; i < n; i++)
        if (instrs[i].kind == ASM_INSTR_LABEL) free(instrs[i].label);
    free(instrs);
}

static void set_err(AsmError *err, AsmStatus code, const char *fmt,
                    const char *arg) {
    if (err == NULL) return;
    err->code = code;
    snprintf(err->message, sizeof(err->message), fmt, arg);
}

static void set_err_count(AsmError *err, const char *mnemonic, size_t expected,
                          size_t got) {
    if (err == NULL) return;
    err->code = ASM_ERR_INVALID_OPERAND_COUNT;
    snprintf(err->message, sizeof(err->message),
             "%s: expected %zu operands, got %zu", mnemonic, expected, got);
}

/* Split `s` (mutable) by ',' into up to `max` trimmed tokens; writes count via
 * *count. An empty string yields 0 tokens; more than `max` tokens returns 0. */
static int split_operands(char *s, char **out, size_t max, size_t *count) {
    *count = 0;
    if (*s == '\0') return 1;
    char *tok = s;
    for (;;) {
        char *comma = strchr(tok, ',');
        if (comma != NULL) *comma = '\0';
        if (*count >= max) return 0;
        out[(*count)++] = trim_inplace(tok);
        if (comma == NULL) break;
        tok = comma + 1;
    }
    return 1;
}

/* Fill operand2 from a trimmed token, returning a status. */
static AsmStatus parse_operand2(const char *tok, AsmOperand2 *op2, AsmError *err) {
    if (tok[0] == '#') {
        uint32_t imm;
        if (!parse_immediate(tok, &imm)) {
            set_err(err, ASM_ERR_INVALID_IMMEDIATE, "Invalid immediate: %s", tok);
            return ASM_ERR_INVALID_IMMEDIATE;
        }
        op2->kind = ASM_OPERAND2_IMMEDIATE;
        op2->value = imm;
        return ASM_OK;
    }
    uint32_t reg;
    if (parse_register(tok, &reg)) {
        op2->kind = ASM_OPERAND2_REGISTER;
        op2->value = reg;
        return ASM_OK;
    }
    set_err(err, ASM_ERR_PARSE, "Parse error: Cannot parse operand: %s", tok);
    return ASM_ERR_PARSE;
}

/* Parse a single (mutable, trimmed) instruction line into *out. */
static AsmStatus parse_instruction(char *line, ArmInstruction *out,
                                   AsmError *err) {
    /* Split mnemonic (first token) from the operand string. */
    char *sp = line;
    while (*sp != '\0' && !is_ws(*sp)) sp++;
    char *operands_str;
    if (*sp != '\0') {
        *sp = '\0';
        operands_str = trim_inplace(sp + 1);
    } else {
        operands_str = sp; /* empty */
    }
    char mnem[16];
    if (!copy_upper(line, strlen(line), mnem, sizeof(mnem))) {
        set_err(err, ASM_ERR_UNKNOWN_MNEMONIC, "Unknown mnemonic: %s", line);
        return ASM_ERR_UNKNOWN_MNEMONIC;
    }

    char *ops[8];
    size_t nops = 0;

    if (strcmp(mnem, "NOP") == 0) {
        *out = (ArmInstruction){0};
        out->kind = ASM_INSTR_NOP;
        return ASM_OK;
    }

    if (strcmp(mnem, "MOV") == 0 || strcmp(mnem, "MOVS") == 0) {
        split_operands(operands_str, ops, 8, &nops);
        if (nops != 2) { set_err_count(err, mnem, 2, nops); return ASM_ERR_INVALID_OPERAND_COUNT; }
        uint32_t rd;
        if (!parse_register(ops[0], &rd)) {
            set_err(err, ASM_ERR_INVALID_REGISTER, "Invalid register: %s", ops[0]);
            return ASM_ERR_INVALID_REGISTER;
        }
        *out = (ArmInstruction){0};
        out->kind = ASM_INSTR_DATA_PROCESSING;
        out->opcode = ARM_MOV;
        out->has_rd = 1;
        out->rd = rd;
        out->has_rn = 0;
        out->set_flags = strcmp(mnem, "MOVS") == 0;
        return parse_operand2(ops[1], &out->operand2, err);
    }

    if (strcmp(mnem, "ADD") == 0 || strcmp(mnem, "ADDS") == 0 ||
        strcmp(mnem, "SUB") == 0 || strcmp(mnem, "SUBS") == 0 ||
        strcmp(mnem, "AND") == 0 || strcmp(mnem, "ANDS") == 0 ||
        strcmp(mnem, "ORR") == 0 || strcmp(mnem, "ORRS") == 0 ||
        strcmp(mnem, "EOR") == 0 || strcmp(mnem, "EORS") == 0 ||
        strcmp(mnem, "RSB") == 0 || strcmp(mnem, "RSBS") == 0) {
        /* base = mnemonic with trailing 'S' removed; set_flags if it had one. */
        char base[16];
        size_t bl = strlen(mnem);
        while (bl > 0 && mnem[bl - 1] == 'S') bl--;
        memcpy(base, mnem, bl);
        base[bl] = '\0';
        int set_flags = strlen(mnem) > bl;
        ArmOpcode opcode;
        if (strcmp(base, "AND") == 0) opcode = ARM_AND;
        else if (strcmp(base, "EOR") == 0) opcode = ARM_EOR;
        else if (strcmp(base, "SUB") == 0) opcode = ARM_SUB;
        else if (strcmp(base, "RSB") == 0) opcode = ARM_RSB;
        else if (strcmp(base, "ADD") == 0) opcode = ARM_ADD;
        else if (strcmp(base, "ORR") == 0) opcode = ARM_ORR;
        else {
            set_err(err, ASM_ERR_UNKNOWN_MNEMONIC, "Unknown mnemonic: %s", mnem);
            return ASM_ERR_UNKNOWN_MNEMONIC;
        }
        split_operands(operands_str, ops, 8, &nops);
        if (nops != 3) { set_err_count(err, mnem, 3, nops); return ASM_ERR_INVALID_OPERAND_COUNT; }
        uint32_t rd, rn;
        if (!parse_register(ops[0], &rd)) {
            set_err(err, ASM_ERR_INVALID_REGISTER, "Invalid register: %s", ops[0]);
            return ASM_ERR_INVALID_REGISTER;
        }
        if (!parse_register(ops[1], &rn)) {
            set_err(err, ASM_ERR_INVALID_REGISTER, "Invalid register: %s", ops[1]);
            return ASM_ERR_INVALID_REGISTER;
        }
        *out = (ArmInstruction){0};
        out->kind = ASM_INSTR_DATA_PROCESSING;
        out->opcode = opcode;
        out->has_rd = 1;
        out->rd = rd;
        out->has_rn = 1;
        out->rn = rn;
        out->set_flags = set_flags;
        return parse_operand2(ops[2], &out->operand2, err);
    }

    if (strcmp(mnem, "CMP") == 0) {
        split_operands(operands_str, ops, 8, &nops);
        if (nops != 2) { set_err_count(err, mnem, 2, nops); return ASM_ERR_INVALID_OPERAND_COUNT; }
        uint32_t rn;
        if (!parse_register(ops[0], &rn)) {
            set_err(err, ASM_ERR_INVALID_REGISTER, "Invalid register: %s", ops[0]);
            return ASM_ERR_INVALID_REGISTER;
        }
        *out = (ArmInstruction){0};
        out->kind = ASM_INSTR_DATA_PROCESSING;
        out->opcode = ARM_CMP;
        out->has_rd = 0;
        out->has_rn = 1;
        out->rn = rn;
        out->set_flags = 1;
        return parse_operand2(ops[1], &out->operand2, err);
    }

    if (strcmp(mnem, "LDR") == 0 || strcmp(mnem, "STR") == 0) {
        split_operands(operands_str, ops, 8, &nops);
        if (nops != 2) { set_err_count(err, mnem, 2, nops); return ASM_ERR_INVALID_OPERAND_COUNT; }
        uint32_t rd, rn;
        if (!parse_register(ops[0], &rd)) {
            set_err(err, ASM_ERR_INVALID_REGISTER, "Invalid register: %s", ops[0]);
            return ASM_ERR_INVALID_REGISTER;
        }
        /* strip a leading '[' and trailing ']', then trim. */
        char *base = ops[1];
        while (*base == '[') base++;
        size_t bl = strlen(base);
        while (bl > 0 && base[bl - 1] == ']') base[--bl] = '\0';
        base = trim_inplace(base);
        if (!parse_register(base, &rn)) {
            set_err(err, ASM_ERR_INVALID_REGISTER, "Invalid register: %s", base);
            return ASM_ERR_INVALID_REGISTER;
        }
        *out = (ArmInstruction){0};
        out->kind = (mnem[0] == 'L') ? ASM_INSTR_LOAD : ASM_INSTR_STORE;
        out->has_rd = 1;
        out->rd = rd;
        out->has_rn = 1;
        out->rn = rn;
        return ASM_OK;
    }

    set_err(err, ASM_ERR_UNKNOWN_MNEMONIC, "Unknown mnemonic: %s", mnem);
    return ASM_ERR_UNKNOWN_MNEMONIC;
}

AsmStatus asm_parse(Assembler *a, const char *source, ArmInstruction **out,
                    size_t *out_len, AsmError *err) {
    *out = NULL;
    *out_len = 0;

    ArmInstruction *arr = NULL;
    size_t len = 0, cap = 0;
    size_t address = 0;
    AsmStatus st = ASM_OK;

    const char *p = source;
    while (*p != '\0') {
        /* next line [line, line_end), split on '\n' (drop a trailing '\r'). */
        const char *line = p;
        while (*p != '\0' && *p != '\n') p++;
        const char *line_end = p;
        if (*p == '\n') p++;
        if (line_end > line && line_end[-1] == '\r') line_end--;

        /* strip comments: first ';' then first "//" within the line. */
        for (const char *q = line; q < line_end; q++)
            if (*q == ';') { line_end = q; break; }
        for (const char *q = line; q + 1 < line_end; q++)
            if (q[0] == '/' && q[1] == '/') { line_end = q; break; }

        /* trim */
        while (line < line_end && is_ws(*line)) line++;
        while (line_end > line && is_ws(line_end[-1])) line_end--;
        size_t line_len = (size_t)(line_end - line);
        if (line_len == 0) continue;

        /* label? (ends with ':') */
        if (line[line_len - 1] == ':') {
            const char *ns = line;
            const char *ne = line_end - 1;
            while (ns < ne && is_ws(*ns)) ns++;
            while (ne > ns && is_ws(ne[-1])) ne--;
            size_t nlen = (size_t)(ne - ns);
            char *name = (char *)malloc(nlen + 1);
            if (name == NULL) { st = ASM_ERR_ALLOC; goto fail; }
            memcpy(name, ns, nlen);
            name[nlen] = '\0';
            if (!label_insert(a, name, address)) {
                free(name);
                st = ASM_ERR_ALLOC;
                goto fail;
            }
            ArmInstruction li = {0};
            li.kind = ASM_INSTR_LABEL;
            li.label = name; /* the instruction owns this copy */
            if (!instrs_push(&arr, &len, &cap, li)) {
                free(name);
                st = ASM_ERR_ALLOC;
                goto fail;
            }
            continue;
        }

        /* mutable copy for in-place tokenising */
        char *buf = (char *)malloc(line_len + 1);
        if (buf == NULL) { st = ASM_ERR_ALLOC; goto fail; }
        memcpy(buf, line, line_len);
        buf[line_len] = '\0';

        ArmInstruction instr;
        st = parse_instruction(buf, &instr, err);
        free(buf);
        if (st != ASM_OK) goto fail;

        if (!instrs_push(&arr, &len, &cap, instr)) {
            if (instr.kind == ASM_INSTR_LABEL) free(instr.label);
            st = ASM_ERR_ALLOC;
            goto fail;
        }
        address += 1;
    }

    *out = arr;
    *out_len = len;
    return ASM_OK;

fail:
    asm_instructions_free(arr, len);
    if (st == ASM_ERR_ALLOC && err != NULL) {
        err->code = ASM_ERR_ALLOC;
        snprintf(err->message, sizeof(err->message), "out of memory");
    }
    return st;
}

/* ── Encoding ─────────────────────────────────────────────────────────────*/

AsmStatus asm_encode(const ArmInstruction *instrs, size_t n, uint32_t **out,
                     size_t *out_len) {
    *out = NULL;
    *out_len = 0;

    size_t words = 0;
    for (size_t i = 0; i < n; i++)
        if (instrs[i].kind != ASM_INSTR_LABEL) words++;

    uint32_t *buf = NULL;
    if (words > 0) {
        if (words > ((size_t)-1) / sizeof(uint32_t)) return ASM_ERR_ALLOC;
        buf = (uint32_t *)malloc(words * sizeof(uint32_t));
        if (buf == NULL) return ASM_ERR_ALLOC;
    }

    size_t w = 0;
    for (size_t i = 0; i < n; i++) {
        const ArmInstruction *in = &instrs[i];
        switch (in->kind) {
            case ASM_INSTR_LABEL:
                break; /* no output */
            case ASM_INSTR_NOP:
                buf[w++] = 0xE1A00000u;
                break;
            case ASM_INSTR_DATA_PROCESSING: {
                uint32_t cond = 0xE;
                uint32_t rd = in->has_rd ? in->rd : 0;
                uint32_t rn = in->has_rn ? in->rn : 0;
                uint32_t s = in->set_flags ? 1u : 0u;
                uint32_t opcode = (uint32_t)in->opcode;
                uint32_t i_bit, op2;
                if (in->operand2.kind == ASM_OPERAND2_IMMEDIATE) {
                    i_bit = 1;
                    op2 = in->operand2.value & 0xFFFu;
                } else {
                    i_bit = 0;
                    op2 = in->operand2.value & 0xFu;
                }
                buf[w++] = (cond << 28) | (i_bit << 25) | (opcode << 21) |
                           (s << 20) | (rn << 16) | (rd << 12) | op2;
                break;
            }
            case ASM_INSTR_LOAD:
                buf[w++] = 0xE5900000u | (in->rn << 16) | (in->rd << 12);
                break;
            case ASM_INSTR_STORE:
                buf[w++] = 0xE5800000u | (in->rn << 16) | (in->rd << 12);
                break;
        }
    }

    *out = buf;
    *out_len = words;
    return ASM_OK;
}

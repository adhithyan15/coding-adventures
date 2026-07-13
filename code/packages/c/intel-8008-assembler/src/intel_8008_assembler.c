/*
 * intel_8008_assembler.c — a two-pass Intel 8008 assembler, pure ISO C17.
 * =======================================================================
 *
 * See intel_8008_assembler.h for the design. The structure mirrors the Rust
 * crate: a line lexer produces `ParsedLine`s; Pass 1 walks them to build the
 * symbol table; Pass 2 walks them again to emit bytes.
 */
#include "intel_8008_assembler.h"

#include <ctype.h>  /* isspace, isalpha, isalnum, isdigit */
#include <stdarg.h> /* va_list */
#include <stdio.h>  /* vsnprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, strcmp, strlen */

/* ── Diagnostics ────────────────────────────────────────────────────────────*/
static void set_err(char *errbuf, size_t errlen, const char *fmt, ...) {
    va_list ap;
    if (!errbuf || errlen == 0) {
        return;
    }
    va_start(ap, fmt);
    vsnprintf(errbuf, errlen, fmt, ap);
    va_end(ap);
}

/* ── String helpers ─────────────────────────────────────────────────────────*/
static char *sdup_n(const char *p, size_t n) {
    char *s = (char *)malloc(n + 1);
    if (!s) {
        return NULL;
    }
    if (n) {
        memcpy(s, p, n);
    }
    s[n] = '\0';
    return s;
}
static char *sdup(const char *p) { return sdup_n(p, strlen(p)); }

/* Trim ASCII whitespace: return start pointer and set *len to trimmed length. */
static const char *trim(const char *s, size_t *len) {
    size_t b = 0, e = strlen(s);
    while (b < e && isspace((unsigned char)s[b])) {
        b++;
    }
    while (e > b && isspace((unsigned char)s[e - 1])) {
        e--;
    }
    *len = e - b;
    return s + b;
}

static char upper_c(char c) { return (c >= 'a' && c <= 'z') ? (char)(c - 32) : c; }
static char lower_c(char c) { return (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c; }

/* Owned uppercase copy. NULL on OOM. */
static char *to_upper_dup(const char *p, size_t n) {
    char *s = (char *)malloc(n + 1);
    size_t i;
    if (!s) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        s[i] = upper_c(p[i]);
    }
    s[n] = '\0';
    return s;
}

static int str_eq(const char *a, const char *b) { return strcmp(a, b) == 0; }

/* ── Growth guard ───────────────────────────────────────────────────────────*/
static int grow(void **data, size_t *cap, size_t need, size_t elem) {
    size_t nc;
    void *nd;
    if (need <= *cap) {
        return 1;
    }
    nc = *cap ? *cap : 4;
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

/* ── Byte buffer (assembler output) ─────────────────────────────────────────*/
typedef struct {
    uint8_t *data;
    size_t len, cap;
    int oom;
} ByteBuf;

static void bb_push(ByteBuf *b, uint8_t v) {
    if (b->oom) {
        return;
    }
    if (!grow((void **)&b->data, &b->cap, b->len + 1, 1)) {
        b->oom = 1;
        return;
    }
    b->data[b->len++] = v;
}
static void bb_fill(ByteBuf *b, uint8_t v, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        bb_push(b, v);
    }
}
static void bb_extend(ByteBuf *b, const uint8_t *p, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        bb_push(b, p[i]);
    }
}

/* ── Symbol table ───────────────────────────────────────────────────────────*/
typedef struct {
    char *name;
    size_t addr;
} SymEnt;
struct Intel8008Symbols {
    SymEnt *data;
    size_t len, cap;
};

Intel8008Symbols *intel8008_symbols_new(void) {
    return (Intel8008Symbols *)calloc(1, sizeof(Intel8008Symbols));
}
void intel8008_symbols_free(Intel8008Symbols *s) {
    size_t i;
    if (!s) {
        return;
    }
    for (i = 0; i < s->len; i++) {
        free(s->data[i].name);
    }
    free(s->data);
    free(s);
}
int intel8008_symbols_set(Intel8008Symbols *s, const char *name, size_t addr) {
    size_t i;
    for (i = 0; i < s->len; i++) {
        if (str_eq(s->data[i].name, name)) {
            s->data[i].addr = addr; /* overwrite (HashMap::insert semantics) */
            return 1;
        }
    }
    if (!grow((void **)&s->data, &s->cap, s->len + 1, sizeof(SymEnt))) {
        return 0;
    }
    s->data[s->len].name = sdup(name);
    if (!s->data[s->len].name) {
        return 0;
    }
    s->data[s->len].addr = addr;
    s->len++;
    return 1;
}
int intel8008_symbols_get(const Intel8008Symbols *s, const char *name,
                          size_t *addr) {
    size_t i;
    if (!s) {
        return 0;
    }
    for (i = 0; i < s->len; i++) {
        if (str_eq(s->data[i].name, name)) {
            if (addr) {
                *addr = s->data[i].addr;
            }
            return 1;
        }
    }
    return 0;
}

/* ── Numeric parsing ────────────────────────────────────────────────────────*/
static Intel8008Status parse_number(const char *raw, size_t *out, char *errbuf,
                                    size_t errlen) {
    size_t len;
    const char *t = trim(raw, &len);
    int hex = 0;
    size_t i, value = 0;
    const char *digits = t;
    size_t dlen = len;
    if (len >= 2 && t[0] == '0' && (t[1] == 'x' || t[1] == 'X')) {
        hex = 1;
        digits = t + 2;
        dlen = len - 2;
    }
    if (dlen == 0) {
        set_err(errbuf, errlen, "Invalid %s literal: \"%.*s\"",
                hex ? "hex" : "numeric", (int)len, t);
        return INTEL8008_ERR;
    }
    for (i = 0; i < dlen; i++) {
        char c = digits[i];
        unsigned d;
        size_t base = hex ? 16u : 10u;
        if (c >= '0' && c <= '9') {
            d = (unsigned)(c - '0');
        } else if (hex && c >= 'a' && c <= 'f') {
            d = (unsigned)(c - 'a' + 10);
        } else if (hex && c >= 'A' && c <= 'F') {
            d = (unsigned)(c - 'A' + 10);
        } else {
            set_err(errbuf, errlen, "Invalid %s literal: \"%.*s\"",
                    hex ? "hex" : "numeric", (int)len, t);
            return INTEL8008_ERR;
        }
        /* overflow guard (usize::parse errors on overflow) */
        if (value > ((size_t)-1 - d) / base) {
            set_err(errbuf, errlen, "Invalid %s literal: \"%.*s\"",
                    hex ? "hex" : "numeric", (int)len, t);
            return INTEL8008_ERR;
        }
        value = value * base + d;
    }
    *out = value;
    return INTEL8008_OK;
}

/* ── Register parsing ───────────────────────────────────────────────────────*/
static Intel8008Status parse_register(const char *name, uint8_t *out,
                                      char *errbuf, size_t errlen) {
    size_t len, i;
    const char *p = trim(name, &len);
    char up[8];
    if (len == 1) {
        char c = upper_c(p[0]);
        switch (c) {
            case 'B': *out = 0; return INTEL8008_OK;
            case 'C': *out = 1; return INTEL8008_OK;
            case 'D': *out = 2; return INTEL8008_OK;
            case 'E': *out = 3; return INTEL8008_OK;
            case 'H': *out = 4; return INTEL8008_OK;
            case 'L': *out = 5; return INTEL8008_OK;
            case 'M': *out = 6; return INTEL8008_OK;
            case 'A': *out = 7; return INTEL8008_OK;
            default: break;
        }
    }
    /* Build an uppercased echo for the diagnostic (bounded). */
    for (i = 0; i < len && i < sizeof up - 1; i++) {
        up[i] = upper_c(p[i]);
    }
    up[i] = '\0';
    set_err(errbuf, errlen,
            "Invalid 8008 register: \"%s\". Valid registers: A, B, C, D, E, H, "
            "L, M",
            up);
    return INTEL8008_ERR;
}

/* ── Opcode tables ──────────────────────────────────────────────────────────*/
static int fixed_opcode(const char *m) {
    if (str_eq(m, "RLC")) return 0x02;
    if (str_eq(m, "RRC")) return 0x0A;
    if (str_eq(m, "RAL")) return 0x12;
    if (str_eq(m, "RAR")) return 0x1A;
    if (str_eq(m, "RFC") || str_eq(m, "RET")) return 0x03;
    if (str_eq(m, "RFZ")) return 0x0B;
    if (str_eq(m, "RFS")) return 0x13;
    if (str_eq(m, "RFP")) return 0x1B;
    if (str_eq(m, "RTC")) return 0x07;
    if (str_eq(m, "RTZ")) return 0x0F;
    if (str_eq(m, "RTS")) return 0x17;
    if (str_eq(m, "RTP")) return 0x1F;
    if (str_eq(m, "HLT")) return 0xFF;
    return -1;
}
static int alu_reg_base(const char *m) {
    if (str_eq(m, "ADD")) return 0x80;
    if (str_eq(m, "ADC")) return 0x88;
    if (str_eq(m, "SUB")) return 0x90;
    if (str_eq(m, "SBB")) return 0x98;
    if (str_eq(m, "ANA")) return 0xA0;
    if (str_eq(m, "XRA")) return 0xA8;
    if (str_eq(m, "ORA")) return 0xB0;
    if (str_eq(m, "CMP")) return 0xB8;
    return -1;
}
static int alu_imm_opcode(const char *m) {
    if (str_eq(m, "ADI")) return 0xC4;
    if (str_eq(m, "ACI")) return 0xCC;
    if (str_eq(m, "SUI")) return 0xD4;
    if (str_eq(m, "SBI")) return 0xDC;
    if (str_eq(m, "ANI")) return 0xE4;
    if (str_eq(m, "XRI")) return 0xEC;
    if (str_eq(m, "ORI")) return 0xF4;
    if (str_eq(m, "CPI")) return 0xFC;
    return -1;
}
static int jump_call_opcode(const char *m) {
    if (str_eq(m, "JMP")) return 0x7C;
    if (str_eq(m, "CAL")) return 0x7E;
    if (str_eq(m, "JFC")) return 0x40;
    if (str_eq(m, "JTC")) return 0x44;
    if (str_eq(m, "JFZ")) return 0x48;
    if (str_eq(m, "JTZ")) return 0x4C;
    if (str_eq(m, "JFS")) return 0x50;
    if (str_eq(m, "JTS")) return 0x54;
    if (str_eq(m, "JFP")) return 0x58;
    if (str_eq(m, "JTP")) return 0x5C;
    if (str_eq(m, "CFC")) return 0x42;
    if (str_eq(m, "CTC")) return 0x46;
    if (str_eq(m, "CFZ")) return 0x4A;
    if (str_eq(m, "CTZ")) return 0x4E;
    if (str_eq(m, "CFS")) return 0x52;
    if (str_eq(m, "CTS")) return 0x56;
    if (str_eq(m, "CFP")) return 0x5A;
    if (str_eq(m, "CTP")) return 0x5E;
    return -1;
}

Intel8008Status intel8008_instruction_size(const char *m, size_t *out,
                                           char *errbuf, size_t errlen) {
    if (str_eq(m, "RFC") || str_eq(m, "RET") || str_eq(m, "RTC") ||
        str_eq(m, "RFZ") || str_eq(m, "RTZ") || str_eq(m, "RFS") ||
        str_eq(m, "RTS") || str_eq(m, "RFP") || str_eq(m, "RTP") ||
        str_eq(m, "RLC") || str_eq(m, "RRC") || str_eq(m, "RAL") ||
        str_eq(m, "RAR") || str_eq(m, "HLT")) {
        *out = 1;
        return INTEL8008_OK;
    }
    if (alu_reg_base(m) >= 0) {
        *out = 1;
        return INTEL8008_OK;
    }
    if (str_eq(m, "MOV") || str_eq(m, "INR") || str_eq(m, "DCR") ||
        str_eq(m, "IN") || str_eq(m, "OUT") || str_eq(m, "RST")) {
        *out = 1;
        return INTEL8008_OK;
    }
    if (str_eq(m, "MVI") || alu_imm_opcode(m) >= 0) {
        *out = 2;
        return INTEL8008_OK;
    }
    if (jump_call_opcode(m) >= 0) {
        *out = 3;
        return INTEL8008_OK;
    }
    if (str_eq(m, "ORG")) {
        *out = 0;
        return INTEL8008_OK;
    }
    set_err(errbuf, errlen, "Unknown mnemonic: \"%s\"", m);
    return INTEL8008_ERR;
}

/* ── Operand resolution ─────────────────────────────────────────────────────*/

/* hi(sym)/lo(sym). Returns: 1 matched (fills *out), 0 not hi/lo, <0 error. */
static int resolve_hi_lo(const char *s, const Intel8008Symbols *symbols,
                         size_t *out, char *errbuf, size_t errlen) {
    size_t n = strlen(s), i;
    int is_hi;
    char *sym;
    size_t addr;
    char lower3[4];
    if (n < 4 || s[n - 1] != ')') {
        return 0;
    }
    for (i = 0; i < 3; i++) {
        lower3[i] = lower_c(s[i]);
    }
    lower3[3] = '\0';
    if (str_eq(lower3, "hi(")) {
        is_hi = 1;
    } else if (str_eq(lower3, "lo(")) {
        is_hi = 0;
    } else {
        return 0;
    }
    /* symbol name from the original (case-preserved) string: s[3 .. n-1) */
    sym = sdup_n(s + 3, n - 4);
    if (!sym) {
        return -1;
    }
    if (!intel8008_symbols_get(symbols, sym, &addr)) {
        set_err(errbuf, errlen, "Undefined label in \"%s\": \"%s\"", s, sym);
        free(sym);
        return -1;
    }
    free(sym);
    *out = is_hi ? ((addr >> 8) & 0x3F) : (addr & 0xFF);
    return 1;
}

/* Resolve an operand to an integer. Returns status. */
static Intel8008Status resolve_operand(const char *operand,
                                       const Intel8008Symbols *symbols,
                                       size_t pc, size_t *out, char *errbuf,
                                       size_t errlen) {
    size_t len;
    const char *s = trim(operand, &len);
    char *owned;
    int hl;
    size_t hival;
    /* need a NUL-terminated trimmed string for lookups */
    owned = sdup_n(s, len);
    if (!owned) {
        return INTEL8008_ERR_OUT_OF_MEMORY;
    }

    if (str_eq(owned, "$")) {
        *out = pc;
        free(owned);
        return INTEL8008_OK;
    }
    hl = resolve_hi_lo(owned, symbols, &hival, errbuf, errlen);
    if (hl < 0) {
        free(owned);
        return INTEL8008_ERR;
    }
    if (hl == 1) {
        *out = hival;
        free(owned);
        return INTEL8008_OK;
    }
    if ((len >= 2 && owned[0] == '0' && (owned[1] == 'x' || owned[1] == 'X')) ||
        (len >= 1 && ((owned[0] >= '0' && owned[0] <= '9') || owned[0] == '-'))) {
        Intel8008Status st = parse_number(owned, out, errbuf, errlen);
        free(owned);
        return st;
    }
    if (intel8008_symbols_get(symbols, owned, out)) {
        free(owned);
        return INTEL8008_OK;
    }
    set_err(errbuf, errlen, "Undefined label: \"%s\"", owned);
    free(owned);
    return INTEL8008_ERR;
}

static Intel8008Status check_range(const char *name, size_t value, size_t lo,
                                   size_t hi, char *errbuf, size_t errlen) {
    if (value < lo || value > hi) {
        set_err(errbuf, errlen,
                "%s value %zu (0x%zX) is out of range [%zu, %zu]", name, value,
                value, lo, hi);
        return INTEL8008_ERR;
    }
    return INTEL8008_OK;
}

static Intel8008Status expect_operands(const char *mnemonic, size_t got,
                                       size_t count, char *errbuf,
                                       size_t errlen) {
    if (got != count) {
        set_err(errbuf, errlen, "%s expects %zu operand(s), got %zu", mnemonic,
                count, got);
        return INTEL8008_ERR;
    }
    return INTEL8008_OK;
}

/* Emit helper: push a multi-byte result and report OOM. */
static Intel8008Status emit(ByteBuf *b, const uint8_t *bytes, size_t n) {
    bb_extend(b, bytes, n);
    return b->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
}

/* Core encoder writing into a ByteBuf. */
static Intel8008Status encode_into(const char *mnemonic,
                                   const char *const *operands,
                                   size_t noperands,
                                   const Intel8008Symbols *symbols, size_t pc,
                                   ByteBuf *out, char *errbuf, size_t errlen) {
    int op;
    Intel8008Status st;

    if (str_eq(mnemonic, "ORG")) {
        return INTEL8008_OK; /* emits nothing */
    }
    op = fixed_opcode(mnemonic);
    if (op >= 0) {
        st = expect_operands(mnemonic, noperands, 0, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)op);
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    if (str_eq(mnemonic, "MOV")) {
        uint8_t dst, src;
        st = expect_operands(mnemonic, noperands, 2, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = parse_register(operands[0], &dst, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = parse_register(operands[1], &src, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)(0x40 | (dst << 3) | src));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    if (str_eq(mnemonic, "MVI")) {
        uint8_t r;
        size_t d8;
        uint8_t bytes[2];
        char name[24];
        st = expect_operands(mnemonic, noperands, 2, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = parse_register(operands[0], &r, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = resolve_operand(operands[1], symbols, pc, &d8, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        snprintf(name, sizeof name, "%.4s immediate", mnemonic);
        st = check_range(name, d8, 0, 255, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bytes[0] = (uint8_t)((r << 3) | 0x06);
        bytes[1] = (uint8_t)d8;
        return emit(out, bytes, 2);
    }
    if (str_eq(mnemonic, "INR")) {
        uint8_t r;
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = parse_register(operands[0], &r, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)(r << 3));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    if (str_eq(mnemonic, "DCR")) {
        uint8_t r;
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = parse_register(operands[0], &r, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)((r << 3) | 0x01));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    if (str_eq(mnemonic, "RST")) {
        size_t n;
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = resolve_operand(operands[0], symbols, pc, &n, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = check_range("RST n", n, 0, 7, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)(((uint8_t)n << 3) | 0x05));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    op = alu_reg_base(mnemonic);
    if (op >= 0) {
        uint8_t r;
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = parse_register(operands[0], &r, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)(op | r));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    op = alu_imm_opcode(mnemonic);
    if (op >= 0) {
        size_t d8;
        uint8_t bytes[2];
        char name[24];
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = resolve_operand(operands[0], symbols, pc, &d8, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        snprintf(name, sizeof name, "%.4s immediate", mnemonic);
        st = check_range(name, d8, 0, 255, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bytes[0] = (uint8_t)op;
        bytes[1] = (uint8_t)d8;
        return emit(out, bytes, 2);
    }
    if (str_eq(mnemonic, "IN")) {
        size_t p;
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = resolve_operand(operands[0], symbols, pc, &p, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = check_range("IN port", p, 0, 7, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)(0x41 | ((uint8_t)p << 3)));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    if (str_eq(mnemonic, "OUT")) {
        size_t p;
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = resolve_operand(operands[0], symbols, pc, &p, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = check_range("OUT port", p, 0, 23, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bb_push(out, (uint8_t)((uint8_t)p << 1));
        return out->oom ? INTEL8008_ERR_OUT_OF_MEMORY : INTEL8008_OK;
    }
    op = jump_call_opcode(mnemonic);
    if (op >= 0) {
        size_t addr;
        uint8_t bytes[3];
        char name[24];
        st = expect_operands(mnemonic, noperands, 1, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        st = resolve_operand(operands[0], symbols, pc, &addr, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        snprintf(name, sizeof name, "%.4s address", mnemonic);
        st = check_range(name, addr, 0, INTEL8008_MAX_ADDRESS, errbuf, errlen);
        if (st != INTEL8008_OK) return st;
        bytes[0] = (uint8_t)op;
        bytes[1] = (uint8_t)(addr & 0xFF);
        bytes[2] = (uint8_t)((addr >> 8) & 0x3F);
        return emit(out, bytes, 3);
    }
    set_err(errbuf, errlen, "Unknown mnemonic: \"%s\"", mnemonic);
    return INTEL8008_ERR;
}

Intel8008Status intel8008_encode_instruction(const char *mnemonic,
                                             const char *const *operands,
                                             size_t noperands,
                                             const Intel8008Symbols *symbols,
                                             size_t pc, uint8_t **out_bytes,
                                             size_t *out_len, char *errbuf,
                                             size_t errlen) {
    ByteBuf b = {0};
    Intel8008Status st;
    *out_bytes = NULL;
    *out_len = 0;
    st = encode_into(mnemonic, operands, noperands, symbols, pc, &b, errbuf,
                     errlen);
    if (st != INTEL8008_OK) {
        free(b.data);
        return st;
    }
    if (b.oom) {
        free(b.data);
        return INTEL8008_ERR_OUT_OF_MEMORY;
    }
    *out_bytes = b.data;
    *out_len = b.len;
    return INTEL8008_OK;
}

/* ── Line lexer ─────────────────────────────────────────────────────────────*/
typedef struct {
    int has_label;
    char *label;
    int has_mnemonic;
    char *mnemonic;
    char **operands;
    size_t nops;
} ParsedLine;

static void free_parsed_line(ParsedLine *l) {
    size_t i;
    free(l->label);
    free(l->mnemonic);
    for (i = 0; i < l->nops; i++) {
        free(l->operands[i]);
    }
    free(l->operands);
    l->label = l->mnemonic = NULL;
    l->operands = NULL;
    l->nops = 0;
}

/* Lex one line into *out. Returns status (OOM only). */
static Intel8008Status lex_line(const char *source, ParsedLine *out) {
    const char *semi, *text_end, *b, *e;
    size_t stripped_len, ident_end, i;
    const char *stripped, *after, *rest;
    size_t rest_len;

    memset(out, 0, sizeof *out);

    /* Step 1: strip comment (from first ';'). */
    semi = strchr(source, ';');
    text_end = semi ? semi : source + strlen(source);
    /* Step 2: trim_end. */
    while (text_end > source && isspace((unsigned char)text_end[-1])) {
        text_end--;
    }
    /* Step 3: trim_start. */
    b = source;
    while (b < text_end && isspace((unsigned char)*b)) {
        b++;
    }
    stripped = b;
    stripped_len = (size_t)(text_end - b);

    /* Step 4: label prefix (ident followed immediately by ':'). */
    after = stripped;
    if (stripped_len > 0) {
        char first = stripped[0];
        if (isalpha((unsigned char)first) || first == '_') {
            ident_end = 0;
            while (ident_end < stripped_len) {
                char c = stripped[ident_end];
                if (!isalnum((unsigned char)c) && c != '_') {
                    break;
                }
                ident_end++;
            }
            if (ident_end < stripped_len && stripped[ident_end] == ':') {
                out->label = sdup_n(stripped, ident_end);
                if (!out->label) {
                    return INTEL8008_ERR_OUT_OF_MEMORY;
                }
                out->has_label = 1;
                after = stripped + ident_end + 1;
            }
        }
    }

    /* rest = after, trim_start. */
    rest = after;
    e = stripped + stripped_len;
    while (rest < e && isspace((unsigned char)*rest)) {
        rest++;
    }
    rest_len = (size_t)(e - rest);
    if (rest_len == 0) {
        return INTEL8008_OK; /* label-only or blank */
    }

    /* Step 5: mnemonic = first whitespace-delimited token, uppercased. */
    {
        size_t mlen = 0;
        while (mlen < rest_len && !isspace((unsigned char)rest[mlen])) {
            mlen++;
        }
        out->mnemonic = to_upper_dup(rest, mlen);
        if (!out->mnemonic) {
            return INTEL8008_ERR_OUT_OF_MEMORY;
        }
        out->has_mnemonic = 1;

        /* Step 6: operands = remainder split on ',', each trimmed, empties
         * dropped. */
        {
            const char *ot = rest + mlen;
            const char *ot_end = rest + rest_len;
            const char *seg = ot;
            size_t cap = 0;
            /* skip nothing — splitting handles leading spaces via per-op trim */
            for (i = 0;; i++) {
                if (ot + i == ot_end || ot[i] == ',') {
                    size_t seglen = (size_t)((ot + i) - seg);
                    char *piece;
                    /* trim segment [seg, seg+seglen) */
                    {
                        const char *sb = seg;
                        const char *se = seg + seglen;
                        while (sb < se && isspace((unsigned char)*sb)) sb++;
                        while (se > sb && isspace((unsigned char)se[-1])) se--;
                        if (se > sb) {
                            piece = sdup_n(sb, (size_t)(se - sb));
                            if (!piece) {
                                return INTEL8008_ERR_OUT_OF_MEMORY;
                            }
                            if (!grow((void **)&out->operands, &cap,
                                      out->nops + 1, sizeof(char *))) {
                                free(piece);
                                return INTEL8008_ERR_OUT_OF_MEMORY;
                            }
                            out->operands[out->nops++] = piece;
                        }
                    }
                    if (ot + i == ot_end) {
                        break;
                    }
                    seg = ot + i + 1;
                }
            }
        }
    }
    return INTEL8008_OK;
}

/* Lex every line (Rust str::lines semantics), filling the out-params. */
static Intel8008Status lex_program(const char *text, ParsedLine **out_lines,
                                   size_t *out_n) {
    ParsedLine *lines = NULL;
    size_t cap = 0, n = 0, start = 0, i, tlen = strlen(text);
    *out_lines = NULL;
    *out_n = 0;
    for (i = 0;; i++) {
        if (i == tlen || text[i] == '\n') {
            size_t seglen = i - start;
            /* strip trailing '\r' */
            if (seglen > 0 && text[start + seglen - 1] == '\r') {
                seglen--;
            }
            /* Drop the trailing empty segment produced by a final '\n'. */
            if (!(i == tlen && start == i)) {
                char *src = sdup_n(text + start, seglen);
                ParsedLine pl;
                Intel8008Status st;
                if (!src) {
                    goto oom;
                }
                st = lex_line(src, &pl);
                free(src);
                if (st != INTEL8008_OK) {
                    free_parsed_line(&pl);
                    goto oom;
                }
                if (!grow((void **)&lines, &cap, n + 1, sizeof(ParsedLine))) {
                    free_parsed_line(&pl);
                    goto oom;
                }
                lines[n++] = pl;
            }
            if (i == tlen) {
                break;
            }
            start = i + 1;
        }
    }
    *out_lines = lines;
    *out_n = n;
    return INTEL8008_OK;
oom:
    for (i = 0; i < n; i++) {
        free_parsed_line(&lines[i]);
    }
    free(lines);
    return INTEL8008_ERR_OUT_OF_MEMORY;
}

/* ── Passes ─────────────────────────────────────────────────────────────────*/
static Intel8008Status pass1(const ParsedLine *lines, size_t n,
                             Intel8008Symbols *symbols, char *errbuf,
                             size_t errlen) {
    size_t pc = 0, i;
    for (i = 0; i < n; i++) {
        const ParsedLine *line = &lines[i];
        if (line->has_label) {
            if (!intel8008_symbols_set(symbols, line->label, pc)) {
                return INTEL8008_ERR_OUT_OF_MEMORY;
            }
        }
        if (!line->has_mnemonic) {
            continue;
        }
        if (str_eq(line->mnemonic, "ORG")) {
            size_t addr;
            Intel8008Status st;
            if (line->nops == 0) {
                set_err(errbuf, errlen, "ORG requires an address operand");
                return INTEL8008_ERR;
            }
            st = parse_number(line->operands[0], &addr, errbuf, errlen);
            if (st != INTEL8008_OK) return st;
            if (addr > INTEL8008_MAX_ADDRESS) {
                set_err(errbuf, errlen,
                        "ORG address 0x%zX exceeds Intel 8008 address space "
                        "(max 0x%zX)",
                        addr, INTEL8008_MAX_ADDRESS);
                return INTEL8008_ERR;
            }
            pc = addr;
        } else {
            size_t sz;
            Intel8008Status st =
                intel8008_instruction_size(line->mnemonic, &sz, errbuf, errlen);
            if (st != INTEL8008_OK) return st;
            pc += sz;
        }
    }
    return INTEL8008_OK;
}

static Intel8008Status pass2(const ParsedLine *lines, size_t n,
                             const Intel8008Symbols *symbols, ByteBuf *out,
                             char *errbuf, size_t errlen) {
    size_t pc = 0, i;
    for (i = 0; i < n; i++) {
        const ParsedLine *line = &lines[i];
        if (!line->has_mnemonic) {
            continue;
        }
        if (str_eq(line->mnemonic, "ORG")) {
            size_t org;
            Intel8008Status st;
            if (line->nops == 0) {
                set_err(errbuf, errlen, "ORG requires an address operand");
                return INTEL8008_ERR;
            }
            st = parse_number(line->operands[0], &org, errbuf, errlen);
            if (st != INTEL8008_OK) return st;
            if (org > INTEL8008_MAX_ADDRESS) {
                set_err(errbuf, errlen,
                        "ORG address 0x%zX exceeds Intel 8008 address space "
                        "(max 0x%zX)",
                        org, INTEL8008_MAX_ADDRESS);
                return INTEL8008_ERR;
            }
            if (org > pc) {
                bb_fill(out, 0xFF, org - pc);
                if (out->oom) return INTEL8008_ERR_OUT_OF_MEMORY;
            }
            pc = org;
        } else {
            size_t before = out->len;
            Intel8008Status st = encode_into(
                line->mnemonic, (const char *const *)line->operands, line->nops,
                symbols, pc, out, errbuf, errlen);
            if (st != INTEL8008_OK) return st;
            if (out->oom) return INTEL8008_ERR_OUT_OF_MEMORY;
            pc += out->len - before;
        }
    }
    return INTEL8008_OK;
}

Intel8008Status intel8008_assemble(const char *text, uint8_t **out_bytes,
                                   size_t *out_len, char *errbuf,
                                   size_t errlen) {
    ParsedLine *lines = NULL;
    size_t n = 0, i;
    Intel8008Symbols *symbols = NULL;
    ByteBuf out = {0};
    Intel8008Status st;

    *out_bytes = NULL;
    *out_len = 0;

    st = lex_program(text, &lines, &n);
    if (st != INTEL8008_OK) {
        return st;
    }
    symbols = intel8008_symbols_new();
    if (!symbols) {
        st = INTEL8008_ERR_OUT_OF_MEMORY;
        goto done;
    }
    st = pass1(lines, n, symbols, errbuf, errlen);
    if (st != INTEL8008_OK) {
        goto done;
    }
    st = pass2(lines, n, symbols, &out, errbuf, errlen);
    if (st != INTEL8008_OK) {
        free(out.data);
        out.data = NULL;
        goto done;
    }
    *out_bytes = out.data;
    *out_len = out.len;
    out.data = NULL;
    st = INTEL8008_OK;

done:
    intel8008_symbols_free(symbols);
    for (i = 0; i < n; i++) {
        free_parsed_line(&lines[i]);
    }
    free(lines);
    free(out.data);
    return st;
}

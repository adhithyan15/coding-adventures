/*
 * intel_4004_assembler.c — implementation of the Intel 4004 two-pass assembler
 * (see intel_4004_assembler.h). A faithful port of the Rust crate: the same
 * lexing, symbol table, ORG handling, and instruction encoding table.
 */
#include "intel_4004_assembler.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, calloc, free */
#include <string.h> /* memcpy, strlen, strcmp */

/* ---- small growable containers ---------------------------------------- */

typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
    int ok;
} ByteBuf;

static void bb_init(ByteBuf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->ok = 1;
}

static void bb_push(ByteBuf *b, uint8_t v) {
    if (!b->ok) {
        return;
    }
    if (b->len == b->cap) {
        size_t ncap = b->cap ? (b->cap > (size_t)-1 / 2 ? b->cap + 1 : b->cap * 2)
                             : 16;
        uint8_t *nd = realloc(b->data, ncap);
        if (!nd) {
            b->ok = 0;
            return;
        }
        b->data = nd;
        b->cap = ncap;
    }
    b->data[b->len++] = v;
}

/* A parsed line: label / mnemonic (uppercased) / comma-separated operands.
 * NULL label or mnemonic means "absent". */
typedef struct {
    char *label;
    char *mnemonic;
    char **operands;
    size_t nops;
} ParsedLine;

/* Symbol table entry (label -> program counter). */
typedef struct {
    char *name;
    size_t pc;
} Symbol;

/* ---- string helpers --------------------------------------------------- */

static int is_ws(char c) { return c == ' ' || c == '\t' || c == '\r'; }

/* Trim leading/trailing whitespace of s[*start,*end); adjusts the range. */
static void trim(const char *s, size_t *start, size_t *end) {
    while (*start < *end && is_ws(s[*start])) {
        (*start)++;
    }
    while (*end > *start && is_ws(s[*end - 1])) {
        (*end)--;
    }
}

/* Duplicate s[start,end) into a NUL-terminated string; NULL on OOM. */
static char *dup_range(const char *s, size_t start, size_t end) {
    size_t n = end - start;
    char *p = malloc(n + 1);
    if (!p) {
        return NULL;
    }
    memcpy(p, s + start, n);
    p[n] = '\0';
    return p;
}

static void upcase(char *s) {
    for (; *s; s++) {
        if (*s >= 'a' && *s <= 'z') {
            *s = (char)(*s - 'a' + 'A');
        }
    }
}

/* ---- lexing ----------------------------------------------------------- */

static void line_free(ParsedLine *pl) {
    size_t i;
    free(pl->label);
    free(pl->mnemonic);
    for (i = 0; i < pl->nops; i++) {
        free(pl->operands[i]);
    }
    free(pl->operands);
    pl->label = NULL;
    pl->mnemonic = NULL;
    pl->operands = NULL;
    pl->nops = 0;
}

/* Parse one raw line (already comment-stripped) into `pl`. Returns 1, or 0 on
 * OOM (pl left partially built; caller frees). */
static int lex_line(const char *raw, size_t rstart, size_t rend, ParsedLine *pl) {
    size_t s = rstart, e = rend;
    size_t rest_s, rest_e, i, k;
    size_t ops_s, ops_e;
    char **ops = NULL;
    size_t nops = 0, ops_cap = 0;

    pl->label = NULL;
    pl->mnemonic = NULL;
    pl->operands = NULL;
    pl->nops = 0;

    trim(raw, &s, &e);
    if (s == e) {
        return 1; /* blank line */
    }

    rest_s = s;
    rest_e = e;
    /* Label: a `prefix:` where the prefix has no space/tab. */
    for (i = s; i < e; i++) {
        if (raw[i] == ':') {
            size_t ps = s, pe = i;
            size_t j;
            int has_ws = 0;
            trim(raw, &ps, &pe);
            for (j = ps; j < pe; j++) {
                if (raw[j] == ' ' || raw[j] == '\t') {
                    has_ws = 1;
                    break;
                }
            }
            if (!has_ws) {
                pl->label = dup_range(raw, ps, pe);
                if (!pl->label) {
                    return 0;
                }
                rest_s = i + 1;
                rest_e = e;
                trim(raw, &rest_s, &rest_e);
            }
            break;
        }
    }

    if (rest_s == rest_e) {
        return 1; /* label only */
    }

    /* Mnemonic = first whitespace-delimited token, uppercased. */
    k = rest_s;
    while (k < rest_e && !is_ws(raw[k])) {
        k++;
    }
    pl->mnemonic = dup_range(raw, rest_s, k);
    if (!pl->mnemonic) {
        return 0;
    }
    upcase(pl->mnemonic);

    /* Operands = trim(rest after the mnemonic), split by ','. */
    ops_s = k;
    ops_e = rest_e;
    trim(raw, &ops_s, &ops_e);
    if (ops_s < ops_e) {
        size_t field_s = ops_s;
        for (i = ops_s; i <= ops_e; i++) {
            if (i == ops_e || raw[i] == ',') {
                size_t fs = field_s, fe = i;
                trim(raw, &fs, &fe);
                if (fs < fe) {
                    if (nops == ops_cap) {
                        size_t nc = ops_cap ? ops_cap * 2 : 4;
                        char **no = realloc(ops, nc * sizeof *no);
                        if (!no) {
                            pl->operands = ops;
                            pl->nops = nops;
                            return 0;
                        }
                        ops = no;
                        ops_cap = nc;
                    }
                    ops[nops] = dup_range(raw, fs, fe);
                    if (!ops[nops]) {
                        pl->operands = ops;
                        pl->nops = nops;
                        return 0;
                    }
                    nops++;
                }
                field_s = i + 1;
            }
        }
    }
    pl->operands = ops;
    pl->nops = nops;
    return 1;
}

/* ---- number / register / symbol parsing ------------------------------- */

static void set_err(char *err, size_t err_len, const char *msg,
                    const char *arg) {
    if (err && err_len > 0) {
        if (arg) {
            snprintf(err, err_len, "%s'%s'", msg, arg);
        } else {
            snprintf(err, err_len, "%s", msg);
        }
    }
}

/* Parse a decimal or 0x-hex number into a u16 (0..65535). 1 ok, 0 error. */
static int parse_number(const char *text, uint16_t *out, char *err,
                        size_t err_len) {
    unsigned long v = 0;
    const char *p = text;
    int hex = 0;
    if (p[0] == '0' && p[1] == 'x') {
        hex = 1;
        p += 2;
    }
    if (*p == '\0') {
        set_err(err, err_len, "Invalid number: ", text);
        return 0;
    }
    for (; *p; p++) {
        unsigned d;
        char c = *p;
        if (c >= '0' && c <= '9') {
            d = (unsigned)(c - '0');
        } else if (hex && c >= 'a' && c <= 'f') {
            d = (unsigned)(c - 'a') + 10;
        } else if (hex && c >= 'A' && c <= 'F') {
            d = (unsigned)(c - 'A') + 10;
        } else {
            set_err(err, err_len, "Invalid number: ", text);
            return 0;
        }
        v = v * (hex ? 16u : 10u) + d;
        if (v > 0xFFFF) {
            set_err(err, err_len, "Invalid number: ", text);
            return 0;
        }
    }
    *out = (uint16_t)v;
    return 1;
}

/* Parse `Rn` -> n (u8). Leading 'R's are stripped, then decimal 0..255. */
static int parse_u8_prefixed(const char *text, char prefix, uint8_t *out,
                             const char *kind, char *err, size_t err_len) {
    const char *p = text;
    unsigned long v = 0;
    while (is_ws(*p)) {
        p++;
    }
    while (*p == prefix) {
        p++;
    }
    if (*p == '\0') {
        set_err(err, err_len, kind, text);
        return 0;
    }
    for (; *p && !is_ws(*p); p++) {
        if (*p < '0' || *p > '9') {
            set_err(err, err_len, kind, text);
            return 0;
        }
        v = v * 10 + (unsigned)(*p - '0');
        if (v > 0xFF) {
            set_err(err, err_len, kind, text);
            return 0;
        }
    }
    *out = (uint8_t)v;
    return 1;
}

static int parse_register(const char *t, uint8_t *out, char *err, size_t el) {
    return parse_u8_prefixed(t, 'R', out, "Invalid register: ", err, el);
}
static int parse_pair(const char *t, uint8_t *out, char *err, size_t el) {
    return parse_u8_prefixed(t, 'P', out, "Invalid register pair: ", err, el);
}

static const Symbol *sym_find(const Symbol *syms, size_t n, const char *name) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (strcmp(syms[i].name, name) == 0) {
            return &syms[i];
        }
    }
    return NULL;
}

/* Resolve an operand to a u16: a number, else a symbol lookup. */
static int resolve_operand(const char *text, const Symbol *syms, size_t nsyms,
                           uint16_t *out, char *err, size_t err_len) {
    if (parse_number(text, out, NULL, 0)) {
        return 1;
    }
    {
        const Symbol *s = sym_find(syms, nsyms, text);
        if (s) {
            *out = (uint16_t)s->pc;
            return 1;
        }
    }
    set_err(err, err_len, "Unknown symbol: ", text);
    return 0;
}

/* ---- instruction sizes / encoding ------------------------------------- */

static int instruction_size(const char *m, size_t *out, char *err,
                            size_t err_len) {
    if (!strcmp(m, "NOP") || !strcmp(m, "HLT") || !strcmp(m, "WRM") ||
        !strcmp(m, "LDM") || !strcmp(m, "BBL") || !strcmp(m, "INC") ||
        !strcmp(m, "ADD") || !strcmp(m, "SUB") || !strcmp(m, "LD") ||
        !strcmp(m, "XCH") || !strcmp(m, "SRC") || !strcmp(m, "FIN") ||
        !strcmp(m, "JIN")) {
        *out = 1;
        return 1;
    }
    if (!strcmp(m, "JCN") || !strcmp(m, "FIM") || !strcmp(m, "JUN") ||
        !strcmp(m, "JMS") || !strcmp(m, "ISZ") || !strcmp(m, "ADD_IMM")) {
        *out = 2;
        return 1;
    }
    set_err(err, err_len, "Unknown mnemonic: ", m);
    return 0;
}

static int need_operands(const ParsedLine *pl, size_t want, char *err,
                        size_t err_len) {
    if (pl->nops != want) {
        if (err && err_len > 0) {
            snprintf(err, err_len, "%s expects %u operand(s), got %u",
                     pl->mnemonic, (unsigned)want, (unsigned)pl->nops);
        }
        return 0;
    }
    return 1;
}

/* Encode one instruction into `bytes` (capacity 2); *nbytes set. 1 ok / 0 err. */
static int encode_instruction(const ParsedLine *pl, const Symbol *syms,
                              size_t nsyms, uint8_t bytes[2], size_t *nbytes,
                              char *err, size_t err_len) {
    const char *m = pl->mnemonic;
    uint16_t v;
    uint8_t r;

#define ONE()                                          \
    do {                                               \
        if (!need_operands(pl, 1, err, err_len))       \
            return 0;                                  \
    } while (0)

    if (!strcmp(m, "NOP")) { bytes[0] = 0x00; *nbytes = 1; return 1; }
    if (!strcmp(m, "HLT")) { bytes[0] = 0x01; *nbytes = 1; return 1; }
    if (!strcmp(m, "WRM")) { bytes[0] = 0xE0; *nbytes = 1; return 1; }
    if (!strcmp(m, "LDM")) {
        ONE();
        if (!resolve_operand(pl->operands[0], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0xD0 | ((uint8_t)v & 0xF)); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "BBL")) {
        ONE();
        if (!resolve_operand(pl->operands[0], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0xC0 | ((uint8_t)v & 0xF)); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "INC")) {
        ONE(); if (!parse_register(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x60 | r); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "ADD")) {
        ONE(); if (!parse_register(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x80 | r); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "SUB")) {
        ONE(); if (!parse_register(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x90 | r); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "LD")) {
        ONE(); if (!parse_register(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0xA0 | r); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "XCH")) {
        ONE(); if (!parse_register(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0xB0 | r); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "SRC")) {
        ONE(); if (!parse_pair(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x20 | (2 * r + 1)); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "FIN")) {
        ONE(); if (!parse_pair(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x30 | (2 * r)); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "JIN")) {
        ONE(); if (!parse_pair(pl->operands[0], &r, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x30 | (2 * r + 1)); *nbytes = 1; return 1;
    }
    if (!strcmp(m, "FIM")) {
        if (!need_operands(pl, 2, err, err_len)) return 0;
        if (!parse_pair(pl->operands[0], &r, err, err_len)) return 0;
        if (!resolve_operand(pl->operands[1], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x20 | (2 * r)); bytes[1] = (uint8_t)v; *nbytes = 2; return 1;
    }
    if (!strcmp(m, "JCN")) {
        uint16_t v2;
        if (!need_operands(pl, 2, err, err_len)) return 0;
        if (!resolve_operand(pl->operands[0], syms, nsyms, &v, err, err_len)) return 0;
        if (!resolve_operand(pl->operands[1], syms, nsyms, &v2, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x10 | ((uint8_t)v & 0xF));
        bytes[1] = (uint8_t)(v2 & 0xFF); *nbytes = 2; return 1;
    }
    if (!strcmp(m, "JUN")) {
        ONE();
        if (!resolve_operand(pl->operands[0], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x40 | ((v >> 8) & 0xF));
        bytes[1] = (uint8_t)(v & 0xFF); *nbytes = 2; return 1;
    }
    if (!strcmp(m, "JMS")) {
        ONE();
        if (!resolve_operand(pl->operands[0], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x50 | ((v >> 8) & 0xF));
        bytes[1] = (uint8_t)(v & 0xFF); *nbytes = 2; return 1;
    }
    if (!strcmp(m, "ISZ")) {
        if (!need_operands(pl, 2, err, err_len)) return 0;
        if (!parse_register(pl->operands[0], &r, err, err_len)) return 0;
        if (!resolve_operand(pl->operands[1], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0x70 | r); bytes[1] = (uint8_t)(v & 0xFF); *nbytes = 2; return 1;
    }
    if (!strcmp(m, "ADD_IMM")) {
        if (!need_operands(pl, 3, err, err_len)) return 0;
        if (!parse_register(pl->operands[1], &r, err, err_len)) return 0;
        if (!resolve_operand(pl->operands[2], syms, nsyms, &v, err, err_len)) return 0;
        bytes[0] = (uint8_t)(0xD0 | ((uint8_t)v & 0xF));
        bytes[1] = (uint8_t)(0x80 | r); *nbytes = 2; return 1;
    }
#undef ONE
    set_err(err, err_len, "Unknown mnemonic: ", m);
    return 0;
}

/* ---- top-level assemble ----------------------------------------------- */

I4004Status i4004_assemble(const char *text, uint8_t **out, size_t *out_len,
                           char *err, size_t err_len) {
    ParsedLine *lines = NULL;
    size_t nlines = 0, lines_cap = 0;
    Symbol *syms = NULL;
    size_t nsyms = 0, syms_cap = 0;
    size_t i, pc;
    ByteBuf bb;
    I4004Status status = I4004_OK;
    size_t pos = 0, tlen;

    if (err && err_len) { err[0] = '\0'; }

    /* Split into lines on '\n' (a preceding '\r' is trimmed as whitespace). */
    tlen = strlen(text);
    for (;;) {
        size_t lstart = pos, lend;
        size_t sc;
        while (pos < tlen && text[pos] != '\n') {
            pos++;
        }
        lend = pos;
        /* Strip a trailing comment starting at ';'. */
        for (sc = lstart; sc < lend; sc++) {
            if (text[sc] == ';') { lend = sc; break; }
        }
        if (nlines == lines_cap) {
            size_t nc = lines_cap ? lines_cap * 2 : 16;
            ParsedLine *nl = realloc(lines, nc * sizeof *nl);
            if (!nl) { status = I4004_ALLOC_ERROR; goto cleanup; }
            lines = nl;
            lines_cap = nc;
        }
        if (!lex_line(text, lstart, lend, &lines[nlines])) {
            line_free(&lines[nlines]);
            status = I4004_ALLOC_ERROR;
            goto cleanup;
        }
        nlines++;
        if (pos >= tlen) { break; }
        pos++; /* skip the '\n' */
    }

    /* Pass 1: symbol table. */
    pc = 0;
    for (i = 0; i < nlines; i++) {
        ParsedLine *pl = &lines[i];
        if (pl->label) {
            Symbol *ex = NULL;
            size_t j;
            for (j = 0; j < nsyms; j++) {
                if (strcmp(syms[j].name, pl->label) == 0) { ex = &syms[j]; break; }
            }
            if (ex) {
                ex->pc = pc;
            } else {
                char *name;
                if (nsyms == syms_cap) {
                    size_t nc = syms_cap ? syms_cap * 2 : 16;
                    Symbol *ns = realloc(syms, nc * sizeof *ns);
                    if (!ns) { status = I4004_ALLOC_ERROR; goto cleanup; }
                    syms = ns;
                    syms_cap = nc;
                }
                name = malloc(strlen(pl->label) + 1);
                if (!name) { status = I4004_ALLOC_ERROR; goto cleanup; }
                strcpy(name, pl->label);
                syms[nsyms].name = name;
                syms[nsyms].pc = pc;
                nsyms++;
            }
        }
        if (!pl->mnemonic) { continue; }
        if (!strcmp(pl->mnemonic, "ORG")) {
            uint16_t addr;
            if (pl->nops < 1) {
                set_err(err, err_len, "ORG requires an operand", NULL);
                status = I4004_ERROR; goto cleanup;
            }
            if (!parse_number(pl->operands[0], &addr, err, err_len)) {
                status = I4004_ERROR; goto cleanup;
            }
            pc = addr;
            continue;
        }
        {
            size_t sz;
            if (!instruction_size(pl->mnemonic, &sz, err, err_len)) {
                status = I4004_ERROR; goto cleanup;
            }
            pc += sz;
        }
    }

    /* Pass 2: encode. */
    bb_init(&bb);
    pc = 0;
    for (i = 0; i < nlines; i++) {
        ParsedLine *pl = &lines[i];
        if (!pl->mnemonic) { continue; }
        if (!strcmp(pl->mnemonic, "ORG")) {
            uint16_t addr;
            if (pl->nops < 1) {
                set_err(err, err_len, "ORG requires an operand", NULL);
                free(bb.data); status = I4004_ERROR; goto cleanup;
            }
            if (!parse_number(pl->operands[0], &addr, err, err_len)) {
                free(bb.data); status = I4004_ERROR; goto cleanup;
            }
            while (pc < addr) { bb_push(&bb, 0); pc++; }
            continue;
        }
        {
            uint8_t bytes[2];
            size_t nb;
            if (!encode_instruction(pl, syms, nsyms, bytes, &nb, err, err_len)) {
                free(bb.data); status = I4004_ERROR; goto cleanup;
            }
            {
                size_t k;
                for (k = 0; k < nb; k++) { bb_push(&bb, bytes[k]); }
            }
            pc += nb;
        }
    }
    if (!bb.ok) { free(bb.data); status = I4004_ALLOC_ERROR; goto cleanup; }
    *out = bb.data;
    *out_len = bb.len;
    status = I4004_OK;

cleanup:
    for (i = 0; i < nlines; i++) { line_free(&lines[i]); }
    free(lines);
    for (i = 0; i < nsyms; i++) { free(syms[i].name); }
    free(syms);
    return status;
}

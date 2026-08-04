/*
 * ge225_simulator.c — implementation of the pure-ISO C GE-225 simulator.
 * =====================================================================
 *
 * See ge225_simulator.h. Faithful to the Rust crate. Two portability rules keep
 * it UBSan-clean while matching Rust's wrapping-shift semantics:
 *   - LEFT shifts and all double-word bit-shuffling use unsigned types, so a
 *     shift that would exceed the signed range wraps (defined) instead of
 *     overflowing (UB). Rust's `i32`/`i64 <<` do the same bit-wrapping.
 *   - Signed RIGHT shifts of possibly-negative values stay signed; C makes that
 *     implementation-defined (arithmetic on every target), matching Rust `>>`.
 */
#include "ge225_simulator.h"

#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy, memset, strcmp, strlen */

#define MASK_20 ((int32_t)((1 << 20) - 1))
#define DATA_MASK ((int32_t)((1 << 19) - 1))
#define SIGN_BIT ((int32_t)(1 << 19))
#define ADDR_MASK ((int32_t)0x1fff)
#define X_MASK ((int32_t)0x7fff)
#define N_MASK ((int32_t)0x3f)
#define WORD_BYTES 3
#define MAX_X_GROUPS 32

/* ── Mnemonics (an enum used for decode-then-dispatch) ─────────────────────*/

typedef enum {
    M_NONE = 0,
    /* memory-reference, opcodes 0o00..0o27 */
    M_LDA, M_ADD, M_SUB, M_STA, M_BXL, M_BXH, M_LDX, M_SPB, M_DLD, M_DAD,
    M_DSU, M_DST, M_INX, M_MPY, M_DVD, M_STX, M_EXT, M_CAB, M_DCB, M_ORY,
    M_MOY, M_RCD, M_BRU, M_STO,
    /* fixed */
    M_OFF, M_TYP, M_TON, M_RCS, M_HPT, M_LDZ, M_LDO, M_LMO, M_CPL, M_NEG,
    M_CHS, M_NOP, M_LAQ, M_LQA, M_XAQ, M_MAQ, M_ADO, M_SBO, M_SET_DECMODE,
    M_SET_BINMODE, M_SXG, M_SET_PST, M_SET_PBK, M_BOD, M_BEV, M_BMI, M_BPL,
    M_BZE, M_BNZ, M_BOV, M_BNO, M_BPE, M_BPC, M_BNR, M_BNN,
    /* shift */
    M_SRA, M_SNA, M_SCA, M_SAN, M_SRD, M_NAQ, M_SCD, M_ANQ, M_SLA, M_SLD,
    M_NOR, M_DNO
} Mnemonic;

/* memory-reference opcode (0o00..) → mnemonic. */
static const Mnemonic MEMREF_BY_OPCODE[24] = {
    M_LDA, M_ADD, M_SUB, M_STA, M_BXL, M_BXH, M_LDX, M_SPB,
    M_DLD, M_DAD, M_DSU, M_DST, M_INX, M_MPY, M_DVD, M_STX,
    M_EXT, M_CAB, M_DCB, M_ORY, M_MOY, M_RCD, M_BRU, M_STO};

typedef struct {
    Mnemonic mnem;
    const char *name;
    int32_t word;
} FixedEntry;

/* The fixed-instruction words, as octal literals (matching the Rust `0o…`). */
static const FixedEntry FIXED[] = {
    {M_OFF, "OFF", 02500005},          {M_TYP, "TYP", 02500006},
    {M_TON, "TON", 02500007},          {M_RCS, "RCS", 02500011},
    {M_HPT, "HPT", 02500016},          {M_LDZ, "LDZ", 02504002},
    {M_LDO, "LDO", 02504022},          {M_LMO, "LMO", 02504102},
    {M_CPL, "CPL", 02504502},          {M_NEG, "NEG", 02504522},
    {M_CHS, "CHS", 02504040},          {M_NOP, "NOP", 02504012},
    {M_LAQ, "LAQ", 02504001},          {M_LQA, "LQA", 02504004},
    {M_XAQ, "XAQ", 02504005},          {M_MAQ, "MAQ", 02504006},
    {M_ADO, "ADO", 02504032},          {M_SBO, "SBO", 02504112},
    {M_SET_DECMODE, "SET_DECMODE", 02506011},
    {M_SET_BINMODE, "SET_BINMODE", 02506012},
    {M_SXG, "SXG", 02506013},          {M_SET_PST, "SET_PST", 02506015},
    {M_SET_PBK, "SET_PBK", 02506016},  {M_BOD, "BOD", 02514000},
    {M_BEV, "BEV", 02516000},          {M_BMI, "BMI", 02514001},
    {M_BPL, "BPL", 02516001},          {M_BZE, "BZE", 02514002},
    {M_BNZ, "BNZ", 02516002},          {M_BOV, "BOV", 02514003},
    {M_BNO, "BNO", 02516003},          {M_BPE, "BPE", 02514004},
    {M_BPC, "BPC", 02516004},          {M_BNR, "BNR", 02514005},
    {M_BNN, "BNN", 02516005}};
static const size_t FIXED_N = sizeof FIXED / sizeof FIXED[0];

typedef struct {
    Mnemonic mnem;
    const char *name;
    int32_t base;
} ShiftEntry;

static const ShiftEntry SHIFTS[] = {
    {M_SRA, "SRA", 02510000}, {M_SNA, "SNA", 02510100},
    {M_SCA, "SCA", 02510040}, {M_SAN, "SAN", 02510400},
    {M_SRD, "SRD", 02511000}, {M_NAQ, "NAQ", 02511100},
    {M_SCD, "SCD", 02511200}, {M_ANQ, "ANQ", 02511400},
    {M_SLA, "SLA", 02512000}, {M_SLD, "SLD", 02512200},
    {M_NOR, "NOR", 02513000}, {M_DNO, "DNO", 02513200}};
static const size_t SHIFTS_N = sizeof SHIFTS / sizeof SHIFTS[0];

static const char *mnemonic_name(Mnemonic m) {
    size_t i;
    switch (m) {
        case M_LDA: return "LDA"; case M_ADD: return "ADD";
        case M_SUB: return "SUB"; case M_STA: return "STA";
        case M_BXL: return "BXL"; case M_BXH: return "BXH";
        case M_LDX: return "LDX"; case M_SPB: return "SPB";
        case M_DLD: return "DLD"; case M_DAD: return "DAD";
        case M_DSU: return "DSU"; case M_DST: return "DST";
        case M_INX: return "INX"; case M_MPY: return "MPY";
        case M_DVD: return "DVD"; case M_STX: return "STX";
        case M_EXT: return "EXT"; case M_CAB: return "CAB";
        case M_DCB: return "DCB"; case M_ORY: return "ORY";
        case M_MOY: return "MOY"; case M_RCD: return "RCD";
        case M_BRU: return "BRU"; case M_STO: return "STO";
        default: break;
    }
    for (i = 0; i < FIXED_N; i++)
        if (FIXED[i].mnem == m) return FIXED[i].name;
    for (i = 0; i < SHIFTS_N; i++)
        if (SHIFTS[i].mnem == m) return SHIFTS[i].name;
    return "?";
}

/* ── Arithmetic helpers (UB-safe sign extension) ───────────────────────────*/

static int32_t to_signed20(int32_t value) {
    int32_t word = value & MASK_20; /* 0..2^20-1 */
    return (word & SIGN_BIT) ? word - (1 << 20) : word;
}
static int32_t from_signed20(int32_t value) { return value & MASK_20; }
static int32_t sign_of(int32_t word) { return (word & SIGN_BIT) ? 1 : 0; }
static int32_t with_sign(int32_t word, int32_t sign) {
    return ((sign & 1) << 19) | (word & DATA_MASK);
}
static int64_t combine_words(int32_t high, int32_t low) {
    return ((int64_t)(high & MASK_20) << 20) | (int64_t)(low & MASK_20);
}
static int64_t to_signed40(int64_t value) {
    uint64_t raw = (uint64_t)value & ((UINT64_C(1) << 40) - 1);
    if (raw & (UINT64_C(1) << 39)) raw |= ~((UINT64_C(1) << 40) - 1);
    return (int64_t)raw;
}
static void split_signed40(int64_t value, int32_t *high, int32_t *low) {
    uint64_t raw = (uint64_t)value & ((UINT64_C(1) << 40) - 1);
    *high = (int32_t)((raw >> 20) & (uint64_t)(uint32_t)MASK_20);
    *low = (int32_t)(raw & (uint64_t)(uint32_t)MASK_20);
}
static int32_t arith_compare(int32_t left, int32_t right) {
    int32_t l = to_signed20(left), r = to_signed20(right);
    return l < r ? -1 : (l > r ? 1 : 0);
}
static int32_t arith_compare_double(int32_t lh, int32_t ll, int32_t rh,
                                    int32_t rl) {
    int64_t l = to_signed40(combine_words(lh, ll));
    int64_t r = to_signed40(combine_words(rh, rl));
    return l < r ? -1 : (l > r ? 1 : 0);
}

/* ── Free functions: encode / decode / assemble / pack ─────────────────────*/

GeStatus ge225_encode_instruction(int32_t opcode, int32_t modifier,
                                  int32_t address, int32_t *out_word) {
    if (opcode < 0 || opcode > 037) return GE_ERR_RANGE;
    if (modifier < 0 || modifier > 03) return GE_ERR_RANGE;
    if (address < 0 || address > ADDR_MASK) return GE_ERR_RANGE;
    *out_word = ((opcode & 0x1f) << 15) | ((modifier & 0x03) << 13) |
                (address & ADDR_MASK);
    return GE_OK;
}
void ge225_decode_instruction(int32_t word, int32_t *out_opcode,
                              int32_t *out_modifier, int32_t *out_address) {
    int32_t normalized = word & MASK_20;
    *out_opcode = (normalized >> 15) & 0x1f;
    *out_modifier = (normalized >> 13) & 0x03;
    *out_address = normalized & ADDR_MASK;
}
GeStatus ge225_assemble_fixed(const char *mnemonic, int32_t *out_word) {
    size_t i;
    for (i = 0; i < FIXED_N; i++)
        if (strcmp(FIXED[i].name, mnemonic) == 0) {
            *out_word = FIXED[i].word;
            return GE_OK;
        }
    return GE_ERR_UNKNOWN_MNEMONIC;
}
GeStatus ge225_assemble_shift(const char *mnemonic, int32_t count,
                              int32_t *out_word) {
    size_t i;
    if (count < 0 || count > 037) return GE_ERR_RANGE;
    for (i = 0; i < SHIFTS_N; i++)
        if (strcmp(SHIFTS[i].name, mnemonic) == 0) {
            *out_word = SHIFTS[i].base | count;
            return GE_OK;
        }
    return GE_ERR_UNKNOWN_MNEMONIC;
}

uint8_t *ge225_pack_words(const int32_t *words, size_t n, size_t *out_len) {
    uint8_t *blob;
    size_t i;
    *out_len = 0;
    if (n > ((size_t)-1) / WORD_BYTES) return NULL;
    blob = (uint8_t *)calloc(n ? n * WORD_BYTES : 1, 1);
    if (blob == NULL) return NULL;
    for (i = 0; i < n; i++) {
        int32_t normalized = words[i] & MASK_20;
        blob[i * WORD_BYTES] = (uint8_t)((normalized >> 16) & 0xff);
        blob[i * WORD_BYTES + 1] = (uint8_t)((normalized >> 8) & 0xff);
        blob[i * WORD_BYTES + 2] = (uint8_t)(normalized & 0xff);
    }
    *out_len = n * WORD_BYTES;
    return blob;
}
GeStatus ge225_unpack_words(const uint8_t *program, size_t len,
                            int32_t **out_words, size_t *out_n) {
    size_t n, i;
    int32_t *words;
    *out_words = NULL;
    *out_n = 0;
    if (len % WORD_BYTES != 0) return GE_ERR_ODD_BYTE_LENGTH;
    n = len / WORD_BYTES;
    words = (int32_t *)calloc(n ? n : 1, sizeof(int32_t));
    if (words == NULL) return GE_ERR_OUT_OF_MEMORY;
    for (i = 0; i < n; i++) {
        const uint8_t *c = program + i * WORD_BYTES;
        words[i] = (((int32_t)c[0] << 16) | ((int32_t)c[1] << 8) |
                    (int32_t)c[2]) &
                   MASK_20;
    }
    *out_words = words;
    *out_n = n;
    return GE_OK;
}

/* ── The simulator ─────────────────────────────────────────────────────────*/

typedef struct {
    int32_t *words; /* n_words each; a queue of card records */
    size_t n_words;
} CardRecord;

struct Ge225Simulator {
    int32_t memory_size;
    int32_t *memory;
    CardRecord *card_queue;
    size_t card_queue_len;
    size_t card_queue_cap;
    int32_t a, q, m, n, pc, ir;
    int overflow, parity_error, decimal_mode, automatic_interrupt_mode;
    size_t selected_x_group;
    int n_ready, typewriter_power;
    char *typewriter_output; /* growable NUL-terminated string */
    size_t tw_len, tw_cap;
    int oom; /* latched typewriter/output allocation failure */
    int32_t control_switches;
    int halted;
    int32_t x_groups[MAX_X_GROUPS][4];
};

static void tw_append(Ge225Simulator *s, const char *str) {
    size_t add = strlen(str);
    if (s->tw_len + add + 1 > s->tw_cap) {
        size_t nc = s->tw_cap ? s->tw_cap : 32;
        char *nb;
        while (nc < s->tw_len + add + 1) {
            if (nc > ((size_t)-1) / 2) {
                s->oom = 1;
                return;
            }
            nc *= 2;
        }
        nb = (char *)realloc(s->typewriter_output, nc);
        if (nb == NULL) {
            s->oom = 1;
            return;
        }
        s->typewriter_output = nb;
        s->tw_cap = nc;
    }
    memcpy(s->typewriter_output + s->tw_len, str, add + 1);
    s->tw_len += add;
}

Ge225Simulator *ge225_new(int32_t memory_words) {
    Ge225Simulator *s;
    if (memory_words <= 0) return NULL;
    s = (Ge225Simulator *)calloc(1, sizeof(Ge225Simulator));
    if (s == NULL) return NULL;
    s->memory_size = memory_words;
    s->memory = (int32_t *)calloc((size_t)memory_words, sizeof(int32_t));
    if (s->memory == NULL) {
        free(s);
        return NULL;
    }
    s->n_ready = 1;
    return s;
}

static void card_queue_clear(Ge225Simulator *s) {
    size_t i;
    for (i = 0; i < s->card_queue_len; i++) free(s->card_queue[i].words);
    free(s->card_queue);
    s->card_queue = NULL;
    s->card_queue_len = 0;
    s->card_queue_cap = 0;
}

void ge225_free(Ge225Simulator *sim) {
    if (sim == NULL) return;
    free(sim->memory);
    card_queue_clear(sim);
    free(sim->typewriter_output);
    free(sim);
}

void ge225_reset(Ge225Simulator *sim) {
    sim->a = sim->q = sim->m = sim->n = sim->pc = sim->ir = 0;
    sim->overflow = sim->parity_error = sim->decimal_mode = 0;
    sim->automatic_interrupt_mode = 0;
    sim->selected_x_group = 0;
    sim->n_ready = 1;
    sim->typewriter_power = 0;
    sim->tw_len = 0;
    if (sim->typewriter_output) sim->typewriter_output[0] = '\0';
    sim->control_switches = 0;
    sim->halted = 0;
    memset(sim->x_groups, 0, sizeof sim->x_groups);
}

void ge225_set_control_switches(Ge225Simulator *sim, int32_t value) {
    sim->control_switches = value & MASK_20;
}
GeStatus ge225_queue_card_reader_record(Ge225Simulator *sim,
                                        const int32_t *words, size_t n) {
    int32_t *copy = NULL;
    size_t i;
    if (sim->card_queue_len == sim->card_queue_cap) {
        size_t nc = sim->card_queue_cap ? sim->card_queue_cap * 2 : 4;
        CardRecord *nq =
            (CardRecord *)realloc(sim->card_queue, nc * sizeof(CardRecord));
        if (nq == NULL) return GE_ERR_OUT_OF_MEMORY;
        sim->card_queue = nq;
        sim->card_queue_cap = nc;
    }
    if (n > 0) {
        copy = (int32_t *)calloc(n, sizeof(int32_t));
        if (copy == NULL) return GE_ERR_OUT_OF_MEMORY;
        for (i = 0; i < n; i++) copy[i] = words[i] & MASK_20;
    }
    sim->card_queue[sim->card_queue_len].words = copy;
    sim->card_queue[sim->card_queue_len].n_words = n;
    sim->card_queue_len++;
    return GE_OK;
}
size_t ge225_typewriter_output(const Ge225Simulator *sim, char *buf,
                               size_t cap) {
    size_t len = sim->tw_len;
    if (buf != NULL && cap > 0) {
        size_t copy = len < cap - 1 ? len : cap - 1;
        if (sim->typewriter_output) memcpy(buf, sim->typewriter_output, copy);
        buf[copy] = '\0';
    }
    return len;
}

static GeStatus check_address(const Ge225Simulator *s, int32_t address) {
    return (address < 0 || address >= s->memory_size)
               ? GE_ERR_ADDRESS_OUT_OF_RANGE
               : GE_OK;
}
GeStatus ge225_read_word(const Ge225Simulator *sim, int32_t address,
                         int32_t *out_value) {
    GeStatus st = check_address(sim, address);
    if (st != GE_OK) return st;
    *out_value = sim->memory[address];
    return GE_OK;
}
GeStatus ge225_write_word(Ge225Simulator *sim, int32_t address, int32_t value) {
    GeStatus st = check_address(sim, address);
    if (st != GE_OK) return st;
    sim->memory[address] = value & MASK_20;
    return GE_OK;
}
GeStatus ge225_load_words(Ge225Simulator *sim, const int32_t *words, size_t n,
                          int32_t start_address) {
    size_t i;
    for (i = 0; i < n; i++) {
        /* Compute in int64 so a large start_address/count can't overflow the
         * int32 address before check_address sees it. */
        int64_t addr = (int64_t)start_address + (int64_t)i;
        GeStatus st;
        if (addr < 0 || addr >= sim->memory_size)
            return GE_ERR_ADDRESS_OUT_OF_RANGE;
        st = ge225_write_word(sim, (int32_t)addr, words[i]);
        if (st != GE_OK) return st;
    }
    return GE_OK;
}

static int32_t get_x_word(const Ge225Simulator *s, size_t slot) {
    return s->x_groups[s->selected_x_group][slot] & X_MASK;
}
static void set_x_word(Ge225Simulator *s, size_t slot, int32_t value) {
    s->x_groups[s->selected_x_group][slot] = value & X_MASK;
}

/* ── Decode ────────────────────────────────────────────────────────────────*/

typedef struct {
    Mnemonic mnem;
    int32_t modifier;
    int32_t address;
    int32_t count;
    int fixed_word;
} Decoded;

static GeStatus decode_word(int32_t word, Decoded *out) {
    int32_t normalized = word & MASK_20;
    int32_t opcode, modifier, address;
    size_t i;
    memset(out, 0, sizeof *out);
    for (i = 0; i < FIXED_N; i++)
        if (FIXED[i].word == normalized) {
            out->mnem = FIXED[i].mnem;
            out->fixed_word = 1;
            return GE_OK;
        }
    for (i = 0; i < SHIFTS_N; i++)
        if ((normalized & ~(int32_t)037) == SHIFTS[i].base) {
            out->mnem = SHIFTS[i].mnem;
            out->fixed_word = 1;
            out->count = normalized & 037;
            return GE_OK;
        }
    ge225_decode_instruction(normalized, &opcode, &modifier, &address);
    if (opcode < 0 || opcode >= 24) return GE_ERR_DECODE;
    out->mnem = MEMREF_BY_OPCODE[opcode];
    out->modifier = modifier;
    out->address = address;
    out->fixed_word = 0;
    return GE_OK;
}

GeStatus ge225_disassemble_word(const Ge225Simulator *sim, int32_t word,
                                char *buf, size_t cap) {
    Decoded d;
    GeStatus st = decode_word(word, &d);
    (void)sim;
    if (st != GE_OK) return st;
    if (buf == NULL || cap == 0) return GE_OK;
    if (d.fixed_word) {
        /* shift instructions carry a count; other fixed words don't */
        int is_shift = 0;
        size_t i;
        for (i = 0; i < SHIFTS_N; i++)
            if (SHIFTS[i].mnem == d.mnem) is_shift = 1;
        if (is_shift) {
            /* "MNEM count" */
            const char *nm = mnemonic_name(d.mnem);
            char num[16];
            size_t p = 0, k;
            int32_t c = d.count;
            char tmp[16];
            size_t tn = 0;
            for (k = 0; nm[k] != '\0' && p + 1 < cap; k++) buf[p++] = nm[k];
            if (p + 1 < cap) buf[p++] = ' ';
            if (c == 0) tmp[tn++] = '0';
            while (c > 0) {
                tmp[tn++] = (char)('0' + c % 10);
                c /= 10;
            }
            for (k = 0; k < tn && p + 1 < cap; k++) num[k] = tmp[tn - 1 - k];
            for (k = 0; k < tn && p + 1 < cap; k++) buf[p++] = num[k];
            buf[p] = '\0';
        } else {
            const char *nm = mnemonic_name(d.mnem);
            size_t p = 0, k;
            for (k = 0; nm[k] != '\0' && p + 1 < cap; k++) buf[p++] = nm[k];
            buf[p] = '\0';
        }
    } else {
        /* "MNEM 0xADDR,Xmod" */
        const char *nm = mnemonic_name(d.mnem);
        static const char *hexd = "0123456789ABCDEF";
        size_t p = 0, k;
        for (k = 0; nm[k] != '\0' && p + 1 < cap; k++) buf[p++] = nm[k];
        if (p + 1 < cap) buf[p++] = ' ';
        if (p + 1 < cap) buf[p++] = '0';
        if (p + 1 < cap) buf[p++] = 'x';
        if (p + 1 < cap) buf[p++] = hexd[(d.address >> 8) & 0xf];
        if (p + 1 < cap) buf[p++] = hexd[(d.address >> 4) & 0xf];
        if (p + 1 < cap) buf[p++] = hexd[d.address & 0xf];
        if (p + 1 < cap) buf[p++] = ',';
        if (p + 1 < cap) buf[p++] = 'X';
        if (p + 1 < cap) buf[p++] = (char)('0' + (d.modifier & 3));
        buf[p] = '\0';
    }
    return GE_OK;
}

/* ── Execution ─────────────────────────────────────────────────────────────*/

static int32_t resolve_effective_address(const Ge225Simulator *s,
                                         int32_t address, int32_t modifier) {
    int32_t base = address % s->memory_size;
    if (modifier == 0) return base;
    return (base + (get_x_word(s, (size_t)modifier) % s->memory_size)) %
           s->memory_size;
}

static const char *typewriter_char(int32_t code) {
    switch (code) {
        case 000: return "0"; case 001: return "1"; case 002: return "2";
        case 003: return "3"; case 004: return "4"; case 005: return "5";
        case 006: return "6"; case 007: return "7"; case 010: return "8";
        case 011: return "9"; case 013: return "/"; case 021: return "A";
        case 022: return "B"; case 023: return "C"; case 024: return "D";
        case 025: return "E"; case 026: return "F"; case 027: return "G";
        case 030: return "H"; case 031: return "I"; case 033: return "-";
        case 040: return "."; case 041: return "J"; case 042: return "K";
        case 043: return "L"; case 044: return "M"; case 045: return "N";
        case 046: return "O"; case 047: return "P"; case 050: return "Q";
        case 051: return "R"; case 053: return "$"; case 060: return " ";
        case 062: return "S"; case 063: return "T"; case 064: return "U";
        case 065: return "V"; case 066: return "W"; case 067: return "X";
        case 070: return "Y"; case 071: return "Z";
        default: return NULL;
    }
}

/* Single-word overflow range test: out of [-2^19, 2^19-1]. */
static int ov20(int32_t total) {
    return total < -(1 << 19) || total > (1 << 19) - 1;
}
/* Double-word overflow range test: out of [-2^39, 2^39-1]. */
static int ov40(int64_t total) {
    return total < -(INT64_C(1) << 39) || total > (INT64_C(1) << 39) - 1;
}

static GeStatus execute_memory_reference(Ge225Simulator *s, Mnemonic mnem,
                                         int32_t modifier,
                                         int32_t effective_or_raw,
                                         int32_t raw_address,
                                         int32_t pc_before) {
    int32_t eff = effective_or_raw % s->memory_size;
    GeStatus st;
    switch (mnem) {
        case M_LDA:
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            s->a = s->m;
            break;
        case M_ADD: {
            int32_t total;
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            total = to_signed20(s->a) + to_signed20(s->m);
            s->a = from_signed20(total);
            s->overflow = ov20(total);
            break;
        }
        case M_SUB: {
            int32_t total;
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            total = to_signed20(s->a) - to_signed20(s->m);
            s->a = from_signed20(total);
            s->overflow = ov20(total);
            break;
        }
        case M_STA:
            if ((st = ge225_write_word(s, eff, s->a)) != GE_OK) return st;
            break;
        case M_BXL:
            if ((get_x_word(s, (size_t)modifier) & ADDR_MASK) >= raw_address)
                s->pc = (s->pc + 1) % s->memory_size;
            break;
        case M_BXH:
            if ((get_x_word(s, (size_t)modifier) & ADDR_MASK) < raw_address)
                s->pc = (s->pc + 1) % s->memory_size;
            break;
        case M_LDX: {
            int32_t word;
            if ((st = ge225_read_word(s, raw_address % s->memory_size,
                                      &word)) != GE_OK)
                return st;
            set_x_word(s, (size_t)modifier, word);
            break;
        }
        case M_SPB:
            set_x_word(s, (size_t)modifier, pc_before);
            s->pc = raw_address % s->memory_size;
            break;
        case M_DLD: {
            int32_t first;
            if ((st = ge225_read_word(s, eff, &first)) != GE_OK) return st;
            if (eff & 1) {
                s->a = first;
                s->q = first;
            } else {
                s->a = first;
                if ((st = ge225_read_word(s, (eff + 1) % s->memory_size,
                                          &s->q)) != GE_OK)
                    return st;
            }
            break;
        }
        case M_DAD:
        case M_DSU: {
            int64_t left = to_signed40(combine_words(s->a, s->q));
            int32_t first, second;
            int64_t total;
            if ((st = ge225_read_word(s, eff, &first)) != GE_OK) return st;
            if (eff & 1)
                second = first;
            else if ((st = ge225_read_word(s, (eff + 1) % s->memory_size,
                                           &second)) != GE_OK)
                return st;
            if (mnem == M_DAD)
                total = left + to_signed40(combine_words(first, second));
            else
                total = left - to_signed40(combine_words(first, second));
            split_signed40(total, &s->a, &s->q);
            s->overflow = ov40(total);
            break;
        }
        case M_DST:
            if (eff & 1) {
                if ((st = ge225_write_word(s, eff, s->q)) != GE_OK) return st;
            } else {
                if ((st = ge225_write_word(s, eff, s->a)) != GE_OK) return st;
                if ((st = ge225_write_word(s, (eff + 1) % s->memory_size,
                                           s->q)) != GE_OK)
                    return st;
            }
            break;
        case M_INX:
            set_x_word(s, (size_t)modifier,
                       (get_x_word(s, (size_t)modifier) + raw_address) & X_MASK);
            break;
        case M_MPY: {
            int64_t product;
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            product = (int64_t)to_signed20(s->q) * (int64_t)to_signed20(s->m) +
                      (int64_t)to_signed20(s->a);
            split_signed40(product, &s->a, &s->q);
            s->overflow = ov40(product);
            break;
        }
        case M_DVD: {
            int64_t divisor, dividend, qmag, rmag, quotient, remainder;
            int32_t sa;
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            divisor = (int64_t)to_signed20(s->m);
            if (divisor == 0) return GE_ERR_DIVIDE_BY_ZERO;
            sa = to_signed20(s->a);
            {
                int64_t sa_abs = sa < 0 ? -(int64_t)sa : (int64_t)sa;
                int64_t div_abs = divisor < 0 ? -divisor : divisor;
                if (sa_abs >= div_abs) {
                    s->overflow = 1;
                    return GE_OK;
                }
            }
            dividend = to_signed40(combine_words(s->a, s->q));
            {
                int64_t dvd_abs = dividend < 0 ? -dividend : dividend;
                int64_t div_abs = divisor < 0 ? -divisor : divisor;
                qmag = dvd_abs / div_abs;
                rmag = dvd_abs % div_abs;
            }
            quotient = ((dividend < 0) ^ (divisor < 0)) ? -qmag : qmag;
            remainder = quotient < 0 ? -rmag : rmag;
            s->a = from_signed20((int32_t)quotient);
            s->q = from_signed20((int32_t)remainder);
            s->overflow =
                quotient < -(INT64_C(1) << 19) || quotient > (INT64_C(1) << 19) - 1;
            break;
        }
        case M_STX:
            if ((st = ge225_write_word(s, raw_address % s->memory_size,
                                       get_x_word(s, (size_t)modifier))) !=
                GE_OK)
                return st;
            break;
        case M_EXT:
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            s->a &= (~s->m) & MASK_20;
            break;
        case M_CAB: {
            int32_t cmp;
            if ((st = ge225_read_word(s, eff, &s->m)) != GE_OK) return st;
            cmp = arith_compare(s->m, s->a);
            if (cmp == 0)
                s->pc = (s->pc + 1) % s->memory_size;
            else if (cmp < 0)
                s->pc = (s->pc + 2) % s->memory_size;
            break;
        }
        case M_DCB: {
            int32_t first, second, cmp;
            if ((st = ge225_read_word(s, eff, &first)) != GE_OK) return st;
            if (eff & 1)
                second = first;
            else if ((st = ge225_read_word(s, (eff + 1) % s->memory_size,
                                           &second)) != GE_OK)
                return st;
            cmp = arith_compare_double(first, second, s->a, s->q);
            if (cmp == 0)
                s->pc = (s->pc + 1) % s->memory_size;
            else if (cmp < 0)
                s->pc = (s->pc + 2) % s->memory_size;
            break;
        }
        case M_ORY: {
            int32_t word;
            if ((st = ge225_read_word(s, eff, &word)) != GE_OK) return st;
            if ((st = ge225_write_word(s, eff, word | s->a)) != GE_OK)
                return st;
            break;
        }
        case M_MOY: {
            int32_t sq = to_signed20(s->q);
            int32_t word_count = -sq > 0 ? -sq : 0;
            int32_t destination = s->a & X_MASK;
            int32_t offset;
            for (offset = 0; offset < word_count; offset++) {
                int32_t word;
                if ((st = ge225_read_word(s, (raw_address + offset) %
                                                 s->memory_size,
                                          &word)) != GE_OK)
                    return st;
                if ((st = ge225_write_word(
                         s, (destination + offset) % s->memory_size, word)) !=
                    GE_OK)
                    return st;
            }
            set_x_word(s, 0, s->pc);
            s->a = 0;
            break;
        }
        case M_RCD: {
            CardRecord rec;
            size_t offset;
            if (s->card_queue_len == 0) return GE_ERR_NO_CARD_RECORD;
            rec = s->card_queue[0];
            memmove(&s->card_queue[0], &s->card_queue[1],
                    (s->card_queue_len - 1) * sizeof(CardRecord));
            s->card_queue_len--;
            for (offset = 0; offset < rec.n_words; offset++) {
                if ((st = ge225_write_word(
                         s, (eff + (int32_t)offset) % s->memory_size,
                         rec.words[offset])) != GE_OK) {
                    free(rec.words);
                    return st;
                }
            }
            free(rec.words);
            break;
        }
        case M_BRU:
            s->pc = eff;
            break;
        case M_STO: {
            int32_t existing;
            if ((st = ge225_read_word(s, eff, &existing)) != GE_OK) return st;
            if ((st = ge225_write_word(
                     s, eff, (existing & ~ADDR_MASK) | (s->a & ADDR_MASK))) !=
                GE_OK)
                return st;
            break;
        }
        default:
            return GE_ERR_DECODE;
    }
    return GE_OK;
}

static void execute_branch_test(Ge225Simulator *s, Mnemonic mnem) {
    int cond = 0;
    switch (mnem) {
        case M_BOD: cond = (s->a & 1) != 0; break;
        case M_BEV: cond = (s->a & 1) == 0; break;
        case M_BMI: cond = (s->a & SIGN_BIT) != 0; break;
        case M_BPL: cond = (s->a & SIGN_BIT) == 0; break;
        case M_BZE: cond = s->a == 0; break;
        case M_BNZ: cond = s->a != 0; break;
        case M_BOV: cond = s->overflow; break;
        case M_BNO: cond = !s->overflow; break;
        case M_BPE: cond = s->parity_error; break;
        case M_BPC: cond = !s->parity_error; break;
        case M_BNR: cond = s->n_ready; break;
        case M_BNN: cond = !s->n_ready; break;
        default: break;
    }
    if (mnem == M_BOV || mnem == M_BNO) s->overflow = 0;
    if (mnem == M_BPE || mnem == M_BPC) s->parity_error = 0;
    if (!cond) s->pc = (s->pc + 1) % s->memory_size;
}

static void execute_shift(Ge225Simulator *s, Mnemonic mnem, int32_t count) {
    int32_t a_sign, q_sign;
    uint32_t a_data, q_data;
    if (count == 0) {
        if (mnem == M_SRD)
            s->q = with_sign(s->q, sign_of(s->a));
        else if (mnem == M_SLD)
            s->a = with_sign(s->a, sign_of(s->q));
        return;
    }
    a_sign = sign_of(s->a);
    a_data = (uint32_t)(s->a & DATA_MASK);
    q_sign = sign_of(s->q);
    q_data = (uint32_t)(s->q & DATA_MASK);
    switch (mnem) {
        case M_SRA: {
            int sh = count < 19 ? (int)count : 19;
            s->a = from_signed20(to_signed20(s->a) >> sh);
            break;
        }
        case M_SLA: {
            int ov_sh = (19 - count) > 0 ? (int)(19 - count) : 0;
            s->overflow = (a_data >> ov_sh) != 0;
            s->a = with_sign((int32_t)((a_data << count) & (uint32_t)DATA_MASK),
                             a_sign);
            break;
        }
        case M_SCA: {
            int32_t rot = count % 19;
            if (rot != 0)
                a_data = ((a_data >> rot) | (a_data << (19 - rot))) &
                         (uint32_t)DATA_MASK;
            s->a = with_sign((int32_t)a_data, a_sign);
            break;
        }
        case M_SAN: {
            uint32_t fill = a_sign ? (((uint32_t)1 << count) - 1) : 0;
            uint32_t combu = ((a_data & (uint32_t)DATA_MASK) << 6) |
                             ((uint32_t)s->n & (uint32_t)N_MASK);
            int32_t comb = (int32_t)((fill << 25) | combu);
            comb = comb >> count; /* arithmetic */
            s->a = with_sign((comb >> 6) & DATA_MASK, a_sign);
            s->n = comb & N_MASK;
            break;
        }
        case M_SNA: {
            int32_t comb = (int32_t)((((uint32_t)s->n & (uint32_t)N_MASK) << 19) |
                                     a_data);
            comb = comb >> count;
            s->n = (comb >> 19) & N_MASK;
            s->a = with_sign(comb & DATA_MASK, a_sign);
            break;
        }
        case M_SRD: {
            int64_t value = combine_words(s->a, s->q) >> count;
            s->a = with_sign((int32_t)((value >> 20) & DATA_MASK), a_sign);
            s->q = with_sign((int32_t)(value & DATA_MASK), a_sign);
            break;
        }
        case M_NAQ: {
            int64_t comb = (int64_t)(
                (((uint64_t)(uint32_t)(s->n & N_MASK)) << 38) |
                (((uint64_t)(a_data & (uint32_t)DATA_MASK)) << 19) |
                (uint64_t)q_data);
            comb = comb >> count;
            s->n = (int32_t)((comb >> 38) & N_MASK);
            s->a = with_sign((int32_t)((comb >> 19) & DATA_MASK), a_sign);
            s->q = with_sign((int32_t)(comb & DATA_MASK), a_sign);
            break;
        }
        case M_SCD: {
            int32_t rot = count % 38;
            uint64_t comb =
                (((uint64_t)(a_data & (uint32_t)DATA_MASK)) << 19) |
                (uint64_t)q_data;
            if (rot != 0)
                comb = ((comb >> rot) | (comb << (38 - rot))) &
                       ((UINT64_C(1) << 38) - 1);
            s->a = with_sign((int32_t)((comb >> 19) & (uint64_t)(uint32_t)DATA_MASK),
                             a_sign);
            s->q = with_sign((int32_t)(comb & (uint64_t)(uint32_t)DATA_MASK),
                             a_sign);
            break;
        }
        case M_ANQ: {
            int32_t i;
            for (i = 0; i < count; i++) {
                int32_t bit = s->a & 1;
                s->a = from_signed20(to_signed20(s->a) >> 1);
                q_data = (uint32_t)(((bit << 18) | ((s->q & DATA_MASK) >> 1)) &
                                    DATA_MASK);
                s->q = with_sign((int32_t)q_data, a_sign);
                s->n = ((bit << 5) | (s->n >> 1)) & N_MASK;
            }
            break;
        }
        case M_SLD: {
            uint64_t comb =
                (((uint64_t)(a_data & (uint32_t)DATA_MASK)) << 19) |
                (uint64_t)q_data;
            int ov_sh = (38 - count) > 0 ? (int)(38 - count) : 0;
            s->overflow = (comb >> ov_sh) != 0;
            comb = (comb << count) & ((UINT64_C(1) << 38) - 1);
            s->a = with_sign((int32_t)((comb >> 19) & (uint64_t)(uint32_t)DATA_MASK),
                             q_sign);
            s->q = with_sign((int32_t)(comb & (uint64_t)(uint32_t)DATA_MASK),
                             q_sign);
            break;
        }
        case M_NOR: {
            int32_t shifts = 0;
            int32_t target = a_sign == 0 ? 0 : 1;
            while (shifts < count) {
                int32_t lead = (int32_t)((a_data >> 18) & 1u);
                if (lead != target) break;
                if (lead == 1) s->overflow = 1;
                a_data = (a_data << 1) & (uint32_t)DATA_MASK;
                shifts++;
            }
            s->a = with_sign((int32_t)a_data, a_sign);
            set_x_word(s, 0, count - shifts);
            break;
        }
        case M_DNO: {
            int32_t shifts = 0;
            int32_t target = a_sign == 0 ? 0 : 1;
            uint64_t comb =
                (((uint64_t)(a_data & (uint32_t)DATA_MASK)) << 19) |
                (uint64_t)q_data;
            while (shifts < count) {
                int32_t lead = (int32_t)((comb >> 37) & 1u);
                if (lead != target) break;
                if (lead == 1) s->overflow = 1;
                comb = (comb << 1) & ((UINT64_C(1) << 38) - 1);
                shifts++;
            }
            s->a = with_sign((int32_t)((comb >> 19) & (uint64_t)(uint32_t)DATA_MASK),
                             q_sign);
            s->q = with_sign((int32_t)(comb & (uint64_t)(uint32_t)DATA_MASK),
                             q_sign);
            set_x_word(s, 0, count - shifts);
            break;
        }
        default:
            break;
    }
}

static GeStatus execute_fixed(Ge225Simulator *s, const Decoded *d) {
    Mnemonic mnem = d->mnem;
    int32_t count = d->count;
    switch (mnem) {
        case M_OFF:
            s->typewriter_power = 0;
            s->n_ready = 1;
            break;
        case M_TYP: {
            int32_t code;
            if (!s->typewriter_power) {
                s->n_ready = 0;
                return GE_OK;
            }
            code = s->n & N_MASK;
            if (code == 037)
                tw_append(s, "\r");
            else if (code == 076)
                tw_append(s, "\t");
            else if (code != 072 && code != 075) {
                const char *ch = typewriter_char(code);
                if (ch == NULL) return GE_ERR_INVALID_TYPEWRITER_CODE;
                tw_append(s, ch);
            }
            s->n_ready = 1;
            break;
        }
        case M_TON: s->typewriter_power = 1; break;
        case M_RCS: s->a |= s->control_switches; break;
        case M_HPT: s->n_ready = 0; break;
        case M_LDZ: s->a = 0; break;
        case M_LDO: s->a = 1; break;
        case M_LMO: s->a = MASK_20; break;
        case M_CPL: s->a = (~s->a) & MASK_20; break;
        case M_NEG: {
            int32_t before = to_signed20(s->a);
            s->a = from_signed20(-before);
            s->overflow = before == -(1 << 19);
            break;
        }
        case M_CHS: s->a ^= SIGN_BIT; break;
        case M_NOP: break;
        case M_LAQ: s->a = s->q; break;
        case M_LQA: s->q = s->a; break;
        case M_XAQ: {
            int32_t t = s->a;
            s->a = s->q;
            s->q = t;
            break;
        }
        case M_MAQ:
            s->q = s->a;
            s->a = 0;
            break;
        case M_ADO: {
            int32_t total = to_signed20(s->a) + 1;
            s->a = from_signed20(total);
            s->overflow = ov20(total);
            break;
        }
        case M_SBO: {
            int32_t total = to_signed20(s->a) - 1;
            s->a = from_signed20(total);
            s->overflow = ov20(total);
            break;
        }
        case M_SET_DECMODE: s->decimal_mode = 1; break;
        case M_SET_BINMODE: s->decimal_mode = 0; break;
        case M_SXG: s->selected_x_group = (size_t)(s->a & 0x1f); break;
        case M_SET_PST: s->automatic_interrupt_mode = 1; break;
        case M_SET_PBK: s->automatic_interrupt_mode = 0; break;
        case M_BOD: case M_BEV: case M_BMI: case M_BPL: case M_BZE:
        case M_BNZ: case M_BOV: case M_BNO: case M_BPE: case M_BPC:
        case M_BNR: case M_BNN:
            execute_branch_test(s, mnem);
            break;
        case M_SRA: case M_SNA: case M_SCA: case M_SAN: case M_SRD:
        case M_NAQ: case M_SCD: case M_ANQ: case M_SLA: case M_SLD:
        case M_NOR: case M_DNO:
            execute_shift(s, mnem, count);
            break;
        default:
            return GE_ERR_DECODE;
    }
    return GE_OK;
}

GeStatus ge225_step(Ge225Simulator *s, Ge225Trace *trace) {
    int32_t pc_before, a_before, q_before;
    Decoded d;
    GeStatus st;
    int has_eff = 0;
    int32_t eff = 0;

    if (s->halted) return GE_ERR_HALTED;
    pc_before = s->pc;
    if ((st = ge225_read_word(s, s->pc, &s->ir)) != GE_OK) return st;
    s->pc = (s->pc + 1) % s->memory_size;
    if ((st = decode_word(s->ir, &d)) != GE_OK) return st;
    a_before = s->a;
    q_before = s->q;

    if (!d.fixed_word) {
        int32_t address = d.address;
        int no_eff = (d.mnem == M_BXL || d.mnem == M_BXH || d.mnem == M_LDX ||
                      d.mnem == M_SPB || d.mnem == M_INX || d.mnem == M_STX ||
                      d.mnem == M_MOY);
        if (!no_eff) {
            eff = resolve_effective_address(s, address, d.modifier);
            has_eff = 1;
        }
        st = execute_memory_reference(s, d.mnem, d.modifier,
                                      has_eff ? eff : address, address,
                                      pc_before);
        if (st != GE_OK) return st;
    } else {
        if ((st = execute_fixed(s, &d)) != GE_OK) return st;
    }

    if (s->oom) return GE_ERR_OUT_OF_MEMORY;
    if (trace != NULL) {
        trace->address = pc_before;
        trace->instruction_word = s->ir;
        trace->a_before = a_before;
        trace->a_after = s->a;
        trace->q_before = q_before;
        trace->q_after = s->q;
        trace->has_effective_address = has_eff;
        trace->effective_address = eff;
    }
    return GE_OK;
}

GeStatus ge225_run(Ge225Simulator *s, size_t max_steps) {
    size_t i;
    for (i = 0; i < max_steps; i++) {
        GeStatus st;
        if (s->halted) break;
        st = ge225_step(s, NULL);
        if (st != GE_OK) return st;
    }
    return GE_OK;
}

/* ── State accessors ───────────────────────────────────────────────────────*/

int32_t ge225_get_a(const Ge225Simulator *s) { return s->a; }
int32_t ge225_get_q(const Ge225Simulator *s) { return s->q; }
int32_t ge225_get_m(const Ge225Simulator *s) { return s->m; }
int32_t ge225_get_n(const Ge225Simulator *s) { return s->n; }
int32_t ge225_get_pc(const Ge225Simulator *s) { return s->pc; }
int32_t ge225_get_ir(const Ge225Simulator *s) { return s->ir; }
int ge225_get_overflow(const Ge225Simulator *s) { return s->overflow; }
int ge225_get_parity_error(const Ge225Simulator *s) { return s->parity_error; }
int ge225_get_decimal_mode(const Ge225Simulator *s) { return s->decimal_mode; }
int ge225_get_automatic_interrupt_mode(const Ge225Simulator *s) {
    return s->automatic_interrupt_mode;
}
size_t ge225_get_selected_x_group(const Ge225Simulator *s) {
    return s->selected_x_group;
}
int ge225_get_n_ready(const Ge225Simulator *s) { return s->n_ready; }
int ge225_get_typewriter_power(const Ge225Simulator *s) {
    return s->typewriter_power;
}
int ge225_get_halted(const Ge225Simulator *s) { return s->halted; }
int32_t ge225_get_x_word(const Ge225Simulator *s, size_t slot) {
    if (slot >= 4) return 0; /* guard the public API against an OOB slot */
    return get_x_word(s, slot);
}

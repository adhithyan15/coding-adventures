/*
 * ldp_format.c — a versioned binary codec for `.ldp` artefacts, pure ISO C17.
 * ==========================================================================
 *
 * See ldp_format.h for the design. `ldp_write` pre-walks the file to build the
 * string table in first-occurrence order, then emits header / table / records.
 * `ldp_read` parses through a bounds-checked cursor that returns
 * LDP_ERR_UNEXPECTED_EOF the moment a field would run past the buffer, and grows
 * nested arrays element-by-element so a hostile count cannot pre-allocate.
 */
#include "ldp_format.h"

#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, strlen, strcmp, memcmp */

#define LDP_MAGIC0 'L'
#define LDP_MAGIC1 'D'
#define LDP_MAGIC2 'P'
#define LDP_VERSION_MAJOR 1
#define LDP_VERSION_MINOR 0
#define LDP_LANGUAGE_FIELD_LEN 16

/* ── Small helpers ──────────────────────────────────────────────────────────*/
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

/* ── Destructors ────────────────────────────────────────────────────────────*/
static void free_instruction(LdpInstruction *ins) {
    size_t i;
    free(ins->opcode);
    for (i = 0; i < ins->types_seen_len; i++) {
        free(ins->types_seen[i].type_name);
    }
    free(ins->types_seen);
    ins->opcode = NULL;
    ins->types_seen = NULL;
    ins->types_seen_len = 0;
}
static void free_function(LdpFunction *fn) {
    size_t i;
    free(fn->name);
    for (i = 0; i < fn->params_len; i++) {
        free(fn->params[i]);
    }
    free(fn->params);
    for (i = 0; i < fn->instructions_len; i++) {
        free_instruction(&fn->instructions[i]);
    }
    free(fn->instructions);
    fn->name = NULL;
    fn->params = NULL;
    fn->instructions = NULL;
    fn->params_len = fn->instructions_len = 0;
}
static void free_module(LdpModule *m) {
    size_t i;
    free(m->name);
    for (i = 0; i < m->functions_len; i++) {
        free_function(&m->functions[i]);
    }
    free(m->functions);
    m->name = NULL;
    m->functions = NULL;
    m->functions_len = 0;
}
void ldp_file_free(LdpFile *file) {
    size_t i;
    if (!file) {
        return;
    }
    free(file->language);
    for (i = 0; i < file->modules_len; i++) {
        free_module(&file->modules[i]);
    }
    free(file->modules);
    free(file);
}

/* ── Byte writer ────────────────────────────────────────────────────────────*/
typedef struct {
    uint8_t *data;
    size_t len, cap;
    int oom;
} ByteBuf;

static void bw_bytes(ByteBuf *b, const uint8_t *p, size_t n) {
    if (b->oom) {
        return;
    }
    if (!grow((void **)&b->data, &b->cap, b->len + n, 1)) {
        b->oom = 1;
        return;
    }
    if (n) {
        memcpy(b->data + b->len, p, n);
    }
    b->len += n;
}
static void bw_u8(ByteBuf *b, uint8_t v) { bw_bytes(b, &v, 1); }
static void bw_u16(ByteBuf *b, uint16_t v) {
    uint8_t t[2];
    t[0] = (uint8_t)(v & 0xFF);
    t[1] = (uint8_t)((v >> 8) & 0xFF);
    bw_bytes(b, t, 2);
}
static void bw_u32(ByteBuf *b, uint32_t v) {
    uint8_t t[4];
    int i;
    for (i = 0; i < 4; i++) {
        t[i] = (uint8_t)((v >> (8 * i)) & 0xFF);
    }
    bw_bytes(b, t, 4);
}
static void bw_u64(ByteBuf *b, uint64_t v) {
    uint8_t t[8];
    int i;
    for (i = 0; i < 8; i++) {
        t[i] = (uint8_t)((v >> (8 * i)) & 0xFF);
    }
    bw_bytes(b, t, 8);
}

/* ── String-interning table (borrows the caller's strings) ──────────────────*/
typedef struct {
    const char **items; /* borrowed pointers, first-occurrence order */
    size_t len, cap;
} StrTable;

/* Return the 0-based index of `s`, adding it if new. On failure sets *st and
 * returns 0. */
static uint32_t intern(StrTable *t, const char *s, LdpStatus *st) {
    size_t i;
    for (i = 0; i < t->len; i++) {
        if (strcmp(t->items[i], s) == 0) {
            return (uint32_t)i;
        }
    }
    if (t->len >= 0xFFFFFFFFu) {
        *st = LDP_ERR_STRING_TABLE_OVERFLOW;
        return 0;
    }
    if (strlen(s) > 0xFFFF) {
        *st = LDP_ERR_STRING_TOO_LONG;
        return 0;
    }
    if (!grow((void **)&t->items, &t->cap, t->len + 1, sizeof(char *))) {
        *st = LDP_ERR_OUT_OF_MEMORY;
        return 0;
    }
    t->items[t->len] = s;
    return (uint32_t)t->len++;
}

/* Walk the whole file, interning every string in first-occurrence order. */
static LdpStatus intern_all(const LdpFile *file, StrTable *t) {
    size_t mi, fi, pi, ii, ti;
    LdpStatus st = LDP_OK;
    for (mi = 0; mi < file->modules_len; mi++) {
        const LdpModule *m = &file->modules[mi];
        intern(t, m->name, &st);
        if (st != LDP_OK) return st;
        for (fi = 0; fi < m->functions_len; fi++) {
            const LdpFunction *fn = &m->functions[fi];
            intern(t, fn->name, &st);
            if (st != LDP_OK) return st;
            for (pi = 0; pi < fn->params_len; pi++) {
                intern(t, fn->params[pi], &st);
                if (st != LDP_OK) return st;
            }
            for (ii = 0; ii < fn->instructions_len; ii++) {
                const LdpInstruction *ins = &fn->instructions[ii];
                intern(t, ins->opcode, &st);
                if (st != LDP_OK) return st;
                for (ti = 0; ti < ins->types_seen_len; ti++) {
                    intern(t, ins->types_seen[ti].type_name, &st);
                    if (st != LDP_OK) return st;
                }
            }
        }
    }
    return LDP_OK;
}

static LdpStatus encode_language(const char *lang, uint8_t out[16]) {
    size_t n = strlen(lang), i;
    if (n > LDP_LANGUAGE_FIELD_LEN) {
        return LDP_ERR_LANGUAGE_TOO_LONG;
    }
    for (i = 0; i < n; i++) {
        if ((unsigned char)lang[i] > 0x7F) {
            return LDP_ERR_LANGUAGE_NOT_ASCII;
        }
    }
    memset(out, 0, LDP_LANGUAGE_FIELD_LEN);
    for (i = 0; i < n; i++) {
        out[i] = (uint8_t)lang[i];
    }
    return LDP_OK;
}

LdpStatus ldp_write(const LdpFile *file, uint8_t **out, size_t *out_len) {
    StrTable table = {0};
    ByteBuf b = {0};
    LdpStatus st;
    uint8_t lang[16];
    size_t mi, fi, ii, pi, ti;

    *out = NULL;
    *out_len = 0;

    st = intern_all(file, &table);
    if (st != LDP_OK) {
        free(table.items);
        return st;
    }
    st = encode_language(file->language ? file->language : "", lang);
    if (st != LDP_OK) {
        free(table.items);
        return st;
    }
    if (file->modules_len > 0xFFFFFFFFu) {
        free(table.items);
        return LDP_ERR_STRING_TABLE_OVERFLOW;
    }

    /* Header. */
    bw_u8(&b, LDP_MAGIC0);
    bw_u8(&b, LDP_MAGIC1);
    bw_u8(&b, LDP_MAGIC2);
    bw_u8(&b, 0);
    bw_u16(&b, LDP_VERSION_MAJOR);
    bw_u16(&b, LDP_VERSION_MINOR);
    bw_bytes(&b, lang, 16);
    bw_u32(&b, file->flags);
    bw_u32(&b, (uint32_t)file->modules_len);
    bw_u32(&b, 0); /* reserved */

    /* String table. */
    bw_u32(&b, (uint32_t)table.len);
    for (mi = 0; mi < table.len; mi++) {
        const char *s = table.items[mi];
        size_t slen = strlen(s);
        bw_u16(&b, (uint16_t)slen);
        bw_bytes(&b, (const uint8_t *)s, slen);
        bw_u8(&b, 0);
    }

    /* Module records (intern here is a pure lookup — all strings present). */
    st = LDP_OK;
    for (mi = 0; mi < file->modules_len; mi++) {
        const LdpModule *m = &file->modules[mi];
        bw_u32(&b, intern(&table, m->name, &st));
        if (m->functions_len > 0xFFFFFFFFu) {
            st = LDP_ERR_STRING_TABLE_OVERFLOW;
            break;
        }
        bw_u32(&b, (uint32_t)m->functions_len);
        for (fi = 0; fi < m->functions_len; fi++) {
            const LdpFunction *fn = &m->functions[fi];
            uint8_t param_count;
            bw_u32(&b, intern(&table, fn->name, &st));
            param_count = fn->params_len > 0xFF ? 0xFF
                                                : (uint8_t)fn->params_len;
            bw_u8(&b, param_count);
            bw_u8(&b, 0);
            bw_u8(&b, 0);
            bw_u8(&b, 0);
            for (pi = 0; pi < fn->params_len; pi++) {
                bw_u32(&b, intern(&table, fn->params[pi], &st));
            }
            bw_u64(&b, fn->call_count);
            bw_u64(&b, fn->total_self_time_ns);
            bw_u8(&b, (uint8_t)fn->type_status);
            bw_u8(&b, (uint8_t)fn->promotion_state);
            bw_u8(&b, 0);
            bw_u8(&b, 0);
            if (fn->instructions_len > 0xFFFFFFFFu) {
                st = LDP_ERR_STRING_TABLE_OVERFLOW;
                break;
            }
            bw_u32(&b, (uint32_t)fn->instructions_len);
            for (ii = 0; ii < fn->instructions_len; ii++) {
                const LdpInstruction *ins = &fn->instructions[ii];
                bw_u32(&b, ins->instr_index);
                bw_u32(&b, intern(&table, ins->opcode, &st));
                bw_u32(&b, ins->observation_count);
                bw_u8(&b, (uint8_t)ins->observed_kind);
                bw_u8(&b, 0);
                bw_u8(&b, 0);
                bw_u8(&b, 0);
                bw_u32(&b, ins->observation_count_at_promotion);
                bw_u64(&b, ins->time_to_first_observation_ns);
                bw_u64(&b, ins->time_to_promotion_ns);
                if (ins->types_seen_len > 0xFFFFFFFFu) {
                    st = LDP_ERR_STRING_TABLE_OVERFLOW;
                    break;
                }
                bw_u32(&b, (uint32_t)ins->types_seen_len);
                for (ti = 0; ti < ins->types_seen_len; ti++) {
                    bw_u32(&b, intern(&table, ins->types_seen[ti].type_name,
                                      &st));
                    bw_u32(&b, ins->types_seen[ti].count);
                }
                bw_u32(&b, 0); /* ic_entry_count */
            }
            if (st != LDP_OK) break;
        }
        if (st != LDP_OK) break;
    }

    free(table.items);
    if (st != LDP_OK) {
        free(b.data);
        return st;
    }
    if (b.oom) {
        free(b.data);
        return LDP_ERR_OUT_OF_MEMORY;
    }
    *out = b.data;
    *out_len = b.len;
    return LDP_OK;
}

/* ── Byte reader ────────────────────────────────────────────────────────────*/
typedef struct {
    const uint8_t *data;
    size_t len, pos;
    LdpStatus status;
} ByteReader;

static int rr_exact(ByteReader *r, uint8_t *out, size_t n) {
    if (r->status != LDP_OK) {
        return 0;
    }
    if (r->len - r->pos < n) {
        r->status = LDP_ERR_UNEXPECTED_EOF;
        return 0;
    }
    if (n) {
        memcpy(out, r->data + r->pos, n);
    }
    r->pos += n;
    return 1;
}
static uint8_t rr_u8(ByteReader *r) {
    uint8_t b;
    return rr_exact(r, &b, 1) ? b : 0;
}
static uint16_t rr_u16(ByteReader *r) {
    uint8_t b[2];
    if (!rr_exact(r, b, 2)) return 0;
    return (uint16_t)(b[0] | (b[1] << 8));
}
static uint32_t rr_u32(ByteReader *r) {
    uint8_t b[4];
    if (!rr_exact(r, b, 4)) return 0;
    return (uint32_t)b[0] | ((uint32_t)b[1] << 8) | ((uint32_t)b[2] << 16) |
           ((uint32_t)b[3] << 24);
}
static uint64_t rr_u64(ByteReader *r) {
    uint8_t b[8];
    int i;
    uint64_t v = 0;
    if (!rr_exact(r, b, 8)) return 0;
    for (i = 0; i < 8; i++) {
        v |= (uint64_t)b[i] << (8 * i);
    }
    return v;
}

/* Owned copy of string-table entry `idx`, or NULL + set status. */
static char *lookup_str(ByteReader *r, char *const *strings, size_t str_count,
                        uint32_t idx) {
    char *copy;
    if (r->status != LDP_OK) {
        return NULL;
    }
    if ((size_t)idx >= str_count) {
        r->status = LDP_ERR_BAD_STRING_INDEX;
        return NULL;
    }
    copy = sdup(strings[idx]);
    if (!copy) {
        r->status = LDP_ERR_OUT_OF_MEMORY;
    }
    return copy;
}

LdpStatus ldp_read(const uint8_t *data, size_t len, LdpFile **out) {
    ByteReader r;
    LdpFile *file = NULL;
    char **strings = NULL;
    size_t str_cap = 0, str_count_read = 0;
    uint8_t magic[4], lang[16];
    uint16_t version_major, version_minor;
    uint32_t flags, record_count, str_count;
    size_t lang_end, i;
    size_t mcap = 0;

    *out = NULL;
    r.data = data;
    r.len = len;
    r.pos = 0;
    r.status = LDP_OK;

    rr_exact(&r, magic, 4);
    if (r.status != LDP_OK) {
        return r.status;
    }
    if (magic[0] != LDP_MAGIC0 || magic[1] != LDP_MAGIC1 ||
        magic[2] != LDP_MAGIC2 || magic[3] != 0) {
        return LDP_ERR_BAD_MAGIC;
    }
    version_major = rr_u16(&r);
    version_minor = rr_u16(&r);
    if (r.status != LDP_OK) {
        return r.status;
    }
    if (version_major != LDP_VERSION_MAJOR) {
        return LDP_ERR_UNSUPPORTED_MAJOR;
    }
    rr_exact(&r, lang, 16);
    flags = rr_u32(&r);
    record_count = rr_u32(&r);
    (void)rr_u32(&r); /* reserved */
    str_count = rr_u32(&r);
    if (r.status != LDP_OK) {
        return r.status;
    }

    file = (LdpFile *)calloc(1, sizeof(LdpFile));
    if (!file) {
        return LDP_ERR_OUT_OF_MEMORY;
    }
    lang_end = 0;
    while (lang_end < 16 && lang[lang_end] != 0) {
        lang_end++;
    }
    file->version_major = version_major;
    file->version_minor = version_minor;
    file->flags = flags;
    file->language = sdup_n((const char *)lang, lang_end);
    if (!file->language) {
        ldp_file_free(file);
        return LDP_ERR_OUT_OF_MEMORY;
    }

    /* String table — grow incrementally (never trust str_count for sizing). */
    for (i = 0; i < str_count; i++) {
        uint16_t slen = rr_u16(&r);
        char *s;
        if (r.status != LDP_OK) {
            goto read_fail;
        }
        s = (char *)malloc((size_t)slen + 1);
        if (!s) {
            r.status = LDP_ERR_OUT_OF_MEMORY;
            goto read_fail;
        }
        if (slen && !rr_exact(&r, (uint8_t *)s, slen)) {
            free(s);
            goto read_fail;
        }
        s[slen] = '\0';
        (void)rr_u8(&r); /* NUL terminator */
        if (r.status != LDP_OK) {
            free(s);
            goto read_fail;
        }
        if (!grow((void **)&strings, &str_cap, str_count_read + 1,
                  sizeof(char *))) {
            free(s);
            r.status = LDP_ERR_OUT_OF_MEMORY;
            goto read_fail;
        }
        strings[str_count_read++] = s;
    }

    /* Module records. */
    for (i = 0; i < record_count; i++) {
        LdpModule module;
        uint32_t module_name_idx, function_count;
        size_t fcap = 0, fj;
        memset(&module, 0, sizeof module);

        module_name_idx = rr_u32(&r);
        module.name = lookup_str(&r, strings, str_count_read, module_name_idx);
        function_count = rr_u32(&r);
        if (r.status != LDP_OK) {
            free_module(&module);
            goto read_fail;
        }
        for (fj = 0; fj < function_count; fj++) {
            LdpFunction fn;
            uint32_t function_name_idx, param_count, instr_count;
            uint8_t pad3[3], pad2[2];
            uint8_t ts_byte, ps_byte;
            size_t pcap = 0, pj, icap = 0, ij;
            memset(&fn, 0, sizeof fn);

            function_name_idx = rr_u32(&r);
            param_count = rr_u8(&r);
            rr_exact(&r, pad3, 3);
            if (r.status != LDP_OK) {
                free_function(&fn);
                free_module(&module);
                goto read_fail;
            }
            for (pj = 0; pj < param_count; pj++) {
                uint32_t idx = rr_u32(&r);
                char *p = lookup_str(&r, strings, str_count_read, idx);
                if (r.status != LDP_OK) {
                    free(p);
                    free_function(&fn);
                    free_module(&module);
                    goto read_fail;
                }
                if (!grow((void **)&fn.params, &pcap, fn.params_len + 1,
                          sizeof(char *))) {
                    free(p);
                    r.status = LDP_ERR_OUT_OF_MEMORY;
                    free_function(&fn);
                    free_module(&module);
                    goto read_fail;
                }
                fn.params[fn.params_len++] = p;
            }
            fn.call_count = rr_u64(&r);
            fn.total_self_time_ns = rr_u64(&r);
            ts_byte = rr_u8(&r);
            ps_byte = rr_u8(&r);
            rr_exact(&r, pad2, 2);
            instr_count = rr_u32(&r);
            if (r.status != LDP_OK) {
                free_function(&fn);
                free_module(&module);
                goto read_fail;
            }
            for (ij = 0; ij < instr_count; ij++) {
                LdpInstruction ins;
                uint32_t opcode_idx, kb, types_count, tj;
                uint8_t pad3b[3];
                memset(&ins, 0, sizeof ins);
                ins.instr_index = rr_u32(&r);
                opcode_idx = rr_u32(&r);
                ins.observation_count = rr_u32(&r);
                kb = rr_u8(&r);
                rr_exact(&r, pad3b, 3);
                ins.observation_count_at_promotion = rr_u32(&r);
                ins.time_to_first_observation_ns = rr_u64(&r);
                ins.time_to_promotion_ns = rr_u64(&r);
                types_count = rr_u32(&r);
                if (r.status != LDP_OK) {
                    free_instruction(&ins);
                    free_function(&fn);
                    free_module(&module);
                    goto read_fail;
                }
                {
                    size_t tcap = 0;
                    for (tj = 0; tj < types_count; tj++) {
                        uint32_t type_idx = rr_u32(&r);
                        uint32_t type_count = rr_u32(&r);
                        char *tn;
                        if (r.status != LDP_OK) {
                            free_instruction(&ins);
                            free_function(&fn);
                            free_module(&module);
                            goto read_fail;
                        }
                        tn = lookup_str(&r, strings, str_count_read, type_idx);
                        if (r.status != LDP_OK) {
                            free(tn);
                            free_instruction(&ins);
                            free_function(&fn);
                            free_module(&module);
                            goto read_fail;
                        }
                        if (!grow((void **)&ins.types_seen, &tcap,
                                  ins.types_seen_len + 1, sizeof(LdpTypeSeen))) {
                            free(tn);
                            r.status = LDP_ERR_OUT_OF_MEMORY;
                            free_instruction(&ins);
                            free_function(&fn);
                            free_module(&module);
                            goto read_fail;
                        }
                        ins.types_seen[ins.types_seen_len].type_name = tn;
                        ins.types_seen[ins.types_seen_len].count = type_count;
                        ins.types_seen_len++;
                    }
                }
                (void)rr_u32(&r); /* ic_entry_count */
                ins.opcode = lookup_str(&r, strings, str_count_read, opcode_idx);
                if (kb <= 3) {
                    ins.observed_kind = (LdpObservedKind)kb;
                } else if (r.status == LDP_OK) {
                    r.status = LDP_ERR_BAD_OBSERVED_KIND;
                }
                if (r.status != LDP_OK) {
                    free_instruction(&ins);
                    free_function(&fn);
                    free_module(&module);
                    goto read_fail;
                }
                if (!grow((void **)&fn.instructions, &icap,
                          fn.instructions_len + 1, sizeof(LdpInstruction))) {
                    free_instruction(&ins);
                    r.status = LDP_ERR_OUT_OF_MEMORY;
                    free_function(&fn);
                    free_module(&module);
                    goto read_fail;
                }
                fn.instructions[fn.instructions_len++] = ins;
            }
            fn.name = lookup_str(&r, strings, str_count_read, function_name_idx);
            if (ts_byte <= 2) {
                fn.type_status = (LdpTypeStatus)ts_byte;
            } else if (r.status == LDP_OK) {
                r.status = LDP_ERR_BAD_TYPE_STATUS;
            }
            if (ps_byte <= 2) {
                fn.promotion_state = (LdpPromotionState)ps_byte;
            } else if (r.status == LDP_OK) {
                r.status = LDP_ERR_BAD_PROMOTION_STATE;
            }
            if (r.status != LDP_OK) {
                free_function(&fn);
                free_module(&module);
                goto read_fail;
            }
            if (!grow((void **)&module.functions, &fcap,
                      module.functions_len + 1, sizeof(LdpFunction))) {
                r.status = LDP_ERR_OUT_OF_MEMORY;
                free_function(&fn);
                free_module(&module);
                goto read_fail;
            }
            module.functions[module.functions_len++] = fn;
        }
        if (!grow((void **)&file->modules, &mcap, file->modules_len + 1,
                  sizeof(LdpModule))) {
            r.status = LDP_ERR_OUT_OF_MEMORY;
            free_module(&module);
            goto read_fail;
        }
        file->modules[file->modules_len++] = module;
    }

    for (i = 0; i < str_count_read; i++) {
        free(strings[i]);
    }
    free(strings);
    *out = file;
    return LDP_OK;

read_fail:
    for (i = 0; i < str_count_read; i++) {
        free(strings[i]);
    }
    free(strings);
    ldp_file_free(file);
    return r.status;
}

/* ── Deep equality ──────────────────────────────────────────────────────────*/
static int streq(const char *a, const char *b) {
    if (!a) a = "";
    if (!b) b = "";
    return strcmp(a, b) == 0;
}

int ldp_file_equal(const LdpFile *a, const LdpFile *b) {
    size_t mi, fi, pi, ii, ti;
    if (!a || !b) {
        return a == b;
    }
    if (a->version_major != b->version_major ||
        a->version_minor != b->version_minor || a->flags != b->flags ||
        !streq(a->language, b->language) ||
        a->modules_len != b->modules_len) {
        return 0;
    }
    for (mi = 0; mi < a->modules_len; mi++) {
        const LdpModule *ma = &a->modules[mi], *mb = &b->modules[mi];
        if (!streq(ma->name, mb->name) ||
            ma->functions_len != mb->functions_len) {
            return 0;
        }
        for (fi = 0; fi < ma->functions_len; fi++) {
            const LdpFunction *fa = &ma->functions[fi], *fb = &mb->functions[fi];
            if (!streq(fa->name, fb->name) ||
                fa->params_len != fb->params_len ||
                fa->call_count != fb->call_count ||
                fa->total_self_time_ns != fb->total_self_time_ns ||
                fa->type_status != fb->type_status ||
                fa->promotion_state != fb->promotion_state ||
                fa->instructions_len != fb->instructions_len) {
                return 0;
            }
            for (pi = 0; pi < fa->params_len; pi++) {
                if (!streq(fa->params[pi], fb->params[pi])) {
                    return 0;
                }
            }
            for (ii = 0; ii < fa->instructions_len; ii++) {
                const LdpInstruction *ia = &fa->instructions[ii];
                const LdpInstruction *ib = &fb->instructions[ii];
                if (ia->instr_index != ib->instr_index ||
                    !streq(ia->opcode, ib->opcode) ||
                    ia->observation_count != ib->observation_count ||
                    ia->observed_kind != ib->observed_kind ||
                    ia->observation_count_at_promotion !=
                        ib->observation_count_at_promotion ||
                    ia->time_to_first_observation_ns !=
                        ib->time_to_first_observation_ns ||
                    ia->time_to_promotion_ns != ib->time_to_promotion_ns ||
                    ia->types_seen_len != ib->types_seen_len) {
                    return 0;
                }
                for (ti = 0; ti < ia->types_seen_len; ti++) {
                    if (!streq(ia->types_seen[ti].type_name,
                               ib->types_seen[ti].type_name) ||
                        ia->types_seen[ti].count != ib->types_seen[ti].count) {
                        return 0;
                    }
                }
            }
        }
    }
    return 1;
}

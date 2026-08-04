/*
 * compiler_source_map.c — implementation of the compiler source-map sidecar.
 * ===========================================================================
 *
 * Each segment is a growable array of entries with linear-scan lookups (the
 * same shape as the Rust source, whose `Vec::iter().find(..)` this mirrors).
 * The SourceMapChain owns its SourceToAst and AstToIr segments and the passes /
 * backend handed to it, and composes them for the two end-to-end queries.
 */
#include "compiler_source_map.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Shared helpers
 * =========================================================================== */

static char *dup_str(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, s, n + 1);
    return out;
}

/* Ensure a dynamic array `*data` (of `*cap` elements, `elem` bytes each) has
 * room for at least `needed` elements, guarding the doubling against overflow.
 * Returns 1 on success, 0 on OOM / overflow. */
static int ensure_cap(void **data, size_t *cap, size_t needed, size_t elem) {
    if (needed <= *cap) return 1;
    size_t nc = *cap ? *cap : 4;
    while (nc < needed) {
        if (nc > ((size_t)-1) / 2 / elem) return 0;
        nc *= 2;
    }
    void *nd = realloc(*data, nc * elem);
    if (!nd) return 0;
    *data = nd;
    *cap = nc;
    return 1;
}

/* Copy `n` int64 values into a fresh array (NULL for n==0). *ok reports OOM. */
static int64_t *copy_i64(const int64_t *src, size_t n, int *ok) {
    *ok = 1;
    if (n == 0) return NULL;
    if (n > ((size_t)-1) / sizeof(int64_t)) { /* guard the multiply first */
        *ok = 0;
        return NULL;
    }
    int64_t *out = malloc(n * sizeof(int64_t));
    if (!out) {
        *ok = 0;
        return NULL;
    }
    memcpy(out, src, n * sizeof(int64_t));
    return out;
}

/* ===========================================================================
 *  SourcePosition
 * =========================================================================== */

int smap_position_to_string(const SmapPosition *p, char *buf, size_t buflen) {
    int n = snprintf(buf, buflen, "%s:%zu:%zu (len=%zu)", p->file, p->line,
                     p->column, p->length);
    if (n < 0 || (size_t)n >= buflen) return -1;
    return n;
}

/* ===========================================================================
 *  Segment 1: SourceToAst
 * =========================================================================== */

typedef struct {
    SmapPosition pos; /* pos.file owns a malloc'd copy */
    size_t ast_node_id;
} S2aEntry;

struct SmapSourceToAst {
    S2aEntry *entries;
    size_t len, cap;
};

SmapSourceToAst *smap_s2a_new(void) {
    return calloc(1, sizeof(SmapSourceToAst));
}

void smap_s2a_free(SmapSourceToAst *s) {
    if (!s) return;
    for (size_t i = 0; i < s->len; i++) free((void *)s->entries[i].pos.file);
    free(s->entries);
    free(s);
}

int smap_s2a_add(SmapSourceToAst *s, const SmapPosition *pos,
                 size_t ast_node_id) {
    if (!ensure_cap((void **)&s->entries, &s->cap, s->len + 1, sizeof(S2aEntry)))
        return -1;
    char *file = dup_str(pos->file);
    if (!file) return -1;
    S2aEntry *e = &s->entries[s->len];
    e->pos.file = file;
    e->pos.line = pos->line;
    e->pos.column = pos->column;
    e->pos.length = pos->length;
    e->ast_node_id = ast_node_id;
    s->len++;
    return 0;
}

const SmapPosition *smap_s2a_lookup_by_node_id(const SmapSourceToAst *s,
                                               size_t ast_node_id) {
    for (size_t i = 0; i < s->len; i++) {
        if (s->entries[i].ast_node_id == ast_node_id) return &s->entries[i].pos;
    }
    return NULL;
}

/* ===========================================================================
 *  Segment 2: AstToIr
 * =========================================================================== */

typedef struct {
    size_t ast_node_id;
    int64_t *ir_ids;
    size_t n_ir;
} A2iEntry;

struct SmapAstToIr {
    A2iEntry *entries;
    size_t len, cap;
};

SmapAstToIr *smap_a2i_new(void) { return calloc(1, sizeof(SmapAstToIr)); }

void smap_a2i_free(SmapAstToIr *a) {
    if (!a) return;
    for (size_t i = 0; i < a->len; i++) free(a->entries[i].ir_ids);
    free(a->entries);
    free(a);
}

int smap_a2i_add(SmapAstToIr *a, size_t ast_node_id, const int64_t *ir_ids,
                 size_t n) {
    if (!ensure_cap((void **)&a->entries, &a->cap, a->len + 1, sizeof(A2iEntry)))
        return -1;
    int ok;
    int64_t *copy = copy_i64(ir_ids, n, &ok);
    if (!ok) return -1;
    A2iEntry *e = &a->entries[a->len];
    e->ast_node_id = ast_node_id;
    e->ir_ids = copy;
    e->n_ir = n;
    a->len++;
    return 0;
}

const int64_t *smap_a2i_lookup_by_ast_node_id(const SmapAstToIr *a,
                                              size_t ast_node_id,
                                              size_t *count_out) {
    for (size_t i = 0; i < a->len; i++) {
        if (a->entries[i].ast_node_id == ast_node_id) {
            *count_out = a->entries[i].n_ir;
            return a->entries[i].ir_ids;
        }
    }
    *count_out = 0;
    return NULL;
}

int smap_a2i_lookup_by_ir_id(const SmapAstToIr *a, int64_t ir_id, size_t *out) {
    for (size_t i = 0; i < a->len; i++) {
        for (size_t j = 0; j < a->entries[i].n_ir; j++) {
            if (a->entries[i].ir_ids[j] == ir_id) {
                *out = a->entries[i].ast_node_id;
                return 1;
            }
        }
    }
    return 0;
}

/* ===========================================================================
 *  Segment 3: IrToIr
 * =========================================================================== */

typedef struct {
    int64_t original_id;
    int64_t *new_ids;
    size_t n_new;
} I2iEntry;

struct SmapIrToIr {
    I2iEntry *entries;
    size_t len, cap;
    int64_t *deleted; /* a linear-scan set */
    size_t n_del, cap_del;
    char *pass_name;
};

SmapIrToIr *smap_i2i_new(const char *pass_name) {
    SmapIrToIr *m = calloc(1, sizeof(SmapIrToIr));
    if (!m) return NULL;
    m->pass_name = dup_str(pass_name);
    if (!m->pass_name) {
        free(m);
        return NULL;
    }
    return m;
}

void smap_i2i_free(SmapIrToIr *m) {
    if (!m) return;
    for (size_t i = 0; i < m->len; i++) free(m->entries[i].new_ids);
    free(m->entries);
    free(m->deleted);
    free(m->pass_name);
    free(m);
}

int smap_i2i_add_mapping(SmapIrToIr *m, int64_t original_id,
                         const int64_t *new_ids, size_t n) {
    if (!ensure_cap((void **)&m->entries, &m->cap, m->len + 1, sizeof(I2iEntry)))
        return -1;
    int ok;
    int64_t *copy = copy_i64(new_ids, n, &ok);
    if (!ok) return -1;
    I2iEntry *e = &m->entries[m->len];
    e->original_id = original_id;
    e->new_ids = copy;
    e->n_new = n;
    m->len++;
    return 0;
}

int smap_i2i_is_deleted(const SmapIrToIr *m, int64_t original_id) {
    for (size_t i = 0; i < m->n_del; i++) {
        if (m->deleted[i] == original_id) return 1;
    }
    return 0;
}

int smap_i2i_add_deletion(SmapIrToIr *m, int64_t original_id) {
    /* Add to the deleted set, plus an empty-new_ids entry (Rust behaviour). */
    if (!ensure_cap((void **)&m->deleted, &m->cap_del, m->n_del + 1,
                    sizeof(int64_t)))
        return -1;
    m->deleted[m->n_del++] = original_id;
    return smap_i2i_add_mapping(m, original_id, NULL, 0);
}

const int64_t *smap_i2i_lookup_by_original_id(const SmapIrToIr *m,
                                             int64_t original_id,
                                             size_t *count_out) {
    if (smap_i2i_is_deleted(m, original_id)) {
        *count_out = 0;
        return NULL;
    }
    for (size_t i = 0; i < m->len; i++) {
        if (m->entries[i].original_id == original_id) {
            *count_out = m->entries[i].n_new;
            return m->entries[i].new_ids;
        }
    }
    *count_out = 0;
    return NULL;
}

int smap_i2i_lookup_by_new_id(const SmapIrToIr *m, int64_t new_id,
                              int64_t *out) {
    for (size_t i = 0; i < m->len; i++) {
        for (size_t j = 0; j < m->entries[i].n_new; j++) {
            if (m->entries[i].new_ids[j] == new_id) {
                *out = m->entries[i].original_id;
                return 1;
            }
        }
    }
    return 0;
}

const char *smap_i2i_pass_name(const SmapIrToIr *m) { return m->pass_name; }

/* ===========================================================================
 *  Segment 4: IrToMachineCode
 * =========================================================================== */

typedef struct {
    int64_t ir_id;
    size_t mc_offset, mc_length;
} I2mcEntry;

struct SmapIrToMc {
    I2mcEntry *entries;
    size_t len, cap;
};

SmapIrToMc *smap_i2mc_new(void) { return calloc(1, sizeof(SmapIrToMc)); }

void smap_i2mc_free(SmapIrToMc *mc) {
    if (!mc) return;
    free(mc->entries);
    free(mc);
}

int smap_i2mc_add(SmapIrToMc *mc, int64_t ir_id, size_t mc_offset,
                  size_t mc_length) {
    if (!ensure_cap((void **)&mc->entries, &mc->cap, mc->len + 1,
                    sizeof(I2mcEntry)))
        return -1;
    I2mcEntry *e = &mc->entries[mc->len];
    e->ir_id = ir_id;
    e->mc_offset = mc_offset;
    e->mc_length = mc_length;
    mc->len++;
    return 0;
}

int smap_i2mc_lookup_by_ir_id(const SmapIrToMc *mc, int64_t ir_id,
                              size_t *offset_out, size_t *length_out) {
    for (size_t i = 0; i < mc->len; i++) {
        if (mc->entries[i].ir_id == ir_id) {
            *offset_out = mc->entries[i].mc_offset;
            *length_out = mc->entries[i].mc_length;
            return 1;
        }
    }
    return 0;
}

int smap_i2mc_lookup_by_mc_offset(const SmapIrToMc *mc, size_t offset,
                                  int64_t *ir_id_out) {
    for (size_t i = 0; i < mc->len; i++) {
        size_t start = mc->entries[i].mc_offset;
        size_t end = start + mc->entries[i].mc_length; /* caller's ranges are sane */
        if (offset >= start && offset < end) {
            *ir_id_out = mc->entries[i].ir_id;
            return 1;
        }
    }
    return 0;
}

/* ===========================================================================
 *  SourceMapChain
 * =========================================================================== */

struct SmapChain {
    SmapSourceToAst *s2a;
    SmapAstToIr *a2i;
    SmapIrToIr **passes;
    size_t n_pass, cap_pass;
    SmapIrToMc *mc; /* NULL until a backend is set */
};

SmapChain *smap_chain_new(void) {
    SmapChain *c = calloc(1, sizeof(SmapChain));
    if (!c) return NULL;
    c->s2a = smap_s2a_new();
    c->a2i = smap_a2i_new();
    if (!c->s2a || !c->a2i) {
        smap_s2a_free(c->s2a);
        smap_a2i_free(c->a2i);
        free(c);
        return NULL;
    }
    return c;
}

void smap_chain_free(SmapChain *c) {
    if (!c) return;
    smap_s2a_free(c->s2a);
    smap_a2i_free(c->a2i);
    for (size_t i = 0; i < c->n_pass; i++) smap_i2i_free(c->passes[i]);
    free(c->passes);
    smap_i2mc_free(c->mc);
    free(c);
}

SmapSourceToAst *smap_chain_source_to_ast(SmapChain *c) { return c->s2a; }
SmapAstToIr *smap_chain_ast_to_ir(SmapChain *c) { return c->a2i; }

void smap_chain_set_machine_code(SmapChain *c, SmapIrToMc *mc) {
    smap_i2mc_free(c->mc);
    c->mc = mc;
}

int smap_chain_add_optimizer_pass(SmapChain *c, SmapIrToIr *segment) {
    if (!ensure_cap((void **)&c->passes, &c->cap_pass, c->n_pass + 1,
                    sizeof(SmapIrToIr *))) {
        smap_i2i_free(segment); /* took ownership; free on failure */
        return -1;
    }
    c->passes[c->n_pass++] = segment;
    return 0;
}

int smap_chain_source_to_mc(const SmapChain *c, const SmapPosition *pos,
                            SmapMcEntry **out, size_t *count_out) {
    *out = NULL;
    *count_out = 0;
    if (!c->mc) return 0; /* no backend yet */

    /* Step 1: source position → AST node ID (match file + line + column). */
    size_t ast_node_id = 0;
    int found = 0;
    for (size_t i = 0; i < c->s2a->len; i++) {
        const SmapPosition *e = &c->s2a->entries[i].pos;
        if (strcmp(e->file, pos->file) == 0 && e->line == pos->line &&
            e->column == pos->column) {
            ast_node_id = c->s2a->entries[i].ast_node_id;
            found = 1;
            break;
        }
    }
    if (!found) return 0;

    /* Step 2: AST node ID → IR instruction IDs. */
    size_t n_ir = 0;
    const int64_t *ir_ids =
        smap_a2i_lookup_by_ast_node_id(c->a2i, ast_node_id, &n_ir);
    if (!ir_ids) return 0;

    int64_t *current = NULL;
    size_t cur_len = 0, cur_cap = 0;
    if (!ensure_cap((void **)&current, &cur_cap, n_ir, sizeof(int64_t)))
        return -1;
    if (n_ir) memcpy(current, ir_ids, n_ir * sizeof(int64_t));
    cur_len = n_ir;

    /* Step 3: follow the IR IDs through each optimiser pass. */
    for (size_t p = 0; p < c->n_pass; p++) {
        const SmapIrToIr *pass = c->passes[p];
        int64_t *next = NULL;
        size_t next_len = 0, next_cap = 0;
        for (size_t k = 0; k < cur_len; k++) {
            int64_t id = current[k];
            if (smap_i2i_is_deleted(pass, id)) continue;
            size_t nn = 0;
            const int64_t *nids =
                smap_i2i_lookup_by_original_id(pass, id, &nn);
            if (nids) {
                if (!ensure_cap((void **)&next, &next_cap, next_len + nn,
                                sizeof(int64_t))) {
                    free(next);
                    free(current);
                    return -1;
                }
                memcpy(next + next_len, nids, nn * sizeof(int64_t));
                next_len += nn;
            }
        }
        free(current);
        current = next;
        cur_len = next_len;
        cur_cap = next_cap;
    }
    (void)cur_cap;

    if (cur_len == 0) {
        free(current);
        return 0;
    }

    /* Step 4: final IR IDs → machine-code entries. */
    SmapMcEntry *res = NULL;
    size_t res_len = 0, res_cap = 0;
    for (size_t k = 0; k < cur_len; k++) {
        int64_t id = current[k];
        size_t off = 0, len = 0;
        if (smap_i2mc_lookup_by_ir_id(c->mc, id, &off, &len)) {
            if (!ensure_cap((void **)&res, &res_cap, res_len + 1,
                            sizeof(SmapMcEntry))) {
                free(res);
                free(current);
                return -1;
            }
            res[res_len].ir_id = id;
            res[res_len].mc_offset = off;
            res[res_len].mc_length = len;
            res_len++;
        }
    }
    free(current);

    if (res_len == 0) {
        free(res);
        return 0;
    }
    *out = res;
    *count_out = res_len;
    return 1;
}

const SmapPosition *smap_chain_mc_to_source(const SmapChain *c,
                                            size_t mc_offset) {
    if (!c->mc) return NULL;

    /* Step 1: MC offset → IR ID. */
    int64_t current_id = 0;
    if (!smap_i2mc_lookup_by_mc_offset(c->mc, mc_offset, &current_id))
        return NULL;

    /* Step 2: follow back through the optimiser passes in reverse order. */
    for (size_t p = c->n_pass; p-- > 0;) {
        int64_t orig = 0;
        if (!smap_i2i_lookup_by_new_id(c->passes[p], current_id, &orig))
            return NULL;
        current_id = orig;
    }

    /* Step 3: IR ID → AST node ID. */
    size_t ast_node_id = 0;
    if (!smap_a2i_lookup_by_ir_id(c->a2i, current_id, &ast_node_id))
        return NULL;

    /* Step 4: AST node ID → source position. */
    return smap_s2a_lookup_by_node_id(c->s2a, ast_node_id);
}

/*
 * vcd_writer.c — implementation of the VCD writer (see vcd_writer.h). A faithful
 * port of the Rust `vcd-writer` crate: the same header preamble, base-94
 * identifier allocation, $var/$scope/$dumpvars emission, and value-change
 * formatting (binary for vectors, a single bit for scalars, r<n> for reals).
 */
#include "vcd_writer.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strlen, strcmp, memcpy */

typedef struct {
    char *var_id;
    char *name;
    uint32_t width;
    char *kind;
} VarDef;

typedef struct {
    char *var_id;
    int64_t value;
} LastVal;

struct VcdWriter {
    char *timescale;
    char *buf;
    size_t buf_len;
    size_t buf_cap;
    size_t id_next;
    int defs_ended;
    int has_time;
    uint64_t cur_time;
    LastVal *lasts;
    size_t nlasts, lasts_cap;
    VarDef *defs;
    size_t ndefs, defs_cap;
    size_t scope_depth;
    int ok;
};

/* ---- buffer emission -------------------------------------------------- */

static void emit_n(VcdWriter *w, const char *s, size_t n) {
    if (!w->ok) {
        return;
    }
    if (n > (size_t)-1 - 1 - w->buf_len) {
        w->ok = 0;
        return;
    }
    if (w->buf_len + n + 1 > w->buf_cap) {
        size_t need = w->buf_len + n + 1;
        size_t ncap = w->buf_cap ? w->buf_cap : 64;
        char *nd;
        while (ncap < need) {
            if (ncap > (size_t)-1 / 2) {
                ncap = need;
                break;
            }
            ncap *= 2;
        }
        nd = realloc(w->buf, ncap);
        if (!nd) {
            w->ok = 0;
            return;
        }
        w->buf = nd;
        w->buf_cap = ncap;
    }
    memcpy(w->buf + w->buf_len, s, n);
    w->buf_len += n;
    w->buf[w->buf_len] = '\0';
}

static void emit(VcdWriter *w, const char *s) { emit_n(w, s, strlen(s)); }

static void emit_u32(VcdWriter *w, uint32_t v) {
    char tmp[16];
    snprintf(tmp, sizeof tmp, "%lu", (unsigned long)v);
    emit(w, tmp);
}

static void emit_u64(VcdWriter *w, uint64_t v) {
    char tmp[24];
    snprintf(tmp, sizeof tmp, "%llu", (unsigned long long)v);
    emit(w, tmp);
}

static void emit_i64(VcdWriter *w, int64_t v) {
    char tmp[24];
    snprintf(tmp, sizeof tmp, "%lld", (long long)v);
    emit(w, tmp);
}

/* Emit the minimal binary representation of `m` (no leading zeros; "0" if 0). */
static void emit_binary(VcdWriter *w, uint64_t m) {
    char tmp[64];
    int n = 0;
    if (m == 0) {
        emit(w, "0");
        return;
    }
    while (m) {
        tmp[n++] = (m & 1) ? '1' : '0';
        m >>= 1;
    }
    {
        char rev[64];
        int i;
        for (i = 0; i < n; i++) {
            rev[i] = tmp[n - 1 - i];
        }
        emit_n(w, rev, (size_t)n);
    }
}

/* ---- small helpers ---------------------------------------------------- */

static char *dup_str(const char *s) {
    size_t n = strlen(s);
    char *p = malloc(n + 1);
    if (p) {
        memcpy(p, s, n + 1);
    }
    return p;
}

static const VarDef *find_def(const VcdWriter *w, const char *var_id) {
    size_t i;
    for (i = 0; i < w->ndefs; i++) {
        if (strcmp(w->defs[i].var_id, var_id) == 0) {
            return &w->defs[i];
        }
    }
    return NULL;
}

/* Look up the last recorded value; returns 1 and writes *out if present. */
static int last_get(const VcdWriter *w, const char *var_id, int64_t *out) {
    size_t i;
    for (i = 0; i < w->nlasts; i++) {
        if (strcmp(w->lasts[i].var_id, var_id) == 0) {
            *out = w->lasts[i].value;
            return 1;
        }
    }
    return 0;
}

static void last_set(VcdWriter *w, const char *var_id, int64_t value) {
    size_t i;
    for (i = 0; i < w->nlasts; i++) {
        if (strcmp(w->lasts[i].var_id, var_id) == 0) {
            w->lasts[i].value = value;
            return;
        }
    }
    if (w->nlasts == w->lasts_cap) {
        size_t nc = w->lasts_cap ? w->lasts_cap * 2 : 8;
        LastVal *nl = realloc(w->lasts, nc * sizeof *nl);
        if (!nl) {
            w->ok = 0;
            return;
        }
        w->lasts = nl;
        w->lasts_cap = nc;
    }
    w->lasts[w->nlasts].var_id = dup_str(var_id);
    if (!w->lasts[w->nlasts].var_id) {
        w->ok = 0;
        return;
    }
    w->lasts[w->nlasts].value = value;
    w->nlasts++;
}

/* Bijective base-94 identifier over '!'..'~'. Writes into out (>= 16). */
static int id_alloc(VcdWriter *w, char *out, size_t out_len) {
    size_t n = w->id_next++;
    char tmp[16];
    int len = 0;
    for (;;) {
        tmp[len++] = (char)('!' + (int)(n % 94));
        n /= 94;
        if (n == 0) {
            break;
        }
        n -= 1;
    }
    if ((size_t)len + 1 > out_len) {
        return 0;
    }
    memcpy(out, tmp, (size_t)len);
    out[len] = '\0';
    return 1;
}

/* ---- public: construction --------------------------------------------- */

VcdWriter *vcd_new(const char *timescale) {
    VcdWriter *w = calloc(1, sizeof *w);
    if (!w) {
        return NULL;
    }
    w->ok = 1;
    w->timescale = dup_str(timescale ? timescale : "");
    if (!w->timescale) {
        free(w);
        return NULL;
    }
    emit(w, "$date 2026-06-13 00:00:00 UTC $end\n");
    emit(w, "$version Silicon-Stack VCD Writer 0.1.0 $end\n");
    emit(w, "$timescale ");
    emit(w, w->timescale);
    emit(w, " $end\n");
    return w;
}

void vcd_free(VcdWriter *w) {
    size_t i;
    if (!w) {
        return;
    }
    for (i = 0; i < w->ndefs; i++) {
        free(w->defs[i].var_id);
        free(w->defs[i].name);
        free(w->defs[i].kind);
    }
    free(w->defs);
    for (i = 0; i < w->nlasts; i++) {
        free(w->lasts[i].var_id);
    }
    free(w->lasts);
    free(w->timescale);
    free(w->buf);
    free(w);
}

int vcd_ok(const VcdWriter *w) { return w->ok; }

/* ---- public: header --------------------------------------------------- */

void vcd_open_scope_kind(VcdWriter *w, const char *name, const char *kind) {
    emit(w, "$scope ");
    emit(w, kind);
    emit(w, " ");
    emit(w, name);
    emit(w, " $end\n");
    w->scope_depth++;
}

void vcd_open_scope(VcdWriter *w, const char *name) {
    vcd_open_scope_kind(w, name, "module");
}

void vcd_close_scope(VcdWriter *w) {
    emit(w, "$upscope $end\n");
    if (w->scope_depth > 0) {
        w->scope_depth--;
    }
}

int vcd_declare(VcdWriter *w, const char *name, uint32_t width, const char *kind,
                char *id_out, size_t id_out_len) {
    char id[16];
    if (!id_alloc(w, id, sizeof id)) {
        return 0;
    }
    if (strlen(id) + 1 > id_out_len) {
        return 0;
    }
    strcpy(id_out, id);

    emit(w, "$var ");
    emit(w, kind);
    emit(w, " ");
    emit_u32(w, width);
    emit(w, " ");
    emit(w, id);
    emit(w, " ");
    emit(w, name);
    if (width > 1) {
        emit(w, " [");
        emit_u32(w, width - 1);
        emit(w, ":0]");
    }
    emit(w, " $end\n");

    if (w->ndefs == w->defs_cap) {
        size_t nc = w->defs_cap ? w->defs_cap * 2 : 8;
        VarDef *nd = realloc(w->defs, nc * sizeof *nd);
        if (!nd) {
            w->ok = 0;
            return 0;
        }
        w->defs = nd;
        w->defs_cap = nc;
    }
    w->defs[w->ndefs].var_id = dup_str(id);
    w->defs[w->ndefs].name = dup_str(name);
    w->defs[w->ndefs].kind = dup_str(kind);
    if (!w->defs[w->ndefs].var_id || !w->defs[w->ndefs].name ||
        !w->defs[w->ndefs].kind) {
        free(w->defs[w->ndefs].var_id);
        free(w->defs[w->ndefs].name);
        free(w->defs[w->ndefs].kind);
        w->ok = 0;
        return 0;
    }
    w->defs[w->ndefs].width = width;
    w->ndefs++;
    return 1;
}

void vcd_end_definitions(VcdWriter *w) {
    while (w->scope_depth > 0) {
        vcd_close_scope(w);
    }
    emit(w, "$enddefinitions $end\n");
    w->defs_ended = 1;
}

/* ---- public: body ----------------------------------------------------- */

void vcd_time(VcdWriter *w, uint64_t t) {
    if (!w->defs_ended) {
        vcd_end_definitions(w);
    }
    if (!w->has_time || w->cur_time != t) {
        emit(w, "#");
        emit_u64(w, t);
        emit(w, "\n");
        w->has_time = 1;
        w->cur_time = t;
    }
}

/* Emit one formatted value change line for `var_id`. */
static void format_value_change(VcdWriter *w, const char *var_id,
                                int64_t value) {
    const VarDef *def = find_def(w, var_id);
    if (!def) {
        return;
    }
    if (strcmp(def->kind, "real") == 0) {
        emit(w, "r");
        emit_i64(w, value);
        emit(w, " ");
        emit(w, var_id);
        emit(w, "\n");
        return;
    }
    if (def->width == 1) {
        emit(w, (value & 1) ? "1" : "0");
        emit(w, var_id);
        emit(w, "\n");
        return;
    }
    {
        uint64_t mask = def->width >= 64 ? ~(uint64_t)0
                                         : (((uint64_t)1 << def->width) - 1);
        uint64_t masked = (uint64_t)value & mask;
        emit(w, "b");
        emit_binary(w, masked);
        emit(w, " ");
        emit(w, var_id);
        emit(w, "\n");
    }
}

void vcd_value_change(VcdWriter *w, const char *var_id, int64_t value) {
    int64_t prev;
    if (last_get(w, var_id, &prev) && prev == value) {
        return;
    }
    last_set(w, var_id, value);
    format_value_change(w, var_id, value);
}

void vcd_value_change_at(VcdWriter *w, uint64_t t, const char *var_id,
                         int64_t value) {
    vcd_time(w, t);
    vcd_value_change(w, var_id, value);
}

void vcd_dump_initial(VcdWriter *w, const char *const *ids,
                      const int64_t *values, size_t n) {
    size_t i;
    if (!w->has_time) {
        vcd_time(w, 0);
    }
    emit(w, "$dumpvars\n");
    for (i = 0; i < w->ndefs; i++) {
        const char *id = w->defs[i].var_id;
        int64_t v = 0;
        size_t j;
        for (j = 0; j < n; j++) {
            if (strcmp(ids[j], id) == 0) {
                v = values[j];
                break;
            }
        }
        format_value_change(w, id, v);
        last_set(w, id, v);
    }
    emit(w, "$end\n");
}

const char *vcd_text(const VcdWriter *w) {
    return w->buf ? w->buf : "";
}

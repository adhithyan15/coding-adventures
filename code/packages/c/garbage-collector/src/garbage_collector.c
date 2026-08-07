/*
 * garbage_collector.c — implementation of the pure-ISO C mark-and-sweep GC.
 * ========================================================================
 *
 * The heap is a grow-only array of object slots. Address `A` lives in slot
 * `A - 0x10000`; a swept object leaves a NULL slot but the slot (hence the
 * address) is never reused, so addresses stay monotonic — matching the Rust
 * crate's incrementing `next_address`.
 */
#include "garbage_collector.h"

#include <stdlib.h> /* malloc, realloc, free, calloc */
#include <string.h> /* strcmp, strlen, memcpy */

#define GC_BASE ((size_t)0x10000)

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

/* ── Root values ───────────────────────────────────────────────────────────*/

GcValue gc_val_int(int64_t v) {
    GcValue x;
    x.kind = GC_VAL_INT;
    x.as.i = v;
    return x;
}
GcValue gc_val_address(size_t addr) {
    GcValue x;
    x.kind = GC_VAL_ADDRESS;
    x.as.address = addr;
    return x;
}
GcValue gc_val_bool(int b) {
    GcValue x;
    x.kind = GC_VAL_BOOL;
    x.as.b = b ? 1 : 0;
    return x;
}
GcValue gc_val_nil(void) {
    GcValue x;
    x.kind = GC_VAL_NIL;
    x.as.i = 0;
    return x;
}
GcValue gc_val_str(const char *s) {
    GcValue x;
    x.kind = GC_VAL_STR;
    x.as.str = str_dup(s);
    return x;
}
GcValue gc_val_list(const GcValue *items, size_t n) {
    GcValue x;
    x.kind = GC_VAL_LIST;
    x.as.list.items = NULL;
    x.as.list.n = 0;
    if (n > 0) {
        x.as.list.items = (GcValue *)malloc(n * sizeof(GcValue));
        if (x.as.list.items != NULL) {
            for (size_t i = 0; i < n; i++) {
                /* deep-copy each item (strings/lists own storage) */
                if (items[i].kind == GC_VAL_STR)
                    x.as.list.items[i] = gc_val_str(items[i].as.str);
                else if (items[i].kind == GC_VAL_LIST)
                    x.as.list.items[i] =
                        gc_val_list(items[i].as.list.items, items[i].as.list.n);
                else
                    x.as.list.items[i] = items[i];
            }
            x.as.list.n = n;
        }
    }
    return x;
}
void gc_value_free(GcValue *v) {
    if (v == NULL) return;
    if (v->kind == GC_VAL_STR) {
        free(v->as.str);
        v->as.str = NULL;
    } else if (v->kind == GC_VAL_LIST) {
        for (size_t i = 0; i < v->as.list.n; i++)
            gc_value_free(&v->as.list.items[i]);
        free(v->as.list.items);
        v->as.list.items = NULL;
        v->as.list.n = 0;
    }
}

/* ── Heap objects ──────────────────────────────────────────────────────────*/

typedef enum { OBJ_CONS, OBJ_SYMBOL, OBJ_CLOSURE } ObjKind;

typedef struct {
    char *key;
    int64_t val;
} EnvEntry;

struct GcObject {
    ObjKind kind;
    int marked;
    union {
        struct {
            int64_t car, cdr;
        } cons;
        struct {
            char *name;
        } symbol;
        struct {
            char *code;
            EnvEntry *env;
            size_t n_env;
            char **params;
            size_t n_params;
        } closure;
    } as;
};

static GcObject *obj_alloc(ObjKind kind) {
    GcObject *o = (GcObject *)calloc(1, sizeof(GcObject));
    if (o != NULL) o->kind = kind;
    return o;
}

GcObject *gc_cons_new(int64_t car, int64_t cdr) {
    GcObject *o = obj_alloc(OBJ_CONS);
    if (o != NULL) {
        o->as.cons.car = car;
        o->as.cons.cdr = cdr;
    }
    return o;
}

GcObject *gc_symbol_new(const char *name) {
    GcObject *o = obj_alloc(OBJ_SYMBOL);
    if (o == NULL) return NULL;
    o->as.symbol.name = str_dup(name);
    if (o->as.symbol.name == NULL) {
        free(o);
        return NULL;
    }
    return o;
}

GcObject *gc_closure_new(const char *code, const char *const *env_keys,
                         const int64_t *env_vals, size_t n_env,
                         const char *const *params, size_t n_params) {
    GcObject *o = obj_alloc(OBJ_CLOSURE);
    if (o == NULL) return NULL;
    o->as.closure.code = str_dup(code);
    if (o->as.closure.code == NULL) goto fail;
    if (n_env > 0) {
        o->as.closure.env = (EnvEntry *)calloc(n_env, sizeof(EnvEntry));
        if (o->as.closure.env == NULL) goto fail;
    }
    o->as.closure.n_env = n_env;
    for (size_t i = 0; i < n_env; i++) {
        o->as.closure.env[i].key = str_dup(env_keys[i]);
        o->as.closure.env[i].val = env_vals[i];
        if (o->as.closure.env[i].key == NULL) goto fail;
    }
    if (n_params > 0) {
        o->as.closure.params = (char **)calloc(n_params, sizeof(char *));
        if (o->as.closure.params == NULL) goto fail;
    }
    o->as.closure.n_params = n_params;
    for (size_t i = 0; i < n_params; i++) {
        o->as.closure.params[i] = str_dup(params[i]);
        if (o->as.closure.params[i] == NULL) goto fail;
    }
    return o;
fail:
    gc_object_free(o);
    return NULL;
}

void gc_object_free(GcObject *o) {
    if (o == NULL) return;
    switch (o->kind) {
        case OBJ_SYMBOL:
            free(o->as.symbol.name);
            break;
        case OBJ_CLOSURE:
            free(o->as.closure.code);
            for (size_t i = 0; i < o->as.closure.n_env; i++)
                free(o->as.closure.env[i].key);
            free(o->as.closure.env);
            for (size_t i = 0; i < o->as.closure.n_params; i++)
                free(o->as.closure.params[i]);
            free(o->as.closure.params);
            break;
        case OBJ_CONS:
            break;
    }
    free(o);
}

const char *gc_object_type_name(const GcObject *o) {
    switch (o->kind) {
        case OBJ_CONS: return "ConsCell";
        case OBJ_SYMBOL: return "Symbol";
        case OBJ_CLOSURE: return "LispClosure";
    }
    return "?";
}

/* Collect the heap addresses this object references. Returns a malloc'd array
 * (NULL if none) and writes the count to *n_out. */
size_t *gc_object_references(const GcObject *o, size_t *n_out) {
    *n_out = 0;
    if (o->kind == OBJ_CONS) {
        size_t *r = (size_t *)malloc(2 * sizeof(size_t));
        if (r == NULL) return NULL;
        size_t k = 0;
        if (o->as.cons.car >= 0) r[k++] = (size_t)o->as.cons.car;
        if (o->as.cons.cdr >= 0) r[k++] = (size_t)o->as.cons.cdr;
        *n_out = k;
        return r;
    }
    if (o->kind == OBJ_CLOSURE && o->as.closure.n_env > 0) {
        size_t *r = (size_t *)malloc(o->as.closure.n_env * sizeof(size_t));
        if (r == NULL) return NULL;
        size_t k = 0;
        for (size_t i = 0; i < o->as.closure.n_env; i++)
            if (o->as.closure.env[i].val >= 0)
                r[k++] = (size_t)o->as.closure.env[i].val;
        *n_out = k;
        return r;
    }
    return NULL;
}

/* ── The collector ─────────────────────────────────────────────────────────*/

struct GcHeap {
    GcObject **slots; /* slot i ↔ address GC_BASE + i; NULL == freed/empty */
    size_t n_slots, cap_slots;
    size_t total_allocations, total_collections, total_freed;
};

GcHeap *gc_new(void) { return (GcHeap *)calloc(1, sizeof(GcHeap)); }

void gc_free(GcHeap *gc) {
    if (gc == NULL) return;
    for (size_t i = 0; i < gc->n_slots; i++) gc_object_free(gc->slots[i]);
    free(gc->slots);
    free(gc);
}

size_t gc_allocate(GcHeap *gc, GcObject *obj) {
    if (obj == NULL) return 0;
    if (gc->n_slots == gc->cap_slots) {
        size_t nc = gc->cap_slots ? gc->cap_slots : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(GcObject *)) {
            gc_object_free(obj);
            return 0;
        }
        nc *= 2;
        GcObject **ns = (GcObject **)realloc(gc->slots, nc * sizeof(GcObject *));
        if (ns == NULL) {
            gc_object_free(obj);
            return 0;
        }
        gc->slots = ns;
        gc->cap_slots = nc;
    }
    size_t addr = GC_BASE + gc->n_slots;
    gc->slots[gc->n_slots++] = obj;
    gc->total_allocations++;
    return addr;
}

static GcObject *slot_at(const GcHeap *gc, size_t address) {
    if (address < GC_BASE) return NULL;
    size_t idx = address - GC_BASE;
    if (idx >= gc->n_slots) return NULL;
    return gc->slots[idx];
}

const GcObject *gc_deref(const GcHeap *gc, size_t address) {
    return slot_at(gc, address);
}

int gc_is_valid_address(const GcHeap *gc, size_t address) {
    return slot_at(gc, address) != NULL;
}

static void mark_address(GcHeap *gc, size_t address) {
    GcObject *o = slot_at(gc, address);
    if (o == NULL || o->marked) return;
    size_t nrefs = 0;
    size_t *refs = gc_object_references(o, &nrefs);
    o->marked = 1;
    for (size_t i = 0; i < nrefs; i++) mark_address(gc, refs[i]);
    free(refs);
}

static void mark_value(GcHeap *gc, const GcValue *v) {
    switch (v->kind) {
        case GC_VAL_ADDRESS:
            mark_address(gc, v->as.address);
            break;
        case GC_VAL_INT:
            /* An integer might be a heap address; mark_address ignores it if
             * it is not a live slot. */
            mark_address(gc, (size_t)v->as.i);
            break;
        case GC_VAL_LIST:
            for (size_t i = 0; i < v->as.list.n; i++)
                mark_value(gc, &v->as.list.items[i]);
            break;
        default:
            break; /* Str, Bool, Nil are not heap references */
    }
}

size_t gc_collect(GcHeap *gc, const GcValue *roots, size_t n_roots) {
    gc->total_collections++;
    for (size_t i = 0; i < n_roots; i++) mark_value(gc, &roots[i]);

    size_t freed = 0;
    for (size_t idx = 0; idx < gc->n_slots; idx++) {
        GcObject *o = gc->slots[idx];
        if (o == NULL) continue;
        if (!o->marked) {
            gc_object_free(o);
            gc->slots[idx] = NULL;
            freed++;
        } else {
            o->marked = 0;
        }
    }
    gc->total_freed += freed;
    return freed;
}

size_t gc_heap_size(const GcHeap *gc) {
    size_t live = 0;
    for (size_t i = 0; i < gc->n_slots; i++)
        if (gc->slots[i] != NULL) live++;
    return live;
}

GcStats gc_stats(const GcHeap *gc) {
    GcStats s;
    s.total_allocations = gc->total_allocations;
    s.total_collections = gc->total_collections;
    s.total_freed = gc->total_freed;
    s.heap_size = gc_heap_size(gc);
    return s;
}

/* ── Symbol table ──────────────────────────────────────────────────────────*/

typedef struct {
    char *name;
    size_t addr;
} SymEntry;

struct GcSymbolTable {
    GcHeap *gc; /* borrowed */
    SymEntry *entries;
    size_t n, cap;
};

GcSymbolTable *gc_symbol_table_new(GcHeap *gc) {
    GcSymbolTable *t = (GcSymbolTable *)calloc(1, sizeof(GcSymbolTable));
    if (t != NULL) t->gc = gc;
    return t;
}

void gc_symbol_table_free(GcSymbolTable *t) {
    if (t == NULL) return;
    for (size_t i = 0; i < t->n; i++) free(t->entries[i].name);
    free(t->entries);
    free(t);
}

static SymEntry *sym_find(const GcSymbolTable *t, const char *name) {
    for (size_t i = 0; i < t->n; i++)
        if (strcmp(t->entries[i].name, name) == 0) return &t->entries[i];
    return NULL;
}

size_t gc_symbol_table_intern(GcSymbolTable *t, const char *name) {
    SymEntry *e = sym_find(t, name);
    if (e != NULL && gc_is_valid_address(t->gc, e->addr)) return e->addr;

    size_t addr = gc_allocate(t->gc, gc_symbol_new(name));
    if (e != NULL) {
        e->addr = addr; /* refresh a stale (collected) binding */
        return addr;
    }
    if (t->n == t->cap) {
        size_t nc = t->cap ? t->cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(SymEntry)) return addr;
        nc *= 2;
        SymEntry *ne = (SymEntry *)realloc(t->entries, nc * sizeof(SymEntry));
        if (ne == NULL) return addr;
        t->entries = ne;
        t->cap = nc;
    }
    t->entries[t->n].name = str_dup(name);
    t->entries[t->n].addr = addr;
    if (t->entries[t->n].name != NULL) t->n++;
    return addr;
}

int gc_symbol_table_lookup(const GcSymbolTable *t, const char *name,
                           size_t *out_addr) {
    SymEntry *e = sym_find(t, name);
    if (e != NULL && gc_is_valid_address(t->gc, e->addr)) {
        if (out_addr != NULL) *out_addr = e->addr;
        return 1;
    }
    return 0;
}

size_t gc_symbol_table_count(const GcSymbolTable *t) {
    size_t live = 0;
    for (size_t i = 0; i < t->n; i++)
        if (gc_is_valid_address(t->gc, t->entries[i].addr)) live++;
    return live;
}

int gc_symbol_table_contains(const GcSymbolTable *t, const char *name) {
    size_t addr;
    return gc_symbol_table_lookup(t, name, &addr);
}

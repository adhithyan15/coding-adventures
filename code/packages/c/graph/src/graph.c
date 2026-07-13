/*
 * graph.c — an undirected weighted graph, pure ISO C17.
 * =====================================================
 *
 * See graph.h for the design. Internally everything hangs off a single
 * *ordered adjacency* model:
 *
 *   adj : ordered map  name -> (ordered map  neighbor -> weight)
 *
 * plus three property stores (graph-level, per-node, per-edge). Every map is a
 * sorted dynamic array with binary-search lookup, so iteration is in ascending
 * key order — exactly Rust's `BTreeMap`. Node names are compared with `strcmp`,
 * whose C-standard "as unsigned char" semantics match Rust's byte-wise `&str`
 * ordering.
 *
 * Growth helpers cap the doubling loop so `cap * sizeof(T)` can never overflow
 * `size_t`; every allocation failure surfaces as GRAPH_ERR_OUT_OF_MEMORY.
 */
#include "graph.h"

#include <float.h>  /* DBL_MAX (our +infinity sentinel) */
#include <stdint.h> /* uint64_t */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strcmp, strlen, memcpy */

/* +infinity sentinel for Dijkstra distances. Only ever stored, never enqueued,
 * so it never participates in an arithmetic sum. */
#define GRAPH_INF DBL_MAX

/* ── Owned-string helper ────────────────────────────────────────────────────*/
static char *sdup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p) {
        memcpy(p, s, n);
    }
    return p;
}

/* ── Rust f64::total_cmp — a total order over all doubles ────────────────────*/
static int total_cmp(double a, double b) {
    uint64_t ua, ub;
    memcpy(&ua, &a, sizeof ua);
    memcpy(&ub, &b, sizeof ub);
    ua ^= ((uint64_t)0 - (ua >> 63)) | 0x8000000000000000ULL;
    ub ^= ((uint64_t)0 - (ub >> 63)) | 0x8000000000000000ULL;
    if (ua < ub) return -1;
    if (ua > ub) return 1;
    return 0;
}

/* ── Generic capacity growth (guards size_t overflow) ───────────────────────*/
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

/* ── PropVal (internal, owns its string) ────────────────────────────────────*/
typedef struct {
    GraphPropKind kind;
    char *s;
    double n;
    int b;
} PropVal;

static void propval_free(PropVal *v) {
    free(v->s);
    v->s = NULL;
}

/* Copy a public GraphPropValue into an owned PropVal. Returns 0 on OOM. */
static int propval_from_public(const GraphPropValue *in, PropVal *out) {
    out->kind = in->kind;
    out->s = NULL;
    out->n = in->n;
    out->b = in->b;
    if (in->kind == GRAPH_PROP_STRING) {
        out->s = sdup(in->s ? in->s : "");
        if (!out->s) {
            return 0;
        }
    }
    return 1;
}

/* Borrowed public view of an owned PropVal. */
static GraphPropValue propval_to_public(const PropVal *v) {
    GraphPropValue out;
    out.kind = v->kind;
    out.s = v->s;
    out.n = v->n;
    out.b = v->b;
    return out;
}

/* ── PropBag: ordered map  key -> PropVal ───────────────────────────────────*/
typedef struct {
    char *key;
    PropVal val;
} PBEnt;
typedef struct {
    PBEnt *data;
    size_t len, cap;
} PropBag;

/* Binary search: returns 1 if found (index in *idx), else 0 (*idx = insert
 * position). */
static int pbag_search(const PropBag *b, const char *key, size_t *idx) {
    size_t lo = 0, hi = b->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(b->data[mid].key, key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static int pbag_set(PropBag *b, const char *key, const GraphPropValue *value) {
    size_t idx;
    PropVal nv;
    if (!propval_from_public(value, &nv)) {
        return 0;
    }
    if (pbag_search(b, key, &idx)) {
        propval_free(&b->data[idx].val);
        b->data[idx].val = nv;
        return 1;
    }
    if (!grow((void **)&b->data, &b->cap, b->len + 1, sizeof(PBEnt))) {
        propval_free(&nv);
        return 0;
    }
    {
        char *k = sdup(key);
        if (!k) {
            propval_free(&nv);
            return 0;
        }
        memmove(&b->data[idx + 1], &b->data[idx],
                (b->len - idx) * sizeof(PBEnt));
        b->data[idx].key = k;
        b->data[idx].val = nv;
        b->len++;
    }
    return 1;
}

static void pbag_remove(PropBag *b, const char *key) {
    size_t idx;
    if (pbag_search(b, key, &idx)) {
        free(b->data[idx].key);
        propval_free(&b->data[idx].val);
        memmove(&b->data[idx], &b->data[idx + 1],
                (b->len - idx - 1) * sizeof(PBEnt));
        b->len--;
    }
}

static const PropVal *pbag_get(const PropBag *b, const char *key) {
    size_t idx;
    if (pbag_search(b, key, &idx)) {
        return &b->data[idx].val;
    }
    return NULL;
}

static void pbag_free(PropBag *b) {
    size_t i;
    for (i = 0; i < b->len; i++) {
        free(b->data[i].key);
        propval_free(&b->data[i].val);
    }
    free(b->data);
    b->data = NULL;
    b->len = b->cap = 0;
}

/* ── WMap: ordered map  key -> double ───────────────────────────────────────*/
typedef struct {
    char *key;
    double val;
} WEnt;
typedef struct {
    WEnt *data;
    size_t len, cap;
} WMap;

static int wmap_search(const WMap *m, const char *key, size_t *idx) {
    size_t lo = 0, hi = m->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(m->data[mid].key, key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static int wmap_set(WMap *m, const char *key, double val) {
    size_t idx;
    if (wmap_search(m, key, &idx)) {
        m->data[idx].val = val;
        return 1;
    }
    if (!grow((void **)&m->data, &m->cap, m->len + 1, sizeof(WEnt))) {
        return 0;
    }
    {
        char *k = sdup(key);
        if (!k) {
            return 0;
        }
        memmove(&m->data[idx + 1], &m->data[idx],
                (m->len - idx) * sizeof(WEnt));
        m->data[idx].key = k;
        m->data[idx].val = val;
        m->len++;
    }
    return 1;
}

static void wmap_remove(WMap *m, const char *key) {
    size_t idx;
    if (wmap_search(m, key, &idx)) {
        free(m->data[idx].key);
        memmove(&m->data[idx], &m->data[idx + 1],
                (m->len - idx - 1) * sizeof(WEnt));
        m->len--;
    }
}

static int wmap_get(const WMap *m, const char *key, double *out) {
    size_t idx;
    if (wmap_search(m, key, &idx)) {
        *out = m->data[idx].val;
        return 1;
    }
    return 0;
}

static void wmap_free(WMap *m) {
    size_t i;
    for (i = 0; i < m->len; i++) {
        free(m->data[i].key);
    }
    free(m->data);
    m->data = NULL;
    m->len = m->cap = 0;
}

/* ── Adj: ordered map  name -> WMap ─────────────────────────────────────────*/
typedef struct {
    char *key;
    WMap val;
} AEnt;
typedef struct {
    AEnt *data;
    size_t len, cap;
} Adj;

static int adj_search(const Adj *a, const char *key, size_t *idx) {
    size_t lo = 0, hi = a->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(a->data[mid].key, key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

/* Get the neighbor map for `key`, creating an empty one if absent. NULL on OOM. */
static WMap *adj_entry(Adj *a, const char *key) {
    size_t idx;
    if (adj_search(a, key, &idx)) {
        return &a->data[idx].val;
    }
    if (!grow((void **)&a->data, &a->cap, a->len + 1, sizeof(AEnt))) {
        return NULL;
    }
    {
        char *k = sdup(key);
        if (!k) {
            return NULL;
        }
        memmove(&a->data[idx + 1], &a->data[idx],
                (a->len - idx) * sizeof(AEnt));
        a->data[idx].key = k;
        a->data[idx].val.data = NULL;
        a->data[idx].val.len = a->data[idx].val.cap = 0;
        a->len++;
        return &a->data[idx].val;
    }
}

static WMap *adj_get(const Adj *a, const char *key) {
    size_t idx;
    if (adj_search(a, key, &idx)) {
        return (WMap *)&a->data[idx].val;
    }
    return NULL;
}

static void adj_remove(Adj *a, const char *key) {
    size_t idx;
    if (adj_search(a, key, &idx)) {
        free(a->data[idx].key);
        wmap_free(&a->data[idx].val);
        memmove(&a->data[idx], &a->data[idx + 1],
                (a->len - idx - 1) * sizeof(AEnt));
        a->len--;
    }
}

/* ── NodeProps: ordered map  name -> PropBag ────────────────────────────────*/
typedef struct {
    char *key;
    PropBag val;
} NPEnt;
typedef struct {
    NPEnt *data;
    size_t len, cap;
} NodeProps;

static int np_search(const NodeProps *n, const char *key, size_t *idx) {
    size_t lo = 0, hi = n->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(n->data[mid].key, key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static PropBag *np_entry(NodeProps *n, const char *key) {
    size_t idx;
    if (np_search(n, key, &idx)) {
        return &n->data[idx].val;
    }
    if (!grow((void **)&n->data, &n->cap, n->len + 1, sizeof(NPEnt))) {
        return NULL;
    }
    {
        char *k = sdup(key);
        if (!k) {
            return NULL;
        }
        memmove(&n->data[idx + 1], &n->data[idx],
                (n->len - idx) * sizeof(NPEnt));
        n->data[idx].key = k;
        n->data[idx].val.data = NULL;
        n->data[idx].val.len = n->data[idx].val.cap = 0;
        n->len++;
        return &n->data[idx].val;
    }
}

static PropBag *np_get(const NodeProps *n, const char *key) {
    size_t idx;
    if (np_search(n, key, &idx)) {
        return (PropBag *)&n->data[idx].val;
    }
    return NULL;
}

static void np_remove(NodeProps *n, const char *key) {
    size_t idx;
    if (np_search(n, key, &idx)) {
        free(n->data[idx].key);
        pbag_free(&n->data[idx].val);
        memmove(&n->data[idx], &n->data[idx + 1],
                (n->len - idx - 1) * sizeof(NPEnt));
        n->len--;
    }
}

/* ── EdgeProps: ordered map  (a,b) -> PropBag, keyed by canonical pair ───────*/
typedef struct {
    char *a;
    char *b;
    PropBag val;
} EPEnt;
typedef struct {
    EPEnt *data;
    size_t len, cap;
} EdgeProps;

static int pair_cmp(const char *a1, const char *b1, const char *a2,
                    const char *b2) {
    int c = strcmp(a1, a2);
    if (c != 0) {
        return c;
    }
    return strcmp(b1, b2);
}

static int ep_search(const EdgeProps *e, const char *a, const char *b,
                     size_t *idx) {
    size_t lo = 0, hi = e->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = pair_cmp(e->data[mid].a, e->data[mid].b, a, b);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static PropBag *ep_entry(EdgeProps *e, const char *a, const char *b) {
    size_t idx;
    if (ep_search(e, a, b, &idx)) {
        return &e->data[idx].val;
    }
    if (!grow((void **)&e->data, &e->cap, e->len + 1, sizeof(EPEnt))) {
        return NULL;
    }
    {
        char *ka = sdup(a);
        char *kb = sdup(b);
        if (!ka || !kb) {
            free(ka);
            free(kb);
            return NULL;
        }
        memmove(&e->data[idx + 1], &e->data[idx],
                (e->len - idx) * sizeof(EPEnt));
        e->data[idx].a = ka;
        e->data[idx].b = kb;
        e->data[idx].val.data = NULL;
        e->data[idx].val.len = e->data[idx].val.cap = 0;
        e->len++;
        return &e->data[idx].val;
    }
}

static PropBag *ep_get(const EdgeProps *e, const char *a, const char *b) {
    size_t idx;
    if (ep_search(e, a, b, &idx)) {
        return (PropBag *)&e->data[idx].val;
    }
    return NULL;
}

static void ep_remove(EdgeProps *e, const char *a, const char *b) {
    size_t idx;
    if (ep_search(e, a, b, &idx)) {
        free(e->data[idx].a);
        free(e->data[idx].b);
        pbag_free(&e->data[idx].val);
        memmove(&e->data[idx], &e->data[idx + 1],
                (e->len - idx - 1) * sizeof(EPEnt));
        e->len--;
    }
}

/* ── The graph ──────────────────────────────────────────────────────────────*/
struct Graph {
    GraphRepr repr;
    Adj adj;
    PropBag graph_props;
    NodeProps node_props;
    EdgeProps edge_props;
};

/* canonical_endpoints: order (left, right) so the first is <= the second. The
 * returned pointers alias the inputs (no allocation). */
static void canonical(const char *left, const char *right, const char **first,
                      const char **second) {
    if (strcmp(left, right) <= 0) {
        *first = left;
        *second = right;
    } else {
        *first = right;
        *second = left;
    }
}

Graph *graph_new(GraphRepr repr) {
    Graph *g = (Graph *)calloc(1, sizeof(Graph));
    if (g) {
        g->repr = repr;
    }
    return g;
}

void graph_free(Graph *g) {
    size_t i;
    if (!g) {
        return;
    }
    for (i = 0; i < g->adj.len; i++) {
        free(g->adj.data[i].key);
        wmap_free(&g->adj.data[i].val);
    }
    free(g->adj.data);
    pbag_free(&g->graph_props);
    for (i = 0; i < g->node_props.len; i++) {
        free(g->node_props.data[i].key);
        pbag_free(&g->node_props.data[i].val);
    }
    free(g->node_props.data);
    for (i = 0; i < g->edge_props.len; i++) {
        free(g->edge_props.data[i].a);
        free(g->edge_props.data[i].b);
        pbag_free(&g->edge_props.data[i].val);
    }
    free(g->edge_props.data);
    free(g);
}

GraphRepr graph_repr(const Graph *g) { return g->repr; }
size_t graph_size(const Graph *g) { return g->adj.len; }
int graph_has_node(const Graph *g, const char *node) {
    return adj_get(&g->adj, node) != NULL;
}

/* ── Property value constructors / equality ─────────────────────────────────*/
GraphPropValue graph_prop_string(const char *s) {
    GraphPropValue v;
    v.kind = GRAPH_PROP_STRING;
    v.s = s;
    v.n = 0.0;
    v.b = 0;
    return v;
}
GraphPropValue graph_prop_number(double n) {
    GraphPropValue v;
    v.kind = GRAPH_PROP_NUMBER;
    v.s = NULL;
    v.n = n;
    v.b = 0;
    return v;
}
GraphPropValue graph_prop_bool(int b) {
    GraphPropValue v;
    v.kind = GRAPH_PROP_BOOL;
    v.s = NULL;
    v.n = 0.0;
    v.b = b ? 1 : 0;
    return v;
}
GraphPropValue graph_prop_null(void) {
    GraphPropValue v;
    v.kind = GRAPH_PROP_NULL;
    v.s = NULL;
    v.n = 0.0;
    v.b = 0;
    return v;
}
int graph_prop_equal(GraphPropValue a, GraphPropValue b) {
    if (a.kind != b.kind) {
        return 0;
    }
    switch (a.kind) {
        case GRAPH_PROP_STRING:
            return strcmp(a.s ? a.s : "", b.s ? b.s : "") == 0;
        case GRAPH_PROP_NUMBER:
            return a.n == b.n;
        case GRAPH_PROP_BOOL:
            return (a.b ? 1 : 0) == (b.b ? 1 : 0);
        case GRAPH_PROP_NULL:
            return 1;
    }
    return 0;
}

/* ── Nodes ──────────────────────────────────────────────────────────────────*/
GraphStatus graph_add_node(Graph *g, const char *node) {
    if (!adj_entry(&g->adj, node)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    /* Ensure a (possibly empty) property bag exists, mirroring the Rust
     * node_properties.entry(node).or_default(). */
    if (!np_entry(&g->node_props, node)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    return GRAPH_OK;
}

GraphStatus graph_add_node_props(Graph *g, const char *node,
                                 const GraphPropEntry *props, size_t nprops) {
    PropBag *bag;
    size_t i;
    if (!adj_entry(&g->adj, node)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    bag = np_entry(&g->node_props, node);
    if (!bag) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < nprops; i++) {
        if (!pbag_set(bag, props[i].key, &props[i].value)) {
            return GRAPH_ERR_OUT_OF_MEMORY;
        }
    }
    return GRAPH_OK;
}

GraphStatus graph_remove_node(Graph *g, const char *node) {
    WMap *neighbors = adj_get(&g->adj, node);
    size_t i;
    WMap snapshot;
    if (!neighbors) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    /* Snapshot neighbor keys (removal mutates adj). */
    snapshot = *neighbors;
    for (i = 0; i < snapshot.len; i++) {
        const char *nb = snapshot.data[i].key;
        const char *fa, *fb;
        WMap *nbmap = adj_get(&g->adj, nb);
        if (nbmap && strcmp(nb, node) != 0) {
            wmap_remove(nbmap, node);
        }
        canonical(node, nb, &fa, &fb);
        ep_remove(&g->edge_props, fa, fb);
    }
    adj_remove(&g->adj, node);
    np_remove(&g->node_props, node);
    return GRAPH_OK;
}

/* ── Edges ──────────────────────────────────────────────────────────────────*/
GraphStatus graph_add_edge(Graph *g, const char *left, const char *right,
                           double weight) {
    return graph_add_edge_props(g, left, right, weight, NULL, 0);
}

GraphStatus graph_add_edge_props(Graph *g, const char *left, const char *right,
                                 double weight, const GraphPropEntry *props,
                                 size_t nprops) {
    WMap *lm, *rm;
    PropBag *bag;
    const char *fa, *fb;
    GraphPropValue wv;
    size_t i;
    GraphStatus st = graph_add_node(g, left);
    if (st != GRAPH_OK) {
        return st;
    }
    st = graph_add_node(g, right);
    if (st != GRAPH_OK) {
        return st;
    }
    lm = adj_get(&g->adj, left);
    rm = adj_get(&g->adj, right);
    if (!lm || !rm) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    if (!wmap_set(lm, right, weight)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    if (!wmap_set(rm, left, weight)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    canonical(left, right, &fa, &fb);
    bag = ep_entry(&g->edge_props, fa, fb);
    if (!bag) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < nprops; i++) {
        if (!pbag_set(bag, props[i].key, &props[i].value)) {
            return GRAPH_ERR_OUT_OF_MEMORY;
        }
    }
    wv = graph_prop_number(weight);
    if (!pbag_set(bag, "weight", &wv)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    return GRAPH_OK;
}

GraphStatus graph_remove_edge(Graph *g, const char *left, const char *right) {
    WMap *lm = adj_get(&g->adj, left);
    const char *fa, *fb;
    double tmp;
    if (!lm || !wmap_get(lm, right, &tmp)) {
        return GRAPH_ERR_EDGE_NOT_FOUND;
    }
    wmap_remove(lm, right);
    {
        WMap *rm = adj_get(&g->adj, right);
        if (rm) {
            wmap_remove(rm, left);
        }
    }
    canonical(left, right, &fa, &fb);
    ep_remove(&g->edge_props, fa, fb);
    return GRAPH_OK;
}

int graph_has_edge(const Graph *g, const char *left, const char *right) {
    WMap *lm = adj_get(&g->adj, left);
    double tmp;
    return lm && wmap_get(lm, right, &tmp);
}

GraphStatus graph_edge_weight(const Graph *g, const char *left,
                              const char *right, double *out) {
    WMap *lm = adj_get(&g->adj, left);
    if (lm && wmap_get(lm, right, out)) {
        return GRAPH_OK;
    }
    return GRAPH_ERR_EDGE_NOT_FOUND;
}

/* Internal: set an existing edge's weight (both directions). */
static GraphStatus set_edge_weight(Graph *g, const char *left,
                                   const char *right, double weight) {
    WMap *lm, *rm;
    double tmp;
    lm = adj_get(&g->adj, left);
    if (!lm || !wmap_get(lm, right, &tmp)) {
        return GRAPH_ERR_EDGE_NOT_FOUND;
    }
    rm = adj_get(&g->adj, right);
    if (!rm) {
        return GRAPH_ERR_EDGE_NOT_FOUND;
    }
    if (!wmap_set(lm, right, weight) || !wmap_set(rm, left, weight)) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    return GRAPH_OK;
}

/* ── nodes() / neighbors() / edges() ────────────────────────────────────────*/
GraphStatus graph_nodes(const Graph *g, GraphStrList *out) {
    size_t i;
    out->items = NULL;
    out->len = 0;
    if (g->adj.len == 0) {
        return GRAPH_OK;
    }
    out->items = (char **)malloc(g->adj.len * sizeof(char *));
    if (!out->items) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    /* adj keys are already sorted. */
    for (i = 0; i < g->adj.len; i++) {
        out->items[i] = sdup(g->adj.data[i].key);
        if (!out->items[i]) {
            out->len = i;
            graph_str_list_free(out);
            return GRAPH_ERR_OUT_OF_MEMORY;
        }
    }
    out->len = g->adj.len;
    return GRAPH_OK;
}

GraphStatus graph_neighbors(const Graph *g, const char *node,
                            GraphStrList *out) {
    WMap *m = adj_get(&g->adj, node);
    size_t i;
    out->items = NULL;
    out->len = 0;
    if (!m) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    if (m->len == 0) {
        return GRAPH_OK;
    }
    out->items = (char **)malloc(m->len * sizeof(char *));
    if (!out->items) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < m->len; i++) {
        out->items[i] = sdup(m->data[i].key);
        if (!out->items[i]) {
            out->len = i;
            graph_str_list_free(out);
            return GRAPH_ERR_OUT_OF_MEMORY;
        }
    }
    out->len = m->len;
    return GRAPH_OK;
}

GraphStatus graph_degree(const Graph *g, const char *node, size_t *out) {
    WMap *m = adj_get(&g->adj, node);
    if (!m) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    *out = m->len;
    return GRAPH_OK;
}

/* Sort an edge list by (total_cmp weight, left, right). Simple insertion sort —
 * edge counts here are small and it keeps the port dependency-free. */
static void sort_edges(GraphEdge *e, size_t n) {
    size_t i, j;
    for (i = 1; i < n; i++) {
        GraphEdge key = e[i];
        j = i;
        while (j > 0) {
            GraphEdge *p = &e[j - 1];
            int c = total_cmp(p->weight, key.weight);
            if (c == 0) {
                c = strcmp(p->left, key.left);
            }
            if (c == 0) {
                c = strcmp(p->right, key.right);
            }
            if (c <= 0) {
                break;
            }
            e[j] = e[j - 1];
            j--;
        }
        e[j] = key;
    }
}

GraphStatus graph_edges(const Graph *g, GraphEdgeList *out) {
    size_t i, k, count = 0;
    out->items = NULL;
    out->len = 0;
    /* Each undirected edge (a,b) appears twice in adj; emit it once, in
     * canonical direction (left <= right). */
    for (i = 0; i < g->adj.len; i++) {
        const char *left = g->adj.data[i].key;
        WMap *m = &g->adj.data[i].val;
        for (k = 0; k < m->len; k++) {
            if (strcmp(left, m->data[k].key) <= 0) {
                count++;
            }
        }
    }
    if (count == 0) {
        return GRAPH_OK;
    }
    out->items = (GraphEdge *)malloc(count * sizeof(GraphEdge));
    if (!out->items) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    out->len = 0;
    for (i = 0; i < g->adj.len; i++) {
        const char *left = g->adj.data[i].key;
        WMap *m = &g->adj.data[i].val;
        for (k = 0; k < m->len; k++) {
            const char *right = m->data[k].key;
            if (strcmp(left, right) <= 0) {
                GraphEdge *e = &out->items[out->len];
                e->left = sdup(left);
                e->right = sdup(right);
                e->weight = m->data[k].val;
                if (!e->left || !e->right) {
                    free(e->left);
                    free(e->right);
                    graph_edge_list_free(out);
                    return GRAPH_ERR_OUT_OF_MEMORY;
                }
                out->len++;
            }
        }
    }
    sort_edges(out->items, out->len);
    return GRAPH_OK;
}

/* ── Property accessors ─────────────────────────────────────────────────────*/
GraphStatus graph_set_graph_property(Graph *g, const char *key,
                                     GraphPropValue value) {
    return pbag_set(&g->graph_props, key, &value) ? GRAPH_OK
                                                  : GRAPH_ERR_OUT_OF_MEMORY;
}
void graph_remove_graph_property(Graph *g, const char *key) {
    pbag_remove(&g->graph_props, key);
}
int graph_get_graph_property(const Graph *g, const char *key,
                             GraphPropValue *out) {
    const PropVal *v = pbag_get(&g->graph_props, key);
    if (!v) {
        return 0;
    }
    *out = propval_to_public(v);
    return 1;
}

GraphStatus graph_set_node_property(Graph *g, const char *node, const char *key,
                                    GraphPropValue value) {
    PropBag *bag;
    if (!graph_has_node(g, node)) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    bag = np_entry(&g->node_props, node);
    if (!bag) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    return pbag_set(bag, key, &value) ? GRAPH_OK : GRAPH_ERR_OUT_OF_MEMORY;
}

GraphStatus graph_remove_node_property(Graph *g, const char *node,
                                       const char *key) {
    PropBag *bag;
    if (!graph_has_node(g, node)) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    bag = np_get(&g->node_props, node);
    if (bag) {
        pbag_remove(bag, key);
    }
    return GRAPH_OK;
}

GraphStatus graph_get_node_property(const Graph *g, const char *node,
                                    const char *key, GraphPropValue *out,
                                    int *found) {
    PropBag *bag;
    const PropVal *v;
    if (!graph_has_node(g, node)) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    bag = np_get(&g->node_props, node);
    v = bag ? pbag_get(bag, key) : NULL;
    if (found) {
        *found = v != NULL;
    }
    if (v) {
        *out = propval_to_public(v);
    }
    return GRAPH_OK;
}

GraphStatus graph_set_edge_property(Graph *g, const char *left,
                                    const char *right, const char *key,
                                    GraphPropValue value) {
    const char *fa, *fb;
    PropBag *bag;
    if (!graph_has_edge(g, left, right)) {
        return GRAPH_ERR_EDGE_NOT_FOUND;
    }
    canonical(left, right, &fa, &fb);
    if (strcmp(key, "weight") == 0) {
        GraphStatus st;
        GraphPropValue wv;
        if (value.kind != GRAPH_PROP_NUMBER) {
            return GRAPH_ERR_EDGE_NOT_FOUND;
        }
        st = set_edge_weight(g, left, right, value.n);
        if (st != GRAPH_OK) {
            return st;
        }
        bag = ep_entry(&g->edge_props, fa, fb);
        if (!bag) {
            return GRAPH_ERR_OUT_OF_MEMORY;
        }
        wv = graph_prop_number(value.n);
        return pbag_set(bag, "weight", &wv) ? GRAPH_OK
                                            : GRAPH_ERR_OUT_OF_MEMORY;
    }
    bag = ep_entry(&g->edge_props, fa, fb);
    if (!bag) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    return pbag_set(bag, key, &value) ? GRAPH_OK : GRAPH_ERR_OUT_OF_MEMORY;
}

GraphStatus graph_remove_edge_property(Graph *g, const char *left,
                                       const char *right, const char *key) {
    const char *fa, *fb;
    PropBag *bag;
    if (!graph_has_edge(g, left, right)) {
        return GRAPH_ERR_EDGE_NOT_FOUND;
    }
    canonical(left, right, &fa, &fb);
    if (strcmp(key, "weight") == 0) {
        GraphPropValue wv;
        GraphStatus st = set_edge_weight(g, left, right, 1.0);
        if (st != GRAPH_OK) {
            return st;
        }
        bag = ep_entry(&g->edge_props, fa, fb);
        if (!bag) {
            return GRAPH_ERR_OUT_OF_MEMORY;
        }
        wv = graph_prop_number(1.0);
        return pbag_set(bag, "weight", &wv) ? GRAPH_OK
                                            : GRAPH_ERR_OUT_OF_MEMORY;
    }
    bag = ep_get(&g->edge_props, fa, fb);
    if (bag) {
        pbag_remove(bag, key);
    }
    return GRAPH_OK;
}

GraphStatus graph_get_edge_property(const Graph *g, const char *left,
                                    const char *right, const char *key,
                                    GraphPropValue *out, int *found) {
    const char *fa, *fb;
    PropBag *bag;
    const PropVal *v;
    if (!graph_has_edge(g, left, right)) {
        return GRAPH_ERR_EDGE_NOT_FOUND;
    }
    /* The "weight" property is synthesized from the live edge weight. */
    if (strcmp(key, "weight") == 0) {
        double w;
        (void)graph_edge_weight(g, left, right, &w);
        if (found) {
            *found = 1;
        }
        *out = graph_prop_number(w);
        return GRAPH_OK;
    }
    canonical(left, right, &fa, &fb);
    bag = ep_get(&g->edge_props, fa, fb);
    v = bag ? pbag_get(bag, key) : NULL;
    if (found) {
        *found = v != NULL;
    }
    if (v) {
        *out = propval_to_public(v);
    }
    return GRAPH_OK;
}

/* ── Owned-list destructors ─────────────────────────────────────────────────*/
void graph_str_list_free(GraphStrList *list) {
    size_t i;
    if (!list) {
        return;
    }
    for (i = 0; i < list->len; i++) {
        free(list->items[i]);
    }
    free(list->items);
    list->items = NULL;
    list->len = 0;
}

void graph_edge_list_free(GraphEdgeList *list) {
    size_t i;
    if (!list) {
        return;
    }
    for (i = 0; i < list->len; i++) {
        free(list->items[i].left);
        free(list->items[i].right);
    }
    free(list->items);
    list->items = NULL;
    list->len = 0;
}

void graph_components_free(GraphComponents *comps) {
    size_t i;
    if (!comps) {
        return;
    }
    for (i = 0; i < comps->len; i++) {
        graph_str_list_free(&comps->items[i]);
    }
    free(comps->items);
    comps->items = NULL;
    comps->len = 0;
}

/* ── A sorted string set (BTreeSet) built on a growable array ────────────────*/
typedef struct {
    char **data;
    size_t len, cap;
} SSet;

static int sset_search(const SSet *s, const char *key, size_t *idx) {
    size_t lo = 0, hi = s->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(s->data[mid], key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

/* Returns 1 if newly inserted, 0 if already present, -1 on OOM. */
static int sset_insert(SSet *s, const char *key) {
    size_t idx;
    char *k;
    if (sset_search(s, key, &idx)) {
        return 0;
    }
    if (!grow((void **)&s->data, &s->cap, s->len + 1, sizeof(char *))) {
        return -1;
    }
    k = sdup(key);
    if (!k) {
        return -1;
    }
    memmove(&s->data[idx + 1], &s->data[idx], (s->len - idx) * sizeof(char *));
    s->data[idx] = k;
    s->len++;
    return 1;
}

static int sset_contains(const SSet *s, const char *key) {
    size_t idx;
    return sset_search(s, key, &idx);
}

static void sset_erase(SSet *s, const char *key) {
    size_t idx;
    if (sset_search(s, key, &idx)) {
        free(s->data[idx]);
        memmove(&s->data[idx], &s->data[idx + 1],
                (s->len - idx - 1) * sizeof(char *));
        s->len--;
    }
}

static void sset_free(SSet *s) {
    size_t i;
    for (i = 0; i < s->len; i++) {
        free(s->data[i]);
    }
    free(s->data);
    s->data = NULL;
    s->len = s->cap = 0;
}

/* Reverse an owned-string list in place. */
static void strlist_reverse(GraphStrList *l) {
    size_t a, b;
    if (l->len < 2) {
        return;
    }
    a = 0;
    b = l->len - 1;
    while (a < b) {
        char *t = l->items[a];
        l->items[a] = l->items[b];
        l->items[b] = t;
        a++;
        b--;
    }
}

/* ── Growable owned-string list (used to build results / queues / stacks) ────*/
static int strlist_push(GraphStrList *l, size_t *cap, const char *s) {
    char *k;
    if (!grow((void **)&l->items, cap, l->len + 1, sizeof(char *))) {
        return 0;
    }
    k = sdup(s);
    if (!k) {
        return 0;
    }
    l->items[l->len++] = k;
    return 1;
}

/* ── BFS / DFS ──────────────────────────────────────────────────────────────*/
GraphStatus graph_bfs(const Graph *g, const char *start, GraphStrList *out) {
    SSet visited = {0};
    GraphStrList queue = {0};
    size_t qcap = 0, head = 0, ocap = 0;
    WMap *sm;
    out->items = NULL;
    out->len = 0;
    sm = adj_get(&g->adj, start);
    if (!sm) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    if (sset_insert(&visited, start) < 0 || !strlist_push(&queue, &qcap, start)) {
        goto oom;
    }
    while (head < queue.len) {
        const char *node = queue.items[head++];
        WMap *m = adj_get(&g->adj, node);
        size_t i;
        if (!strlist_push(out, &ocap, node)) {
            goto oom;
        }
        if (!m) {
            continue;
        }
        for (i = 0; i < m->len; i++) {
            const char *nb = m->data[i].key;
            int ins = sset_insert(&visited, nb);
            if (ins < 0) {
                goto oom;
            }
            if (ins == 1 && !strlist_push(&queue, &qcap, nb)) {
                goto oom;
            }
        }
    }
    sset_free(&visited);
    graph_str_list_free(&queue);
    return GRAPH_OK;
oom:
    sset_free(&visited);
    graph_str_list_free(&queue);
    graph_str_list_free(out);
    return GRAPH_ERR_OUT_OF_MEMORY;
}

GraphStatus graph_dfs(const Graph *g, const char *start, GraphStrList *out) {
    SSet visited = {0};
    GraphStrList stack = {0};
    size_t scap = 0, ocap = 0;
    WMap *sm;
    out->items = NULL;
    out->len = 0;
    sm = adj_get(&g->adj, start);
    if (!sm) {
        return GRAPH_ERR_NODE_NOT_FOUND;
    }
    if (!strlist_push(&stack, &scap, start)) {
        goto oom;
    }
    while (stack.len > 0) {
        char *node = stack.items[--stack.len]; /* pop (owned) */
        WMap *m;
        int ins = sset_insert(&visited, node);
        if (ins < 0) {
            free(node);
            goto oom;
        }
        if (ins == 0) { /* already visited */
            free(node);
            continue;
        }
        if (!strlist_push(out, &ocap, node)) {
            free(node);
            goto oom;
        }
        m = adj_get(&g->adj, node);
        free(node);
        if (m) {
            /* push neighbors in reverse so the smallest is popped first */
            size_t i = m->len;
            while (i > 0) {
                const char *nb = m->data[--i].key;
                if (!sset_contains(&visited, nb) &&
                    !strlist_push(&stack, &scap, nb)) {
                    goto oom;
                }
            }
        }
    }
    sset_free(&visited);
    graph_str_list_free(&stack);
    return GRAPH_OK;
oom:
    sset_free(&visited);
    graph_str_list_free(&stack);
    graph_str_list_free(out);
    return GRAPH_ERR_OUT_OF_MEMORY;
}

int graph_is_connected(const Graph *g) {
    GraphStrList visited;
    int connected;
    if (g->adj.len == 0) {
        return 1;
    }
    /* smallest node = first adj key (sorted). */
    if (graph_bfs(g, g->adj.data[0].key, &visited) != GRAPH_OK) {
        return 0;
    }
    connected = visited.len == g->adj.len;
    graph_str_list_free(&visited);
    return connected;
}

GraphStatus graph_connected_components(const Graph *g, GraphComponents *out) {
    SSet remaining = {0};
    size_t i, ocap = 0;
    out->items = NULL;
    out->len = 0;
    for (i = 0; i < g->adj.len; i++) {
        if (sset_insert(&remaining, g->adj.data[i].key) < 0) {
            goto oom;
        }
    }
    while (remaining.len > 0) {
        const char *start = remaining.data[0]; /* smallest */
        GraphStrList component;
        size_t k;
        GraphStatus st = graph_bfs(g, start, &component);
        if (st != GRAPH_OK) {
            goto oom;
        }
        for (k = 0; k < component.len; k++) {
            sset_erase(&remaining, component.items[k]);
        }
        if (!grow((void **)&out->items, &ocap, out->len + 1,
                  sizeof(GraphStrList))) {
            graph_str_list_free(&component);
            goto oom;
        }
        out->items[out->len++] = component;
    }
    sset_free(&remaining);
    return GRAPH_OK;
oom:
    sset_free(&remaining);
    graph_components_free(out);
    return GRAPH_ERR_OUT_OF_MEMORY;
}

/* ── Cycle detection (recursive DFS, faithful to the crate) ─────────────────*/
static int cycle_visit(const Graph *g, const char *node, const char *parent,
                       SSet *visited, int *oom) {
    WMap *m;
    size_t i;
    if (sset_insert(visited, node) < 0) {
        *oom = 1;
        return 0;
    }
    m = adj_get(&g->adj, node);
    if (!m) {
        return 0;
    }
    for (i = 0; i < m->len; i++) {
        const char *nb = m->data[i].key;
        if (!sset_contains(visited, nb)) {
            if (cycle_visit(g, nb, node, visited, oom)) {
                return 1;
            }
            if (*oom) {
                return 0;
            }
        } else if (parent == NULL || strcmp(nb, parent) != 0) {
            return 1;
        }
    }
    return 0;
}

int graph_has_cycle(const Graph *g) {
    SSet visited = {0};
    size_t i;
    int oom = 0, found = 0;
    for (i = 0; i < g->adj.len; i++) {
        const char *node = g->adj.data[i].key;
        if (!sset_contains(&visited, node) &&
            cycle_visit(g, node, NULL, &visited, &oom)) {
            found = 1;
            break;
        }
        if (oom) {
            break;
        }
    }
    sset_free(&visited);
    return found;
}

/* ── Shortest path ──────────────────────────────────────────────────────────*/

/* parent map for the unit-weight BFS variant: key -> optional parent. */
typedef struct {
    char *key;
    char *parent; /* NULL == None */
    int has_parent;
} PPEnt;
typedef struct {
    PPEnt *data;
    size_t len, cap;
} PMap;

static int pmap_search(const PMap *p, const char *key, size_t *idx) {
    size_t lo = 0, hi = p->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(p->data[mid].key, key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static int pmap_set(PMap *p, const char *key, const char *parent) {
    size_t idx;
    char *k, *pp = NULL;
    if (pmap_search(p, key, &idx)) {
        return 1; /* insert-if-absent semantics: keep the first parent */
    }
    if (!grow((void **)&p->data, &p->cap, p->len + 1, sizeof(PPEnt))) {
        return 0;
    }
    k = sdup(key);
    if (!k) {
        return 0;
    }
    if (parent) {
        pp = sdup(parent);
        if (!pp) {
            free(k);
            return 0;
        }
    }
    memmove(&p->data[idx + 1], &p->data[idx], (p->len - idx) * sizeof(PPEnt));
    p->data[idx].key = k;
    p->data[idx].parent = pp;
    p->data[idx].has_parent = parent != NULL;
    p->len++;
    return 1;
}

static int pmap_contains(const PMap *p, const char *key) {
    size_t idx;
    return pmap_search(p, key, &idx);
}

/* Fetch parent: returns 1 if key present (*parent may be NULL for None). */
static int pmap_get(const PMap *p, const char *key, const char **parent) {
    size_t idx;
    if (pmap_search(p, key, &idx)) {
        *parent = p->data[idx].has_parent ? p->data[idx].parent : NULL;
        return 1;
    }
    return 0;
}

static void pmap_free(PMap *p) {
    size_t i;
    for (i = 0; i < p->len; i++) {
        free(p->data[i].key);
        free(p->data[i].parent);
    }
    free(p->data);
    p->data = NULL;
    p->len = p->cap = 0;
}

static GraphStatus bfs_shortest_path(const Graph *g, const char *start,
                                     const char *end, GraphStrList *out) {
    PMap parent = {0};
    GraphStrList queue = {0};
    size_t qcap = 0, head = 0, ocap = 0;
    const char *cur;
    if (!pmap_set(&parent, start, NULL) || !strlist_push(&queue, &qcap, start)) {
        goto oom;
    }
    while (head < queue.len) {
        const char *node = queue.items[head++];
        WMap *m;
        size_t i;
        if (strcmp(node, end) == 0) {
            break;
        }
        m = adj_get(&g->adj, node);
        if (!m) {
            continue;
        }
        for (i = 0; i < m->len; i++) {
            const char *nb = m->data[i].key;
            if (!pmap_contains(&parent, nb)) {
                if (!pmap_set(&parent, nb, node) ||
                    !strlist_push(&queue, &qcap, nb)) {
                    goto oom;
                }
            }
        }
    }
    if (!pmap_contains(&parent, end)) {
        pmap_free(&parent);
        graph_str_list_free(&queue);
        return GRAPH_OK; /* empty path */
    }
    /* Reconstruct end -> start, then reverse. */
    cur = end;
    while (cur) {
        const char *prev = NULL;
        if (!strlist_push(out, &ocap, cur)) {
            goto oom;
        }
        if (!pmap_get(&parent, cur, &prev)) {
            prev = NULL;
        }
        cur = prev;
    }
    strlist_reverse(out);
    pmap_free(&parent);
    graph_str_list_free(&queue);
    return GRAPH_OK;
oom:
    pmap_free(&parent);
    graph_str_list_free(&queue);
    graph_str_list_free(out);
    return GRAPH_ERR_OUT_OF_MEMORY;
}

/* string -> string map for Dijkstra parents. */
typedef struct {
    char *key;
    char *val;
} SSEnt;
typedef struct {
    SSEnt *data;
    size_t len, cap;
} SSMap;

static int ssmap_search(const SSMap *m, const char *key, size_t *idx) {
    size_t lo = 0, hi = m->len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(m->data[mid].key, key);
        if (c == 0) {
            *idx = mid;
            return 1;
        }
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static int ssmap_set(SSMap *m, const char *key, const char *val) {
    size_t idx;
    char *nv;
    if (ssmap_search(m, key, &idx)) {
        nv = sdup(val);
        if (!nv) {
            return 0;
        }
        free(m->data[idx].val);
        m->data[idx].val = nv;
        return 1;
    }
    if (!grow((void **)&m->data, &m->cap, m->len + 1, sizeof(SSEnt))) {
        return 0;
    }
    {
        char *k = sdup(key);
        nv = sdup(val);
        if (!k || !nv) {
            free(k);
            free(nv);
            return 0;
        }
        memmove(&m->data[idx + 1], &m->data[idx],
                (m->len - idx) * sizeof(SSEnt));
        m->data[idx].key = k;
        m->data[idx].val = nv;
        m->len++;
    }
    return 1;
}

static int ssmap_get(const SSMap *m, const char *key, const char **val) {
    size_t idx;
    if (ssmap_search(m, key, &idx)) {
        *val = m->data[idx].val;
        return 1;
    }
    return 0;
}

static void ssmap_free(SSMap *m) {
    size_t i;
    for (i = 0; i < m->len; i++) {
        free(m->data[i].key);
        free(m->data[i].val);
    }
    free(m->data);
    m->data = NULL;
    m->len = m->cap = 0;
}

/* A Dijkstra priority-queue item (sorted lazily, like the Rust crate). */
typedef struct {
    double distance;
    size_t sequence;
    char *node;
} DItem;

static GraphStatus dijkstra_shortest_path(const Graph *g, const char *start,
                                          const char *end, GraphStrList *out) {
    WMap distances = {0};
    SSMap parent = {0};
    DItem *queue = NULL;
    size_t qlen = 0, qcap = 0, sequence = 0, ocap = 0;
    size_t i;
    const char *cur;
    double dend;
    GraphStatus rc = GRAPH_ERR_OUT_OF_MEMORY;

    for (i = 0; i < g->adj.len; i++) {
        if (!wmap_set(&distances, g->adj.data[i].key, GRAPH_INF)) {
            goto done;
        }
    }
    if (!wmap_set(&distances, start, 0.0)) {
        goto done;
    }
    if (!grow((void **)&queue, &qcap, 1, sizeof(DItem))) {
        goto done;
    }
    queue[0].distance = 0.0;
    queue[0].sequence = sequence;
    queue[0].node = sdup(start);
    if (!queue[0].node) {
        goto done;
    }
    qlen = 1;

    while (qlen > 0) {
        /* Select the min (distance, sequence) — a lazy sort/scan. */
        size_t best = 0;
        DItem top;
        WMap *nbrs;
        double distance;
        double curdist;
        for (i = 1; i < qlen; i++) {
            int c = total_cmp(queue[i].distance, queue[best].distance);
            if (c < 0 || (c == 0 && queue[i].sequence < queue[best].sequence)) {
                best = i;
            }
        }
        top = queue[best];
        memmove(&queue[best], &queue[best + 1],
                (qlen - best - 1) * sizeof(DItem));
        qlen--;
        distance = top.distance;

        curdist = GRAPH_INF;
        (void)wmap_get(&distances, top.node, &curdist);
        if (distance > curdist) {
            free(top.node);
            continue;
        }
        if (strcmp(top.node, end) == 0) {
            free(top.node);
            break;
        }
        nbrs = adj_get(&g->adj, top.node);
        if (nbrs) {
            for (i = 0; i < nbrs->len; i++) {
                const char *nb = nbrs->data[i].key;
                double w = nbrs->data[i].val;
                double next = distance + w;
                double known = GRAPH_INF;
                (void)wmap_get(&distances, nb, &known);
                if (next < known) {
                    if (!wmap_set(&distances, nb, next) ||
                        !ssmap_set(&parent, nb, top.node)) {
                        free(top.node);
                        goto done;
                    }
                    sequence++;
                    if (!grow((void **)&queue, &qcap, qlen + 1, sizeof(DItem))) {
                        free(top.node);
                        goto done;
                    }
                    queue[qlen].distance = next;
                    queue[qlen].sequence = sequence;
                    queue[qlen].node = sdup(nb);
                    if (!queue[qlen].node) {
                        free(top.node);
                        goto done;
                    }
                    qlen++;
                }
            }
        }
        free(top.node);
    }

    dend = GRAPH_INF;
    (void)wmap_get(&distances, end, &dend);
    if (dend == GRAPH_INF) {
        rc = GRAPH_OK; /* empty path */
        goto done;
    }
    cur = end;
    for (;;) {
        const char *prev;
        if (!strlist_push(out, &ocap, cur)) {
            goto done;
        }
        if (strcmp(cur, start) == 0) {
            break;
        }
        if (!ssmap_get(&parent, cur, &prev)) {
            graph_str_list_free(out); /* no path */
            rc = GRAPH_OK;
            goto done;
        }
        cur = prev;
    }
    strlist_reverse(out);
    rc = GRAPH_OK;

done:
    for (i = 0; i < qlen; i++) {
        free(queue[i].node);
    }
    free(queue);
    wmap_free(&distances);
    ssmap_free(&parent);
    if (rc != GRAPH_OK) {
        graph_str_list_free(out);
    }
    return rc;
}

GraphStatus graph_shortest_path(const Graph *g, const char *start,
                                const char *end, GraphStrList *out) {
    GraphEdgeList edges;
    int all_unit = 1;
    size_t i;
    out->items = NULL;
    out->len = 0;
    if (!graph_has_node(g, start) || !graph_has_node(g, end)) {
        return GRAPH_OK;
    }
    if (strcmp(start, end) == 0) {
        size_t cap = 0;
        return strlist_push(out, &cap, start) ? GRAPH_OK
                                              : GRAPH_ERR_OUT_OF_MEMORY;
    }
    if (graph_edges(g, &edges) != GRAPH_OK) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < edges.len; i++) {
        if (edges.items[i].weight != 1.0) {
            all_unit = 0;
            break;
        }
    }
    graph_edge_list_free(&edges);
    return all_unit ? bfs_shortest_path(g, start, end, out)
                    : dijkstra_shortest_path(g, start, end, out);
}

/* ── Minimum spanning tree (Kruskal with union-find) ────────────────────────*/

/* Union-find over node names (path compression + union by rank). */
typedef struct {
    SSMap parent; /* node -> parent name */
    WMap rank;    /* node -> rank (stored as double; small integers) */
} UnionFind;

static const char *uf_find(UnionFind *uf, const char *node) {
    const char *p = NULL;
    if (!ssmap_get(&uf->parent, node, &p)) {
        return node;
    }
    if (strcmp(p, node) != 0) {
        const char *root = uf_find(uf, p);
        (void)ssmap_set(&uf->parent, node, root); /* path compression */
        return root;
    }
    return p;
}

static void uf_union(UnionFind *uf, const char *left, const char *right) {
    const char *lr = uf_find(uf, left);
    const char *rr = uf_find(uf, right);
    double lrank = 0.0, rrank = 0.0;
    if (strcmp(lr, rr) == 0) {
        return;
    }
    (void)wmap_get(&uf->rank, lr, &lrank);
    (void)wmap_get(&uf->rank, rr, &rrank);
    if (lrank < rrank) {
        const char *tmp = lr;
        lr = rr;
        rr = tmp;
        {
            double t = lrank;
            lrank = rrank;
            rrank = t;
        }
    }
    (void)ssmap_set(&uf->parent, rr, lr);
    if (lrank == rrank) {
        (void)wmap_set(&uf->rank, lr, lrank + 1.0);
    }
}

GraphStatus graph_minimum_spanning_tree(const Graph *g, GraphEdgeList *out) {
    GraphEdgeList edges;
    UnionFind uf = {0};
    size_t i, ocap = 0, node_count = g->adj.len;
    GraphStatus rc = GRAPH_ERR_OUT_OF_MEMORY;
    out->items = NULL;
    out->len = 0;

    if (graph_edges(g, &edges) != GRAPH_OK) {
        return GRAPH_ERR_OUT_OF_MEMORY;
    }
    if (node_count <= 1 || edges.len == 0) {
        graph_edge_list_free(&edges);
        return GRAPH_OK;
    }
    if (!graph_is_connected(g)) {
        graph_edge_list_free(&edges);
        return GRAPH_ERR_NOT_CONNECTED;
    }
    for (i = 0; i < g->adj.len; i++) {
        const char *n = g->adj.data[i].key;
        if (!ssmap_set(&uf.parent, n, n) || !wmap_set(&uf.rank, n, 0.0)) {
            goto done;
        }
    }
    for (i = 0; i < edges.len; i++) {
        const char *a = edges.items[i].left;
        const char *b = edges.items[i].right;
        if (strcmp(uf_find(&uf, a), uf_find(&uf, b)) != 0) {
            GraphEdge *e;
            uf_union(&uf, a, b);
            if (!grow((void **)&out->items, &ocap, out->len + 1,
                      sizeof(GraphEdge))) {
                goto done;
            }
            e = &out->items[out->len];
            e->left = sdup(a);
            e->right = sdup(b);
            e->weight = edges.items[i].weight;
            if (!e->left || !e->right) {
                free(e->left);
                free(e->right);
                goto done;
            }
            out->len++;
            if (out->len == node_count - 1) {
                break;
            }
        }
    }
    rc = GRAPH_OK;

done:
    graph_edge_list_free(&edges);
    ssmap_free(&uf.parent);
    wmap_free(&uf.rank);
    if (rc != GRAPH_OK) {
        graph_edge_list_free(out);
    }
    return rc;
}

/*
 * neural_network.c — implementation of the neural-network graph IR.
 * ===========================================================================
 * Property bags are small string->value assoc-arrays; the graph keeps its nodes
 * (with per-node bags) and edges in insertion order. See neural_network.h.
 */
#include "neural_network.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *dup_cstr(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) {
        memcpy(out, s, n + 1);
    }
    return out;
}

static int ensure_cap(void **data, size_t *cap, size_t need, size_t elem) {
    if (need <= *cap) {
        return 0;
    }
    size_t nc = *cap ? *cap : 4;
    while (nc < need) {
        if (nc > ((size_t)-1) / 2 / elem) {
            return -1;
        }
        nc *= 2;
    }
    void *nd = realloc(*data, nc * elem);
    if (!nd) {
        return -1;
    }
    *data = nd;
    *cap = nc;
    return 0;
}

/* Finite (not NaN, not +/-inf) without <math.h>: x-x == 0 only for finite x. */
static int is_finite(double x) { return (x - x) == 0.0; }

/* ===========================================================================
 *  NgProperty
 * =========================================================================== */

NgProperty ng_prop_null(void) {
    NgProperty p;
    p.tag = NG_PROP_NULL;
    p.string = NULL;
    p.number = 0.0;
    p.boolean = 0;
    return p;
}
NgProperty ng_prop_number(double n) {
    NgProperty p = ng_prop_null();
    p.tag = NG_PROP_NUMBER;
    p.number = n;
    return p;
}
NgProperty ng_prop_boolean(int b) {
    NgProperty p = ng_prop_null();
    p.tag = NG_PROP_BOOLEAN;
    p.boolean = b ? 1 : 0;
    return p;
}
int ng_prop_string(const char *s, NgProperty *out) {
    char *dup = dup_cstr(s);
    if (!dup) {
        return -1;
    }
    *out = ng_prop_null();
    out->tag = NG_PROP_STRING;
    out->string = dup;
    return 0;
}
int ng_prop_copy(const NgProperty *src, NgProperty *out) {
    *out = *src;
    out->string = NULL;
    if (src->tag == NG_PROP_STRING) {
        out->string = dup_cstr(src->string);
        if (!out->string) {
            return -1;
        }
    }
    return 0;
}
void ng_prop_free(NgProperty *p) {
    if (p && p->tag == NG_PROP_STRING) {
        free(p->string);
        p->string = NULL;
    }
}
int ng_prop_equals(const NgProperty *a, const NgProperty *b) {
    if (a->tag != b->tag) {
        return 0;
    }
    switch (a->tag) {
        case NG_PROP_STRING: return strcmp(a->string, b->string) == 0;
        case NG_PROP_NUMBER: return a->number == b->number;
        case NG_PROP_BOOLEAN: return a->boolean == b->boolean;
        case NG_PROP_NULL: return 1;
    }
    return 0;
}

/* ===========================================================================
 *  PropertyBag
 * =========================================================================== */

void ng_bag_init(NgPropertyBag *bag) {
    bag->entries = NULL;
    bag->n = 0;
    bag->cap = 0;
}

void ng_bag_free(NgPropertyBag *bag) {
    if (!bag) {
        return;
    }
    size_t i;
    for (i = 0; i < bag->n; i++) {
        free(bag->entries[i].key);
        ng_prop_free(&bag->entries[i].val);
    }
    free(bag->entries);
    bag->entries = NULL;
    bag->n = 0;
    bag->cap = 0;
}

int ng_bag_insert(NgPropertyBag *bag, const char *key, NgProperty val) {
    size_t i;
    for (i = 0; i < bag->n; i++) {
        if (strcmp(bag->entries[i].key, key) == 0) {
            ng_prop_free(&bag->entries[i].val);
            bag->entries[i].val = val;
            return 0;
        }
    }
    if (ensure_cap((void **)&bag->entries, &bag->cap, bag->n + 1,
                   sizeof(NgBagEntry)) != 0) {
        ng_prop_free(&val);
        return -1;
    }
    char *k = dup_cstr(key);
    if (!k) {
        ng_prop_free(&val);
        return -1;
    }
    bag->entries[bag->n].key = k;
    bag->entries[bag->n].val = val;
    bag->n++;
    return 0;
}

const NgProperty *ng_bag_get(const NgPropertyBag *bag, const char *key) {
    size_t i;
    for (i = 0; i < bag->n; i++) {
        if (strcmp(bag->entries[i].key, key) == 0) {
            return &bag->entries[i].val;
        }
    }
    return NULL;
}

size_t ng_bag_size(const NgPropertyBag *bag) { return bag->n; }

int ng_bag_extend(NgPropertyBag *dst, const NgPropertyBag *src) {
    if (!src) {
        return 0;
    }
    size_t i;
    for (i = 0; i < src->n; i++) {
        NgProperty v;
        if (ng_prop_copy(&src->entries[i].val, &v) != 0) {
            return -1;
        }
        if (ng_bag_insert(dst, src->entries[i].key, v) != 0) {
            return -1;
        }
    }
    return 0;
}

int ng_bag_copy(const NgPropertyBag *src, NgPropertyBag *out) {
    ng_bag_init(out);
    if (ng_bag_extend(out, src) != 0) {
        ng_bag_free(out);
        return -1;
    }
    return 0;
}

/* ===========================================================================
 *  ActivationKind
 * =========================================================================== */

const char *ng_activation_str(NgActivation a) {
    switch (a) {
        case NG_ACT_RELU: return "relu";
        case NG_ACT_SIGMOID: return "sigmoid";
        case NG_ACT_TANH: return "tanh";
        case NG_ACT_NONE: return "none";
    }
    return "none";
}

/* ===========================================================================
 *  WeightedInput
 * =========================================================================== */

int ng_weighted_input_init(NgWeightedInput *out, const char *from,
                           double weight, const char *edge_id) {
    out->from = dup_cstr(from);
    out->weight = weight;
    out->edge_id = NULL;
    ng_bag_init(&out->properties);
    if (!out->from) {
        return -1;
    }
    if (edge_id) {
        out->edge_id = dup_cstr(edge_id);
        if (!out->edge_id) {
            free(out->from);
            out->from = NULL;
            return -1;
        }
    }
    return 0;
}

void ng_weighted_input_free(NgWeightedInput *wi) {
    if (!wi) {
        return;
    }
    free(wi->from);
    free(wi->edge_id);
    ng_bag_free(&wi->properties);
    wi->from = NULL;
    wi->edge_id = NULL;
}

/* ===========================================================================
 *  NeuralGraph
 * =========================================================================== */

typedef struct {
    char *key;
    NgPropertyBag val;
} NodeEntry;

struct NeuralGraph {
    NgPropertyBag graph_properties;
    NodeEntry *nodes; /* insertion-ordered map: node name -> property bag */
    size_t n_nodes, cap_nodes;
    NgEdge *edges;
    size_t n_edges, cap_edges;
    size_t next_edge_id;
};

static void edge_free(NgEdge *e) {
    free(e->id);
    free(e->from);
    free(e->to);
    ng_bag_free(&e->properties);
}

int ng_new(NeuralGraph **out, const char *name) {
    NeuralGraph *g = calloc(1, sizeof *g);
    if (!g) {
        return -1;
    }
    ng_bag_init(&g->graph_properties);
    NgProperty ver;
    if (ng_prop_string("0", &ver) != 0) {
        free(g);
        return -1;
    }
    if (ng_bag_insert(&g->graph_properties, "nn.version", ver) != 0) {
        ng_bag_free(&g->graph_properties);
        free(g);
        return -1;
    }
    if (name) {
        NgProperty nm;
        if (ng_prop_string(name, &nm) != 0 ||
            ng_bag_insert(&g->graph_properties, "nn.name", nm) != 0) {
            ng_bag_free(&g->graph_properties);
            free(g);
            return -1;
        }
    }
    *out = g;
    return 0;
}

void ng_free(NeuralGraph *g) {
    if (!g) {
        return;
    }
    ng_bag_free(&g->graph_properties);
    size_t i;
    for (i = 0; i < g->n_nodes; i++) {
        free(g->nodes[i].key);
        ng_bag_free(&g->nodes[i].val);
    }
    free(g->nodes);
    for (i = 0; i < g->n_edges; i++) {
        edge_free(&g->edges[i]);
    }
    free(g->edges);
    free(g);
}

const NgPropertyBag *ng_graph_properties(const NeuralGraph *g) {
    return &g->graph_properties;
}
size_t ng_node_count(const NeuralGraph *g) { return g->n_nodes; }
const char *ng_node_at(const NeuralGraph *g, size_t i) {
    return g->nodes[i].key;
}
size_t ng_edge_count(const NeuralGraph *g) { return g->n_edges; }
const NgEdge *ng_edge_at(const NeuralGraph *g, size_t i) {
    return &g->edges[i];
}

static NodeEntry *find_node(NeuralGraph *g, const char *node) {
    size_t i;
    for (i = 0; i < g->n_nodes; i++) {
        if (strcmp(g->nodes[i].key, node) == 0) {
            return &g->nodes[i];
        }
    }
    return NULL;
}

int ng_add_node(NeuralGraph *g, const char *node,
                const NgPropertyBag *properties) {
    NodeEntry *e = find_node(g, node);
    if (!e) {
        if (ensure_cap((void **)&g->nodes, &g->cap_nodes, g->n_nodes + 1,
                       sizeof(NodeEntry)) != 0) {
            return -1;
        }
        char *k = dup_cstr(node);
        if (!k) {
            return -1;
        }
        e = &g->nodes[g->n_nodes];
        e->key = k;
        ng_bag_init(&e->val);
        g->n_nodes++;
    }
    return ng_bag_extend(&e->val, properties);
}

const NgPropertyBag *ng_node_properties(const NeuralGraph *g,
                                        const char *node) {
    size_t i;
    for (i = 0; i < g->n_nodes; i++) {
        if (strcmp(g->nodes[i].key, node) == 0) {
            return &g->nodes[i].val;
        }
    }
    return NULL;
}

NgStatus ng_add_edge(NeuralGraph *g, const char *from, const char *to,
                     double weight, const NgPropertyBag *properties,
                     const char *edge_id, char **out_id) {
    if (ng_add_node(g, from, NULL) != 0 || ng_add_node(g, to, NULL) != 0) {
        return NG_ERR_NOMEM;
    }

    char *id = NULL;
    if (edge_id) {
        id = dup_cstr(edge_id);
    } else {
        char buf[32];
        snprintf(buf, sizeof buf, "e%zu", g->next_edge_id);
        id = dup_cstr(buf);
    }
    if (!id) {
        return NG_ERR_NOMEM;
    }

    NgPropertyBag merged;
    if (ng_bag_copy(properties, &merged) != 0) {
        free(id);
        return NG_ERR_NOMEM;
    }
    if (ng_bag_insert(&merged, "weight", ng_prop_number(weight)) != 0) {
        ng_bag_free(&merged);
        free(id);
        return NG_ERR_NOMEM;
    }

    char *e_from = dup_cstr(from);
    char *e_to = dup_cstr(to);
    if (!e_from || !e_to ||
        ensure_cap((void **)&g->edges, &g->cap_edges, g->n_edges + 1,
                   sizeof(NgEdge)) != 0) {
        free(e_from);
        free(e_to);
        ng_bag_free(&merged);
        free(id);
        return NG_ERR_NOMEM;
    }

    char *out_copy = NULL;
    if (out_id) {
        out_copy = dup_cstr(id);
        if (!out_copy) {
            free(e_from);
            free(e_to);
            ng_bag_free(&merged);
            free(id);
            return NG_ERR_NOMEM;
        }
    }

    NgEdge *e = &g->edges[g->n_edges];
    e->id = id;
    e->from = e_from;
    e->to = e_to;
    e->weight = weight;
    e->properties = merged;
    g->n_edges++;

    /* Mint the id only after the edge is committed (matches Rust ordering). */
    if (!edge_id) {
        g->next_edge_id++;
    }
    if (out_id) {
        *out_id = out_copy;
    }
    return NG_OK;
}

int ng_incoming_edges(const NeuralGraph *g, const char *node,
                      const NgEdge ***out, size_t *count) {
    size_t i, k = 0;
    for (i = 0; i < g->n_edges; i++) {
        if (strcmp(g->edges[i].to, node) == 0) {
            k++;
        }
    }
    if (k == 0) {
        *out = NULL;
        *count = 0;
        return 0;
    }
    const NgEdge **arr = calloc(k, sizeof(NgEdge *));
    if (!arr) {
        return -1;
    }
    size_t j = 0;
    for (i = 0; i < g->n_edges; i++) {
        if (strcmp(g->edges[i].to, node) == 0) {
            arr[j++] = &g->edges[i];
        }
    }
    *out = arr;
    *count = k;
    return 0;
}

/* Insertion-sort node indices by their node name (deterministic tie-break). */
static void sort_indices_by_name(const NeuralGraph *g, size_t *idx, size_t n) {
    size_t i, j;
    for (i = 1; i < n; i++) {
        size_t key = idx[i];
        j = i;
        while (j > 0 &&
               strcmp(g->nodes[idx[j - 1]].key, g->nodes[key].key) > 0) {
            idx[j] = idx[j - 1];
            j--;
        }
        idx[j] = key;
    }
}

static size_t node_index(const NeuralGraph *g, const char *name) {
    size_t i;
    for (i = 0; i < g->n_nodes; i++) {
        if (strcmp(g->nodes[i].key, name) == 0) {
            return i;
        }
    }
    return (size_t)-1; /* unreachable: edge endpoints are always nodes */
}

NgStatus ng_topological_sort(const NeuralGraph *g, char ***out, size_t *count) {
    size_t nn = g->n_nodes;
    *out = NULL;
    *count = 0;
    if (nn == 0) {
        return NG_OK; /* empty graph -> empty order */
    }

    size_t *indeg = calloc(nn, sizeof(size_t));
    size_t *queue = malloc(nn * sizeof(size_t)); /* FIFO of node indices */
    size_t *order = malloc(nn * sizeof(size_t));
    size_t *released = malloc(nn * sizeof(size_t));
    if (!indeg || !queue || !order || !released) {
        free(indeg);
        free(queue);
        free(order);
        free(released);
        return NG_ERR_NOMEM;
    }

    size_t i;
    for (i = 0; i < g->n_edges; i++) {
        indeg[node_index(g, g->edges[i].to)]++;
    }

    size_t qhead = 0, qtail = 0, n_order = 0;
    /* Seed with the indegree-0 nodes, sorted by name. */
    for (i = 0; i < nn; i++) {
        if (indeg[i] == 0) {
            queue[qtail++] = i;
        }
    }
    sort_indices_by_name(g, queue, qtail);

    while (qhead < qtail) {
        size_t u = queue[qhead++];
        order[n_order++] = u;
        size_t n_rel = 0;
        for (i = 0; i < g->n_edges; i++) {
            if (strcmp(g->edges[i].from, g->nodes[u].key) != 0) {
                continue;
            }
            size_t v = node_index(g, g->edges[i].to);
            if (indeg[v] > 0) {
                indeg[v]--;
                if (indeg[v] == 0) {
                    released[n_rel++] = v;
                }
            }
        }
        sort_indices_by_name(g, released, n_rel);
        size_t r;
        for (r = 0; r < n_rel; r++) {
            queue[qtail++] = released[r];
        }
    }

    NgStatus status = NG_OK;
    if (n_order != nn) {
        status = NG_ERR_CYCLE;
    } else {
        char **names = calloc(nn, sizeof(char *));
        if (!names) {
            status = NG_ERR_NOMEM;
        } else {
            for (i = 0; i < nn; i++) {
                names[i] = dup_cstr(g->nodes[order[i]].key);
                if (!names[i]) {
                    size_t j;
                    for (j = 0; j < i; j++) {
                        free(names[j]);
                    }
                    free(names);
                    names = NULL;
                    status = NG_ERR_NOMEM;
                    break;
                }
            }
            if (names) {
                *out = names;
                *count = nn;
            }
        }
    }

    free(indeg);
    free(queue);
    free(order);
    free(released);
    return status;
}

void ng_string_array_free(char **arr, size_t count) {
    if (!arr) {
        return;
    }
    size_t i;
    for (i = 0; i < count; i++) {
        free(arr[i]);
    }
    free(arr);
}

/* ===========================================================================
 *  Layer builders
 * =========================================================================== */

/* Build a bag = copy(base) then insert one string property; then add_node. */
static int add_node_with(NeuralGraph *g, const char *node,
                         const NgPropertyBag *base, const char *k1,
                         NgProperty v1, const char *k2, NgProperty v2) {
    NgPropertyBag bag;
    if (ng_bag_copy(base, &bag) != 0) {
        ng_prop_free(&v1);
        if (k2) {
            ng_prop_free(&v2);
        }
        return -1;
    }
    if (ng_bag_insert(&bag, k1, v1) != 0) {
        if (k2) {
            ng_prop_free(&v2);
        }
        ng_bag_free(&bag);
        return -1;
    }
    if (k2 && ng_bag_insert(&bag, k2, v2) != 0) {
        ng_bag_free(&bag);
        return -1;
    }
    int rc = ng_add_node(g, node, &bag);
    ng_bag_free(&bag);
    return rc;
}

int ng_add_input(NeuralGraph *g, const char *node, const char *input_name,
                 const NgPropertyBag *properties) {
    NgProperty op, in;
    if (ng_prop_string("input", &op) != 0) {
        return -1;
    }
    if (ng_prop_string(input_name, &in) != 0) {
        ng_prop_free(&op);
        return -1;
    }
    return add_node_with(g, node, properties, "nn.op", op, "nn.input", in);
}

NgStatus ng_add_constant(NeuralGraph *g, const char *node, double value,
                         const NgPropertyBag *properties) {
    if (!is_finite(value)) {
        return NG_ERR_NOT_FINITE;
    }
    NgProperty op;
    if (ng_prop_string("constant", &op) != 0) {
        return NG_ERR_NOMEM;
    }
    return add_node_with(g, node, properties, "nn.op", op, "nn.value",
                         ng_prop_number(value)) == 0
               ? NG_OK
               : NG_ERR_NOMEM;
}

int ng_add_weighted_sum(NeuralGraph *g, const char *node,
                        const NgWeightedInput *inputs, size_t n_inputs,
                        const NgPropertyBag *properties) {
    NgProperty op;
    if (ng_prop_string("weighted_sum", &op) != 0) {
        return -1;
    }
    if (add_node_with(g, node, properties, "nn.op", op, NULL,
                      ng_prop_null()) != 0) {
        return -1;
    }
    size_t i;
    for (i = 0; i < n_inputs; i++) {
        if (ng_add_edge(g, inputs[i].from, node, inputs[i].weight,
                        &inputs[i].properties, inputs[i].edge_id, NULL) !=
            NG_OK) {
            return -1;
        }
    }
    return 0;
}

NgStatus ng_add_activation(NeuralGraph *g, const char *node, const char *input,
                           NgActivation activation,
                           const NgPropertyBag *properties, const char *edge_id,
                           char **out_id) {
    NgProperty op, act;
    if (ng_prop_string("activation", &op) != 0) {
        return NG_ERR_NOMEM;
    }
    if (ng_prop_string(ng_activation_str(activation), &act) != 0) {
        ng_prop_free(&op);
        return NG_ERR_NOMEM;
    }
    if (add_node_with(g, node, properties, "nn.op", op, "nn.activation", act) !=
        0) {
        return NG_ERR_NOMEM;
    }
    return ng_add_edge(g, input, node, 1.0, NULL, edge_id, out_id);
}

NgStatus ng_add_output(NeuralGraph *g, const char *node, const char *input,
                       const char *output_name, const NgPropertyBag *properties,
                       const char *edge_id, char **out_id) {
    NgProperty op, outp;
    if (ng_prop_string("output", &op) != 0) {
        return NG_ERR_NOMEM;
    }
    if (ng_prop_string(output_name, &outp) != 0) {
        ng_prop_free(&op);
        return NG_ERR_NOMEM;
    }
    if (add_node_with(g, node, properties, "nn.op", op, "nn.output", outp) !=
        0) {
        return NG_ERR_NOMEM;
    }
    return ng_add_edge(g, input, node, 1.0, NULL, edge_id, out_id);
}

/* ---- XOR network -------------------------------------------------------- */

/* A single-entry bag {key: String(value)} (Rust's `prop` helper). */
static int prop_bag(NgPropertyBag *out, const char *key, const char *value) {
    ng_bag_init(out);
    NgProperty v;
    if (ng_prop_string(value, &v) != 0) {
        return -1;
    }
    return ng_bag_insert(out, key, v);
}

/* A WeightedInput with an empty property bag. */
static int wi(NgWeightedInput *out, const char *from, double weight,
              const char *edge_id) {
    return ng_weighted_input_init(out, from, weight, edge_id);
}

int ng_create_xor_network(NeuralGraph **out, const char *name) {
    NeuralGraph *g;
    if (ng_new(&g, name) != 0) {
        return -1;
    }
    int ok = 0;
    NgPropertyBag hidden, output, bias_role, empty;
    ng_bag_init(&hidden);
    ng_bag_init(&output);
    ng_bag_init(&bias_role);
    ng_bag_init(&empty);
    NgWeightedInput in[3];
    size_t i;
    for (i = 0; i < 3; i++) {
        in[i].from = NULL;
        in[i].edge_id = NULL;
        ng_bag_init(&in[i].properties);
    }

    if (prop_bag(&hidden, "nn.layer", "hidden") != 0 ||
        prop_bag(&output, "nn.layer", "output") != 0 ||
        prop_bag(&bias_role, "nn.role", "bias") != 0) {
        goto done;
    }

#define WSUM(node, layer, a, aw, aid, b, bw, bid, c, cw, cid)                \
    do {                                                                     \
        if (wi(&in[0], a, aw, aid) != 0 || wi(&in[1], b, bw, bid) != 0 ||    \
            wi(&in[2], c, cw, cid) != 0) {                                   \
            goto done;                                                       \
        }                                                                    \
        if (ng_add_weighted_sum(g, node, in, 3, &layer) != 0) {             \
            goto done;                                                       \
        }                                                                    \
        for (i = 0; i < 3; i++) {                                            \
            ng_weighted_input_free(&in[i]);                                  \
        }                                                                    \
    } while (0)

    if (ng_add_input(g, "x0", "x0", &empty) != 0 ||
        ng_add_input(g, "x1", "x1", &empty) != 0 ||
        ng_add_constant(g, "bias", 1.0, &bias_role) != NG_OK) {
        goto done;
    }
    WSUM("h_or_sum", hidden, "x0", 20.0, "x0_to_h_or", "x1", 20.0, "x1_to_h_or",
         "bias", -10.0, "bias_to_h_or");
    if (ng_add_activation(g, "h_or", "h_or_sum", NG_ACT_SIGMOID, &hidden,
                          "h_or_sum_to_h_or", NULL) != NG_OK) {
        goto done;
    }
    WSUM("h_nand_sum", hidden, "x0", -20.0, "x0_to_h_nand", "x1", -20.0,
         "x1_to_h_nand", "bias", 30.0, "bias_to_h_nand");
    if (ng_add_activation(g, "h_nand", "h_nand_sum", NG_ACT_SIGMOID, &hidden,
                          "h_nand_sum_to_h_nand", NULL) != NG_OK) {
        goto done;
    }
    WSUM("out_sum", output, "h_or", 20.0, "h_or_to_out", "h_nand", 20.0,
         "h_nand_to_out", "bias", -30.0, "bias_to_out");
    if (ng_add_activation(g, "out_activation", "out_sum", NG_ACT_SIGMOID,
                          &output, "out_sum_to_activation", NULL) != NG_OK) {
        goto done;
    }
    if (ng_add_output(g, "out", "out_activation", "prediction", &output,
                      "activation_to_out", NULL) != NG_OK) {
        goto done;
    }
    ok = 1;

#undef WSUM
done:
    ng_bag_free(&hidden);
    ng_bag_free(&output);
    ng_bag_free(&bias_role);
    ng_bag_free(&empty);
    for (i = 0; i < 3; i++) {
        ng_weighted_input_free(&in[i]);
    }
    if (!ok) {
        ng_free(g);
        return -1;
    }
    *out = g;
    return 0;
}

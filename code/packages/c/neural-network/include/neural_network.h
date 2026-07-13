/*
 * neural_network.h — a property-graph representation of neural-network
 * topologies, in pure ISO C17. A faithful port of the Rust `neural-network`
 * crate.
 * ===========================================================================
 *
 * This is NOT a trainable network — it is the *graph IR* a compiler builds to
 * describe one: named nodes (input / constant / weighted_sum / activation /
 * output), weighted directed edges, and a property bag on the graph, each node,
 * and each edge. On top of that sits a builder and a topological sort.
 *
 *   NgProperty     a tagged value: String / Number / Boolean / Null
 *   NgPropertyBag  a string -> NgProperty map
 *   NgEdge         { id, from, to, weight, properties }
 *   NeuralGraph    nodes + per-node properties + edges + an edge-id counter
 *
 * `ng_add_edge` auto-creates its endpoints and mints an id ("e0", "e1", ...)
 * when none is given; `ng_topological_sort` runs Kahn's algorithm with
 * deterministic (lexicographic) tie-breaking, reporting a cycle.
 *
 * OWNERSHIP. Every value that owns strings / arrays / bags pairs a constructor
 * with a matching `*_free`. Functions that take a `const NgPropertyBag *` copy
 * it; those that return owned strings/arrays document the matching free.
 *
 * DIVERGENCE FROM RUST. Rust panics on a non-finite constant and returns owned
 * values / `Result`; this port returns an `NgStatus` and writes results through
 * out-parameters. Property-bag getters return borrowed pointers (Rust clones).
 *
 * PORTABILITY. Pure ISO C17 — no <math.h>, no compiler extensions. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_NEURAL_NETWORK_H
#define CA_NEURAL_NETWORK_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    NG_OK = 0,
    NG_ERR_NOMEM,
    NG_ERR_NOT_FINITE, /* a constant value that is NaN or infinite */
    NG_ERR_CYCLE       /* topological sort found a cycle */
} NgStatus;

/* ── PropertyValue ────────────────────────────────────────────────────────── */

typedef enum {
    NG_PROP_STRING,
    NG_PROP_NUMBER,
    NG_PROP_BOOLEAN,
    NG_PROP_NULL
} NgPropTag;

typedef struct {
    NgPropTag tag;
    char *string; /* owned, NG_PROP_STRING */
    double number;
    int boolean;
} NgProperty;

NgProperty ng_prop_null(void);
NgProperty ng_prop_number(double n);
NgProperty ng_prop_boolean(int b);
int ng_prop_string(const char *s, NgProperty *out); /* deep-copies; 0 / -1 */
int ng_prop_copy(const NgProperty *src, NgProperty *out);
void ng_prop_free(NgProperty *p);
int ng_prop_equals(const NgProperty *a, const NgProperty *b);

/* ── PropertyBag (string -> NgProperty) ───────────────────────────────────── */

typedef struct {
    char *key;
    NgProperty val;
} NgBagEntry;

typedef struct {
    NgBagEntry *entries;
    size_t n, cap;
} NgPropertyBag;

void ng_bag_init(NgPropertyBag *bag);
void ng_bag_free(NgPropertyBag *bag);
/* Insert (replacing an existing key), taking ownership of `val`. 0 / -1. */
int ng_bag_insert(NgPropertyBag *bag, const char *key, NgProperty val);
const NgProperty *ng_bag_get(const NgPropertyBag *bag, const char *key);
size_t ng_bag_size(const NgPropertyBag *bag);
int ng_bag_copy(const NgPropertyBag *src, NgPropertyBag *out);
/* Copy every entry of `src` into `dst` (replacing on key collision). 0 / -1. */
int ng_bag_extend(NgPropertyBag *dst, const NgPropertyBag *src);

/* ── ActivationKind ───────────────────────────────────────────────────────── */

typedef enum {
    NG_ACT_RELU,
    NG_ACT_SIGMOID,
    NG_ACT_TANH,
    NG_ACT_NONE
} NgActivation;
const char *ng_activation_str(NgActivation a);

/* ── Edge / WeightedInput ─────────────────────────────────────────────────── */

typedef struct {
    char *id;
    char *from;
    char *to;
    double weight;
    NgPropertyBag properties;
} NgEdge;

typedef struct {
    char *from;
    double weight;
    char *edge_id; /* NULL if none */
    NgPropertyBag properties;
} NgWeightedInput;

/* from + weight + edge_id (the Rust WeightedInput::new; empty property bag). */
int ng_weighted_input_init(NgWeightedInput *out, const char *from, double weight,
                           const char *edge_id);
void ng_weighted_input_free(NgWeightedInput *wi);

/* ── NeuralGraph ──────────────────────────────────────────────────────────── */

typedef struct NeuralGraph NeuralGraph;

/* Create a graph; seeds "nn.version"="0" and, if name != NULL, "nn.name". */
int ng_new(NeuralGraph **out, const char *name);
void ng_free(NeuralGraph *g);

const NgPropertyBag *ng_graph_properties(const NeuralGraph *g);
size_t ng_node_count(const NeuralGraph *g);
const char *ng_node_at(const NeuralGraph *g, size_t i);
size_t ng_edge_count(const NeuralGraph *g);
const NgEdge *ng_edge_at(const NeuralGraph *g, size_t i);

/* Add a node (idempotent by name), extending its property bag. 0 / -1. */
int ng_add_node(NeuralGraph *g, const char *node,
                const NgPropertyBag *properties);
/* Borrowed property bag of a node, or NULL if the node is absent. */
const NgPropertyBag *ng_node_properties(const NeuralGraph *g, const char *node);

/* Add a directed weighted edge (auto-adding both endpoints). If edge_id is NULL
 * a fresh "e<n>" id is minted. A "weight" property is merged in. On NG_OK, if
 * out_id != NULL it receives the edge id as an owned string (caller frees). */
NgStatus ng_add_edge(NeuralGraph *g, const char *from, const char *to,
                     double weight, const NgPropertyBag *properties,
                     const char *edge_id, char **out_id);

/* Malloc'd array of borrowed pointers to edges whose `to` == node. 0 / -1
 * (free only the array). */
int ng_incoming_edges(const NeuralGraph *g, const char *node,
                      const NgEdge ***out, size_t *count);

/* Kahn's algorithm. On NG_OK writes a malloc'd array of `*count` owned node-name
 * strings (free with ng_string_array_free); NG_ERR_CYCLE on a cycle. */
NgStatus ng_topological_sort(const NeuralGraph *g, char ***out, size_t *count);
void ng_string_array_free(char **arr, size_t count);

/* ── Layer builders (set the nn.op / nn.* properties, add nodes/edges) ────── */

int ng_add_input(NeuralGraph *g, const char *node, const char *input_name,
                 const NgPropertyBag *properties);
NgStatus ng_add_constant(NeuralGraph *g, const char *node, double value,
                         const NgPropertyBag *properties);
int ng_add_weighted_sum(NeuralGraph *g, const char *node,
                        const NgWeightedInput *inputs, size_t n_inputs,
                        const NgPropertyBag *properties);
NgStatus ng_add_activation(NeuralGraph *g, const char *node, const char *input,
                           NgActivation activation,
                           const NgPropertyBag *properties, const char *edge_id,
                           char **out_id);
NgStatus ng_add_output(NeuralGraph *g, const char *node, const char *input,
                       const char *output_name, const NgPropertyBag *properties,
                       const char *edge_id, char **out_id);

/* The classic hand-wired XOR topology (input/hidden/output layers). */
int ng_create_xor_network(NeuralGraph **out, const char *name);

#ifdef __cplusplus
}
#endif

#endif /* CA_NEURAL_NETWORK_H */

/*
 * graph.h — an undirected weighted graph, pure ISO C17.
 * =====================================================
 *
 * A faithful port of the Rust `graph` crate: an undirected, weighted graph over
 * string-named nodes, with heterogeneous property bags on the graph, its nodes,
 * and its edges, plus the standard algorithms (BFS, DFS, connectivity,
 * connected components, cycle detection, shortest path, minimum spanning tree).
 *
 * Every internal map is *ordered by key* (the Rust crate uses `BTreeMap`), so
 * `graph_nodes`, `graph_neighbors`, `graph_edges`, and every traversal come out
 * in a deterministic, sorted order — matching the Rust crate byte-for-byte on
 * the shared conformance vectors.
 *
 * NOTE ON REPRESENTATION. The Rust crate keeps two interchangeable internal
 * representations (adjacency list and adjacency matrix) that produce identical
 * observable output. This C port stores the chosen `GraphRepr` (returned by
 * `graph_repr`) but backs BOTH with a single ordered-adjacency model — the
 * public behavior is identical across representations (as the Rust crate's own
 * tests assert), so a second physical layout would add code without changing a
 * single result.
 *
 * OWNERSHIP. The graph copies every string you pass in. Functions that return a
 * list (`GraphStrList`, `GraphEdgeList`, `GraphComponents`) hand back owned
 * copies you must release with the matching `_free`. Single-property getters
 * return a value whose string field (for `GRAPH_PROP_STRING`) is BORROWED from
 * the graph and is valid until the graph is next mutated or freed.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef GRAPH_H
#define GRAPH_H

#include <stddef.h> /* size_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Representation & status ────────────────────────────────────────────────*/
typedef enum { GRAPH_ADJ_LIST, GRAPH_ADJ_MATRIX } GraphRepr;

typedef enum {
    GRAPH_OK = 0,
    GRAPH_ERR_NODE_NOT_FOUND,
    GRAPH_ERR_EDGE_NOT_FOUND,
    GRAPH_ERR_NOT_CONNECTED,
    GRAPH_ERR_OUT_OF_MEMORY
} GraphStatus;

/* ── Property values ────────────────────────────────────────────────────────*/
typedef enum {
    GRAPH_PROP_STRING,
    GRAPH_PROP_NUMBER,
    GRAPH_PROP_BOOL,
    GRAPH_PROP_NULL
} GraphPropKind;

/* A tagged property value. On input the `s` string (for GRAPH_PROP_STRING) is
 * copied by the graph; on output it is borrowed from the graph (see OWNERSHIP).
 */
typedef struct {
    GraphPropKind kind;
    const char *s; /* GRAPH_PROP_STRING */
    double n;      /* GRAPH_PROP_NUMBER */
    int b;         /* GRAPH_PROP_BOOL   */
} GraphPropValue;

/* One (key, value) property entry, for bulk add_*_with_properties calls. */
typedef struct {
    const char *key;
    GraphPropValue value;
} GraphPropEntry;

/* Convenience constructors. */
GraphPropValue graph_prop_string(const char *s);
GraphPropValue graph_prop_number(double n);
GraphPropValue graph_prop_bool(int b);
GraphPropValue graph_prop_null(void);
/* Structural equality (matches the Rust PartialEq). */
int graph_prop_equal(GraphPropValue a, GraphPropValue b);

/* ── Owned result lists ─────────────────────────────────────────────────────*/
typedef struct {
    char **items;
    size_t len;
} GraphStrList;
void graph_str_list_free(GraphStrList *list);

typedef struct {
    char *left;
    char *right;
    double weight;
} GraphEdge;
typedef struct {
    GraphEdge *items;
    size_t len;
} GraphEdgeList;
void graph_edge_list_free(GraphEdgeList *list);

typedef struct {
    GraphStrList *items;
    size_t len;
} GraphComponents;
void graph_components_free(GraphComponents *comps);

/* ── The graph ──────────────────────────────────────────────────────────────*/
typedef struct Graph Graph;

Graph *graph_new(GraphRepr repr); /* NULL on OOM */
void graph_free(Graph *g);
GraphRepr graph_repr(const Graph *g);
size_t graph_size(const Graph *g);

/* Nodes. add_* copy `node`; return GRAPH_ERR_OUT_OF_MEMORY on allocation
 * failure. remove returns GRAPH_ERR_NODE_NOT_FOUND if absent. */
GraphStatus graph_add_node(Graph *g, const char *node);
GraphStatus graph_add_node_props(Graph *g, const char *node,
                                 const GraphPropEntry *props, size_t nprops);
GraphStatus graph_remove_node(Graph *g, const char *node);
int graph_has_node(const Graph *g, const char *node);
GraphStatus graph_nodes(const Graph *g, GraphStrList *out); /* sorted */

/* Edges (undirected). add_* auto-create endpoints and copy names. */
GraphStatus graph_add_edge(Graph *g, const char *left, const char *right,
                           double weight);
GraphStatus graph_add_edge_props(Graph *g, const char *left, const char *right,
                                 double weight, const GraphPropEntry *props,
                                 size_t nprops);
GraphStatus graph_remove_edge(Graph *g, const char *left, const char *right);
int graph_has_edge(const Graph *g, const char *left, const char *right);
GraphStatus graph_edge_weight(const Graph *g, const char *left,
                              const char *right, double *out);
GraphStatus graph_edges(const Graph *g, GraphEdgeList *out); /* sorted */

/* Neighbors. */
GraphStatus graph_neighbors(const Graph *g, const char *node, GraphStrList *out);
GraphStatus graph_degree(const Graph *g, const char *node, size_t *out);

/* Graph-level properties. */
GraphStatus graph_set_graph_property(Graph *g, const char *key,
                                     GraphPropValue value);
void graph_remove_graph_property(Graph *g, const char *key);
/* Returns 1 and fills *out if the key is present, else 0. */
int graph_get_graph_property(const Graph *g, const char *key,
                             GraphPropValue *out);

/* Node properties. `*found` (may be NULL) reports key presence. */
GraphStatus graph_set_node_property(Graph *g, const char *node, const char *key,
                                    GraphPropValue value);
GraphStatus graph_remove_node_property(Graph *g, const char *node,
                                       const char *key);
GraphStatus graph_get_node_property(const Graph *g, const char *node,
                                    const char *key, GraphPropValue *out,
                                    int *found);

/* Edge properties. Every edge always exposes a "weight" property mirroring its
 * numeric weight. Setting "weight" to a number updates the edge weight; setting
 * it to a non-number returns GRAPH_ERR_EDGE_NOT_FOUND (faithful to the crate).
 * Removing "weight" resets it to 1.0. */
GraphStatus graph_set_edge_property(Graph *g, const char *left,
                                    const char *right, const char *key,
                                    GraphPropValue value);
GraphStatus graph_remove_edge_property(Graph *g, const char *left,
                                       const char *right, const char *key);
GraphStatus graph_get_edge_property(const Graph *g, const char *left,
                                    const char *right, const char *key,
                                    GraphPropValue *out, int *found);

/* ── Algorithms ─────────────────────────────────────────────────────────────*/
GraphStatus graph_bfs(const Graph *g, const char *start, GraphStrList *out);
GraphStatus graph_dfs(const Graph *g, const char *start, GraphStrList *out);
int graph_is_connected(const Graph *g);
GraphStatus graph_connected_components(const Graph *g, GraphComponents *out);
int graph_has_cycle(const Graph *g);
/* Shortest path: an empty list means no path (or missing endpoints), matching
 * the Rust crate. Returns GRAPH_ERR_OUT_OF_MEMORY only on allocation failure. */
GraphStatus graph_shortest_path(const Graph *g, const char *start,
                                const char *end, GraphStrList *out);
/* MST edges (sorted). GRAPH_ERR_NOT_CONNECTED if the graph is disconnected. */
GraphStatus graph_minimum_spanning_tree(const Graph *g, GraphEdgeList *out);

#ifdef __cplusplus
}
#endif

#endif /* GRAPH_H */

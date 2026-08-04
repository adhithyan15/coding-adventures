/*
 * Tests for the C neural-network library, using the header-only iso_test.h
 * harness (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "neural_network.h"

/* Does the graph contain an edge with the given id? */
static int has_edge_id(const NeuralGraph *g, const char *id) {
    size_t i;
    for (i = 0; i < ng_edge_count(g); i++) {
        if (strcmp(ng_edge_at(g, i)->id, id) == 0) {
            return 1;
        }
    }
    return 0;
}

int main(void) {
    /* ── property values & bag basics ────────────────────────────────────── */
    {
        NgPropertyBag bag;
        ng_bag_init(&bag);
        ISO_CHECK(ng_bag_insert(&bag, "n", ng_prop_number(1.5)) == 0);
        ISO_CHECK(ng_bag_insert(&bag, "b", ng_prop_boolean(1)) == 0);
        NgProperty s;
        ISO_CHECK(ng_prop_string("hi", &s) == 0);
        ISO_CHECK(ng_bag_insert(&bag, "s", s) == 0);
        ISO_CHECK_EQ_UINT(ng_bag_size(&bag), 3u);
        ISO_CHECK(ng_bag_get(&bag, "n")->number == 1.5);
        ISO_CHECK(ng_bag_get(&bag, "b")->boolean == 1);
        ISO_CHECK_STR_EQ(ng_bag_get(&bag, "s")->string, "hi");
        ISO_CHECK(ng_bag_get(&bag, "missing") == NULL);
        /* Re-inserting a key replaces it. */
        ISO_CHECK(ng_bag_insert(&bag, "n", ng_prop_number(2.0)) == 0);
        ISO_CHECK_EQ_UINT(ng_bag_size(&bag), 3u);
        ISO_CHECK(ng_bag_get(&bag, "n")->number == 2.0);
        ng_bag_free(&bag);
    }

    /* ── activation names ────────────────────────────────────────────────── */
    ISO_CHECK_STR_EQ(ng_activation_str(NG_ACT_RELU), "relu");
    ISO_CHECK_STR_EQ(ng_activation_str(NG_ACT_SIGMOID), "sigmoid");
    ISO_CHECK_STR_EQ(ng_activation_str(NG_ACT_TANH), "tanh");
    ISO_CHECK_STR_EQ(ng_activation_str(NG_ACT_NONE), "none");

    /* ── new graph seeds nn.version (+ nn.name) ──────────────────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_new(&g, "tiny") == 0);
        const NgPropertyBag *gp = ng_graph_properties(g);
        ISO_CHECK_STR_EQ(ng_bag_get(gp, "nn.version")->string, "0");
        ISO_CHECK_STR_EQ(ng_bag_get(gp, "nn.name")->string, "tiny");
        ng_free(g);
    }

    /* ── builds a tiny weighted graph; incoming + topo sort ──────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_new(&g, "tiny") == 0);
        ISO_CHECK(ng_add_input(g, "x0", "x0", NULL) == 0);
        ISO_CHECK(ng_add_input(g, "x1", "x1", NULL) == 0);
        ISO_CHECK(ng_add_constant(g, "bias", 1.0, NULL) == NG_OK);

        NgWeightedInput in[3];
        ISO_CHECK(ng_weighted_input_init(&in[0], "x0", 0.25, "x0_to_sum") == 0);
        ISO_CHECK(ng_weighted_input_init(&in[1], "x1", 0.75, "x1_to_sum") == 0);
        ISO_CHECK(ng_weighted_input_init(&in[2], "bias", -1.0, "bias_to_sum") ==
                  0);
        ISO_CHECK(ng_add_weighted_sum(g, "sum", in, 3, NULL) == 0);
        size_t i;
        for (i = 0; i < 3; i++) {
            ng_weighted_input_free(&in[i]);
        }
        ISO_CHECK(ng_add_activation(g, "relu", "sum", NG_ACT_RELU, NULL,
                                    "sum_to_relu", NULL) == NG_OK);
        ISO_CHECK(ng_add_output(g, "out", "relu", "prediction", NULL,
                                "relu_to_out", NULL) == NG_OK);

        /* sum has three incoming edges (x0, x1, bias). */
        const NgEdge **inc = NULL;
        size_t count = 0;
        ISO_CHECK(ng_incoming_edges(g, "sum", &inc, &count) == 0);
        ISO_CHECK_EQ_UINT(count, 3u);
        free((void *)inc);

        /* topological sort ends at "out". */
        char **order = NULL;
        size_t n = 0;
        ISO_CHECK(ng_topological_sort(g, &order, &n) == NG_OK);
        ISO_CHECK(n >= 1);
        ISO_CHECK_STR_EQ(order[n - 1], "out");
        ng_string_array_free(order, n);

        ng_free(g);
    }

    /* ── weighted-sum node carries the nn.op property ────────────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_new(&g, NULL) == 0);
        NgWeightedInput wi0;
        ISO_CHECK(ng_weighted_input_init(&wi0, "a", 2.0, "a_to_s") == 0);
        ISO_CHECK(ng_add_weighted_sum(g, "s", &wi0, 1, NULL) == 0);
        ng_weighted_input_free(&wi0);
        const NgPropertyBag *sp = ng_node_properties(g, "s");
        ISO_CHECK(sp != NULL);
        if (sp) {
            ISO_CHECK_STR_EQ(ng_bag_get(sp, "nn.op")->string, "weighted_sum");
        }
        /* The edge carries the merged "weight" property. */
        const NgEdge **inc = NULL;
        size_t count = 0;
        ISO_CHECK(ng_incoming_edges(g, "s", &inc, &count) == 0);
        ISO_CHECK_EQ_UINT(count, 1u);
        if (count == 1) {
            ISO_CHECK(ng_bag_get(&inc[0]->properties, "weight")->number == 2.0);
        }
        free((void *)inc);
        ng_free(g);
    }

    /* ── a non-finite constant is rejected ───────────────────────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_new(&g, NULL) == 0);
        volatile double zero = 0.0;
        double nan = zero / zero;
        double inf = 1e308 * 10.0;
        ISO_CHECK(ng_add_constant(g, "c", nan, NULL) == NG_ERR_NOT_FINITE);
        ISO_CHECK(ng_add_constant(g, "c", inf, NULL) == NG_ERR_NOT_FINITE);
        ISO_CHECK(ng_add_constant(g, "c", 3.5, NULL) == NG_OK);
        ng_free(g);
    }

    /* ── auto-generated edge ids ("e0", "e1", ...) ───────────────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_new(&g, NULL) == 0);
        char *id0 = NULL, *id1 = NULL;
        ISO_CHECK(ng_add_edge(g, "a", "b", 1.0, NULL, NULL, &id0) == NG_OK);
        ISO_CHECK(ng_add_edge(g, "b", "c", 1.0, NULL, NULL, &id1) == NG_OK);
        ISO_CHECK_STR_EQ(id0, "e0");
        ISO_CHECK_STR_EQ(id1, "e1");
        free(id0);
        free(id1);
        ng_free(g);
    }

    /* ── cycle detection in topological sort ─────────────────────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_new(&g, NULL) == 0);
        ISO_CHECK(ng_add_edge(g, "a", "b", 1.0, NULL, "e_ab", NULL) == NG_OK);
        ISO_CHECK(ng_add_edge(g, "b", "a", 1.0, NULL, "e_ba", NULL) == NG_OK);
        char **order = NULL;
        size_t n = 0;
        ISO_CHECK(ng_topological_sort(g, &order, &n) == NG_ERR_CYCLE);
        ISO_CHECK(order == NULL);
        ng_free(g);
    }

    /* ── the XOR network topology ────────────────────────────────────────── */
    {
        NeuralGraph *g;
        ISO_CHECK(ng_create_xor_network(&g, "xor") == 0);
        /* out_sum draws from h_or, h_nand, and bias. */
        const NgEdge **inc = NULL;
        size_t count = 0;
        ISO_CHECK(ng_incoming_edges(g, "out_sum", &inc, &count) == 0);
        ISO_CHECK_EQ_UINT(count, 3u);
        free((void *)inc);
        /* the named hidden->output edge exists. */
        ISO_CHECK(has_edge_id(g, "h_or_to_out"));
        /* the whole thing is a DAG (topological sort succeeds). */
        char **order = NULL;
        size_t n = 0;
        ISO_CHECK(ng_topological_sort(g, &order, &n) == NG_OK);
        ng_string_array_free(order, n);
        ng_free(g);
    }

    return ISO_TEST_RESULT();
}

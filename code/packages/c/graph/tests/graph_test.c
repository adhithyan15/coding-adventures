/*
 * Tests for graph, using the header-only iso_test.h harness (pure ISO).
 * The vectors mirror the Rust crate's unit tests, run for BOTH representation
 * values (they must produce identical results, as in the crate).
 */
#include "iso_test.h"

#include <string.h>

#include "graph.h"

/* Compare a GraphStrList against a NULL-terminated array of expected names. */
static int list_equals(const GraphStrList *l, const char *const *expected) {
    size_t i = 0;
    for (; expected[i] != NULL; i++) {
        if (i >= l->len || strcmp(l->items[i], expected[i]) != 0) {
            return 0;
        }
    }
    return i == l->len;
}

static Graph *make_graph(GraphRepr repr) {
    Graph *g = graph_new(repr);
    graph_add_edge(g, "London", "Paris", 300.0);
    graph_add_edge(g, "London", "Amsterdam", 520.0);
    graph_add_edge(g, "Paris", "Berlin", 878.0);
    graph_add_edge(g, "Amsterdam", "Berlin", 655.0);
    graph_add_edge(g, "Amsterdam", "Brussels", 180.0);
    return g;
}
static Graph *make_triangle(GraphRepr repr) {
    Graph *g = graph_new(repr);
    graph_add_edge(g, "A", "B", 1.0);
    graph_add_edge(g, "B", "C", 1.0);
    graph_add_edge(g, "C", "A", 1.0);
    return g;
}
static Graph *make_path(GraphRepr repr) {
    Graph *g = graph_new(repr);
    graph_add_edge(g, "A", "B", 1.0);
    graph_add_edge(g, "B", "C", 1.0);
    return g;
}

static void run_for_repr(GraphRepr repr) {
    /* ── construction / nodes ──────────────────────────────────────────────*/
    {
        Graph *g = graph_new(repr);
        ISO_CHECK_EQ_UINT(graph_size(g), 0u);
        graph_add_node(g, "A");
        graph_add_node(g, "B");
        ISO_CHECK(graph_has_node(g, "A"));
        ISO_CHECK(graph_has_node(g, "B"));
        ISO_CHECK_EQ_INT(graph_remove_node(g, "A"), GRAPH_OK);
        ISO_CHECK(!graph_has_node(g, "A"));
        ISO_CHECK_EQ_UINT(graph_size(g), 1u);
        ISO_CHECK_EQ_INT(graph_remove_node(g, "Z"), GRAPH_ERR_NODE_NOT_FOUND);
        graph_free(g);
    }

    /* ── edge operations / undirected neighbors ────────────────────────────*/
    {
        Graph *g = graph_new(repr);
        double w = 0.0;
        GraphStrList nb;
        const char *expect[] = {"B", NULL};
        graph_add_edge(g, "A", "B", 2.5);
        ISO_CHECK(graph_has_edge(g, "A", "B"));
        ISO_CHECK(graph_has_edge(g, "B", "A"));
        ISO_CHECK_EQ_INT(graph_edge_weight(g, "A", "B", &w), GRAPH_OK);
        ISO_CHECK_EQ_DBL(w, 2.5, 0.0);
        ISO_CHECK_EQ_INT(graph_neighbors(g, "A", &nb), GRAPH_OK);
        ISO_CHECK(list_equals(&nb, expect));
        graph_str_list_free(&nb);
        graph_free(g);
    }

    /* ── property bags ─────────────────────────────────────────────────────*/
    {
        Graph *g = graph_new(repr);
        GraphPropValue out;
        int found = 0;
        GraphPropEntry np[1];
        GraphPropEntry ep[1];

        graph_set_graph_property(g, "name", graph_prop_string("city-map"));
        graph_set_graph_property(g, "version", graph_prop_number(1.0));
        ISO_CHECK(graph_get_graph_property(g, "name", &out));
        ISO_CHECK(graph_prop_equal(out, graph_prop_string("city-map")));
        graph_remove_graph_property(g, "version");
        ISO_CHECK(!graph_get_graph_property(g, "version", &out));

        np[0].key = "kind";
        np[0].value = graph_prop_string("input");
        graph_add_node_props(g, "A", np, 1);
        {
            GraphPropEntry extra[1];
            extra[0].key = "trainable";
            extra[0].value = graph_prop_bool(0);
            graph_add_node_props(g, "A", extra, 1);
        }
        graph_set_node_property(g, "A", "slot", graph_prop_number(0.0));
        ISO_CHECK_EQ_INT(graph_get_node_property(g, "A", "kind", &out, &found),
                         GRAPH_OK);
        ISO_CHECK(found);
        ISO_CHECK(graph_prop_equal(out, graph_prop_string("input")));

        ep[0].key = "role";
        ep[0].value = graph_prop_string("distance");
        graph_add_edge_props(g, "A", "B", 2.5, ep, 1);
        ISO_CHECK_EQ_INT(
            graph_get_edge_property(g, "B", "A", "weight", &out, &found),
            GRAPH_OK);
        ISO_CHECK(found);
        ISO_CHECK(graph_prop_equal(out, graph_prop_number(2.5)));

        graph_set_edge_property(g, "B", "A", "weight", graph_prop_number(7.0));
        {
            double w = 0.0;
            ISO_CHECK_EQ_INT(graph_edge_weight(g, "A", "B", &w), GRAPH_OK);
            ISO_CHECK_EQ_DBL(w, 7.0, 0.0);
        }
        graph_set_edge_property(g, "A", "B", "trainable", graph_prop_bool(1));
        graph_remove_edge_property(g, "A", "B", "role");
        ISO_CHECK_EQ_INT(
            graph_get_edge_property(g, "A", "B", "trainable", &out, &found),
            GRAPH_OK);
        ISO_CHECK(found);
        ISO_CHECK(graph_prop_equal(out, graph_prop_bool(1)));
        ISO_CHECK_EQ_INT(
            graph_get_edge_property(g, "A", "B", "weight", &out, &found),
            GRAPH_OK);
        ISO_CHECK(graph_prop_equal(out, graph_prop_number(7.0)));
        ISO_CHECK_EQ_INT(
            graph_get_edge_property(g, "A", "B", "role", &out, &found),
            GRAPH_OK);
        ISO_CHECK(!found);

        graph_remove_edge(g, "A", "B");
        ISO_CHECK_EQ_INT(
            graph_get_edge_property(g, "A", "B", "weight", &out, &found),
            GRAPH_ERR_EDGE_NOT_FOUND);
        graph_free(g);
    }

    /* ── self-loops and zero-weight edges ──────────────────────────────────*/
    {
        Graph *g = graph_new(repr);
        double w = 1.0;
        GraphStrList nb;
        const char *expect[] = {"A", NULL};
        graph_add_edge(g, "A", "A", 0.0);
        ISO_CHECK(graph_has_edge(g, "A", "A"));
        ISO_CHECK_EQ_INT(graph_edge_weight(g, "A", "A", &w), GRAPH_OK);
        ISO_CHECK_EQ_DBL(w, 0.0, 0.0);
        ISO_CHECK_EQ_INT(graph_neighbors(g, "A", &nb), GRAPH_OK);
        ISO_CHECK(list_equals(&nb, expect));
        graph_str_list_free(&nb);
        graph_free(g);
    }

    /* ── traversals / connectivity / cycles ────────────────────────────────*/
    {
        const char *path_order[] = {"A", "B", "C", NULL};
        const char *city_order[] = {"London", "Amsterdam", "Paris", "Berlin",
                                    "Brussels", NULL};
        Graph *p, *gr, *tri;
        GraphStrList t;

        p = make_path(repr);
        ISO_CHECK_EQ_INT(graph_bfs(p, "A", &t), GRAPH_OK);
        ISO_CHECK(list_equals(&t, path_order));
        graph_str_list_free(&t);
        ISO_CHECK_EQ_INT(graph_dfs(p, "A", &t), GRAPH_OK);
        ISO_CHECK(list_equals(&t, path_order));
        graph_str_list_free(&t);
        ISO_CHECK(!graph_has_cycle(p));
        graph_free(p);

        gr = make_graph(repr);
        ISO_CHECK_EQ_INT(graph_bfs(gr, "London", &t), GRAPH_OK);
        ISO_CHECK(list_equals(&t, city_order));
        graph_str_list_free(&t);
        ISO_CHECK(graph_is_connected(gr));
        graph_free(gr);

        tri = make_triangle(repr);
        ISO_CHECK(graph_has_cycle(tri));
        graph_free(tri);
    }

    /* ── connected components ──────────────────────────────────────────────*/
    {
        Graph *g = graph_new(repr);
        GraphComponents comps;
        const char *abc[] = {"A", "B", "C", NULL};
        const char *de[] = {"D", "E", NULL};
        const char *f[] = {"F", NULL};
        int has_abc = 0, has_de = 0, has_f = 0;
        size_t i;
        graph_add_edge(g, "A", "B", 1.0);
        graph_add_edge(g, "B", "C", 1.0);
        graph_add_edge(g, "D", "E", 1.0);
        graph_add_node(g, "F");
        ISO_CHECK_EQ_INT(graph_connected_components(g, &comps), GRAPH_OK);
        ISO_CHECK_EQ_UINT(comps.len, 3u);
        for (i = 0; i < comps.len; i++) {
            if (list_equals(&comps.items[i], abc)) has_abc = 1;
            if (list_equals(&comps.items[i], de)) has_de = 1;
            if (list_equals(&comps.items[i], f)) has_f = 1;
        }
        ISO_CHECK(has_abc && has_de && has_f);
        graph_components_free(&comps);
        graph_free(g);
    }

    /* ── shortest path / MST ───────────────────────────────────────────────*/
    {
        Graph *g = make_graph(repr);
        GraphStrList path;
        GraphEdgeList mst;
        const char *expect[] = {"London", "Amsterdam", "Berlin", NULL};
        ISO_CHECK_EQ_INT(graph_shortest_path(g, "London", "Berlin", &path),
                         GRAPH_OK);
        ISO_CHECK(list_equals(&path, expect));
        graph_str_list_free(&path);
        ISO_CHECK_EQ_INT(graph_minimum_spanning_tree(g, &mst), GRAPH_OK);
        ISO_CHECK_EQ_UINT(mst.len, 4u);
        graph_edge_list_free(&mst);
        graph_free(g);
    }

    /* ── disconnected graph has no spanning tree ───────────────────────────*/
    {
        Graph *g = graph_new(repr);
        GraphEdgeList mst;
        graph_add_edge(g, "A", "B", 1.0);
        graph_add_node(g, "C");
        ISO_CHECK_EQ_INT(graph_minimum_spanning_tree(g, &mst),
                         GRAPH_ERR_NOT_CONNECTED);
        graph_free(g);
    }
}

int main(void) {
    run_for_repr(GRAPH_ADJ_LIST);
    run_for_repr(GRAPH_ADJ_MATRIX);

    /* bfs on a missing node reports NODE_NOT_FOUND, not a crash. */
    {
        Graph *g = graph_new(GRAPH_ADJ_LIST);
        GraphStrList t;
        graph_add_node(g, "A");
        ISO_CHECK_EQ_INT(graph_bfs(g, "Z", &t), GRAPH_ERR_NODE_NOT_FOUND);
        ISO_CHECK_EQ_INT(graph_repr(g), GRAPH_ADJ_LIST);
        graph_free(g);
    }

    return ISO_TEST_RESULT();
}

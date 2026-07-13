// Tests for graph, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's unit tests, run across BOTH representations.
#include "iso_test.h"

#include <string>
#include <vector>

#include "graph.hpp"

namespace g = ca::graph;
using Names = std::vector<std::string>;

static g::Graph make_graph(g::Repr repr) {
    g::Graph gr(repr);
    gr.add_edge("London", "Paris", 300.0);
    gr.add_edge("London", "Amsterdam", 520.0);
    gr.add_edge("Paris", "Berlin", 878.0);
    gr.add_edge("Amsterdam", "Berlin", 655.0);
    gr.add_edge("Amsterdam", "Brussels", 180.0);
    return gr;
}
static g::Graph make_triangle(g::Repr repr) {
    g::Graph gr(repr);
    gr.add_edge("A", "B", 1.0);
    gr.add_edge("B", "C", 1.0);
    gr.add_edge("C", "A", 1.0);
    return gr;
}
static g::Graph make_path(g::Repr repr) {
    g::Graph gr(repr);
    gr.add_edge("A", "B", 1.0);
    gr.add_edge("B", "C", 1.0);
    return gr;
}

int main() {
    const g::Repr reprs[2] = {g::Repr::AdjacencyList, g::Repr::AdjacencyMatrix};

    for (g::Repr repr : reprs) {
        // ── construction / nodes ────────────────────────────────────────────
        {
            g::Graph gr(repr);
            ISO_CHECK_EQ_UINT(gr.size(), 0u);
            gr.add_node("A");
            gr.add_node("B");
            ISO_CHECK(gr.has_node("A"));
            ISO_CHECK(gr.has_node("B"));
            gr.remove_node("A");
            ISO_CHECK(!gr.has_node("A"));
            ISO_CHECK_EQ_UINT(gr.size(), 1u);
        }

        // ── edge operations / undirected neighbors ──────────────────────────
        {
            g::Graph gr(repr);
            gr.add_edge("A", "B", 2.5);
            ISO_CHECK(gr.has_edge("A", "B"));
            ISO_CHECK(gr.has_edge("B", "A"));
            ISO_CHECK_EQ_DBL(gr.edge_weight("A", "B"), 2.5, 0.0);
            ISO_CHECK((gr.neighbors("A") == Names{"B"}));
        }

        // ── property bags ───────────────────────────────────────────────────
        {
            g::Graph gr(repr);
            gr.set_graph_property("name", g::PropertyValue::String("city-map"));
            gr.set_graph_property("version", g::PropertyValue::Number(1.0));
            ISO_CHECK(gr.graph_properties().at("name") ==
                      g::PropertyValue::String("city-map"));
            gr.remove_graph_property("version");
            ISO_CHECK(gr.graph_properties().count("version") == 0);

            g::PropertyBag node_props;
            node_props["kind"] = g::PropertyValue::String("input");
            gr.add_node_with_properties("A", node_props);
            g::PropertyBag extra;
            extra["trainable"] = g::PropertyValue::Bool(false);
            gr.add_node_with_properties("A", extra);
            gr.set_node_property("A", "slot", g::PropertyValue::Number(0.0));
            ISO_CHECK(gr.node_properties("A").at("kind") ==
                      g::PropertyValue::String("input"));

            g::PropertyBag edge_props;
            edge_props["role"] = g::PropertyValue::String("distance");
            gr.add_edge_with_properties("A", "B", 2.5, edge_props);
            ISO_CHECK(gr.edge_properties("B", "A").at("weight") ==
                      g::PropertyValue::Number(2.5));

            gr.set_edge_property("B", "A", "weight", g::PropertyValue::Number(7.0));
            ISO_CHECK_EQ_DBL(gr.edge_weight("A", "B"), 7.0, 0.0);
            gr.set_edge_property("A", "B", "trainable", g::PropertyValue::Bool(true));
            gr.remove_edge_property("A", "B", "role");
            g::PropertyBag props = gr.edge_properties("A", "B");
            ISO_CHECK(props.at("trainable") == g::PropertyValue::Bool(true));
            ISO_CHECK(props.at("weight") == g::PropertyValue::Number(7.0));

            gr.remove_edge("A", "B");
            bool threw = false;
            try {
                gr.edge_properties("A", "B");
            } catch (const g::Error&) {
                threw = true;
            }
            ISO_CHECK(threw);
        }

        // ── self-loops and zero-weight edges ────────────────────────────────
        {
            g::Graph gr(repr);
            gr.add_edge("A", "A", 0.0);
            ISO_CHECK(gr.has_edge("A", "A"));
            ISO_CHECK_EQ_DBL(gr.edge_weight("A", "A"), 0.0, 0.0);
            ISO_CHECK((gr.neighbors("A") == Names{"A"}));
        }

        // ── traversals / connectivity / cycles ──────────────────────────────
        {
            ISO_CHECK((g::bfs(make_path(repr), "A") == Names{"A", "B", "C"}));
            ISO_CHECK((g::dfs(make_path(repr), "A") == Names{"A", "B", "C"}));
            ISO_CHECK((g::bfs(make_graph(repr), "London") ==
                       Names{"London", "Amsterdam", "Paris", "Berlin",
                             "Brussels"}));
            ISO_CHECK(g::is_connected(make_graph(repr)));
            ISO_CHECK(g::has_cycle(make_triangle(repr)));
            ISO_CHECK(!g::has_cycle(make_path(repr)));
        }

        // ── connected components ────────────────────────────────────────────
        {
            g::Graph gr(repr);
            gr.add_edge("A", "B", 1.0);
            gr.add_edge("B", "C", 1.0);
            gr.add_edge("D", "E", 1.0);
            gr.add_node("F");
            auto components = g::connected_components(gr);
            ISO_CHECK_EQ_UINT(components.size(), 3u);
            bool has_abc = false, has_de = false, has_f = false;
            for (const auto& c : components) {
                if (c == Names{"A", "B", "C"}) has_abc = true;
                if (c == Names{"D", "E"}) has_de = true;
                if (c == Names{"F"}) has_f = true;
            }
            ISO_CHECK(has_abc && has_de && has_f);
        }

        // ── shortest path / MST ─────────────────────────────────────────────
        {
            auto path = g::shortest_path(make_graph(repr), "London", "Berlin");
            ISO_CHECK((path == Names{"London", "Amsterdam", "Berlin"}));
            auto mst = g::minimum_spanning_tree(make_graph(repr));
            ISO_CHECK_EQ_UINT(mst.size(), 4u);
        }

        // ── disconnected graph has no spanning tree ─────────────────────────
        {
            g::Graph gr(repr);
            gr.add_edge("A", "B", 1.0);
            gr.add_node("C");
            bool threw = false;
            try {
                g::minimum_spanning_tree(gr);
            } catch (const g::Error& e) {
                threw = true;
                ISO_CHECK(e.kind() == g::ErrorKind::NotConnected);
            }
            ISO_CHECK(threw);
        }
    }

    return ISO_TEST_RESULT();
}

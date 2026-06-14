"""
test_graph.py — Tests for DT00: Undirected Graph
==================================================

Each test class targets one cohesive area of the Graph API.
Tests run on BOTH internal representations (adjacency list and adjacency
matrix) using the ``graph_factory`` fixture, so every assertion is verified
twice with zero code duplication.

Coverage targets: 95%+
"""

from __future__ import annotations

import pytest

from graph import (
    Graph,
    GraphRepr,
    bfs,
    connected_components,
    dfs,
    has_cycle,
    is_connected,
    minimum_spanning_tree,
    shortest_path,
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(params=[GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX])
def empty_graph(request: pytest.FixtureRequest) -> Graph[str]:
    """Empty graph in each representation."""
    return Graph(repr=request.param)


def make_graph(repr: GraphRepr = GraphRepr.ADJACENCY_LIST) -> Graph[str]:
    """Helper: build a city graph for tests.

       London —300— Paris
          |               |
         520             878
          |               |
       Amsterdam —655— Berlin
          |
         180
          |
       Brussels
    """
    g: Graph[str] = Graph(repr=repr)
    g.add_edge("London", "Paris", weight=300)
    g.add_edge("London", "Amsterdam", weight=520)
    g.add_edge("Paris", "Berlin", weight=878)
    g.add_edge("Amsterdam", "Berlin", weight=655)
    g.add_edge("Amsterdam", "Brussels", weight=180)
    return g


def make_triangle(repr: GraphRepr = GraphRepr.ADJACENCY_LIST) -> Graph[str]:
    """A—B—C—A triangle (has a cycle)."""
    g: Graph[str] = Graph(repr=repr)
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "A")
    return g


def make_path(repr: GraphRepr = GraphRepr.ADJACENCY_LIST) -> Graph[str]:
    """A—B—C path (no cycle)."""
    g: Graph[str] = Graph(repr=repr)
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    return g


# ---------------------------------------------------------------------------
# TestConstruction
# ---------------------------------------------------------------------------


class TestConstruction:
    """Graph can be built with either representation."""

    def test_default_repr_is_adjacency_list(self) -> None:
        g: Graph[str] = Graph()
        assert g._repr is GraphRepr.ADJACENCY_LIST

    def test_adjacency_matrix_repr(self) -> None:
        g: Graph[str] = Graph(repr=GraphRepr.ADJACENCY_MATRIX)
        assert g._repr is GraphRepr.ADJACENCY_MATRIX

    def test_empty_graph_has_zero_nodes(self, empty_graph: Graph[str]) -> None:
        assert len(empty_graph) == 0
        assert empty_graph.nodes() == frozenset()

    def test_repr_string(self) -> None:
        g: Graph[int] = Graph()
        g.add_node(1)
        assert "Graph(" in repr(g)
        assert "nodes=1" in repr(g)


# ---------------------------------------------------------------------------
# TestNodeOperations
# ---------------------------------------------------------------------------


class TestNodeOperations:
    """add_node, remove_node, has_node, nodes."""

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_add_node(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert g.has_node("A")
        assert len(g) == 1

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_add_node_idempotent(self, repr: GraphRepr) -> None:
        """Adding the same node twice is a no-op."""
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        g.add_node("A")
        assert len(g) == 1

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_has_node_false(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        assert not g.has_node("X")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_nodes_returns_frozenset(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        g.add_node("B")
        assert g.nodes() == frozenset({"A", "B"})

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_remove_node(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.remove_node("A")
        assert not g.has_node("A")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("B", "A")
        assert g.has_node("B")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_remove_node_missing_raises(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        with pytest.raises(KeyError):
            g.remove_node("X")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_contains(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert "A" in g
        assert "B" not in g

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_len(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        for c in "ABCDE":
            g.add_node(c)
        assert len(g) == 5


# ---------------------------------------------------------------------------
# TestEdgeOperations
# ---------------------------------------------------------------------------


class TestEdgeOperations:
    """add_edge, remove_edge, has_edge, edges, edge_weight."""

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_add_edge_both_directions(self, repr: GraphRepr) -> None:
        """An undirected edge is accessible from both endpoints."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=2.5)
        assert g.has_edge("A", "B")
        assert g.has_edge("B", "A")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_add_edge_creates_nodes(self, repr: GraphRepr) -> None:
        """add_edge implicitly creates missing nodes."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        assert g.has_node("A")
        assert g.has_node("B")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_default_weight_is_one(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        assert g.edge_weight("A", "B") == 1.0

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_update_edge_weight(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=3.0)
        g.add_edge("A", "B", weight=7.0)  # update
        assert g.edge_weight("A", "B") == 7.0

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_remove_edge(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("B", "A")
        # Nodes still exist.
        assert g.has_node("A")
        assert g.has_node("B")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_remove_missing_edge_raises(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(KeyError):
            g.remove_edge("A", "B")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_edges_no_duplicates(self, repr: GraphRepr) -> None:
        """Each undirected edge appears exactly once in edges()."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "C", weight=2.0)
        assert len(g.edges()) == 2

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_edge_weight_missing_raises(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(KeyError):
            g.edge_weight("A", "B")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_has_edge_false_unknown_node(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        assert not g.has_edge("X", "Y")


# ---------------------------------------------------------------------------
# TestPropertyBags
# ---------------------------------------------------------------------------


class TestPropertyBags:
    """Graph, node, and edge property bags."""

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_graph_properties(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.set_graph_property("name", "city-map")
        g.set_graph_property("version", 1)
        assert g.graph_properties() == {"name": "city-map", "version": 1}

        g.remove_graph_property("version")
        assert g.graph_properties() == {"name": "city-map"}

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_node_properties_merge_and_copy(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A", {"kind": "input"})
        g.add_node("A", {"trainable": False})
        g.set_node_property("A", "slot", 0)
        assert g.node_properties("A") == {
            "kind": "input",
            "trainable": False,
            "slot": 0,
        }

        copy = g.node_properties("A")
        copy["kind"] = "mutated"
        assert g.node_properties("A")["kind"] == "input"

        g.remove_node_property("A", "slot")
        assert g.node_properties("A") == {
            "kind": "input",
            "trainable": False,
        }

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_edge_properties_include_canonical_weight(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=2.5, properties={"role": "distance"})

        assert g.edge_properties("A", "B") == {
            "role": "distance",
            "weight": 2.5,
        }
        assert g.edge_properties("B", "A") == {
            "role": "distance",
            "weight": 2.5,
        }

        g.set_edge_property("B", "A", "weight", 7)
        assert g.edge_weight("A", "B") == 7.0
        assert g.edge_properties("A", "B")["weight"] == 7

        g.set_edge_property("A", "B", "trainable", True)
        g.remove_edge_property("A", "B", "role")
        assert g.edge_properties("A", "B") == {
            "trainable": True,
            "weight": 7.0,
        }

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_removing_structure_removes_properties(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A", {"kind": "input"})
        g.add_edge("A", "B", weight=3, properties={"role": "data"})

        g.remove_edge("A", "B")
        with pytest.raises(KeyError):
            g.edge_properties("A", "B")

        g.add_edge("A", "B", weight=3, properties={"role": "data"})
        g.remove_node("A")
        with pytest.raises(KeyError):
            g.node_properties("A")
        with pytest.raises(KeyError):
            g.edge_properties("A", "B")


# ---------------------------------------------------------------------------
# TestNeighborhood
# ---------------------------------------------------------------------------


class TestNeighborhood:
    """neighbors, neighbors_weighted, degree."""

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_neighbors(self, repr: GraphRepr) -> None:
        g = make_graph(repr)
        assert g.neighbors("Amsterdam") == frozenset(
            {"London", "Berlin", "Brussels"}
        )

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_neighbors_weighted(self, repr: GraphRepr) -> None:
        g = make_graph(repr)
        nw = g.neighbors_weighted("Amsterdam")
        assert nw["London"] == 520.0
        assert nw["Brussels"] == 180.0

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_degree(self, repr: GraphRepr) -> None:
        g = make_graph(repr)
        assert g.degree("Amsterdam") == 3
        assert g.degree("Brussels") == 1

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_neighbors_unknown_raises(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        with pytest.raises(KeyError):
            g.neighbors("X")

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_isolated_node_has_degree_zero(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert g.degree("A") == 0
        assert g.neighbors("A") == frozenset()


# ---------------------------------------------------------------------------
# TestBFS
# ---------------------------------------------------------------------------


class TestBFS:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_bfs_connected(self, repr: GraphRepr) -> None:
        g = make_path(repr)
        result = bfs(g, "A")
        assert result[0] == "A"
        assert set(result) == {"A", "B", "C"}

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_bfs_single_node(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert bfs(g, "A") == ["A"]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_bfs_disconnected_only_reachable(self, repr: GraphRepr) -> None:
        """BFS only visits nodes reachable from start."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_node("C")  # isolated
        assert set(bfs(g, "A")) == {"A", "B"}

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_bfs_level_order(self, repr: GraphRepr) -> None:
        """A star graph: centre visited before all leaves."""
        g: Graph[str] = Graph(repr=repr)
        for leaf in "BCDE":
            g.add_edge("A", leaf)
        result = bfs(g, "A")
        assert result[0] == "A"
        assert set(result[1:]) == set("BCDE")


# ---------------------------------------------------------------------------
# TestDFS
# ---------------------------------------------------------------------------


class TestDFS:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_dfs_visits_all_reachable(self, repr: GraphRepr) -> None:
        g = make_path(repr)
        assert set(dfs(g, "A")) == {"A", "B", "C"}

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_dfs_single_node(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("X")
        assert dfs(g, "X") == ["X"]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_dfs_disconnected_only_reachable(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_node("C")
        assert set(dfs(g, "A")) == {"A", "B"}

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_dfs_start_is_first(self, repr: GraphRepr) -> None:
        g = make_path(repr)
        assert dfs(g, "A")[0] == "A"


# ---------------------------------------------------------------------------
# TestIsConnected
# ---------------------------------------------------------------------------


class TestIsConnected:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_connected(self, repr: GraphRepr) -> None:
        assert is_connected(make_graph(repr))

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_disconnected(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_node("C")
        assert not is_connected(g)

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_empty_graph_is_connected(self, repr: GraphRepr) -> None:
        assert is_connected(Graph(repr=repr))

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_single_node_is_connected(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert is_connected(g)


# ---------------------------------------------------------------------------
# TestConnectedComponents
# ---------------------------------------------------------------------------


class TestConnectedComponents:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_three_components(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("D", "E")
        g.add_node("F")
        comps = connected_components(g)
        assert len(comps) == 3
        assert frozenset({"A", "B", "C"}) in comps
        assert frozenset({"D", "E"}) in comps
        assert frozenset({"F"}) in comps

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_connected_graph_one_component(self, repr: GraphRepr) -> None:
        comps = connected_components(make_graph(repr))
        assert len(comps) == 1

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_empty_graph_no_components(self, repr: GraphRepr) -> None:
        assert connected_components(Graph(repr=repr)) == []


# ---------------------------------------------------------------------------
# TestHasCycle
# ---------------------------------------------------------------------------


class TestHasCycle:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_triangle_has_cycle(self, repr: GraphRepr) -> None:
        assert has_cycle(make_triangle(repr))

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_path_no_cycle(self, repr: GraphRepr) -> None:
        assert not has_cycle(make_path(repr))

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_empty_graph_no_cycle(self, repr: GraphRepr) -> None:
        assert not has_cycle(Graph(repr=repr))

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_single_node_no_cycle(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert not has_cycle(g)

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_disconnected_one_component_cyclic(self, repr: GraphRepr) -> None:
        """Cycle in one component; other is acyclic — overall has_cycle is True."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")  # cycle
        g.add_edge("D", "E")  # path, no cycle
        assert has_cycle(g)

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_star_no_cycle(self, repr: GraphRepr) -> None:
        """Star graph (centre + leaves) has no cycle."""
        g: Graph[str] = Graph(repr=repr)
        for leaf in "BCDE":
            g.add_edge("A", leaf)
        assert not has_cycle(g)


# ---------------------------------------------------------------------------
# TestShortestPath
# ---------------------------------------------------------------------------


class TestShortestPath:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_unweighted_path(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "D")
        path = shortest_path(g, "A", "D")
        assert path == ["A", "B", "C", "D"]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_weighted_prefers_lower_weight(self, repr: GraphRepr) -> None:
        """Dijkstra picks the cheaper route even if it has more hops."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "D", weight=10.0)
        g.add_edge("A", "C", weight=3.0)
        g.add_edge("C", "D", weight=3.0)
        # A→B→D costs 11;  A→C→D costs 6.
        path = shortest_path(g, "A", "D")
        assert path == ["A", "C", "D"]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_same_start_end(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert shortest_path(g, "A", "A") == ["A"]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_no_path_returns_empty(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        g.add_node("B")
        assert shortest_path(g, "A", "B") == []

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_weighted_city_graph(self, repr: GraphRepr) -> None:
        """London to Berlin: via Amsterdam (520+655=1175) beats via Paris (300+878=1178)."""
        g = make_graph(repr)
        path = shortest_path(g, "London", "Berlin")
        assert path == ["London", "Amsterdam", "Berlin"]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_direct_edge(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=5.0)
        assert shortest_path(g, "A", "B") == ["A", "B"]


# ---------------------------------------------------------------------------
# TestMinimumSpanningTree
# ---------------------------------------------------------------------------


class TestMinimumSpanningTree:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_mst_has_v_minus_one_edges(self, repr: GraphRepr) -> None:
        g = make_graph(repr)
        mst = minimum_spanning_tree(g)
        assert len(mst) == len(g) - 1

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_mst_covers_all_nodes(self, repr: GraphRepr) -> None:
        g = make_graph(repr)
        mst = minimum_spanning_tree(g)
        nodes_in_mst: set[str] = set()
        for u, v, _ in mst:
            nodes_in_mst.add(u)
            nodes_in_mst.add(v)
        assert nodes_in_mst == set(g.nodes())

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_mst_minimum_weight(self, repr: GraphRepr) -> None:
        """Triangle A—1—B—2—C—4—A: MST uses the two cheapest edges."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "C", weight=2.0)
        g.add_edge("C", "A", weight=4.0)
        mst = minimum_spanning_tree(g)
        total = sum(w for _, _, w in mst)
        assert total == pytest.approx(3.0)

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_mst_empty_graph(self, repr: GraphRepr) -> None:
        assert minimum_spanning_tree(Graph(repr=repr)) == frozenset()

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_mst_single_node(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_node("A")
        assert minimum_spanning_tree(g) == frozenset()

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_mst_disconnected_raises(self, repr: GraphRepr) -> None:
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "B")
        g.add_node("C")  # isolated
        with pytest.raises(ValueError, match="not connected"):
            minimum_spanning_tree(g)


# ---------------------------------------------------------------------------
# TestEdgeCases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_integer_nodes(self, repr: GraphRepr) -> None:
        g: Graph[int] = Graph(repr=repr)
        g.add_edge(1, 2)
        g.add_edge(2, 3)
        assert g.has_edge(1, 2)
        assert shortest_path(g, 1, 3) == [1, 2, 3]

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_tuple_nodes(self, repr: GraphRepr) -> None:
        g: Graph[tuple[int, int]] = Graph(repr=repr)
        g.add_edge((0, 0), (0, 1))
        g.add_edge((0, 1), (1, 1))
        assert is_connected(g)

    def test_large_sparse_graph(self) -> None:
        """1000-node path graph — algorithms must complete quickly.

        Uses adjacency list only: adjacency matrix at V=1000 allocates V²=1M
        cells and makes neighbour iteration O(V), which is unnecessarily slow
        for this correctness + performance check.
        """
        g: Graph[int] = Graph(repr=GraphRepr.ADJACENCY_LIST)
        for i in range(999):
            g.add_edge(i, i + 1)
        assert len(g) == 1000
        assert is_connected(g)
        assert not has_cycle(g)

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_complete_graph_k4(self, repr: GraphRepr) -> None:
        """K4: 4 nodes, 6 edges — fully connected, has many cycles."""
        g: Graph[str] = Graph(repr=repr)
        nodes = ["A", "B", "C", "D"]
        for i, u in enumerate(nodes):
            for v in nodes[i + 1 :]:
                g.add_edge(u, v)
        assert len(g.edges()) == 6
        assert is_connected(g)
        assert has_cycle(g)
        mst = minimum_spanning_tree(g)
        assert len(mst) == 3

    @pytest.mark.parametrize("repr", list(GraphRepr))
    def test_self_loop_not_added_as_neighbour(self, repr: GraphRepr) -> None:
        """A self-loop adds both endpoints; the node is its own neighbour."""
        g: Graph[str] = Graph(repr=repr)
        g.add_edge("A", "A", weight=1.0)
        assert g.has_edge("A", "A")
        assert "A" in g.neighbors("A")

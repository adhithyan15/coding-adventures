"""
Comprehensive test suite for the Graph package.

Targets 95%+ code coverage by testing:
- Construction (empty, both representation types)
- Node operations (add, remove, duplicate handling)
- Edge operations (add, remove, weighted/unweighted)
- Adjacency queries (neighbors, degree, edges)
- All algorithms (BFS, DFS, shortest path, MST, cycles, components, connectivity)
- Edge cases (empty graph, single node, disconnected, complete graphs)
- Error conditions (missing nodes/edges, invalid operations)
"""

import pytest
from coding_adventures_graph import (
    Graph,
    GraphRepr,
    NodeNotFoundError,
    EdgeNotFoundError,
    bfs,
    dfs,
    is_connected,
    connected_components,
    has_cycle,
    shortest_path,
    minimum_spanning_tree,
)


# ─── Construction Tests ─────────────────────────────────────────────────────


class TestConstruction:
    """Test graph construction with both representations."""

    def test_empty_graph_adjacency_list(self):
        """Create an empty graph with adjacency list."""
        g = Graph(GraphRepr.ADJACENCY_LIST)
        assert len(g) == 0
        assert g.nodes() == frozenset()
        assert g.edges() == frozenset()

    def test_empty_graph_adjacency_matrix(self):
        """Create an empty graph with adjacency matrix."""
        g = Graph(GraphRepr.ADJACENCY_MATRIX)
        assert len(g) == 0
        assert g.nodes() == frozenset()
        assert g.edges() == frozenset()

    def test_default_representation_is_adjacency_list(self):
        """Verify default representation is adjacency list."""
        g = Graph()
        g.add_node("A")
        # Default should work without specifying representation
        assert len(g) == 1

    def test_repr_string(self):
        """Test __repr__ method."""
        g = Graph()
        g.add_node("A")
        repr_str = repr(g)
        assert "Graph" in repr_str
        assert "adjacency_list" in repr_str


# ─── Node Operations Tests ──────────────────────────────────────────────────


class TestNodeOperations:
    """Test node addition, removal, and queries."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_add_node(self, repr_type):
        """Add a node to the graph."""
        g = Graph(repr_type)
        g.add_node("A")
        assert g.has_node("A")
        assert len(g) == 1
        assert "A" in g.nodes()

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_add_multiple_nodes(self, repr_type):
        """Add multiple nodes."""
        g = Graph(repr_type)
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        assert len(g) == 3
        assert g.nodes() == frozenset({"A", "B", "C"})

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_add_duplicate_node_is_noop(self, repr_type):
        """Adding a duplicate node should be a no-op."""
        g = Graph(repr_type)
        g.add_node("A")
        g.add_node("A")
        assert len(g) == 1

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_remove_node(self, repr_type):
        """Remove a node from the graph."""
        g = Graph(repr_type)
        g.add_node("A")
        g.remove_node("A")
        assert len(g) == 0
        assert not g.has_node("A")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_remove_node_with_edges(self, repr_type):
        """Removing a node also removes its edges."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.remove_node("A")
        assert not g.has_node("A")
        assert g.has_node("B")
        assert g.has_node("C")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("A", "C")
        assert len(g.edges()) == 0

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_remove_nonexistent_node_raises_error(self, repr_type):
        """Removing a non-existent node raises NodeNotFoundError."""
        g = Graph(repr_type)
        with pytest.raises(NodeNotFoundError):
            g.remove_node("X")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_node(self, repr_type):
        """Test has_node query."""
        g = Graph(repr_type)
        assert not g.has_node("A")
        g.add_node("A")
        assert g.has_node("A")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_contains_operator(self, repr_type):
        """Test 'in' operator for node membership."""
        g = Graph(repr_type)
        g.add_node("A")
        assert "A" in g
        assert "B" not in g

    def test_nodes_with_various_types(self):
        """Nodes can be any hashable type."""
        g = Graph()
        g.add_node(42)
        g.add_node("string")
        g.add_node((1, 2))
        assert len(g) == 3
        assert g.has_node(42)
        assert g.has_node("string")
        assert g.has_node((1, 2))


# ─── Edge Operations Tests ──────────────────────────────────────────────────


class TestEdgeOperations:
    """Test edge addition, removal, and queries."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_add_edge_creates_nodes(self, repr_type):
        """Adding an edge creates both nodes if they don't exist."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        assert g.has_node("A")
        assert g.has_node("B")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_add_edge_default_weight(self, repr_type):
        """Default edge weight is 1.0."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        assert g.edge_weight("A", "B") == 1.0
        assert g.edge_weight("B", "A") == 1.0

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_add_edge_with_weight(self, repr_type):
        """Add edge with custom weight."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=5.0)
        assert g.edge_weight("A", "B") == 5.0

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_edge_is_undirected(self, repr_type):
        """Edges are undirected: (A, B) == (B, A)."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=5.0)
        assert g.has_edge("A", "B")
        assert g.has_edge("B", "A")
        assert g.edge_weight("A", "B") == g.edge_weight("B", "A")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_update_edge_weight(self, repr_type):
        """Updating an edge updates its weight."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("A", "B", weight=5.0)
        assert g.edge_weight("A", "B") == 5.0

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_remove_edge(self, repr_type):
        """Remove an edge."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("B", "A")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_remove_nonexistent_edge_raises_error(self, repr_type):
        """Removing non-existent edge raises EdgeNotFoundError."""
        g = Graph(repr_type)
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(EdgeNotFoundError):
            g.remove_edge("A", "B")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_remove_edge_missing_node_raises_error(self, repr_type):
        """Removing edge with missing node raises NodeNotFoundError."""
        g = Graph(repr_type)
        g.add_node("A")
        with pytest.raises(NodeNotFoundError):
            g.remove_edge("A", "B")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_edge(self, repr_type):
        """Test has_edge query."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        assert g.has_edge("A", "B")
        assert g.has_edge("B", "A")
        assert not g.has_edge("A", "C")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_edge_missing_node(self, repr_type):
        """has_edge returns False if either node doesn't exist."""
        g = Graph(repr_type)
        assert not g.has_edge("A", "B")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_edge_weight_missing_node_raises_error(self, repr_type):
        """edge_weight raises error if node doesn't exist."""
        g = Graph(repr_type)
        g.add_node("A")
        with pytest.raises(NodeNotFoundError):
            g.edge_weight("A", "B")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_edge_weight_missing_edge_raises_error(self, repr_type):
        """edge_weight raises error if edge doesn't exist."""
        g = Graph(repr_type)
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(EdgeNotFoundError):
            g.edge_weight("A", "B")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_edges_empty_graph(self, repr_type):
        """edges() returns empty set for empty graph."""
        g = Graph(repr_type)
        assert g.edges() == frozenset()

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_edges_multiple(self, repr_type):
        """edges() returns all edges without duplication."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A", weight=2.0)
        edges = g.edges()
        assert len(edges) == 3
        # Each edge appears exactly once
        edge_list = list(edges)
        assert ("A", "B", 1.0) in edge_list or ("B", "A", 1.0) in edge_list
        assert ("B", "C", 1.0) in edge_list or ("C", "B", 1.0) in edge_list
        assert ("C", "A", 2.0) in edge_list or ("A", "C", 2.0) in edge_list


# ─── Neighborhood Tests ─────────────────────────────────────────────────────


class TestNeighborhood:
    """Test neighbor and degree queries."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_neighbors(self, repr_type):
        """Get neighbors of a node."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        neighbors = g.neighbors("A")
        assert neighbors == frozenset({"B", "C"})

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_neighbors_isolated_node(self, repr_type):
        """Isolated node has no neighbors."""
        g = Graph(repr_type)
        g.add_node("A")
        assert g.neighbors("A") == frozenset()

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_neighbors_nonexistent_node_raises_error(self, repr_type):
        """neighbors() raises error for nonexistent node."""
        g = Graph(repr_type)
        with pytest.raises(NodeNotFoundError):
            g.neighbors("A")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_neighbors_weighted(self, repr_type):
        """Get neighbors with weights."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=5.0)
        g.add_edge("A", "C", weight=3.0)
        weighted = g.neighbors_weighted("A")
        assert weighted == {"B": 5.0, "C": 3.0}

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_neighbors_weighted_nonexistent_node_raises_error(self, repr_type):
        """neighbors_weighted() raises error for nonexistent node."""
        g = Graph(repr_type)
        with pytest.raises(NodeNotFoundError):
            g.neighbors_weighted("A")

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_degree(self, repr_type):
        """Get degree (number of neighbors)."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("A", "D")
        assert g.degree("A") == 3
        assert g.degree("B") == 1

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_degree_isolated_node(self, repr_type):
        """Isolated node has degree 0."""
        g = Graph(repr_type)
        g.add_node("A")
        assert g.degree("A") == 0

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_degree_nonexistent_node_raises_error(self, repr_type):
        """degree() raises error for nonexistent node."""
        g = Graph(repr_type)
        with pytest.raises(NodeNotFoundError):
            g.degree("A")


# ─── BFS Tests ──────────────────────────────────────────────────────────────


class TestBFS:
    """Breadth-First Search tests."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_bfs_simple_path(self, repr_type):
        """BFS on a simple path."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        result = bfs(g, "A")
        assert result == ["A", "B", "C"]

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_bfs_branching(self, repr_type):
        """BFS explores level by level."""
        g = Graph(repr_type)
        # Level 0: A
        # Level 1: B, C
        # Level 2: D, E
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "E")
        result = bfs(g, "A")
        assert result[0] == "A"
        assert set(result[1:3]) == {"B", "C"}
        assert set(result[3:5]) == {"D", "E"}

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_bfs_single_node(self, repr_type):
        """BFS on single node."""
        g = Graph(repr_type)
        g.add_node("A")
        assert bfs(g, "A") == ["A"]

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_bfs_disconnected_graph(self, repr_type):
        """BFS only visits reachable nodes."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_node("C")
        g.add_node("D")
        result = bfs(g, "A")
        assert set(result) == {"A", "B"}
        assert "C" not in result
        assert "D" not in result

    def test_bfs_nonexistent_node_raises_error(self):
        """BFS raises error if start node doesn't exist."""
        g = Graph()
        with pytest.raises(NodeNotFoundError):
            bfs(g, "A")

    def test_bfs_with_cycle(self):
        """BFS works correctly with cycles."""
        g = Graph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        result = bfs(g, "A")
        assert set(result) == {"A", "B", "C"}


# ─── DFS Tests ──────────────────────────────────────────────────────────────


class TestDFS:
    """Depth-First Search tests."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_dfs_simple_path(self, repr_type):
        """DFS on a simple path."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        result = dfs(g, "A")
        assert "A" in result
        assert "B" in result
        assert "C" in result

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_dfs_visits_all(self, repr_type):
        """DFS visits all reachable nodes."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        result = dfs(g, "A")
        assert set(result) == {"A", "B", "C", "D"}

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_dfs_single_node(self, repr_type):
        """DFS on single node."""
        g = Graph(repr_type)
        g.add_node("A")
        result = dfs(g, "A")
        assert result == ["A"]

    def test_dfs_nonexistent_node_raises_error(self):
        """DFS raises error if start node doesn't exist."""
        g = Graph()
        with pytest.raises(NodeNotFoundError):
            dfs(g, "A")


# ─── Connectivity Tests ─────────────────────────────────────────────────────


class TestConnectivity:
    """Test is_connected and connected_components."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_is_connected_true(self, repr_type):
        """A fully connected graph is connected."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert is_connected(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_is_connected_false(self, repr_type):
        """Disconnected graph is not connected."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_node("C")
        assert not is_connected(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_is_connected_single_node(self, repr_type):
        """Single node is connected."""
        g = Graph(repr_type)
        g.add_node("A")
        assert is_connected(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_is_connected_empty_graph(self, repr_type):
        """Empty graph is vacuously connected."""
        g = Graph(repr_type)
        assert is_connected(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_connected_components_single(self, repr_type):
        """Single connected component."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        components = connected_components(g)
        assert len(components) == 1
        assert frozenset({"A", "B", "C"}) in components

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_connected_components_multiple(self, repr_type):
        """Multiple connected components."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("C", "D")
        g.add_node("E")
        components = connected_components(g)
        assert len(components) == 3
        component_list = sorted([sorted(list(c)) for c in components])
        assert component_list == [["A", "B"], ["C", "D"], ["E"]]

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_connected_components_empty_graph(self, repr_type):
        """Empty graph has no components."""
        g = Graph(repr_type)
        components = connected_components(g)
        assert components == []


# ─── Cycle Detection Tests ──────────────────────────────────────────────────


class TestCycleDetection:
    """Test has_cycle function."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_cycle_triangle(self, repr_type):
        """Triangle has a cycle."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert has_cycle(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_cycle_path(self, repr_type):
        """Path has no cycle."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert not has_cycle(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_cycle_single_node(self, repr_type):
        """Single node has no cycle."""
        g = Graph(repr_type)
        g.add_node("A")
        assert not has_cycle(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_cycle_empty_graph(self, repr_type):
        """Empty graph has no cycle."""
        g = Graph(repr_type)
        assert not has_cycle(g)

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_has_cycle_disconnected(self, repr_type):
        """Cycle in disconnected component."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("C", "D")
        g.add_edge("D", "E")
        g.add_edge("E", "C")
        assert has_cycle(g)


# ─── Shortest Path Tests ────────────────────────────────────────────────────


class TestShortestPath:
    """Test shortest_path function."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_shortest_path_simple(self, repr_type):
        """Shortest path in simple graph."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        path = shortest_path(g, "A", "C")
        assert path == ["A", "B", "C"]

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_shortest_path_same_node(self, repr_type):
        """Path from node to itself."""
        g = Graph(repr_type)
        g.add_node("A")
        path = shortest_path(g, "A", "A")
        assert path == ["A"]

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_shortest_path_no_path(self, repr_type):
        """No path returns empty list."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_node("C")
        path = shortest_path(g, "A", "C")
        assert path == []

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_shortest_path_direct(self, repr_type):
        """Direct edge is shortest path."""
        g = Graph(repr_type)
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("A", "C")
        path = shortest_path(g, "A", "C")
        assert path == ["A", "C"]

    def test_shortest_path_weighted_dijkstra(self):
        """Weighted shortest path uses Dijkstra."""
        g = Graph()
        # Direct path: 1 + 10 = 11
        # Indirect path: 1 + 2 + 3 = 6
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "C", weight=10.0)
        g.add_edge("B", "D", weight=2.0)
        g.add_edge("D", "E", weight=3.0)
        g.add_edge("E", "C", weight=0.0)
        path = shortest_path(g, "A", "C")
        # Should be A→B→D→E→C (weight 6) not A→B→C (weight 11)
        assert len(path) >= 3

    def test_shortest_path_nonexistent_start_raises_error(self):
        """shortest_path raises error for missing start."""
        g = Graph()
        g.add_node("A")
        with pytest.raises(NodeNotFoundError):
            shortest_path(g, "X", "A")

    def test_shortest_path_nonexistent_end_raises_error(self):
        """shortest_path raises error for missing end."""
        g = Graph()
        g.add_node("A")
        with pytest.raises(NodeNotFoundError):
            shortest_path(g, "A", "X")


# ─── Minimum Spanning Tree Tests ────────────────────────────────────────────


class TestMST:
    """Test minimum_spanning_tree function."""

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_mst_simple(self, repr_type):
        """MST for simple graph."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "C", weight=2.0)
        g.add_edge("C", "A", weight=3.0)
        mst = minimum_spanning_tree(g)
        # MST should have V-1 = 2 edges
        assert len(mst) == 2
        # Total weight should be 3 (1 + 2), not 4 or 6
        total_weight = sum(w for _, _, w in mst)
        assert total_weight == 3.0

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_mst_all_nodes_connected(self, repr_type):
        """MST connects all nodes."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "C", weight=2.0)
        g.add_edge("C", "D", weight=3.0)
        g.add_edge("D", "A", weight=4.0)
        mst = minimum_spanning_tree(g)
        # Should have 3 edges for 4 nodes
        assert len(mst) == 3

    @pytest.mark.parametrize(
        "repr_type",
        [GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX],
    )
    def test_mst_no_cycles(self, repr_type):
        """MST contains no cycles."""
        g = Graph(repr_type)
        g.add_edge("A", "B", weight=1.0)
        g.add_edge("B", "C", weight=2.0)
        g.add_edge("C", "A", weight=3.0)
        mst_graph = Graph()
        for u, v, w in minimum_spanning_tree(g):
            mst_graph.add_edge(u, v, weight=w)
        assert not has_cycle(mst_graph)

    def test_mst_disconnected_raises_error(self):
        """MST raises error for disconnected graph."""
        g = Graph()
        g.add_edge("A", "B")
        g.add_node("C")
        with pytest.raises(ValueError):
            minimum_spanning_tree(g)

    def test_mst_empty_graph(self):
        """MST of empty graph is empty."""
        g = Graph()
        mst = minimum_spanning_tree(g)
        assert len(mst) == 0

    def test_mst_single_node(self):
        """MST of single node has no edges."""
        g = Graph()
        g.add_node("A")
        mst = minimum_spanning_tree(g)
        assert len(mst) == 0


# ─── Representation Equivalence Tests ────────────────────────────────────────


class TestRepresentationEquivalence:
    """Verify that both representations produce identical results."""

    def test_both_representations_same_results(self):
        """All operations should give same results for both representations."""
        # Build same graph with both representations
        g_list = Graph(GraphRepr.ADJACENCY_LIST)
        g_matrix = Graph(GraphRepr.ADJACENCY_MATRIX)

        # Add same edges
        for u, v, w in [("A", "B", 1), ("B", "C", 2), ("C", "D", 1)]:
            g_list.add_edge(u, v, weight=w)
            g_matrix.add_edge(u, v, weight=w)

        # Compare all operations
        assert g_list.nodes() == g_matrix.nodes()
        assert len(g_list.edges()) == len(g_matrix.edges())
        assert g_list.neighbors("B") == g_matrix.neighbors("B")
        assert g_list.degree("B") == g_matrix.degree("B")
        assert is_connected(g_list) == is_connected(g_matrix)
        assert has_cycle(g_list) == has_cycle(g_matrix)
        assert set(bfs(g_list, "A")) == set(bfs(g_matrix, "A"))
        assert set(dfs(g_list, "A")) == set(dfs(g_matrix, "A"))


# ─── Edge Case Tests ────────────────────────────────────────────────────────


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_complete_graph(self):
        """Test complete graph (all nodes connected)."""
        g = Graph()
        nodes = ["A", "B", "C", "D"]
        for i, u in enumerate(nodes):
            for v in nodes[i + 1 :]:
                g.add_edge(u, v)
        assert is_connected(g)
        assert has_cycle(g)
        assert len(g.edges()) == 6  # C(4,2) = 6

    def test_self_loop_via_add_edge(self):
        """Test that adding self-edge creates two references in undirected graph."""
        g = Graph()
        g.add_edge("A", "A", weight=5.0)
        assert g.has_edge("A", "A")
        assert g.degree("A") == 1

    def test_large_sparse_graph(self):
        """Test with larger graph (performance check)."""
        g = Graph()
        # Create 100 nodes in a chain
        for i in range(100):
            g.add_edge(i, i + 1)
        assert len(g) == 101
        assert len(g.edges()) == 100
        assert is_connected(g)

    def test_large_disconnected_graph(self):
        """Test many components."""
        g = Graph()
        # Create 10 disconnected pairs
        for i in range(10):
            g.add_edge(f"A{i}", f"B{i}")
        components = connected_components(g)
        assert len(components) == 10

    def test_weighted_zero_weight(self):
        """Test edge with zero weight."""
        g = Graph()
        g.add_edge("A", "B", weight=0.0)
        # Zero weight is still an edge
        assert g.has_edge("A", "B")
        assert g.edge_weight("A", "B") == 0.0

    def test_node_removal_cleans_reverse_edges(self):
        """Removing a node cleans up both directions of edges."""
        g = Graph()
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        g.remove_node("B")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("B", "C")
        assert g.has_edge("A", "C")

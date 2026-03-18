"""
test_graph.py -- Tests for DirectedGraph Core Operations
=========================================================

These tests cover the fundamental data structure operations of the directed
graph: adding/removing nodes and edges, querying neighbors, and error handling.
We leave algorithm tests (topological sort, cycle detection, etc.) to
``test_algorithms.py`` so each file has a clear focus.

The tests are organized from simplest to most complex, following a natural
progression that mirrors how you'd learn the API:

1. Empty graph behavior
2. Single node operations
3. Single edge operations
4. Multi-node/edge operations
5. Error conditions
6. Edge cases (duplicates, repr, etc.)

Each test function is named to describe WHAT it tests and WHAT the expected
outcome is, so you can read the test names as a specification of the graph's
behavior.
"""

import pytest

from directed_graph import (
    DirectedGraph,
    EdgeNotFoundError,
    NodeNotFoundError,
)


# ======================================================================
# 1. Empty Graph
# ======================================================================
# An empty graph has no nodes and no edges. All query methods should return
# empty collections rather than raising errors.


class TestEmptyGraph:
    """Verify that a freshly created graph behaves correctly."""

    def test_empty_graph_has_no_nodes(self) -> None:
        """An empty graph should report zero nodes."""
        g = DirectedGraph()
        assert g.nodes() == []
        assert len(g) == 0

    def test_empty_graph_has_no_edges(self) -> None:
        """An empty graph should report zero edges."""
        g = DirectedGraph()
        assert g.edges() == []

    def test_empty_graph_has_node_returns_false(self) -> None:
        """has_node should return False for any node in an empty graph."""
        g = DirectedGraph()
        assert g.has_node("A") is False
        assert "A" not in g

    def test_empty_graph_has_edge_returns_false(self) -> None:
        """has_edge should return False for any edge in an empty graph."""
        g = DirectedGraph()
        assert g.has_edge("A", "B") is False


# ======================================================================
# 2. Single Node
# ======================================================================
# Adding a single node is the simplest mutation. We test that it appears
# in the graph and can be removed.


class TestSingleNode:
    """Test adding, querying, and removing a single node."""

    def test_add_node_makes_it_present(self) -> None:
        """After add_node, the node should be findable."""
        g = DirectedGraph()
        g.add_node("A")
        assert g.has_node("A") is True
        assert "A" in g
        assert len(g) == 1
        assert g.nodes() == ["A"]

    def test_remove_node_makes_it_absent(self) -> None:
        """After remove_node, the node should be gone."""
        g = DirectedGraph()
        g.add_node("A")
        g.remove_node("A")
        assert g.has_node("A") is False
        assert len(g) == 0

    def test_add_node_is_idempotent(self) -> None:
        """Adding the same node twice should not create duplicates."""
        g = DirectedGraph()
        g.add_node("A")
        g.add_node("A")  # Should be a no-op
        assert len(g) == 1
        assert g.nodes() == ["A"]

    def test_predecessors_of_isolated_node(self) -> None:
        """An isolated node has no predecessors."""
        g = DirectedGraph()
        g.add_node("A")
        assert g.predecessors("A") == []

    def test_successors_of_isolated_node(self) -> None:
        """An isolated node has no successors."""
        g = DirectedGraph()
        g.add_node("A")
        assert g.successors("A") == []


# ======================================================================
# 3. Single Edge
# ======================================================================
# An edge implicitly adds both endpoints, so add_edge("A", "B") creates
# nodes A and B plus the edge A -> B.


class TestSingleEdge:
    """Test adding a single edge and querying its properties."""

    def test_add_edge_creates_both_nodes(self) -> None:
        """add_edge should implicitly add both endpoint nodes."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert g.has_node("A")
        assert g.has_node("B")
        assert len(g) == 2

    def test_add_edge_creates_the_edge(self) -> None:
        """After add_edge, has_edge should return True."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert g.has_edge("A", "B") is True
        # The reverse direction should NOT exist.
        assert g.has_edge("B", "A") is False

    def test_edges_returns_the_edge(self) -> None:
        """edges() should include the added edge."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert g.edges() == [("A", "B")]

    def test_predecessors_and_successors(self) -> None:
        """For edge A -> B: A's successor is B, B's predecessor is A."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert g.successors("A") == ["B"]
        assert g.predecessors("B") == ["A"]
        # A has no predecessors, B has no successors.
        assert g.predecessors("A") == []
        assert g.successors("B") == []

    def test_remove_edge(self) -> None:
        """remove_edge should delete the edge but keep both nodes."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        assert g.has_edge("A", "B") is False
        # Both nodes should still exist.
        assert g.has_node("A")
        assert g.has_node("B")

    def test_duplicate_edge_is_idempotent(self) -> None:
        """Adding the same edge twice should not create duplicates."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("A", "B")  # Should be a no-op
        assert g.edges() == [("A", "B")]


# ======================================================================
# 4. Multi-Node Operations
# ======================================================================
# Test more complex graph structures to make sure the adjacency dicts
# stay consistent.


class TestMultiNodeOperations:
    """Test graphs with multiple nodes and edges."""

    def test_remove_node_cleans_up_edges(self) -> None:
        """Removing a node should remove all its incoming and outgoing edges.

        Given A -> B -> C, removing B should leave A and C as isolated nodes
        with no edges between them.
        """
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.remove_node("B")

        assert not g.has_node("B")
        assert g.has_node("A")
        assert g.has_node("C")
        assert g.edges() == []
        assert g.successors("A") == []
        assert g.predecessors("C") == []

    def test_remove_node_with_multiple_edges(self) -> None:
        """Removing a hub node should clean up all connected edges.

        Given A -> B, C -> B, B -> D, removing B should leave A, C, D
        with no edges.
        """
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("C", "B")
        g.add_edge("B", "D")
        g.remove_node("B")

        assert len(g) == 3
        assert g.edges() == []


# ======================================================================
# 5. Error Conditions
# ======================================================================
# These tests verify that the graph raises the right exceptions for
# invalid operations.


class TestErrorConditions:
    """Test that invalid operations raise appropriate errors."""

    def test_self_loop_raises_value_error(self) -> None:
        """A self-loop (A -> A) should raise ValueError."""
        g = DirectedGraph()
        with pytest.raises(ValueError, match="Self-loops are not allowed"):
            g.add_edge("A", "A")

    def test_remove_nonexistent_node_raises(self) -> None:
        """Removing a node that doesn't exist should raise NodeNotFoundError."""
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.remove_node("X")

    def test_node_not_found_error_has_node_attribute(self) -> None:
        """NodeNotFoundError should carry the missing node value."""
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError) as exc_info:
            g.remove_node("X")
        assert exc_info.value.node == "X"

    def test_remove_nonexistent_edge_raises(self) -> None:
        """Removing an edge that doesn't exist should raise EdgeNotFoundError."""
        g = DirectedGraph()
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(EdgeNotFoundError):
            g.remove_edge("A", "B")

    def test_edge_not_found_error_has_attributes(self) -> None:
        """EdgeNotFoundError should carry both node values."""
        g = DirectedGraph()
        with pytest.raises(EdgeNotFoundError) as exc_info:
            g.remove_edge("X", "Y")
        assert exc_info.value.from_node == "X"
        assert exc_info.value.to_node == "Y"

    def test_predecessors_of_nonexistent_node_raises(self) -> None:
        """predecessors() should raise NodeNotFoundError for missing nodes."""
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.predecessors("X")

    def test_successors_of_nonexistent_node_raises(self) -> None:
        """successors() should raise NodeNotFoundError for missing nodes."""
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.successors("X")


# ======================================================================
# 6. Edge Cases and Repr
# ======================================================================


class TestEdgeCases:
    """Test repr, contains, and other edge cases."""

    def test_repr_shows_counts(self) -> None:
        """__repr__ should show node and edge counts."""
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert repr(g) == "DirectedGraph(nodes=2, edges=1)"

    def test_contains_is_same_as_has_node(self) -> None:
        """The 'in' operator should work like has_node."""
        g = DirectedGraph()
        g.add_node("A")
        assert ("A" in g) is True
        assert ("B" in g) is False

    def test_integer_nodes(self) -> None:
        """The graph should work with integer nodes, not just strings."""
        g = DirectedGraph()
        g.add_edge(1, 2)
        g.add_edge(2, 3)
        assert g.has_edge(1, 2)
        assert g.successors(1) == [2]
        assert len(g) == 3

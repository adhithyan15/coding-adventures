"""Tests for the native (Rust-backed) directed graph.

These tests mirror the pure Python directed_graph test suite to ensure
the native extension provides identical behavior. If these tests pass,
the native extension is a valid drop-in replacement.
"""

from __future__ import annotations

import pytest

from directed_graph_native import (
    CycleError,
    DirectedGraph,
    EdgeNotFoundError,
    NodeNotFoundError,
)


# ---------------------------------------------------------------------------
# Node operations
# ---------------------------------------------------------------------------


class TestNodeOperations:
    def test_add_and_has_node(self):
        g = DirectedGraph()
        g.add_node("A")
        assert g.has_node("A")
        assert not g.has_node("B")

    def test_add_duplicate_node_is_noop(self):
        g = DirectedGraph()
        g.add_node("A")
        g.add_node("A")  # should not raise
        assert len(g) == 1

    def test_remove_node(self):
        g = DirectedGraph()
        g.add_node("A")
        g.remove_node("A")
        assert not g.has_node("A")

    def test_remove_nonexistent_node_raises(self):
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.remove_node("X")

    def test_remove_node_cleans_up_edges(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.remove_node("B")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("B", "C")
        assert g.has_node("A")
        assert g.has_node("C")

    def test_nodes_returns_sorted_list(self):
        g = DirectedGraph()
        g.add_node("C")
        g.add_node("A")
        g.add_node("B")
        assert g.nodes() == ["A", "B", "C"]

    def test_len(self):
        g = DirectedGraph()
        assert len(g) == 0
        g.add_node("A")
        assert len(g) == 1
        g.add_node("B")
        assert len(g) == 2

    def test_contains(self):
        g = DirectedGraph()
        g.add_node("A")
        assert "A" in g
        assert "B" not in g


# ---------------------------------------------------------------------------
# Edge operations
# ---------------------------------------------------------------------------


class TestEdgeOperations:
    def test_add_and_has_edge(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert g.has_edge("A", "B")
        assert not g.has_edge("B", "A")  # directed!

    def test_add_edge_creates_nodes(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        assert g.has_node("A")
        assert g.has_node("B")

    def test_remove_edge(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        assert not g.has_edge("A", "B")
        # Nodes should still exist after edge removal
        assert g.has_node("A")
        assert g.has_node("B")

    def test_remove_nonexistent_edge_raises(self):
        g = DirectedGraph()
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(EdgeNotFoundError):
            g.remove_edge("A", "B")

    def test_self_loop_raises(self):
        g = DirectedGraph()
        with pytest.raises(ValueError):
            g.add_edge("A", "A")

    def test_edges_returns_sorted_list(self):
        g = DirectedGraph()
        g.add_edge("B", "C")
        g.add_edge("A", "B")
        edges = g.edges()
        assert edges == [("A", "B"), ("B", "C")]


# ---------------------------------------------------------------------------
# Neighbor queries
# ---------------------------------------------------------------------------


class TestNeighborQueries:
    def test_predecessors(self):
        g = DirectedGraph()
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        preds = g.predecessors("C")
        assert sorted(preds) == ["A", "B"]

    def test_successors(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        succs = g.successors("A")
        assert sorted(succs) == ["B", "C"]

    def test_predecessors_nonexistent_raises(self):
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.predecessors("X")

    def test_successors_nonexistent_raises(self):
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.successors("X")


# ---------------------------------------------------------------------------
# Topological sort
# ---------------------------------------------------------------------------


class TestTopologicalSort:
    def test_linear_chain(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        order = g.topological_sort()
        assert order == ["A", "B", "C"]

    def test_diamond(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        order = g.topological_sort()
        # A must come first, D must come last
        assert order[0] == "A"
        assert order[-1] == "D"
        # B and C can be in either order
        assert set(order[1:3]) == {"B", "C"}

    def test_cycle_raises(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        with pytest.raises(CycleError):
            g.topological_sort()

    def test_empty_graph(self):
        g = DirectedGraph()
        assert g.topological_sort() == []

    def test_single_node(self):
        g = DirectedGraph()
        g.add_node("A")
        assert g.topological_sort() == ["A"]


# ---------------------------------------------------------------------------
# Cycle detection
# ---------------------------------------------------------------------------


class TestCycleDetection:
    def test_no_cycle(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert not g.has_cycle()

    def test_has_cycle(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert g.has_cycle()

    def test_empty_graph_no_cycle(self):
        g = DirectedGraph()
        assert not g.has_cycle()


# ---------------------------------------------------------------------------
# Transitive closure
# ---------------------------------------------------------------------------


class TestTransitiveClosure:
    def test_linear_chain(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        closure = g.transitive_closure("A")
        assert closure == {"B", "C"}

    def test_diamond(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        closure = g.transitive_closure("A")
        assert closure == {"B", "C", "D"}

    def test_leaf_node_has_empty_closure(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        closure = g.transitive_closure("B")
        assert closure == set()

    def test_nonexistent_node_raises(self):
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.transitive_closure("X")


# ---------------------------------------------------------------------------
# Affected nodes
# ---------------------------------------------------------------------------


class TestAffectedNodes:
    def test_single_change_propagates(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        affected = g.affected_nodes({"A"})
        assert affected == {"A", "B", "C"}

    def test_leaf_change_affects_only_itself(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        affected = g.affected_nodes({"B"})
        assert affected == {"B"}

    def test_diamond_change_at_root(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        affected = g.affected_nodes({"A"})
        assert affected == {"A", "B", "C", "D"}

    def test_unknown_nodes_included(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        # "X" doesn't exist in graph but should still appear in result
        affected = g.affected_nodes({"X"})
        assert "X" in affected


# ---------------------------------------------------------------------------
# Independent groups
# ---------------------------------------------------------------------------


class TestIndependentGroups:
    def test_linear_chain(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        groups = g.independent_groups()
        assert groups == [["A"], ["B"], ["C"]]

    def test_diamond(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        groups = g.independent_groups()
        assert len(groups) == 3
        assert groups[0] == ["A"]
        assert sorted(groups[1]) == ["B", "C"]
        assert groups[2] == ["D"]

    def test_parallel_roots(self):
        g = DirectedGraph()
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        groups = g.independent_groups()
        assert len(groups) == 1
        assert sorted(groups[0]) == ["A", "B", "C"]

    def test_cycle_raises(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        with pytest.raises(CycleError):
            g.independent_groups()

    def test_empty_graph(self):
        g = DirectedGraph()
        assert g.independent_groups() == []


# ---------------------------------------------------------------------------
# Repr
# ---------------------------------------------------------------------------


class TestRepr:
    def test_repr(self):
        g = DirectedGraph()
        g.add_edge("A", "B")
        r = repr(g)
        assert "DirectedGraph" in r
        assert "nodes=2" in r

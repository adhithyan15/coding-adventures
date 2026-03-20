"""
test_labeled_graph.py -- Tests for LabeledDirectedGraph
========================================================

These tests cover the labeled directed graph, which adds string labels to edges.
The tests progress from basic operations to complex scenarios:

1. Empty graph behavior
2. Single labeled edge
3. Multiple labels per edge pair
4. Self-loops with labels
5. Node removal and label cleanup
6. Edge removal with label tracking
7. Label filtering on successors/predecessors
8. Algorithm delegation
9. Error conditions
10. Edge cases and repr
11. Integration tests with realistic data

Each test verifies a specific behavior contract of the LabeledDirectedGraph API.
"""

import pytest

from directed_graph import (
    EdgeNotFoundError,
    LabeledDirectedGraph,
    NodeNotFoundError,
)


# ======================================================================
# 1. Empty Graph
# ======================================================================


class TestEmptyLabeledGraph:
    """Verify that a freshly created labeled graph behaves correctly."""

    def test_empty_graph_has_no_nodes(self) -> None:
        """An empty labeled graph has zero nodes."""
        lg = LabeledDirectedGraph()
        assert lg.nodes() == []
        assert len(lg) == 0

    def test_empty_graph_has_no_edges(self) -> None:
        """An empty labeled graph has zero edges."""
        lg = LabeledDirectedGraph()
        assert lg.edges() == []

    def test_empty_graph_has_node_returns_false(self) -> None:
        """has_node returns False for any node in an empty graph."""
        lg = LabeledDirectedGraph()
        assert lg.has_node("A") is False
        assert "A" not in lg

    def test_empty_graph_has_edge_returns_false(self) -> None:
        """has_edge returns False for any edge in an empty graph."""
        lg = LabeledDirectedGraph()
        assert lg.has_edge("A", "B") is False
        assert lg.has_edge("A", "B", "x") is False

    def test_empty_graph_labels_returns_empty_set(self) -> None:
        """labels() returns an empty set for nonexistent edges."""
        lg = LabeledDirectedGraph()
        assert lg.labels("A", "B") == set()


# ======================================================================
# 2. Single Labeled Edge
# ======================================================================


class TestSingleLabeledEdge:
    """Test adding a single labeled edge."""

    def test_add_edge_creates_nodes(self) -> None:
        """add_edge auto-creates both endpoint nodes."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.has_node("A")
        assert lg.has_node("B")
        assert len(lg) == 2

    def test_add_edge_creates_labeled_edge(self) -> None:
        """After add_edge, has_edge returns True for the label."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.has_edge("A", "B", "friend") is True

    def test_has_edge_without_label(self) -> None:
        """has_edge without label checks for ANY label between the nodes."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.has_edge("A", "B") is True

    def test_has_edge_wrong_label(self) -> None:
        """has_edge with a non-existent label returns False."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.has_edge("A", "B", "enemy") is False

    def test_has_edge_wrong_direction(self) -> None:
        """has_edge in the reverse direction returns False."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.has_edge("B", "A") is False
        assert lg.has_edge("B", "A", "friend") is False

    def test_edges_returns_triple(self) -> None:
        """edges() returns (from, to, label) triples."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.edges() == [("A", "B", "friend")]

    def test_labels_returns_label_set(self) -> None:
        """labels() returns the set of labels on an edge."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.labels("A", "B") == {"friend"}

    def test_successors_of_source(self) -> None:
        """The source node's successors include the target."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.successors("A") == ["B"]

    def test_predecessors_of_target(self) -> None:
        """The target node's predecessors include the source."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.predecessors("B") == ["A"]


# ======================================================================
# 3. Multiple Labels Per Edge Pair
# ======================================================================


class TestMultipleLabels:
    """Test adding multiple labels to the same edge pair."""

    def test_two_labels_on_same_edge(self) -> None:
        """Two labels can exist on the same (from, to) pair."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        lg.add_edge("A", "B", "coworker")
        assert lg.labels("A", "B") == {"friend", "coworker"}

    def test_multiple_labels_one_structural_edge(self) -> None:
        """Multiple labels share one structural edge in the inner graph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        lg.add_edge("A", "B", "coworker")
        # The inner graph should only have one edge A -> B.
        assert len(lg.graph.edges()) == 1

    def test_edges_returns_one_triple_per_label(self) -> None:
        """edges() returns a separate triple for each label."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        lg.add_edge("A", "B", "coworker")
        edges = lg.edges()
        assert len(edges) == 2
        assert ("A", "B", "coworker") in edges
        assert ("A", "B", "friend") in edges

    def test_has_edge_checks_specific_label(self) -> None:
        """has_edge with label checks only that specific label."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        lg.add_edge("A", "B", "coworker")
        assert lg.has_edge("A", "B", "friend") is True
        assert lg.has_edge("A", "B", "coworker") is True
        assert lg.has_edge("A", "B", "enemy") is False

    def test_add_same_label_twice_is_idempotent(self) -> None:
        """Adding the same label twice is a no-op."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        lg.add_edge("A", "B", "friend")
        assert lg.labels("A", "B") == {"friend"}
        assert len(lg.edges()) == 1

    def test_three_labels_on_same_edge(self) -> None:
        """Three distinct labels on a single edge pair."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("A", "B", "y")
        lg.add_edge("A", "B", "z")
        assert lg.labels("A", "B") == {"x", "y", "z"}
        assert len(lg.edges()) == 3

    def test_labels_returns_copy(self) -> None:
        """labels() returns a copy, not the internal set."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        labels = lg.labels("A", "B")
        labels.add("hacked")
        assert lg.labels("A", "B") == {"x"}


# ======================================================================
# 4. Self-Loops with Labels
# ======================================================================


class TestSelfLoopsLabeled:
    """Test self-loops in the labeled graph (always enabled)."""

    def test_self_loop_can_be_added(self) -> None:
        """Self-loops are always allowed in labeled graphs."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "reflexive")
        assert lg.has_edge("A", "A", "reflexive") is True

    def test_self_loop_appears_in_edges(self) -> None:
        """Self-loop edges appear in the edges() list."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        assert ("A", "A", "loop") in lg.edges()

    def test_self_loop_node_is_own_successor(self) -> None:
        """A self-loop makes the node its own successor."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        assert "A" in lg.successors("A")

    def test_self_loop_node_is_own_predecessor(self) -> None:
        """A self-loop makes the node its own predecessor."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        assert "A" in lg.predecessors("A")

    def test_self_loop_creates_cycle(self) -> None:
        """A self-loop is a cycle."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        assert lg.has_cycle() is True

    def test_self_loop_multiple_labels(self) -> None:
        """Multiple labels on a self-loop edge."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "x")
        lg.add_edge("A", "A", "y")
        assert lg.labels("A", "A") == {"x", "y"}

    def test_self_loop_with_normal_edge(self) -> None:
        """Self-loops coexist with normal edges."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "self")
        lg.add_edge("A", "B", "other")
        assert lg.has_edge("A", "A", "self")
        assert lg.has_edge("A", "B", "other")
        assert len(lg.edges()) == 2


# ======================================================================
# 5. Node Removal and Label Cleanup
# ======================================================================


class TestNodeRemoval:
    """Test that removing a node cleans up all associated labels."""

    def test_remove_node_cleans_outgoing_labels(self) -> None:
        """Removing a source node cleans up labels on outgoing edges."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.remove_node("A")
        assert not lg.has_node("A")
        assert lg.has_node("B")
        assert lg.labels("A", "B") == set()
        assert lg.edges() == []

    def test_remove_node_cleans_incoming_labels(self) -> None:
        """Removing a target node cleans up labels on incoming edges."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.remove_node("B")
        assert lg.has_node("A")
        assert not lg.has_node("B")
        assert lg.labels("A", "B") == set()

    def test_remove_hub_node_cleans_all_labels(self) -> None:
        """Removing a hub node cleans up all connected labels."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        lg.add_edge("B", "C", "z")
        lg.remove_node("B")
        assert lg.edges() == []
        assert lg.has_node("A")
        assert lg.has_node("C")

    def test_remove_node_with_self_loop(self) -> None:
        """Removing a node with a self-loop cleans up the self-loop label."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        lg.add_edge("A", "B", "out")
        lg.remove_node("A")
        assert not lg.has_node("A")
        assert lg.has_node("B")
        assert lg.edges() == []

    def test_remove_nonexistent_node_raises(self) -> None:
        """Removing a node that doesn't exist raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.remove_node("X")


# ======================================================================
# 6. Edge Removal with Label Tracking
# ======================================================================


class TestEdgeRemoval:
    """Test removing specific labeled edges."""

    def test_remove_only_label_removes_structural_edge(self) -> None:
        """Removing the only label also removes the structural edge."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.remove_edge("A", "B", "x")
        assert not lg.has_edge("A", "B")
        assert not lg.has_edge("A", "B", "x")
        assert lg.edges() == []
        # Nodes should still exist.
        assert lg.has_node("A")
        assert lg.has_node("B")

    def test_remove_one_of_two_labels(self) -> None:
        """Removing one label keeps the other and the structural edge."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("A", "B", "y")
        lg.remove_edge("A", "B", "x")
        assert lg.has_edge("A", "B", "y") is True
        assert lg.has_edge("A", "B", "x") is False
        assert lg.has_edge("A", "B") is True  # still connected
        assert lg.labels("A", "B") == {"y"}

    def test_remove_all_labels_sequentially(self) -> None:
        """Removing all labels one by one removes the structural edge."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("A", "B", "y")
        lg.remove_edge("A", "B", "x")
        lg.remove_edge("A", "B", "y")
        assert not lg.has_edge("A", "B")

    def test_remove_nonexistent_label_raises(self) -> None:
        """Removing a label that doesn't exist raises EdgeNotFoundError."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        with pytest.raises(EdgeNotFoundError):
            lg.remove_edge("A", "B", "nonexistent")

    def test_remove_edge_from_nonexistent_pair_raises(self) -> None:
        """Removing an edge between unconnected nodes raises EdgeNotFoundError."""
        lg = LabeledDirectedGraph()
        lg.add_node("A")
        lg.add_node("B")
        with pytest.raises(EdgeNotFoundError):
            lg.remove_edge("A", "B", "x")

    def test_remove_self_loop_label(self) -> None:
        """Removing a self-loop label works correctly."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        lg.remove_edge("A", "A", "loop")
        assert not lg.has_edge("A", "A")
        assert lg.has_node("A")


# ======================================================================
# 7. Label Filtering on Successors/Predecessors
# ======================================================================


class TestLabelFiltering:
    """Test filtered neighbor queries."""

    def test_successors_unfiltered(self) -> None:
        """successors() without label returns all successors."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("A", "C", "y")
        result = sorted(lg.successors("A"))
        assert result == ["B", "C"]

    def test_successors_filtered_by_label(self) -> None:
        """successors() with label returns only matching successors."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        lg.add_edge("A", "C", "coworker")
        lg.add_edge("A", "D", "friend")
        result = sorted(lg.successors("A", "friend"))
        assert result == ["B", "D"]

    def test_successors_filtered_no_match(self) -> None:
        """successors() with non-matching label returns empty list."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.successors("A", "enemy") == []

    def test_predecessors_unfiltered(self) -> None:
        """predecessors() without label returns all predecessors."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "C", "x")
        lg.add_edge("B", "C", "y")
        result = sorted(lg.predecessors("C"))
        assert result == ["A", "B"]

    def test_predecessors_filtered_by_label(self) -> None:
        """predecessors() with label returns only matching predecessors."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "C", "friend")
        lg.add_edge("B", "C", "coworker")
        lg.add_edge("D", "C", "friend")
        result = sorted(lg.predecessors("C", "friend"))
        assert result == ["A", "D"]

    def test_predecessors_filtered_no_match(self) -> None:
        """predecessors() with non-matching label returns empty list."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "friend")
        assert lg.predecessors("B", "enemy") == []

    def test_successors_nonexistent_node_raises(self) -> None:
        """successors() on a missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.successors("X")

    def test_predecessors_nonexistent_node_raises(self) -> None:
        """predecessors() on a missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.predecessors("X")

    def test_successors_filtered_nonexistent_node_raises(self) -> None:
        """successors() with label on a missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.successors("X", "friend")

    def test_predecessors_filtered_nonexistent_node_raises(self) -> None:
        """predecessors() with label on missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.predecessors("X", "friend")

    def test_self_loop_in_filtered_successors(self) -> None:
        """Self-loop appears in label-filtered successors if label matches."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "self")
        lg.add_edge("A", "B", "other")
        assert lg.successors("A", "self") == ["A"]
        assert lg.successors("A", "other") == ["B"]

    def test_self_loop_in_filtered_predecessors(self) -> None:
        """Self-loop appears in label-filtered predecessors if label matches."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "self")
        lg.add_edge("B", "A", "other")
        assert lg.predecessors("A", "self") == ["A"]
        assert lg.predecessors("A", "other") == ["B"]


# ======================================================================
# 8. Algorithm Delegation
# ======================================================================


class TestAlgorithmDelegation:
    """Test that graph algorithms work through delegation."""

    def test_topological_sort_dag(self) -> None:
        """Topological sort works on a labeled DAG."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "dep")
        lg.add_edge("B", "C", "dep")
        assert lg.topological_sort() == ["A", "B", "C"]

    def test_topological_sort_with_multiple_labels(self) -> None:
        """Multiple labels don't affect topological ordering."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "compile")
        lg.add_edge("A", "B", "test")
        lg.add_edge("B", "C", "compile")
        assert lg.topological_sort() == ["A", "B", "C"]

    def test_has_cycle_false_for_dag(self) -> None:
        """A labeled DAG has no cycle."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        assert lg.has_cycle() is False

    def test_has_cycle_true_for_cycle(self) -> None:
        """A labeled graph with a cycle is detected."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        lg.add_edge("C", "A", "z")
        assert lg.has_cycle() is True

    def test_has_cycle_true_for_self_loop(self) -> None:
        """A self-loop is a cycle."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "loop")
        assert lg.has_cycle() is True

    def test_transitive_closure(self) -> None:
        """transitive_closure works through the labeled graph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        lg.add_edge("B", "D", "z")
        assert lg.transitive_closure("A") == {"B", "C", "D"}

    def test_transitive_closure_with_self_loop(self) -> None:
        """transitive_closure includes self-loop nodes."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "A", "self")
        lg.add_edge("A", "B", "next")
        assert lg.transitive_closure("A") == {"A", "B"}

    def test_transitive_dependents(self) -> None:
        """transitive_dependents works through the labeled graph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        assert lg.transitive_dependents("C") == {"A", "B"}

    def test_graph_property_returns_inner_graph(self) -> None:
        """The graph property exposes the inner DirectedGraph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        g = lg.graph
        assert g.has_node("A")
        assert g.has_edge("A", "B")

    def test_independent_groups_via_inner_graph(self) -> None:
        """independent_groups works via the inner graph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("A", "C", "y")
        lg.add_edge("B", "D", "z")
        lg.add_edge("C", "D", "w")
        groups = lg.graph.independent_groups()
        assert groups[0] == ["A"]
        assert sorted(groups[1]) == ["B", "C"]
        assert groups[2] == ["D"]

    def test_affected_nodes_via_inner_graph(self) -> None:
        """affected_nodes works via the inner graph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        affected = lg.graph.affected_nodes({"C"})
        assert affected == {"A", "B", "C"}


# ======================================================================
# 9. Error Conditions
# ======================================================================


class TestLabeledGraphErrors:
    """Test error handling in the labeled graph."""

    def test_remove_nonexistent_node(self) -> None:
        """Removing a missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.remove_node("X")

    def test_remove_edge_nonexistent_nodes(self) -> None:
        """Removing an edge between nonexistent nodes raises EdgeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(EdgeNotFoundError):
            lg.remove_edge("A", "B", "x")

    def test_remove_edge_wrong_label(self) -> None:
        """Removing a label that doesn't exist on an existing edge raises."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        with pytest.raises(EdgeNotFoundError):
            lg.remove_edge("A", "B", "y")

    def test_topological_sort_cycle_raises(self) -> None:
        """topological_sort on a cyclic graph raises CycleError."""
        from directed_graph import CycleError

        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "A", "y")
        with pytest.raises(CycleError):
            lg.topological_sort()

    def test_transitive_closure_nonexistent_node(self) -> None:
        """transitive_closure on a missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.transitive_closure("X")

    def test_transitive_dependents_nonexistent_node(self) -> None:
        """transitive_dependents on a missing node raises NodeNotFoundError."""
        lg = LabeledDirectedGraph()
        with pytest.raises(NodeNotFoundError):
            lg.transitive_dependents("X")


# ======================================================================
# 10. Edge Cases and Repr
# ======================================================================


class TestLabeledEdgeCases:
    """Test repr, contains, and other edge cases."""

    def test_repr_shows_counts(self) -> None:
        """repr shows node and labeled edge counts."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("A", "B", "y")
        assert repr(lg) == "LabeledDirectedGraph(nodes=2, labeled_edges=2)"

    def test_contains_operator(self) -> None:
        """The 'in' operator works for node membership."""
        lg = LabeledDirectedGraph()
        lg.add_node("A")
        assert "A" in lg
        assert "B" not in lg

    def test_add_isolated_node(self) -> None:
        """add_node adds a node with no edges."""
        lg = LabeledDirectedGraph()
        lg.add_node("A")
        assert lg.has_node("A")
        assert lg.edges() == []
        assert lg.successors("A") == []
        assert lg.predecessors("A") == []

    def test_add_node_is_idempotent(self) -> None:
        """Adding the same node twice is a no-op."""
        lg = LabeledDirectedGraph()
        lg.add_node("A")
        lg.add_node("A")
        assert len(lg) == 1

    def test_edges_sorted_by_label(self) -> None:
        """edges() sorts labels alphabetically within each edge pair."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "z")
        lg.add_edge("A", "B", "a")
        edges = lg.edges()
        assert edges[0] == ("A", "B", "a")
        assert edges[1] == ("A", "B", "z")

    def test_integer_nodes(self) -> None:
        """The graph works with integer nodes."""
        lg = LabeledDirectedGraph()
        lg.add_edge(1, 2, "next")
        assert lg.has_edge(1, 2, "next")
        assert lg.successors(1) == [2]

    def test_empty_string_label(self) -> None:
        """An empty string is a valid label."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "")
        assert lg.has_edge("A", "B", "")
        assert lg.labels("A", "B") == {""}


# ======================================================================
# 11. Integration: Knowledge Graph Example
# ======================================================================


class TestKnowledgeGraphIntegration:
    """Test with a realistic knowledge graph scenario.

    We model a small social network:
    - Alice is friends with Bob and Carol
    - Alice is a coworker of Bob
    - Bob is friends with Dave
    - Dave follows Alice
    """

    @pytest.fixture()
    def social_graph(self) -> LabeledDirectedGraph:
        """Build the social network graph."""
        lg = LabeledDirectedGraph()
        lg.add_edge("Alice", "Bob", "friend")
        lg.add_edge("Alice", "Bob", "coworker")
        lg.add_edge("Alice", "Carol", "friend")
        lg.add_edge("Bob", "Dave", "friend")
        lg.add_edge("Dave", "Alice", "follows")
        return lg

    def test_social_graph_node_count(self, social_graph: LabeledDirectedGraph) -> None:
        """The social graph has 4 people."""
        assert len(social_graph) == 4

    def test_social_graph_edge_count(self, social_graph: LabeledDirectedGraph) -> None:
        """The social graph has 5 labeled edges."""
        assert len(social_graph.edges()) == 5

    def test_alice_friends(self, social_graph: LabeledDirectedGraph) -> None:
        """Alice's friends are Bob and Carol."""
        friends = sorted(social_graph.successors("Alice", "friend"))
        assert friends == ["Bob", "Carol"]

    def test_alice_coworkers(self, social_graph: LabeledDirectedGraph) -> None:
        """Alice's coworker is Bob."""
        coworkers = social_graph.successors("Alice", "coworker")
        assert coworkers == ["Bob"]

    def test_alice_bob_labels(self, social_graph: LabeledDirectedGraph) -> None:
        """Alice and Bob have two relationships."""
        assert social_graph.labels("Alice", "Bob") == {"friend", "coworker"}

    def test_social_graph_has_cycle(self, social_graph: LabeledDirectedGraph) -> None:
        """The graph has a cycle: Alice -> Bob -> Dave -> Alice."""
        assert social_graph.has_cycle() is True

    def test_transitive_closure_from_alice(
        self, social_graph: LabeledDirectedGraph
    ) -> None:
        """Alice can reach Bob, Carol, Dave, and back to herself (cycle)."""
        closure = social_graph.transitive_closure("Alice")
        assert closure == {"Bob", "Carol", "Dave", "Alice"}

    def test_remove_alice_cleans_everything(
        self, social_graph: LabeledDirectedGraph
    ) -> None:
        """Removing Alice cleans up all her edges and labels."""
        social_graph.remove_node("Alice")
        assert not social_graph.has_node("Alice")
        # Only Bob -> Dave should remain.
        assert len(social_graph.edges()) == 1
        assert social_graph.has_edge("Bob", "Dave", "friend")

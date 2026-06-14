"""
test_directed_graph.py — Comprehensive tests for DT01 DirectedGraph package
=============================================================================

Tests are organized into classes matching the DT01 spec:

  TestDirectedGraphBasic           — construction, nodes, edges, self-loops
  TestSuccessorsPredecessors       — directional neighbor queries, degree
  TestNeighborsOverride            — neighbors() returns successors only
  TestTopologicalSort              — Kahn's algorithm, cycle detection
  TestHasCycle                     — 3-color DFS including cross-edge cases
  TestTransitiveClosure            — forward BFS reachability
  TestTransitiveDependents         — reverse BFS reachability
  TestIndependentGroups            — parallel execution levels
  TestAffectedNodes                — union of transitive dependents
  TestStronglyConnectedComponents  — Kosaraju's two-pass DFS
  TestLabeledDirectedGraph         — composition class with edge labels
  TestCompatibilityWithGraphAlgorithms — bfs/dfs from graph package work

Coverage target: 95%+
"""

from __future__ import annotations

import pytest

from directed_graph import (
    DirectedGraph,
    LabeledDirectedGraph,
    affected_nodes,
    has_cycle,
    independent_groups,
    strongly_connected_components,
    topological_sort,
    transitive_closure,
    transitive_dependents,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_chain(n: int) -> DirectedGraph[int]:
    """Return a linear chain 0 → 1 → 2 → ... → n-1."""
    g: DirectedGraph[int] = DirectedGraph()
    for i in range(n - 1):
        g.add_edge(i, i + 1)
    return g


def make_diamond() -> DirectedGraph[str]:
    """Return a diamond graph:
         A
        / \\
       B   C
        \\ /
         D
    """
    g: DirectedGraph[str] = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("A", "C")
    g.add_edge("B", "D")
    g.add_edge("C", "D")
    return g


# ---------------------------------------------------------------------------
# TestDirectedGraphBasic
# ---------------------------------------------------------------------------


class TestDirectedGraphBasic:
    """Core node and edge operations on DirectedGraph."""

    def test_empty_graph(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        assert len(g) == 0
        assert g.nodes() == frozenset()

    def test_add_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert g.has_node("A")
        assert "A" in g
        assert len(g) == 1

    def test_add_node_idempotent(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        g.add_node("A")  # no-op
        assert len(g) == 1

    def test_add_node_initializes_reverse(self) -> None:
        """add_node must initialise _reverse[node] = {}."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("X")
        assert "X" in g._reverse
        assert g._reverse["X"] == {}

    def test_add_edge_creates_nodes(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        assert g.has_node("A")
        assert g.has_node("B")
        assert g.has_edge("A", "B")

    def test_add_edge_directed(self) -> None:
        """Edge A→B does NOT imply B→A."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        assert g.has_edge("A", "B")
        assert not g.has_edge("B", "A")

    def test_add_edge_updates_adj(self) -> None:
        """Forward edge is stored in _adj."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B", 3.0)
        assert g._adj["A"]["B"] == 3.0
        # Reverse direction NOT stored as forward edge.
        assert "A" not in g._adj.get("B", {})

    def test_add_edge_updates_reverse(self) -> None:
        """Reverse edge is stored in _reverse."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B", 2.5)
        assert g._reverse["B"]["A"] == 2.5

    def test_add_edge_weight_update(self) -> None:
        """Re-adding an edge updates its weight."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B", 1.0)
        g.add_edge("A", "B", 5.0)
        assert g._adj["A"]["B"] == 5.0
        assert g._reverse["B"]["A"] == 5.0

    def test_remove_edge(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        g.remove_edge("A", "B")
        assert not g.has_edge("A", "B")
        # Reverse direction unaffected.
        assert g.has_edge("B", "A")

    def test_remove_edge_updates_reverse(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        assert "A" not in g._reverse.get("B", {})

    def test_remove_edge_missing_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(KeyError):
            g.remove_edge("A", "B")

    def test_remove_node_removes_edges(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("C", "B")
        g.add_edge("B", "D")
        g.remove_node("B")
        assert not g.has_node("B")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("C", "B")
        assert not g.has_edge("B", "D")
        # A, C, D still exist.
        assert g.has_node("A")
        assert g.has_node("C")
        assert g.has_node("D")

    def test_remove_node_cleans_reverse(self) -> None:
        """_reverse must be clean after remove_node."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.remove_node("B")
        assert "B" not in g._reverse
        assert "B" not in g._adj.get("A", {})

    def test_remove_node_missing_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            g.remove_node("X")

    def test_edges_directed(self) -> None:
        """edges() includes both A→B and B→A when both are added."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B", 1.0)
        g.add_edge("B", "A", 2.0)
        edges = g.edges()
        assert ("A", "B", 1.0) in edges
        assert ("B", "A", 2.0) in edges
        assert len(edges) == 2

    def test_has_edge_false_for_missing_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        assert not g.has_edge("X", "Y")

    def test_self_loop_disallowed_by_default(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(ValueError):
            g.add_edge("A", "A")

    def test_self_loop_allowed(self) -> None:
        g: DirectedGraph[str] = DirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A")
        assert g.has_edge("A", "A")
        assert g._adj["A"]["A"] == 1.0
        assert g._reverse["A"]["A"] == 1.0

    def test_contains(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert "A" in g
        assert "B" not in g

    def test_repr(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        r = repr(g)
        assert "DirectedGraph" in r
        assert "nodes=2" in r
        assert "edges=1" in r

    def test_repr_with_self_loops(self) -> None:
        g: DirectedGraph[str] = DirectedGraph(allow_self_loops=True)
        r = repr(g)
        assert "allow_self_loops=True" in r

    def test_integer_nodes(self) -> None:
        g: DirectedGraph[int] = DirectedGraph()
        g.add_edge(1, 2)
        g.add_edge(2, 3)
        assert g.has_edge(1, 2)
        assert g.has_edge(2, 3)
        assert not g.has_edge(1, 3)

    def test_nodes_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        g.add_node("B")
        result = g.nodes()
        assert isinstance(result, frozenset)
        assert result == frozenset({"A", "B"})

    def test_isolated_node_in_nodes(self) -> None:
        """Isolated nodes (no edges) must appear in nodes()."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("isolated")
        g.add_edge("A", "B")
        assert "isolated" in g.nodes()
        assert len(g) == 3


class TestDirectedGraphProperties:
    """DT00 property bags with directed edge identity."""

    def test_graph_node_and_edge_property_bags(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()

        g.set_graph_property("name", "neural-dag")
        g.set_graph_property("version", 1)
        assert g.graph_properties() == {"name": "neural-dag", "version": 1}
        g.remove_graph_property("version")
        assert g.graph_properties() == {"name": "neural-dag"}

        g.add_node("input", {"kind": "input"})
        g.add_node("input", {"slot": 0})
        assert g.node_properties("input") == {"kind": "input", "slot": 0}

        node_properties = g.node_properties("input")
        node_properties["kind"] = "mutated"
        assert g.node_properties("input")["kind"] == "input"

        g.add_edge("input", "sum", 0.5, {"trainable": True})
        assert g.edge_properties("input", "sum") == {
            "trainable": True,
            "weight": 0.5,
        }

        g.set_edge_property("input", "sum", "weight", 0.75)
        assert g.edge_weight("input", "sum") == 0.75
        assert g._reverse["sum"]["input"] == 0.75

        g.remove_edge_property("input", "sum", "trainable")
        assert g.edge_properties("input", "sum") == {"weight": 0.75}

    def test_reverse_edges_have_independent_properties(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B", 2.0, {"role": "forward"})
        g.add_edge("B", "A", 3.0, {"role": "reverse"})

        assert g.edge_properties("A", "B") == {"role": "forward", "weight": 2.0}
        assert g.edge_properties("B", "A") == {"role": "reverse", "weight": 3.0}

        g.set_edge_property("B", "A", "weight", 4.0)
        assert g.edge_weight("A", "B") == 2.0
        assert g.edge_weight("B", "A") == 4.0

    def test_removing_structure_removes_properties(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A", {"kind": "input"})
        g.add_edge("A", "B", 1.5, {"trainable": True})

        g.remove_edge("A", "B")
        with pytest.raises(KeyError):
            g.edge_properties("A", "B")

        g.add_edge("A", "B", 1.5, {"trainable": True})
        g.remove_node("A")
        with pytest.raises(KeyError):
            g.node_properties("A")
        with pytest.raises(KeyError):
            g.edge_properties("A", "B")


# ---------------------------------------------------------------------------
# TestSuccessorsPredecessors
# ---------------------------------------------------------------------------


class TestSuccessorsPredecessors:
    """Directional neighbor queries and degree methods."""

    def test_successors(self) -> None:
        g = make_diamond()
        assert g.successors("A") == frozenset({"B", "C"})
        assert g.successors("B") == frozenset({"D"})
        assert g.successors("D") == frozenset()

    def test_predecessors(self) -> None:
        g = make_diamond()
        assert g.predecessors("D") == frozenset({"B", "C"})
        assert g.predecessors("B") == frozenset({"A"})
        assert g.predecessors("A") == frozenset()

    def test_successors_missing_node_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            g.successors("X")

    def test_predecessors_missing_node_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            g.predecessors("X")

    def test_out_degree(self) -> None:
        g = make_diamond()
        assert g.out_degree("A") == 2
        assert g.out_degree("B") == 1
        assert g.out_degree("D") == 0

    def test_in_degree(self) -> None:
        g = make_diamond()
        assert g.in_degree("A") == 0
        assert g.in_degree("B") == 1
        assert g.in_degree("D") == 2

    def test_out_degree_missing_node_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            g.out_degree("X")

    def test_in_degree_missing_node_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            g.in_degree("X")

    def test_successors_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        result = g.successors("A")
        assert isinstance(result, frozenset)

    def test_predecessors_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        result = g.predecessors("B")
        assert isinstance(result, frozenset)

    def test_bidirectional_edges_are_independent(self) -> None:
        """A→B and B→A are two separate directed edges."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert g.successors("A") == frozenset({"B"})
        assert g.successors("B") == frozenset({"A"})
        assert g.predecessors("A") == frozenset({"B"})
        assert g.predecessors("B") == frozenset({"A"})


# ---------------------------------------------------------------------------
# TestNeighborsOverride
# ---------------------------------------------------------------------------


class TestNeighborsOverride:
    """neighbors() must return successors only (forward edges)."""

    def test_neighbors_returns_successors_only(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("C", "A")   # A has an incoming edge from C
        # neighbors(A) should return B (outgoing), NOT C (incoming).
        assert g.neighbors("A") == frozenset({"B"})

    def test_neighbors_empty_for_sink(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        assert g.neighbors("B") == frozenset()

    def test_neighbors_missing_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            g.neighbors("X")

    def test_neighbors_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert isinstance(g.neighbors("A"), frozenset)


# ---------------------------------------------------------------------------
# TestTopologicalSort
# ---------------------------------------------------------------------------


class TestTopologicalSort:
    """Kahn's topological sort algorithm."""

    def test_empty_graph(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        assert topological_sort(g) == []

    def test_single_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert topological_sort(g) == ["A"]

    def test_linear_chain(self) -> None:
        g = make_chain(4)
        result = topological_sort(g)
        assert result == [0, 1, 2, 3]

    def test_diamond(self) -> None:
        g = make_diamond()
        result = topological_sort(g)
        # A must come first, D must come last.
        assert result[0] == "A"
        assert result[-1] == "D"
        # B and C must both come before D.
        assert result.index("B") < result.index("D")
        assert result.index("C") < result.index("D")

    def test_multiple_roots(self) -> None:
        """Nodes with no predecessors can appear in any order."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("B", "C")
        g.add_edge("A", "C")
        result = topological_sort(g)
        assert result[-1] == "C"
        assert "A" in result
        assert "B" in result

    def test_cycle_raises_value_error(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        with pytest.raises(ValueError, match="cycle"):
            topological_sort(g)

    def test_self_loop_raises_value_error(self) -> None:
        g: DirectedGraph[str] = DirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A")
        with pytest.raises(ValueError):
            topological_sort(g)

    def test_result_respects_edge_order(self) -> None:
        """Every u→v edge must have u before v in the result."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        g.add_edge("C", "D")
        result = topological_sort(g)
        idx = {n: i for i, n in enumerate(result)}
        assert idx["A"] < idx["C"]
        assert idx["B"] < idx["C"]
        assert idx["C"] < idx["D"]

    def test_disconnected_graph(self) -> None:
        """Disconnected components are all included."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_node("C")  # isolated
        g.add_edge("D", "E")
        result = topological_sort(g)
        assert set(result) == {"A", "B", "C", "D", "E"}
        assert result.index("A") < result.index("B")
        assert result.index("D") < result.index("E")

    def test_large_chain_no_recursion_error(self) -> None:
        """Chain of 2000 nodes should not hit recursion limit."""
        g = make_chain(2000)
        result = topological_sort(g)
        assert result == list(range(2000))


# ---------------------------------------------------------------------------
# TestHasCycle
# ---------------------------------------------------------------------------


class TestHasCycle:
    """3-color iterative DFS cycle detection."""

    def test_empty_graph_no_cycle(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        assert not has_cycle(g)

    def test_single_node_no_cycle(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert not has_cycle(g)

    def test_linear_chain_no_cycle(self) -> None:
        assert not has_cycle(make_chain(5))

    def test_diamond_no_cycle(self) -> None:
        assert not has_cycle(make_diamond())

    def test_simple_cycle(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert has_cycle(g)

    def test_three_node_cycle(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert has_cycle(g)

    def test_self_loop_is_cycle(self) -> None:
        g: DirectedGraph[str] = DirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A")
        assert has_cycle(g)

    def test_cross_edge_not_a_cycle(self) -> None:
        """A→B→C and D→C should NOT be a cycle.

        When DFS reaches C from A→B→C, C is colored BLACK.
        When D later finds C (already BLACK), that's a cross-edge — no cycle.
        """
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("D", "C")
        assert not has_cycle(g)

    def test_forward_edge_not_a_cycle(self) -> None:
        """A→B, A→C, B→C: A→C is a forward edge, not a back edge."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("A", "C")
        assert not has_cycle(g)

    def test_cycle_in_second_component(self) -> None:
        """Cycle in a disconnected component must still be detected."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("X", "Y")   # DAG component
        g.add_edge("A", "B")   # cycle component
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert has_cycle(g)

    def test_large_dag_no_recursion_error(self) -> None:
        """Chain of 2000 nodes should not hit recursion limit."""
        g = make_chain(2000)
        assert not has_cycle(g)


# ---------------------------------------------------------------------------
# TestTransitiveClosure
# ---------------------------------------------------------------------------


class TestTransitiveClosure:
    """BFS forward reachability."""

    def test_single_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert transitive_closure(g, "A") == frozenset()

    def test_linear_chain(self) -> None:
        g = make_chain(4)
        assert transitive_closure(g, 0) == frozenset({1, 2, 3})
        assert transitive_closure(g, 1) == frozenset({2, 3})
        assert transitive_closure(g, 3) == frozenset()

    def test_diamond(self) -> None:
        g = make_diamond()
        assert transitive_closure(g, "A") == frozenset({"B", "C", "D"})
        assert transitive_closure(g, "B") == frozenset({"D"})

    def test_does_not_include_start(self) -> None:
        """Starting node must NOT be in the closure."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        result = transitive_closure(g, "A")
        assert "A" not in result

    def test_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert isinstance(transitive_closure(g, "A"), frozenset)

    def test_missing_node_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            transitive_closure(g, "X")

    def test_cycle_graph_reachability(self) -> None:
        """In a cycle A→B→C→A, every node can reach every other node."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert transitive_closure(g, "A") == frozenset({"B", "C"})

    def test_disconnected_graph(self) -> None:
        """Cannot reach nodes in a disconnected component."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_node("C")  # isolated
        assert "C" not in transitive_closure(g, "A")


# ---------------------------------------------------------------------------
# TestTransitiveDependents
# ---------------------------------------------------------------------------


class TestTransitiveDependents:
    """BFS reverse reachability (who depends on this node?)."""

    def test_single_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert transitive_dependents(g, "A") == frozenset()

    def test_linear_chain(self) -> None:
        """In 0→1→2→3 (A→B means A depends on B), dependents of 3 = {0,1,2}.

        Edge direction convention: A→B means "A depends on B".
        transitive_dependents(node) asks "who depends (directly or transitively)
        on this node?" — answered by following reverse edges (predecessors).

        In the chain: 3 is the root dependency; 0, 1, 2 all transitively depend
        on 3. Node 0 has no predecessors, so nothing depends on it.
        """
        g = make_chain(4)
        assert transitive_dependents(g, 3) == frozenset({0, 1, 2})
        assert transitive_dependents(g, 1) == frozenset({0})
        assert transitive_dependents(g, 0) == frozenset()

    def test_diamond(self) -> None:
        """In diamond A→B, A→C, B→D, C→D — dependents of A = {B, C, D}."""
        g = make_diamond()
        # Note: for transitive_dependents, "A depends on" = predecessors of A.
        # In the diamond, A has no predecessors, so its dependents in the
        # "who depends on A?" sense = B, C, D (they all come after A).
        # transitive_dependents follows _reverse: predecessors of B are A,
        # predecessors of A are none.
        # So dependents of D = {B, C, A}.
        assert transitive_dependents(g, "D") == frozenset({"A", "B", "C"})
        assert transitive_dependents(g, "B") == frozenset({"A"})
        assert transitive_dependents(g, "A") == frozenset()

    def test_does_not_include_start(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        assert "B" not in transitive_dependents(g, "B")

    def test_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert isinstance(transitive_dependents(g, "A"), frozenset)

    def test_missing_node_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        with pytest.raises(KeyError):
            transitive_dependents(g, "X")

    def test_build_system_scenario(self) -> None:
        """Build system: A→B means A depends on B.

        If B changes, A must be rebuilt. If "parse" changes, "compile" and
        "link" and "package" must all be rebuilt.
        """
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("compile", "parse")
        g.add_edge("link", "compile")
        g.add_edge("package", "link")
        result = transitive_dependents(g, "parse")
        assert result == frozenset({"compile", "link", "package"})


# ---------------------------------------------------------------------------
# TestIndependentGroups
# ---------------------------------------------------------------------------


class TestIndependentGroups:
    """Parallel execution levels via modified Kahn's algorithm."""

    def test_empty_graph(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        assert independent_groups(g) == []

    def test_single_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert independent_groups(g) == [["A"]]

    def test_linear_chain(self) -> None:
        """Linear chain: every node is its own level."""
        g = make_chain(4)
        groups = independent_groups(g)
        assert len(groups) == 4
        assert groups == [[0], [1], [2], [3]]

    def test_diamond(self) -> None:
        """Diamond: A alone, then B+C in parallel, then D alone."""
        g = make_diamond()
        groups = independent_groups(g)
        assert groups[0] == ["A"]
        # B and C can run in parallel (order within group may vary).
        assert set(groups[1]) == {"B", "C"}
        assert groups[2] == ["D"]

    def test_all_independent(self) -> None:
        """No edges: all nodes in a single group."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        groups = independent_groups(g)
        assert len(groups) == 1
        assert set(groups[0]) == {"A", "B", "C"}

    def test_cycle_raises(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        with pytest.raises(ValueError, match="cycle"):
            independent_groups(g)

    def test_groups_cover_all_nodes(self) -> None:
        """Every node must appear in exactly one group."""
        g = make_diamond()
        groups = independent_groups(g)
        all_nodes = [n for group in groups for n in group]
        assert set(all_nodes) == {"A", "B", "C", "D"}
        assert len(all_nodes) == len(set(all_nodes))  # no duplicates

    def test_respects_ordering(self) -> None:
        """For every edge u→v, u's group index must be < v's group index."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        g.add_edge("C", "D")
        groups = independent_groups(g)
        # Build group index lookup.
        group_idx = {n: i for i, group in enumerate(groups) for n in group}
        for u, v, _ in g.edges():
            assert group_idx[u] < group_idx[v], (
                f"{u} (level {group_idx[u]}) should be before {v} (level {group_idx[v]})"
            )


# ---------------------------------------------------------------------------
# TestAffectedNodes
# ---------------------------------------------------------------------------


class TestAffectedNodes:
    """Union of transitive dependents for a set of changed nodes."""

    def test_empty_changed(self) -> None:
        g = make_diamond()
        assert affected_nodes(g, frozenset()) == frozenset()

    def test_single_changed_root(self) -> None:
        g = make_diamond()
        result = affected_nodes(g, frozenset({"A"}))
        # A→B, A→C, B→D, C→D means A depends on B and C; they depend on D.
        # "Who depends on A?" = nothing (A is a leaf in the dependency graph,
        # no other node has A as a dependency). So affected = {A} only.
        assert result == frozenset({"A"})

    def test_single_changed_leaf(self) -> None:
        g = make_diamond()
        # D has predecessors B and C; B has predecessor A; C has predecessor A.
        result = affected_nodes(g, frozenset({"D"}))
        assert result == frozenset({"D", "B", "C", "A"})

    def test_unknown_nodes_ignored(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        # "X" is not in the graph — silently ignored.
        result = affected_nodes(g, frozenset({"X", "A"}))
        assert result == frozenset({"A"})

    def test_changed_includes_the_changed_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        result = affected_nodes(g, frozenset({"B"}))
        # B is included even though it has no predecessors.
        assert "B" in result

    def test_build_system(self) -> None:
        """Rebuild only what's affected after a file change."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("compile", "parse")
        g.add_edge("link", "compile")
        g.add_edge("package", "link")
        g.add_edge("test", "compile")

        result = affected_nodes(g, frozenset({"compile"}))
        # compile changed → link, package, test must rebuild
        assert result == frozenset({"compile", "link", "package", "test"})

    def test_multiple_changed_nodes(self) -> None:
        # A→C, B→D, C→E, D→E: A depends on C; B depends on D; C and D depend on E.
        # A and B are leaf nodes (nothing depends on them). So changing A or B
        # affects only A and B themselves (no predecessors).
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "E")
        g.add_edge("D", "E")

        result = affected_nodes(g, frozenset({"A", "B"}))
        assert result == frozenset({"A", "B"})

    def test_returns_frozenset(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        assert isinstance(affected_nodes(g, frozenset({"A"})), frozenset)


# ---------------------------------------------------------------------------
# TestStronglyConnectedComponents
# ---------------------------------------------------------------------------


class TestStronglyConnectedComponents:
    """Kosaraju's two-pass iterative DFS."""

    def test_empty_graph(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        assert strongly_connected_components(g) == []

    def test_single_node(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        sccs = strongly_connected_components(g)
        assert len(sccs) == 1
        assert sccs[0] == frozenset({"A"})

    def test_dag_all_singletons(self) -> None:
        """A DAG has no non-trivial SCCs — every node is its own SCC."""
        g = make_chain(4)
        sccs = strongly_connected_components(g)
        assert len(sccs) == 4
        # Each SCC has exactly one node.
        for scc in sccs:
            assert len(scc) == 1

    def test_all_one_scc(self) -> None:
        """Fully cyclic graph: all nodes form one SCC."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        sccs = strongly_connected_components(g)
        assert len(sccs) == 1
        assert sccs[0] == frozenset({"A", "B", "C"})

    def test_mixed_sccs(self) -> None:
        """A→B→C→A forms one SCC; C→D→E→D forms another (D,E)."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")   # cycle: {A,B,C}
        g.add_edge("C", "D")
        g.add_edge("D", "E")
        g.add_edge("E", "D")   # cycle: {D,E}
        sccs = strongly_connected_components(g)
        scc_sets = {frozenset(s) for s in sccs}
        assert frozenset({"A", "B", "C"}) in scc_sets
        assert frozenset({"D", "E"}) in scc_sets

    def test_two_separate_cycles(self) -> None:
        """Two disconnected cycles each form their own SCC."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "A")   # SCC 1: {A, B}
        g.add_edge("C", "D")
        g.add_edge("D", "C")   # SCC 2: {C, D}
        sccs = strongly_connected_components(g)
        scc_sets = {frozenset(s) for s in sccs}
        assert frozenset({"A", "B"}) in scc_sets
        assert frozenset({"C", "D"}) in scc_sets

    def test_covers_all_nodes(self) -> None:
        """Every node must appear in exactly one SCC."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        g.add_edge("C", "D")
        g.add_edge("D", "E")
        sccs = strongly_connected_components(g)
        all_nodes_in_sccs = set()
        for scc in sccs:
            # No node should appear in more than one SCC.
            assert scc.isdisjoint(all_nodes_in_sccs)
            all_nodes_in_sccs.update(scc)
        assert all_nodes_in_sccs == {"A", "B", "C", "D", "E"}

    def test_self_loop_scc(self) -> None:
        """A self-loop makes a node its own (non-trivial) SCC."""
        g: DirectedGraph[str] = DirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A")
        sccs = strongly_connected_components(g)
        assert any("A" in scc for scc in sccs)

    def test_large_dag_no_recursion_error(self) -> None:
        """Chain of 1000 nodes — all singletons, no stack overflow."""
        g = make_chain(1000)
        sccs = strongly_connected_components(g)
        assert len(sccs) == 1000

    def test_returns_list_of_frozensets(self) -> None:
        g: DirectedGraph[str] = DirectedGraph()
        g.add_node("A")
        sccs = strongly_connected_components(g)
        assert isinstance(sccs, list)
        assert all(isinstance(s, frozenset) for s in sccs)


# ---------------------------------------------------------------------------
# TestLabeledDirectedGraph
# ---------------------------------------------------------------------------


class TestLabeledDirectedGraph:
    """Composition-based directed graph with string edge labels."""

    def test_add_and_query_label(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="imports")
        assert g.edge_label("A", "B") == "imports"

    def test_has_edge(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="uses")
        assert g.has_edge("A", "B")
        assert not g.has_edge("B", "A")

    def test_add_node(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_node("X")
        assert g.has_node("X")

    def test_nodes(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_node("A")
        g.add_node("B")
        result = g.nodes()
        assert isinstance(result, frozenset)
        assert result == frozenset({"A", "B"})

    def test_remove_edge(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="x")
        g.remove_edge("A", "B")
        assert not g.has_edge("A", "B")
        with pytest.raises(KeyError):
            g.edge_label("A", "B")

    def test_remove_edge_missing_raises(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        with pytest.raises(KeyError):
            g.remove_edge("A", "B")

    def test_remove_node(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="uses")
        g.add_edge("C", "B", label="extends")
        g.remove_node("B")
        assert not g.has_node("B")
        assert not g.has_edge("A", "B")
        assert not g.has_edge("C", "B")

    def test_remove_node_missing_raises(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        with pytest.raises(KeyError):
            g.remove_node("X")

    def test_edge_label_missing_raises(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_node("A")
        g.add_node("B")
        with pytest.raises(KeyError):
            g.edge_label("A", "B")

    def test_edges_labeled(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="imports", weight=1.5)
        g.add_edge("B", "C", label="extends", weight=2.0)
        result = g.edges_labeled()
        assert isinstance(result, frozenset)
        assert ("A", "B", "imports", 1.5) in result
        assert ("B", "C", "extends", 2.0) in result

    def test_successors(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="x")
        g.add_edge("A", "C", label="y")
        assert g.successors("A") == frozenset({"B", "C"})

    def test_predecessors(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "C", label="x")
        g.add_edge("B", "C", label="y")
        assert g.predecessors("C") == frozenset({"A", "B"})

    def test_update_label(self) -> None:
        """Re-adding an edge updates both label and weight."""
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="old", weight=1.0)
        g.add_edge("A", "B", label="new", weight=3.0)
        assert g.edge_label("A", "B") == "new"
        result = g.edges_labeled()
        assert ("A", "B", "new", 3.0) in result

    def test_self_loop_disallowed_by_default(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        with pytest.raises(ValueError):
            g.add_edge("A", "A", label="self")

    def test_self_loop_allowed(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A", label="self")
        assert g.edge_label("A", "A") == "self"

    def test_repr(self) -> None:
        g: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        g.add_edge("A", "B", label="x")
        r = repr(g)
        assert "LabeledDirectedGraph" in r
        assert "nodes=2" in r
        assert "edges=1" in r

    def test_state_machine(self) -> None:
        """A state machine modeled as a LabeledDirectedGraph."""
        sm: LabeledDirectedGraph[str] = LabeledDirectedGraph()
        sm.add_edge("idle", "running", label="start")
        sm.add_edge("running", "idle", label="stop")
        sm.add_edge("running", "paused", label="pause")
        sm.add_edge("paused", "running", label="resume")

        assert sm.edge_label("running", "paused") == "pause"
        assert sm.edge_label("paused", "running") == "resume"
        assert sm.successors("running") == frozenset({"idle", "paused"})
        assert sm.predecessors("idle") == frozenset({"running"})


# ---------------------------------------------------------------------------
# TestCompatibilityWithGraphAlgorithms
# ---------------------------------------------------------------------------


class TestCompatibilityWithGraphAlgorithms:
    """bfs and dfs from the graph package must work correctly on DirectedGraph.

    DirectedGraph.neighbors() returns successors only, so these algorithms
    naturally traverse only forward edges without any modification.
    """

    def test_bfs_follows_forward_edges(self) -> None:
        from graph import bfs

        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "D")
        g.add_edge("X", "A")  # incoming edge to A from X

        # BFS from A should follow A→B→C→D only; should NOT visit X
        result = bfs(g, "A")
        assert result == ["A", "B", "C", "D"]
        assert "X" not in result

    def test_dfs_follows_forward_edges(self) -> None:
        from graph import dfs

        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("Y", "A")  # incoming to A

        result = dfs(g, "A")
        assert "A" in result
        assert "B" in result
        assert "C" in result
        assert "Y" not in result

    def test_bfs_diamond(self) -> None:
        from graph import bfs

        g = make_diamond()
        result = bfs(g, "A")
        # A first, then B and C (in some order), then D.
        assert result[0] == "A"
        assert result[-1] == "D"
        assert set(result) == {"A", "B", "C", "D"}

    def test_dfs_linear_chain(self) -> None:
        from graph import dfs

        g = make_chain(5)
        result = dfs(g, 0)
        assert result == [0, 1, 2, 3, 4]

    def test_directed_graph_is_graph_subclass(self) -> None:
        """DirectedGraph must be a subclass of Graph."""
        from graph import Graph

        g: DirectedGraph[str] = DirectedGraph()
        assert isinstance(g, Graph)

    def test_len_and_contains(self) -> None:
        """len() and 'in' operator inherited from Graph."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B")
        assert len(g) == 2
        assert "A" in g
        assert "C" not in g

    def test_neighbors_weighted_not_symmetric(self) -> None:
        """neighbors_weighted should reflect directed edges only."""
        g: DirectedGraph[str] = DirectedGraph()
        g.add_edge("A", "B", 3.0)
        # neighbors_weighted is inherited from Graph — works on _adj.
        weighted = g.neighbors_weighted("A")
        assert "B" in weighted
        assert weighted["B"] == 3.0
        # B has no outgoing edges.
        assert g.neighbors_weighted("B") == {}

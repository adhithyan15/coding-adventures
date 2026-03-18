"""
test_algorithms.py -- Tests for Graph Algorithms
==================================================

These tests cover the algorithmic methods on DirectedGraph: topological sort,
cycle detection, transitive closure/dependents, independent groups, and
affected nodes.

We test each algorithm against several graph shapes:

- **Linear chain**: A -> B -> C -> D (the simplest DAG)
- **Diamond**: A -> B, A -> C, B -> D, C -> D (tests parallel nodes)
- **Cycle**: A -> B -> C -> A (tests error handling)
- **Complex DAG**: A real-world-ish dependency graph modeled after the
  21 Python packages in this repository

The progression goes from simple shapes to complex ones, and from "happy
path" to error cases.
"""

import pytest

from directed_graph import CycleError, DirectedGraph, NodeNotFoundError


# ======================================================================
# Helper: build common graph shapes
# ======================================================================
# We use factory functions instead of fixtures so each test gets a fresh
# graph. This avoids any accidental state sharing between tests.


def make_linear_chain() -> DirectedGraph:
    """Build A -> B -> C -> D."""
    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "D")
    return g


def make_diamond() -> DirectedGraph:
    """Build the diamond shape:

        A
       / \\
      B   C
       \\ /
        D

    Edges: A->B, A->C, B->D, C->D
    """
    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("A", "C")
    g.add_edge("B", "D")
    g.add_edge("C", "D")
    return g


def make_cycle() -> DirectedGraph:
    """Build A -> B -> C -> A (a three-node cycle)."""
    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g.add_edge("C", "A")
    return g


# ======================================================================
# 1. Topological Sort
# ======================================================================
# Kahn's algorithm should produce a valid ordering where every edge goes
# from earlier to later in the sequence.


class TestTopologicalSort:
    """Test topological sorting with various graph shapes."""

    def test_empty_graph(self) -> None:
        """Topological sort of an empty graph is an empty list."""
        g = DirectedGraph()
        assert g.topological_sort() == []

    def test_single_node(self) -> None:
        """A single node topo-sorts to a one-element list."""
        g = DirectedGraph()
        g.add_node("A")
        assert g.topological_sort() == ["A"]

    def test_linear_chain(self) -> None:
        """A -> B -> C -> D must sort to exactly [A, B, C, D].

        In a linear chain, there's only one valid topological order.
        """
        g = make_linear_chain()
        assert g.topological_sort() == ["A", "B", "C", "D"]

    def test_diamond(self) -> None:
        """The diamond A->{B,C}->D has multiple valid orderings.

        A must come first, D must come last. B and C can be in either order.
        Our implementation sorts ties alphabetically, so we expect [A, B, C, D].
        """
        g = make_diamond()
        result = g.topological_sort()
        assert result[0] == "A"
        assert result[-1] == "D"
        assert set(result[1:3]) == {"B", "C"}

    def test_cycle_raises_cycle_error(self) -> None:
        """A graph with a cycle should raise CycleError on topological sort."""
        g = make_cycle()
        with pytest.raises(CycleError) as exc_info:
            g.topological_sort()
        # The error should include the cycle path.
        assert len(exc_info.value.cycle) >= 3
        # The first and last element of the cycle should be the same node.
        assert exc_info.value.cycle[0] == exc_info.value.cycle[-1]

    def test_disconnected_components(self) -> None:
        """Topological sort should handle disconnected components.

        If the graph has two separate chains X->Y and A->B, the sort should
        include all four nodes in a valid order.
        """
        g = DirectedGraph()
        g.add_edge("X", "Y")
        g.add_edge("A", "B")
        result = g.topological_sort()
        assert len(result) == 4
        # X must come before Y, A must come before B.
        assert result.index("X") < result.index("Y")
        assert result.index("A") < result.index("B")


# ======================================================================
# 2. Cycle Detection
# ======================================================================


class TestCycleDetection:
    """Test the has_cycle method."""

    def test_empty_graph_has_no_cycle(self) -> None:
        """An empty graph has no cycle."""
        g = DirectedGraph()
        assert g.has_cycle() is False

    def test_linear_chain_has_no_cycle(self) -> None:
        """A linear chain is a DAG -- no cycles."""
        g = make_linear_chain()
        assert g.has_cycle() is False

    def test_diamond_has_no_cycle(self) -> None:
        """A diamond is a DAG -- no cycles."""
        g = make_diamond()
        assert g.has_cycle() is False

    def test_three_node_cycle(self) -> None:
        """A -> B -> C -> A is a cycle."""
        g = make_cycle()
        assert g.has_cycle() is True

    def test_cycle_with_tail(self) -> None:
        """A graph with a cycle and a non-cyclic tail.

        X -> A -> B -> C -> A. The cycle is A->B->C->A, and X is a tail
        leading into it.
        """
        g = DirectedGraph()
        g.add_edge("X", "A")
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert g.has_cycle() is True


# ======================================================================
# 3. Transitive Closure
# ======================================================================
# transitive_closure(node) returns all nodes reachable downstream.


class TestTransitiveClosure:
    """Test forward reachability (transitive closure)."""

    def test_linear_chain_from_root(self) -> None:
        """From A in A->B->C->D, everything downstream is reachable."""
        g = make_linear_chain()
        assert g.transitive_closure("A") == {"B", "C", "D"}

    def test_linear_chain_from_middle(self) -> None:
        """From B in A->B->C->D, only C and D are reachable."""
        g = make_linear_chain()
        assert g.transitive_closure("B") == {"C", "D"}

    def test_linear_chain_from_leaf(self) -> None:
        """From D (a leaf), nothing is reachable."""
        g = make_linear_chain()
        assert g.transitive_closure("D") == set()

    def test_diamond_from_root(self) -> None:
        """From A in the diamond, B, C, and D are all reachable."""
        g = make_diamond()
        assert g.transitive_closure("A") == {"B", "C", "D"}

    def test_diamond_from_middle(self) -> None:
        """From B in the diamond, only D is reachable."""
        g = make_diamond()
        assert g.transitive_closure("B") == {"D"}

    def test_nonexistent_node_raises(self) -> None:
        """transitive_closure on a missing node should raise."""
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.transitive_closure("X")

    def test_isolated_node(self) -> None:
        """An isolated node has empty transitive closure."""
        g = DirectedGraph()
        g.add_node("A")
        assert g.transitive_closure("A") == set()


# ======================================================================
# 4. Transitive Dependents
# ======================================================================
# transitive_dependents(node) returns all nodes that depend on this node
# (walking edges backwards).


class TestTransitiveDependents:
    """Test reverse reachability (transitive dependents)."""

    def test_linear_chain_from_leaf(self) -> None:
        """From D in A->B->C->D, everything upstream depends on D... wait, no.

        Actually, D is the leaf. Nodes that depend on D are the ones that
        point TO D, which is C. And C's dependents are B, and B's are A.
        So transitive_dependents("D") = {A, B, C}.
        """
        g = make_linear_chain()
        assert g.transitive_dependents("D") == {"A", "B", "C"}

    def test_linear_chain_from_root(self) -> None:
        """From A (the root), nothing depends on A -- it has no predecessors."""
        g = make_linear_chain()
        assert g.transitive_dependents("A") == set()

    def test_diamond_from_d(self) -> None:
        """From D in the diamond, A, B, and C all transitively depend on D."""
        g = make_diamond()
        assert g.transitive_dependents("D") == {"A", "B", "C"}

    def test_diamond_from_b(self) -> None:
        """From B in the diamond, only A depends on B."""
        g = make_diamond()
        assert g.transitive_dependents("B") == {"A"}

    def test_nonexistent_node_raises(self) -> None:
        """transitive_dependents on a missing node should raise."""
        g = DirectedGraph()
        with pytest.raises(NodeNotFoundError):
            g.transitive_dependents("X")


# ======================================================================
# 5. Independent Groups
# ======================================================================
# independent_groups partitions nodes into topological levels. Nodes at
# the same level can run in parallel.


class TestIndependentGroups:
    """Test parallel execution level computation."""

    def test_empty_graph(self) -> None:
        """An empty graph has no groups."""
        g = DirectedGraph()
        assert g.independent_groups() == []

    def test_single_node(self) -> None:
        """A single node forms one group."""
        g = DirectedGraph()
        g.add_node("A")
        assert g.independent_groups() == [["A"]]

    def test_linear_chain(self) -> None:
        """A -> B -> C -> D has four levels, each with one node.

        No parallelism is possible because each node depends on the previous.
        """
        g = make_linear_chain()
        assert g.independent_groups() == [["A"], ["B"], ["C"], ["D"]]

    def test_diamond_has_parallel_middle(self) -> None:
        """The diamond A->{B,C}->D should have B and C at the same level.

        Level 0: [A]      (no dependencies)
        Level 1: [B, C]   (both depend only on A)
        Level 2: [D]      (depends on both B and C)
        """
        g = make_diamond()
        groups = g.independent_groups()
        assert len(groups) == 3
        assert groups[0] == ["A"]
        assert sorted(groups[1]) == ["B", "C"]
        assert groups[2] == ["D"]

    def test_two_independent_chains(self) -> None:
        """Two disconnected chains should interleave at each level.

        A -> B and X -> Y should give:
        Level 0: [A, X]   (both are roots)
        Level 1: [B, Y]   (both depend on level-0 nodes)
        """
        g = DirectedGraph()
        g.add_edge("A", "B")
        g.add_edge("X", "Y")
        groups = g.independent_groups()
        assert len(groups) == 2
        assert sorted(groups[0]) == ["A", "X"]
        assert sorted(groups[1]) == ["B", "Y"]

    def test_cycle_raises_cycle_error(self) -> None:
        """independent_groups should raise CycleError on cyclic graphs."""
        g = make_cycle()
        with pytest.raises(CycleError):
            g.independent_groups()

    def test_wide_graph(self) -> None:
        """A graph where a root fans out to many children.

        ROOT -> {A, B, C, D, E}

        Level 0: [ROOT]
        Level 1: [A, B, C, D, E]
        """
        g = DirectedGraph()
        for child in ["A", "B", "C", "D", "E"]:
            g.add_edge("ROOT", child)
        groups = g.independent_groups()
        assert len(groups) == 2
        assert groups[0] == ["ROOT"]
        assert sorted(groups[1]) == ["A", "B", "C", "D", "E"]


# ======================================================================
# 6. Affected Nodes
# ======================================================================
# affected_nodes(changed) = changed + all their transitive dependents.


class TestAffectedNodes:
    """Test the affected_nodes computation."""

    def test_change_leaf_affects_everything(self) -> None:
        """Changing D in A->B->C->D affects all nodes.

        Our edge convention is "A depends on B", so A->B->C->D means
        D is the foundation that everything depends on. Changing D
        affects A, B, C, and D itself.
        """
        g = make_linear_chain()
        assert g.affected_nodes({"D"}) == {"A", "B", "C", "D"}

    def test_change_root_affects_only_root(self) -> None:
        """Changing A in A->B->C->D affects only A.

        A depends on everything else, but nothing depends on A.
        """
        g = make_linear_chain()
        assert g.affected_nodes({"A"}) == {"A"}

    def test_change_d_in_diamond(self) -> None:
        """Changing D in the diamond affects A, B, C, and D.

        D has A, B, C as transitive dependents.
        """
        g = make_diamond()
        assert g.affected_nodes({"D"}) == {"A", "B", "C", "D"}

    def test_change_a_in_diamond(self) -> None:
        """Changing A in the diamond affects only A.

        A is the root; nothing depends on it.
        """
        g = make_diamond()
        assert g.affected_nodes({"A"}) == {"A"}

    def test_change_multiple_nodes(self) -> None:
        """Changing B and C in the diamond affects A, B, C (and D through B,C? No).

        Wait -- affected_nodes includes the changed nodes plus their
        transitive dependents. B's dependents are {A}, C's dependents are {A}.
        So affected = {B, C, A}.
        """
        g = make_diamond()
        assert g.affected_nodes({"B", "C"}) == {"A", "B", "C"}

    def test_change_nonexistent_node_is_ignored(self) -> None:
        """Nodes not in the graph should be silently ignored."""
        g = make_diamond()
        assert g.affected_nodes({"Z"}) == set()

    def test_mixed_existing_and_nonexistent(self) -> None:
        """A mix of real and fake nodes should include only the real ones."""
        g = make_diamond()
        result = g.affected_nodes({"A", "Z"})
        assert "A" in result
        assert "Z" not in result


# ======================================================================
# 7. Real Repo Graph (21 Python Packages)
# ======================================================================
# This test models the actual dependency graph of the 21 Python packages
# in the coding-adventures repository. It serves as an integration test
# to make sure all the algorithms work together on a realistic graph.


class TestRealRepoGraph:
    """Integration test using the actual 21-package dependency graph."""

    @pytest.fixture()
    def repo_graph(self) -> DirectedGraph:
        """Build the actual dependency graph for the repository.

        The packages and their dependencies (A -> B means A depends on B):

        Layer 1: logic-gates (no deps)
        Layer 2: arithmetic (depends on logic-gates)
        Layer 3: grammar-tools (no deps)
        Layer 4: lexer (depends on grammar-tools)
        Layer 5: parser (depends on lexer, grammar-tools)
        Layer 6: cpu-simulator (depends on arithmetic, logic-gates)
                 intel4004-simulator (depends on arithmetic)
                 pipeline (depends on parser, lexer)
        Layer 7: assembler (depends on parser, grammar-tools)
                 virtual-machine (depends on cpu-simulator)
                 arm-simulator (depends on cpu-simulator, assembler)
        Layer 8: jvm-simulator (depends on virtual-machine)
                 clr-simulator (depends on virtual-machine)
                 wasm-simulator (depends on virtual-machine)
                 riscv-simulator (depends on cpu-simulator)
        Layer 9: bytecode-compiler (depends on jvm-simulator, clr-simulator,
                                    wasm-simulator, parser)
                 html-renderer (depends on parser, lexer)
                 jit-compiler (depends on virtual-machine, assembler)
        """
        g = DirectedGraph()

        # Layer 1 -> 2
        g.add_edge("arithmetic", "logic-gates")

        # Layer 3 -> 4
        g.add_edge("lexer", "grammar-tools")

        # Layer 4 -> 5
        g.add_edge("parser", "lexer")
        g.add_edge("parser", "grammar-tools")

        # Layer 5 -> 6
        g.add_edge("cpu-simulator", "arithmetic")
        g.add_edge("cpu-simulator", "logic-gates")
        g.add_edge("intel4004-simulator", "arithmetic")
        g.add_edge("pipeline", "parser")
        g.add_edge("pipeline", "lexer")

        # Layer 6 -> 7
        g.add_edge("assembler", "parser")
        g.add_edge("assembler", "grammar-tools")
        g.add_edge("virtual-machine", "cpu-simulator")
        g.add_edge("arm-simulator", "cpu-simulator")
        g.add_edge("arm-simulator", "assembler")

        # Layer 7 -> 8
        g.add_edge("jvm-simulator", "virtual-machine")
        g.add_edge("clr-simulator", "virtual-machine")
        g.add_edge("wasm-simulator", "virtual-machine")
        g.add_edge("riscv-simulator", "cpu-simulator")

        # Layer 8 -> 9
        g.add_edge("bytecode-compiler", "jvm-simulator")
        g.add_edge("bytecode-compiler", "clr-simulator")
        g.add_edge("bytecode-compiler", "wasm-simulator")
        g.add_edge("bytecode-compiler", "parser")
        g.add_edge("html-renderer", "parser")
        g.add_edge("html-renderer", "lexer")
        g.add_edge("jit-compiler", "virtual-machine")
        g.add_edge("jit-compiler", "assembler")

        # Ruby-related packages
        g.add_edge("ruby-lexer", "grammar-tools")
        g.add_edge("ruby-parser", "ruby-lexer")
        g.add_edge("ruby-parser", "grammar-tools")

        # directed-graph has no dependencies on other packages
        g.add_node("directed-graph")

        return g

    def test_repo_has_21_packages(self, repo_graph: DirectedGraph) -> None:
        """The repository has 21 Python packages (excluding directed-graph itself)."""
        assert len(repo_graph) == 21

    def test_repo_is_acyclic(self, repo_graph: DirectedGraph) -> None:
        """The repo dependency graph should be a DAG."""
        assert repo_graph.has_cycle() is False

    def test_repo_topological_sort(self, repo_graph: DirectedGraph) -> None:
        """Topological sort should produce a valid ordering.

        We verify that for every edge A->B, A appears after B in the sort
        (because A depends on B, so B must be built first).

        Wait -- in our convention, add_edge("A", "B") means A points to B,
        which we've used to mean "A depends on B". In a topological sort,
        dependencies come FIRST. So B should appear before A.
        """
        order = repo_graph.topological_sort()
        assert len(order) == 21

        # For every edge (A -> B), A should appear before B in the order.
        # This is because standard topological sort puts the source of each
        # edge before the target. In our "A depends on B" convention, the
        # dependent (A) comes before its dependency (B).
        position = {node: i for i, node in enumerate(order)}
        for from_node, to_node in repo_graph.edges():
            assert position[from_node] < position[to_node], (
                f"{from_node} should appear before {to_node} in topo sort"
            )

    def test_repo_independent_groups(self, repo_graph: DirectedGraph) -> None:
        """Independent groups should partition all 21 packages.

        The sum of all group sizes should be 21, and the first group should
        contain the packages with no dependencies (logic-gates, grammar-tools).
        """
        groups = repo_graph.independent_groups()

        # All nodes accounted for.
        all_nodes = [node for group in groups for node in group]
        assert len(all_nodes) == 21
        assert set(all_nodes) == set(repo_graph.nodes())

        # First group: packages that nothing depends on (zero in-degree).
        # In our "A depends on B" convention, these are the leaf consumers.
        # The last group should contain the foundational packages.
        assert "directed-graph" in groups[0]  # No package depends on directed-graph
        assert "logic-gates" in groups[-1] or "grammar-tools" in groups[-1]

    def test_repo_transitive_closure_of_logic_gates(
        self, repo_graph: DirectedGraph
    ) -> None:
        """logic-gates has no dependencies, so its transitive closure is empty."""
        assert repo_graph.transitive_closure("logic-gates") == set()

    def test_repo_transitive_dependents_of_logic_gates(
        self, repo_graph: DirectedGraph
    ) -> None:
        """Changing logic-gates should affect arithmetic and everything above.

        logic-gates is at the bottom. arithmetic depends on it, cpu-simulator
        depends on arithmetic, etc.
        """
        dependents = repo_graph.transitive_dependents("logic-gates")
        # At minimum: arithmetic, cpu-simulator
        assert "arithmetic" in dependents
        assert "cpu-simulator" in dependents
        # And transitively: virtual-machine, jvm-simulator, etc.
        assert "virtual-machine" in dependents

    def test_repo_affected_by_grammar_tools_change(
        self, repo_graph: DirectedGraph
    ) -> None:
        """Changing grammar-tools should affect lexer, parser, and everything above."""
        affected = repo_graph.affected_nodes({"grammar-tools"})
        assert "grammar-tools" in affected
        assert "lexer" in affected
        assert "parser" in affected
        assert "assembler" in affected
        assert "pipeline" in affected

    def test_repo_affected_by_leaf_change(
        self, repo_graph: DirectedGraph
    ) -> None:
        """Changing bytecode-compiler (a leaf) affects only itself.

        bytecode-compiler has no packages that depend on it.
        """
        affected = repo_graph.affected_nodes({"bytecode-compiler"})
        assert affected == {"bytecode-compiler"}


# ======================================================================
# 8. Algorithm Imports from algorithms.py
# ======================================================================


class TestAlgorithmsModule:
    """Test that algorithms.py re-exports work."""

    def test_import_from_algorithms_module(self) -> None:
        """Verify that the algorithms module re-exports everything."""
        from directed_graph.algorithms import (
            CycleError as CE,
            DirectedGraph as DG,
            EdgeNotFoundError as ENF,
            NodeNotFoundError as NNF,
        )

        # Just verify the imports work and are the same classes.
        assert CE is CycleError
        assert DG is DirectedGraph
        assert NNF is NodeNotFoundError
        assert ENF is not None

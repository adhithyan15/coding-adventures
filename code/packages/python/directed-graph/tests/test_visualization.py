"""Tests for the visualization module.

We test all three output formats (DOT, Mermaid, ASCII) with both graph types
(DirectedGraph and LabeledDirectedGraph). The tests verify both structural
correctness (the output contains the right elements) and format validity
(the output is parseable by the target tool).

Test naming convention:
    test_<format>_<graph_type>_<scenario>

For example:
    test_dot_directed_basic      -- to_dot with a simple DirectedGraph
    test_mermaid_labeled_multi   -- to_mermaid with multi-labeled edges
"""

import pytest

from directed_graph import DirectedGraph, LabeledDirectedGraph
from directed_graph.visualization import (
    _escape_dot,
    _escape_mermaid,
    to_ascii_table,
    to_dot,
    to_mermaid,
)


# ===========================================================================
# Fixtures: reusable graph instances
# ===========================================================================


@pytest.fixture
def simple_dag():
    """A -> B -> C (linear chain)."""
    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    return g


@pytest.fixture
def diamond_dag():
    """A -> B, A -> C, B -> D, C -> D (diamond shape)."""
    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("A", "C")
    g.add_edge("B", "D")
    g.add_edge("C", "D")
    return g


@pytest.fixture
def turnstile():
    """Classic turnstile FSM as a labeled graph."""
    lg = LabeledDirectedGraph()
    lg.add_edge("locked", "unlocked", "coin")
    lg.add_edge("locked", "locked", "push")
    lg.add_edge("unlocked", "unlocked", "coin")
    lg.add_edge("unlocked", "locked", "push")
    return lg


@pytest.fixture
def multi_label_graph():
    """A graph with multiple labels between the same node pair."""
    lg = LabeledDirectedGraph()
    lg.add_edge("A", "B", "x")
    lg.add_edge("A", "B", "y")
    lg.add_edge("B", "C", "z")
    return lg


@pytest.fixture
def self_loop_graph():
    """A labeled graph with a self-loop."""
    lg = LabeledDirectedGraph()
    lg.add_edge("q0", "q0", "a")
    lg.add_edge("q0", "q1", "b")
    return lg


@pytest.fixture
def empty_directed():
    """An empty DirectedGraph."""
    return DirectedGraph()


@pytest.fixture
def empty_labeled():
    """An empty LabeledDirectedGraph."""
    return LabeledDirectedGraph()


@pytest.fixture
def isolated_nodes():
    """A graph with nodes but no edges."""
    g = DirectedGraph()
    g.add_node("X")
    g.add_node("Y")
    g.add_node("Z")
    return g


# ===========================================================================
# Tests for _escape_dot helper
# ===========================================================================


class TestEscapeDot:
    """Tests for the DOT string escaping function."""

    def test_plain_text(self):
        assert _escape_dot("hello") == "hello"

    def test_double_quotes(self):
        assert _escape_dot('say "hi"') == 'say \\"hi\\"'

    def test_backslash(self):
        assert _escape_dot("a\\b") == "a\\\\b"

    def test_angle_brackets(self):
        assert _escape_dot("<html>") == "\\<html\\>"

    def test_braces(self):
        assert _escape_dot("{a|b}") == "\\{a\\|b\\}"

    def test_combined_specials(self):
        result = _escape_dot('"<{|}>\\')
        assert '\\"' in result
        assert "\\<" in result
        assert "\\{" in result
        assert "\\|" in result
        assert "\\}" in result
        assert "\\>" in result


# ===========================================================================
# Tests for _escape_mermaid helper
# ===========================================================================


class TestEscapeMermaid:
    """Tests for the Mermaid string escaping function."""

    def test_plain_text(self):
        assert _escape_mermaid("hello") == "hello"

    def test_double_quotes_replaced(self):
        assert _escape_mermaid('say "hi"') == "say 'hi'"


# ===========================================================================
# Tests for to_dot() with DirectedGraph
# ===========================================================================


class TestDotDirectedGraph:
    """DOT output for unlabeled directed graphs."""

    def test_basic_structure(self, simple_dag):
        """DOT output has digraph header, nodes, and edges."""
        result = to_dot(simple_dag)
        assert result.startswith("digraph G {")
        assert result.endswith("}")
        assert "rankdir=LR;" in result

    def test_nodes_present(self, simple_dag):
        """All nodes appear in the output."""
        result = to_dot(simple_dag)
        assert '"A";' in result
        assert '"B";' in result
        assert '"C";' in result

    def test_edges_present(self, simple_dag):
        """All edges appear as arrow declarations."""
        result = to_dot(simple_dag)
        assert '"A" -> "B";' in result
        assert '"B" -> "C";' in result

    def test_no_label_on_unlabeled_edges(self, simple_dag):
        """Unlabeled graph edges should not have [label=...] attributes."""
        result = to_dot(simple_dag)
        assert "label=" not in result

    def test_custom_name(self, simple_dag):
        """Custom digraph name appears in header."""
        result = to_dot(simple_dag, name="MyGraph")
        assert "digraph MyGraph {" in result

    def test_custom_rankdir(self, simple_dag):
        """Custom rankdir is reflected in output."""
        result = to_dot(simple_dag, rankdir="TB")
        assert "rankdir=TB;" in result

    def test_diamond_graph(self, diamond_dag):
        """Diamond-shaped graph has all four edges."""
        result = to_dot(diamond_dag)
        assert '"A" -> "B";' in result
        assert '"A" -> "C";' in result
        assert '"B" -> "D";' in result
        assert '"C" -> "D";' in result

    def test_node_attrs(self, simple_dag):
        """Custom node attributes appear in brackets."""
        result = to_dot(
            simple_dag,
            node_attrs={"A": {"shape": "box", "color": "red"}},
        )
        assert '"A" [color=red, shape=box];' in result
        # Other nodes should not have attrs
        assert '"B";' in result

    def test_edge_attrs(self, simple_dag):
        """Custom edge attributes appear on the edge."""
        result = to_dot(
            simple_dag,
            edge_attrs={("A", "B"): {"color": "blue", "style": "dashed"}},
        )
        assert "color=blue" in result
        assert "style=dashed" in result

    def test_initial_state(self, simple_dag):
        """Initial state arrow with invisible start node."""
        result = to_dot(simple_dag, initial="A")
        assert "__start [shape=point, width=0.2];" in result
        assert '__start -> "A";' in result

    def test_empty_graph(self, empty_directed):
        """Empty graph produces valid DOT with no nodes or edges."""
        result = to_dot(empty_directed)
        assert "digraph G {" in result
        assert "}" in result

    def test_isolated_nodes(self, isolated_nodes):
        """Isolated nodes appear without edges."""
        result = to_dot(isolated_nodes)
        assert '"X";' in result
        assert '"Y";' in result
        assert '"Z";' in result
        assert "->" not in result


# ===========================================================================
# Tests for to_dot() with LabeledDirectedGraph
# ===========================================================================


class TestDotLabeledGraph:
    """DOT output for labeled directed graphs."""

    def test_edge_labels(self, turnstile):
        """Labeled edges have [label="..."] attributes."""
        result = to_dot(turnstile)
        assert 'label="coin"' in result
        assert 'label="push"' in result

    def test_multi_labels_combined(self, multi_label_graph):
        """Multiple labels between same pair are combined with commas."""
        result = to_dot(multi_label_graph)
        # A->B should have combined label "x, y"
        assert 'label="x, y"' in result
        # B->C should have single label "z"
        assert 'label="z"' in result

    def test_self_loop(self, self_loop_graph):
        """Self-loop edges appear correctly."""
        result = to_dot(self_loop_graph)
        assert '"q0" -> "q0"' in result
        assert '"q0" -> "q1"' in result

    def test_turnstile_all_edges(self, turnstile):
        """All turnstile transitions appear."""
        result = to_dot(turnstile)
        assert '"locked" -> "locked"' in result
        assert '"locked" -> "unlocked"' in result
        assert '"unlocked" -> "locked"' in result
        assert '"unlocked" -> "unlocked"' in result

    def test_initial_state_on_labeled(self, turnstile):
        """Initial state arrow works on labeled graphs."""
        result = to_dot(turnstile, initial="locked")
        assert '__start -> "locked";' in result

    def test_node_attrs_on_labeled(self, turnstile):
        """Node attributes work on labeled graphs."""
        result = to_dot(
            turnstile,
            node_attrs={"unlocked": {"shape": "doublecircle"}},
        )
        assert '"unlocked" [shape=doublecircle];' in result

    def test_empty_labeled_graph(self, empty_labeled):
        """Empty labeled graph produces valid DOT."""
        result = to_dot(empty_labeled)
        assert "digraph G {" in result

    def test_special_chars_in_node_names(self):
        """Node names with special characters are escaped."""
        lg = LabeledDirectedGraph()
        lg.add_edge('say "hi"', "target", "go")
        result = to_dot(lg)
        assert 'say \\"hi\\"' in result


# ===========================================================================
# Tests for to_mermaid() with DirectedGraph
# ===========================================================================


class TestMermaidDirectedGraph:
    """Mermaid output for unlabeled directed graphs."""

    def test_basic_structure(self, simple_dag):
        """Mermaid output starts with graph direction."""
        result = to_mermaid(simple_dag)
        assert result.startswith("graph LR")

    def test_edges_present(self, simple_dag):
        """Edges use --> syntax."""
        result = to_mermaid(simple_dag)
        assert "A --> B" in result
        assert "B --> C" in result

    def test_no_label_syntax(self, simple_dag):
        """Unlabeled edges don't use |label| syntax."""
        result = to_mermaid(simple_dag)
        assert "-->|" not in result

    def test_custom_direction(self, simple_dag):
        """TD direction produces top-down layout."""
        result = to_mermaid(simple_dag, direction="TD")
        assert result.startswith("graph TD")

    def test_initial_state(self, simple_dag):
        """Initial state uses invisible circle syntax."""
        result = to_mermaid(simple_dag, initial="A")
        assert "__start(( ))" in result
        assert "__start(( )) --> A" in result

    def test_empty_graph(self, empty_directed):
        """Empty graph produces just the header."""
        result = to_mermaid(empty_directed)
        assert result == "graph LR"

    def test_isolated_nodes(self, isolated_nodes):
        """Isolated nodes appear as standalone declarations."""
        result = to_mermaid(isolated_nodes)
        assert "X" in result
        assert "Y" in result
        assert "Z" in result

    def test_diamond(self, diamond_dag):
        """Diamond graph has all edges."""
        result = to_mermaid(diamond_dag)
        assert "A --> B" in result
        assert "A --> C" in result
        assert "B --> D" in result
        assert "C --> D" in result


# ===========================================================================
# Tests for to_mermaid() with LabeledDirectedGraph
# ===========================================================================


class TestMermaidLabeledGraph:
    """Mermaid output for labeled directed graphs."""

    def test_labeled_edge_syntax(self, turnstile):
        """Labeled edges use -->|"label"| syntax."""
        result = to_mermaid(turnstile)
        assert '-->|"coin"|' in result
        assert '-->|"push"|' in result

    def test_multi_labels_combined(self, multi_label_graph):
        """Multiple labels between same pair are combined."""
        result = to_mermaid(multi_label_graph)
        assert '-->|"x, y"|' in result
        assert '-->|"z"|' in result

    def test_self_loop(self, self_loop_graph):
        """Self-loops appear in Mermaid output."""
        result = to_mermaid(self_loop_graph)
        assert 'q0 -->|"a"| q0' in result
        assert 'q0 -->|"b"| q1' in result

    def test_initial_state_on_labeled(self, turnstile):
        """Initial state works on labeled graphs."""
        result = to_mermaid(turnstile, initial="locked")
        assert "__start(( )) --> locked" in result

    def test_td_direction(self, turnstile):
        """TD direction works with labeled graphs."""
        result = to_mermaid(turnstile, direction="TD")
        assert result.startswith("graph TD")

    def test_empty_labeled(self, empty_labeled):
        """Empty labeled graph produces just header."""
        result = to_mermaid(empty_labeled)
        assert result == "graph LR"


# ===========================================================================
# Tests for to_ascii_table() with DirectedGraph
# ===========================================================================


class TestAsciiTableDirectedGraph:
    """ASCII table output for unlabeled directed graphs."""

    def test_header_present(self, simple_dag):
        """Table has Node and Successors headers."""
        result = to_ascii_table(simple_dag)
        assert "Node" in result
        assert "Successors" in result

    def test_separator_present(self, simple_dag):
        """Table has a separator line with dashes and plus."""
        result = to_ascii_table(simple_dag)
        lines = result.split("\n")
        assert any("+" in line and "-" in line for line in lines)

    def test_node_successors(self, simple_dag):
        """Each node row shows correct successors."""
        result = to_ascii_table(simple_dag)
        # A -> B
        assert "A" in result
        assert "B" in result
        # C has no successors
        assert "(none)" in result

    def test_multiple_successors(self, diamond_dag):
        """Nodes with multiple successors show comma-separated list."""
        result = to_ascii_table(diamond_dag)
        # A has successors B and C
        lines = result.split("\n")
        a_line = [line for line in lines if line.startswith("A")][0]
        assert "B" in a_line
        assert "C" in a_line

    def test_empty_graph(self, empty_directed):
        """Empty graph shows a placeholder message."""
        result = to_ascii_table(empty_directed)
        assert result == "(empty graph)"

    def test_isolated_nodes_no_successors(self, isolated_nodes):
        """Isolated nodes all show (none) for successors."""
        result = to_ascii_table(isolated_nodes)
        assert result.count("(none)") == 3


# ===========================================================================
# Tests for to_ascii_table() with LabeledDirectedGraph
# ===========================================================================


class TestAsciiTableLabeledGraph:
    """ASCII table output for labeled directed graphs."""

    def test_header_has_labels(self, turnstile):
        """Header row has State and all unique labels."""
        result = to_ascii_table(turnstile)
        first_line = result.split("\n")[0]
        assert "State" in first_line
        assert "coin" in first_line
        assert "push" in first_line

    def test_transition_values(self, turnstile):
        """Cells contain correct target states."""
        result = to_ascii_table(turnstile)
        lines = result.split("\n")
        # Find the "locked" row
        locked_line = [line for line in lines if line.startswith("locked")][0]
        assert "unlocked" in locked_line

    def test_missing_transitions_show_dash(self):
        """Missing transitions show a dash."""
        lg = LabeledDirectedGraph()
        lg.add_edge("A", "B", "x")
        lg.add_edge("B", "C", "y")
        result = to_ascii_table(lg)
        # A has no "y" transition, B has no "x" transition
        assert "-" in result

    def test_multi_label_cells(self):
        """Multiple targets for same (node, label) are comma-separated."""
        lg = LabeledDirectedGraph()
        lg.add_edge("q0", "q1", "a")
        lg.add_edge("q0", "q2", "a")
        result = to_ascii_table(lg)
        lines = result.split("\n")
        q0_line = [line for line in lines if line.startswith("q0")][0]
        # Both q1 and q2 should appear in the "a" column
        assert "q1" in q0_line
        assert "q2" in q0_line

    def test_empty_labeled_graph(self, empty_labeled):
        """Empty labeled graph shows placeholder."""
        result = to_ascii_table(empty_labeled)
        assert result == "(empty graph)"

    def test_labeled_graph_no_edges(self):
        """Labeled graph with nodes but no edges."""
        lg = LabeledDirectedGraph()
        lg.add_node("A")
        lg.add_node("B")
        result = to_ascii_table(lg)
        assert "Node" in result
        assert "(none)" in result

    def test_self_loop_in_table(self, self_loop_graph):
        """Self-loop appears correctly in transition table."""
        result = to_ascii_table(self_loop_graph)
        lines = result.split("\n")
        q0_line = [line for line in lines if line.startswith("q0")][0]
        # q0 on "a" -> q0 (self-loop)
        assert "q0" in q0_line


# ===========================================================================
# Integration / Real-world scenario tests
# ===========================================================================


class TestRealWorldScenarios:
    """Tests using realistic graph configurations."""

    def test_turnstile_dfa_full_dot(self):
        """Full turnstile DFA with initial and accepting states."""
        lg = LabeledDirectedGraph()
        lg.add_edge("locked", "unlocked", "coin")
        lg.add_edge("locked", "locked", "push")
        lg.add_edge("unlocked", "unlocked", "coin")
        lg.add_edge("unlocked", "locked", "push")

        result = to_dot(
            lg,
            name="Turnstile",
            initial="locked",
            node_attrs={"unlocked": {"shape": "doublecircle"}},
            rankdir="LR",
        )

        assert "digraph Turnstile {" in result
        assert "rankdir=LR;" in result
        assert '__start -> "locked";' in result
        assert '"unlocked" [shape=doublecircle];' in result
        assert 'label="coin"' in result
        assert 'label="push"' in result

    def test_build_dependency_graph(self):
        """Build system dependency graph."""
        g = DirectedGraph()
        g.add_edge("logic-gates", "adder")
        g.add_edge("logic-gates", "multiplexer")
        g.add_edge("adder", "alu")
        g.add_edge("multiplexer", "alu")
        g.add_edge("alu", "cpu")

        dot = to_dot(g, name="BuildDeps")
        assert "digraph BuildDeps {" in dot
        assert '"logic-gates" -> "adder";' in dot

        mermaid = to_mermaid(g)
        assert "logic-gates --> adder" in mermaid

        table = to_ascii_table(g)
        assert "logic-gates" in table
        assert "alu" in table

    def test_knowledge_graph_multi_labels(self):
        """Knowledge graph with multiple relationship types."""
        lg = LabeledDirectedGraph()
        lg.add_edge("Alice", "Bob", "friend")
        lg.add_edge("Alice", "Bob", "coworker")
        lg.add_edge("Alice", "Carol", "friend")
        lg.add_edge("Bob", "Carol", "manager")

        dot = to_dot(lg)
        assert 'label="coworker, friend"' in dot  # combined, sorted

        mermaid = to_mermaid(lg)
        assert '-->|"coworker, friend"|' in mermaid

        table = to_ascii_table(lg)
        assert "coworker" in table
        assert "friend" in table
        assert "manager" in table

    def test_single_node_graph(self):
        """Graph with a single isolated node."""
        g = DirectedGraph()
        g.add_node("lonely")

        dot = to_dot(g)
        assert '"lonely";' in dot

        mermaid = to_mermaid(g)
        assert "lonely" in mermaid

        table = to_ascii_table(g)
        assert "lonely" in table
        assert "(none)" in table

    def test_integer_nodes(self):
        """Graphs with integer nodes."""
        g = DirectedGraph()
        g.add_edge(1, 2)
        g.add_edge(2, 3)

        dot = to_dot(g)
        assert '"1" -> "2";' in dot
        assert '"2" -> "3";' in dot

        mermaid = to_mermaid(g)
        assert "1 --> 2" in mermaid

        table = to_ascii_table(g)
        assert "1" in table

    def test_self_loop_unlabeled(self):
        """DirectedGraph with self-loops enabled."""
        g = DirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A")
        g.add_edge("A", "B")

        dot = to_dot(g)
        assert '"A" -> "A";' in dot
        assert '"A" -> "B";' in dot

    def test_large_alphabet_table(self):
        """Labeled graph with many labels produces wide table."""
        lg = LabeledDirectedGraph()
        for char in "abcdefgh":
            lg.add_edge("start", f"s_{char}", char)

        table = to_ascii_table(lg)
        for char in "abcdefgh":
            assert char in table

    def test_dot_output_is_valid_structure(self, turnstile):
        """DOT output has balanced braces and proper structure."""
        result = to_dot(turnstile)
        assert result.count("{") == result.count("}")
        lines = result.split("\n")
        assert lines[0].startswith("digraph")
        assert lines[-1] == "}"

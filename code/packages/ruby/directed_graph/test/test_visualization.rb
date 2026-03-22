# frozen_string_literal: true

# --------------------------------------------------------------------------
# test_visualization.rb — Tests for DOT, Mermaid, and ASCII visualization
# --------------------------------------------------------------------------
#
# We test all three output formats with both graph types (Graph and
# LabeledGraph).  Tests verify structural correctness (the output contains
# the right elements) and format validity.
#
# Test naming convention:
#   test_<format>_<graph_type>_<scenario>
# --------------------------------------------------------------------------

require "test_helper"

# rubocop:disable Metrics/ClassLength
class TestVisualization < Minitest::Test
  V = CodingAdventures::DirectedGraph

  # ======================================================================
  # Helpers: reusable graph builders
  # ======================================================================

  def simple_dag
    g = V::Graph.new
    g.add_edge("A", "B")
    g.add_edge("B", "C")
    g
  end

  def diamond_dag
    g = V::Graph.new
    g.add_edge("A", "B")
    g.add_edge("A", "C")
    g.add_edge("B", "D")
    g.add_edge("C", "D")
    g
  end

  def turnstile
    lg = V::LabeledGraph.new
    lg.add_edge("locked", "unlocked", "coin")
    lg.add_edge("locked", "locked", "push")
    lg.add_edge("unlocked", "unlocked", "coin")
    lg.add_edge("unlocked", "locked", "push")
    lg
  end

  def multi_label_graph
    lg = V::LabeledGraph.new
    lg.add_edge("A", "B", "x")
    lg.add_edge("A", "B", "y")
    lg.add_edge("B", "C", "z")
    lg
  end

  def self_loop_graph
    lg = V::LabeledGraph.new
    lg.add_edge("q0", "q0", "a")
    lg.add_edge("q0", "q1", "b")
    lg
  end

  def empty_graph
    V::Graph.new
  end

  def empty_labeled_graph
    V::LabeledGraph.new
  end

  def isolated_nodes_graph
    g = V::Graph.new
    g.add_node("X")
    g.add_node("Y")
    g.add_node("Z")
    g
  end

  # ======================================================================
  # Tests: escape_dot
  # ======================================================================

  def test_escape_dot_plain_text
    assert_equal "hello", V.escape_dot("hello")
  end

  def test_escape_dot_double_quotes
    result = V.escape_dot('say "hi"')
    assert_includes result, '\\"'
  end

  def test_escape_dot_angle_brackets
    result = V.escape_dot("<html>")
    assert_includes result, "\\<"
    assert_includes result, "\\>"
  end

  def test_escape_dot_braces_and_pipe
    result = V.escape_dot("{a|b}")
    assert_includes result, "\\{"
    assert_includes result, "\\|"
    assert_includes result, "\\}"
  end

  # ======================================================================
  # Tests: escape_mermaid
  # ======================================================================

  def test_escape_mermaid_plain_text
    assert_equal "hello", V.escape_mermaid("hello")
  end

  def test_escape_mermaid_double_quotes
    assert_equal "say 'hi'", V.escape_mermaid('say "hi"')
  end

  # ======================================================================
  # Tests: to_dot with Graph (unlabeled)
  # ======================================================================

  def test_dot_directed_basic_structure
    result = V.to_dot(simple_dag)
    assert result.start_with?("digraph G {")
    assert result.end_with?("}")
    assert_includes result, "rankdir=LR;"
  end

  def test_dot_directed_nodes_present
    result = V.to_dot(simple_dag)
    assert_includes result, '"A";'
    assert_includes result, '"B";'
    assert_includes result, '"C";'
  end

  def test_dot_directed_edges_present
    result = V.to_dot(simple_dag)
    assert_includes result, '"A" -> "B";'
    assert_includes result, '"B" -> "C";'
  end

  def test_dot_directed_no_labels_on_edges
    result = V.to_dot(simple_dag)
    refute_includes result, "label="
  end

  def test_dot_directed_custom_name
    result = V.to_dot(simple_dag, name: "MyGraph")
    assert_includes result, "digraph MyGraph {"
  end

  def test_dot_directed_custom_rankdir
    result = V.to_dot(simple_dag, rankdir: "TB")
    assert_includes result, "rankdir=TB;"
  end

  def test_dot_directed_diamond
    result = V.to_dot(diamond_dag)
    assert_includes result, '"A" -> "B";'
    assert_includes result, '"A" -> "C";'
    assert_includes result, '"B" -> "D";'
    assert_includes result, '"C" -> "D";'
  end

  def test_dot_directed_node_attrs
    result = V.to_dot(simple_dag, node_attrs: {"A" => {"shape" => "box", "color" => "red"}})
    assert_includes result, '"A" [color=red, shape=box];'
    assert_includes result, '"B";'
  end

  def test_dot_directed_edge_attrs
    result = V.to_dot(simple_dag,
      edge_attrs: {["A", "B"] => {"color" => "blue", "style" => "dashed"}})
    assert_includes result, "color=blue"
    assert_includes result, "style=dashed"
  end

  def test_dot_directed_initial_state
    result = V.to_dot(simple_dag, initial: "A")
    assert_includes result, "__start [shape=point, width=0.2];"
    assert_includes result, '__start -> "A";'
  end

  def test_dot_directed_empty_graph
    result = V.to_dot(empty_graph)
    assert_includes result, "digraph G {"
    assert_includes result, "}"
  end

  def test_dot_directed_isolated_nodes
    result = V.to_dot(isolated_nodes_graph)
    assert_includes result, '"X";'
    assert_includes result, '"Y";'
    assert_includes result, '"Z";'
    refute_includes result, "->"
  end

  # ======================================================================
  # Tests: to_dot with LabeledGraph
  # ======================================================================

  def test_dot_labeled_edge_labels
    result = V.to_dot(turnstile)
    assert_includes result, 'label="coin"'
    assert_includes result, 'label="push"'
  end

  def test_dot_labeled_multi_labels_combined
    result = V.to_dot(multi_label_graph)
    assert_includes result, 'label="x, y"'
    assert_includes result, 'label="z"'
  end

  def test_dot_labeled_self_loop
    result = V.to_dot(self_loop_graph)
    assert_includes result, '"q0" -> "q0"'
    assert_includes result, '"q0" -> "q1"'
  end

  def test_dot_labeled_turnstile_all_edges
    result = V.to_dot(turnstile)
    assert_includes result, '"locked" -> "locked"'
    assert_includes result, '"locked" -> "unlocked"'
    assert_includes result, '"unlocked" -> "locked"'
    assert_includes result, '"unlocked" -> "unlocked"'
  end

  def test_dot_labeled_initial_state
    result = V.to_dot(turnstile, initial: "locked")
    assert_includes result, '__start -> "locked";'
  end

  def test_dot_labeled_node_attrs
    result = V.to_dot(turnstile, node_attrs: {"unlocked" => {"shape" => "doublecircle"}})
    assert_includes result, '"unlocked" [shape=doublecircle];'
  end

  def test_dot_labeled_empty
    result = V.to_dot(empty_labeled_graph)
    assert_includes result, "digraph G {"
  end

  def test_dot_labeled_special_chars
    lg = V::LabeledGraph.new
    lg.add_edge('say "hi"', "target", "go")
    result = V.to_dot(lg)
    assert_includes result, 'say \\"hi\\"'
  end

  # ======================================================================
  # Tests: to_mermaid with Graph (unlabeled)
  # ======================================================================

  def test_mermaid_directed_basic_structure
    result = V.to_mermaid(simple_dag)
    assert result.start_with?("graph LR")
  end

  def test_mermaid_directed_edges
    result = V.to_mermaid(simple_dag)
    assert_includes result, "A --> B"
    assert_includes result, "B --> C"
  end

  def test_mermaid_directed_no_label_syntax
    result = V.to_mermaid(simple_dag)
    refute_includes result, "-->|"
  end

  def test_mermaid_directed_td_direction
    result = V.to_mermaid(simple_dag, direction: "TD")
    assert result.start_with?("graph TD")
  end

  def test_mermaid_directed_initial_state
    result = V.to_mermaid(simple_dag, initial: "A")
    assert_includes result, "__start(( ))"
    assert_includes result, "__start(( )) --> A"
  end

  def test_mermaid_directed_empty
    result = V.to_mermaid(empty_graph)
    assert_equal "graph LR", result
  end

  def test_mermaid_directed_isolated_nodes
    result = V.to_mermaid(isolated_nodes_graph)
    assert_includes result, "X"
    assert_includes result, "Y"
    assert_includes result, "Z"
  end

  def test_mermaid_directed_diamond
    result = V.to_mermaid(diamond_dag)
    assert_includes result, "A --> B"
    assert_includes result, "A --> C"
    assert_includes result, "B --> D"
    assert_includes result, "C --> D"
  end

  # ======================================================================
  # Tests: to_mermaid with LabeledGraph
  # ======================================================================

  def test_mermaid_labeled_edge_syntax
    result = V.to_mermaid(turnstile)
    assert_includes result, '-->|"coin"|'
    assert_includes result, '-->|"push"|'
  end

  def test_mermaid_labeled_multi_labels_combined
    result = V.to_mermaid(multi_label_graph)
    assert_includes result, '-->|"x, y"|'
    assert_includes result, '-->|"z"|'
  end

  def test_mermaid_labeled_self_loop
    result = V.to_mermaid(self_loop_graph)
    assert_includes result, 'q0 -->|"a"| q0'
    assert_includes result, 'q0 -->|"b"| q1'
  end

  def test_mermaid_labeled_initial_state
    result = V.to_mermaid(turnstile, initial: "locked")
    assert_includes result, "__start(( )) --> locked"
  end

  def test_mermaid_labeled_td_direction
    result = V.to_mermaid(turnstile, direction: "TD")
    assert result.start_with?("graph TD")
  end

  def test_mermaid_labeled_empty
    result = V.to_mermaid(empty_labeled_graph)
    assert_equal "graph LR", result
  end

  # ======================================================================
  # Tests: to_ascii_table with Graph (unlabeled)
  # ======================================================================

  def test_ascii_directed_header
    result = V.to_ascii_table(simple_dag)
    assert_includes result, "Node"
    assert_includes result, "Successors"
  end

  def test_ascii_directed_separator
    result = V.to_ascii_table(simple_dag)
    lines = result.split("\n")
    assert(lines.any? { |line| line.include?("+") && line.include?("-") })
  end

  def test_ascii_directed_node_successors
    result = V.to_ascii_table(simple_dag)
    assert_includes result, "A"
    assert_includes result, "B"
    assert_includes result, "(none)"
  end

  def test_ascii_directed_multiple_successors
    result = V.to_ascii_table(diamond_dag)
    lines = result.split("\n")
    a_line = lines.find { |line| line.start_with?("A") }
    assert_includes a_line, "B"
    assert_includes a_line, "C"
  end

  def test_ascii_directed_empty
    result = V.to_ascii_table(empty_graph)
    assert_equal "(empty graph)", result
  end

  def test_ascii_directed_isolated_nodes
    result = V.to_ascii_table(isolated_nodes_graph)
    assert_equal 3, result.scan("(none)").length
  end

  # ======================================================================
  # Tests: to_ascii_table with LabeledGraph
  # ======================================================================

  def test_ascii_labeled_header_has_labels
    result = V.to_ascii_table(turnstile)
    first_line = result.split("\n").first
    assert_includes first_line, "State"
    assert_includes first_line, "coin"
    assert_includes first_line, "push"
  end

  def test_ascii_labeled_transition_values
    result = V.to_ascii_table(turnstile)
    lines = result.split("\n")
    locked_line = lines.find { |line| line.start_with?("locked") }
    assert_includes locked_line, "unlocked"
  end

  def test_ascii_labeled_missing_transitions
    lg = V::LabeledGraph.new
    lg.add_edge("A", "B", "x")
    lg.add_edge("B", "C", "y")
    result = V.to_ascii_table(lg)
    assert_includes result, "-"
  end

  def test_ascii_labeled_multi_targets
    lg = V::LabeledGraph.new
    lg.add_edge("q0", "q1", "a")
    lg.add_edge("q0", "q2", "a")
    result = V.to_ascii_table(lg)
    lines = result.split("\n")
    q0_line = lines.find { |line| line.start_with?("q0") }
    assert_includes q0_line, "q1"
    assert_includes q0_line, "q2"
  end

  def test_ascii_labeled_empty
    result = V.to_ascii_table(empty_labeled_graph)
    assert_equal "(empty graph)", result
  end

  def test_ascii_labeled_no_edges
    lg = V::LabeledGraph.new
    lg.add_node("A")
    lg.add_node("B")
    result = V.to_ascii_table(lg)
    assert_includes result, "Node"
    assert_includes result, "(none)"
  end

  def test_ascii_labeled_self_loop
    result = V.to_ascii_table(self_loop_graph)
    lines = result.split("\n")
    q0_line = lines.find { |line| line.start_with?("q0") }
    assert_includes q0_line, "q0"
  end

  # ======================================================================
  # Integration / Real-world scenarios
  # ======================================================================

  def test_turnstile_dfa_full_dot
    lg = turnstile
    result = V.to_dot(lg,
      name: "Turnstile",
      initial: "locked",
      node_attrs: {"unlocked" => {"shape" => "doublecircle"}},
      rankdir: "LR")

    assert_includes result, "digraph Turnstile {"
    assert_includes result, "rankdir=LR;"
    assert_includes result, '__start -> "locked";'
    assert_includes result, '"unlocked" [shape=doublecircle];'
    assert_includes result, 'label="coin"'
    assert_includes result, 'label="push"'
  end

  def test_build_dependency_graph
    g = V::Graph.new
    g.add_edge("logic-gates", "adder")
    g.add_edge("logic-gates", "multiplexer")
    g.add_edge("adder", "alu")
    g.add_edge("multiplexer", "alu")
    g.add_edge("alu", "cpu")

    dot = V.to_dot(g, name: "BuildDeps")
    assert_includes dot, "digraph BuildDeps {"
    assert_includes dot, '"logic-gates" -> "adder";'

    mermaid = V.to_mermaid(g)
    assert_includes mermaid, "logic-gates --> adder"

    table = V.to_ascii_table(g)
    assert_includes table, "logic-gates"
    assert_includes table, "alu"
  end

  def test_knowledge_graph_multi_labels
    lg = V::LabeledGraph.new
    lg.add_edge("Alice", "Bob", "friend")
    lg.add_edge("Alice", "Bob", "coworker")
    lg.add_edge("Alice", "Carol", "friend")
    lg.add_edge("Bob", "Carol", "manager")

    dot = V.to_dot(lg)
    assert_includes dot, 'label="coworker, friend"'

    mermaid = V.to_mermaid(lg)
    assert_includes mermaid, '-->|"coworker, friend"|'

    table = V.to_ascii_table(lg)
    assert_includes table, "coworker"
    assert_includes table, "friend"
    assert_includes table, "manager"
  end

  def test_single_node_graph
    g = V::Graph.new
    g.add_node("lonely")

    dot = V.to_dot(g)
    assert_includes dot, '"lonely";'

    mermaid = V.to_mermaid(g)
    assert_includes mermaid, "lonely"

    table = V.to_ascii_table(g)
    assert_includes table, "lonely"
    assert_includes table, "(none)"
  end

  def test_dot_balanced_braces
    result = V.to_dot(turnstile)
    assert_equal result.count("{"), result.count("}")
    lines = result.split("\n")
    assert lines.first.start_with?("digraph")
    assert_equal "}", lines.last
  end

  def test_self_loop_unlabeled
    g = V::Graph.new(allow_self_loops: true)
    g.add_edge("A", "A")
    g.add_edge("A", "B")

    dot = V.to_dot(g)
    assert_includes dot, '"A" -> "A";'
    assert_includes dot, '"A" -> "B";'
  end

  def test_large_alphabet_table
    lg = V::LabeledGraph.new
    ("a".."h").each do |char|
      lg.add_edge("start", "s_#{char}", char)
    end

    table = V.to_ascii_table(lg)
    ("a".."h").each do |char|
      assert_includes table, char
    end
  end
end
# rubocop:enable Metrics/ClassLength

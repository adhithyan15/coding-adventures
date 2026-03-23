# frozen_string_literal: true

# --------------------------------------------------------------------------
# graph_test.rb — Tests for the Rust-backed DirectedGraphNative::Graph
# --------------------------------------------------------------------------
#
# These tests mirror the pure Ruby directed_graph tests to ensure the
# native extension provides identical behavior.

require "minitest/autorun"
require_relative "../lib/coding_adventures_directed_graph_native"

class GraphTest < Minitest::Test
  # -- Setup ---------------------------------------------------------------

  def setup
    @graph = CodingAdventures::DirectedGraphNative::Graph.new
  end

  # -- Node operations -----------------------------------------------------

  def test_add_and_has_node
    @graph.add_node("A")
    assert @graph.has_node?("A")
    refute @graph.has_node?("B")
  end

  def test_add_node_is_idempotent
    @graph.add_node("A")
    @graph.add_node("A")
    assert_equal 1, @graph.size
  end

  def test_nodes_returns_sorted_array
    @graph.add_node("C")
    @graph.add_node("A")
    @graph.add_node("B")
    assert_equal %w[A B C], @graph.nodes
  end

  def test_size
    assert_equal 0, @graph.size
    @graph.add_node("A")
    assert_equal 1, @graph.size
    @graph.add_node("B")
    assert_equal 2, @graph.size
  end

  def test_remove_node
    @graph.add_node("A")
    @graph.add_node("B")
    @graph.add_edge("A", "B")
    @graph.remove_node("A")

    refute @graph.has_node?("A")
    assert @graph.has_node?("B")
    assert_equal [], @graph.edges
  end

  def test_remove_node_not_found
    assert_raises(RuntimeError) { @graph.remove_node("X") }
  end

  # -- Edge operations -----------------------------------------------------

  def test_add_and_has_edge
    @graph.add_edge("A", "B")
    assert @graph.has_edge?("A", "B")
    refute @graph.has_edge?("B", "A")
  end

  def test_add_edge_creates_nodes
    @graph.add_edge("A", "B")
    assert @graph.has_node?("A")
    assert @graph.has_node?("B")
  end

  def test_add_edge_self_loop_raises
    assert_raises(ArgumentError) { @graph.add_edge("A", "A") }
  end

  def test_edges_returns_sorted_pairs
    @graph.add_edge("B", "C")
    @graph.add_edge("A", "B")
    edges = @graph.edges
    assert_equal [%w[A B], %w[B C]], edges
  end

  def test_remove_edge
    @graph.add_edge("A", "B")
    @graph.remove_edge("A", "B")
    refute @graph.has_edge?("A", "B")
    # Nodes should still exist
    assert @graph.has_node?("A")
    assert @graph.has_node?("B")
  end

  def test_remove_edge_not_found
    @graph.add_node("A")
    @graph.add_node("B")
    assert_raises(RuntimeError) { @graph.remove_edge("A", "B") }
  end

  # -- Neighbor queries ----------------------------------------------------

  def test_predecessors
    @graph.add_edge("A", "C")
    @graph.add_edge("B", "C")
    assert_equal %w[A B], @graph.predecessors("C")
  end

  def test_predecessors_not_found
    assert_raises(RuntimeError) { @graph.predecessors("X") }
  end

  def test_successors
    @graph.add_edge("A", "B")
    @graph.add_edge("A", "C")
    assert_equal %w[B C], @graph.successors("A")
  end

  def test_successors_not_found
    assert_raises(RuntimeError) { @graph.successors("X") }
  end

  # -- Topological sort ----------------------------------------------------

  def test_topological_sort_linear
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    assert_equal %w[A B C], @graph.topological_sort
  end

  def test_topological_sort_diamond
    @graph.add_edge("A", "B")
    @graph.add_edge("A", "C")
    @graph.add_edge("B", "D")
    @graph.add_edge("C", "D")
    result = @graph.topological_sort

    # A must come before B, C; B and C must come before D
    assert_operator result.index("A"), :<, result.index("B")
    assert_operator result.index("A"), :<, result.index("C")
    assert_operator result.index("B"), :<, result.index("D")
    assert_operator result.index("C"), :<, result.index("D")
  end

  def test_topological_sort_with_cycle_raises
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    @graph.add_edge("C", "A")
    assert_raises(RuntimeError) { @graph.topological_sort }
  end

  # -- Cycle detection -----------------------------------------------------

  def test_has_cycle_false
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    refute @graph.has_cycle?
  end

  def test_has_cycle_true
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    @graph.add_edge("C", "A")
    assert @graph.has_cycle?
  end

  def test_empty_graph_has_no_cycle
    refute @graph.has_cycle?
  end

  # -- Transitive closure --------------------------------------------------

  def test_transitive_closure
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    @graph.add_edge("C", "D")

    closure = @graph.transitive_closure("A")
    assert_equal %w[B C D], closure
  end

  def test_transitive_closure_leaf_node
    @graph.add_edge("A", "B")
    closure = @graph.transitive_closure("B")
    assert_equal [], closure
  end

  def test_transitive_closure_not_found
    assert_raises(RuntimeError) { @graph.transitive_closure("X") }
  end

  # -- Affected nodes ------------------------------------------------------

  def test_affected_nodes
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    @graph.add_edge("D", "C")

    affected = @graph.affected_nodes(["A"])
    assert_equal %w[A B C], affected
  end

  def test_affected_nodes_multiple_changed
    @graph.add_edge("A", "C")
    @graph.add_edge("B", "C")
    @graph.add_edge("C", "D")

    affected = @graph.affected_nodes(%w[A B])
    assert_equal %w[A B C D], affected
  end

  # -- Independent groups --------------------------------------------------

  def test_independent_groups_linear
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "C")
    groups = @graph.independent_groups
    assert_equal [["A"], ["B"], ["C"]], groups
  end

  def test_independent_groups_parallel
    @graph.add_edge("A", "C")
    @graph.add_edge("B", "C")
    groups = @graph.independent_groups
    assert_equal [%w[A B], ["C"]], groups
  end

  def test_independent_groups_with_cycle_raises
    @graph.add_edge("A", "B")
    @graph.add_edge("B", "A")
    assert_raises(RuntimeError) { @graph.independent_groups }
  end

  # -- Empty graph ---------------------------------------------------------

  def test_empty_graph
    assert_equal 0, @graph.size
    assert_equal [], @graph.nodes
    assert_equal [], @graph.edges
    assert_equal [], @graph.topological_sort
    assert_equal [], @graph.independent_groups
  end
end

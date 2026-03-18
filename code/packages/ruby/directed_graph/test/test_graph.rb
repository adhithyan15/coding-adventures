# frozen_string_literal: true

require_relative "test_helper"

# --------------------------------------------------------------------------
# test_graph.rb — Tests for the core Graph data structure
# --------------------------------------------------------------------------
#
# We organise tests into logical groups:
#   1. Empty graph behaviour
#   2. Single-node operations
#   3. Adding and removing edges
#   4. Error conditions (self-loops, missing nodes/edges)
#   5. Predecessors and successors
#   6. Size and membership queries
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    class TestGraphCore < Minitest::Test
      # ----------------------------------------------------------------
      # Empty graph
      # ----------------------------------------------------------------

      def test_empty_graph_has_no_nodes
        g = Graph.new
        assert_equal [], g.nodes
      end

      def test_empty_graph_has_no_edges
        g = Graph.new
        assert_equal [], g.edges
      end

      def test_empty_graph_size_is_zero
        g = Graph.new
        assert_equal 0, g.size
      end

      # ----------------------------------------------------------------
      # Single node
      # ----------------------------------------------------------------

      def test_add_single_node
        g = Graph.new
        g.add_node("A")
        assert_equal ["A"], g.nodes
        assert_equal 1, g.size
      end

      def test_add_duplicate_node_is_noop
        g = Graph.new
        g.add_node("A")
        g.add_node("A")
        assert_equal ["A"], g.nodes
        assert_equal 1, g.size
      end

      def test_add_node_returns_self_for_chaining
        g = Graph.new
        result = g.add_node("A")
        assert_same g, result
      end

      def test_has_node_returns_true_for_existing
        g = Graph.new
        g.add_node("X")
        assert g.has_node?("X")
      end

      def test_has_node_returns_false_for_missing
        g = Graph.new
        refute g.has_node?("X")
      end

      # ----------------------------------------------------------------
      # Adding edges
      # ----------------------------------------------------------------

      def test_add_edge_creates_both_nodes
        g = Graph.new
        g.add_edge("A", "B")
        assert g.has_node?("A")
        assert g.has_node?("B")
        assert_equal 2, g.size
      end

      def test_add_edge_creates_directed_connection
        g = Graph.new
        g.add_edge("A", "B")
        assert g.has_edge?("A", "B")
        refute g.has_edge?("B", "A")
      end

      def test_add_duplicate_edge_is_noop
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "B")
        assert_equal [["A", "B"]], g.edges
      end

      def test_add_edge_returns_self_for_chaining
        g = Graph.new
        result = g.add_edge("A", "B")
        assert_same g, result
      end

      def test_self_loop_raises_cycle_error
        g = Graph.new
        assert_raises(CycleError) { g.add_edge("A", "A") }
      end

      def test_self_loop_error_message_includes_node
        g = Graph.new
        error = assert_raises(CycleError) { g.add_edge("A", "A") }
        assert_match(/Self-loop/, error.message)
        assert_match(/"A"/, error.message)
      end

      # ----------------------------------------------------------------
      # Removing nodes
      # ----------------------------------------------------------------

      def test_remove_node_deletes_node
        g = Graph.new
        g.add_node("A")
        g.remove_node("A")
        refute g.has_node?("A")
        assert_equal 0, g.size
      end

      def test_remove_node_cleans_up_outgoing_edges
        g = Graph.new
        g.add_edge("A", "B")
        g.remove_node("A")
        refute g.has_edge?("A", "B")
        assert_equal [], g.predecessors("B")
      end

      def test_remove_node_cleans_up_incoming_edges
        g = Graph.new
        g.add_edge("A", "B")
        g.remove_node("B")
        refute g.has_edge?("A", "B")
        assert_equal [], g.successors("A")
      end

      def test_remove_missing_node_raises_error
        g = Graph.new
        assert_raises(NodeNotFoundError) { g.remove_node("Z") }
      end

      def test_remove_node_returns_self_for_chaining
        g = Graph.new
        g.add_node("A")
        result = g.remove_node("A")
        assert_same g, result
      end

      # ----------------------------------------------------------------
      # Removing edges
      # ----------------------------------------------------------------

      def test_remove_edge
        g = Graph.new
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        refute g.has_edge?("A", "B")
        # Both nodes should still exist.
        assert g.has_node?("A")
        assert g.has_node?("B")
      end

      def test_remove_edge_missing_source_raises
        g = Graph.new
        g.add_node("B")
        assert_raises(NodeNotFoundError) { g.remove_edge("A", "B") }
      end

      def test_remove_edge_missing_target_raises
        g = Graph.new
        g.add_node("A")
        assert_raises(NodeNotFoundError) { g.remove_edge("A", "B") }
      end

      def test_remove_nonexistent_edge_raises
        g = Graph.new
        g.add_node("A")
        g.add_node("B")
        assert_raises(EdgeNotFoundError) { g.remove_edge("A", "B") }
      end

      def test_remove_edge_returns_self_for_chaining
        g = Graph.new
        g.add_edge("A", "B")
        result = g.remove_edge("A", "B")
        assert_same g, result
      end

      # ----------------------------------------------------------------
      # Predecessors and successors
      # ----------------------------------------------------------------

      def test_predecessors
        g = Graph.new
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        assert_equal %w[A B], g.predecessors("C")
      end

      def test_successors
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        assert_equal %w[B C], g.successors("A")
      end

      def test_predecessors_missing_node_raises
        g = Graph.new
        assert_raises(NodeNotFoundError) { g.predecessors("Z") }
      end

      def test_successors_missing_node_raises
        g = Graph.new
        assert_raises(NodeNotFoundError) { g.successors("Z") }
      end

      def test_predecessors_of_root_is_empty
        g = Graph.new
        g.add_edge("A", "B")
        assert_equal [], g.predecessors("A")
      end

      def test_successors_of_leaf_is_empty
        g = Graph.new
        g.add_edge("A", "B")
        assert_equal [], g.successors("B")
      end

      # ----------------------------------------------------------------
      # Edges listing
      # ----------------------------------------------------------------

      def test_edges_returns_all_pairs_sorted
        g = Graph.new
        g.add_edge("B", "C")
        g.add_edge("A", "B")
        assert_equal [["A", "B"], ["B", "C"]], g.edges
      end

      # ----------------------------------------------------------------
      # has_edge? on missing nodes
      # ----------------------------------------------------------------

      def test_has_edge_returns_false_for_missing_source
        g = Graph.new
        g.add_node("B")
        refute g.has_edge?("A", "B")
      end

      def test_has_edge_returns_false_for_missing_target
        g = Graph.new
        g.add_node("A")
        refute g.has_edge?("A", "B")
      end
    end
  end
end

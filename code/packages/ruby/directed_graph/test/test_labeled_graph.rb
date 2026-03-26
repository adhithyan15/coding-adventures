# frozen_string_literal: true

require_relative "test_helper"

# --------------------------------------------------------------------------
# test_labeled_graph.rb — Tests for the LabeledGraph class
# --------------------------------------------------------------------------
#
# A LabeledGraph extends a regular directed graph with labels on edges.
# Think of it like a state machine: nodes are states, and each edge
# carries a label (the input symbol that triggers the transition).
#
# We organize tests into logical groups:
#   1.  Empty graph behaviour
#   2.  Node operations (add, remove, query)
#   3.  Adding labeled edges
#   4.  Removing labeled edges
#   5.  Edge queries (has_edge?, edges, labels)
#   6.  Multi-label edges (same pair, different labels)
#   7.  Self-loops (a state transitioning to itself)
#   8.  Successors and predecessors with label filtering
#   9.  Algorithm delegation (topological_sort, has_cycle?, etc.)
#   10. Error conditions
#   11. Complex graph scenarios
#   12. State machine modeling (integration test)
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    # ==================================================================
    # 1. Empty graph
    # ==================================================================

    class TestLabeledGraphEmpty < Minitest::Test
      def test_empty_graph_has_no_nodes
        g = LabeledGraph.new
        assert_equal [], g.nodes
      end

      def test_empty_graph_has_no_edges
        g = LabeledGraph.new
        assert_equal [], g.edges
      end

      def test_empty_graph_size_is_zero
        g = LabeledGraph.new
        assert_equal 0, g.size
      end
    end

    # ==================================================================
    # 2. Node operations
    # ==================================================================

    class TestLabeledGraphNodes < Minitest::Test
      def test_add_node
        g = LabeledGraph.new
        g.add_node("A")
        assert g.has_node?("A")
        assert_equal 1, g.size
      end

      def test_add_node_returns_self
        g = LabeledGraph.new
        result = g.add_node("A")
        assert_same g, result
      end

      def test_add_duplicate_node_is_noop
        g = LabeledGraph.new
        g.add_node("A")
        g.add_node("A")
        assert_equal 1, g.size
      end

      def test_has_node_false_for_missing
        g = LabeledGraph.new
        refute g.has_node?("X")
      end

      def test_remove_node
        g = LabeledGraph.new
        g.add_node("A")
        g.remove_node("A")
        refute g.has_node?("A")
        assert_equal 0, g.size
      end

      def test_remove_node_returns_self
        g = LabeledGraph.new
        g.add_node("A")
        result = g.remove_node("A")
        assert_same g, result
      end

      def test_remove_node_cleans_up_outgoing_edges
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.remove_node("A")
        refute g.has_edge?("A", "B")
        assert_equal [], g.edges
      end

      def test_remove_node_cleans_up_incoming_edges
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.remove_node("B")
        refute g.has_edge?("A", "B")
        assert_equal [], g.edges
      end

      def test_remove_node_cleans_up_self_loop
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        g.remove_node("A")
        refute g.has_node?("A")
        assert_equal [], g.edges
      end

      def test_nodes_sorted
        g = LabeledGraph.new
        g.add_node("C")
        g.add_node("A")
        g.add_node("B")
        assert_equal %w[A B C], g.nodes
      end
    end

    # ==================================================================
    # 3. Adding labeled edges
    # ==================================================================

    class TestLabeledGraphAddEdge < Minitest::Test
      def test_add_edge_creates_both_nodes
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert g.has_node?("A")
        assert g.has_node?("B")
      end

      def test_add_edge_creates_edge
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert g.has_edge?("A", "B")
      end

      def test_add_edge_creates_labeled_edge
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert g.has_edge?("A", "B", "x")
      end

      def test_add_edge_returns_self
        g = LabeledGraph.new
        result = g.add_edge("A", "B", "x")
        assert_same g, result
      end

      def test_add_duplicate_edge_is_noop
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "x")
        assert_equal [["A", "B", "x"]], g.edges
      end

      def test_add_edge_reverse_does_not_exist
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        refute g.has_edge?("B", "A")
        refute g.has_edge?("B", "A", "x")
      end
    end

    # ==================================================================
    # 4. Removing labeled edges
    # ==================================================================

    class TestLabeledGraphRemoveEdge < Minitest::Test
      def test_remove_edge
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.remove_edge("A", "B", "x")
        refute g.has_edge?("A", "B", "x")
        refute g.has_edge?("A", "B")
      end

      def test_remove_edge_returns_self
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        result = g.remove_edge("A", "B", "x")
        assert_same g, result
      end

      def test_remove_edge_keeps_nodes
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.remove_edge("A", "B", "x")
        assert g.has_node?("A")
        assert g.has_node?("B")
      end

      def test_remove_one_label_keeps_other
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        g.remove_edge("A", "B", "x")
        refute g.has_edge?("A", "B", "x")
        assert g.has_edge?("A", "B", "y")
        # The structural edge should still exist.
        assert g.has_edge?("A", "B")
      end

      def test_remove_last_label_removes_structural_edge
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.remove_edge("A", "B", "x")
        refute g.has_edge?("A", "B")
      end
    end

    # ==================================================================
    # 5. Edge queries
    # ==================================================================

    class TestLabeledGraphEdgeQueries < Minitest::Test
      def test_has_edge_without_label
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert g.has_edge?("A", "B")
      end

      def test_has_edge_with_matching_label
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert g.has_edge?("A", "B", "x")
      end

      def test_has_edge_with_wrong_label
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        refute g.has_edge?("A", "B", "y")
      end

      def test_has_edge_missing_source
        g = LabeledGraph.new
        g.add_node("B")
        refute g.has_edge?("A", "B")
      end

      def test_has_edge_missing_target
        g = LabeledGraph.new
        g.add_node("A")
        refute g.has_edge?("A", "B")
      end

      def test_edges_returns_triples
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert_equal [["A", "B", "x"]], g.edges
      end

      def test_edges_sorted
        g = LabeledGraph.new
        g.add_edge("B", "C", "y")
        g.add_edge("A", "B", "x")
        assert_equal [["A", "B", "x"], ["B", "C", "y"]], g.edges
      end

      def test_labels_returns_set
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        assert_equal Set["x", "y"], g.labels("A", "B")
      end

      def test_labels_returns_empty_set_for_no_edge
        g = LabeledGraph.new
        g.add_node("A")
        g.add_node("B")
        assert_equal Set.new, g.labels("A", "B")
      end

      def test_labels_returns_copy
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        labels = g.labels("A", "B")
        labels.add("sneaky")
        # Modifying the returned set should not affect the graph.
        refute g.has_edge?("A", "B", "sneaky")
      end
    end

    # ==================================================================
    # 6. Multi-label edges
    # ==================================================================

    class TestLabeledGraphMultiLabel < Minitest::Test
      def test_two_labels_on_same_pair
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        assert g.has_edge?("A", "B", "x")
        assert g.has_edge?("A", "B", "y")
      end

      def test_three_labels_on_same_pair
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        g.add_edge("A", "B", "z")
        assert_equal Set["x", "y", "z"], g.labels("A", "B")
      end

      def test_multi_label_edges_list
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        expected = [["A", "B", "x"], ["A", "B", "y"]]
        assert_equal expected, g.edges
      end

      def test_multi_label_structural_edge_count
        # Two labels on the same pair should not duplicate the structural edge.
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        assert_equal 2, g.size  # only 2 nodes
      end

      def test_same_label_different_pairs
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("C", "D", "x")
        assert g.has_edge?("A", "B", "x")
        assert g.has_edge?("C", "D", "x")
      end
    end

    # ==================================================================
    # 7. Self-loops
    # ==================================================================

    class TestLabeledGraphSelfLoops < Minitest::Test
      def test_self_loop_allowed
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        assert g.has_edge?("A", "A", "loop")
      end

      def test_self_loop_in_edges
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        assert_equal [["A", "A", "loop"]], g.edges
      end

      def test_self_loop_successor
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        assert_includes g.successors("A"), "A"
      end

      def test_self_loop_predecessor
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        assert_includes g.predecessors("A"), "A"
      end

      def test_self_loop_with_other_edges
        g = LabeledGraph.new
        g.add_edge("A", "A", "self")
        g.add_edge("A", "B", "other")
        assert g.has_edge?("A", "A", "self")
        assert g.has_edge?("A", "B", "other")
      end

      def test_self_loop_multi_label
        g = LabeledGraph.new
        g.add_edge("A", "A", "x")
        g.add_edge("A", "A", "y")
        assert_equal Set["x", "y"], g.labels("A", "A")
      end

      def test_remove_self_loop
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        g.remove_edge("A", "A", "loop")
        refute g.has_edge?("A", "A")
      end

      def test_self_loop_labels
        g = LabeledGraph.new
        g.add_edge("A", "A", "loop")
        assert_equal Set["loop"], g.labels("A", "A")
      end
    end

    # ==================================================================
    # 8. Successors and predecessors with label filtering
    # ==================================================================

    class TestLabeledGraphNeighbors < Minitest::Test
      def test_successors_without_filter
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "C", "y")
        assert_equal %w[B C], g.successors("A")
      end

      def test_successors_with_label_filter
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "C", "y")
        assert_equal ["B"], g.successors("A", label: "x")
      end

      def test_successors_label_filter_no_match
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert_equal [], g.successors("A", label: "z")
      end

      def test_predecessors_without_filter
        g = LabeledGraph.new
        g.add_edge("A", "C", "x")
        g.add_edge("B", "C", "y")
        assert_equal %w[A B], g.predecessors("C")
      end

      def test_predecessors_with_label_filter
        g = LabeledGraph.new
        g.add_edge("A", "C", "x")
        g.add_edge("B", "C", "y")
        assert_equal ["A"], g.predecessors("C", label: "x")
      end

      def test_predecessors_label_filter_no_match
        g = LabeledGraph.new
        g.add_edge("A", "C", "x")
        assert_equal [], g.predecessors("C", label: "z")
      end

      def test_successors_multi_label_filter
        # Two labels on A->B, but only "x" matches.
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        g.add_edge("A", "C", "x")
        assert_equal %w[B C], g.successors("A", label: "x")
      end

      def test_predecessors_multi_label_filter
        g = LabeledGraph.new
        g.add_edge("A", "C", "x")
        g.add_edge("A", "C", "y")
        g.add_edge("B", "C", "x")
        assert_equal %w[A B], g.predecessors("C", label: "x")
      end

      def test_self_loop_in_successors_with_filter
        g = LabeledGraph.new
        g.add_edge("A", "A", "self")
        g.add_edge("A", "B", "other")
        assert_equal ["A"], g.successors("A", label: "self")
      end

      def test_self_loop_in_predecessors_with_filter
        g = LabeledGraph.new
        g.add_edge("A", "A", "self")
        g.add_edge("B", "A", "other")
        assert_equal ["A"], g.predecessors("A", label: "self")
      end
    end

    # ==================================================================
    # 9. Algorithm delegation
    # ==================================================================

    class TestLabeledGraphAlgorithms < Minitest::Test
      def test_topological_sort_linear
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("B", "C", "y")
        assert_equal %w[A B C], g.topological_sort
      end

      def test_topological_sort_empty
        g = LabeledGraph.new
        assert_equal [], g.topological_sort
      end

      def test_has_cycle_false
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        refute g.has_cycle?
      end

      def test_has_cycle_true
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("B", "A", "y")
        assert g.has_cycle?
      end

      def test_has_cycle_with_self_loop
        g = LabeledGraph.new
        g.add_edge("A", "A", "self")
        assert g.has_cycle?
      end

      def test_transitive_closure
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("B", "C", "y")
        closure = g.transitive_closure
        assert_equal Set["B", "C"], closure["A"]
        assert_equal Set["C"], closure["B"]
        assert_equal Set.new, closure["C"]
      end

      def test_transitive_closure_empty
        g = LabeledGraph.new
        assert_equal({}, g.transitive_closure)
      end
    end

    # ==================================================================
    # 10. Error conditions
    # ==================================================================

    class TestLabeledGraphErrors < Minitest::Test
      def test_remove_missing_node_raises
        g = LabeledGraph.new
        assert_raises(NodeNotFoundError) { g.remove_node("Z") }
      end

      def test_remove_edge_missing_source_raises
        g = LabeledGraph.new
        g.add_node("B")
        assert_raises(NodeNotFoundError) { g.remove_edge("A", "B", "x") }
      end

      def test_remove_edge_missing_target_raises
        g = LabeledGraph.new
        g.add_node("A")
        assert_raises(NodeNotFoundError) { g.remove_edge("A", "B", "x") }
      end

      def test_remove_edge_missing_label_raises
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        assert_raises(EdgeNotFoundError) { g.remove_edge("A", "B", "y") }
      end

      def test_remove_edge_no_edge_raises
        g = LabeledGraph.new
        g.add_node("A")
        g.add_node("B")
        assert_raises(EdgeNotFoundError) { g.remove_edge("A", "B", "x") }
      end

      def test_labels_missing_source_raises
        g = LabeledGraph.new
        g.add_node("B")
        assert_raises(NodeNotFoundError) { g.labels("A", "B") }
      end

      def test_labels_missing_target_raises
        g = LabeledGraph.new
        g.add_node("A")
        assert_raises(NodeNotFoundError) { g.labels("A", "B") }
      end

      def test_successors_missing_node_raises
        g = LabeledGraph.new
        assert_raises(NodeNotFoundError) { g.successors("Z") }
      end

      def test_predecessors_missing_node_raises
        g = LabeledGraph.new
        assert_raises(NodeNotFoundError) { g.predecessors("Z") }
      end

      def test_successors_with_label_missing_node_raises
        g = LabeledGraph.new
        assert_raises(NodeNotFoundError) { g.successors("Z", label: "x") }
      end

      def test_predecessors_with_label_missing_node_raises
        g = LabeledGraph.new
        assert_raises(NodeNotFoundError) { g.predecessors("Z", label: "x") }
      end
    end

    # ==================================================================
    # 11. Complex graph scenarios
    # ==================================================================

    class TestLabeledGraphComplex < Minitest::Test
      def test_diamond_with_labels
        g = LabeledGraph.new
        g.add_edge("A", "B", "left")
        g.add_edge("A", "C", "right")
        g.add_edge("B", "D", "merge")
        g.add_edge("C", "D", "merge")
        assert_equal 4, g.size
        assert_equal %w[A B C D], g.topological_sort
      end

      def test_remove_node_from_middle_of_chain
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("B", "C", "y")
        g.remove_node("B")
        refute g.has_node?("B")
        assert g.has_node?("A")
        assert g.has_node?("C")
        assert_equal [], g.edges
      end

      def test_remove_hub_node
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("C", "B", "y")
        g.add_edge("B", "D", "z")
        g.remove_node("B")
        assert_equal 3, g.size
        assert_equal [], g.edges
      end

      def test_many_labels_same_pair
        g = LabeledGraph.new
        labels = (1..10).map { |i| "label_#{i}" }
        labels.each { |l| g.add_edge("A", "B", l) }
        assert_equal Set.new(labels), g.labels("A", "B")
        assert_equal 10, g.edges.size
      end

      def test_remove_all_labels_one_by_one
        g = LabeledGraph.new
        g.add_edge("A", "B", "x")
        g.add_edge("A", "B", "y")
        g.add_edge("A", "B", "z")
        g.remove_edge("A", "B", "x")
        g.remove_edge("A", "B", "y")
        assert g.has_edge?("A", "B", "z")
        g.remove_edge("A", "B", "z")
        refute g.has_edge?("A", "B")
      end
    end

    # ==================================================================
    # 12. State machine modeling (integration test)
    # ==================================================================
    #
    # A turnstile has two states: "locked" and "unlocked".
    # - In "locked" state:
    #   - "coin" input -> transitions to "unlocked"
    #   - "push" input -> stays "locked" (self-loop)
    # - In "unlocked" state:
    #   - "coin" input -> stays "unlocked" (self-loop)
    #   - "push" input -> transitions to "locked"

    class TestLabeledGraphStateMachine < Minitest::Test
      def setup
        @fsm = LabeledGraph.new
        @fsm.add_edge("locked", "unlocked", "coin")
        @fsm.add_edge("locked", "locked", "push")
        @fsm.add_edge("unlocked", "locked", "push")
        @fsm.add_edge("unlocked", "unlocked", "coin")
      end

      def test_fsm_size
        assert_equal 2, @fsm.size
      end

      def test_fsm_nodes
        assert_equal %w[locked unlocked], @fsm.nodes
      end

      def test_fsm_all_edges
        expected = [
          ["locked", "locked", "push"],
          ["locked", "unlocked", "coin"],
          ["unlocked", "locked", "push"],
          ["unlocked", "unlocked", "coin"]
        ]
        assert_equal expected, @fsm.edges
      end

      def test_fsm_coin_from_locked
        assert_equal ["unlocked"], @fsm.successors("locked", label: "coin")
      end

      def test_fsm_push_from_locked
        assert_equal ["locked"], @fsm.successors("locked", label: "push")
      end

      def test_fsm_coin_from_unlocked
        assert_equal ["unlocked"], @fsm.successors("unlocked", label: "coin")
      end

      def test_fsm_push_from_unlocked
        assert_equal ["locked"], @fsm.successors("unlocked", label: "push")
      end

      def test_fsm_labels_locked_to_unlocked
        assert_equal Set["coin"], @fsm.labels("locked", "unlocked")
      end

      def test_fsm_labels_locked_to_locked
        assert_equal Set["push"], @fsm.labels("locked", "locked")
      end

      def test_fsm_has_cycle
        assert @fsm.has_cycle?
      end

      def test_fsm_predecessors_of_locked
        assert_equal %w[locked unlocked], @fsm.predecessors("locked")
      end

      def test_fsm_predecessors_of_locked_by_push
        assert_equal %w[locked unlocked], @fsm.predecessors("locked", label: "push")
      end

      def test_fsm_predecessors_of_locked_by_coin
        assert_equal [], @fsm.predecessors("locked", label: "coin")
      end
    end
  end
end

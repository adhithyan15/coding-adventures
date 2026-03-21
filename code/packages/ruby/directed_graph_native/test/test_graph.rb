# frozen_string_literal: true

require_relative "test_helper"

# --------------------------------------------------------------------------
# test_graph.rb -- Tests for the Rust-backed DirectedGraph native extension
# --------------------------------------------------------------------------
#
# These tests mirror the pure Ruby directed_graph test suite and the Python
# directed-graph-native test suite to ensure the native extension provides
# identical behavior. If these tests pass, the native extension is a valid
# drop-in replacement for the pure Ruby implementation.
#
# We organise tests into logical groups:
#   1. Empty graph behaviour
#   2. Node operations (add, remove, has_node?, nodes)
#   3. Edge operations (add, remove, has_edge?, edges, self-loops)
#   4. Neighbor queries (predecessors, successors)
#   5. Graph properties (size, inspect, to_s)
#   6. Topological sort (Kahn's algorithm)
#   7. Cycle detection (has_cycle?)
#   8. Transitive closure
#   9. Affected nodes (incremental build detection)
#   10. Independent groups (parallel execution levels)
#   11. Real-world repo graph (integration test)
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraphNative
    # ======================================================================
    # 1. Empty Graph
    # ======================================================================

    class TestEmptyGraph < Minitest::Test
      def test_empty_graph_has_no_nodes
        g = DirectedGraph.new
        assert_equal [], g.nodes
      end

      def test_empty_graph_has_no_edges
        g = DirectedGraph.new
        assert_equal [], g.edges
      end

      def test_empty_graph_size_is_zero
        g = DirectedGraph.new
        assert_equal 0, g.size
      end

      def test_empty_graph_has_no_cycle
        g = DirectedGraph.new
        refute g.has_cycle?
      end

      def test_empty_graph_topological_sort_is_empty
        g = DirectedGraph.new
        assert_equal [], g.topological_sort
      end

      def test_empty_graph_independent_groups_is_empty
        g = DirectedGraph.new
        assert_equal [], g.independent_groups
      end
    end

    # ======================================================================
    # 2. Node Operations
    # ======================================================================

    class TestNodeOperations < Minitest::Test
      def test_add_single_node
        g = DirectedGraph.new
        g.add_node("A")
        assert_equal ["A"], g.nodes
        assert_equal 1, g.size
      end

      def test_add_duplicate_node_is_noop
        g = DirectedGraph.new
        g.add_node("A")
        g.add_node("A")
        assert_equal ["A"], g.nodes
        assert_equal 1, g.size
      end

      def test_has_node_returns_true_for_existing
        g = DirectedGraph.new
        g.add_node("X")
        assert g.has_node?("X")
      end

      def test_has_node_returns_false_for_missing
        g = DirectedGraph.new
        refute g.has_node?("X")
      end

      def test_nodes_returns_sorted_list
        g = DirectedGraph.new
        g.add_node("C")
        g.add_node("A")
        g.add_node("B")
        assert_equal %w[A B C], g.nodes
      end

      def test_remove_node_deletes_node
        g = DirectedGraph.new
        g.add_node("A")
        g.remove_node("A")
        refute g.has_node?("A")
        assert_equal 0, g.size
      end

      def test_remove_missing_node_raises_error
        g = DirectedGraph.new
        assert_raises(NodeNotFoundError) { g.remove_node("Z") }
      end

      def test_remove_node_cleans_up_outgoing_edges
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.remove_node("A")
        refute g.has_edge?("A", "B")
      end

      def test_remove_node_cleans_up_incoming_edges
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.remove_node("B")
        refute g.has_edge?("A", "B")
      end

      def test_remove_node_keeps_other_nodes
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.remove_node("B")
        assert g.has_node?("A")
        assert g.has_node?("C")
        refute g.has_edge?("A", "B")
        refute g.has_edge?("B", "C")
      end
    end

    # ======================================================================
    # 3. Edge Operations
    # ======================================================================

    class TestEdgeOperations < Minitest::Test
      def test_add_edge_creates_both_nodes
        g = DirectedGraph.new
        g.add_edge("A", "B")
        assert g.has_node?("A")
        assert g.has_node?("B")
        assert_equal 2, g.size
      end

      def test_add_edge_creates_directed_connection
        g = DirectedGraph.new
        g.add_edge("A", "B")
        assert g.has_edge?("A", "B")
        refute g.has_edge?("B", "A")
      end

      def test_add_duplicate_edge_is_noop
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "B")
        assert_equal [["A", "B"]], g.edges
      end

      def test_self_loop_raises_argument_error
        g = DirectedGraph.new
        assert_raises(ArgumentError) { g.add_edge("A", "A") }
      end

      def test_self_loop_error_message_includes_node
        g = DirectedGraph.new
        error = assert_raises(ArgumentError) { g.add_edge("A", "A") }
        assert_match(/self-loop/, error.message)
      end

      def test_remove_edge
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.remove_edge("A", "B")
        refute g.has_edge?("A", "B")
        # Both nodes should still exist after edge removal.
        assert g.has_node?("A")
        assert g.has_node?("B")
      end

      def test_remove_nonexistent_edge_raises
        g = DirectedGraph.new
        g.add_node("A")
        g.add_node("B")
        assert_raises(EdgeNotFoundError) { g.remove_edge("A", "B") }
      end

      def test_remove_edge_missing_source_raises
        g = DirectedGraph.new
        g.add_node("B")
        assert_raises { g.remove_edge("A", "B") }
      end

      def test_edges_returns_sorted_list
        g = DirectedGraph.new
        g.add_edge("B", "C")
        g.add_edge("A", "B")
        assert_equal [["A", "B"], ["B", "C"]], g.edges
      end

      def test_has_edge_returns_false_for_missing_source
        g = DirectedGraph.new
        g.add_node("B")
        refute g.has_edge?("A", "B")
      end

      def test_has_edge_returns_false_for_missing_target
        g = DirectedGraph.new
        g.add_node("A")
        refute g.has_edge?("A", "B")
      end
    end

    # ======================================================================
    # 4. Neighbor Queries
    # ======================================================================

    class TestNeighborQueries < Minitest::Test
      def test_predecessors
        g = DirectedGraph.new
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        assert_equal %w[A B], g.predecessors("C").sort
      end

      def test_successors
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        assert_equal %w[B C], g.successors("A").sort
      end

      def test_predecessors_missing_node_raises
        g = DirectedGraph.new
        assert_raises(NodeNotFoundError) { g.predecessors("Z") }
      end

      def test_successors_missing_node_raises
        g = DirectedGraph.new
        assert_raises(NodeNotFoundError) { g.successors("Z") }
      end

      def test_predecessors_of_root_is_empty
        g = DirectedGraph.new
        g.add_edge("A", "B")
        assert_equal [], g.predecessors("A")
      end

      def test_successors_of_leaf_is_empty
        g = DirectedGraph.new
        g.add_edge("A", "B")
        assert_equal [], g.successors("B")
      end
    end

    # ======================================================================
    # 5. Graph Properties
    # ======================================================================

    class TestGraphProperties < Minitest::Test
      def test_size_tracks_nodes
        g = DirectedGraph.new
        assert_equal 0, g.size
        g.add_node("A")
        assert_equal 1, g.size
        g.add_node("B")
        assert_equal 2, g.size
      end

      def test_inspect_includes_class_name
        g = DirectedGraph.new
        g.add_edge("A", "B")
        result = g.inspect
        assert_match(/DirectedGraph/, result)
        assert_match(/nodes=2/, result)
        assert_match(/edges=1/, result)
      end

      def test_to_s_returns_string
        g = DirectedGraph.new
        assert_kind_of String, g.to_s
      end
    end

    # ======================================================================
    # 6. Topological Sort
    # ======================================================================

    class TestTopologicalSort < Minitest::Test
      def test_single_node
        g = DirectedGraph.new
        g.add_node("A")
        assert_equal ["A"], g.topological_sort
      end

      def test_linear_chain
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[A B C], g.topological_sort
      end

      def test_diamond
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        result = g.topological_sort

        # A must come before B and C; B and C must come before D.
        assert_equal "A", result.first
        assert_equal "D", result.last
        assert_equal %w[B C], result[1..2].sort
      end

      def test_multiple_roots
        g = DirectedGraph.new
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        result = g.topological_sort
        assert_operator result.index("A"), :<, result.index("C")
        assert_operator result.index("B"), :<, result.index("C")
      end

      def test_disconnected_nodes
        g = DirectedGraph.new
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        # Any order is valid; just check all are present.
        assert_equal %w[A B C], g.topological_sort.sort
      end

      def test_cycle_raises_error
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert_raises(CycleError) { g.topological_sort }
      end

      def test_cycle_error_message
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        error = assert_raises(CycleError) { g.topological_sort }
        assert_match(/cycle/, error.message.downcase)
      end
    end

    # ======================================================================
    # 7. Cycle Detection
    # ======================================================================

    class TestCycleDetection < Minitest::Test
      def test_acyclic_graph
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        refute g.has_cycle?
      end

      def test_simple_cycle
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert g.has_cycle?
      end

      def test_longer_cycle
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert g.has_cycle?
      end

      def test_diamond_has_no_cycle
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        refute g.has_cycle?
      end
    end

    # ======================================================================
    # 8. Transitive Closure
    # ======================================================================

    class TestTransitiveClosure < Minitest::Test
      def test_linear_chain
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        closure = g.transitive_closure("A")
        assert_equal %w[B C], closure.sort
      end

      def test_diamond
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        closure = g.transitive_closure("A")
        assert_equal %w[B C D], closure.sort
      end

      def test_leaf_node_has_empty_closure
        g = DirectedGraph.new
        g.add_edge("A", "B")
        closure = g.transitive_closure("B")
        assert_equal [], closure
      end

      def test_single_node_has_empty_closure
        g = DirectedGraph.new
        g.add_node("A")
        closure = g.transitive_closure("A")
        assert_equal [], closure
      end

      def test_nonexistent_node_raises
        g = DirectedGraph.new
        assert_raises(NodeNotFoundError) { g.transitive_closure("X") }
      end
    end

    # ======================================================================
    # 9. Affected Nodes
    # ======================================================================

    class TestAffectedNodes < Minitest::Test
      def test_single_changed_node_with_no_dependents
        g = DirectedGraph.new
        g.add_node("A")
        assert_equal ["A"], g.affected_nodes(["A"])
      end

      def test_single_change_propagates
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[A B C], g.affected_nodes(["A"]).sort
      end

      def test_leaf_change_affects_only_itself
        g = DirectedGraph.new
        g.add_edge("A", "B")
        assert_equal ["B"], g.affected_nodes(["B"])
      end

      def test_middle_node_changed
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[B C], g.affected_nodes(["B"]).sort
      end

      def test_diamond_change_at_root
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        assert_equal %w[A B C D], g.affected_nodes(["A"]).sort
      end

      def test_multiple_changed_nodes
        g = DirectedGraph.new
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        g.add_edge("C", "D")
        assert_equal %w[A B C D], g.affected_nodes(%w[A B]).sort
      end

      def test_unknown_nodes_ignored
        g = DirectedGraph.new
        g.add_edge("A", "B")
        # "X" doesn't exist -- Rust implementation skips unknown nodes.
        affected = g.affected_nodes(["X"])
        refute_includes affected, "X"
      end

      def test_empty_changed_set
        g = DirectedGraph.new
        g.add_edge("A", "B")
        assert_equal [], g.affected_nodes([])
      end
    end

    # ======================================================================
    # 10. Independent Groups
    # ======================================================================

    class TestIndependentGroups < Minitest::Test
      def test_single_node
        g = DirectedGraph.new
        g.add_node("A")
        assert_equal [["A"]], g.independent_groups
      end

      def test_linear_chain
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal [["A"], ["B"], ["C"]], g.independent_groups
      end

      def test_diamond
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        groups = g.independent_groups
        assert_equal 3, groups.length
        assert_equal ["A"], groups[0]
        assert_equal %w[B C], groups[1].sort
        assert_equal ["D"], groups[2]
      end

      def test_parallel_roots
        g = DirectedGraph.new
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        groups = g.independent_groups
        assert_equal 1, groups.length
        assert_equal %w[A B C], groups[0].sort
      end

      def test_two_independent_chains
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("C", "D")
        groups = g.independent_groups
        assert_equal [%w[A C], %w[B D]], groups.map(&:sort)
      end

      def test_cycle_raises_error
        g = DirectedGraph.new
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert_raises(CycleError) { g.independent_groups }
      end
    end

    # ======================================================================
    # 11. Real-world Repo Graph (Integration Test)
    # ======================================================================
    #
    # This simulates a real package dependency graph similar to this
    # repository's computing stack:
    #
    #   logic_gates -> arithmetic -> cpu -> assembler -> vm -> compiler -> ...
    #
    # It tests that the algorithms work on a larger, more realistic graph.

    class TestRealRepoGraph < Minitest::Test
      def setup
        @g = DirectedGraph.new
        @g.add_edge("logic_gates", "arithmetic")
        @g.add_edge("arithmetic", "cpu")
        @g.add_edge("cpu", "assembler")
        @g.add_edge("assembler", "riscv_sim")
        @g.add_edge("assembler", "vm")
        @g.add_edge("vm", "jit")
        @g.add_edge("vm", "compiler")
        @g.add_edge("compiler", "parser")
        @g.add_edge("parser", "lexer")
      end

      def test_size
        assert_equal 10, @g.size
      end

      def test_topological_sort_valid
        sorted = @g.topological_sort
        # Every edge must go forward in the sorted order.
        @g.edges.each do |src, tgt|
          assert_operator sorted.index(src), :<, sorted.index(tgt),
                          "#{src} should come before #{tgt}"
        end
      end

      def test_no_cycle
        refute @g.has_cycle?
      end

      def test_independent_groups_first_is_root
        groups = @g.independent_groups
        assert_equal ["logic_gates"], groups.first
      end

      def test_independent_groups_last_contains_leaves
        groups = @g.independent_groups
        assert_includes groups.last, "lexer"
      end

      def test_affected_by_assembler_change
        affected = @g.affected_nodes(["assembler"]).sort
        expected = %w[assembler compiler jit lexer parser riscv_sim vm]
        assert_equal expected, affected
      end

      def test_predecessors_of_vm
        assert_equal ["assembler"], @g.predecessors("vm")
      end

      def test_successors_of_vm
        assert_equal %w[compiler jit], @g.successors("vm").sort
      end

      def test_transitive_closure_of_vm
        closure = @g.transitive_closure("vm").sort
        assert_equal %w[compiler jit lexer parser], closure
      end
    end
  end
end

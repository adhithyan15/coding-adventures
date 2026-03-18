# frozen_string_literal: true

require_relative "test_helper"

# --------------------------------------------------------------------------
# test_algorithms.rb — Tests for graph algorithms
# --------------------------------------------------------------------------
#
# This file covers the algorithmic methods on Graph:
#   1. topological_sort  (Kahn's algorithm)
#   2. has_cycle?
#   3. transitive_closure
#   4. transitive_dependents
#   5. independent_groups (parallel execution levels)
#   6. affected_nodes
#
# We use several canonical graph shapes:
#   - Linear chain:  A -> B -> C
#   - Diamond:       A -> B, A -> C, B -> D, C -> D
#   - Cycle:         A -> B -> C -> A
#   - Real repo:     a realistic package dependency graph
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    class TestTopologicalSort < Minitest::Test
      def test_empty_graph
        g = Graph.new
        assert_equal [], g.topological_sort
      end

      def test_single_node
        g = Graph.new
        g.add_node("A")
        assert_equal ["A"], g.topological_sort
      end

      def test_linear_chain
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[A B C], g.topological_sort
      end

      def test_diamond
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        result = g.topological_sort

        # A must come before B and C; B and C must come before D.
        assert_operator result.index("A"), :<, result.index("B")
        assert_operator result.index("A"), :<, result.index("C")
        assert_operator result.index("B"), :<, result.index("D")
        assert_operator result.index("C"), :<, result.index("D")
      end

      def test_multiple_roots
        g = Graph.new
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        result = g.topological_sort
        # Both A and B must come before C.
        assert_operator result.index("A"), :<, result.index("C")
        assert_operator result.index("B"), :<, result.index("C")
      end

      def test_disconnected_nodes
        g = Graph.new
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        # Any order is valid; just check all are present.
        assert_equal %w[A B C], g.topological_sort.sort
      end

      def test_cycle_raises_error
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert_raises(CycleError) { g.topological_sort }
      end

      def test_cycle_error_message
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        error = assert_raises(CycleError) { g.topological_sort }
        assert_match(/cycle/, error.message.downcase)
      end
    end

    class TestHasCycle < Minitest::Test
      def test_empty_graph_has_no_cycle
        g = Graph.new
        refute g.has_cycle?
      end

      def test_acyclic_graph
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        refute g.has_cycle?
      end

      def test_simple_cycle
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert g.has_cycle?
      end

      def test_longer_cycle
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("C", "A")
        assert g.has_cycle?
      end

      def test_diamond_has_no_cycle
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        refute g.has_cycle?
      end
    end

    class TestTransitiveClosure < Minitest::Test
      def test_empty_graph
        g = Graph.new
        assert_equal({}, g.transitive_closure)
      end

      def test_single_node
        g = Graph.new
        g.add_node("A")
        closure = g.transitive_closure
        assert_equal Set.new, closure["A"]
      end

      def test_linear_chain
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        closure = g.transitive_closure
        assert_equal Set["B", "C"], closure["A"]
        assert_equal Set["C"], closure["B"]
        assert_equal Set.new, closure["C"]
      end

      def test_diamond
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        closure = g.transitive_closure
        assert_equal Set["B", "C", "D"], closure["A"]
        assert_equal Set["D"], closure["B"]
        assert_equal Set["D"], closure["C"]
        assert_equal Set.new, closure["D"]
      end
    end

    class TestTransitiveDependents < Minitest::Test
      def test_leaf_node_has_no_dependents
        g = Graph.new
        g.add_edge("A", "B")
        assert_equal [], g.transitive_dependents("B")
      end

      def test_linear_chain
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[B C], g.transitive_dependents("A")
      end

      def test_diamond
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        assert_equal %w[B C D], g.transitive_dependents("A")
      end

      def test_missing_node_raises
        g = Graph.new
        assert_raises(NodeNotFoundError) { g.transitive_dependents("Z") }
      end
    end

    class TestIndependentGroups < Minitest::Test
      def test_empty_graph
        g = Graph.new
        assert_equal [], g.independent_groups
      end

      def test_single_node
        g = Graph.new
        g.add_node("A")
        assert_equal [["A"]], g.independent_groups
      end

      def test_linear_chain
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal [["A"], ["B"], ["C"]], g.independent_groups
      end

      def test_diamond
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        groups = g.independent_groups
        assert_equal [["A"], ["B", "C"], ["D"]], groups
      end

      def test_two_independent_chains
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("C", "D")
        groups = g.independent_groups
        assert_equal [%w[A C], %w[B D]], groups
      end

      def test_cycle_raises_error
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "A")
        assert_raises(CycleError) { g.independent_groups }
      end

      def test_disconnected_nodes_all_in_first_group
        g = Graph.new
        g.add_node("A")
        g.add_node("B")
        g.add_node("C")
        assert_equal [%w[A B C]], g.independent_groups
      end
    end

    class TestAffectedNodes < Minitest::Test
      def test_single_changed_node_with_no_dependents
        g = Graph.new
        g.add_node("A")
        assert_equal ["A"], g.affected_nodes(["A"])
      end

      def test_linear_chain_first_node_changed
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[A B C], g.affected_nodes(["A"])
      end

      def test_linear_chain_middle_node_changed
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal %w[B C], g.affected_nodes(["B"])
      end

      def test_linear_chain_last_node_changed
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("B", "C")
        assert_equal ["C"], g.affected_nodes(["C"])
      end

      def test_multiple_changed_nodes
        g = Graph.new
        g.add_edge("A", "C")
        g.add_edge("B", "C")
        g.add_edge("C", "D")
        assert_equal %w[A B C D], g.affected_nodes(%w[A B])
      end

      def test_diamond_root_changed
        g = Graph.new
        g.add_edge("A", "B")
        g.add_edge("A", "C")
        g.add_edge("B", "D")
        g.add_edge("C", "D")
        assert_equal %w[A B C D], g.affected_nodes(["A"])
      end

      def test_missing_node_raises
        g = Graph.new
        assert_raises(NodeNotFoundError) { g.affected_nodes(["Z"]) }
      end

      def test_empty_changed_set
        g = Graph.new
        g.add_edge("A", "B")
        assert_equal [], g.affected_nodes([])
      end
    end

    # ------------------------------------------------------------------
    # Real-world repo graph
    # ------------------------------------------------------------------
    #
    # This simulates a real package dependency graph similar to this
    # very repository's computing stack:
    #
    #   logic_gates -> arithmetic -> assembler -> vm -> compiler -> ...
    #
    # It tests that the algorithms work on a larger, more realistic graph.
    # ------------------------------------------------------------------

    class TestRealRepoGraph < Minitest::Test
      def setup
        @g = Graph.new
        # Layer 10: logic_gates (no deps)
        # Layer 9:  arithmetic  depends on logic_gates
        # Layer 8:  cpu         depends on arithmetic
        # Layer 7:  assembler   depends on cpu
        # Layer 6:  riscv_sim   depends on assembler
        # Layer 5:  vm          depends on assembler
        # Layer 4:  jit         depends on vm
        # Layer 3:  compiler    depends on vm
        # Layer 2:  parser      depends on compiler (conceptually, parser feeds compiler)
        # Layer 1:  lexer       depends on parser
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

      def test_independent_groups
        groups = @g.independent_groups
        # First group should be the single root: logic_gates
        assert_equal ["logic_gates"], groups.first
        # Last group should contain the leaves
        assert_includes groups.last, "lexer"
      end

      def test_affected_by_assembler_change
        affected = @g.affected_nodes(["assembler"])
        # assembler -> riscv_sim, vm -> jit, compiler -> parser -> lexer
        expected = %w[assembler compiler jit lexer parser riscv_sim vm]
        assert_equal expected, affected
      end

      def test_transitive_dependents_of_vm
        deps = @g.transitive_dependents("vm")
        assert_equal %w[compiler jit lexer parser], deps
      end

      def test_predecessors_of_vm
        assert_equal ["assembler"], @g.predecessors("vm")
      end

      def test_successors_of_vm
        assert_equal %w[compiler jit], @g.successors("vm")
      end
    end
  end
end

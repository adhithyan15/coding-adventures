# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module Graph
    class TestGraphCore < Minitest::Test
      def graph_for(repr)
        Graph.new(repr: repr)
      end

      def each_repr(&block)
        GraphRepr::ALL.each(&block)
      end

      def test_node_operations_work_in_both_representations
        each_repr do |repr|
          graph = graph_for(repr)
          graph.add_node("A")
          graph.add_node("B")

          assert graph.has_node?("A")
          assert_equal %w[A B], graph.nodes
          assert_equal 2, graph.size

          graph.remove_node("A")
          refute graph.has_node?("A")
          assert_equal ["B"], graph.nodes
        end
      end

      def test_missing_node_raises
        each_repr do |repr|
          graph = graph_for(repr)
          assert_raises(NodeNotFoundError) { graph.remove_node("Z") }
        end
      end

      def test_edge_operations_are_undirected
        each_repr do |repr|
          graph = graph_for(repr)
          graph.add_edge("A", "B", 2.5)

          assert graph.has_edge?("A", "B")
          assert graph.has_edge?("B", "A")
          assert_equal 2.5, graph.edge_weight("A", "B")
          assert_equal %w[B], graph.neighbors("A")
          assert_equal({"B" => 2.5}, graph.neighbors_weighted("A"))
          assert_equal [["A", "B", 2.5]], graph.edges
        end
      end

      def test_remove_edge_keeps_nodes
        each_repr do |repr|
          graph = graph_for(repr)
          graph.add_edge("A", "B")
          graph.remove_edge("A", "B")

          refute graph.has_edge?("A", "B")
          assert graph.has_node?("A")
          assert graph.has_node?("B")
        end
      end

      def test_self_loops_and_zero_weights_are_supported
        each_repr do |repr|
          graph = graph_for(repr)
          graph.add_edge("A", "A", 0.0)

          assert graph.has_edge?("A", "A")
          assert_equal 0.0, graph.edge_weight("A", "A")
          assert_equal ["A"], graph.neighbors("A")
          assert_equal 1, graph.degree("A")
        end
      end

      def test_edge_errors_are_specific
        each_repr do |repr|
          graph = graph_for(repr)
          graph.add_node("A")
          graph.add_node("B")

          assert_raises(EdgeNotFoundError) { graph.remove_edge("A", "B") }
          assert_raises(EdgeNotFoundError) { graph.edge_weight("A", "B") }
          assert_raises(NodeNotFoundError) { graph.neighbors("Z") }
        end
      end
    end
  end
end

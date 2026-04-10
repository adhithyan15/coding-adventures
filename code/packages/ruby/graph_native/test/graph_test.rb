# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GraphNative
    class GraphTest < Minitest::Test
      def each_repr(&block)
        GraphRepr::ALL.each(&block)
      end

      def make_graph(repr)
        graph = Graph.new(repr: repr)
        graph.add_edge("London", "Paris", 300.0)
        graph.add_edge("London", "Amsterdam", 520.0)
        graph.add_edge("Paris", "Berlin", 878.0)
        graph.add_edge("Amsterdam", "Berlin", 655.0)
        graph.add_edge("Amsterdam", "Brussels", 180.0)
        graph
      end

      def test_node_and_edge_operations
        each_repr do |repr|
          graph = Graph.new(repr: repr)
          graph.add_node("A")
          graph.add_edge("A", "B", 2.5)

          assert graph.has_node?("A")
          assert graph.has_edge?("A", "B")
          assert graph.has_edge?("B", "A")
          assert_equal 2.5, graph.edge_weight("A", "B")
          assert_equal 1, graph.degree("A")
          assert_equal ["A", "B"], graph.nodes
          assert_equal [["A", "B", 2.5]], graph.edges
        end
      end

      def test_neighbors_weighted_and_repr
        each_repr do |repr|
          graph = make_graph(repr)
          assert_equal %w[Berlin Brussels London], graph.neighbors("Amsterdam")
          assert_equal(
            { "Berlin" => 655.0, "Brussels" => 180.0, "London" => 520.0 },
            graph.neighbors_weighted("Amsterdam")
          )
          assert_match(/Graph\(nodes=5, edges=5/, graph.to_s)
        end
      end

      def test_top_level_algorithms_delegate_to_instance_methods
        each_repr do |repr|
          graph = make_graph(repr)
          assert_equal(
            ["London", "Amsterdam", "Paris", "Berlin", "Brussels"],
            CodingAdventures::GraphNative.bfs(graph, "London")
          )
          assert_equal(
            ["London", "Amsterdam", "Berlin"],
            CodingAdventures::GraphNative.shortest_path(graph, "London", "Berlin")
          )
          assert CodingAdventures::GraphNative.is_connected(graph)
        end
      end

      def test_components_cycle_and_mst
        each_repr do |repr|
          graph = Graph.new(repr: repr)
          graph.add_edge("A", "B")
          graph.add_edge("B", "C")
          graph.add_edge("C", "A")
          graph.add_edge("D", "E")

          assert CodingAdventures::GraphNative.has_cycle(graph)
          assert_includes CodingAdventures::GraphNative.connected_components(graph), %w[A B C]
          assert_includes CodingAdventures::GraphNative.connected_components(graph), %w[D E]
        end

        each_repr do |repr|
          mst = CodingAdventures::GraphNative.minimum_spanning_tree(make_graph(repr))
          assert_equal 4, mst.length
          assert_equal 1655.0, mst.sum { |_, _, weight| weight }
        end
      end

      def test_disconnected_mst_raises_custom_error
        each_repr do |repr|
          graph = Graph.new(repr: repr)
          graph.add_edge("A", "B")
          graph.add_node("C")

          assert_raises(NotConnectedError) do
            graph.minimum_spanning_tree
          end
        end
      end

      def test_missing_nodes_and_edges_raise_custom_errors
        graph = Graph.new
        assert_raises(NodeNotFoundError) { graph.neighbors("missing") }
        assert_raises(EdgeNotFoundError) { graph.remove_edge("A", "B") }
      end
    end
  end
end

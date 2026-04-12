# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module Graph
    class TestAlgorithms < Minitest::Test
      def each_repr(&block)
        GraphRepr::ALL.each(&block)
      end

      def make_city_graph(repr)
        graph = Graph.new(repr: repr)
        graph.add_edge("London", "Paris", 300.0)
        graph.add_edge("London", "Amsterdam", 520.0)
        graph.add_edge("Paris", "Berlin", 878.0)
        graph.add_edge("Amsterdam", "Berlin", 655.0)
        graph.add_edge("Amsterdam", "Brussels", 180.0)
        graph
      end

      def make_triangle(repr)
        graph = Graph.new(repr: repr)
        graph.add_edge("A", "B", 1.0)
        graph.add_edge("B", "C", 1.0)
        graph.add_edge("C", "A", 1.0)
        graph
      end

      def make_path(repr)
        graph = Graph.new(repr: repr)
        graph.add_edge("A", "B", 1.0)
        graph.add_edge("B", "C", 1.0)
        graph
      end

      def test_bfs_and_dfs_reach_expected_nodes
        each_repr do |repr|
          path = make_path(repr)
          assert_equal %w[A B C], CodingAdventures::Graph.bfs(path, "A")
          assert_equal %w[A B C], CodingAdventures::Graph.dfs(path, "A")
        end
      end

      def test_connectivity_and_components
        each_repr do |repr|
          graph = Graph.new(repr: repr)
          graph.add_edge("A", "B")
          graph.add_edge("B", "C")
          graph.add_edge("D", "E")
          graph.add_node("F")

          refute CodingAdventures::Graph.is_connected(graph)
          components = CodingAdventures::Graph.connected_components(graph)

          assert_includes components, %w[A B C]
          assert_includes components, %w[D E]
          assert_includes components, ["F"]
        end
      end

      def test_cycle_detection
        each_repr do |repr|
          assert CodingAdventures::Graph.has_cycle(make_triangle(repr))
          refute CodingAdventures::Graph.has_cycle(make_path(repr))
        end
      end

      def test_shortest_path_prefers_lower_weight
        each_repr do |repr|
          path = CodingAdventures::Graph.shortest_path(make_city_graph(repr), "London", "Berlin")
          assert_equal ["London", "Amsterdam", "Berlin"], path
        end
      end

      def test_minimum_spanning_tree
        each_repr do |repr|
          mst = CodingAdventures::Graph.minimum_spanning_tree(make_city_graph(repr))
          assert_equal 4, mst.length
          assert_includes mst, ["Amsterdam", "Brussels", 180.0]
          assert_equal 1655.0, mst.sum { |_, _, weight| weight }
        end
      end

      def test_disconnected_graph_has_no_spanning_tree
        each_repr do |repr|
          graph = Graph.new(repr: repr)
          graph.add_edge("A", "B")
          graph.add_node("C")

          assert_raises(NotConnectedError) do
            CodingAdventures::Graph.minimum_spanning_tree(graph)
          end
        end
      end
    end
  end
end

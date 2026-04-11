# frozen_string_literal: true

module CodingAdventures
  module GraphNative
    module GraphRepr
      ADJACENCY_LIST = "adjacency_list"
      ADJACENCY_MATRIX = "adjacency_matrix"
      ALL = [ADJACENCY_LIST, ADJACENCY_MATRIX].freeze
    end

    class Graph
      attr_reader :repr

      def initialize(repr: GraphRepr::ADJACENCY_LIST)
        unless GraphRepr::ALL.include?(repr)
          raise ArgumentError, "unknown graph repr: #{repr.inspect}"
        end

        @repr = repr
        @native = NativeGraph.new(repr)
      end

      def add_node(node)
        @native.add_node(node)
        self
      end

      def remove_node(node)
        @native.remove_node(node)
        self
      end

      def has_node?(node)
        @native.has_node?(node)
      end

      def nodes
        @native.nodes
      end

      def add_edge(left, right, weight = 1.0)
        @native.add_edge(left, right, weight)
        self
      end

      def remove_edge(left, right)
        @native.remove_edge(left, right)
        self
      end

      def has_edge?(left, right)
        @native.has_edge?(left, right)
      end

      def edges
        @native.edges
      end

      def edge_weight(left, right)
        @native.edge_weight(left, right)
      end

      def neighbors(node)
        @native.neighbors(node)
      end

      def neighbors_weighted(node)
        @native.neighbors_weighted_entries(node).to_h
      end

      def degree(node)
        @native.degree(node)
      end

      def bfs(start)
        @native.bfs(start)
      end

      def dfs(start)
        @native.dfs(start)
      end

      def is_connected?
        @native.is_connected?
      end

      def connected_components
        @native.connected_components
      end

      def has_cycle?
        @native.has_cycle?
      end

      def shortest_path(start, finish)
        @native.shortest_path(start, finish)
      end

      def minimum_spanning_tree
        @native.minimum_spanning_tree
      end

      def size
        @native.size
      end

      def to_s
        @native.to_s
      end

      alias inspect to_s
    end

    def self.bfs(graph, start)
      graph.bfs(start)
    end

    def self.dfs(graph, start)
      graph.dfs(start)
    end

    def self.is_connected(graph)
      graph.is_connected?
    end

    def self.connected_components(graph)
      graph.connected_components
    end

    def self.has_cycle(graph)
      graph.has_cycle?
    end

    def self.shortest_path(graph, start, finish)
      graph.shortest_path(start, finish)
    end

    def self.minimum_spanning_tree(graph)
      graph.minimum_spanning_tree
    end
  end
end

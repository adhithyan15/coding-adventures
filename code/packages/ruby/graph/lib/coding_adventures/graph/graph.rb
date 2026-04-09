# frozen_string_literal: true

module CodingAdventures
  module Graph
    # ========================================================================
    # Graph Representation Enum
    # ========================================================================
    #
    # Choose whether to use an adjacency list or adjacency matrix to represent
    # the graph internally.
    class GraphRepr
      ADJACENCY_LIST = "adjacency_list"
      ADJACENCY_MATRIX = "adjacency_matrix"
    end

    # ========================================================================
    # Custom Exception Classes
    # ========================================================================

    class GraphError < StandardError; end
    class NodeNotFoundError < GraphError; end
    class EdgeNotFoundError < GraphError; end

    # ========================================================================
    # Graph — Undirected Network of Nodes and Edges
    # ========================================================================
    #
    # A graph G = (V, E) is a pair of sets:
    #
    #   V  — vertices (nodes): strings in this Ruby implementation
    #   E  — edges: unordered pairs {u, v} — no direction, {u,v} == {v,u}
    #
    # Two Representations
    # -------------------
    # ADJACENCY_LIST (default):
    #   A Hash mapping each node to a Hash of its neighbours with edge weights.
    #   Space: O(V + E), Edge lookup: O(degree(u))
    #   Best for SPARSE graphs.
    #
    # ADJACENCY_MATRIX:
    #   A V×V boolean matrix where matrix[i][j] = true means an edge exists.
    #   Nodes are mapped to integer indices.
    #   Space: O(V²), Edge lookup: O(1)
    #   Best for DENSE graphs.
    #
    # Both representations expose the same public API.
    class Graph
      def initialize(repr = GraphRepr::ADJACENCY_LIST)
        @repr = repr

        case repr
        when GraphRepr::ADJACENCY_LIST
          # adj[u][v] = weight for every edge {u, v}
          # Both directions are stored: adj[u][v] and adj[v][u]
          @adj = Hash.new { |h, k| h[k] = {} }
        when GraphRepr::ADJACENCY_MATRIX
          # Ordered list of nodes for matrix index mapping
          @node_list = []
          # node → row/col index in @matrix
          @node_idx = {}
          # V×V matrix; false means no edge
          @matrix = []
          # weights for each edge
          @weights = Hash.new { |h, k| h[k] = {} }
        end
      end

      # ------------------------------------------------------------------
      # Node operations
      # ------------------------------------------------------------------

      # Add a node to the graph. No-op if the node already exists.
      def add_node(node)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          @adj[node] ||= {}
        when GraphRepr::ADJACENCY_MATRIX
          unless @node_idx.key?(node)
            idx = @node_list.length
            @node_list << node
            @node_idx[node] = idx
            @weights[node] = {}

            # Add a new row and column of false values
            @matrix.each { |row| row << false }
            @matrix << Array.new(idx + 1, false)
          end
        end
      end

      # Remove a node and all edges incident to it.
      # Raises NodeNotFoundError if the node does not exist.
      def remove_node(node)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          raise NodeNotFoundError, "Node not found: #{node}" unless @adj.key?(node)
          # Remove all edges that touch this node
          @adj[node].each_key { |neighbour| @adj[neighbour].delete(node) }
          @adj.delete(node)
        when GraphRepr::ADJACENCY_MATRIX
          raise NodeNotFoundError, "Node not found: #{node}" unless @node_idx.key?(node)
          idx = @node_idx.delete(node)
          @node_list.delete_at(idx)

          # Update indices for nodes that shifted down
          @node_list[idx..].each do |n|
            @node_idx[n] -= 1
          end

          # Remove the row
          @matrix.delete_at(idx)

          # Remove the column from every remaining row
          @matrix.each { |row| row.delete_at(idx) }

          @weights.delete(node)
        end
      end

      # Return true if node is in the graph.
      def has_node?(node)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          @adj.key?(node)
        when GraphRepr::ADJACENCY_MATRIX
          @node_idx.key?(node)
        end
      end

      # Return all nodes as an array.
      def nodes
        case @repr
        when GraphRepr::ADJACENCY_LIST
          @adj.keys
        when GraphRepr::ADJACENCY_MATRIX
          @node_list.dup
        end
      end

      # ------------------------------------------------------------------
      # Edge operations
      # ------------------------------------------------------------------

      # Add an undirected edge between u and v with the given weight (default 1.0).
      # Both nodes are added automatically if they do not already exist.
      # If the edge already exists its weight is updated.
      def add_edge(u, v, weight = 1.0)
        add_node(u)
        add_node(v)

        case @repr
        when GraphRepr::ADJACENCY_LIST
          @adj[u][v] = weight
          @adj[v][u] = weight
        when GraphRepr::ADJACENCY_MATRIX
          i, j = @node_idx[u], @node_idx[v]
          @matrix[i][j] = true
          @matrix[j][i] = true
          @weights[u][v] = weight
          @weights[v][u] = weight
        end
      end

      # Remove the edge between u and v.
      # Raises NodeNotFoundError if either node doesn't exist.
      # Raises EdgeNotFoundError if the edge doesn't exist.
      def remove_edge(u, v)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          raise NodeNotFoundError, "Node not found: #{u}" unless @adj.key?(u)
          raise NodeNotFoundError, "Node not found: #{v}" unless @adj.key?(v)
          raise EdgeNotFoundError, "Edge not found: (#{u}, #{v})" unless @adj[u].key?(v)
          @adj[u].delete(v)
          @adj[v].delete(u)
        when GraphRepr::ADJACENCY_MATRIX
          raise NodeNotFoundError, "Node not found: #{u}" unless @node_idx.key?(u)
          raise NodeNotFoundError, "Node not found: #{v}" unless @node_idx.key?(v)
          i, j = @node_idx[u], @node_idx[v]
          raise EdgeNotFoundError, "Edge not found: (#{u}, #{v})" unless @matrix[i][j]
          @matrix[i][j] = false
          @matrix[j][i] = false
          @weights[u].delete(v)
          @weights[v].delete(u)
        end
      end

      # Return true if an edge exists between u and v.
      def has_edge?(u, v)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          @adj.key?(u) && @adj[u].key?(v)
        when GraphRepr::ADJACENCY_MATRIX
          return false unless @node_idx.key?(u) && @node_idx.key?(v)
          i, j = @node_idx[u], @node_idx[v]
          @matrix[i][j]
        end
      end

      # Return all edges as an array of [u, v, weight] triples.
      # Each undirected edge appears exactly once.
      def edges
        result = []

        case @repr
        when GraphRepr::ADJACENCY_LIST
          seen = Set.new
          @adj.each do |u, neighbours|
            neighbours.each do |v, w|
              key = [u, v].sort.join(",")
              unless seen.include?(key)
                a, b = u <= v ? [u, v] : [v, u]
                result << [a, b, w]
                seen << key
              end
            end
          end
        when GraphRepr::ADJACENCY_MATRIX
          n = @node_list.length
          (0...n).each do |i|
            (i + 1...n).each do |j|
              if @matrix[i][j]
                u, v = @node_list[i], @node_list[j]
                w = @weights[u][v]
                result << [u, v, w]
              end
            end
          end
        end

        result
      end

      # Return the weight of edge (u, v).
      # Raises NodeNotFoundError if either node doesn't exist.
      # Raises EdgeNotFoundError if the edge doesn't exist.
      def edge_weight(u, v)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          raise NodeNotFoundError, "Node not found: #{u}" unless @adj.key?(u)
          raise NodeNotFoundError, "Node not found: #{v}" unless @adj.key?(v)
          raise EdgeNotFoundError, "Edge not found: (#{u}, #{v})" unless @adj[u].key?(v)
          @adj[u][v]
        when GraphRepr::ADJACENCY_MATRIX
          raise NodeNotFoundError, "Node not found: #{u}" unless @node_idx.key?(u)
          raise NodeNotFoundError, "Node not found: #{v}" unless @node_idx.key?(v)
          i, j = @node_idx[u], @node_idx[v]
          raise EdgeNotFoundError, "Edge not found: (#{u}, #{v})" unless @matrix[i][j]
          @weights[u][v]
        end
      end

      # ------------------------------------------------------------------
      # Neighbourhood queries
      # ------------------------------------------------------------------

      # Return all neighbours of node.
      # Raises NodeNotFoundError if the node does not exist.
      def neighbors(node)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          raise NodeNotFoundError, "Node not found: #{node}" unless @adj.key?(node)
          @adj[node].keys
        when GraphRepr::ADJACENCY_MATRIX
          raise NodeNotFoundError, "Node not found: #{node}" unless @node_idx.key?(node)
          idx = @node_idx[node]
          result = []
          @matrix[idx].each_with_index do |has_edge, j|
            result << @node_list[j] if has_edge
          end
          result
        end
      end

      # Return {neighbour => weight} for all neighbours of node.
      # Raises NodeNotFoundError if the node does not exist.
      def neighbors_weighted(node)
        case @repr
        when GraphRepr::ADJACENCY_LIST
          raise NodeNotFoundError, "Node not found: #{node}" unless @adj.key?(node)
          @adj[node].dup
        when GraphRepr::ADJACENCY_MATRIX
          raise NodeNotFoundError, "Node not found: #{node}" unless @node_idx.key?(node)
          idx = @node_idx[node]
          result = {}
          @matrix[idx].each_with_index do |has_edge, j|
            if has_edge
              neighbour = @node_list[j]
              result[neighbour] = @weights[node][neighbour]
            end
          end
          result
        end
      end

      # Return the degree of node (number of incident edges).
      # Raises NodeNotFoundError if the node does not exist.
      def degree(node)
        neighbors(node).length
      end

      # ------------------------------------------------------------------
      # Utility methods
      # ------------------------------------------------------------------

      # Return the number of nodes in the graph.
      def length
        case @repr
        when GraphRepr::ADJACENCY_LIST
          @adj.length
        when GraphRepr::ADJACENCY_MATRIX
          @node_list.length
        end
      end

      alias_method :size, :length

      # String representation of the graph.
      def to_s
        "Graph(nodes=#{length}, edges=#{edges.length}, repr=#{@repr})"
      end

      alias_method :inspect, :to_s
    end
  end
end

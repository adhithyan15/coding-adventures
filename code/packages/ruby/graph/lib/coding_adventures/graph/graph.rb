# frozen_string_literal: true

require "set"

module CodingAdventures
  module Graph
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
        @adj = {}
        @node_list = []
        @node_index = {}
        @matrix = []
        @graph_properties = {}
        @node_properties = {}
        @edge_properties = {}
      end

      def add_node(node, properties = {})
        if @repr == GraphRepr::ADJACENCY_LIST
          @adj[node] ||= {}
        elsif !@node_index.key?(node)
          index = @node_list.length
          @node_list << node
          @node_index[node] = index
          @matrix.each { |row| row << nil }
          @matrix << Array.new(index + 1)
        end
        @node_properties[node] ||= {}
        @node_properties[node].merge!(properties)
        self
      end

      def remove_node(node)
        raise NodeNotFoundError, "Node not found: #{node.inspect}" unless has_node?(node)

        if @repr == GraphRepr::ADJACENCY_LIST
          @adj[node].keys.each do |neighbor|
            @adj[neighbor]&.delete(node)
            @edge_properties.delete(edge_key(node, neighbor))
          end
          @adj.delete(node)
        else
          @node_list.each { |other| @edge_properties.delete(edge_key(node, other)) }
          index = @node_index.delete(node)
          @node_list.delete_at(index)
          @matrix.delete_at(index)
          @matrix.each { |row| row.delete_at(index) }
          @node_list.each_with_index { |name, offset| @node_index[name] = offset }
        end
        @node_properties.delete(node)

        self
      end

      def has_node?(node)
        if @repr == GraphRepr::ADJACENCY_LIST
          @adj.key?(node)
        else
          @node_index.key?(node)
        end
      end

      def nodes
        sort_nodes(@repr == GraphRepr::ADJACENCY_LIST ? @adj.keys : @node_list)
      end

      def add_edge(left, right, weight = 1.0, properties = {})
        add_node(left)
        add_node(right)

        if @repr == GraphRepr::ADJACENCY_LIST
          @adj[left][right] = weight
          @adj[right][left] = weight
        else
          left_index = @node_index.fetch(left)
          right_index = @node_index.fetch(right)
          @matrix[left_index][right_index] = weight
          @matrix[right_index][left_index] = weight
        end
        @edge_properties[edge_key(left, right)] ||= {}
        @edge_properties[edge_key(left, right)].merge!(properties)
        @edge_properties[edge_key(left, right)]["weight"] = weight

        self
      end

      def remove_edge(left, right)
        raise EdgeNotFoundError, "Edge not found: #{left.inspect} -- #{right.inspect}" unless has_edge?(left, right)

        if @repr == GraphRepr::ADJACENCY_LIST
          @adj[left].delete(right)
          @adj[right].delete(left)
        else
          left_index = @node_index.fetch(left)
          right_index = @node_index.fetch(right)
          @matrix[left_index][right_index] = nil
          @matrix[right_index][left_index] = nil
        end
        @edge_properties.delete(edge_key(left, right))

        self
      end

      def has_edge?(left, right)
        if @repr == GraphRepr::ADJACENCY_LIST
          @adj.key?(left) && @adj[left].key?(right)
        else
          return false unless @node_index.key?(left) && @node_index.key?(right)

          left_index = @node_index.fetch(left)
          right_index = @node_index.fetch(right)
          !@matrix[left_index][right_index].nil?
        end
      end

      def edges
        result = []

        if @repr == GraphRepr::ADJACENCY_LIST
          seen = Set.new
          @adj.each do |left, neighbors|
            neighbors.each do |right, weight|
              first, second = canonical_endpoints(left, right)
              key = [first, second]
              next if seen.include?(key)

              seen.add(key)
              result << [first, second, weight]
            end
          end
        else
          (0...@node_list.length).each do |row|
            (row...@node_list.length).each do |col|
              weight = @matrix[row][col]
              next if weight.nil?

              result << [@node_list[row], @node_list[col], weight]
            end
          end
        end

        result.sort_by { |left, right, weight| [node_key(left), node_key(right), weight] }
      end

      def edge_weight(left, right)
        if @repr == GraphRepr::ADJACENCY_LIST
          weight = @adj.fetch(left) { raise EdgeNotFoundError, "Edge not found: #{left.inspect} -- #{right.inspect}" }[right]
          return weight unless weight.nil?
        else
          if @node_index.key?(left) && @node_index.key?(right)
            weight = @matrix[@node_index.fetch(left)][@node_index.fetch(right)]
            return weight unless weight.nil?
          end
        end

        raise EdgeNotFoundError, "Edge not found: #{left.inspect} -- #{right.inspect}"
      end

      def graph_properties
        @graph_properties.dup
      end

      def set_graph_property(key, value)
        @graph_properties[key] = value
        self
      end

      def remove_graph_property(key)
        @graph_properties.delete(key)
        self
      end

      def node_properties(node)
        raise NodeNotFoundError, "Node not found: #{node.inspect}" unless has_node?(node)

        @node_properties.fetch(node, {}).dup
      end

      def set_node_property(node, key, value)
        raise NodeNotFoundError, "Node not found: #{node.inspect}" unless has_node?(node)

        @node_properties[node] ||= {}
        @node_properties[node][key] = value
        self
      end

      def remove_node_property(node, key)
        raise NodeNotFoundError, "Node not found: #{node.inspect}" unless has_node?(node)

        @node_properties.fetch(node, {}).delete(key)
        self
      end

      def edge_properties(left, right)
        raise EdgeNotFoundError, "Edge not found: #{left.inspect} -- #{right.inspect}" unless has_edge?(left, right)

        @edge_properties.fetch(edge_key(left, right), {}).merge("weight" => edge_weight(left, right))
      end

      def set_edge_property(left, right, key, value)
        raise EdgeNotFoundError, "Edge not found: #{left.inspect} -- #{right.inspect}" unless has_edge?(left, right)

        if key == "weight"
          raise ArgumentError, "edge property 'weight' must be numeric" unless value.is_a?(Numeric)

          set_edge_weight(left, right, value)
        end
        @edge_properties[edge_key(left, right)] ||= {}
        @edge_properties[edge_key(left, right)][key] = value
        self
      end

      def remove_edge_property(left, right, key)
        raise EdgeNotFoundError, "Edge not found: #{left.inspect} -- #{right.inspect}" unless has_edge?(left, right)

        if key == "weight"
          set_edge_weight(left, right, 1.0)
          @edge_properties[edge_key(left, right)] ||= {}
          @edge_properties[edge_key(left, right)]["weight"] = 1.0
        else
          @edge_properties.fetch(edge_key(left, right), {}).delete(key)
        end
        self
      end

      def neighbors(node)
        raise NodeNotFoundError, "Node not found: #{node.inspect}" unless has_node?(node)

        if @repr == GraphRepr::ADJACENCY_LIST
          sort_nodes(@adj.fetch(node).keys)
        else
          index = @node_index.fetch(node)
          result = []
          @matrix[index].each_with_index do |weight, column|
            result << @node_list[column] unless weight.nil?
          end
          sort_nodes(result)
        end
      end

      def neighbors_weighted(node)
        raise NodeNotFoundError, "Node not found: #{node.inspect}" unless has_node?(node)

        result = if @repr == GraphRepr::ADJACENCY_LIST
                   @adj.fetch(node).dup
                 else
                   index = @node_index.fetch(node)
                   values = {}
                   @matrix[index].each_with_index do |weight, column|
                     values[@node_list[column]] = weight unless weight.nil?
                   end
                   values
                 end

        result.sort_by { |neighbor, _| node_key(neighbor) }.to_h
      end

      def degree(node)
        neighbors(node).length
      end

      def size
        @repr == GraphRepr::ADJACENCY_LIST ? @adj.length : @node_list.length
      end

      def to_s
        "Graph(nodes=#{size}, edges=#{edges.length}, repr=#{@repr})"
      end

      alias inspect to_s

      private

      def sort_nodes(nodes)
        nodes.sort_by { |node| node_key(node) }
      end

      def node_key(node)
        "#{node.class}:#{node.inspect}"
      end

      def canonical_endpoints(left, right)
        node_key(left) <= node_key(right) ? [left, right] : [right, left]
      end

      def edge_key(left, right)
        canonical_endpoints(left, right)
      end

      def set_edge_weight(left, right, weight)
        if @repr == GraphRepr::ADJACENCY_LIST
          @adj[left][right] = weight
          @adj[right][left] = weight
        else
          left_index = @node_index.fetch(left)
          right_index = @node_index.fetch(right)
          @matrix[left_index][right_index] = weight
          @matrix[right_index][left_index] = weight
        end
      end
    end

    def self.bfs(graph, start)
      raise NodeNotFoundError, "Node not found: #{start.inspect}" unless graph.has_node?(start)

      visited = Set.new([start])
      queue = [start]
      result = []

      until queue.empty?
        node = queue.shift
        result << node
        graph.neighbors(node).each do |neighbor|
          next if visited.include?(neighbor)

          visited.add(neighbor)
          queue << neighbor
        end
      end

      result
    end

    def self.dfs(graph, start)
      raise NodeNotFoundError, "Node not found: #{start.inspect}" unless graph.has_node?(start)

      visited = Set.new
      stack = [start]
      result = []

      until stack.empty?
        node = stack.pop
        next if visited.include?(node)

        visited.add(node)
        result << node
        graph.neighbors(node).reverse_each do |neighbor|
          stack << neighbor unless visited.include?(neighbor)
        end
      end

      result
    end

    def self.is_connected(graph)
      return true if graph.size.zero?

      bfs(graph, graph.nodes.first).length == graph.size
    end

    def self.connected_components(graph)
      remaining = Set.new(graph.nodes)
      components = []

      until remaining.empty?
        start = remaining.first
        component = bfs(graph, start)
        components << component
        component.each { |node| remaining.delete(node) }
      end

      components
    end

    def self.has_cycle(graph)
      visited = Set.new

      graph.nodes.each do |start|
        next if visited.include?(start)

        stack = [[start, nil]]
        until stack.empty?
          node, parent = stack.pop
          next if visited.include?(node)

          visited.add(node)
          graph.neighbors(node).each do |neighbor|
            if !visited.include?(neighbor)
              stack << [neighbor, node]
            elsif neighbor != parent
              return true
            end
          end
        end
      end

      false
    end

    def self.shortest_path(graph, start, finish)
      return [] unless graph.has_node?(start) && graph.has_node?(finish)
      return [start] if start == finish

      all_unit = graph.edges.all? { |_, _, weight| weight == 1.0 }
      all_unit ? bfs_shortest_path(graph, start, finish) : dijkstra_shortest_path(graph, start, finish)
    end

    def self.minimum_spanning_tree(graph)
      all_nodes = graph.nodes
      return [] if all_nodes.empty? || graph.edges.empty?
      raise NotConnectedError, "graph is not connected" unless is_connected(graph)

      union_find = UnionFind.new(all_nodes)
      result = []

      graph.edges.sort_by { |left, right, weight| [weight, left.inspect, right.inspect] }.each do |left, right, weight|
        next if union_find.find(left) == union_find.find(right)

        union_find.union(left, right)
        result << [left, right, weight]
        break if result.length == all_nodes.length - 1
      end

      result
    end

    class << self
      private

      def bfs_shortest_path(graph, start, finish)
        parent = { start => nil }
        queue = [start]

        until queue.empty?
          node = queue.shift
          break if node == finish

          graph.neighbors(node).each do |neighbor|
            next if parent.key?(neighbor)

            parent[neighbor] = node
            queue << neighbor
          end
        end

        return [] unless parent.key?(finish)

        path = []
        current = finish
        until current.nil?
          path << current
          current = parent[current]
        end
        path.reverse
      end

      def dijkstra_shortest_path(graph, start, finish)
        distances = graph.nodes.to_h { |node| [node, Float::INFINITY] }
        parents = {}
        distances[start] = 0.0
        queue = [[0.0, 0, start]]
        sequence = 0

        until queue.empty?
          queue.sort_by! { |distance, order, _| [distance, order] }
          distance, _, node = queue.shift
          next if distance > distances.fetch(node)
          break if node == finish

          graph.neighbors_weighted(node).each do |neighbor, weight|
            next_distance = distance + weight
            next unless next_distance < distances.fetch(neighbor, Float::INFINITY)

            distances[neighbor] = next_distance
            parents[neighbor] = node
            sequence += 1
            queue << [next_distance, sequence, neighbor]
          end
        end

        return [] if distances.fetch(finish, Float::INFINITY).infinite?

        path = []
        current = finish
        until current.nil?
          path << current
          current = parents[current]
        end
        path.reverse
      end
    end

    class UnionFind
      def initialize(nodes)
        @parent = {}
        @rank = {}
        nodes.each do |node|
          @parent[node] = node
          @rank[node] = 0
        end
      end

      def find(node)
        parent = @parent.fetch(node)
        if parent != node
          @parent[node] = find(parent)
        end
        @parent[node]
      end

      def union(left, right)
        left_root = find(left)
        right_root = find(right)
        return if left_root == right_root

        left_rank = @rank.fetch(left_root)
        right_rank = @rank.fetch(right_root)
        if left_rank < right_rank
          left_root, right_root = right_root, left_root
          left_rank, right_rank = right_rank, left_rank
        end

        @parent[right_root] = left_root
        @rank[left_root] += 1 if left_rank == right_rank
      end
    end
  end
end

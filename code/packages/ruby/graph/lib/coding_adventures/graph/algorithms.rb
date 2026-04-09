# frozen_string_literal: true

module CodingAdventures
  module Graph
    # ========================================================================
    # Pure Graph Algorithms
    # ========================================================================
    #
    # All functions here are pure — they take a Graph as input and return a result.
    # They never mutate the graph.
    #
    # Algorithms provided:
    #   bfs                   — breadth-first traversal
    #   dfs                   — depth-first traversal
    #   is_connected          — does every node reach every other?
    #   connected_components  — find all isolated clusters
    #   has_cycle             — does the graph contain a cycle?
    #   shortest_path         — fewest-hops or lowest-weight path
    #   minimum_spanning_tree — cheapest set of edges connecting all nodes

    # ========================================================================
    # BFS — Breadth-First Search
    # ========================================================================

    def self.bfs(graph, start)
      # Return nodes reachable from start in breadth-first order.
      visited = Set.new([start])
      queue = [start]
      result = []

      while queue.any?
        node = queue.shift
        result << node
        # Sort neighbours for deterministic output
        graph.neighbors(node).sort.each do |neighbour|
          next if visited.include?(neighbour)
          visited << neighbour
          queue << neighbour
        end
      end

      result
    end

    # ========================================================================
    # DFS — Depth-First Search
    # ========================================================================

    def self.dfs(graph, start)
      # Return nodes reachable from start in depth-first order.
      visited = Set.new
      stack = [start]
      result = []

      while stack.any?
        node = stack.pop
        next if visited.include?(node)

        visited << node
        result << node

        # Reverse-sort so that when we push all neighbours the first (alphabetically)
        # is on top — this makes output deterministic
        graph.neighbors(node).sort.reverse.each do |neighbour|
          next if visited.include?(neighbour)
          stack.push(neighbour)
        end
      end

      result
    end

    # ========================================================================
    # is_connected
    # ========================================================================

    def self.is_connected?(graph)
      # Return true if every node can reach every other node.
      nodes = graph.nodes
      return true if nodes.empty?
      bfs(graph, nodes.first).length == nodes.length
    end

    # ========================================================================
    # connected_components
    # ========================================================================

    def self.connected_components(graph)
      # Return a list of connected components, each as a Set of nodes.
      unvisited = Set.new(graph.nodes)
      components = []

      while unvisited.any?
        start = unvisited.first
        component = Set.new(bfs(graph, start))
        components << component
        unvisited.subtract(component)
      end

      components
    end

    # ========================================================================
    # has_cycle
    # ========================================================================

    def self.has_cycle?(graph)
      # Return true if the graph contains any cycle.
      visited = Set.new

      graph.nodes.each do |start|
        next if visited.include?(start)

        # Stack holds [node, parent] pairs
        stack = [[start, nil]]

        while stack.any?
          node, par = stack.pop

          next if visited.include?(node)

          visited << node

          graph.neighbors(node).each do |neighbour|
            if !visited.include?(neighbour)
              stack.push([neighbour, node])
            elsif neighbour != par
              # Back edge: visited neighbour that isn't our parent → cycle
              return true
            end
          end
        end
      end

      false
    end

    # ========================================================================
    # shortest_path
    # ========================================================================

    def self.shortest_path(graph, start, end_node)
      # Return the shortest (lowest-weight) path from start to end.
      return [start] if start == end_node && graph.has_node?(start)
      return [] if start == end_node

      # Decide strategy: BFS if all weights are 1.0, else Dijkstra
      all_unit = graph.edges.all? { |_, _, w| w == 1.0 }

      if all_unit
        bfs_path(graph, start, end_node)
      else
        dijkstra(graph, start, end_node)
      end
    end

    private

    def self.bfs_path(graph, start, end_node)
      # BFS shortest path (for unweighted graphs)
      parent = { start => nil }
      queue = [start]

      while queue.any?
        node = queue.shift
        break if node == end_node

        graph.neighbors(node).each do |neighbour|
          unless parent.key?(neighbour)
            parent[neighbour] = node
            queue << neighbour
          end
        end
      end

      return [] unless parent.key?(end_node)

      # Trace back from end to start via parent pointers
      path = []
      cur = end_node
      while cur
        path.unshift(cur)
        cur = parent[cur]
      end
      path
    end

    def self.dijkstra(graph, start, end_node)
      # Dijkstra's algorithm for weighted shortest path
      inf = Float::INFINITY
      dist = {}
      parent = {}

      graph.nodes.each { |node| dist[node] = inf }
      dist[start] = 0.0

      # Min-heap entries: [distance, node]
      heap = [[0.0, start]]

      while heap.any?
        heap.sort_by!(&:first)
        d, node = heap.shift

        next if d > dist[node]
        break if node == end_node

        graph.neighbors_weighted(node).each do |neighbour, weight|
          new_dist = dist[node] + weight
          if new_dist < dist[neighbour]
            dist[neighbour] = new_dist
            parent[neighbour] = node
            heap << [new_dist, neighbour]
          end
        end
      end

      return [] if dist[end_node] == inf

      # Trace back
      path = []
      cur = end_node
      while cur
        path.unshift(cur)
        cur = parent[cur]
      end
      path
    end

    # ========================================================================
    # minimum_spanning_tree
    # ========================================================================

    public

    def self.minimum_spanning_tree(graph)
      # Return the minimum spanning tree as an array of [u, v, weight] triples.
      all_nodes = graph.nodes
      return [] if all_nodes.empty?

      # Sort edges by weight
      sorted_edges = graph.edges.sort_by { |_, _, w| w }

      uf = UnionFind.new(all_nodes)
      mst = []

      sorted_edges.each do |u, v, w|
        if uf.find(u) != uf.find(v)
          uf.union(u, v)
          mst << [u, v, w]
          break if mst.length == all_nodes.length - 1
        end
      end

      if mst.length < all_nodes.length - 1 && all_nodes.length > 1
        raise GraphError, "Graph is not connected — no spanning tree exists"
      end

      mst
    end

    # ========================================================================
    # Union-Find (helper for Kruskal's algorithm)
    # ========================================================================

    private

    class UnionFind
      def initialize(nodes)
        @parent = {}
        @rank = {}
        nodes.each do |node|
          @parent[node] = node
          @rank[node] = 0
        end
      end

      def find(x)
        # Return the representative (root) of x's component
        if @parent[x] != x
          @parent[x] = find(@parent[x]) # path compression
        end
        @parent[x]
      end

      def union(a, b)
        # Merge the components of a and b (union by rank)
        ra = find(a)
        rb = find(b)
        return if ra == rb

        # Attach the shorter tree under the taller tree
        ra, rb = rb, ra if @rank[ra] < @rank[rb]

        @parent[rb] = ra
        @rank[ra] += 1 if @rank[ra] == @rank[rb]
      end
    end
  end
end

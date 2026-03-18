# frozen_string_literal: true

require "set"

# --------------------------------------------------------------------------
# graph.rb — The directed-graph data structure and its algorithms
# --------------------------------------------------------------------------
#
# A directed graph (or "digraph") is a set of *nodes* connected by
# *edges*, where every edge has a direction: it goes FROM one node TO
# another.  Think of it like one-way streets on a city map.
#
# == Internal representation
#
# We store the graph as two parallel adjacency hashes:
#
#   @forward  — { node => Set of successor nodes }
#   @reverse  — { node => Set of predecessor nodes }
#
# Having both directions lets us answer "who depends on X?" and "what does
# X depend on?" in O(1) time, which is essential for algorithms like
# topological sort and transitive-closure computation.
#
# == Why Set instead of Array?
#
# Sets give us O(1) membership tests and automatic deduplication.  We never
# want duplicate edges between the same pair of nodes.
#
# == Thread safety
#
# This class is *not* thread-safe.  If you need concurrent access, wrap
# calls in a Mutex or use a concurrent data structure.
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    class Graph
      # ------------------------------------------------------------------
      # Construction
      # ------------------------------------------------------------------

      # Creates a new, empty directed graph.
      #
      # The two hashes use `Hash.new { |h, k| h[k] = Set.new }` so that
      # accessing a missing key automatically creates an empty Set for it.
      # This eliminates nil-checks throughout the code.
      def initialize
        @forward = {}
        @reverse = {}
      end

      # ------------------------------------------------------------------
      # Core mutators
      # ------------------------------------------------------------------

      # Adds a node to the graph.  If the node already exists, this is a
      # no-op — we simply return self for chaining.
      #
      #   graph.add_node("A").add_node("B")
      #
      # Nodes can be any object that responds to `eql?` and `hash` (i.e.
      # anything usable as a Hash key).  Strings, symbols, and integers
      # are the most common choices.
      def add_node(node)
        @forward[node] ||= Set.new
        @reverse[node] ||= Set.new
        self
      end

      # Adds a directed edge from +source+ to +target+.
      #
      # Both nodes are implicitly added if they are not already present,
      # just like `git` creates branches on first commit.
      #
      # Self-loops (source == target) are rejected with a CycleError
      # because they are always cycles of length 1 and break topological
      # sort.
      #
      # Duplicate edges are silently ignored (Set semantics).
      #
      # Raises CycleError if source == target (self-loop).
      def add_edge(source, target)
        if source == target
          raise CycleError, "Self-loop detected: #{source.inspect} -> #{target.inspect}"
        end

        add_node(source)
        add_node(target)
        @forward[source].add(target)
        @reverse[target].add(source)
        self
      end

      # Removes a node and all edges that touch it (both incoming and
      # outgoing).
      #
      # Raises NodeNotFoundError if the node is not in the graph.
      def remove_node(node)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        # Remove all outgoing edges: for each successor, delete `node`
        # from their reverse (predecessor) set.
        @forward[node].each { |succ| @reverse[succ].delete(node) }

        # Remove all incoming edges: for each predecessor, delete `node`
        # from their forward (successor) set.
        @reverse[node].each { |pred| @forward[pred].delete(node) }

        # Finally, remove the node itself from both hashes.
        @forward.delete(node)
        @reverse.delete(node)
        self
      end

      # Removes a single directed edge from +source+ to +target+.
      #
      # Raises NodeNotFoundError if either endpoint is missing.
      # Raises EdgeNotFoundError if the edge does not exist.
      def remove_edge(source, target)
        unless has_node?(source)
          raise NodeNotFoundError, "Node not found: #{source.inspect}"
        end
        unless has_node?(target)
          raise NodeNotFoundError, "Node not found: #{target.inspect}"
        end
        unless has_edge?(source, target)
          raise EdgeNotFoundError,
                "Edge not found: #{source.inspect} -> #{target.inspect}"
        end

        @forward[source].delete(target)
        @reverse[target].delete(source)
        self
      end

      # ------------------------------------------------------------------
      # Core queries
      # ------------------------------------------------------------------

      # Returns true if the node is in the graph.
      def has_node?(node)
        @forward.key?(node)
      end

      # Returns true if there is a directed edge from +source+ to +target+.
      def has_edge?(source, target)
        @forward.key?(source) && @forward[source].include?(target)
      end

      # Returns a sorted array of all nodes in the graph.
      #
      # We sort so that test assertions are deterministic regardless of
      # Hash insertion order.  For non-comparable nodes, use `nodes_unsorted`.
      def nodes
        @forward.keys.sort
      end

      # Returns an array of all edges as [source, target] pairs, sorted
      # for deterministic output.
      def edges
        result = []
        @forward.each do |source, targets|
          targets.each { |target| result << [source, target] }
        end
        result.sort
      end

      # Returns a sorted array of nodes that have edges pointing TO the
      # given node (i.e. its "parents" or "dependencies").
      #
      # Raises NodeNotFoundError if the node is not in the graph.
      def predecessors(node)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        @reverse[node].to_a.sort
      end

      # Returns a sorted array of nodes that the given node has edges
      # pointing TO (i.e. its "children" or "dependents").
      #
      # Raises NodeNotFoundError if the node is not in the graph.
      def successors(node)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        @forward[node].to_a.sort
      end

      # Returns the number of nodes in the graph.
      def size
        @forward.size
      end

      # ------------------------------------------------------------------
      # Algorithms
      # ------------------------------------------------------------------

      # == Topological sort (Kahn's algorithm)
      #
      # A topological sort arranges nodes so that for every edge A -> B,
      # A appears before B.  Think of it as a valid build order: you must
      # compile dependencies before the things that depend on them.
      #
      # Kahn's algorithm works by repeatedly removing nodes with no
      # incoming edges (in-degree zero).  If we run out of such nodes
      # before processing every node, the graph contains a cycle.
      #
      # Time complexity:  O(V + E)
      # Space complexity: O(V)
      #
      # Raises CycleError if the graph contains a cycle.
      def topological_sort
        # Step 1: compute in-degrees for every node.
        in_degree = {}
        @forward.each_key { |node| in_degree[node] = 0 }
        @forward.each_value do |targets|
          targets.each { |t| in_degree[t] += 1 }
        end

        # Step 2: seed the queue with all zero-in-degree nodes.
        # We sort the initial queue so the output is deterministic when
        # there are multiple valid orderings.
        queue = in_degree.select { |_, deg| deg.zero? }.keys.sort

        # Step 3: process the queue.
        result = []
        until queue.empty?
          node = queue.shift
          result << node

          # "Removing" a node means decrementing the in-degree of each
          # successor.  When a successor reaches zero, it joins the queue.
          @forward[node].to_a.sort.each do |succ|
            in_degree[succ] -= 1
            queue << succ if in_degree[succ].zero?
          end
        end

        # Step 4: if we didn't process every node, there's a cycle.
        if result.size != @forward.size
          raise CycleError, "Graph contains a cycle — topological sort is impossible"
        end

        result
      end

      # Returns true if the graph contains at least one cycle.
      #
      # We detect cycles by attempting a topological sort.  If it raises
      # CycleError, we know there's a cycle.  This is a clean, simple
      # approach — we reuse the well-tested topological_sort rather than
      # writing a separate DFS-based cycle detector.
      def has_cycle?
        topological_sort
        false
      rescue CycleError
        true
      end

      # == Transitive closure
      #
      # The transitive closure of a graph answers the question: "Can I
      # reach node B from node A, following any number of edges?"
      #
      # We compute it by running a BFS/DFS from every node and collecting
      # all reachable nodes.  The result is a Hash mapping each node to
      # a Set of all nodes reachable from it.
      #
      # Example: if A -> B -> C, then:
      #   transitive_closure[A] = Set[B, C]
      #   transitive_closure[B] = Set[C]
      #   transitive_closure[C] = Set[]
      #
      # Time complexity: O(V * (V + E))  — a BFS for each node.
      def transitive_closure
        closure = {}
        @forward.each_key do |start_node|
          reachable = Set.new
          stack = @forward[start_node].to_a
          until stack.empty?
            current = stack.pop
            next if reachable.include?(current)

            reachable.add(current)
            stack.concat(@forward[current].to_a)
          end
          closure[start_node] = reachable
        end
        closure
      end

      # Returns a sorted array of ALL nodes reachable from +node+ by
      # following edges forward (transitively).
      #
      # "If I change this package, what else might break?"
      #
      # Raises NodeNotFoundError if the node is not in the graph.
      def transitive_dependents(node)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        reachable = Set.new
        stack = @forward[node].to_a
        until stack.empty?
          current = stack.pop
          next if reachable.include?(current)

          reachable.add(current)
          stack.concat(@forward[current].to_a)
        end
        reachable.to_a.sort
      end

      # == Independent groups (parallel execution levels)
      #
      # This method partitions the nodes into "layers" where every node
      # in a layer depends only on nodes in earlier layers.  Nodes within
      # the same layer are independent of each other and can be processed
      # in parallel.
      #
      # This is essentially a level-by-level topological sort:
      #
      #   Layer 0: nodes with no dependencies (in-degree 0)
      #   Layer 1: nodes whose dependencies are all in layer 0
      #   Layer 2: nodes whose dependencies are all in layers 0-1
      #   ...and so on.
      #
      # Raises CycleError if the graph contains a cycle.
      def independent_groups
        # Compute in-degrees.
        in_degree = {}
        @forward.each_key { |node| in_degree[node] = 0 }
        @forward.each_value do |targets|
          targets.each { |t| in_degree[t] += 1 }
        end

        # Seed the first layer with zero-in-degree nodes.
        current_layer = in_degree.select { |_, deg| deg.zero? }.keys.sort

        groups = []
        processed = 0

        until current_layer.empty?
          groups << current_layer
          processed += current_layer.size

          next_layer = Set.new
          current_layer.each do |node|
            @forward[node].each do |succ|
              in_degree[succ] -= 1
              next_layer.add(succ) if in_degree[succ].zero?
            end
          end

          current_layer = next_layer.to_a.sort
        end

        if processed != @forward.size
          raise CycleError, "Graph contains a cycle — cannot compute independent groups"
        end

        groups
      end

      # == Affected nodes
      #
      # Given a set of "changed" nodes, returns ALL nodes that could be
      # affected — the changed nodes themselves plus all their transitive
      # dependents.
      #
      # This is the key operation for incremental builds: "These files
      # changed; what do I need to rebuild?"
      #
      # Raises NodeNotFoundError if any changed node is not in the graph.
      def affected_nodes(changed)
        affected = Set.new

        changed.each do |node|
          unless has_node?(node)
            raise NodeNotFoundError, "Node not found: #{node.inspect}"
          end

          affected.add(node)
          transitive_dependents(node).each { |dep| affected.add(dep) }
        end

        affected.to_a.sort
      end
    end
  end
end

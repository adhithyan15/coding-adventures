# frozen_string_literal: true

# --------------------------------------------------------------------------
# labeled_graph.rb — A directed graph where edges carry labels
# --------------------------------------------------------------------------
#
# == What is a labeled graph?
#
# A regular directed graph says "there IS an edge from A to B."  A labeled
# directed graph says "there is an edge from A to B *with label L*."
# Multiple labels can exist on the same pair of nodes, and the same label
# can appear on different edges.
#
# == Why do we need this?
#
# Labeled graphs are essential for modeling systems where the *kind* of
# relationship matters, not just its existence.  The canonical example is
# a finite state machine (FSM):
#
#   - Nodes are states (e.g. "locked", "unlocked")
#   - Edge labels are input symbols (e.g. "coin", "push")
#   - The edge ("locked", "unlocked", "coin") means:
#     "when in state 'locked' and input is 'coin', go to 'unlocked'"
#
# Self-loops are allowed because an FSM state can transition back to
# itself (e.g. inserting a coin in an already-unlocked turnstile).
#
# == Design: composition over inheritance
#
# Rather than inheriting from Graph, we *wrap* a Graph instance.  This
# is the "composition" pattern — LabeledGraph HAS-A Graph rather than
# IS-A Graph.  We do this because:
#
#   1. We need to set `allow_self_loops: true` on the inner graph.
#   2. The edge semantics are different — we store (from, to, label)
#      triples, not just (from, to) pairs.
#   3. We can still delegate algorithms like topological_sort directly
#      to the inner graph, since those only care about node connectivity.
#
# == Internal storage
#
# The labels are stored in a Hash keyed by [from, to] pairs:
#
#   @labels = { ["locked", "unlocked"] => Set["coin"],
#               ["unlocked", "locked"] => Set["push"] }
#
# The inner Graph handles node storage and adjacency.  The labels Hash
# adds the extra dimension of *which* labels exist on each edge.
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    class LabeledGraph
      # ----------------------------------------------------------------
      # Construction
      # ----------------------------------------------------------------

      # Creates a new, empty labeled directed graph.
      #
      # The inner graph allows self-loops because labeled graphs commonly
      # model state machines where a state can transition to itself.
      def initialize
        @graph = Graph.new(allow_self_loops: true)
        @labels = {}
      end

      # ----------------------------------------------------------------
      # Node operations — delegated to the inner graph
      # ----------------------------------------------------------------
      #
      # Nodes in a labeled graph work exactly like nodes in a regular
      # graph.  We simply forward these calls.

      # Adds a node to the graph.  No-op if it already exists.
      def add_node(node)
        @graph.add_node(node)
        self
      end

      # Removes a node and ALL edges (with all labels) touching it.
      #
      # We must clean up the labels Hash entries for any edge that
      # involves this node — both outgoing and incoming.
      #
      # Raises NodeNotFoundError if the node is not in the graph.
      def remove_node(node)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        # Clean up label entries for outgoing edges.
        @graph.successors(node).each do |succ|
          @labels.delete([node, succ])
        end

        # Clean up label entries for incoming edges.
        @graph.predecessors(node).each do |pred|
          @labels.delete([pred, node])
        end

        # Clean up self-loop label entries if present.
        @labels.delete([node, node])

        @graph.remove_node(node)
        self
      end

      # Returns true if the node exists in the graph.
      def has_node?(node)
        @graph.has_node?(node)
      end

      # Returns a sorted array of all nodes.
      def nodes
        @graph.nodes
      end

      # Returns the number of nodes in the graph.
      def size
        @graph.size
      end

      # ----------------------------------------------------------------
      # Labeled edge operations
      # ----------------------------------------------------------------
      #
      # These are the methods that distinguish LabeledGraph from Graph.
      # Every edge operation requires a label.

      # Adds a labeled edge from +from_node+ to +to_node+ with +label+.
      #
      # Both nodes are implicitly created if they don't exist.
      # If the exact same (from, to, label) triple already exists, this
      # is a no-op (Set deduplication on the label set).
      #
      # Adding a second label to the same (from, to) pair creates a
      # multi-labeled edge — the underlying graph still has one structural
      # edge, but the labels Set grows.
      def add_edge(from_node, to_node, label)
        # Ensure the structural edge exists in the inner graph.
        # The inner graph handles node creation and allows self-loops.
        @graph.add_edge(from_node, to_node) unless @graph.has_edge?(from_node, to_node)

        key = [from_node, to_node]
        @labels[key] ||= Set.new
        @labels[key].add(label)
        self
      end

      # Removes a specific labeled edge.
      #
      # If this was the last label on the (from, to) pair, the
      # structural edge is also removed from the inner graph.
      #
      # Raises NodeNotFoundError if either node doesn't exist.
      # Raises EdgeNotFoundError if the (from, to, label) triple doesn't exist.
      def remove_edge(from_node, to_node, label)
        unless has_node?(from_node)
          raise NodeNotFoundError, "Node not found: #{from_node.inspect}"
        end
        unless has_node?(to_node)
          raise NodeNotFoundError, "Node not found: #{to_node.inspect}"
        end

        key = [from_node, to_node]
        unless @labels.key?(key) && @labels[key].include?(label)
          raise EdgeNotFoundError,
            "Edge not found: #{from_node.inspect} -> #{to_node.inspect} [#{label.inspect}]"
        end

        @labels[key].delete(label)

        # If no labels remain, remove the structural edge too.
        if @labels[key].empty?
          @labels.delete(key)
          @graph.remove_edge(from_node, to_node)
        end

        self
      end

      # Checks whether an edge exists.
      #
      # - With a label:  returns true if the exact (from, to, label) triple exists.
      # - Without a label: returns true if ANY edge from -> to exists (regardless of label).
      def has_edge?(from_node, to_node, label = nil)
        if label.nil?
          @graph.has_edge?(from_node, to_node)
        else
          key = [from_node, to_node]
          @labels.key?(key) && @labels[key].include?(label)
        end
      end

      # Returns all edges as [from, to, label] triples, sorted for
      # deterministic output.
      #
      # If a pair (A, B) has labels {"x", "y"}, we return two triples:
      #   ["A", "B", "x"] and ["A", "B", "y"]
      def edges
        result = []
        @labels.each do |(from_node, to_node), label_set|
          label_set.each do |label|
            result << [from_node, to_node, label]
          end
        end
        result.sort
      end

      # Returns the Set of labels on the edge from +from_node+ to +to_node+.
      #
      # Returns an empty Set if no edge exists between the two nodes.
      #
      # Raises NodeNotFoundError if either node doesn't exist.
      def labels(from_node, to_node)
        unless has_node?(from_node)
          raise NodeNotFoundError, "Node not found: #{from_node.inspect}"
        end
        unless has_node?(to_node)
          raise NodeNotFoundError, "Node not found: #{to_node.inspect}"
        end

        key = [from_node, to_node]
        @labels.key?(key) ? @labels[key].dup : Set.new
      end

      # ----------------------------------------------------------------
      # Neighbor queries with optional label filtering
      # ----------------------------------------------------------------
      #
      # These extend the basic Graph predecessors/successors with an
      # optional `label:` keyword that filters by edge label.

      # Returns successors of +node+, optionally filtered by +label+.
      #
      # - Without label: returns ALL direct successors (same as Graph).
      # - With label: returns only successors reachable via an edge
      #   carrying that specific label.
      #
      # Raises NodeNotFoundError if the node doesn't exist.
      def successors(node, label: nil)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        if label.nil?
          @graph.successors(node)
        else
          result = []
          @graph.successors(node).each do |succ|
            key = [node, succ]
            result << succ if @labels.key?(key) && @labels[key].include?(label)
          end
          result.sort
        end
      end

      # Returns predecessors of +node+, optionally filtered by +label+.
      #
      # - Without label: returns ALL direct predecessors (same as Graph).
      # - With label: returns only predecessors connected via an edge
      #   carrying that specific label.
      #
      # Raises NodeNotFoundError if the node doesn't exist.
      def predecessors(node, label: nil)
        unless has_node?(node)
          raise NodeNotFoundError, "Node not found: #{node.inspect}"
        end

        if label.nil?
          @graph.predecessors(node)
        else
          result = []
          @graph.predecessors(node).each do |pred|
            key = [pred, node]
            result << pred if @labels.key?(key) && @labels[key].include?(label)
          end
          result.sort
        end
      end

      # ----------------------------------------------------------------
      # Algorithm delegation
      # ----------------------------------------------------------------
      #
      # Graph algorithms operate on the structural topology (which nodes
      # are connected to which), not on labels.  We delegate directly to
      # the inner Graph.

      # Returns a topological ordering of all nodes.
      # Raises CycleError if the graph contains a cycle.
      def topological_sort
        @graph.topological_sort
      end

      # Returns true if the graph contains at least one cycle.
      def has_cycle?
        @graph.has_cycle?
      end

      # Returns the full transitive closure as a Hash of Sets.
      def transitive_closure
        @graph.transitive_closure
      end
    end
  end
end

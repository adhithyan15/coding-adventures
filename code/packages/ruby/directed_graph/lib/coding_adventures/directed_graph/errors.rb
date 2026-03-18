# frozen_string_literal: true

# --------------------------------------------------------------------------
# errors.rb — Custom exception hierarchy
# --------------------------------------------------------------------------
#
# Every library benefits from fine-grained error types so callers can rescue
# exactly what they expect.  We define three:
#
#   CycleError         — raised when an operation (e.g. topological sort)
#                        discovers a cycle in a graph that must be acyclic.
#
#   NodeNotFoundError  — raised when a caller references a node that has
#                        not been added to the graph.
#
#   EdgeNotFoundError  — raised when a caller tries to remove or query an
#                        edge that does not exist.
#
# All three inherit from StandardError so a bare `rescue` will still catch
# them, but callers can also write `rescue CodingAdventures::DirectedGraph::CycleError`.
# --------------------------------------------------------------------------

module CodingAdventures
  module DirectedGraph
    # Raised when a cycle is detected in a graph that requires acyclicity.
    class CycleError < StandardError; end

    # Raised when an operation references a node not present in the graph.
    class NodeNotFoundError < StandardError; end

    # Raised when an operation references an edge not present in the graph.
    class EdgeNotFoundError < StandardError; end
  end
end

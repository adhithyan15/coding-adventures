# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_directed_graph.rb — Gem entry point
# --------------------------------------------------------------------------
#
# This is the file that gets loaded when someone writes:
#
#   require "coding_adventures_directed_graph"
#
# It pulls in the three internal files in dependency order:
#   1. version  — the VERSION constant (no dependencies)
#   2. errors   — the custom exception classes (no dependencies)
#   3. graph    — the Graph class (uses errors + Set from stdlib)
#
# Usage:
#   require "coding_adventures_directed_graph"
#
#   g = CodingAdventures::DirectedGraph::Graph.new
#   g.add_edge("A", "B")
#   g.add_edge("B", "C")
#   g.topological_sort  # => ["A", "B", "C"]
# --------------------------------------------------------------------------

require_relative "coding_adventures/directed_graph/version"
require_relative "coding_adventures/directed_graph/errors"
require_relative "coding_adventures/directed_graph/graph"
require_relative "coding_adventures/directed_graph/labeled_graph"
require_relative "coding_adventures/directed_graph/visualization"

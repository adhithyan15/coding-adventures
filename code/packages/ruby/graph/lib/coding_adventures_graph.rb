# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_graph.rb — Gem entry point
# --------------------------------------------------------------------------
#
# This is the file that gets loaded when someone writes:
#
#   require "coding_adventures_graph"
#
# It pulls in the internal files in dependency order:
#   1. version  — the VERSION constant (no dependencies)
#   2. graph    — the Graph class
#
# Usage:
#   require "coding_adventures_graph"
#
#   g = CodingAdventures::Graph::Graph.new
#   g.add_edge("A", "B")
#   g.add_edge("B", "C")
#   g.nodes  # => ["A", "B", "C"]
# --------------------------------------------------------------------------

require_relative "coding_adventures/graph/version"
require_relative "coding_adventures/graph/graph"
require_relative "coding_adventures/graph/algorithms"

# Extend the module to include Graph methods globally
include CodingAdventures::Graph

# Make algorithm functions available at module level
extend CodingAdventures::Graph

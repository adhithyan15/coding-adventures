# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_directed_graph_native.rb -- Gem entry point
# --------------------------------------------------------------------------
#
# This is the file that gets loaded when someone writes:
#
#   require "coding_adventures_directed_graph_native"
#
# It simply delegates to the native extension loader which handles
# finding and loading the compiled Rust shared library.
#
# Usage:
#   require "coding_adventures_directed_graph_native"
#
#   g = CodingAdventures::DirectedGraphNative::DirectedGraph.new
#   g.add_edge("A", "B")
#   g.topological_sort  # => ["A", "B"]
# --------------------------------------------------------------------------

require_relative "directed_graph_native"

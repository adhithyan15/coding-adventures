# frozen_string_literal: true

# --------------------------------------------------------------------------
# directed_graph_native.rb -- Native extension loader
# --------------------------------------------------------------------------
#
# This file is the entry point for the native extension. When Ruby does:
#
#   require "directed_graph_native"
#
# it loads this file, which in turn loads the compiled Rust shared library.
# The shared library registers the CodingAdventures::DirectedGraphNative
# module and the DirectedGraph class within it.
#
# The native extension is compiled from Rust source in src/lib.rs using
# Magnus (the Rust-Ruby bridge). The compiled shared library has a
# platform-specific name:
#   - Linux:   directed_graph_native.so
#   - macOS:   directed_graph_native.bundle
#   - Windows: directed_graph_native.dll
#
# Usage:
#   require "directed_graph_native"
#
#   g = CodingAdventures::DirectedGraphNative::DirectedGraph.new
#   g.add_edge("A", "B")
#   g.add_edge("B", "C")
#   g.topological_sort  # => ["A", "B", "C"]
# --------------------------------------------------------------------------

begin
  # Try to load the precompiled native extension from the lib directory.
  # rb_sys and rake-compiler place the compiled .so/.bundle here after
  # `rake compile`.
  require_relative "directed_graph_native/directed_graph_native"
rescue LoadError
  # If the extension hasn't been compiled yet, raise a helpful error.
  raise LoadError,
    "Could not load the directed_graph_native native extension. " \
    "Make sure you have compiled it with `rake compile`. " \
    "See the README for build instructions."
end

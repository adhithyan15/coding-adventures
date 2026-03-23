# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_directed_graph_native.rb — Entry point for the gem
# --------------------------------------------------------------------------
#
# This file is the main require target for the gem. It loads:
# 1. The compiled Rust native extension (.so/.bundle/.dll)
# 2. The version constant
#
# The native extension defines:
#   CodingAdventures::DirectedGraphNative::Graph
#
# which is a Rust-backed directed graph with the same API as the pure
# Ruby version (CodingAdventures::DirectedGraph::Graph), but faster
# because all algorithms run in native code.

require "set"
require_relative "coding_adventures/directed_graph_native/version"

# Load the compiled native extension
# Ruby will search for directed_graph_native.so (Linux),
# directed_graph_native.bundle (macOS), or directed_graph_native.dll (Windows)
require "directed_graph_native"

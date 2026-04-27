# frozen_string_literal: true

# --------------------------------------------------------------------------
# coding_adventures_markov_chain.rb — Gem entry point
# --------------------------------------------------------------------------
#
# This file is loaded when someone writes:
#
#   require "coding_adventures_markov_chain"
#
# It loads dependencies in the correct order:
#   1. coding_adventures_directed_graph — the graph topology layer
#   2. version  — the VERSION constant for this gem
#   3. markov_chain — the MarkovChain implementation
#
# The `require "coding_adventures_directed_graph"` call MUST come before
# the require_relative calls so that the CodingAdventures::DirectedGraph
# constants are defined before markov_chain.rb references them.
#
# Usage:
#   require "coding_adventures_markov_chain"
#
#   chain = CodingAdventures::MarkovChain.new(order: 1, smoothing: 0.0)
#   chain.train(%w[A B A C A B B A])
#   chain.probability("A", "B")  # => ~0.667
# --------------------------------------------------------------------------

require "coding_adventures_directed_graph"
require_relative "coding_adventures/markov_chain/version"
require_relative "coding_adventures/markov_chain"

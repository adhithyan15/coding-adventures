# frozen_string_literal: true

# Entry point for the coding_adventures_clock gem.
#
# This gem simulates the system clock that drives all sequential logic
# in a computer. It provides:
#
# - ClockGenerator: A square-wave generator alternating between 0 and 1
# - ClockDivider: Derives slower clocks from a fast master clock
# - MultiPhaseClock: Generates multiple non-overlapping clock phases
#
# Usage:
#   require "coding_adventures_clock"
#
#   clk = CodingAdventures::Clock::ClockGenerator.new
#   edge = clk.tick  # => ClockEdge (rising, cycle 1)

require_relative "coding_adventures/clock/version"
require_relative "coding_adventures/clock/clock"

# frozen_string_literal: true

# Display --- VGA text-mode framebuffer simulation.
#
# This package simulates a VGA text-mode framebuffer display, modeled after
# the classic 80x25 text mode that dominated personal computing from the
# 1980s through the early 2000s. Each cell is 2 bytes: one for the ASCII
# character and one for the color attribute.
#
# Quick start:
#   require "coding_adventures_display"
#   config = CodingAdventures::Display::DisplayConfig.new
#   memory = Array.new(config.columns * config.rows * 2, 0)
#   driver = CodingAdventures::Display::DisplayDriver.new(config, memory)
#   driver.puts_str("Hello World")
#   snap = driver.snapshot
#   snap.lines[0]  # => "Hello World"

require_relative "coding_adventures/display/version"
require_relative "coding_adventures/display/framebuffer"
require_relative "coding_adventures/display/driver"
require_relative "coding_adventures/display/snapshot"

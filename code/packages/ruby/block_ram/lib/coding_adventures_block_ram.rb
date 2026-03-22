# frozen_string_literal: true

# Entry point for the coding_adventures_block_ram gem.
#
# This gem implements SRAM cells, arrays, and RAM modules that form
# the memory hierarchy used in FPGAs and CPUs.
#
# Usage:
#   require "coding_adventures_block_ram"
#
#   cell = CodingAdventures::BlockRam::SRAMCell.new
#   cell.write(1, 1)
#   cell.read(1)  # => 1

require_relative "coding_adventures/block_ram/version"
require_relative "coding_adventures/block_ram/sram"
require_relative "coding_adventures/block_ram/ram"
require_relative "coding_adventures/block_ram/bram"

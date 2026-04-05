# frozen_string_literal: true

# ================================================================
# coding_adventures_mosaic_vm -- Top-Level Require File
# ================================================================
#
# Usage:
#   require "coding_adventures_mosaic_vm"
#   vm = CodingAdventures::MosaicVm::MosaicVM.new(ir)
#   result = vm.run(renderer)
# ================================================================

# IMPORTANT: Require dependencies FIRST.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_mosaic_parser"
require "coding_adventures_mosaic_analyzer"

require_relative "coding_adventures/mosaic_vm/version"
require_relative "coding_adventures/mosaic_vm/vm"

module CodingAdventures
  # Generic tree walker that drives Mosaic compiler backends
  module MosaicVm
  end
end

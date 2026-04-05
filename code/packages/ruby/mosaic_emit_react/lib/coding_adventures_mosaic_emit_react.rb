# frozen_string_literal: true

# ================================================================
# coding_adventures_mosaic_emit_react -- Top-Level Require File
# ================================================================
#
# Usage:
#   require "coding_adventures_mosaic_emit_react"
#   ir = CodingAdventures::MosaicAnalyzer.analyze(source)
#   vm = CodingAdventures::MosaicVm::MosaicVM.new(ir)
#   result = vm.run(CodingAdventures::MosaicEmitReact::ReactRenderer.new)
# ================================================================

# IMPORTANT: Require dependencies FIRST.
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_mosaic_parser"
require "coding_adventures_mosaic_analyzer"
require "coding_adventures_mosaic_vm"

require_relative "coding_adventures/mosaic_emit_react/version"
require_relative "coding_adventures/mosaic_emit_react/react_renderer"

module CodingAdventures
  # React backend: emits TSX functional components from MosaicIR
  module MosaicEmitReact
  end
end

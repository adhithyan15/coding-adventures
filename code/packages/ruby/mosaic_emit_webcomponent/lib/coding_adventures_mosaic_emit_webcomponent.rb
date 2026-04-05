# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_mosaic_vm"
require "coding_adventures_mosaic_analyzer"
require "coding_adventures_mosaic_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_directed_graph"
require "coding_adventures_parser"
require "coding_adventures_state_machine"

require_relative "coding_adventures/mosaic_emit_webcomponent/version"
require_relative "coding_adventures/mosaic_emit_webcomponent/webcomponent_renderer"

module CodingAdventures
  # Web Components backend: emits Custom Element classes from MosaicIR
  module MosaicEmitWebcomponent
  end
end

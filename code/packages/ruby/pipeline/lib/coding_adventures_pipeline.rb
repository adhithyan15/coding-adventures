# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_lexer"
require "coding_adventures_parser"

require_relative "coding_adventures/pipeline/version"
require_relative "coding_adventures/pipeline/orchestrator"

module CodingAdventures
  # Orchestrator that chains lexer and parser into a single execution flow,
  # capturing traces at every stage.
  #
  # Usage:
  #
  #   result = CodingAdventures::Pipeline::Orchestrator.new.run("x = 1 + 2")
  #   result.lexer_stage.token_count   # => 7
  #   result.parser_stage.ast          # => Program(...)
  #
  # See CodingAdventures::Pipeline::Orchestrator for the full API.
  module Pipeline
  end
end

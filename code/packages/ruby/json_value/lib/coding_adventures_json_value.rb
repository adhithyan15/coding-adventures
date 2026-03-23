# frozen_string_literal: true

# ================================================================
# coding_adventures_json_value -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_json_value"
#
# Ruby loads this file, which sets up the CodingAdventures::JsonValue
# module with all its type classes and conversion methods.
#
# Dependencies are loaded first (json_parser and its transitive deps),
# then our own modules in the correct order:
#   1. version  -- gem version constant
#   2. error    -- exception class (needed by converter)
#   3. types    -- JsonValue type classes (needed by converter)
#   4. converter -- the conversion logic (from_ast, to_native, etc.)
# ================================================================

# Load dependency first -- json_parser brings in the lexer, parser,
# grammar tools, and json_lexer that we need for AST types and parsing.
require "coding_adventures_json_parser"

# Load our own modules in dependency order.
require_relative "coding_adventures/json_value/version"
require_relative "coding_adventures/json_value/error"
require_relative "coding_adventures/json_value/types"
require_relative "coding_adventures/json_value/converter"

module CodingAdventures
  module JsonValue
  end
end

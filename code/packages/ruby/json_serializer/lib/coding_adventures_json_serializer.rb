# frozen_string_literal: true

# ================================================================
# coding_adventures_json_serializer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_json_serializer"
#
# Ruby loads this file, which sets up the CodingAdventures::JsonSerializer
# module with its serialization methods and configuration.
#
# Dependencies are loaded first -- json_value (which transitively
# loads json_parser, json_lexer, parser, lexer, grammar_tools).
# Then our own modules:
#   1. version    -- gem version constant
#   2. error      -- exception class
#   3. config     -- SerializerConfig for pretty-printing options
#   4. serializer -- the serialization logic
# ================================================================

# Load dependency first -- json_value brings in the full JSON
# parsing pipeline that we need for type checking and from_native.
require "coding_adventures_json_value"

# Load our own modules in dependency order.
require_relative "coding_adventures/json_serializer/version"
require_relative "coding_adventures/json_serializer/error"
require_relative "coding_adventures/json_serializer/config"
require_relative "coding_adventures/json_serializer/serializer"

module CodingAdventures
  module JsonSerializer
  end
end

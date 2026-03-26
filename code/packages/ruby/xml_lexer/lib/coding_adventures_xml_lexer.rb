# frozen_string_literal: true

# ================================================================
# coding_adventures_xml_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_xml_lexer"
#
# Ruby loads this file, which in turn loads the version and the
# tokenizer module. The tokenizer is where the real work happens --
# it wraps the grammar-driven lexer with an on_token callback that
# switches between pattern groups for XML's context-sensitive
# lexical structure.
# ================================================================

require_relative "coding_adventures/xml_lexer/version"
require_relative "coding_adventures/xml_lexer/tokenizer"

module CodingAdventures
  module XmlLexer
  end
end

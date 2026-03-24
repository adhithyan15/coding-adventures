# frozen_string_literal: true

# ================================================================
# coding_adventures_sql_parser -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_sql_parser"
#
# Ruby loads this file, which in turn loads the version and the
# parser module. The parser is where the real work happens.
# ================================================================

require_relative "coding_adventures/sql_parser/version"
require_relative "coding_adventures/sql_parser/parser"

module CodingAdventures
  module SqlParser
  end
end

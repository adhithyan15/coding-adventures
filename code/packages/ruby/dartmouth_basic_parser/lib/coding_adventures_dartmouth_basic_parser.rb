# frozen_string_literal: true

# ================================================================
# coding_adventures_dartmouth_basic_parser -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_dartmouth_basic_parser"
#
# Ruby loads this file, which in turn loads the version constant and
# the parser module. The parser module is where the real work happens.
#
# Dartmouth BASIC (1964) was designed by John Kemeny and Thomas Kurtz
# to make time-shared computing accessible to non-specialists. The GE-225
# mainframe at Dartmouth College could support 30 simultaneous teletype
# users — a revolutionary arrangement in an era of batch processing.
#
# The language's simplicity made it viral. By the mid-1970s, BASIC was
# everywhere. By the mid-1980s, every home computer had BASIC built in.
# Bill Gates and Paul Allen wrote a BASIC interpreter for the Altair 8800
# as Microsoft's first product. Without Dartmouth BASIC, the history of
# personal computing would look very different.
# ================================================================

require_relative "coding_adventures/dartmouth_basic_parser/version"
require_relative "coding_adventures/dartmouth_basic_parser/parser"

module CodingAdventures
  module DartmouthBasicParser
  end
end

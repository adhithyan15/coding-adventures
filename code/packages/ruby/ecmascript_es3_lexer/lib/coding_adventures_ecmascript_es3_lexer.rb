# frozen_string_literal: true

# ================================================================
# coding_adventures_ecmascript_es3_lexer -- Top-Level Require File
# ================================================================
#
# This is the entry point for the gem. When someone writes:
#
#   require "coding_adventures_ecmascript_es3_lexer"
#
# Ruby loads this file, which in turn loads the version and the
# tokenizer module. The tokenizer is where the real work happens.
# ================================================================

require_relative "coding_adventures/ecmascript_es3_lexer/version"
require_relative "coding_adventures/ecmascript_es3_lexer/tokenizer"

module CodingAdventures
  module EcmascriptEs3Lexer
  end
end

# frozen_string_literal: true

# ==========================================================================
# Token -- The Smallest Meaningful Unit of Source Code
# ==========================================================================
#
# A token pairs a type (what kind of thing it is) with a value (the actual
# text from the source code), plus position information for error reporting.
#
# We use Ruby's Data.define to create an immutable value object -- once
# created, a token never changes. This mirrors the Python version's
# frozen dataclass.
# ==========================================================================

module CodingAdventures
  module Lexer
    Token = Data.define(:type, :value, :line, :column) do
      def to_s
        "Token(#{type}, #{value.inspect}, #{line}:#{column})"
      end
    end
  end
end

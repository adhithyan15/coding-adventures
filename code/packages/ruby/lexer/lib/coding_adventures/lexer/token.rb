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
      # Return the token type as a string, regardless of representation.
      #
      # The type field can be either a TokenType symbol or a plain String
      # (for grammar-driven tokens that define custom types like
      # "SIZED_NUMBER" or "SYSTEM_TASK"). This method provides uniform access:
      #
      #   TokenType::NAME → "NAME"
      #   "SIZED_NUMBER"  → "SIZED_NUMBER"
      def type_name
        type.is_a?(::String) ? type : type.to_s
      end

      def to_s
        "Token(#{type_name}, #{value.inspect}, #{line}:#{column})"
      end
    end
  end
end

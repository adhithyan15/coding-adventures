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
#
# Token Flags
# -----------
#
# Flags carry information that is neither type nor value but affects how
# downstream consumers (parsers, formatters, linters) interpret a token.
# Flags are optional -- when nil, all flags are off. Use bitwise AND to
# test: (token.flags.to_i & TOKEN_PRECEDED_BY_NEWLINE) != 0
#
# Available flags:
#
#   TOKEN_PRECEDED_BY_NEWLINE (1) -- set when a line break appeared between
#     this token and the previous one. Languages with automatic semicolon
#     insertion (JavaScript, Go) use this to decide whether an implicit
#     semicolon should be inserted.
#
#   TOKEN_CONTEXT_KEYWORD (2) -- set for context-sensitive keywords: words
#     that are keywords in some syntactic positions but identifiers in
#     others (e.g., JavaScript's async, yield, get, set). The lexer emits
#     these as NAME tokens with this flag, leaving the final decision to
#     the language-specific parser.
# ==========================================================================

module CodingAdventures
  module Lexer
    # Bitmask flag: a line break appeared before this token.
    TOKEN_PRECEDED_BY_NEWLINE = 1

    # Bitmask flag: this NAME token is a context-sensitive keyword.
    TOKEN_CONTEXT_KEYWORD = 2

    Token = Data.define(:type, :value, :line, :column, :flags) do
      # The flags parameter defaults to nil when not provided, meaning
      # "no flags set." This keeps backward compatibility -- existing code
      # that creates tokens without flags continues to work unchanged.
      def initialize(type:, value:, line:, column:, flags: nil)
        super(type: type, value: value, line: line, column: column, flags: flags)
      end

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

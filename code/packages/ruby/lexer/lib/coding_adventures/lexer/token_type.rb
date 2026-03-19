# frozen_string_literal: true

# ==========================================================================
# TokenType -- Every Possible Kind of Token
# ==========================================================================
#
# Think of this like a catalog of "word types" in a language. In English,
# you have nouns, verbs, adjectives, and punctuation. In a programming
# language, you have names (identifiers), numbers, operators, and so on.
#
# We use a Ruby module with string constants because Ruby does not have
# Python-style enums built in. Each constant is a frozen string that serves
# as a unique tag for that token type.
# ==========================================================================

module CodingAdventures
  module Lexer
    module TokenType
      # Literals
      NAME           = "NAME"
      NUMBER         = "NUMBER"
      STRING         = "STRING"
      KEYWORD        = "KEYWORD"

      # Operators
      PLUS           = "PLUS"
      MINUS          = "MINUS"
      STAR           = "STAR"
      SLASH          = "SLASH"
      EQUALS         = "EQUALS"
      EQUALS_EQUALS  = "EQUALS_EQUALS"

      # Delimiters
      LPAREN         = "LPAREN"
      RPAREN         = "RPAREN"
      COMMA          = "COMMA"
      COLON          = "COLON"
      SEMICOLON      = "SEMICOLON"
      LBRACE         = "LBRACE"
      RBRACE         = "RBRACE"
      LBRACKET       = "LBRACKET"
      RBRACKET       = "RBRACKET"
      DOT            = "DOT"
      BANG           = "BANG"

      # Structural
      NEWLINE        = "NEWLINE"
      EOF            = "EOF"

      # Map from string name to constant value for dynamic lookup.
      ALL = {
        "NAME" => NAME,
        "NUMBER" => NUMBER,
        "STRING" => STRING,
        "KEYWORD" => KEYWORD,
        "PLUS" => PLUS,
        "MINUS" => MINUS,
        "STAR" => STAR,
        "SLASH" => SLASH,
        "EQUALS" => EQUALS,
        "EQUALS_EQUALS" => EQUALS_EQUALS,
        "LPAREN" => LPAREN,
        "RPAREN" => RPAREN,
        "COMMA" => COMMA,
        "COLON" => COLON,
        "SEMICOLON" => SEMICOLON,
        "LBRACE" => LBRACE,
        "RBRACE" => RBRACE,
        "LBRACKET" => LBRACKET,
        "RBRACKET" => RBRACKET,
        "DOT" => DOT,
        "BANG" => BANG,
        "NEWLINE" => NEWLINE,
        "EOF" => EOF
      }.freeze
    end
  end
end

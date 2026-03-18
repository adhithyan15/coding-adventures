# frozen_string_literal: true

# ==========================================================================
# Tokenizer -- Hand-Written Lexer
# ==========================================================================
#
# This is the reference lexer implementation. It reads source code character
# by character, dispatching on the first character to determine what kind
# of token is starting at each position. The algorithm is classic "dispatch
# on first character":
#
#   1. Space/tab  -> skip whitespace
#   2. Newline    -> emit NEWLINE token
#   3. Digit      -> read a number
#   4. Letter/_   -> read a name/keyword
#   5. Double "   -> read a string
#   6. =          -> peek ahead: = or ==
#   7. Simple op  -> look up in table
#   8. Otherwise  -> error
#
# The lexer is language-agnostic. The only language-specific part is the
# keyword list, which is configurable via the keywords parameter.
# ==========================================================================

module CodingAdventures
  module Lexer
    # An error encountered during tokenization.
    class LexerError < StandardError
      attr_reader :message, :line, :column

      def initialize(message, line:, column:)
        @message = message
        @line = line
        @column = column
        super("Lexer error at #{line}:#{column}: #{message}")
      end
    end

    # The main hand-written lexer.
    class Tokenizer
      # Map from single characters to their token types.
      SIMPLE_TOKENS = {
        "+" => TokenType::PLUS,
        "-" => TokenType::MINUS,
        "*" => TokenType::STAR,
        "/" => TokenType::SLASH,
        "(" => TokenType::LPAREN,
        ")" => TokenType::RPAREN,
        "," => TokenType::COMMA,
        ":" => TokenType::COLON
      }.freeze

      # @param source [String] the raw source code to tokenize
      # @param keywords [Array<String>] reserved words for this language
      def initialize(source, keywords: [])
        @source = source
        @pos = 0
        @line = 1
        @column = 1
        @tokens = []
        @keyword_set = keywords.to_set.freeze
      end

      # Tokenize the entire source code and return a list of tokens.
      # Always ends with an EOF token.
      def tokenize
        @tokens = []

        while (char = current_char)
          # Whitespace (spaces and tabs, NOT newlines).
          if char == " " || char == "\t" || char == "\r"
            skip_whitespace
            next
          end

          # Newlines.
          if char == "\n"
            @tokens << Token.new(
              type: TokenType::NEWLINE, value: "\\n",
              line: @line, column: @column
            )
            advance
            next
          end

          # Numbers.
          if char.match?(/[0-9]/)
            @tokens << read_number
            next
          end

          # Names and keywords.
          if char.match?(/[a-zA-Z_]/)
            @tokens << read_name
            next
          end

          # String literals.
          if char == '"'
            @tokens << read_string
            next
          end

          # The = and == operators (requires lookahead).
          if char == "="
            start_line = @line
            start_column = @column
            advance
            if current_char == "="
              advance
              @tokens << Token.new(
                type: TokenType::EQUALS_EQUALS, value: "==",
                line: start_line, column: start_column
              )
            else
              @tokens << Token.new(
                type: TokenType::EQUALS, value: "=",
                line: start_line, column: start_column
              )
            end
            next
          end

          # Simple single-character tokens.
          if SIMPLE_TOKENS.key?(char)
            @tokens << Token.new(
              type: SIMPLE_TOKENS[char], value: char,
              line: @line, column: @column
            )
            advance
            next
          end

          # Unexpected character.
          raise LexerError.new(
            "Unexpected character: #{char.inspect}",
            line: @line, column: @column
          )
        end

        # EOF token.
        @tokens << Token.new(
          type: TokenType::EOF, value: "",
          line: @line, column: @column
        )

        @tokens
      end

      private

      def current_char
        return nil if @pos >= @source.length
        @source[@pos]
      end

      def peek
        peek_pos = @pos + 1
        return nil if peek_pos >= @source.length
        @source[peek_pos]
      end

      def advance
        char = @source[@pos]
        @pos += 1
        if char == "\n"
          @line += 1
          @column = 1
        else
          @column += 1
        end
        char
      end

      def skip_whitespace
        while (char = current_char) && (char == " " || char == "\t" || char == "\r")
          advance
        end
      end

      def read_number
        start_line = @line
        start_column = @column
        digits = +""
        while (char = current_char) && char.match?(/[0-9]/)
          digits << advance
        end
        Token.new(
          type: TokenType::NUMBER, value: digits,
          line: start_line, column: start_column
        )
      end

      def read_name
        start_line = @line
        start_column = @column
        chars = +""
        while (char = current_char) && char.match?(/[a-zA-Z0-9_]/)
          chars << advance
        end
        token_type = @keyword_set.include?(chars) ? TokenType::KEYWORD : TokenType::NAME
        Token.new(
          type: token_type, value: chars,
          line: start_line, column: start_column
        )
      end

      def read_string
        start_line = @line
        start_column = @column
        chars = +""
        advance # consume opening quote

        loop do
          current = current_char
          if current.nil?
            raise LexerError.new(
              "Unterminated string literal",
              line: start_line, column: start_column
            )
          end

          if current == '"'
            advance # consume closing quote
            break
          end

          if current == "\\"
            advance # consume backslash
            escaped = current_char
            if escaped.nil?
              raise LexerError.new(
                "Unterminated string literal (ends with backslash)",
                line: start_line, column: start_column
              )
            end
            escape_map = { "n" => "\n", "t" => "\t", "\\" => "\\", '"' => '"' }
            chars << (escape_map[escaped] || escaped)
            advance
          else
            chars << current
            advance
          end
        end

        Token.new(
          type: TokenType::STRING, value: chars,
          line: start_line, column: start_column
        )
      end
    end
  end
end

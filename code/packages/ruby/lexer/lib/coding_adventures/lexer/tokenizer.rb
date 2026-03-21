# frozen_string_literal: true

# ==========================================================================
# Tokenizer -- Hand-Written Lexer
# ==========================================================================
#
# This is the reference lexer implementation. It reads source code character
# by character, using a formal DFA to classify each character and dispatch
# to the appropriate sub-routine.
#
# The dispatch logic is driven by the TOKENIZER_DFA defined in
# tokenizer_dfa.rb. At each step, the lexer:
#
#   1. Classifies the current character into a character class
#      (e.g., "5" -> "digit", '"' -> "quote").
#   2. Feeds the character class to the DFA to get the next state
#      (e.g., "start" + "digit" -> "in_number").
#   3. Dispatches to the appropriate sub-routine based on the DFA state:
#      - "in_number"      -> read_number
#      - "in_name"        -> read_name
#      - "in_string"      -> read_string
#      - "in_operator"    -> look up in SIMPLE_TOKENS table
#      - "in_equals"      -> lookahead for = vs ==
#      - "at_newline"     -> emit NEWLINE token
#      - "at_whitespace"  -> skip whitespace
#      - "done"           -> append EOF and stop
#      - "error"          -> raise LexerError
#   4. Resets the DFA to "start" and repeats from step 1.
#
# The DFA does NOT replace the sub-routines -- it formalizes the top-level
# dispatch decision. The sub-routines (read_number, read_string, etc.)
# still do the actual character-by-character work of building tokens.
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
        ":" => TokenType::COLON,
        ";" => TokenType::SEMICOLON,
        "{" => TokenType::LBRACE,
        "}" => TokenType::RBRACE,
        "[" => TokenType::LBRACKET,
        "]" => TokenType::RBRACKET,
        "." => TokenType::DOT,
        "!" => TokenType::BANG
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
      #
      # This is the main entry point. It loops through the source code,
      # character by character, classifying each character and consulting
      # the tokenizer DFA to determine which sub-routine should handle it.
      #
      # The DFA-driven dispatch works as follows:
      #
      #   1. Classify the current character into a character class.
      #   2. Feed the character class to the DFA to get the next state.
      #   3. Dispatch to the appropriate sub-routine.
      #   4. Reset the DFA to "start" and repeat.
      #
      # Always ends with an EOF token.
      def tokenize
        @tokens = []

        # Create a fresh DFA instance for this tokenization run.
        dfa = TokenizerDFA.new_tokenizer_dfa

        loop do
          char = current_char
          char_class = TokenizerDFA.classify_char(char)
          next_state = dfa.process(char_class)

          case next_state
          when "at_whitespace"
            skip_whitespace
          when "at_newline"
            @tokens << Token.new(
              type: TokenType::NEWLINE, value: "\\n",
              line: @line, column: @column
            )
            advance
          when "in_number"
            @tokens << read_number
          when "in_name"
            @tokens << read_name
          when "in_string"
            @tokens << read_string
          when "in_equals"
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
          when "in_operator"
            @tokens << Token.new(
              type: SIMPLE_TOKENS[char], value: char,
              line: @line, column: @column
            )
            advance
          when "done"
            break
          when "error"
            raise LexerError.new(
              "Unexpected character: #{char.inspect}",
              line: @line, column: @column
            )
          end

          # Reset the DFA back to "start" for the next character.
          dfa.reset
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
            escape_map = {"n" => "\n", "t" => "\t", "\\" => "\\", '"' => '"'}
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

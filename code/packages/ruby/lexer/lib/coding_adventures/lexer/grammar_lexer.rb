# frozen_string_literal: true

require "coding_adventures_grammar_tools"

# ==========================================================================
# GrammarLexer -- Grammar-Driven Tokenization from .tokens Files
# ==========================================================================
#
# Instead of hardcoded character-dispatching logic, this lexer reads token
# definitions from a .tokens file (parsed by grammar_tools) and uses those
# definitions to drive tokenization at runtime.
#
# How it works:
#
# 1. Compile each token definition into a Regexp. Literal patterns are
#    escaped so that + and * are treated as literal characters.
#
# 2. At each position in the source code, try each compiled pattern in
#    order (first match wins).
#
# 3. Emit a Token with the matched type and value.
#
# Because both lexers produce identical Token objects, downstream consumers
# (the parser) don't care which lexer generated the tokens.
# ==========================================================================

module CodingAdventures
  module Lexer
    class GrammarLexer
      # @param source [String] the raw source code to tokenize
      # @param grammar [CodingAdventures::GrammarTools::TokenGrammar]
      def initialize(source, grammar)
        @source = source
        @grammar = grammar
        @pos = 0
        @line = 1
        @column = 1
        @keyword_set = grammar.keywords.to_set.freeze

        # Compile token patterns into Regexp objects.
        @patterns = grammar.definitions.map do |defn|
          pattern = if defn.is_regex
            Regexp.new(defn.pattern)
          else
            Regexp.new(Regexp.escape(defn.pattern))
          end
          [defn.name, pattern]
        end
      end

      # Tokenize the source code using the grammar's token definitions.
      def tokenize
        tokens = []

        while @pos < @source.length
          char = @source[@pos]

          # Skip whitespace (spaces, tabs, carriage returns).
          if char == " " || char == "\t" || char == "\r"
            do_advance
            next
          end

          # Newlines become NEWLINE tokens.
          if char == "\n"
            tokens << Token.new(
              type: TokenType::NEWLINE, value: "\\n",
              line: @line, column: @column
            )
            do_advance
            next
          end

          # Try each pattern in priority order (first match wins).
          matched = false
          remaining = @source[@pos..]

          @patterns.each do |token_name, pattern|
            m = pattern.match(remaining)
            next unless m && m.begin(0) == 0

            value = m[0]
            start_line = @line
            start_column = @column

            token_type = resolve_token_type(token_name, value)

            # Handle STRING tokens: strip quotes and process escapes.
            if token_name == "STRING"
              inner = value[1..-2]
              inner = process_escapes(inner)
              tokens << Token.new(
                type: token_type, value: inner,
                line: start_line, column: start_column
              )
            else
              tokens << Token.new(
                type: token_type, value: value,
                line: start_line, column: start_column
              )
            end

            value.length.times { do_advance }
            matched = true
            break
          end

          unless matched
            raise LexerError.new(
              "Unexpected character: #{char.inspect}",
              line: @line, column: @column
            )
          end
        end

        # EOF sentinel.
        tokens << Token.new(
          type: TokenType::EOF, value: "",
          line: @line, column: @column
        )

        tokens
      end

      private

      def do_advance
        return if @pos >= @source.length
        if @source[@pos] == "\n"
          @line += 1
          @column = 1
        else
          @column += 1
        end
        @pos += 1
      end

      # Map a token name from the grammar to a TokenType constant.
      def resolve_token_type(token_name, value)
        if token_name == "NAME" && @keyword_set.include?(value)
          return TokenType::KEYWORD
        end
        TokenType::ALL[token_name] || TokenType::NAME
      end

      # Process escape sequences in a string value.
      def process_escapes(s)
        result = +""
        i = 0
        while i < s.length
          if s[i] == "\\" && i + 1 < s.length
            escape_map = { "n" => "\n", "t" => "\t", "\\" => "\\", '"' => '"' }
            next_char = s[i + 1]
            result << (escape_map[next_char] || next_char)
            i += 2
          else
            result << s[i]
            i += 1
          end
        end
        result
      end
    end
  end
end

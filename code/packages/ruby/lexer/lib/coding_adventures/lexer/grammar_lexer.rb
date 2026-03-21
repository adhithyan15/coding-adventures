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
# Extended features (Starlark support):
#
# - Skip patterns: matched and consumed without producing tokens.
# - Type aliases: a definition with alias_name emits that alias type.
# - Reserved keywords: identifiers matching reserved words raise errors.
# - Indentation mode: Python-style INDENT/DEDENT/NEWLINE tracking.
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
        @reserved_set = grammar.reserved_keywords.to_set.freeze

        # Compile token patterns into Regexp objects.
        @patterns = grammar.definitions.map do |defn|
          pattern = if defn.is_regex
            Regexp.new(defn.pattern)
          else
            Regexp.new(Regexp.escape(defn.pattern))
          end
          [defn.name, pattern, defn.alias_name]
        end

        # Compile skip patterns.
        @skip_patterns = grammar.skip_definitions.map do |defn|
          if defn.is_regex
            Regexp.new(defn.pattern)
          else
            Regexp.new(Regexp.escape(defn.pattern))
          end
        end

        # Indentation mode state.
        @indentation_mode = grammar.mode == "indentation"
        @indent_stack = [0]
        @bracket_depth = 0
      end

      # Tokenize the source code using the grammar's token definitions.
      def tokenize
        if @indentation_mode
          tokenize_indentation
        else
          tokenize_standard
        end
      end

      private

      # Standard tokenization (no indentation tracking).
      def tokenize_standard
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

          # Try skip patterns first.
          next if try_skip

          # Try each token pattern in priority order.
          token = try_match_token
          if token
            tokens << token
            next
          end

          raise LexerError.new(
            "Unexpected character: #{char.inspect}",
            line: @line, column: @column
          )
        end

        # EOF sentinel.
        tokens << Token.new(
          type: TokenType::EOF, value: "",
          line: @line, column: @column
        )

        tokens
      end

      # Indentation-aware tokenization (Python/Starlark style).
      #
      # At each logical line start, count leading spaces, compare to the
      # indent stack, and emit INDENT or DEDENT tokens. Inside brackets
      # (parens, brackets, braces), suppress NEWLINE/INDENT/DEDENT.
      def tokenize_indentation
        tokens = []
        at_line_start = true

        while @pos < @source.length
          char = @source[@pos]

          # Process line start: count indentation, emit INDENT/DEDENT.
          if at_line_start && @bracket_depth == 0
            indent_tokens = process_line_start
            if indent_tokens == :skip_line
              # Blank or comment-only line -- skip entirely.
              next
            end
            tokens.concat(indent_tokens) if indent_tokens
            at_line_start = false
            next if @pos >= @source.length
            char = @source[@pos]
          end

          # Newline handling.
          if char == "\n"
            if @bracket_depth == 0
              tokens << Token.new(
                type: TokenType::NEWLINE, value: "\\n",
                line: @line, column: @column
              )
            end
            do_advance
            at_line_start = true
            next
          end

          # Inside brackets: skip whitespace (implicit line joining).
          if @bracket_depth > 0 && (char == " " || char == "\t" || char == "\r")
            do_advance
            next
          end

          # Try skip patterns.
          next if try_skip

          # Try each token pattern.
          token = try_match_token
          if token
            # Track bracket depth.
            case token.value
            when "(", "[", "{"
              @bracket_depth += 1
            when ")", "]", "}"
              @bracket_depth -= 1
            end
            tokens << token
            next
          end

          raise LexerError.new(
            "Unexpected character: #{char.inspect}",
            line: @line, column: @column
          )
        end

        # EOF: emit remaining DEDENTs.
        while @indent_stack.length > 1
          @indent_stack.pop
          tokens << Token.new(
            type: "DEDENT", value: "",
            line: @line, column: @column
          )
        end

        # Final NEWLINE if the last token isn't one.
        if tokens.empty? || tokens.last.type != TokenType::NEWLINE
          tokens << Token.new(
            type: TokenType::NEWLINE, value: "\\n",
            line: @line, column: @column
          )
        end

        tokens << Token.new(
          type: TokenType::EOF, value: "",
          line: @line, column: @column
        )

        tokens
      end

      # Process the start of a logical line in indentation mode.
      # Returns an array of INDENT/DEDENT tokens, :skip_line for blank/comment
      # lines, or nil if no indent change.
      def process_line_start
        # Count leading spaces. Tabs are not allowed in indentation.
        indent = 0
        while @pos < @source.length
          char = @source[@pos]
          if char == " "
            indent += 1
            do_advance
          elsif char == "\t"
            raise LexerError.new(
              "Tab character in indentation (use spaces only)",
              line: @line, column: @column
            )
          else
            break
          end
        end

        # Blank line or end of file -- skip without emitting NEWLINE.
        return :skip_line if @pos >= @source.length
        if @source[@pos] == "\n"
          do_advance  # Consume the newline to avoid infinite loop.
          return :skip_line
        end

        # Comment-only line -- consume the comment via skip patterns,
        # then check if we're at newline/EOF.
        remaining = @source[@pos..]
        @skip_patterns.each do |pattern|
          m = pattern.match(remaining)
          if m && m.begin(0) == 0
            # Check if after skipping we're at newline or EOF.
            peek_pos = @pos + m[0].length
            if peek_pos >= @source.length || @source[peek_pos] == "\n"
              m[0].length.times { do_advance }
              do_advance if @pos < @source.length && @source[@pos] == "\n"
              return :skip_line
            end
          end
        end

        # Compare indent to current level.
        current_indent = @indent_stack.last
        tokens = []

        if indent > current_indent
          @indent_stack.push(indent)
          tokens << Token.new(
            type: "INDENT", value: "",
            line: @line, column: 1
          )
        elsif indent < current_indent
          while @indent_stack.length > 1 && @indent_stack.last > indent
            @indent_stack.pop
            tokens << Token.new(
              type: "DEDENT", value: "",
              line: @line, column: 1
            )
          end
          unless @indent_stack.last == indent
            raise LexerError.new(
              "Inconsistent dedent (indent level #{indent} does not match any outer level)",
              line: @line, column: 1
            )
          end
        end

        tokens.empty? ? nil : tokens
      end

      # Try to match and consume a skip pattern at the current position.
      # Returns true if something was skipped.
      def try_skip
        remaining = @source[@pos..]
        @skip_patterns.each do |pattern|
          m = pattern.match(remaining)
          if m && m.begin(0) == 0
            m[0].length.times { do_advance }
            return true
          end
        end
        false
      end

      # Try to match a token at the current position.
      # Returns a Token on success, nil on failure.
      def try_match_token
        remaining = @source[@pos..]

        @patterns.each do |token_name, pattern, alias_name|
          m = pattern.match(remaining)
          next unless m && m.begin(0) == 0

          value = m[0]
          start_line = @line
          start_column = @column

          token_type = resolve_token_type(token_name, value, alias_name)

          # Handle STRING tokens: strip quotes and process escapes.
          if token_name == "STRING" || (alias_name && alias_name.include?("STRING"))
            # Only strip if the value is quoted.
            if value.length >= 2 && (value.start_with?('"') || value.start_with?("'"))
              inner = value[1..-2]
              inner = process_escapes(inner)
              value = inner
            end
          end

          result = Token.new(
            type: token_type, value: value,
            line: start_line, column: start_column
          )

          m[0].length.times { do_advance }
          return result
        end

        nil
      end

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

      # Map a token name from the grammar to a token type string.
      #
      # Resolution order:
      # 1. Check reserved keywords -- raise error if matched.
      # 2. Check regular keywords -- return KEYWORD type.
      # 3. Use alias_name if present.
      # 4. Look up in TokenType::ALL.
      # 5. Fall back to the token name as a string (grammar-driven types).
      def resolve_token_type(token_name, value, alias_name)
        # Reserved keyword check.
        if token_name == "NAME" && @reserved_set.include?(value)
          raise LexerError.new(
            "Reserved keyword '#{value}' cannot be used as an identifier",
            line: @line, column: @column
          )
        end

        # Regular keyword check.
        if token_name == "NAME" && @keyword_set.include?(value)
          return TokenType::KEYWORD
        end

        # Alias takes precedence.
        if alias_name
          return TokenType::ALL[alias_name] || alias_name
        end

        # Standard lookup, falling back to the name as a string type.
        TokenType::ALL[token_name] || token_name
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

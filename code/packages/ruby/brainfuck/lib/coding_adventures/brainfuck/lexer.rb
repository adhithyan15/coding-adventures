# frozen_string_literal: true

# ================================================================
# Brainfuck Lexer -- Tokenizes Brainfuck Source from Ruby
# ================================================================
#
# This module demonstrates the grammar-driven approach applied to the
# simplest possible programming language. Brainfuck has exactly 8
# meaningful characters; every other character in the source is a
# comment (ignored by the lexer).
#
# The eight Brainfuck commands are:
#
#   >   RIGHT       — move the data pointer one cell to the right
#   <   LEFT        — move the data pointer one cell to the left
#   +   INC         — increment the byte at the current cell
#   -   DEC         — decrement the byte at the current cell
#   .   OUTPUT      — write the byte at the current cell to stdout
#   ,   INPUT       — read one byte from stdin into the current cell
#   [   LOOP_START  — jump forward past matching ] if cell is zero
#   ]   LOOP_END    — jump back to matching [ if cell is nonzero
#
# Everything else — letters, spaces, punctuation — is treated as a
# comment. The brainfuck.tokens grammar defines these as "skip:"
# patterns, so they are consumed silently and never appear in the
# token stream.
#
# This design keeps the parser grammar (brainfuck.grammar) clean:
# every token the parser sees is a command, never a comment.
#
# Usage:
#   tokens = CodingAdventures::Brainfuck::Lexer.tokenize("++[>+<-]")
#   tokens.each { |t| puts "#{t.type} #{t.value.inspect} @ #{t.line}:#{t.col}" }
# ================================================================

module CodingAdventures
  module Brainfuck
    module Lexer
      # Path to the grammars directory, computed relative to this file.
      #
      # The directory structure looks like this:
      #
      #   code/
      #     grammars/
      #       brainfuck.tokens   <-- we need this file
      #     packages/
      #       ruby/
      #         brainfuck/
      #           lib/
      #             coding_adventures/
      #               brainfuck/
      #                 lexer.rb  <-- we are here (__dir__)
      #
      # Counting up from __dir__:
      #   1. brainfuck/       (coding_adventures/brainfuck -> coding_adventures)
      #   2. coding_adventures/
      #   3. lib/
      #   4. brainfuck/       (ruby package)
      #   5. ruby/
      #   6. packages/
      #
      # So 6 levels up from __dir__ lands at code/.
      GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
      BF_TOKENS_PATH = File.join(GRAMMAR_DIR, "brainfuck.tokens")

      # Tokenize a Brainfuck source string into an array of Token objects.
      #
      # This is the primary entry point. The pipeline is:
      #   1. Read brainfuck.tokens from the grammars directory
      #   2. Parse the token grammar using CodingAdventures::GrammarTools
      #   3. Feed the grammar and source into GrammarLexer
      #   4. Return the resulting flat token array
      #
      # Comments (any non-command character) are silently discarded by
      # the lexer's skip: mechanism, so the returned array contains only
      # the 8 command token types plus a terminal EOF.
      #
      # @param source [String] Brainfuck source code to tokenize
      # @return [Array<CodingAdventures::Lexer::Token>] the command token stream
      def self.tokenize(source)
        # Parse the brainfuck.tokens grammar file into a TokenGrammar struct.
        # The TokenGrammar holds compiled regexes for RIGHT, LEFT, INC, DEC,
        # OUTPUT, INPUT, LOOP_START, LOOP_END, and the two skip patterns
        # (WHITESPACE and COMMENT).
        grammar = CodingAdventures::GrammarTools.parse_token_grammar(
          File.read(BF_TOKENS_PATH, encoding: "UTF-8")
        )

        # Create a GrammarLexer and run it against the source.
        # The lexer walks the source character by character, matching patterns
        # in declaration order (first match wins). When it matches a skip:
        # pattern, it advances the position without emitting a token. When it
        # matches a command pattern, it emits a Token with:
        #   - type:   "RIGHT", "LEFT", "INC", etc.
        #   - value:  the matched character (">", "<", "+", etc.)
        #   - line:   1-based line number
        #   - col:    1-based column number
        lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
        lexer.tokenize
      end

      # Return a GrammarLexer instance loaded with the Brainfuck token grammar.
      #
      # Use this when you need to inspect or reuse the lexer object directly
      # rather than just getting the token array. For most use cases, prefer
      # the simpler tokenize/1 method.
      #
      # @param source [String] Brainfuck source code
      # @return [CodingAdventures::Lexer::GrammarLexer] a ready-to-run lexer
      def self.create_lexer(source)
        grammar = CodingAdventures::GrammarTools.parse_token_grammar(
          File.read(BF_TOKENS_PATH, encoding: "UTF-8")
        )
        CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      end
    end
  end
end

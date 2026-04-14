# frozen_string_literal: true

# ================================================================
# JavaScript Parser -- Parses JavaScript Source Code into ASTs from Ruby
# ================================================================
#
# This module mirrors the javascript_lexer pattern: instead of writing
# a JavaScript-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the JavaScript grammar from javascript.grammar.
#
# The pipeline is:
#
#   1. Read javascript.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read javascript.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# Version-aware usage
# -------------------
# Both the lexer and parser steps can target a specific ECMAScript version:
#
#   # Generic (uses javascript.grammar + javascript.tokens)
#   ast = CodingAdventures::JavascriptParser.parse("let x = 1 + 2;")
#
#   # ES2020 (uses ecmascript/es2020.grammar + ecmascript/es2020.tokens)
#   ast = CodingAdventures::JavascriptParser.parse("let x = 1;", version: "es2020")
#
# Valid versions: "es1", "es3", "es5", "es2015", "es2016", "es2017",
#   "es2018", "es2019", "es2020", "es2021", "es2022", "es2023", "es2024", "es2025"
# Pass nil (or omit) for the generic grammar.
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_javascript_lexer"

module CodingAdventures
  module JavascriptParser
    # Paths to the grammar files, computed relative to this file.
    #
    # Directory structure (counting upward from __dir__):
    #   lib/coding_adventures/javascript_parser/  <- __dir__
    #   lib/coding_adventures/                    <- ..
    #   lib/                                      <- ../..
    #   <gem root>/                               <- ../../..
    #   ruby/                                     <- ../../../..
    #   packages/                                 <- ../../../../..
    #   code/                                     <- ../../../../../..
    #   grammars/                                 <- ../../../../../../grammars
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    JS_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "javascript.grammar")
    COMPILED_GRAMMAR_DIR = __dir__

    # All valid ECMAScript grammar versions supported by the versioned grammar
    # files in code/grammars/ecmascript/.
    VALID_VERSIONS = %w[
      es1 es3 es5
      es2015 es2016 es2017 es2018 es2019 es2020
      es2021 es2022 es2023 es2024 es2025
    ].freeze

    # Resolve the path to the .grammar file for a given ECMAScript version.
    #
    # When `version` is nil or empty, the generic javascript.grammar is used.
    # When a valid version string is given, the versioned file is returned.
    # An unknown version raises ArgumentError immediately.
    #
    # @param version [String, nil] version tag or nil
    # @return [String] absolute path to the .grammar file
    # @raise [ArgumentError] if version is not in VALID_VERSIONS
    def self.resolve_grammar_path(version)
      if version.nil? || version.empty?
        File.join(GRAMMAR_DIR, "javascript.grammar")
      elsif VALID_VERSIONS.include?(version)
        File.join(GRAMMAR_DIR, "ecmascript", "#{version}.grammar")
      else
        raise ArgumentError,
          "Unknown JavaScript/ECMAScript version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end
    end

    def self.resolve_compiled_grammar_path(version)
      resolve_grammar_path(version)

      if version.nil? || version.empty?
        File.join(COMPILED_GRAMMAR_DIR, "_grammar.rb")
      else
        File.join(COMPILED_GRAMMAR_DIR, "_grammar_#{version}.rb")
      end
    end

    def self.parser_grammar(version)
      CodingAdventures::GrammarTools.load_parser_grammar(resolve_compiled_grammar_path(version))
    end

    # Parse a string of JavaScript source code into a generic AST.
    #
    # The optional `version:` keyword argument selects a specific versioned
    # ECMAScript grammar.  The same version is forwarded to the JavaScript
    # lexer so that both the token grammar and the parser grammar match.
    # When omitted (or nil), the generic grammars are used — the same
    # behaviour as version 0.1.0.
    #
    # @param source [String] JavaScript source code to parse
    # @param version [String, nil] ECMAScript version tag (e.g. "es2020") or nil
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    # @raise [ArgumentError] if version is not nil and not in VALID_VERSIONS
    def self.parse(source, version: nil)
      # Step 1: Tokenize using the JavaScript lexer (version-aware)
      tokens = CodingAdventures::JavascriptLexer.tokenize(source, version: version)

      # Step 2: Parse tokens using the compiled grammar for this version.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, parser_grammar(version))
      parser.parse
    end
  end
end

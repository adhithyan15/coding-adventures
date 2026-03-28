# frozen_string_literal: true

# ================================================================
# CSS Lexer — Tokenizes CSS Source Code from Ruby
# ================================================================
#
# CSS tokenization is surprisingly complex for what looks like a
# simple language. The main challenges are:
#
#   1. DIMENSION vs NUMBER: "12px" is a single DIMENSION token,
#      not a NUMBER "12" followed by IDENT "px". Priority order
#      in the grammar handles this: DIMENSION comes before NUMBER.
#
#   2. PERCENTAGE vs NUMBER: "50%" is PERCENTAGE, not NUMBER + PERCENT.
#
#   3. FUNCTION vs IDENT: "rgb(" is a FUNCTION token, not IDENT "rgb"
#      followed by LPAREN. Again, grammar priority handles this.
#
#   4. URL tokens: url("...") and url(...) are single URL tokens
#      with different quoting rules.
#
#   5. Custom properties: --variable-name is a CUSTOM_PROPERTY token.
#
#   6. Vendor prefixes: -webkit-something, -moz-something are
#      IDENT tokens with the prefix included.
#
#   7. Unicode ranges: U+0025-00FF for font subsetting.
#
#   8. Selector operators: >, +, ~, |, ^ as part of selectors.
#
# The grammar-driven approach handles all of this: the css.tokens
# file defines the regex patterns in priority order. We just load it.
#
# Usage:
#   tokens = CodingAdventures::CssLexer.tokenize("color: red;")
#   tokens.each { |t| puts "#{t.type}: #{t.value}" }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module CssLexer
    # Path to the grammars directory, relative to this file.
    # Structure:
    #   code/grammars/css.tokens              <-- target
    #   code/packages/ruby/css_lexer/lib/
    #     coding_adventures/css_lexer/
    #       tokenizer.rb                      <-- __dir__
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    CSS_TOKENS_PATH = File.join(GRAMMAR_DIR, "css.tokens")

    # Create a GrammarLexer configured for CSS text.
    #
    # Reads css.tokens, parses it into a TokenGrammar, and creates
    # a GrammarLexer ready to tokenize the given source.
    #
    # @param source [String] CSS source code to tokenize
    # @return [CodingAdventures::Lexer::GrammarLexer]
    def self.create_css_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(CSS_TOKENS_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
    end

    # Tokenize a string of CSS source code into an array of Token objects.
    #
    # This is the main entry point. The returned tokens include:
    #
    # - **DIMENSION** — a number with a unit (e.g., "12px", "1.5em")
    # - **PERCENTAGE** — a number with a percent sign (e.g., "50%")
    # - **FUNCTION** — function name with opening paren (e.g., "rgb(")
    # - **IDENT** — identifier (e.g., "color", "red", "-webkit-box")
    # - **CUSTOM_PROPERTY** — CSS custom property name (e.g., "--color")
    # - **STRING** — quoted string (e.g., '"serif"', "'sans-serif'")
    # - **NUMBER** — bare number (e.g., "0", "12", "3.14")
    # - **HASH** — hash value (e.g., "#fff", "#336699")
    # - **URL** — url token (e.g., 'url("image.png")')
    # - **COLON**, **SEMICOLON**, **LBRACE**, **RBRACE** — punctuation
    # - **AT_KEYWORD** — @-rule keyword (e.g., "@media", "@import")
    # - **COMMENT** — CSS comment /* ... */ (not skipped)
    # - **WHITESPACE** — significant whitespace
    # - **EOF** — end of input
    #
    # @param source [String] CSS source code to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      create_css_lexer(source).tokenize
    end
  end
end

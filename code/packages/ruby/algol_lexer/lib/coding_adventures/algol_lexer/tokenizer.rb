# frozen_string_literal: true

# ================================================================
# ALGOL 60 Lexer -- Tokenizes ALGOL 60 Source Text from Ruby
# ================================================================
#
# ALGOL 60 (ALGOrithmic Language, 1960) is one of the most historically
# significant programming languages ever designed. It was the first language
# specified using BNF (Backus-Naur Form), introduced block structure and
# lexical scoping, and gave us the call stack and recursion as first-class
# language features. Every modern imperative language — Pascal, C, Ada,
# Simula (the first OOP language), Java, Rust, Go — descends from ALGOL.
#
# ALGOL 60's lexical design is richer than JSON's but simpler than Python's:
#
#   Value tokens:
#     REAL_LIT     3.14  1.5E3  100E2  (must precede INTEGER_LIT)
#     INTEGER_LIT  0  42  1000
#     STRING_LIT   'hello'  'x = 5'  ''
#     IDENT        x  sum  customerName  A1
#
#   Keyword tokens (reclassified from IDENT):
#     begin  end  if  then  else  for  do  step  until  while  goto
#     switch  procedure  own  array  label  value
#     integer  real  boolean  string
#     true  false
#     not  and  or  impl  eqv  div  mod
#     comment  (triggers comment-skip mode in the lexer)
#
#   Multi-character operators (must precede single-char):
#     ASSIGN  :=      POWER  **      LEQ  <=     GEQ  >=     NEQ  !=
#
#   Single-character operators:
#     PLUS  +   MINUS  -   STAR  *   SLASH  /   CARET  ^
#     EQ    =   LT     <   GT    >
#
#   Delimiters:
#     LPAREN  (   RPAREN  )   LBRACKET  [   RBRACKET  ]
#     SEMICOLON  ;   COMMA  ,   COLON  :
#
#   Skipped silently:
#     WHITESPACE  spaces, tabs, carriage returns, newlines
#     COMMENT     comment <text up to ;>
#
# Comment syntax in ALGOL 60:
#   comment this is a comment; x := 1
#
# The word "comment" begins a comment block; everything up to and
# including the next semicolon is consumed silently. This is unusual
# compared to C's // and /* */ — ALGOL's comments are terminated
# by semicolons, which means a comment "looks like" a statement.
#
# Usage:
#   tokens = CodingAdventures::AlgolLexer.tokenize('begin integer x; x := 42 end')
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module AlgolLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/algol_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       algol.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         algol_lexer/
    #           lib/
    #             coding_adventures/
    #               algol_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    VALID_VERSIONS = %w[algol60].freeze
    ALGOL_TOKENS_PATH = File.join(GRAMMAR_DIR, "algol", "algol60.tokens")

    def self.resolve_tokens_path(version = "algol60")
      if version.nil? || version.empty?
        version = "algol60"
      end

      unless VALID_VERSIONS.include?(version)
        raise ArgumentError,
          "Unknown ALGOL version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end

      File.join(GRAMMAR_DIR, "algol", "#{version}.tokens")
    end

    # Tokenize a string of ALGOL 60 source text into an array of Token objects.
    #
    # This is the main entry point. It:
    # 1. Reads the algol.tokens grammar file
    # 2. Parses it into a TokenGrammar using grammar_tools
    # 3. Feeds the grammar and source into GrammarLexer
    # 4. Returns the resulting token array
    #
    # Keywords are reclassified from IDENT after a full-token match.
    # This means that "begin" produces a BEGIN token, but "beginning"
    # produces an IDENT token because the keyword only matches whole tokens.
    # (The grammar marks ALGOL keywords as case-insensitive per the
    # ALGOL 60 report: BEGIN, Begin, and begin are all equivalent.)
    #
    # Comments are consumed silently. The lexer recognises the pattern
    # /comment[^;]*;/ and skips it, so no COMMENT tokens appear in the output.
    #
    # @param source [String] ALGOL 60 source text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source, version: "algol60")
      # Read the algol.tokens file and parse it into a TokenGrammar.
      # The TokenGrammar contains the regex patterns for REAL_LIT, INTEGER_LIT,
      # STRING_LIT, IDENT, all operators and delimiters, the keyword list,
      # and the skip rules for whitespace and comments.
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(resolve_tokens_path(version), encoding: "UTF-8")
      )

      # Create a GrammarLexer instance and run it. The lexer walks through
      # the source character by character, matching patterns from the grammar
      # in priority order (first match wins). Multi-character operators like
      # := and ** are defined before their single-character prefixes (: and *)
      # so that ":=" tokenizes as ASSIGN rather than COLON + EQ.
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
      lexer.tokenize
    end
  end
end

# frozen_string_literal: true

# ================================================================
# SQL Lexer -- Tokenizes SQL Text from Ruby
# ================================================================
#
# This module is a thin wrapper around the grammar-driven GrammarLexer
# engine, feeding it the SQL token definitions from sql.tokens.
#
# SQL (Structured Query Language, ANSI SQL subset) has significantly
# more structure than JSON. Where JSON has 9 token types, SQL has:
#
#   - Identifiers:  NAME (unquoted), QUOTED_ID (backtick-quoted)
#   - Literals:     NUMBER (integers and decimals), STRING (single-quoted)
#   - Operators:    EQUALS, NOT_EQUALS, LESS_THAN, GREATER_THAN,
#                   LESS_EQUALS, GREATER_EQUALS, PLUS, MINUS,
#                   STAR, SLASH, PERCENT
#   - Punctuation:  LPAREN, RPAREN, COMMA, SEMICOLON, DOT
#   - Keywords:     SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER,
#                   LIMIT, OFFSET, INSERT, INTO, VALUES, UPDATE, SET,
#                   DELETE, CREATE, DROP, TABLE, IF, EXISTS, NOT, AND,
#                   OR, NULL, IS, IN, BETWEEN, LIKE, AS, DISTINCT, ALL,
#                   UNION, INTERSECT, EXCEPT, JOIN, INNER, LEFT, RIGHT,
#                   OUTER, CROSS, FULL, ON, ASC, DESC, TRUE, FALSE,
#                   CASE, WHEN, THEN, ELSE, END, PRIMARY, KEY, UNIQUE,
#                   DEFAULT
#   - Skipped:      WHITESPACE, LINE_COMMENT (--), BLOCK_COMMENT (/* */)
#
# Case Insensitivity
# ------------------
#
# The sql.tokens grammar includes the directive:
#
#   # @case_insensitive true
#
# This means the GrammarLexer normalizes keyword values to uppercase.
# So "select", "SELECT", and "Select" all produce:
#
#   Token(type: "KEYWORD", value: "SELECT")
#
# The GrammarLexer reads @case_insensitive from the grammar struct
# and applies it automatically -- no call to set_case_insensitive
# is needed here.
#
# Token Aliases
# -------------
#
# sql.tokens defines two aliased token types:
#
#   STRING_SQ = /'([^'\\]|\\.)*'/ -> STRING
#   QUOTED_ID = /`[^`]+`/         -> NAME
#   NEQ_ANSI  = "<>"              -> NOT_EQUALS
#
# This means single-quoted strings produce STRING tokens (same type
# as any string), and backtick-quoted identifiers produce NAME tokens.
# The NOT_EQUALS type is used for both "!=" and "<>".
#
# Note: For QUOTED_ID (backtick-quoted identifiers), the backticks
# are NOT stripped. The value includes the backticks (e.g., "`foo`").
# This is different from STRING_SQ where single quotes ARE stripped.
#
# Usage:
#   lexer  = CodingAdventures::SqlLexer.create_sql_lexer("SELECT 1")
#   tokens = CodingAdventures::SqlLexer.tokenize_sql("SELECT 1")
#   tokens.each { |t| puts "#{t.type}: #{t.value}" }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module SqlLexer
    # Path to the grammars directory, computed relative to this file.
    # We navigate up from lib/coding_adventures/sql_lexer/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       sql.tokens    <-- we need this file
    #     packages/
    #       ruby/
    #         sql_lexer/
    #           lib/
    #             coding_adventures/
    #               sql_lexer/
    #                 tokenizer.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    SQL_TOKENS_PATH = File.join(GRAMMAR_DIR, "sql.tokens")

    # Create a GrammarLexer configured for SQL text.
    #
    # This is the lower-level entry point for users who want a lexer
    # object they can configure further (e.g., to register callbacks).
    # For most use cases, prefer tokenize_sql which calls this method
    # and immediately runs the lexer.
    #
    # Steps:
    # 1. Reads sql.tokens from the grammars directory.
    # 2. Parses it into a TokenGrammar using grammar_tools.
    # 3. Creates a GrammarLexer instance with source and grammar.
    #
    # The GrammarLexer picks up @case_insensitive true from the
    # TokenGrammar struct and normalizes keyword values to uppercase
    # automatically.
    #
    # @param source [String] SQL text to tokenize
    # @return [CodingAdventures::Lexer::GrammarLexer] configured lexer
    # @raise [StandardError] if sql.tokens cannot be read or parsed
    def self.create_sql_lexer(source)
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(SQL_TOKENS_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Lexer::GrammarLexer.new(source, grammar)
    end

    # Tokenize a string of SQL text into an array of Token objects.
    #
    # This is the main entry point. It creates a GrammarLexer via
    # create_sql_lexer and immediately runs tokenize, returning the
    # flat token array.
    #
    # Token types you will see:
    #
    # **Keyword tokens** (type = "KEYWORD", value = uppercase string):
    #   SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER, LIMIT, OFFSET,
    #   INSERT, INTO, VALUES, UPDATE, SET, DELETE, CREATE, DROP, TABLE,
    #   IF, EXISTS, NOT, AND, OR, NULL, IS, IN, BETWEEN, LIKE, AS,
    #   DISTINCT, ALL, UNION, INTERSECT, EXCEPT, JOIN, INNER, LEFT,
    #   RIGHT, OUTER, CROSS, FULL, ON, ASC, DESC, TRUE, FALSE, CASE,
    #   WHEN, THEN, ELSE, END, PRIMARY, KEY, UNIQUE, DEFAULT
    #
    # **Identifier tokens** (type = "NAME"):
    #   - Unquoted identifiers:    column_name, table1, etc.
    #   - Backtick-quoted:         `foo`, `my table` (backticks kept)
    #
    # **Literal tokens**:
    #   - NUMBER:  42, 3.14, 0, 100
    #   - STRING:  'hello' (single quotes stripped, value = "hello")
    #
    # **Operator tokens** (type = type_name string):
    #   EQUALS (=), NOT_EQUALS (!= or <>), LESS_THAN (<),
    #   GREATER_THAN (>), LESS_EQUALS (<=), GREATER_EQUALS (>=),
    #   PLUS (+), MINUS (-), STAR (*), SLASH (/), PERCENT (%)
    #
    # **Punctuation tokens**:
    #   LPAREN ((), RPAREN ()), COMMA (,), SEMICOLON (;), DOT (.)
    #
    # **Skipped** (not in token stream):
    #   WHITESPACE, LINE_COMMENT (--...\n), BLOCK_COMMENT (/* ... */)
    #
    # **Always present**:
    #   EOF -- end of input
    #
    # @param source [String] SQL text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize_sql(source)
      create_sql_lexer(source).tokenize
    end
  end
end

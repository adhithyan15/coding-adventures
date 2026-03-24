# frozen_string_literal: true

# ================================================================
# SQL Parser -- Parses SQL Text into ASTs from Ruby
# ================================================================
#
# This module mirrors the sql_lexer pattern: instead of writing
# a SQL-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the SQL grammar from sql.grammar.
#
# The pipeline is:
#
#   1. Read sql.tokens -> build TokenGrammar -> GrammarLexer -> tokens
#   2. Read sql.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# The same two grammar files that describe SQL's syntax are all
# that's needed to go from raw SQL text to an Abstract Syntax Tree.
# No SQL-specific code is involved in the parsing itself -- just
# data files and generic engines.
#
# SQL's grammar (sql.grammar) supports an ANSI SQL subset:
#
#   program           = statement { ";" statement } [ ";" ]
#   statement         = select_stmt | insert_stmt | update_stmt
#                     | delete_stmt | create_table_stmt | drop_table_stmt
#
# SELECT statements support:
#   - DISTINCT / ALL qualifier
#   - Expression select lists with optional AS aliases
#   - FROM with table references and optional AS aliases
#   - JOIN clauses (INNER, LEFT, RIGHT, CROSS, FULL OUTER)
#   - WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET clauses
#
# Expressions support:
#   - Boolean operators: OR, AND, NOT
#   - Comparisons: =, !=/<>, <, >, <=, >=
#   - Range tests: BETWEEN ... AND ...
#   - Set membership: IN (...)
#   - Pattern matching: LIKE
#   - NULL tests: IS NULL, IS NOT NULL
#   - Arithmetic: +, -, *, /, %
#   - Function calls: name(args)
#   - Column references: table.column
#   - Primary values: NUMBER, STRING, NULL, TRUE, FALSE
#
# Case Insensitivity
# ------------------
#
# Because sql.tokens has @case_insensitive true, the sql_lexer
# normalizes all keyword values to uppercase. The grammar file
# references keywords as quoted uppercase strings (e.g., "SELECT"),
# and the parser matches them exactly against the normalized tokens.
#
# This is the fundamental promise of grammar-driven language tooling:
# support a new language by writing grammar files, not code.
#
# Usage:
#   ast = CodingAdventures::SqlParser.parse_sql("SELECT id FROM users")
#   # => ASTNode(rule_name: "program", children: [...])
#
#   lexer, ast = CodingAdventures::SqlParser.create_sql_parser("SELECT 1")
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_sql_lexer"

module CodingAdventures
  module SqlParser
    # Paths to the grammar files, computed relative to this file.
    # We navigate up from lib/coding_adventures/sql_parser/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       sql.grammar   <-- we need this file
    #     packages/
    #       ruby/
    #         sql_parser/
    #           lib/
    #             coding_adventures/
    #               sql_parser/
    #                 parser.rb  <-- we are here (__dir__)
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    SQL_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "sql.grammar")

    # Create a GrammarDrivenParser configured for SQL text.
    #
    # This is the lower-level entry point. It:
    # 1. Tokenizes the source using SqlLexer (which loads sql.tokens)
    # 2. Reads the sql.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Returns a GrammarDrivenParser ready to call parse
    #
    # @param source [String] SQL text to parse
    # @return [CodingAdventures::Parser::GrammarDrivenParser] configured parser
    def self.create_sql_parser(source)
      tokens = CodingAdventures::SqlLexer.tokenize_sql(source)
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(SQL_GRAMMAR_PATH, encoding: "UTF-8")
      )
      CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
    end

    # Parse a string of SQL text into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using SqlLexer (which loads sql.tokens)
    # 2. Reads the sql.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # The root node of the AST will have rule_name "program" (as
    # defined in sql.grammar's first rule). Each child node corresponds
    # to a grammar rule match, and leaf nodes are tokens.
    #
    # Because the sql_lexer normalizes keywords to uppercase, the
    # grammar's keyword literals (e.g., "SELECT") always match
    # regardless of how the user wrote them ("select", "SELECT", etc.).
    #
    # @param source [String] SQL text to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse_sql(source)
      create_sql_parser(source).parse
    end
  end
end

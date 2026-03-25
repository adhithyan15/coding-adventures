# frozen_string_literal: true

# ================================================================
# coding_adventures_sql_parser -- Main Entry Point
# ================================================================
#
# This file is the top-level require for the sql_parser gem.
# It wires together the version and the parser.
#
# After requiring this file, the public API is:
#
#   CodingAdventures::SqlParser.create_sql_parser(source)
#     -> CodingAdventures::Parser::GrammarDrivenParser
#
#   CodingAdventures::SqlParser.parse_sql(source)
#     -> CodingAdventures::Parser::ASTNode
#
# The parser uses sql.grammar (an ANSI SQL subset grammar) together
# with the sql_lexer gem (which applies sql.tokens and normalizes
# keywords to uppercase via @case_insensitive true).
# ================================================================

require_relative "sql_parser/version"
require_relative "sql_parser/parser"

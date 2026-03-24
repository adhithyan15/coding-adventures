# frozen_string_literal: true

# ================================================================
# coding_adventures_sql_lexer -- Main Entry Point
# ================================================================
#
# This file is the top-level require for the sql_lexer gem.
# It wires together the version and the tokenizer.
#
# After requiring this file, the public API is:
#
#   CodingAdventures::SqlLexer.create_sql_lexer(source)
#     -> CodingAdventures::Lexer::GrammarLexer
#
#   CodingAdventures::SqlLexer.tokenize_sql(source)
#     -> Array<CodingAdventures::Lexer::Token>
#
# The tokenizer uses sql.tokens (an ANSI SQL subset token grammar)
# with @case_insensitive true, so keyword values are normalized to
# uppercase regardless of how they appear in the source.
# ================================================================

require_relative "sql_lexer/version"
require_relative "sql_lexer/tokenizer"

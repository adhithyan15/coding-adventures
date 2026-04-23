# frozen_string_literal: true

# ================================================================
# ALGOL 60 Parser -- Parses ALGOL 60 Source Text into ASTs
# ================================================================
#
# This module mirrors the json_parser pattern: instead of writing
# an ALGOL-specific parser from scratch, we reuse the general-purpose
# GrammarDrivenParser engine from the coding_adventures_parser gem,
# feeding it the ALGOL 60 grammar from algol.grammar.
#
# The pipeline is:
#
#   1. Read algol.tokens  -> build TokenGrammar  -> GrammarLexer -> tokens
#   2. Read algol.grammar -> build ParserGrammar -> GrammarDrivenParser -> AST
#
# ALGOL 60's grammar is significantly richer than JSON's. The grammar
# (algol.grammar) captures the full language:
#
#   Top level:
#     program = block
#     block   = BEGIN { declaration ; } statement { ; statement } END
#
#   Declarations:
#     type_decl, array_decl, switch_decl, procedure_decl
#
#   Statements:
#     assign_stmt, goto_stmt, proc_stmt, compound_stmt,
#     cond_stmt (if/then/else), for_stmt, empty_stmt
#
#   Expressions:
#     arith_expr, bool_expr — both support conditional form (if/then/else)
#     Arithmetic operator precedence: ** > * / div mod > + -
#     Boolean operator precedence: eqv > impl > or > and > not
#
# ALGOL 60's grammar design decisions worth noting:
#
#   DANGLING ELSE RESOLUTION:
#     ALGOL 60 eliminates the dangling-else ambiguity at the grammar level.
#     The then-branch is 'unlabeled_stmt' (which excludes cond_stmt), so a
#     conditional cannot appear as a then-branch without being wrapped in
#     begin...end. In C and Java, the else binds to the nearest if by
#     convention; ALGOL makes it a grammar error.
#
#   LEFT-ASSOCIATIVE EXPONENTIATION:
#     Per the ALGOL 60 report, exponentiation is LEFT-associative:
#     2^3^4 = (2^3)^4 = 8^4 = 4096.
#     This differs from most modern languages and mathematical convention
#     (which use right-associativity). The grammar reflects this with
#     { (CARET | POWER) primary } (repetition, left-to-right).
#
#   CALL-BY-NAME DEFAULT:
#     Parameters not listed in a VALUE declaration are passed by name.
#     Call-by-name means the argument expression is re-evaluated every time
#     the parameter is used inside the procedure body — very different from
#     call-by-value. The grammar's value_part and spec_part rules capture
#     this distinction.
#
# This is the fundamental promise of grammar-driven language tooling:
# support a new language by writing grammar files, not code.
#
# Usage:
#   ast = CodingAdventures::AlgolParser.parse('begin integer x; x := 42 end')
#   # => ASTNode(rule_name: "program", children: [...])
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_algol_lexer"

module CodingAdventures
  module AlgolParser
    # Paths to the grammar files, computed relative to this file.
    # We navigate up from lib/coding_adventures/algol_parser/ to the
    # repository root's code/grammars/ directory.
    #
    # The directory structure looks like this:
    #   code/
    #     grammars/
    #       algol.grammar   <-- we need this file
    #     packages/
    #       ruby/
    #         algol_parser/
    #           lib/
    #             coding_adventures/
    #               algol_parser/
    #                 parser.rb  <-- we are here (__dir__)
    #
    # So from __dir__ we go up 6 levels to reach code/, then into grammars/.
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    VALID_VERSIONS = %w[algol60].freeze
    ALGOL_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "algol", "algol60.grammar")

    def self.resolve_grammar_path(version = "algol60")
      if version.nil? || version.empty?
        version = "algol60"
      end

      unless VALID_VERSIONS.include?(version)
        raise ArgumentError,
          "Unknown ALGOL version #{version.inspect}. " \
          "Valid versions: #{VALID_VERSIONS.sort.join(", ")}"
      end

      File.join(GRAMMAR_DIR, "algol", "#{version}.grammar")
    end

    # Parse a string of ALGOL 60 source text into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using AlgolLexer (which loads algol.tokens)
    # 2. Reads the algol.grammar file
    # 3. Parses the grammar into a ParserGrammar
    # 4. Feeds the tokens and grammar into GrammarDrivenParser
    # 5. Returns the resulting AST
    #
    # The root node of the AST will have rule_name "program" (as defined
    # in algol.grammar's first rule). Each child node corresponds to a
    # grammar rule match, and leaf nodes are tokens.
    #
    # @param source [String] ALGOL 60 source text to parse
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    def self.parse(source, version: "algol60")
      # Step 1: Tokenize using the ALGOL 60 lexer.
      # This loads algol.tokens and produces a flat token stream.
      # Comments (comment...;) and whitespace are consumed silently.
      # Keywords like "begin" and "integer" are reclassified from IDENT.
      tokens = CodingAdventures::AlgolLexer.tokenize(source, version: version)

      # Step 2: Load and parse the ALGOL 60 grammar.
      # The grammar file uses EBNF notation to describe ALGOL 60's
      # block structure, declarations, statements, and expressions.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(resolve_grammar_path(version), encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser.
      # The parser uses recursive descent with backtracking to match
      # the token stream against the grammar rules, producing an AST
      # where each node records which rule produced it.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end

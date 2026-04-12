# frozen_string_literal: true

# ================================================================
# Dartmouth BASIC 1964 Parser -- Parses BASIC Source Text into ASTs
# ================================================================
#
# This module mirrors the json_parser pattern: instead of writing a
# BASIC-specific recursive descent parser from scratch, we reuse the
# general-purpose GrammarDrivenParser engine from the
# coding_adventures_parser gem, feeding it the Dartmouth BASIC grammar
# from dartmouth_basic.grammar.
#
# The pipeline is:
#
#   1. Read dartmouth_basic.grammar -> ParserGrammar
#   2. Tokenize source via DartmouthBasicLexer -> token stream
#   3. Feed tokens + grammar into GrammarDrivenParser -> AST
#
# The same grammar file that defines BASIC's syntax is used by all
# language implementations: Python, Ruby, Go, Rust, TypeScript, Elixir.
# One grammar file; many parsers. This is the promise of grammar-driven
# language tooling.
#
# ----------------------------------------------------------------
# WHAT IS 1964 DARTMOUTH BASIC?
# ----------------------------------------------------------------
#
# BASIC (Beginner's All-purpose Symbolic Instruction Code) was designed
# by John Kemeny and Thomas Kurtz at Dartmouth College in 1964. Their
# goal was radical for the era: make interactive computing available to
# EVERY student on campus, not just computer science majors.
#
# The original system ran on a GE-225 mainframe connected to 30
# teletypes via the Dartmouth Time-Sharing System (DTSS). A student
# could log in, type a BASIC program, run it, and get results — all
# within minutes, without waiting for a batch job.
#
# The language was deliberately simple:
#   - 20 keywords (LET, PRINT, INPUT, IF, GOTO, GOSUB, RETURN, ...)
#   - 11 built-in math functions (SIN, COS, LOG, SQR, INT, RND, ...)
#   - Line-numbered statements (every line starts with an integer)
#   - No type declarations — every variable is a number (or FNx function)
#   - No block structure — IF has no THEN body, only a GOTO-style branch
#
# The influence was enormous. By 1980, almost every microcomputer shipped
# with a BASIC interpreter. Microsoft's first product was a BASIC for the
# Altair 8800. Apple II, Commodore 64, BBC Micro, TRS-80 — all had BASIC.
# A generation of programmers learned to code in this language.
#
# ----------------------------------------------------------------
# GRAMMAR-DRIVEN PARSING
# ----------------------------------------------------------------
#
# A grammar file describes a language's syntax in EBNF notation:
#
#   program  = { line } ;
#   line     = LINE_NUM [ statement ] NEWLINE ;
#   let_stmt = "LET" variable EQ expr ;
#   expr     = term { ( PLUS | MINUS ) term } ;
#   term     = power { ( STAR | SLASH ) power } ;
#   power    = unary [ CARET power ] ;          # right-associative!
#   unary    = MINUS primary | primary ;
#   primary  = NUMBER | BUILTIN_FN LPAREN expr RPAREN | ...
#
# The GrammarDrivenParser reads these rules and applies them to the
# token stream recursively. No BASIC-specific parsing code is needed —
# just the grammar file and the generic engine.
#
# The CARET rule uses right-recursion (`power = unary [ CARET power ]`)
# to achieve right-associativity: 2^3^2 = 2^(3^2) = 512, not 64.
#
# ----------------------------------------------------------------
# HOW TO READ THE AST
# ----------------------------------------------------------------
#
# Every node in the returned tree is an ASTNode with:
#   - rule_name: which grammar rule produced this node (e.g. "let_stmt")
#   - children:  array of ASTNode or Token objects
#
# Example: parsing "10 LET X = 5\n" produces roughly:
#
#   ASTNode(rule_name: "program")
#     ASTNode(rule_name: "line")
#       Token(LINE_NUM, "10")
#       ASTNode(rule_name: "statement")
#         ASTNode(rule_name: "let_stmt")
#           Token(KEYWORD, "LET")
#           ASTNode(rule_name: "variable") -> Token(NAME, "X")
#           Token(EQ, "=")
#           ASTNode(rule_name: "expr") -> ... -> Token(NUMBER, "5")
#       Token(NEWLINE, "\n")
#
# ----------------------------------------------------------------
# USAGE
# ----------------------------------------------------------------
#
#   ast = CodingAdventures::DartmouthBasicParser.parse(source)
#   # => ASTNode(rule_name: "program", children: [...])
#
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_parser"
require "coding_adventures_dartmouth_basic_lexer"

module CodingAdventures
  module DartmouthBasicParser
    # Paths to the grammar file, computed relative to this file.
    # We navigate up from lib/coding_adventures/dartmouth_basic_parser/
    # to the repository root's code/grammars/ directory.
    #
    # Directory structure:
    #   code/
    #     grammars/
    #       dartmouth_basic.grammar   <-- we need this file
    #     packages/
    #       ruby/
    #         dartmouth_basic_parser/
    #           lib/
    #             coding_adventures/
    #               dartmouth_basic_parser/
    #                 parser.rb  <-- we are here (__dir__)
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    DARTMOUTH_BASIC_GRAMMAR_PATH = File.join(GRAMMAR_DIR, "dartmouth_basic.grammar")

    # Parse a string of Dartmouth BASIC 1964 source code into a generic AST.
    #
    # This is the main entry point. It:
    # 1. Tokenizes the source using DartmouthBasicLexer (which loads
    #    dartmouth_basic.tokens and applies two post-tokenize hooks:
    #    - LINE_NUM relabeling: the first integer on each source line is
    #      relabeled from NUMBER to LINE_NUM.
    #    - REM suppression: everything between REM and NEWLINE is stripped.)
    # 2. Reads the dartmouth_basic.grammar file.
    # 3. Parses the grammar into a ParserGrammar.
    # 4. Feeds the tokens and grammar into GrammarDrivenParser.
    # 5. Returns the resulting AST.
    #
    # The root node of the AST will have rule_name "program" (as defined
    # in dartmouth_basic.grammar's first rule). Each child node corresponds
    # to a grammar rule match, and leaf nodes are tokens.
    #
    # @param source [String] Dartmouth BASIC source text to parse.
    #   Each statement must be on its own numbered line, terminated by
    #   a newline character. The language is case-insensitive: "let",
    #   "LET", and "Let" all produce the same tokens.
    # @return [CodingAdventures::Parser::ASTNode] the root AST node
    # @raise [StandardError] if source has syntax errors
    def self.parse(source)
      # Step 1: Tokenize the BASIC source.
      # The lexer handles BASIC-specific quirks: LINE_NUM labeling (the
      # first integer on each line gets type LINE_NUM, not NUMBER) and
      # REM suppression (comment text is stripped before the parser sees it).
      tokens = CodingAdventures::DartmouthBasicLexer.tokenize(source)

      # Step 2: Load and parse the Dartmouth BASIC grammar.
      # This grammar file defines all 17 statement types and the full
      # expression precedence hierarchy.
      grammar = CodingAdventures::GrammarTools.parse_parser_grammar(
        File.read(DARTMOUTH_BASIC_GRAMMAR_PATH, encoding: "UTF-8")
      )

      # Step 3: Parse tokens using the grammar-driven parser.
      # The parser applies recursive descent with backtracking against the
      # grammar rules, producing an ASTNode tree where each node records
      # which grammar rule matched it.
      parser = CodingAdventures::Parser::GrammarDrivenParser.new(tokens, grammar)
      parser.parse
    end
  end
end

# frozen_string_literal: true

# ================================================================
# Dartmouth BASIC Lexer -- Tokenizes 1964 Dartmouth BASIC Source Text
# ================================================================
#
# Dartmouth BASIC was created in 1964 by John Kemeny and Thomas Kurtz
# at Dartmouth College. It ran on a GE-225 mainframe connected to
# teletypes. The design goals were radical for the time:
#
#   - Accessible to complete beginners (non-science students)
#   - Line-numbered: every statement lives on a numbered line
#   - Interactive: results print as the program runs
#   - Forgiving: every variable is pre-initialized to 0
#
# The language that became BASIC was so influential that by the 1970s
# and 1980s almost every home computer shipped with a BASIC interpreter.
# Microsoft's first product was a BASIC interpreter for the Altair 8800.
#
# ----------------------------------------------------------------
# WHAT A LEXER DOES
# ----------------------------------------------------------------
#
# A lexer (also called a tokenizer or scanner) is the first stage of
# a language processing pipeline:
#
#   Source text → [LEXER] → Token stream → [PARSER] → AST → ...
#
# The lexer breaks raw characters into meaningful chunks called tokens.
# For "10 LET X = 5\n", the lexer produces:
#
#   LINE_NUM("10")   ← this line is numbered 10
#   KEYWORD("LET")   ← reserved word
#   NAME("X")        ← variable name
#   EQ("=")          ← equals sign
#   NUMBER("5")      ← numeric literal
#   NEWLINE          ← end of this statement
#
# The parser then reads this token stream and builds a tree structure.
# By doing lexing and parsing as two separate passes, each stage is
# simpler and easier to test independently.
#
# ----------------------------------------------------------------
# WHY CASE-INSENSITIVE?
# ----------------------------------------------------------------
#
# The GE-225 teletypes of 1964 had no lowercase letters. Every program
# was typed in uppercase. When the grammar declares @case_insensitive true,
# the lexer normalises all input to uppercase before matching. This means:
#
#   "print x"  →  "PRINT X"  →  KEYWORD("PRINT"), NAME("X")
#   "PRINT X"  →  "PRINT X"  →  KEYWORD("PRINT"), NAME("X")
#   "Print X"  →  "PRINT X"  →  KEYWORD("PRINT"), NAME("X")
#
# All three produce identical token streams. This is historically correct:
# the original system had no lowercase, so "print" and "PRINT" were the
# same thing.
#
# ----------------------------------------------------------------
# THE LINE_NUM PROBLEM
# ----------------------------------------------------------------
#
# Dartmouth BASIC has a peculiar structure: every statement begins with
# a line number:
#
#   10 LET X = 5
#   20 PRINT X
#   30 GOTO 10
#
# Line numbers serve two purposes:
#   1. Addressing: the interpreter sorts all lines by number before
#      running, so you can type lines in any order and they execute
#      in numeric order.
#   2. Branching: GOTO 30 and GOSUB 100 use line numbers as targets.
#
# The challenge: a line number like "10" looks exactly like a NUMBER
# token containing the integer 10. Both the lexer and the parser need
# to know which "10" is a line label vs which is a numeric value.
#
# Our solution: the grammar uses LINE_NUM and NUMBER with the same
# regex. After the base lexer runs, we apply a post-tokenize hook
# (relabel_line_numbers) that walks the token list and relabels the
# first NUMBER token on each line as LINE_NUM. This is position-based
# disambiguation: the token at the start of each line (position 0,
# or immediately after NEWLINE) is the line number.
#
# ----------------------------------------------------------------
# THE REM PROBLEM
# ----------------------------------------------------------------
#
# REM (short for REMark) introduces a comment that runs to the end
# of the physical line. Everything after REM is human-readable text
# and should not be tokenised:
#
#   10 REM THIS IS A COMMENT
#   20 LET X = 5  REM SO IS THIS
#
# The grammar cannot express "consume until newline" in a pure
# regex. Instead we use a second post-tokenize hook (suppress_rem_content)
# that filters out any tokens between REM and the next NEWLINE.
#
# After both hooks, the stream for "10 REM THIS IS A COMMENT\n" is:
#   LINE_NUM("10"), KEYWORD("REM"), NEWLINE
#
# ----------------------------------------------------------------
# TOKEN TYPES PRODUCED
# ----------------------------------------------------------------
#
#   LINE_NUM    "10", "100"     — integer at start of line
#   NUMBER      "42", "3.14"   — numeric literal in expression
#   STRING      "HELLO"        — string content (quotes stripped by lexer)
#   KEYWORD     "PRINT", "LET" — reserved word (always uppercase)
#   BUILTIN_FN  "SIN", "COS"   — one of the 11 built-in math functions
#   USER_FN     "FNA", "FNZ"   — FN + one letter (user-defined function)
#   NAME        "X", "A1"      — variable: 1 letter + optional digit
#   PLUS        "+"            — addition
#   MINUS       "-"            — subtraction / unary minus
#   STAR        "*"            — multiplication
#   SLASH       "/"            — division
#   CARET       "^"            — exponentiation (right-associative)
#   EQ          "="            — assignment (LET X = 5) or equality (IF X = 5)
#   LT          "<"            — less than
#   GT          ">"            — greater than
#   LE          "<="           — less than or equal
#   GE          ">="           — greater than or equal
#   NE          "<>"           — not equal
#   LPAREN      "("            — open paren
#   RPAREN      ")"            — close paren
#   COMMA       ","            — print zone separator
#   SEMICOLON   ";"            — tight print separator (no space)
#   NEWLINE     "\n"           — statement terminator
#   EOF                        — end of token stream
#   UNKNOWN     "@"            — unrecognised character (error recovery)
#
# Usage:
#   tokens = CodingAdventures::DartmouthBasicLexer.tokenize("10 PRINT \"HELLO\"")
#   tokens.each { |t| puts t }
# ================================================================

require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"

module CodingAdventures
  module DartmouthBasicLexer
    # ----------------------------------------------------------------
    # Grammar File Path Resolution
    # ----------------------------------------------------------------
    #
    # The grammar file lives in the shared code/grammars/ directory so
    # that all language implementations (Ruby, Python, TypeScript, Go, etc.)
    # can share one canonical definition.
    #
    # Directory layout:
    #   code/
    #     grammars/
    #       dartmouth_basic.tokens   <-- we need this
    #     packages/
    #       ruby/
    #         dartmouth_basic_lexer/
    #           lib/
    #             coding_adventures/
    #               dartmouth_basic_lexer/
    #                 tokenizer.rb   <-- we are here (__dir__)
    #
    # Counting the steps from __dir__ upward:
    #   dartmouth_basic_lexer/   (1)
    #   coding_adventures/       (2)
    #   lib/                     (3)
    #   dartmouth_basic_lexer/   (4)
    #   ruby/                    (5)
    #   packages/                (6)
    #   code/                    ← then into grammars/
    #
    GRAMMAR_DIR = File.expand_path("../../../../../../grammars", __dir__)
    DARTMOUTH_BASIC_TOKENS_PATH = File.join(GRAMMAR_DIR, "dartmouth_basic.tokens")

    # ----------------------------------------------------------------
    # tokenize(source) — The Main Entry Point
    # ----------------------------------------------------------------
    #
    # Tokenizes a string of Dartmouth BASIC source code. Returns an array
    # of Token objects, always ending with an EOF token.
    #
    # The pipeline is:
    #   1. Read and parse the dartmouth_basic.tokens grammar file.
    #   2. Create a GrammarLexer and register two post-tokenize hooks.
    #   3. Run the lexer — it produces raw tokens where line numbers and
    #      regular numbers look the same (both are NUMBER tokens), and
    #      where REM comment text appears as regular tokens.
    #   4. Post-hook 1: relabel_line_numbers_hook renames the first NUMBER
    #      on each line to LINE_NUM.
    #   5. Post-hook 2: suppress_rem_content_hook removes tokens between
    #      REM and the next NEWLINE.
    #
    # @param source [String] Dartmouth BASIC source text to tokenize
    # @return [Array<CodingAdventures::Lexer::Token>] the token stream
    def self.tokenize(source)
      # Parse the grammar file into a TokenGrammar struct. The struct
      # contains all the regex patterns, keyword list, skip rules, and
      # metadata like case_insensitive: true.
      grammar = CodingAdventures::GrammarTools.parse_token_grammar(
        File.read(DARTMOUTH_BASIC_TOKENS_PATH, encoding: "UTF-8")
      )

      # Create a GrammarLexer. The grammar has case_sensitive: false, so
      # the lexer lowercases the source before matching. Keywords like
      # PRINT, LET, IF match the lowercase KEYWORD regex pattern and are
      # then normalised back to uppercase by the @case_insensitive emit path.
      # Non-keyword identifiers (NAME, BUILTIN_FN, USER_FN) are upcased
      # by hook 3 below.
      lexer = CodingAdventures::Lexer::GrammarLexer.new(source, grammar)

      # Hook 1: relabel the first NUMBER on each line to LINE_NUM.
      # The grammar matches both line numbers and numeric literals with
      # the same regex. After tokenization, we walk the flat list and
      # use position context to tell them apart:
      #   - The first NUMBER token on a line → LINE_NUM
      #   - All other NUMBER tokens → stay as NUMBER
      lexer.add_post_tokenize(relabel_line_numbers_hook)

      # Hook 2: suppress tokens that follow a REM keyword until NEWLINE.
      # REM introduces a comment. Everything up to (but not including)
      # the next NEWLINE should be dropped from the output.
      lexer.add_post_tokenize(suppress_rem_content_hook)

      # Hook 3: normalise identifiers to uppercase.
      # The grammar uses case_sensitive: false, so the lexer lowercases
      # the source before matching. NAME, BUILTIN_FN, and USER_FN tokens
      # therefore have lowercase values ("x", "sin", "fna"). We upcase
      # them here so the caller always sees canonical uppercase values
      # ("X", "SIN", "FNA"). KEYWORD values are already upcased by the
      # GrammarLexer's @case_insensitive path. NUMBER values are unaffected
      # by casing. NEWLINE, EOF, and operator tokens have no letters.
      lexer.add_post_tokenize(upcase_identifiers_hook)

      lexer.tokenize
    end

    private

    # ----------------------------------------------------------------
    # Hook 3: Upcase Identifier Values
    # ----------------------------------------------------------------
    #
    # Returns a lambda that upcases the values of NAME, BUILTIN_FN, and
    # USER_FN tokens. The GrammarLexer already upcases KEYWORD tokens
    # when @case_insensitive is true, but custom token types (NAME,
    # BUILTIN_FN, USER_FN) are not in the KEYWORD path.
    #
    # Example:
    #   NAME("x")      → NAME("X")     (variable X)
    #   BUILTIN_FN("sin") → BUILTIN_FN("SIN")
    #   USER_FN("fna") → USER_FN("FNA")
    #
    # Why a separate hook rather than handling in the on_token callback?
    # Because the post-tokenize hook pipeline is the cleanest extension
    # point for this kind of global transformation. It does not interfere
    # with the LINE_NUM relabelling or REM suppression hooks.
    def self.upcase_identifiers_hook
      lambda do |tokens|
        # Token types whose values should be uppercased.
        upcase_types = %w[NAME BUILTIN_FN USER_FN].to_set

        tokens.map do |token|
          if upcase_types.include?(token.type)
            CodingAdventures::Lexer::Token.new(
              type:   token.type,
              value:  token.value.upcase,
              line:   token.line,
              column: token.column
            )
          else
            token
          end
        end
      end
    end

    # ----------------------------------------------------------------
    # Hook 1: Relabel Line Numbers
    # ----------------------------------------------------------------
    #
    # Returns a lambda that takes the raw token list and returns a new
    # list with position-based LINE_NUM labelling applied.
    #
    # Algorithm:
    #   - Walk through the token array left-to-right.
    #   - Track a boolean flag: are we "at line start"?
    #   - Initially: yes (the very first token in the file is a line start).
    #   - When we are at line start and see a NUMBER: relabel it LINE_NUM,
    #     then clear the flag (we are now "in" the line body).
    #   - When we see a NEWLINE: set the flag (next token starts a new line).
    #   - Any other token at line start: just clear the flag without relabelling.
    #
    # Example input (schematic):
    #   NUMBER("10") KEYWORD("LET") NAME("X") EQ("=") NUMBER("5") NEWLINE
    #   NUMBER("20") KEYWORD("PRINT") NAME("X") NEWLINE
    #   NUMBER("30") KEYWORD("END") NEWLINE
    #
    # Example output:
    #   LINE_NUM("10") KEYWORD("LET") NAME("X") EQ("=") NUMBER("5") NEWLINE
    #   LINE_NUM("20") KEYWORD("PRINT") NAME("X") NEWLINE
    #   LINE_NUM("30") KEYWORD("END") NEWLINE
    #
    # The Token type is Ruby's Data.define (immutable). To "relabel" we
    # create a new Token with the same value, line, and column but a
    # different type string.
    def self.relabel_line_numbers_hook
      lambda do |tokens|
        # at_line_start is true for:
        #   - The very first token in the stream (we're at the start of
        #     the first line before seeing anything)
        #   - Any token immediately following a NEWLINE token
        at_line_start = true

        tokens.map do |token|
          if at_line_start && token.type == "NUMBER"
            # This NUMBER is in line-number position. Relabel it.
            at_line_start = false
            CodingAdventures::Lexer::Token.new(
              type: "LINE_NUM",
              value: token.value,
              line: token.line,
              column: token.column
            )
          else
            # Not relabelling. Just update the state flag.
            at_line_start = false if at_line_start
            at_line_start = true  if token.type == "NEWLINE"
            token
          end
        end
      end
    end

    # ----------------------------------------------------------------
    # Hook 2: Suppress REM Comment Content
    # ----------------------------------------------------------------
    #
    # Returns a lambda that takes the token list (after hook 1 has run)
    # and returns a new list with comment text removed.
    #
    # Algorithm:
    #   - Walk the token list left-to-right.
    #   - When we see KEYWORD("REM"): set suppressing = true.
    #     (We keep the REM token itself — it tells the parser a comment
    #     was here, which can be useful for documentation extractors.)
    #   - While suppressing = true: drop every token we see.
    #   - When we see NEWLINE while suppressing: set suppressing = false
    #     and KEEP the NEWLINE (it is the statement terminator).
    #
    # Example input:
    #   LINE_NUM("10") KEYWORD("REM") UNKNOWN("T") UNKNOWN("H") UNKNOWN("I") ... NEWLINE
    #   (The comment text "THIS IS A COMMENT" was tokenised as individual
    #    UNKNOWN tokens because those chars matched the error recovery rule.)
    #
    # Wait — actually with @case_insensitive true the source is uppercased.
    # "THIS IS A COMMENT" in uppercase matches nothing useful in our grammar
    # (no identifier rule can match multi-char sequences after the NAME rule
    # only matches one letter + optional digit). So comment words appear as
    # NAME tokens (single letters) and UNKNOWN tokens. Either way we suppress
    # everything between REM and NEWLINE.
    #
    # Example output:
    #   LINE_NUM("10") KEYWORD("REM") NEWLINE
    def self.suppress_rem_content_hook
      lambda do |tokens|
        result = []
        suppressing = false

        tokens.each do |token|
          if suppressing
            # We are in a REM comment. Suppress everything until NEWLINE.
            if token.type == "NEWLINE"
              # The NEWLINE ends the statement — keep it but stop suppressing.
              result << token
              suppressing = false
            end
            # else: drop the token entirely — it's comment text.
          else
            # Normal mode: emit the token.
            result << token
            # If this is REM, start suppressing the next token onward.
            if token.type == "KEYWORD" && token.value == "REM"
              suppressing = true
            end
          end
        end

        result
      end
    end
  end
end

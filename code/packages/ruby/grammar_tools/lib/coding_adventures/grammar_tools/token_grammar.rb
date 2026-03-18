# frozen_string_literal: true

# ==========================================================================
# token_grammar.rb -- Parser and Validator for .tokens Files
# ==========================================================================
#
# A .tokens file is a declarative description of the lexical grammar of a
# programming language. It lists every token the lexer should recognize, in
# priority order (first match wins), along with an optional keywords section
# for reserved words.
#
# File format overview
# --------------------
#
# Each non-blank, non-comment line has one of three forms:
#
#   TOKEN_NAME = /regex_pattern/      -- a regex-based token
#   TOKEN_NAME = "literal_string"     -- a literal-string token
#   keywords:                         -- begins the keywords section
#
# Lines starting with # are comments. Blank lines are ignored.
#
# The keywords section lists one reserved word per line (indented). Keywords
# are identifiers that the lexer recognizes as NAME tokens but then
# reclassifies. For instance, `if` matches the NAME pattern but is promoted
# to an IF keyword.
#
# Design decisions
# ----------------
#
# Why hand-parse instead of using regex or a parser library? Because the
# format is simple enough that a line-by-line parser is clearer, faster, and
# produces better error messages than any generic tool would. Every error
# includes the line number where the problem occurred, which matters a lot
# when users are writing grammars by hand.
# ==========================================================================

module CodingAdventures
  module GrammarTools
    # Raised when a .tokens file cannot be parsed.
    class TokenGrammarError < StandardError
      attr_reader :message, :line_number

      def initialize(message, line_number)
        @message = message
        @line_number = line_number
        super("Line #{line_number}: #{message}")
      end
    end

    # A single token rule from a .tokens file.
    #
    # Attributes:
    #   name        -- the token name, e.g. "NUMBER" or "PLUS"
    #   pattern     -- the pattern string (regex body or literal body)
    #   is_regex    -- true if written as /regex/, false if "literal"
    #   line_number -- 1-based line where this definition appeared
    TokenDefinition = Data.define(:name, :pattern, :is_regex, :line_number)

    # The complete contents of a parsed .tokens file.
    #
    # definitions -- ordered list of TokenDefinition (order matters for
    #                first-match-wins semantics)
    # keywords    -- list of reserved words from the keywords: section
    class TokenGrammar
      attr_reader :definitions, :keywords

      def initialize(definitions: [], keywords: [])
        @definitions = definitions
        @keywords = keywords
      end

      # Return the set of all defined token names.
      def token_names
        @definitions.map(&:name).to_set
      end
    end

    # Parse the text of a .tokens file into a TokenGrammar.
    #
    # The parser operates line-by-line with two modes:
    #
    # 1. Definition mode (default) -- each line is a comment, blank, or
    #    token definition of the form NAME = /pattern/ or NAME = "literal".
    #
    # 2. Keywords mode -- entered on "keywords:" line. Subsequent indented
    #    lines are keywords until a non-indented, non-blank line appears.
    def self.parse_token_grammar(source)
      lines = source.split("\n")
      definitions = []
      keywords = []
      in_keywords = false

      lines.each_with_index do |raw_line, index|
        line_number = index + 1
        line = raw_line.rstrip
        stripped = line.strip

        # Blank lines and comments are always skipped.
        next if stripped.empty? || stripped.start_with?("#")

        # Keywords section header.
        if stripped == "keywords:" || stripped == "keywords :"
          in_keywords = true
          next
        end

        # Inside keywords section.
        if in_keywords
          if line.start_with?(" ", "\t")
            keywords << stripped unless stripped.empty?
            next
          else
            in_keywords = false
            # Fall through to parse as definition.
          end
        end

        # Token definition -- NAME = /pattern/ or NAME = "literal"
        unless line.include?("=")
          raise TokenGrammarError.new(
            "Expected token definition (NAME = pattern), got: #{stripped.inspect}",
            line_number
          )
        end

        eq_index = line.index("=")
        name_part = line[0...eq_index].strip
        pattern_part = line[(eq_index + 1)..].strip

        if name_part.empty?
          raise TokenGrammarError.new("Missing token name before '='", line_number)
        end

        unless name_part.match?(/\A[a-zA-Z_][a-zA-Z0-9_]*\z/)
          raise TokenGrammarError.new(
            "Invalid token name: #{name_part.inspect} (must be an identifier like NAME or PLUS_EQUALS)",
            line_number
          )
        end

        if pattern_part.empty?
          raise TokenGrammarError.new(
            "Missing pattern after '=' for token #{name_part.inspect}",
            line_number
          )
        end

        if pattern_part.start_with?("/") && pattern_part.end_with?("/")
          regex_body = pattern_part[1..-2]
          if regex_body.empty?
            raise TokenGrammarError.new(
              "Empty regex pattern for token #{name_part.inspect}",
              line_number
            )
          end
          definitions << TokenDefinition.new(
            name: name_part, pattern: regex_body,
            is_regex: true, line_number: line_number
          )
        elsif pattern_part.start_with?('"') && pattern_part.end_with?('"')
          literal_body = pattern_part[1..-2]
          if literal_body.empty?
            raise TokenGrammarError.new(
              "Empty literal pattern for token #{name_part.inspect}",
              line_number
            )
          end
          definitions << TokenDefinition.new(
            name: name_part, pattern: literal_body,
            is_regex: false, line_number: line_number
          )
        else
          raise TokenGrammarError.new(
            "Pattern for token #{name_part.inspect} must be /regex/ or \"literal\", got: #{pattern_part.inspect}",
            line_number
          )
        end
      end

      TokenGrammar.new(definitions: definitions, keywords: keywords)
    end

    # Check a parsed TokenGrammar for common problems.
    #
    # Validation checks:
    # - Duplicate token names
    # - Invalid regex patterns
    # - Empty patterns (safety net)
    # - Non-UPPER_CASE names (convention warning)
    def self.validate_token_grammar(grammar)
      issues = []
      seen_names = {}

      grammar.definitions.each do |defn|
        # Duplicate check.
        if seen_names.key?(defn.name)
          issues << "Line #{defn.line_number}: Duplicate token name '#{defn.name}' " \
                    "(first defined on line #{seen_names[defn.name]})"
        else
          seen_names[defn.name] = defn.line_number
        end

        # Empty pattern check.
        if defn.pattern.empty?
          issues << "Line #{defn.line_number}: Empty pattern for token '#{defn.name}'"
        end

        # Invalid regex check.
        if defn.is_regex
          begin
            Regexp.new(defn.pattern)
          rescue RegexpError => e
            issues << "Line #{defn.line_number}: Invalid regex for token '#{defn.name}': #{e.message}"
          end
        end

        # Naming convention check.
        unless defn.name == defn.name.upcase
          issues << "Line #{defn.line_number}: Token name '#{defn.name}' should be UPPER_CASE"
        end
      end

      issues
    end
  end
end

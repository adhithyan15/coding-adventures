# frozen_string_literal: true

# ==========================================================================
# parser_grammar.rb -- Parser and Validator for .grammar Files
# ==========================================================================
#
# A .grammar file describes the syntactic structure of a programming language
# using EBNF (Extended Backus-Naur Form). Where a .tokens file says "these
# are the words," a .grammar file says "these are the sentences."
#
# EBNF: a brief history
# ---------------------
#
# BNF (Backus-Naur Form) was invented in the late 1950s by John Backus and
# Peter Naur to describe the syntax of ALGOL 60. EBNF extends BNF with
# three conveniences:
#
#     { x }   -- zero or more repetitions of x
#     [ x ]   -- optional x
#     ( x )   -- grouping
#
# The recursive descent parser
# ----------------------------
#
# This module contains a hand-written recursive descent parser for the EBNF
# notation used in .grammar files. The meta-grammar is:
#
#     grammar_file  = { rule } ;
#     rule          = rule_name "=" body ";" ;
#     body          = sequence { "|" sequence } ;
#     sequence      = { element } ;
#     element       = rule_ref | token_ref | literal
#                   | "{" body "}"
#                   | "[" body "]"
#                   | "(" body ")" ;
# ==========================================================================

module CodingAdventures
  module GrammarTools
    # Raised when a .grammar file cannot be parsed.
    class ParserGrammarError < StandardError
      attr_reader :message, :line_number

      def initialize(message, line_number)
        @message = message
        @line_number = line_number
        super("Line #{line_number}: #{message}")
      end
    end

    # -----------------------------------------------------------------------
    # AST node types for the grammar elements
    # -----------------------------------------------------------------------

    # A reference to another rule (lowercase) or a token (UPPERCASE).
    RuleReference = Data.define(:name, :is_token)

    # A literal string match in the grammar, written as "..." in EBNF.
    Literal = Data.define(:value)

    # A sequence of elements that must appear in order.
    Sequence = Data.define(:elements)

    # A choice between alternatives, written with | in EBNF.
    Alternation = Data.define(:choices)

    # Zero-or-more repetition, written as { x } in EBNF.
    Repetition = Data.define(:element)

    # Optional element, written as [ x ] in EBNF.
    OptionalElement = Data.define(:element)

    # Explicit grouping, written as ( x ) in EBNF.
    Group = Data.define(:element)

    # A single rule from a .grammar file.
    GrammarRule = Data.define(:name, :body, :line_number)

    # The complete contents of a parsed .grammar file.
    class ParserGrammar
      attr_reader :rules

      def initialize(rules: [])
        @rules = rules
      end

      def rule_names
        @rules.map(&:name).to_set
      end

      # Return all UPPERCASE names referenced anywhere in the grammar.
      def token_references
        refs = Set.new
        @rules.each { |rule| collect_token_refs(rule.body, refs) }
        refs
      end

      # Return all lowercase names referenced anywhere in the grammar.
      def rule_references
        refs = Set.new
        @rules.each { |rule| collect_rule_refs(rule.body, refs) }
        refs
      end

      private

      def collect_token_refs(node, refs)
        case node
        when RuleReference
          refs.add(node.name) if node.is_token
        when Sequence
          node.elements.each { |e| collect_token_refs(e, refs) }
        when Alternation
          node.choices.each { |c| collect_token_refs(c, refs) }
        when Repetition, OptionalElement, Group
          collect_token_refs(node.element, refs)
        end
      end

      def collect_rule_refs(node, refs)
        case node
        when RuleReference
          refs.add(node.name) unless node.is_token
        when Sequence
          node.elements.each { |e| collect_rule_refs(e, refs) }
        when Alternation
          node.choices.each { |c| collect_rule_refs(c, refs) }
        when Repetition, OptionalElement, Group
          collect_rule_refs(node.element, refs)
        end
      end
    end

    # -----------------------------------------------------------------------
    # Internal tokenizer for .grammar files
    # -----------------------------------------------------------------------

    # @private
    GrammarToken = Data.define(:kind, :value, :line)

    # Break .grammar source text into tokens.
    # @private
    def self._tokenize_grammar(source)
      tokens = []
      lines = source.split("\n")

      lines.each_with_index do |raw_line, index|
        line_number = index + 1
        line = raw_line.rstrip
        stripped = line.strip
        next if stripped.empty? || stripped.start_with?("#")

        i = 0
        while i < line.length
          ch = line[i]

          if ch == " " || ch == "\t"
            i += 1
            next
          end

          # Inline comments.
          break if ch == "#"

          case ch
          when "="
            tokens << GrammarToken.new(kind: "EQUALS", value: "=", line: line_number)
            i += 1
          when ";"
            tokens << GrammarToken.new(kind: "SEMI", value: ";", line: line_number)
            i += 1
          when "|"
            tokens << GrammarToken.new(kind: "PIPE", value: "|", line: line_number)
            i += 1
          when "{"
            tokens << GrammarToken.new(kind: "LBRACE", value: "{", line: line_number)
            i += 1
          when "}"
            tokens << GrammarToken.new(kind: "RBRACE", value: "}", line: line_number)
            i += 1
          when "["
            tokens << GrammarToken.new(kind: "LBRACKET", value: "[", line: line_number)
            i += 1
          when "]"
            tokens << GrammarToken.new(kind: "RBRACKET", value: "]", line: line_number)
            i += 1
          when "("
            tokens << GrammarToken.new(kind: "LPAREN", value: "(", line: line_number)
            i += 1
          when ")"
            tokens << GrammarToken.new(kind: "RPAREN", value: ")", line: line_number)
            i += 1
          when '"'
            j = i + 1
            while j < line.length && line[j] != '"'
              j += 1 if line[j] == "\\"
              j += 1
            end
            if j >= line.length
              raise ParserGrammarError.new("Unterminated string literal", line_number)
            end
            tokens << GrammarToken.new(kind: "STRING", value: line[(i + 1)...j], line: line_number)
            i = j + 1
          else
            if ch.match?(/[a-zA-Z_]/)
              j = i
              j += 1 while j < line.length && line[j].match?(/[a-zA-Z0-9_]/)
              tokens << GrammarToken.new(kind: "IDENT", value: line[i...j], line: line_number)
              i = j
            else
              raise ParserGrammarError.new("Unexpected character: #{ch.inspect}", line_number)
            end
          end
        end
      end

      tokens << GrammarToken.new(kind: "EOF", value: "", line: lines.length)
      tokens
    end

    # -----------------------------------------------------------------------
    # Recursive descent parser for EBNF
    # -----------------------------------------------------------------------

    # @private
    class EBNFParser
      def initialize(tokens)
        @tokens = tokens
        @pos = 0
      end

      def parse
        rules = []
        rules << parse_rule while peek.kind != "EOF"
        rules
      end

      private

      def peek
        @tokens[@pos]
      end

      def advance
        tok = @tokens[@pos]
        @pos += 1
        tok
      end

      def expect(kind)
        tok = advance
        unless tok.kind == kind
          raise ParserGrammarError.new(
            "Expected #{kind}, got #{tok.kind} (#{tok.value.inspect})",
            tok.line
          )
        end
        tok
      end

      def parse_rule
        name_tok = expect("IDENT")
        expect("EQUALS")
        body = parse_body
        expect("SEMI")
        GrammarRule.new(name: name_tok.value, body: body, line_number: name_tok.line)
      end

      # body = sequence { "|" sequence }
      def parse_body
        first = parse_sequence
        alternatives = [first]
        while peek.kind == "PIPE"
          advance
          alternatives << parse_sequence
        end
        return alternatives[0] if alternatives.length == 1
        Alternation.new(choices: alternatives)
      end

      # sequence = { element }
      def parse_sequence
        stop_kinds = %w[PIPE SEMI RBRACE RBRACKET RPAREN EOF]
        elements = []
        elements << parse_element until stop_kinds.include?(peek.kind)

        if elements.empty?
          raise ParserGrammarError.new(
            "Expected at least one element in sequence",
            peek.line
          )
        end

        return elements[0] if elements.length == 1
        Sequence.new(elements: elements)
      end

      # element = ident | string | "{" body "}" | "[" body "]" | "(" body ")"
      def parse_element
        tok = peek

        case tok.kind
        when "IDENT"
          advance
          is_token = tok.value == tok.value.upcase && tok.value[0].match?(/[A-Z]/)
          RuleReference.new(name: tok.value, is_token: is_token)
        when "STRING"
          advance
          Literal.new(value: tok.value)
        when "LBRACE"
          advance
          body = parse_body
          expect("RBRACE")
          Repetition.new(element: body)
        when "LBRACKET"
          advance
          body = parse_body
          expect("RBRACKET")
          OptionalElement.new(element: body)
        when "LPAREN"
          advance
          body = parse_body
          expect("RPAREN")
          Group.new(element: body)
        else
          raise ParserGrammarError.new(
            "Unexpected token: #{tok.kind} (#{tok.value.inspect})",
            tok.line
          )
        end
      end
    end

    # Parse the text of a .grammar file into a ParserGrammar.
    def self.parse_parser_grammar(source)
      tokens = _tokenize_grammar(source)
      parser = EBNFParser.new(tokens)
      rules = parser.parse
      ParserGrammar.new(rules: rules)
    end

    # Check a parsed ParserGrammar for common problems.
    def self.validate_parser_grammar(grammar, token_names: nil)
      issues = []
      defined = grammar.rule_names
      referenced_rules = grammar.rule_references
      referenced_tokens = grammar.token_references

      # Duplicate rule names.
      seen = {}
      grammar.rules.each do |rule|
        if seen.key?(rule.name)
          issues << "Line #{rule.line_number}: Duplicate rule name '#{rule.name}' " \
                    "(first defined on line #{seen[rule.name]})"
        else
          seen[rule.name] = rule.line_number
        end
      end

      # Non-lowercase rule names.
      grammar.rules.each do |rule|
        unless rule.name == rule.name.downcase
          issues << "Line #{rule.line_number}: Rule name '#{rule.name}' should be lowercase"
        end
      end

      # Undefined rule references.
      referenced_rules.sort.each do |ref|
        unless defined.include?(ref)
          issues << "Undefined rule reference: '#{ref}'"
        end
      end

      # Undefined token references.
      if token_names
        # Synthetic tokens are always valid — the lexer produces these
        # implicitly without needing a .tokens definition:
        #   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
        #   INDENT/DEDENT — emitted in indentation mode
        #   EOF — always emitted at end of input
        synthetic_tokens = Set.new(%w[NEWLINE INDENT DEDENT EOF])
        referenced_tokens.sort.each do |ref|
          unless token_names.include?(ref) || synthetic_tokens.include?(ref)
            issues << "Undefined token reference: '#{ref}'"
          end
        end
      end

      # Unreachable rules.
      if grammar.rules.any?
        start_rule = grammar.rules[0].name
        grammar.rules.each do |rule|
          if rule.name != start_rule && !referenced_rules.include?(rule.name)
            issues << "Line #{rule.line_number}: Rule '#{rule.name}' is " \
                      "defined but never referenced (unreachable)"
          end
        end
      end

      issues
    end
  end
end

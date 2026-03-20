# frozen_string_literal: true

require "coding_adventures_lexer"
require "coding_adventures_grammar_tools"

# ==========================================================================
# GrammarParser -- Grammar-Driven Parser from .grammar Files
# ==========================================================================
#
# Instead of hardcoding grammar rules as Ruby methods, this parser reads
# rules from a .grammar file and interprets them at runtime. The same code
# can parse any language -- just swap the .grammar file.
#
# Produces generic ASTNode objects rather than typed nodes like NumberLiteral
# or BinaryOp. Each node records which grammar rule produced it and its
# matched children (tokens and sub-nodes).
#
# Key features:
#
# - Backtracking: when an alternation's first choice fails, the parser
#   restores position and tries the next choice.
#
# - Packrat memoization: caches (rule_name, position) -> result so that
#   re-parsing the same rule at the same position is O(1). This converts
#   potentially exponential backtracking into O(n * R) where n = input
#   length and R = number of grammar rules.
#
# - Significant newlines: auto-detects if the grammar references NEWLINE
#   tokens. If so, newlines are treated as meaningful; otherwise they are
#   skipped automatically.
#
# - String-based token types: handles both TokenType constants (from the
#   hand-written lexer) and plain string types (from the grammar-driven
#   lexer with extended grammars like Starlark).
#
# - Furthest failure tracking: when parsing fails, reports what was expected
#   at the furthest position reached, giving much better error messages.
# ==========================================================================

module CodingAdventures
  module Parser
    # A generic AST node produced by grammar-driven parsing.
    ASTNode = Data.define(:rule_name, :children) do
      def leaf?
        children.length == 1 && children[0].is_a?(CodingAdventures::Lexer::Token)
      end

      def token
        return children[0] if leaf?
        nil
      end
    end

    # Error during grammar-driven parsing.
    class GrammarParseError < StandardError
      attr_reader :token

      def initialize(message, token = nil)
        @token = token
        if token
          super("Parse error at #{token.line}:#{token.column}: #{message}")
        else
          super("Parse error: #{message}")
        end
      end
    end

    class GrammarDrivenParser
      TT = CodingAdventures::Lexer::TokenType
      GT = CodingAdventures::GrammarTools

      def initialize(tokens, grammar)
        @tokens = tokens
        @grammar = grammar
        @pos = 0
        @rules = {}
        grammar.rules.each { |rule| @rules[rule.name] = rule }

        # Detect whether newlines are significant in this grammar.
        @newlines_significant = grammar_references_newline?

        # Packrat memoization cache: [rule_name, position] -> [result, end_pos]
        @memo = {}

        # Furthest failure tracking for better error messages.
        @furthest_pos = 0
        @furthest_expected = []
      end

      # Whether newlines are significant in this grammar.
      attr_reader :newlines_significant

      # Parse using the first grammar rule as entry point.
      def parse
        raise GrammarParseError.new("Grammar has no rules") if @grammar.rules.empty?

        entry_rule = @grammar.rules[0]
        result = parse_rule(entry_rule.name)

        # Skip trailing newlines.
        while @pos < @tokens.length && token_type_name(current) == "NEWLINE"
          @pos += 1
        end

        # Verify all tokens consumed.
        if @pos < @tokens.length && token_type_name(current) != "EOF"
          # Use furthest failure info for a better error message.
          if @furthest_expected.any? && @furthest_pos > @pos
            expected_str = @furthest_expected.first(5).join(" or ")
            furthest_tok = if @furthest_pos < @tokens.length
              @tokens[@furthest_pos]
            else
              current
            end
            raise GrammarParseError.new(
              "Expected #{expected_str}, got #{furthest_tok.value.inspect}",
              furthest_tok
            )
          end
          raise GrammarParseError.new(
            "Unexpected token: #{current.value.inspect}",
            current
          )
        end

        result
      end

      private

      def current
        return @tokens[-1] if @pos >= @tokens.length
        @tokens[@pos]
      end

      # Extract the type name from a token (works with both string and
      # constant-based types).
      def token_type_name(token)
        token.type.to_s
      end

      # Record an expected token/rule at the current position for error messages.
      def record_failure(expected)
        if @pos > @furthest_pos
          @furthest_pos = @pos
          @furthest_expected = [expected]
        elsif @pos == @furthest_pos && !@furthest_expected.include?(expected)
          @furthest_expected << expected
        end
      end

      # Check if any grammar rule references the NEWLINE token.
      def grammar_references_newline?
        @grammar.rules.any? { |rule| element_references_newline?(rule.body) }
      end

      def element_references_newline?(element)
        case element
        when GT::RuleReference
          element.is_token && element.name == "NEWLINE"
        when GT::Sequence
          element.elements.any? { |e| element_references_newline?(e) }
        when GT::Alternation
          element.choices.any? { |c| element_references_newline?(c) }
        when GT::Repetition, GT::OptionalElement, GT::Group
          element_references_newline?(element.element)
        else
          false
        end
      end

      # Parse a named grammar rule with memoization.
      def parse_rule(rule_name)
        unless @rules.key?(rule_name)
          raise GrammarParseError.new("Undefined rule: #{rule_name}")
        end

        # Check memo cache.
        memo_key = [rule_name, @pos]
        if @memo.key?(memo_key)
          cached_result, cached_end_pos = @memo[memo_key]
          @pos = cached_end_pos
          if cached_result.nil?
            raise GrammarParseError.new(
              "Expected #{rule_name}, got #{current.value.inspect}",
              current
            )
          end
          return ASTNode.new(rule_name: rule_name, children: cached_result)
        end

        # Not cached -- parse the rule.
        start_pos = @pos
        rule = @rules[rule_name]
        children = match_element(rule.body)

        # Cache the result.
        @memo[memo_key] = [children, @pos]

        unless children
          @pos = start_pos
          record_failure(rule_name)
          raise GrammarParseError.new(
            "Expected #{rule_name}, got #{current.value.inspect}",
            current
          )
        end

        ASTNode.new(rule_name: rule_name, children: children)
      end

      # Try to match a grammar element against the token stream.
      # Returns an array of matched children on success, nil on failure.
      # Restores position on failure (backtracking).
      def match_element(element)
        save_pos = @pos

        case element
        when GT::Sequence
          children = []
          element.elements.each do |sub|
            result = match_element(sub)
            if result.nil?
              @pos = save_pos
              return nil
            end
            children.concat(result)
          end
          children

        when GT::Alternation
          element.choices.each do |choice|
            @pos = save_pos
            result = match_element(choice)
            return result if result
          end
          @pos = save_pos
          nil

        when GT::Repetition
          children = []
          loop do
            save_rep = @pos
            result = match_element(element.element)
            if result.nil?
              @pos = save_rep
              break
            end
            children.concat(result)
          end
          children # Always succeeds (zero matches is fine)

        when GT::OptionalElement
          result = match_element(element.element)
          result || [] # Always succeeds

        when GT::Group
          match_element(element.element)

        when GT::RuleReference
          if element.is_token
            match_token_reference(element)
          else
            # lowercase: parse another grammar rule recursively.
            begin
              node = parse_rule(element.name)
              [node]
            rescue GrammarParseError
              @pos = save_pos
              nil
            end
          end

        when GT::Literal
          tok = current

          # Skip insignificant newlines before literal matching.
          unless @newlines_significant
            while token_type_name(tok) == "NEWLINE"
              @pos += 1
              tok = current
            end
          end

          if tok.value == element.value
            @pos += 1
            [tok]
          else
            record_failure("\"#{element.value}\"")
            nil
          end
        end
      end

      # Match a token reference (UPPERCASE name) against the current token.
      def match_token_reference(element)
        tok = current

        # Skip newlines when matching non-NEWLINE tokens, but only if
        # newlines are not significant.
        if !@newlines_significant && element.name != "NEWLINE"
          while token_type_name(tok) == "NEWLINE"
            @pos += 1
            tok = current
          end
        end

        type_name = token_type_name(tok)

        # Direct string comparison -- works for both constant and string types.
        if type_name == element.name
          @pos += 1
          return [tok]
        end

        # Backward compatibility: try TokenType::ALL lookup.
        expected_type = TT::ALL[element.name]
        if expected_type && tok.type == expected_type
          @pos += 1
          return [tok]
        end

        record_failure(element.name)
        nil
      end
    end
  end
end

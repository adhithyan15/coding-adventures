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
# Uses backtracking for alternations: if the first choice fails, the parser
# restores position and tries the next choice.
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
      end

      # Parse using the first grammar rule as entry point.
      def parse
        raise GrammarParseError.new("Grammar has no rules") if @grammar.rules.empty?

        entry_rule = @grammar.rules[0]
        result = parse_rule(entry_rule.name)

        # Skip trailing newlines.
        while @pos < @tokens.length && current.type == TT::NEWLINE
          @pos += 1
        end

        # Verify all tokens consumed.
        if @pos < @tokens.length && current.type != TT::EOF
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

      def parse_rule(rule_name)
        unless @rules.key?(rule_name)
          raise GrammarParseError.new("Undefined rule: #{rule_name}")
        end

        rule = @rules[rule_name]
        children = match_element(rule.body)

        unless children
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
            # UPPERCASE: match a token type.
            tok = current
            # Skip newlines when matching non-NEWLINE tokens.
            while tok.type == TT::NEWLINE && element.name != "NEWLINE"
              @pos += 1
              tok = current
            end
            expected_type = TT::ALL[element.name]
            return nil unless expected_type
            if tok.type == expected_type
              @pos += 1
              return [tok]
            end
            nil
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
          if tok.value == element.value
            @pos += 1
            [tok]
          end
        end
      end
    end
  end
end

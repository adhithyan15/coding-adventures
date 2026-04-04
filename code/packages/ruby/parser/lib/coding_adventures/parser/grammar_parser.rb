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
#
# - Syntactic predicates: &element (positive lookahead) and !element
#   (negative lookahead) check if a pattern matches without consuming input.
#
# - One-or-more repetition: { element }+ requires at least one match.
#
# - Separated repetition: { element // separator } matches zero-or-more
#   elements with separators between them.
#
# AST Position Tracking
# ---------------------
#
# ASTNode objects carry optional position information (start_line,
# start_column, end_line, end_column) computed from their child tokens.
# This enables downstream tools (linters, formatters, error reporters)
# to map AST nodes back to source locations.
#
# AST Walking Utilities
# ---------------------
#
# Three utility methods are provided for working with AST trees:
#
# - walk_ast(node, visitor) -- depth-first traversal with enter/leave
# - find_nodes(node, rule_name) -- find all nodes with a given rule name
# - collect_tokens(node, type) -- collect all tokens, optionally filtered
# ==========================================================================

module CodingAdventures
  module Parser
    # A generic AST node produced by grammar-driven parsing.
    #
    # Attributes:
    #   rule_name    -- the grammar rule that produced this node
    #   children     -- array of Token and ASTNode children
    #   start_line   -- 1-based line of the first token (nil if empty)
    #   start_column -- 1-based column of the first token (nil if empty)
    #   end_line     -- 1-based line of the last token (nil if empty)
    #   end_column   -- 1-based column of the last token (nil if empty)
    ASTNode = Data.define(:rule_name, :children, :start_line, :start_column,
                          :end_line, :end_column) do
      def initialize(rule_name:, children:, start_line: nil, start_column: nil,
                     end_line: nil, end_column: nil)
        super(rule_name: rule_name, children: children,
              start_line: start_line, start_column: start_column,
              end_line: end_line, end_column: end_column)
      end

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

    # =====================================================================
    # AST Walking Utilities
    # =====================================================================

    # Visitor interface for walk_ast.
    #
    # Both callbacks are optional. Each receives the current node and its
    # parent (nil for the root). Returning an ASTNode replaces the visited
    # node; returning nil keeps the original.
    #
    # Usage:
    #
    #   visitor = {
    #     enter: ->(node, parent) { ... },
    #     leave: ->(node, parent) { ... }
    #   }
    #   new_ast = CodingAdventures::Parser.walk_ast(root, visitor)

    # Depth-first walk of an AST tree with enter/leave visitor callbacks.
    #
    # Visitor callbacks can return a replacement node or nil (keep original).
    # Token children are not visited -- only ASTNode children are walked.
    #
    # This is the generic traversal primitive. Language packages use it for
    # cover grammar rewriting, desugaring, and semantic analysis.
    #
    # @param node [ASTNode] the root node to walk
    # @param visitor [Hash] with optional :enter and :leave procs
    # @return [ASTNode] the (possibly replaced) root node
    def self.walk_ast(node, visitor)
      walk_node(node, nil, visitor)
    end

    # Find all nodes matching a rule name (depth-first order).
    #
    # @param node [ASTNode] the root node to search
    # @param rule_name [String] the rule name to match
    # @return [Array<ASTNode>] all matching nodes
    def self.find_nodes(node, rule_name)
      results = []
      walk_ast(node, {
        enter: ->(n, _parent) {
          results << n if n.rule_name == rule_name
          nil
        }
      })
      results
    end

    # Collect all tokens in depth-first order, optionally filtered by type.
    #
    # @param node [ASTNode] the root node
    # @param type [String, nil] optional type filter (nil = all tokens)
    # @return [Array<Token>] matching tokens
    def self.collect_tokens(node, type = nil)
      results = []
      collect_tokens_recursive(node, type, results)
      results
    end

    # @private
    def self.walk_node(node, parent, visitor)
      # Enter phase -- visitor may replace the node.
      current = node
      if visitor[:enter]
        replacement = visitor[:enter].call(current, parent)
        current = replacement if replacement.is_a?(ASTNode)
      end

      # Walk children recursively.
      children_changed = false
      new_children = current.children.map do |child|
        if child.is_a?(ASTNode)
          walked = walk_node(child, current, visitor)
          children_changed = true if walked.object_id != child.object_id
          walked
        else
          child
        end
      end

      # If children changed, create a new node with updated children.
      if children_changed
        current = ASTNode.new(
          rule_name: current.rule_name, children: new_children,
          start_line: current.start_line, start_column: current.start_column,
          end_line: current.end_line, end_column: current.end_column
        )
      end

      # Leave phase -- visitor may replace the node.
      if visitor[:leave]
        replacement = visitor[:leave].call(current, parent)
        current = replacement if replacement.is_a?(ASTNode)
      end

      current
    end
    private_class_method :walk_node

    # @private
    def self.collect_tokens_recursive(node, type, results)
      node.children.each do |child|
        if child.is_a?(ASTNode)
          collect_tokens_recursive(child, type, results)
        else
          results << child if type.nil? || child.type.to_s == type
        end
      end
    end
    private_class_method :collect_tokens_recursive

    # =====================================================================
    # The Grammar-Driven Parser
    # =====================================================================

    class GrammarDrivenParser
      TT = CodingAdventures::Lexer::TokenType
      GT = CodingAdventures::GrammarTools

      # Initialize a grammar-driven parser.
      #
      # Arguments:
      #   tokens  -- flat array of Token objects from the lexer
      #   grammar -- ParserGrammar produced by GT.parse_parser_grammar
      #
      # Keyword arguments:
      #   trace: false  -- when true, emit a [TRACE] line to $stderr for
      #                    every rule attempt. Useful for debugging why a
      #                    parse is failing or succeeding.
      #
      # Trace format:
      #
      #   [TRACE] rule 'qualified_rule' at token 5 (IDENT "h1") → match
      #   [TRACE] rule 'at_rule' at token 5 (IDENT "h1") → fail
      #
      # The token index is the position *before* the attempt. The type and
      # value shown are those of the token at that position.
      def initialize(tokens, grammar, trace: false)
        @tokens = tokens
        @grammar = grammar
        @pos = 0
        @rules = {}
        @trace = trace
        grammar.rules.each { |rule| @rules[rule.name] = rule }

        # Detect whether newlines are significant in this grammar.
        @newlines_significant = grammar_references_newline?

        # Packrat memoization cache: [rule_name, position] -> [result, end_pos]
        @memo = {}

        # Furthest failure tracking for better error messages.
        @furthest_pos = 0
        @furthest_expected = []

        # Transform hooks — pluggable pipeline stages for language-specific
        # processing. Hooks compose left-to-right.
        @pre_parse_hooks = []
        @post_parse_hooks = []
      end

      # Whether newlines are significant in this grammar.
      attr_reader :newlines_significant

      # Register a token transform to run before parsing begins.
      # The hook receives the token list and returns a (possibly
      # modified) token list. Multiple hooks compose left-to-right.
      def add_pre_parse(hook)
        @pre_parse_hooks << hook
      end

      # Register an AST transform to run after parsing completes.
      # The hook receives the root ASTNode and returns a (possibly
      # modified) ASTNode. Multiple hooks compose left-to-right.
      def add_post_parse(hook)
        @post_parse_hooks << hook
      end

      # Parse using the first grammar rule as entry point.
      def parse
        # Pre-parse hooks transform the token list before parsing.
        @pre_parse_hooks.each { |hook| @tokens = hook.call(@tokens) }

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

        # Post-parse hooks transform the AST after parsing completes.
        @post_parse_hooks.each { |hook| result = hook.call(result) }

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
        when GT::Repetition, GT::OptionalElement, GT::Group,
             GT::PositiveLookahead, GT::NegativeLookahead, GT::OneOrMoreRepetition
          element_references_newline?(element.element)
        when GT::SeparatedRepetition
          element_references_newline?(element.element) ||
            element_references_newline?(element.separator)
        else
          false
        end
      end

      # Parse a named grammar rule with memoization and left-recursion support.
      #
      # Implements the seed-and-grow technique from Warth et al.,
      # "Packrat Parsers Can Support Left Recursion" (2008).
      def parse_rule(rule_name)
        unless @rules.key?(rule_name)
          raise GrammarParseError.new("Undefined rule: #{rule_name}")
        end

        # Capture position and token info for trace output.
        attempt_pos = @pos
        tok = current

        # Check memo cache.
        memo_key = [rule_name, @pos]
        if @memo.key?(memo_key)
          cached_result, cached_end_pos = @memo[memo_key]
          @pos = cached_end_pos
          if cached_result.nil?
            emit_trace(rule_name, attempt_pos, tok, :fail) if @trace
            raise GrammarParseError.new(
              "Expected #{rule_name}, got #{current.value.inspect}",
              current
            )
          end
          emit_trace(rule_name, attempt_pos, tok, :match) if @trace
          return make_ast_node(rule_name, cached_result)
        end

        start_pos = @pos
        rule = @rules[rule_name]

        # Left-recursion guard: seed the memo with a failure entry BEFORE
        # parsing the rule body.
        @memo[memo_key] = [nil, start_pos]

        children = match_element(rule.body)

        # Cache the result.
        @memo[memo_key] = [children, @pos]

        # If the initial parse succeeded, try to grow the match.
        if children
          loop do
            prev_end = @pos
            @pos = start_pos
            @memo[memo_key] = [children, prev_end]
            new_children = match_element(rule.body)
            if new_children.nil? || @pos <= prev_end
              # Could not grow — restore the best result.
              @pos = prev_end
              @memo[memo_key] = [children, prev_end]
              break
            end
            children = new_children
          end
        end

        unless children
          @pos = start_pos
          record_failure(rule_name)
          emit_trace(rule_name, attempt_pos, tok, :fail) if @trace
          raise GrammarParseError.new(
            "Expected #{rule_name}, got #{current.value.inspect}",
            current
          )
        end

        emit_trace(rule_name, attempt_pos, tok, :match) if @trace
        make_ast_node(rule_name, children)
      end

      # Construct an ASTNode with computed position information.
      #
      # Walks the children to find the first and last leaf tokens,
      # then uses their line/column as the node's span. Returns a
      # node with nil positions if children contain no tokens.
      def make_ast_node(rule_name, children)
        first_tok = find_first_token(children)
        last_tok = find_last_token(children)
        if first_tok && last_tok
          ASTNode.new(
            rule_name: rule_name, children: children,
            start_line: first_tok.line, start_column: first_tok.column,
            end_line: last_tok.line, end_column: last_tok.column
          )
        else
          ASTNode.new(rule_name: rule_name, children: children)
        end
      end

      # Find the first leaf token in a children array (depth-first).
      def find_first_token(children)
        children.each do |child|
          if child.is_a?(ASTNode)
            tok = find_first_token(child.children)
            return tok if tok
          else
            return child
          end
        end
        nil
      end

      # Find the last leaf token in a children array (depth-first, reversed).
      def find_last_token(children)
        children.reverse_each do |child|
          if child.is_a?(ASTNode)
            tok = find_last_token(child.children)
            return tok if tok
          else
            return child
          end
        end
        nil
      end

      # Emit a single trace line to $stderr.
      #
      # Format:
      #   [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail
      def emit_trace(rule_name, pos, tok, outcome)
        type_str = tok.type.to_s
        val_str = tok.value.to_s
        result_str = (outcome == :match) ? "match" : "fail"
        warn "[TRACE] rule '#{rule_name}' at token #{pos} (#{type_str} \"#{val_str}\") \u2192 #{result_str}"
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

        # ---------------------------------------------------------------
        # Extension: Syntactic predicates (lookahead without consuming)
        # ---------------------------------------------------------------

        when GT::PositiveLookahead
          # Succeed if inner element matches, but consume no input.
          result = match_element(element.element)
          @pos = save_pos
          result.nil? ? nil : []

        when GT::NegativeLookahead
          # Succeed if inner element does NOT match, consume no input.
          result = match_element(element.element)
          @pos = save_pos
          result.nil? ? [] : nil

        # ---------------------------------------------------------------
        # Extension: One-or-more repetition
        # ---------------------------------------------------------------

        when GT::OneOrMoreRepetition
          # Match one required, then zero or more additional.
          first = match_element(element.element)
          if first.nil?
            @pos = save_pos
            return nil
          end
          children = first.dup
          loop do
            save_rep = @pos
            result = match_element(element.element)
            if result.nil?
              @pos = save_rep
              break
            end
            children.concat(result)
          end
          children

        # ---------------------------------------------------------------
        # Extension: Separated repetition
        # ---------------------------------------------------------------

        when GT::SeparatedRepetition
          # Match: element { separator element }
          # Or with at_least_one=false: [ element { separator element } ]
          first = match_element(element.element)
          if first.nil?
            @pos = save_pos
            return nil if element.at_least_one
            return [] # zero occurrences is valid
          end
          children = first.dup
          loop do
            save_sep = @pos
            sep = match_element(element.separator)
            if sep.nil?
              @pos = save_sep
              break
            end
            nxt = match_element(element.element)
            if nxt.nil?
              @pos = save_sep
              break
            end
            children.concat(sep)
            children.concat(nxt)
          end
          children
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

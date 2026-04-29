# frozen_string_literal: true

# ================================================================
# Expression Evaluator — evaluate AST expression nodes against a row
# ================================================================
#
# The SQL grammar produces a recursive tree of expression nodes.
# This module walks that tree and computes a Ruby value.
#
# Expression grammar (abbreviated):
#
#   expr            = or_expr
#   or_expr         = and_expr { "OR" and_expr }
#   and_expr        = not_expr { "AND" not_expr }
#   not_expr        = [ "NOT" ] comparison
#   comparison      = additive { cmp_op additive }
#                   | additive "BETWEEN" additive "AND" additive
#                   | additive "IN" "(" value_list ")"
#                   | additive "LIKE" additive
#                   | additive "IS" "NULL"
#                   | additive "IS" "NOT" "NULL"
#   additive        = multiplicative { ("+" | "-") multiplicative }
#   multiplicative  = unary { ("*" | "/" | "%") unary }
#   unary           = [ "-" ] primary
#   primary         = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
#                   | column_ref | function_call | "(" expr ")"
#   column_ref      = NAME [ "." NAME ]
#   function_call   = NAME "(" ( "*" | expr { "," expr } ) ")"
#
# Row context:
#   row_ctx is a Hash mapping column names to their values.
#   Qualified names like "employees.id" may also be present.
#
# SQL NULL:
#   Ruby nil represents SQL NULL. NULL propagates through arithmetic
#   and comparisons. IS NULL / IS NOT NULL are the only tests for it.
# ================================================================

require "strscan"

module CodingAdventures
  module SqlExecutionEngine
    # Recursive expression evaluator.
    module Expression
      ASTNode = CodingAdventures::Parser::ASTNode
      Token   = CodingAdventures::Lexer::Token

      # Evaluate an AST expression node against row_ctx.
      #
      # @param node    [ASTNode | Token]
      # @param row_ctx [Hash] current row values
      # @return [Object] nil, Integer, Float, String, or Boolean
      def self.eval_expr(node, row_ctx)
        if node.is_a?(Token)
          return eval_token(node, row_ctx)
        end

        rule = node.rule_name

        case rule
        when "expr", "statement", "program"
          eval_expr(node.children.first, row_ctx)
        when "or_expr"
          eval_or(node, row_ctx)
        when "and_expr"
          eval_and(node, row_ctx)
        when "not_expr"
          eval_not(node, row_ctx)
        when "comparison"
          eval_comparison(node, row_ctx)
        when "additive"
          eval_additive(node, row_ctx)
        when "multiplicative"
          eval_multiplicative(node, row_ctx)
        when "unary"
          eval_unary(node, row_ctx)
        when "primary"
          eval_primary(node, row_ctx)
        when "column_ref"
          eval_column_ref(node, row_ctx)
        when "function_call"
          eval_function_call(node, row_ctx)
        else
          # Pass-through: delegate to first child
          node.children.empty? ? nil : eval_expr(node.children.first, row_ctx)
        end
      end

      # ----------------------------------------------------------------
      # Token evaluation
      # ----------------------------------------------------------------

      def self.eval_token(token, row_ctx)
        type_name = token_type_name(token)
        value = token.value

        case type_name
        when "NUMBER"
          value.include?(".") ? value.to_f : value.to_i
        when "STRING"
          # The lexer already strips surrounding quotes
          value
        when "KEYWORD"
          case value.upcase
          when "NULL"  then nil
          when "TRUE"  then true
          when "FALSE" then false
          else value
          end
        when "NAME"
          resolve_column(value, row_ctx)
        else
          value
        end
      end

      # ----------------------------------------------------------------
      # Boolean operators
      # ----------------------------------------------------------------

      def self.eval_or(node, row_ctx)
        result = eval_expr(node.children[0], row_ctx)
        i = 1
        while i < node.children.size
          i += 1 # skip "OR" keyword
          break if i >= node.children.size
          right = eval_expr(node.children[i], row_ctx)
          i += 1
          # SQL three-valued logic
          if result == true || right == true
            result = true
          elsif result.nil? || right.nil?
            result = nil
          else
            result = false
          end
        end
        result
      end

      def self.eval_and(node, row_ctx)
        result = eval_expr(node.children[0], row_ctx)
        i = 1
        while i < node.children.size
          i += 1 # skip "AND" keyword
          break if i >= node.children.size
          right = eval_expr(node.children[i], row_ctx)
          i += 1
          if result == false || right == false
            result = false
          elsif result.nil? || right.nil?
            result = nil
          else
            result = result && right
          end
        end
        result
      end

      def self.eval_not(node, row_ctx)
        children = node.children
        if keyword?(children[0], "NOT")
          val = eval_expr(children[1], row_ctx)
          val.nil? ? nil : !val
        else
          eval_expr(children[0], row_ctx)
        end
      end

      # ----------------------------------------------------------------
      # Comparison
      # ----------------------------------------------------------------

      def self.eval_comparison(node, row_ctx)
        children = node.children

        return eval_expr(children[0], row_ctx) if children.size == 1

        left = eval_expr(children[0], row_ctx)
        second_kw = keyword_value(children[1])

        # IS NULL / IS NOT NULL
        if second_kw == "IS"
          third_kw = keyword_value(children[2])
          return third_kw == "NOT" ? !left.nil? : left.nil?
        end

        # BETWEEN low AND high
        if second_kw == "BETWEEN"
          low  = eval_expr(children[2], row_ctx)
          high = eval_expr(children[4], row_ctx)
          return nil if left.nil? || low.nil? || high.nil?
          return low <= left && left <= high
        end

        # IN (value_list)
        if second_kw == "IN"
          values = eval_value_list(find_descendant_rule(children, "value_list") || children[3], row_ctx)
          return nil if left.nil?
          return values.include?(left)
        end

        # LIKE
        if second_kw == "LIKE"
          pattern = eval_expr(children[2], row_ctx)
          return nil if left.nil? || pattern.nil?
          return like_match(left.to_s, pattern.to_s)
        end

        # NOT IN / NOT LIKE / NOT BETWEEN
        if second_kw == "NOT"
          third_kw = keyword_value(children[2])
          case third_kw
          when "BETWEEN"
            low  = eval_expr(children[3], row_ctx)
            high = eval_expr(children[5], row_ctx)
            return nil if left.nil? || low.nil? || high.nil?
            return !(low <= left && left <= high)
          when "IN"
            values = eval_value_list(find_descendant_rule(children, "value_list") || children[4], row_ctx)
            return nil if left.nil?
            return !values.include?(left)
          when "LIKE"
            pattern = eval_expr(children[3], row_ctx)
            return nil if left.nil? || pattern.nil?
            return !like_match(left.to_s, pattern.to_s)
          end
        end

        # Standard binary: left op right
        op  = get_op_string(children[1])
        right = eval_expr(children[2], row_ctx)
        return nil if left.nil? || right.nil?
        apply_cmp(op, left, right)
      end

      def self.apply_cmp(op, left, right)
        case op
        when "="  then left == right
        when "!=" then left != right
        when "<>" then left != right
        when "<"  then left < right
        when ">"  then left > right
        when "<=" then left <= right
        when ">=" then left >= right
        else false
        end
      end

      def self.eval_value_list(node, row_ctx)
        return [] if node.nil?
        if node.is_a?(Token)
          return [eval_expr(node, row_ctx)]
        end
        unless node.rule_name == "value_list"
          return [eval_expr(node, row_ctx)]
        end

        node.children.each_with_object([]) do |child, values|
          next if child.is_a?(Token) && child.value == ","

          values.concat(
            child.is_a?(ASTNode) && child.rule_name == "value_list" ?
              eval_value_list(child, row_ctx) :
              [eval_expr(child, row_ctx)]
          )
        end
      end

      def self.find_descendant_rule(children, rule_name)
        children.each do |child|
          next unless child.is_a?(ASTNode)
          return child if child.rule_name == rule_name

          descendant = find_descendant_rule(child.children, rule_name)
          return descendant if descendant
        end

        nil
      end

      # SQL LIKE pattern matching — supports % wildcard only.
      # Split on % wildcard (using -1 to keep trailing empty strings),
      # escape each literal part, join with .*, then anchor.
      def self.like_match(value, pattern)
        regex_str = pattern.split("%", -1).map { |part| Regexp.escape(part) }.join(".*")
        Regexp.new("\\A#{regex_str}\\z", Regexp::IGNORECASE).match?(value)
      end

      # ----------------------------------------------------------------
      # Arithmetic
      # ----------------------------------------------------------------

      def self.eval_additive(node, row_ctx)
        result = eval_expr(node.children[0], row_ctx)
        i = 1
        while i < node.children.size
          op_token = node.children[i]
          i += 1
          right = eval_expr(node.children[i], row_ctx)
          i += 1
          next result = nil if result.nil? || right.nil?
          op = op_token.is_a?(Token) ? op_token.value : op_token.to_s
          result = op == "+" ? result + right : result - right
        end
        result
      end

      def self.eval_multiplicative(node, row_ctx)
        result = eval_expr(node.children[0], row_ctx)
        i = 1
        while i < node.children.size
          op_token = node.children[i]
          i += 1
          right = eval_expr(node.children[i], row_ctx)
          i += 1
          next result = nil if result.nil? || right.nil?
          op = op_token.is_a?(Token) ? op_token.value : op_token.to_s
          result = case op
                   when "*" then result * right
                   when "/" then right != 0 ? result.to_f / right : nil
                   when "%" then right != 0 ? result % right : nil
                   else result
                   end
        end
        result
      end

      def self.eval_unary(node, row_ctx)
        children = node.children
        if children[0].is_a?(Token) && children[0].value == "-"
          val = eval_expr(children[1], row_ctx)
          val.nil? ? nil : -val
        else
          eval_expr(children[0], row_ctx)
        end
      end

      # ----------------------------------------------------------------
      # Primary
      # ----------------------------------------------------------------

      def self.eval_primary(node, row_ctx)
        children = node.children
        return nil if children.empty?

        first = children[0]

        # Parenthesized expression: "(" expr ")"
        if first.is_a?(Token) && first.value == "("
          return eval_expr(children[1], row_ctx)
        end

        # Sub-rule
        return eval_expr(first, row_ctx) if first.is_a?(ASTNode)

        # Raw token
        eval_token(first, row_ctx)
      end

      # ----------------------------------------------------------------
      # Column reference
      # ----------------------------------------------------------------

      def self.eval_column_ref(node, row_ctx)
        children = node.children
        if children.size == 1
          resolve_column(children[0].value, row_ctx)
        elsif children.size >= 3
          # table.column
          table = children[0].value
          col   = children[2].value
          qualified = "#{table}.#{col}"
          return row_ctx[qualified] if row_ctx.key?(qualified)
          resolve_column(col, row_ctx)
        else
          nil
        end
      end

      def self.resolve_column(name, row_ctx)
        return row_ctx[name] if row_ctx.key?(name)
        name_down = name.downcase
        row_ctx.each do |k, v|
          return v if k.to_s.downcase == name_down
        end
        raise ColumnNotFoundError.new(name)
      end

      # ----------------------------------------------------------------
      # Function call
      # ----------------------------------------------------------------

      def self.eval_function_call(node, row_ctx)
        func_name = node.children[0].value.upcase
        # Check for pre-computed aggregate value in row context
        agg_key = make_agg_key(func_name, node.children[2..-2], row_ctx)
        return row_ctx[agg_key] if agg_key && row_ctx.key?(agg_key)

        # Scalar functions
        arg = node.children[2]
        val = arg.is_a?(Token) && arg.value == "*" ? nil : eval_expr(arg, row_ctx)
        case func_name
        when "UPPER"  then val.is_a?(String) ? val.upcase : val
        when "LOWER"  then val.is_a?(String) ? val.downcase : val
        when "LENGTH" then val.is_a?(String) ? val.length : nil
        when "ABS"    then val.nil? ? nil : val.abs
        else nil
        end
      end

      def self.make_agg_key(func_name, arg_children, _row_ctx)
        return nil unless %w[COUNT SUM AVG MIN MAX].include?(func_name)
        parts = arg_children.filter_map do |c|
          next if c.is_a?(Token) && c.value == ","
          c.is_a?(Token) ? c.value : node_text(c)
        end
        "_agg_#{func_name}(#{parts.join})"
      end

      def self.node_text(node)
        return node.value if node.is_a?(Token)
        node.children.map { |c| node_text(c) }.join
      end

      # ----------------------------------------------------------------
      # Helper utilities
      # ----------------------------------------------------------------

      def self.keyword?(node_or_token, value)
        node_or_token.is_a?(Token) &&
          node_or_token.value.upcase == value.upcase
      end

      def self.keyword_value(node_or_token)
        return "" unless node_or_token.is_a?(Token)
        node_or_token.value.upcase
      end

      def self.get_op_string(node_or_token)
        return node_or_token.value if node_or_token.is_a?(Token)
        return "" if node_or_token.children.empty?
        node_or_token.children
                     .select { |c| c.is_a?(Token) }
                     .map(&:value)
                     .join
      end

      def self.token_type_name(token)
        if token.respond_to?(:type_name)
          token.type_name.to_s
        else
          token.token_type.to_s
        end
      end
    end
  end
end

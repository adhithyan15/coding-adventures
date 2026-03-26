# frozen_string_literal: true

# ================================================================
# Join Logic — five SQL join types
# ================================================================
#
# SQL Join Types:
#
#   INNER       — only rows matching the ON condition on both sides
#   LEFT OUTER  — all left rows; unmatched left rows get nil right cols
#   RIGHT OUTER — all right rows; unmatched right rows get nil left cols
#   FULL OUTER  — all rows from both sides; unmatched sides get nils
#   CROSS       — Cartesian product (every left × every right row)
#
# Implementation note:
#   This is a nested-loop join — O(|left| × |right|).
#   Suitable for an educational engine; production systems use
#   hash joins or merge joins for much better performance.
#
# Row representation:
#   Qualified names (alias.col) prevent key collisions when both
#   tables have a column named "id", for example.
# ================================================================

module CodingAdventures
  module SqlExecutionEngine
    # Stateless join helper.
    module Join
      # Perform a SQL join between left_rows and right_rows.
      #
      # @param left_rows   [Array<Hash>] already-qualified left rows
      # @param left_alias  [String]      table alias for left side
      # @param right_rows  [Array<Hash>] raw right rows
      # @param right_alias [String]      table alias for right side
      # @param join_type   [String]      "INNER", "LEFT", "RIGHT", "FULL", "CROSS"
      # @param on_condition [ASTNode | Token | nil]
      # @return [Array<Hash>]
      def self.perform(left_rows, left_alias, right_rows, right_alias, join_type, on_condition)
        if join_type == "CROSS" || on_condition.nil?
          return cross_join(left_rows, right_alias, right_rows)
        end

        null_left  = null_row_for(left_rows)
        null_right = null_row_for(right_rows, right_alias)

        case join_type
        when "INNER"
          inner_join(left_rows, right_rows, right_alias, on_condition)
        when "LEFT", "LEFT OUTER"
          left_join(left_rows, right_rows, right_alias, null_right, on_condition)
        when "RIGHT", "RIGHT OUTER"
          right_join(left_rows, right_rows, right_alias, null_left, on_condition)
        when "FULL", "FULL OUTER"
          full_join(left_rows, left_alias, right_rows, right_alias, null_left, null_right, on_condition)
        else
          inner_join(left_rows, right_rows, right_alias, on_condition)
        end
      end

      def self.inner_join(left_rows, right_rows, right_alias, condition)
        result = []
        left_rows.each do |lrow|
          right_rows.each do |rrow|
            merged = merge(lrow, right_alias, rrow)
            result << merged if test_condition(condition, merged)
          end
        end
        result
      end

      def self.left_join(left_rows, right_rows, right_alias, null_right, condition)
        result = []
        left_rows.each do |lrow|
          matched = false
          right_rows.each do |rrow|
            merged = merge(lrow, right_alias, rrow)
            if test_condition(condition, merged)
              result << merged
              matched = true
            end
          end
          result << merge(lrow, right_alias, null_right) unless matched
        end
        result
      end

      def self.right_join(left_rows, right_rows, right_alias, null_left, condition)
        result = []
        right_rows.each do |rrow|
          matched = false
          left_rows.each do |lrow|
            merged = merge(lrow, right_alias, rrow)
            if test_condition(condition, merged)
              result << merged
              matched = true
            end
          end
          result << merge(null_left, right_alias, rrow) unless matched
        end
        result
      end

      def self.full_join(left_rows, _left_alias, right_rows, right_alias, null_left, null_right, condition)
        result = []
        matched_right = Set.new

        left_rows.each do |lrow|
          left_matched = false
          right_rows.each_with_index do |rrow, i|
            merged = merge(lrow, right_alias, rrow)
            if test_condition(condition, merged)
              result << merged
              left_matched = true
              matched_right.add(i)
            end
          end
          result << merge(lrow, right_alias, null_right) unless left_matched
        end

        right_rows.each_with_index do |rrow, i|
          result << merge(null_left, right_alias, rrow) unless matched_right.include?(i)
        end

        result
      end

      def self.cross_join(left_rows, right_alias, right_rows)
        left_rows.flat_map do |lrow|
          right_rows.map { |rrow| merge(lrow, right_alias, rrow) }
        end
      end

      # Merge a left row and a right row.
      # Right columns are stored with qualified names (right_alias.col)
      # and also as bare names for convenience.
      def self.merge(left, right_alias, right)
        merged = left.dup
        right.each do |key, val|
          if key.include?(".")
            merged[key] = val
          else
            merged["#{right_alias}.#{key}"] = val
            merged[key] = val
          end
        end
        merged
      end

      # Build a NULL-value row (all values nil) from the schema of rows.
      def self.null_row_for(rows, _alias = nil)
        return {} if rows.empty?
        rows[0].transform_values { nil }
      end

      # Evaluate the ON condition against a merged row.
      def self.test_condition(condition, merged)
        val = Expression.eval_expr(condition, merged)
        val ? true : false
      end
    end
  end
end

# frozen_string_literal: true

# ================================================================
# Aggregate Functions — COUNT, SUM, AVG, MIN, MAX
# ================================================================
#
# Aggregate functions operate on a GROUP of rows and return a single
# summary value. They are used with GROUP BY (or on the entire table).
#
# NULL-skipping semantics:
#   All aggregate functions except COUNT(*) ignore NULL values.
#   If all values are NULL, SUM/AVG/MIN/MAX return nil; COUNT returns 0.
#
# Usage (called by Executor):
#
#   agg_specs = [["COUNT", "*"], ["SUM", "salary"]]
#   result = Aggregate.compute(rows, agg_specs)
#   # => {"_agg_COUNT(*)" => 3, "_agg_SUM(salary)" => 260000}
# ================================================================

module CodingAdventures
  module SqlExecutionEngine
    # Stateless aggregate computation helper.
    module Aggregate
      # Compute aggregate functions over a group of rows.
      #
      # @param rows       [Array<Hash>] rows in the current group
      # @param agg_specs  [Array<[String,String]>] e.g. [["COUNT","*"],["SUM","salary"]]
      # @return [Hash] mapping "_agg_FUNC(arg)" keys to their computed values
      def self.compute(rows, agg_specs)
        result = {}
        agg_specs.each do |(func_name, arg)|
          key = "_agg_#{func_name.upcase}(#{arg})"
          result[key] = compute_one(rows, func_name.upcase, arg)
        end
        result
      end

      # Compute a single aggregate function over rows.
      def self.compute_one(rows, func, arg)
        case func
        when "COUNT"
          if arg == "*"
            rows.size
          else
            rows.count { |row| get_val(row, arg) != nil }
          end
        when "SUM"
          vals = rows.filter_map { |row| get_val(row, arg) }
          vals.empty? ? nil : vals.sum
        when "AVG"
          vals = rows.filter_map { |row| get_val(row, arg) }
          vals.empty? ? nil : vals.sum.to_f / vals.size
        when "MIN"
          vals = rows.filter_map { |row| get_val(row, arg) }
          vals.empty? ? nil : vals.min
        when "MAX"
          vals = rows.filter_map { |row| get_val(row, arg) }
          vals.empty? ? nil : vals.max
        else
          nil
        end
      end

      # Get a column value from a row by name (case-insensitive).
      def self.get_val(row, col)
        return row[col] if row.key?(col)
        col_down = col.downcase
        row.each do |k, v|
          return v if k.to_s.downcase == col_down
        end
        nil
      end
    end
  end
end

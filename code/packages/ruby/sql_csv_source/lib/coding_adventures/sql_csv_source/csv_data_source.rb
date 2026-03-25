# frozen_string_literal: true

# =============================================================================
# CsvDataSource — SQL DataSource adapter backed by CSV files
# =============================================================================
#
# This class implements the DataSource mixin from
# CodingAdventures::SqlExecutionEngine.  It is intentionally thin: all the
# complex SQL logic (filtering, joining, grouping, ordering) lives in the
# execution engine.  This adapter's only jobs are:
#
#   1. Map a table_name to a file path: {directory}/{table_name}.csv
#   2. Parse the CSV text into row hashes via CsvParser
#   3. Coerce each string value to its natural Ruby/SQL type
#   4. Report missing tables as TableNotFoundError
#
# Directory layout:
#
#   data/
#     employees.csv
#     departments.csv
#
# Query: "SELECT * FROM employees" → reads data/employees.csv
#
# Type Coercion
# -------------
# CSV is untyped — every field is a String.  The engine needs Ruby values
# to evaluate expressions like WHERE salary > 80000 or WHERE active = true.
#
#   | CSV string  | Ruby value      |
#   |-------------|-----------------|
#   | ""          | nil  (SQL NULL) |
#   | "true"      | true            |
#   | "false"     | false           |
#   | "42"        | 42 (Integer)    |
#   | "3.14"      | 3.14 (Float)    |
#   | "hello"     | "hello" (String)|
#
# Column Ordering
# ---------------
# schema() reads the first line of the file directly and splits on comma.
# This avoids any Hash ordering ambiguity and is fast (no full parse needed).
# =============================================================================

module CodingAdventures
  module SqlCsvSource
    # DataSource adapter backed by CSV files in a directory.
    #
    # Each file +tablename.csv+ in +directory+ is one queryable table.
    # Column names come from the CSV header row.  Values are type-coerced
    # from Strings to the most appropriate Ruby/SQL type.
    #
    # @example Basic usage
    #   source = CsvDataSource.new("data/")
    #   result = CodingAdventures::SqlExecutionEngine.execute(
    #     "SELECT * FROM employees WHERE active = true",
    #     source
    #   )
    #   result.rows.each { |row| puts row["name"] }
    #
    class CsvDataSource
      include CodingAdventures::SqlExecutionEngine::DataSource

      # @param directory [String] path to the directory containing *.csv files
      def initialize(directory)
        # Store the directory path as a String for File.join.
        @directory = directory.to_s
      end

      # Return the column names for +table_name+ in header order.
      #
      # Reads only the first line of the CSV file — fast and unambiguous.
      #
      # @param table_name [String]
      # @return [Array<String>]
      # @raise [CodingAdventures::SqlExecutionEngine::TableNotFoundError]
      def schema(table_name)
        path = resolve!(table_name)
        # Read just the first line: header row.
        first_line = File.open(path, "r:UTF-8", &:readline).chomp
        return [] if first_line.empty?
        # Split on comma to get ordered column names.
        first_line.split(",").map(&:strip)
      end

      # Return all data rows from +table_name+ with type-coerced values.
      #
      # @param table_name [String]
      # @return [Array<Hash{String => Object}>]
      # @raise [CodingAdventures::SqlExecutionEngine::TableNotFoundError]
      def scan(table_name)
        path = resolve!(table_name)
        content = File.read(path, encoding: "UTF-8")
        # parse_csv returns Array<Hash{String => String}> — all values strings.
        str_rows = CodingAdventures::CsvParser.parse_csv(content)
        # Coerce every value from String to its natural Ruby type.
        str_rows.map { |row| row.transform_values { |v| coerce(v) } }
      end

      private

      # Build the CSV file path; raise TableNotFoundError if it doesn't exist.
      #
      # @param table_name [String]
      # @return [String] the absolute/relative file path
      # @raise [CodingAdventures::SqlExecutionEngine::TableNotFoundError]
      def resolve!(table_name)
        path = File.join(@directory, "#{table_name}.csv")
        unless File.exist?(path)
          raise CodingAdventures::SqlExecutionEngine::TableNotFoundError.new(table_name)
        end
        path
      end

      # Coerce a single CSV string value to the most appropriate Ruby type.
      #
      # Rules applied in priority order:
      #   1. "" → nil          (SQL NULL)
      #   2. "true"  → true    (boolean)
      #   3. "false" → false   (boolean)
      #   4. /\A-?\d+\z/ → Integer  (whole number)
      #   5. /\A-?\d+\.\d+\z/ → Float  (decimal number)
      #   6. otherwise → String (keep as-is)
      #
      # Why booleans before numbers?
      #   In Ruby, true.to_i raises NoMethodError, but we check booleans
      #   by string comparison, so this is safe.  Putting booleans first
      #   also avoids any confusion with "0"/"1" booleans from other systems.
      #
      # @param value [String]
      # @return [nil, true, false, Integer, Float, String]
      def coerce(value)
        # ── NULL ──────────────────────────────────────────────────────────────
        return nil if value == ""

        # ── Boolean ───────────────────────────────────────────────────────────
        return true  if value == "true"
        return false if value == "false"

        # ── Integer ───────────────────────────────────────────────────────────
        # Regex matches optional leading minus + one or more digits, nothing else.
        # This avoids treating "3.14" or "12abc" as integers.
        return Integer(value) if value.match?(/\A-?\d+\z/)

        # ── Float ─────────────────────────────────────────────────────────────
        # Matches optional minus + digits + dot + digits.
        return Float(value) if value.match?(/\A-?\d+\.\d+\z/)

        # ── String fallthrough ─────────────────────────────────────────────────
        value
      end
    end
  end
end

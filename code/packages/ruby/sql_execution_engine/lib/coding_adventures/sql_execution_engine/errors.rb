# frozen_string_literal: true

# ================================================================
# Error Classes for the SQL Execution Engine
# ================================================================
#
# All errors inherit from ExecutionError so callers can catch the
# whole family with a single rescue clause:
#
#   begin
#     result = SqlExecutionEngine.execute(sql, source)
#   rescue CodingAdventures::SqlExecutionEngine::ExecutionError => e
#     puts "SQL failed: #{e.message}"
#   end
#
# Error Hierarchy:
#
#   ExecutionError          (base)
#   ├── TableNotFoundError  (unknown table name)
#   └── ColumnNotFoundError (unknown column reference)
# ================================================================

module CodingAdventures
  module SqlExecutionEngine
    # Base class for all SQL execution engine errors.
    class ExecutionError < StandardError; end

    # Raised when the DataSource does not recognize a table name.
    #
    # Attributes:
    #   table_name [String] — the table that was not found.
    class TableNotFoundError < ExecutionError
      attr_reader :table_name

      def initialize(table_name)
        @table_name = table_name
        super("Table not found: #{table_name.inspect}")
      end
    end

    # Raised when a column reference cannot be resolved.
    #
    # Attributes:
    #   column_name [String] — the column that was not found.
    class ColumnNotFoundError < ExecutionError
      attr_reader :column_name

      def initialize(column_name)
        @column_name = column_name
        super("Column not found: #{column_name.inspect}")
      end
    end
  end
end

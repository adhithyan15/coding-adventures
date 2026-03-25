# frozen_string_literal: true

# =============================================================================
# coding_adventures_sql_csv_source — Main Entry Point
# =============================================================================
#
# Connects the SQL execution engine to CSV files on disk.
#
# Public API:
#
#   CodingAdventures::SqlCsvSource::CsvDataSource.new(directory)
#     Creates a DataSource backed by *.csv files in +directory+.
#     Pass it to SqlExecutionEngine.execute to run SQL queries.
#
# Example:
#
#   require "coding_adventures/sql_csv_source"
#
#   source = CodingAdventures::SqlCsvSource::CsvDataSource.new("data/")
#   result = CodingAdventures::SqlExecutionEngine.execute(
#     "SELECT name FROM employees WHERE dept_id IS NULL",
#     source
#   )
#   result.rows.each { |row| puts row["name"] }
#   # => Dave
#
# Dependency chain (must be loaded in this order):
#   csv_parser  →  sql_execution_engine  →  sql_csv_source
# =============================================================================

# Load the SQL execution engine first — CsvDataSource includes its DataSource
# mixin and raises its error types.
require "coding_adventures_sql_execution_engine"

# Load the CSV parser — used by CsvDataSource to read CSV files.
require "coding_adventures_csv_parser"

# Internal modules, in dependency order.
require_relative "sql_csv_source/version"
require_relative "sql_csv_source/csv_data_source"

module CodingAdventures
  # The SqlCsvSource module provides the CsvDataSource adapter.
  module SqlCsvSource
  end
end

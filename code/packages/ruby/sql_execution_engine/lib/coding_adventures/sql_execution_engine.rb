# frozen_string_literal: true

# ================================================================
# coding_adventures_sql_execution_engine — Main Entry Point
# ================================================================
#
# This file is the top-level require for the sql_execution_engine gem.
# It requires all components and exposes the public API:
#
#   CodingAdventures::SqlExecutionEngine.execute(sql, source)
#     -> QueryResult
#
#   CodingAdventures::SqlExecutionEngine.execute_all(sql, source)
#     -> Array<QueryResult>
#
# DataSource interface:
#
#   Include CodingAdventures::SqlExecutionEngine::DataSource in your
#   class and implement `schema(table_name)` and `scan(table_name)`.
# ================================================================

require "coding_adventures_sql_parser"

require_relative "sql_execution_engine/version"
require_relative "sql_execution_engine/errors"
require_relative "sql_execution_engine/result"
require_relative "sql_execution_engine/data_source"
require_relative "sql_execution_engine/aggregate"
require_relative "sql_execution_engine/expression"
require_relative "sql_execution_engine/join"
require_relative "sql_execution_engine/executor"

module CodingAdventures
  # The SQL execution engine module.
  module SqlExecutionEngine
    # Execute a single SQL SELECT statement against a data source.
    #
    # @param sql    [String]     the SQL text (one SELECT statement)
    # @param source [DataSource] the data provider
    # @return [QueryResult]
    def self.execute(sql, source)
      ast = CodingAdventures::SqlParser.parse_sql(sql)
      select_node = find_first_select(ast)
      return QueryResult.new(columns: [], rows: []) unless select_node

      Executor.new(source).execute(select_node)
    end

    # Execute all SELECT statements in a SQL string.
    #
    # @param sql    [String]     one or more semicolon-separated statements
    # @param source [DataSource] the data provider
    # @return [Array<QueryResult>]
    def self.execute_all(sql, source)
      ast = CodingAdventures::SqlParser.parse_sql(sql)
      executor = Executor.new(source)
      find_all_selects(ast).map { |stmt| executor.execute(stmt) }
    end

    # ---- Private helpers --------------------------------------------------

    def self.find_first_select(ast)
      return ast if ast.respond_to?(:rule_name) && ast.rule_name == "select_stmt"
      return nil unless ast.respond_to?(:children)
      ast.children.each do |child|
        result = find_first_select(child)
        return result if result
      end
      nil
    end

    def self.find_all_selects(ast)
      results = []
      collect_selects(ast, results)
      results
    end

    def self.collect_selects(node, results)
      return unless node.respond_to?(:rule_name)
      if node.rule_name == "select_stmt"
        results << node
        return
      end
      node.children.each { |c| collect_selects(c, results) }
    end

    private_class_method :find_first_select, :find_all_selects, :collect_selects
  end
end

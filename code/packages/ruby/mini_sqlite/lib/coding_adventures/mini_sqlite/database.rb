# frozen_string_literal: true

require "coding_adventures_sql_execution_engine"

module CodingAdventures
  module MiniSqlite
    ROW_ID = "__mini_sqlite_rowid"

    class InMemoryDatabase
      include CodingAdventures::SqlExecutionEngine::DataSource

      def initialize
        @tables = {}
      end

      def schema(table_name)
        table(table_name)[:columns].dup
      end

      def scan(table_name)
        table(table_name)[:rows].map(&:dup)
      end

      def snapshot
        @tables.transform_values do |table|
          {columns: table[:columns].dup, rows: table[:rows].map(&:dup)}
        end
      end

      def restore(snapshot)
        @tables = snapshot.transform_values do |table|
          {columns: table[:columns].dup, rows: table[:rows].map(&:dup)}
        end
      end

      def create(stmt)
        key = normalize(stmt[:table])
        if @tables.key?(key)
          return empty_result if stmt[:if_not_exists]

          raise OperationalError, "table already exists: #{stmt[:table]}"
        end
        seen = {}
        stmt[:columns].each do |column|
          normalized = normalize(column)
          raise ProgrammingError, "duplicate column: #{column}" if seen[normalized]

          seen[normalized] = true
        end
        @tables[key] = {columns: stmt[:columns].dup, rows: []}
        empty_result
      end

      def drop(stmt)
        key = normalize(stmt[:table])
        unless @tables.key?(key)
          return empty_result if stmt[:if_exists]

          raise OperationalError, "no such table: #{stmt[:table]}"
        end
        @tables.delete(key)
        empty_result
      end

      def insert(stmt)
        table_data = table(stmt[:table])
        columns = stmt[:columns] || table_data[:columns]
        assert_known_columns(table_data, columns)
        stmt[:rows].each do |values|
          if values.length != columns.length
            raise IntegrityError, "INSERT expected #{columns.length} values, got #{values.length}"
          end
          row = table_data[:columns].to_h { |column| [column, nil] }
          columns.each_with_index { |column, index| row[column] = values[index] }
          table_data[:rows] << row
        end
        {columns: [], rows: [], rows_affected: stmt[:rows].length}
      end

      def update(stmt)
        table_data = table(stmt[:table])
        assert_known_columns(table_data, stmt[:assignments].keys)
        row_ids = matching_row_ids(stmt[:table], stmt[:where])
        row_ids.each do |row_id|
          stmt[:assignments].each { |column, value| table_data[:rows][row_id][column] = value }
        end
        {columns: [], rows: [], rows_affected: row_ids.length}
      end

      def delete(stmt)
        table_data = table(stmt[:table])
        row_ids = matching_row_ids(stmt[:table], stmt[:where])
        remove = row_ids.to_h { |row_id| [row_id, true] }
        table_data[:rows] = table_data[:rows].each_with_index.reject { |_row, index| remove[index] }.map(&:first)
        {columns: [], rows: [], rows_affected: row_ids.length}
      end

      def select(sql)
        result = CodingAdventures::SqlExecutionEngine.execute(sql, self)
        {
          columns: result.columns,
          rows: result.rows.map { |row| result.columns.map { |column| row[column] } },
          rows_affected: -1
        }
      rescue StandardError => e
        raise MiniSqlite.translate_error(e)
      end

      private

      def matching_row_ids(table_name, where_sql)
        table_data = table(table_name)
        return table_data[:rows].each_index.to_a if where_sql.to_s.strip.empty?

        source = RowIdSource.new(table_name, table_data)
        result = CodingAdventures::SqlExecutionEngine.execute(
          "SELECT #{ROW_ID} FROM #{table_name} WHERE #{where_sql}",
          source
        )
        result.rows.map { |row| row[ROW_ID].to_i }
      rescue StandardError => e
        raise MiniSqlite.translate_error(e)
      end

      def table(table_name)
        @tables.fetch(normalize(table_name)) { raise OperationalError, "no such table: #{table_name}" }
      end

      def assert_known_columns(table_data, columns)
        known = table_data[:columns].map { |column| normalize(column) }.to_h { |column| [column, true] }
        columns.each do |column|
          raise OperationalError, "no such column: #{column}" unless known[normalize(column)]
        end
      end

      def normalize(name)
        name.to_s.downcase
      end

      def empty_result
        {columns: [], rows: [], rows_affected: 0}
      end
    end

    class RowIdSource
      include CodingAdventures::SqlExecutionEngine::DataSource

      def initialize(table_name, table_data)
        @table_name = table_name
        @table_data = table_data
      end

      def schema(table_name)
        assert_table(table_name)
        @table_data[:columns] + [ROW_ID]
      end

      def scan(table_name)
        assert_table(table_name)
        @table_data[:rows].each_with_index.map do |row, index|
          row.merge(ROW_ID => index)
        end
      end

      private

      def assert_table(table_name)
        return if table_name.to_s.downcase == @table_name.to_s.downcase

        raise CodingAdventures::SqlExecutionEngine::TableNotFoundError, table_name
      end
    end
  end
end

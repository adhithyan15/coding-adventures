# frozen_string_literal: true

module CodingAdventures
  module MiniSqlite
    class Connection
      def initialize(autocommit: false)
        @database = InMemoryDatabase.new
        @autocommit = autocommit
        @snapshot = nil
        @closed = false
      end

      def cursor
        assert_open!
        Cursor.new(self)
      end

      def execute(sql, params = [])
        cursor.execute(sql, params)
      end

      def executemany(sql, sequence_of_params)
        cursor.executemany(sql, sequence_of_params)
      end

      def commit
        assert_open!
        @snapshot = nil
        nil
      end

      def rollback
        assert_open!
        if @snapshot
          @database.restore(@snapshot)
          @snapshot = nil
        end
        nil
      end

      def close
        return nil if @closed

        @database.restore(@snapshot) if @snapshot
        @snapshot = nil
        @closed = true
        nil
      end

      def execute_bound(sql, params)
        assert_open!
        bound = Binding.substitute(sql, params)
        case SQL.first_keyword(bound)
        when "BEGIN"
          ensure_snapshot
          {columns: [], rows: [], rows_affected: 0}
        when "COMMIT"
          commit
          {columns: [], rows: [], rows_affected: 0}
        when "ROLLBACK"
          rollback
          {columns: [], rows: [], rows_affected: 0}
        when "SELECT"
          @database.select(bound)
        when "CREATE"
          ensure_snapshot
          @database.create(SQL.parse_create(bound))
        when "DROP"
          ensure_snapshot
          @database.drop(SQL.parse_drop(bound))
        when "INSERT"
          ensure_snapshot
          @database.insert(SQL.parse_insert(bound))
        when "UPDATE"
          ensure_snapshot
          @database.update(SQL.parse_update(bound))
        when "DELETE"
          ensure_snapshot
          @database.delete(SQL.parse_delete(bound))
        else
          raise ProgrammingError, "unsupported SQL statement"
        end
      end

      private

      def ensure_snapshot
        @snapshot = @database.snapshot if !@autocommit && @snapshot.nil?
      end

      def assert_open!
        raise ProgrammingError, "connection is closed" if @closed
      end
    end
  end
end

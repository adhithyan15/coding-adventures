# frozen_string_literal: true

module CodingAdventures
  module MiniSqlite
    class Cursor
      attr_reader :description, :rowcount, :lastrowid
      attr_accessor :arraysize

      def initialize(connection)
        @connection = connection
        @description = nil
        @rowcount = -1
        @lastrowid = nil
        @arraysize = 1
        @rows = []
        @offset = 0
        @closed = false
      end

      def execute(sql, params = [])
        assert_open!
        result = @connection.execute_bound(sql, params)
        @rows = result[:rows]
        @offset = 0
        @rowcount = result[:rows_affected]
        @description = result[:columns].empty? ? nil : result[:columns].map { |name| [name, nil, nil, nil, nil, nil, nil] }
        self
      end

      def executemany(sql, sequence_of_params)
        assert_open!
        total = 0
        sequence_of_params.each do |params|
          execute(sql, params)
          total += @rowcount if @rowcount.positive?
        end
        @rowcount = total unless sequence_of_params.empty?
        self
      end

      def fetchone
        assert_open!
        return nil if @offset >= @rows.length

        row = @rows[@offset]
        @offset += 1
        row
      end

      def fetchmany(size = nil)
        assert_open!
        size = @arraysize if size.nil? || size.negative?
        rows = @rows[@offset, size] || []
        @offset += rows.length
        rows
      end

      def fetchall
        assert_open!
        rows = @rows[@offset..] || []
        @offset = @rows.length
        rows
      end

      def close
        @closed = true
        @rows = []
        @description = nil
        nil
      end

      def each
        return enum_for(:each) unless block_given?

        while (row = fetchone)
          yield row
        end
      end

      private

      def assert_open!
        raise ProgrammingError, "cursor is closed" if @closed
      end
    end
  end
end

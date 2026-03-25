# frozen_string_literal: true

# ================================================================
# QueryResult — the output of a SQL SELECT execution
# ================================================================
#
# QueryResult bundles:
#   - columns [Array<String>] — output column names after AS aliases.
#   - rows    [Array<Hash>]   — result rows, each a Hash of col→value.
#
# Values may be nil (SQL NULL), Integer, Float, String, or TrueClass/FalseClass.
# ================================================================

module CodingAdventures
  module SqlExecutionEngine
    # The output of a successfully executed SELECT query.
    QueryResult = Data.define(:columns, :rows) do
      def initialize(columns: [], rows: [])
        super(columns: columns.freeze, rows: rows.freeze)
      end

      def to_s
        n = rows.size
        "QueryResult(columns=#{columns.inspect}, #{n} row#{n == 1 ? "" : "s"})"
      end
    end
  end
end

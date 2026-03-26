# frozen_string_literal: true

# ================================================================
# DataSource — the pluggable data interface
# ================================================================
#
# The SQL execution engine is decoupled from any particular storage
# system via the DataSource mixin module.
#
# To connect the engine to your data, include this module in your
# class and implement two methods:
#
#   schema(table_name) → Array<String>
#     Return the column names for the given table.
#     Raise TableNotFoundError for unknown tables.
#
#   scan(table_name) → Array<Hash>
#     Return all rows as an array of Hash objects.
#     Keys are column names; values may be nil, Integer, Float, String,
#     or TrueClass/FalseClass.
#     Raise TableNotFoundError for unknown tables.
#
# Example:
#
#   class MySource
#     include CodingAdventures::SqlExecutionEngine::DataSource
#
#     def schema(table_name)
#       raise TableNotFoundError.new(table_name) unless table_name == "users"
#       ["id", "name", "age"]
#     end
#
#     def scan(table_name)
#       raise TableNotFoundError.new(table_name) unless table_name == "users"
#       [{"id" => 1, "name" => "Alice", "age" => 30}]
#     end
#   end
# ================================================================

module CodingAdventures
  module SqlExecutionEngine
    # Mixin module that defines the DataSource interface.
    # Include this in your data-provider class and implement
    # `schema` and `scan`.
    module DataSource
      # Return the column names for the given table.
      #
      # @param table_name [String]
      # @return [Array<String>]
      # @raise [TableNotFoundError] if the table is unknown
      def schema(table_name)
        raise NotImplementedError,
              "#{self.class}#schema(#{table_name.inspect}) not implemented"
      end

      # Return all rows of a table as an array of hashes.
      #
      # @param table_name [String]
      # @return [Array<Hash>]
      # @raise [TableNotFoundError] if the table is unknown
      def scan(table_name)
        raise NotImplementedError,
              "#{self.class}#scan(#{table_name.inspect}) not implemented"
      end
    end
  end
end

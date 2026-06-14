# frozen_string_literal: true

module CodingAdventures
  module MiniSqlite
    class Warning < StandardError; end
    class Error < StandardError; end
    class InterfaceError < Error; end
    class DatabaseError < Error; end
    class DataError < DatabaseError; end
    class OperationalError < DatabaseError; end
    class IntegrityError < DatabaseError; end
    class InternalError < DatabaseError; end
    class ProgrammingError < DatabaseError; end
    class NotSupportedError < DatabaseError; end

    def self.translate_error(error)
      return error if error.is_a?(Error)

      message = error.message
      case error.class.name
      when /TableNotFound|ColumnNotFound/
        OperationalError.new(message)
      else
        ProgrammingError.new(message)
      end
    end
  end
end

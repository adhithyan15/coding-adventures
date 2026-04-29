# frozen_string_literal: true

require_relative "mini_sqlite/version"
require_relative "mini_sqlite/errors"
require_relative "mini_sqlite/binding"
require_relative "mini_sqlite/sql"
require_relative "mini_sqlite/database"
require_relative "mini_sqlite/connection"
require_relative "mini_sqlite/cursor"

module CodingAdventures
  module MiniSqlite
    APILEVEL = "2.0"
    THREADSAFETY = 1
    PARAMSTYLE = "qmark"

    module_function

    def connect(database = ":memory:", autocommit: false)
      raise NotSupportedError, "Ruby mini-sqlite supports only :memory: in Level 0" unless database == ":memory:"

      Connection.new(autocommit:)
    end
  end
end

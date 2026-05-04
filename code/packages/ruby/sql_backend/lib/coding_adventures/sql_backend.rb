# frozen_string_literal: true

require_relative "sql_backend/version"

module CodingAdventures
  module SqlBackend
    no_default = Object.new
    def no_default.inspect
      "NO_DEFAULT"
    end
    NO_DEFAULT = no_default.freeze

    TransactionHandle = Struct.new(:value, keyword_init: true)

    class Blob
      include Comparable

      attr_reader :bytes

      def initialize(bytes)
        @bytes =
          if bytes.is_a?(Array)
            bytes.pack("C*")
          else
            bytes.to_s.b.dup
          end.freeze
      end

      def <=>(other)
        return nil unless other.is_a?(Blob)

        bytes <=> other.bytes
      end

      def ==(other)
        other.is_a?(Blob) && other.bytes == bytes
      end
      alias_method :eql?, :==

      def hash
        bytes.hash
      end

      def inspect
        "Blob(#{bytes.bytes.inspect})"
      end
    end

    module SqlValues
      module_function

      def sql_value?(value)
        value.nil? ||
          value == true ||
          value == false ||
          value.is_a?(Integer) ||
          (value.is_a?(Float) && value.finite?) ||
          value.is_a?(String) ||
          value.is_a?(Blob)
      end

      def type_name(value)
        case value
        when nil
          "NULL"
        when true, false
          "BOOLEAN"
        when Integer
          "INTEGER"
        when Float
          "REAL"
        when String
          "TEXT"
        when Blob
          "BLOB"
        else
          raise TypeError, "not a SqlValue: #{value.inspect} (#{value.class})"
        end
      end

      def compare(left, right)
        left_type = type_rank(left)
        right_type = type_rank(right)
        rank_compare = left_type <=> right_type
        return rank_compare unless rank_compare == 0

        case left_type
        when 0
          0
        when 1
          bool_value(left) <=> bool_value(right)
        when 2
          left <=> right
        when 3
          left.to_f <=> right.to_f
        when 4
          left <=> right
        when 5
          left <=> right
        else
          raise TypeError, "not comparable: #{left.inspect}, #{right.inspect}"
        end
      end

      def type_rank(value)
        case value
        when nil
          0
        when true, false
          1
        when Integer
          2
        when Float
          3
        when String
          4
        when Blob
          5
        else
          raise TypeError, "not a SqlValue: #{value.inspect} (#{value.class})"
        end
      end

      def bool_value(value)
        value ? 1 : 0
      end
    end

    class BackendError < StandardError
    end

    class TableNotFound < BackendError
      attr_reader :table

      def initialize(table = nil, **keywords)
        @table = table || keywords.fetch(:table)
        super("table not found: #{@table.inspect}")
      end

      def ==(other)
        other.is_a?(TableNotFound) && other.table == table
      end
    end

    class TableAlreadyExists < BackendError
      attr_reader :table

      def initialize(table = nil, **keywords)
        @table = table || keywords.fetch(:table)
        super("table already exists: #{@table.inspect}")
      end

      def ==(other)
        other.is_a?(TableAlreadyExists) && other.table == table
      end
    end

    class ColumnNotFound < BackendError
      attr_reader :table, :column

      def initialize(table = nil, column = nil, **keywords)
        @table = table || keywords.fetch(:table)
        @column = column || keywords.fetch(:column)
        super("column not found: #{@table.inspect}.#{@column.inspect}")
      end

      def ==(other)
        other.is_a?(ColumnNotFound) && other.table == table && other.column == column
      end
    end

    class ColumnAlreadyExists < BackendError
      attr_reader :table, :column

      def initialize(table = nil, column = nil, **keywords)
        @table = table || keywords.fetch(:table)
        @column = column || keywords.fetch(:column)
        super("column already exists: #{@table.inspect}.#{@column.inspect}")
      end

      def ==(other)
        other.is_a?(ColumnAlreadyExists) && other.table == table && other.column == column
      end
    end

    class ConstraintViolation < BackendError
      attr_reader :table, :column

      def initialize(message:, table: nil, column: nil)
        @table = table
        @column = column
        super(message)
      end

      def ==(other)
        other.is_a?(ConstraintViolation) &&
          other.table == table &&
          other.column == column &&
          other.message == message
      end
    end

    class Unsupported < BackendError
      attr_reader :operation

      def initialize(operation = nil, **keywords)
        @operation = operation || keywords.fetch(:operation)
        super("operation not supported: #{@operation}")
      end

      def ==(other)
        other.is_a?(Unsupported) && other.operation == operation
      end
    end

    class Internal < BackendError
      attr_reader :detail

      def initialize(message = nil, **keywords)
        @detail = message || keywords.fetch(:message)
        super(@detail)
      end

      def ==(other)
        other.is_a?(Internal) && other.detail == detail
      end
    end

    class IndexAlreadyExists < BackendError
      attr_reader :index

      def initialize(index = nil, **keywords)
        @index = index || keywords.fetch(:index)
        super("index already exists: #{@index.inspect}")
      end

      def ==(other)
        other.is_a?(IndexAlreadyExists) && other.index == index
      end
    end

    class IndexNotFound < BackendError
      attr_reader :index

      def initialize(index = nil, **keywords)
        @index = index || keywords.fetch(:index)
        super("index not found: #{@index.inspect}")
      end

      def ==(other)
        other.is_a?(IndexNotFound) && other.index == index
      end
    end

    class TriggerAlreadyExists < BackendError
      attr_reader :name

      def initialize(name = nil, **keywords)
        @name = name || keywords.fetch(:name)
        super("trigger already exists: #{@name.inspect}")
      end

      def ==(other)
        other.is_a?(TriggerAlreadyExists) && other.name == name
      end
    end

    class TriggerNotFound < BackendError
      attr_reader :name

      def initialize(name = nil, **keywords)
        @name = name || keywords.fetch(:name)
        super("trigger not found: #{@name.inspect}")
      end

      def ==(other)
        other.is_a?(TriggerNotFound) && other.name == name
      end
    end

    class ColumnDef
      attr_reader :name,
        :type_name,
        :not_null,
        :primary_key,
        :unique,
        :autoincrement,
        :default,
        :check_expression,
        :foreign_key

      def initialize(
        name:,
        type_name:,
        not_null: false,
        primary_key: false,
        unique: false,
        autoincrement: false,
        default: NO_DEFAULT,
        check_expression: nil,
        foreign_key: nil
      )
        @name = name.to_s
        @type_name = type_name.to_s
        @not_null = not_null
        @primary_key = primary_key
        @unique = unique
        @autoincrement = autoincrement
        @default = default
        @check_expression = check_expression
        @foreign_key = foreign_key
      end

      def self.with_default(**keywords)
        new(**keywords)
      end

      def effective_not_null?
        not_null || primary_key
      end

      def effective_unique?
        unique || primary_key
      end

      def has_default?
        !default.equal?(NO_DEFAULT)
      end

      def ==(other)
        other.is_a?(ColumnDef) &&
          other.name == name &&
          other.type_name == type_name &&
          other.not_null == not_null &&
          other.primary_key == primary_key &&
          other.unique == unique &&
          other.autoincrement == autoincrement &&
          other.default == default &&
          other.check_expression == check_expression &&
          other.foreign_key == foreign_key
      end

      def dup
        ColumnDef.new(
          name: name,
          type_name: type_name,
          not_null: not_null,
          primary_key: primary_key,
          unique: unique,
          autoincrement: autoincrement,
          default: SqlBackend.copy_sql_value(default),
          check_expression: check_expression,
          foreign_key: foreign_key
        )
      end
    end

    class IndexDef
      attr_reader :name, :table, :columns, :unique, :auto

      def initialize(name:, table:, columns: [], unique: false, auto: false)
        @name = name.to_s
        @table = table.to_s
        @columns = columns.map(&:to_s).freeze
        @unique = unique
        @auto = auto
      end

      def ==(other)
        other.is_a?(IndexDef) &&
          other.name == name &&
          other.table == table &&
          other.columns == columns &&
          other.unique == unique &&
          other.auto == auto
      end

      def dup
        IndexDef.new(name: name, table: table, columns: columns.dup, unique: unique, auto: auto)
      end
    end

    class TriggerDef
      attr_reader :name, :table, :timing, :event, :body

      def initialize(name:, table:, timing:, event:, body:)
        @name = name.to_s
        @table = table.to_s
        @timing = timing.to_s.upcase
        @event = event.to_s.upcase
        @body = body.to_s
      end

      def ==(other)
        other.is_a?(TriggerDef) &&
          other.name == name &&
          other.table == table &&
          other.timing == timing &&
          other.event == event &&
          other.body == body
      end

      def dup
        TriggerDef.new(name: name, table: table, timing: timing, event: event, body: body)
      end
    end

    class RowIterator
      include Enumerable

      def next
        raise NotImplementedError, "#{self.class}#next not implemented"
      end

      def close
      end

      def each
        return enum_for(:each) unless block_given?

        while (row = self.next)
          yield row
        end
      ensure
        close
      end
    end

    class Cursor
      def next
        raise NotImplementedError, "#{self.class}#next not implemented"
      end

      def current_row
        raise NotImplementedError, "#{self.class}#current_row not implemented"
      end

      def current_index
        raise NotImplementedError, "#{self.class}#current_index not implemented"
      end

      def table_key
        nil
      end

      def adjust_after_delete
      end
    end

    class ListRowIterator < RowIterator
      def initialize(rows)
        @rows = rows.map { |row| SqlBackend.copy_row(row) }
        @index = 0
        @closed = false
      end

      def next
        return nil if @closed || @index >= @rows.length

        row = SqlBackend.copy_row(@rows[@index])
        @index += 1
        row
      end

      def close
        @closed = true
      end
    end

    class ListCursor < Cursor
      def initialize(rows, table_key: nil)
        @rows = rows.map { |row| SqlBackend.copy_row(row) }
        @table_key = table_key
        @index = -1
      end

      attr_reader :table_key

      def next
        @index += 1
        current_row
      end

      def current_row
        return nil unless @index.between?(0, @rows.length - 1)

        SqlBackend.copy_row(@rows[@index])
      end

      def current_index
        @index
      end

      def adjust_after_delete
        @index -= 1 if @index >= 0
      end
    end

    class Backend
      def tables
        raise NotImplementedError, "#{self.class}#tables not implemented"
      end

      def columns(table)
        raise NotImplementedError, "#{self.class}#columns(#{table.inspect}) not implemented"
      end

      def scan(table)
        raise NotImplementedError, "#{self.class}#scan(#{table.inspect}) not implemented"
      end

      def insert(table, row)
        raise NotImplementedError, "#{self.class}#insert(#{table.inspect}, #{row.inspect}) not implemented"
      end

      def update(table, cursor, assignments)
        raise NotImplementedError, "#{self.class}#update(#{table.inspect}, #{cursor.inspect}, #{assignments.inspect}) not implemented"
      end

      def delete(table, cursor)
        raise NotImplementedError, "#{self.class}#delete(#{table.inspect}, #{cursor.inspect}) not implemented"
      end

      def create_table(table, columns, if_not_exists:)
        raise NotImplementedError, "#{self.class}#create_table(#{table.inspect}, #{columns.inspect}, if_not_exists: #{if_not_exists}) not implemented"
      end

      def drop_table(table, if_exists:)
        raise NotImplementedError, "#{self.class}#drop_table(#{table.inspect}, if_exists: #{if_exists}) not implemented"
      end

      def add_column(table, column)
        raise NotImplementedError, "#{self.class}#add_column(#{table.inspect}, #{column.inspect}) not implemented"
      end

      def create_index(index)
        raise NotImplementedError, "#{self.class}#create_index(#{index.inspect}) not implemented"
      end

      def drop_index(name, if_exists: false)
        raise NotImplementedError, "#{self.class}#drop_index(#{name.inspect}, if_exists: #{if_exists}) not implemented"
      end

      def list_indexes(table = nil)
        raise NotImplementedError, "#{self.class}#list_indexes(#{table.inspect}) not implemented"
      end

      def scan_index(index_name, lo, hi, lo_inclusive: true, hi_inclusive: true)
        raise NotImplementedError, "#{self.class}#scan_index(#{index_name.inspect}, #{lo.inspect}, #{hi.inspect}, lo_inclusive: #{lo_inclusive}, hi_inclusive: #{hi_inclusive}) not implemented"
      end

      def scan_by_rowids(table, rowids)
        raise NotImplementedError, "#{self.class}#scan_by_rowids(#{table.inspect}, #{rowids.inspect}) not implemented"
      end

      def begin_transaction
        raise NotImplementedError, "#{self.class}#begin_transaction not implemented"
      end

      def commit(handle)
        raise NotImplementedError, "#{self.class}#commit(#{handle.inspect}) not implemented"
      end

      def rollback(handle)
        raise NotImplementedError, "#{self.class}#rollback(#{handle.inspect}) not implemented"
      end

      def current_transaction
        nil
      end

      def create_savepoint(_name)
        raise Unsupported, "savepoints"
      end

      def release_savepoint(_name)
        raise Unsupported, "savepoints"
      end

      def rollback_to_savepoint(_name)
        raise Unsupported, "savepoints"
      end

      def create_trigger(_definition)
        raise Unsupported, "triggers"
      end

      def drop_trigger(_name, if_exists: false)
        raise Unsupported, "triggers" unless if_exists
      end

      def list_triggers(_table)
        []
      end
    end

    class SchemaProvider
      def columns(table)
        raise NotImplementedError, "#{self.class}#columns(#{table.inspect}) not implemented"
      end
    end

    class BackendSchemaProvider < SchemaProvider
      def initialize(backend)
        @backend = backend
      end

      def columns(table)
        @backend.columns(table).map(&:name)
      end

      def list_indexes(table)
        @backend.list_indexes(table)
      end
    end

    def self.backend_as_schema_provider(backend)
      BackendSchemaProvider.new(backend)
    end

    TableState = Struct.new(:name, :columns, :rows, :next_rowid, keyword_init: true)

    class TableCursor < Cursor
      def initialize(table_key, state)
        @table_key = table_key
        @state = state
        @index = -1
      end

      attr_reader :table_key

      def next
        @index += 1
        current_row
      end

      def current_row
        record = current_record
        record ? SqlBackend.copy_row(record[:row]) : nil
      end

      def current_index
        @index
      end

      def current_record
        return nil unless @index.between?(0, @state.rows.length - 1)

        @state.rows[@index]
      end

      def adjust_after_delete
        @index -= 1 if @index >= 0
      end
    end

    class InMemoryBackend < Backend
      attr_reader :schema_version, :current_transaction
      attr_accessor :user_version

      def initialize
        @tables = {}
        @indexes = {}
        @triggers = {}
        @triggers_by_table = Hash.new { |hash, key| hash[key] = [] }
        @user_version = 0
        @schema_version = 0
        @transaction_snapshot = nil
        @current_transaction = nil
        @next_transaction = 1
        @savepoints = []
      end

      def tables
        @tables.values.map(&:name)
      end

      def columns(table)
        table_state(table).columns.map(&:dup)
      end

      def scan(table)
        state = table_state(table)
        ListRowIterator.new(state.rows.map { |record| record[:row] })
      end

      def open_cursor(table)
        key = normalize_name(table)
        state = table_state(table)
        TableCursor.new(key, state)
      end

      def insert(table, row)
        state = table_state(table)
        candidate = materialize_row(state, row)
        validate_row!(state, candidate)
        record = {rowid: state.next_rowid, row: candidate}
        state.next_rowid += 1
        state.rows << record
      end

      def update(table, cursor, assignments)
        state = table_state(table)
        record = current_record_for!(state, cursor)
        candidate = SqlBackend.copy_row(record[:row])
        assignment_column_names(state, assignments).each do |column, value|
          validate_sql_value!(value)
          candidate[column.name] = SqlBackend.copy_sql_value(value)
        end
        validate_row!(state, candidate, skip_rowid: record[:rowid])
        record[:row] = candidate
      end

      def delete(table, cursor)
        state = table_state(table)
        record = current_record_for!(state, cursor)
        state.rows.delete(record)
        cursor.adjust_after_delete
      end

      def create_table(table, columns, if_not_exists:)
        key = normalize_name(table)
        if @tables.key?(key)
          return if if_not_exists

          raise TableAlreadyExists, table
        end

        seen = {}
        copied_columns = columns.map do |column|
          copied = column.dup
          column_key = normalize_name(copied.name)
          raise ColumnAlreadyExists.new(table, copied.name) if seen.key?(column_key)

          seen[column_key] = true
          copied
        end
        @tables[key] = TableState.new(name: table.to_s, columns: copied_columns, rows: [], next_rowid: 0)
        bump_schema_version
      end

      def drop_table(table, if_exists:)
        key = normalize_name(table)
        unless @tables.key?(key)
          return if if_exists

          raise TableNotFound, table
        end

        @tables.delete(key)
        @indexes.delete_if { |_index_key, index| normalize_name(index.table) == key }
        @triggers_by_table.delete(key)
        @triggers.delete_if { |_name, trigger| normalize_name(trigger.table) == key }
        bump_schema_version
      end

      def add_column(table, column)
        state = table_state(table)
        if find_column(state, column.name)
          raise ColumnAlreadyExists.new(state.name, column.name)
        end
        if column.effective_not_null? && !column.has_default? && state.rows.any?
          raise ConstraintViolation.new(
            table: state.name,
            column: column.name,
            message: "NOT NULL constraint failed: #{state.name}.#{column.name}"
          )
        end

        copied = column.dup
        state.columns << copied
        value = copied.has_default? ? copied.default : nil
        state.rows.each do |record|
          record[:row][copied.name] = SqlBackend.copy_sql_value(value)
        end
        bump_schema_version
      end

      def create_index(index)
        key = normalize_name(index.name)
        raise IndexAlreadyExists, index.name if @indexes.key?(key)

        state = table_state(index.table)
        index.columns.each do |column_name|
          raise ColumnNotFound.new(state.name, column_name) unless find_column(state, column_name)
        end

        copied = index.dup
        validate_unique_index!(state, copied) if copied.unique
        @indexes[key] = copied
        bump_schema_version
      end

      def drop_index(name, if_exists: false)
        key = normalize_name(name)
        unless @indexes.key?(key)
          return if if_exists

          raise IndexNotFound, name
        end

        @indexes.delete(key)
        bump_schema_version
      end

      def list_indexes(table = nil)
        indexes = @indexes.values
        if table
          table_key = normalize_name(table)
          indexes = indexes.select { |index| normalize_name(index.table) == table_key }
        end
        indexes.map(&:dup)
      end

      def scan_index(index_name, lo, hi, lo_inclusive: true, hi_inclusive: true)
        index = @indexes[normalize_name(index_name)]
        raise IndexNotFound, index_name unless index

        state = table_state(index.table)
        entries = state.rows.map do |record|
          [index.columns.map { |column| record[:row][real_column_name(state, column)] }, record[:rowid]]
        end
        entries.sort! do |left, right|
          key_compare = compare_keys(left[0], right[0])
          key_compare.zero? ? (left[1] <=> right[1]) : key_compare
        end
        entries.filter_map do |key, rowid|
          next unless within_lower_bound?(key, lo, lo_inclusive)
          next unless within_upper_bound?(key, hi, hi_inclusive)

          rowid
        end
      end

      def scan_by_rowids(table, rowids)
        state = table_state(table)
        by_rowid = state.rows.to_h { |record| [record[:rowid], record[:row]] }
        ListRowIterator.new(rowids.filter_map { |rowid| by_rowid[rowid] })
      end

      def begin_transaction
        raise Unsupported, "nested transactions" if @current_transaction

        @transaction_snapshot = snapshot_state
        @current_transaction = TransactionHandle.new(value: @next_transaction)
        @next_transaction += 1
        @current_transaction
      end

      def commit(handle)
        validate_transaction_handle!(handle)
        @transaction_snapshot = nil
        @current_transaction = nil
        @savepoints.clear
      end

      def rollback(handle)
        validate_transaction_handle!(handle)
        restore_state(@transaction_snapshot)
        @transaction_snapshot = nil
        @current_transaction = nil
        @savepoints.clear
      end

      def create_savepoint(name)
        raise Unsupported, "savepoints outside transaction" unless @current_transaction

        @savepoints << [name.to_s, snapshot_state]
      end

      def release_savepoint(name)
        index = savepoint_index(name)
        @savepoints = @savepoints[0...index]
      end

      def rollback_to_savepoint(name)
        index = savepoint_index(name)
        restore_state(@savepoints[index][1])
        @savepoints = @savepoints[0..index]
      end

      def create_trigger(definition)
        key = normalize_name(definition.name)
        raise TriggerAlreadyExists, definition.name if @triggers.key?(key)

        state = table_state(definition.table)
        copied = definition.dup
        @triggers[key] = copied
        @triggers_by_table[normalize_name(state.name)] << key
        bump_schema_version
      end

      def drop_trigger(name, if_exists: false)
        key = normalize_name(name)
        trigger = @triggers[key]
        unless trigger
          return if if_exists

          raise TriggerNotFound, name
        end

        @triggers.delete(key)
        @triggers_by_table[normalize_name(trigger.table)].delete(key)
        bump_schema_version
      end

      def list_triggers(table)
        keys = @triggers_by_table[normalize_name(table)]
        keys.filter_map { |key| @triggers[key]&.dup }
      end

      private

      def normalize_name(name)
        name.to_s.downcase
      end

      def bump_schema_version
        @schema_version += 1
      end

      def table_state(table)
        @tables.fetch(normalize_name(table)) { raise TableNotFound, table }
      end

      def materialize_row(state, row)
        row_hash = row.transform_keys(&:to_s)
        candidate = {}
        state.columns.each do |column|
          value =
            if row_hash.key?(column.name)
              row_hash[column.name]
            elsif (matched_key = row_hash.keys.find { |key| normalize_name(key) == normalize_name(column.name) })
              row_hash[matched_key]
            elsif column.autoincrement && column.primary_key
              next_autoincrement_value(state, column)
            elsif column.has_default?
              column.default
            end
          validate_sql_value!(value)
          candidate[column.name] = SqlBackend.copy_sql_value(value)
        end
        row_hash.keys.each do |name|
          raise ColumnNotFound.new(state.name, name) unless find_column(state, name)
        end
        candidate
      end

      def next_autoincrement_value(state, column)
        values = state.rows.map { |record| record[:row][column.name] }.select { |value| value.is_a?(Integer) }
        values.empty? ? 1 : values.max + 1
      end

      def assignment_column_names(state, assignments)
        assignments.map do |name, value|
          column = find_column(state, name)
          raise ColumnNotFound.new(state.name, name) unless column

          [column, value]
        end
      end

      def find_column(state, column_name)
        key = normalize_name(column_name)
        state.columns.find { |column| normalize_name(column.name) == key }
      end

      def real_column_name(state, column_name)
        column = find_column(state, column_name)
        raise ColumnNotFound.new(state.name, column_name) unless column

        column.name
      end

      def validate_sql_value!(value)
        return if SqlValues.sql_value?(value)

        raise TypeError, "not a SqlValue: #{value.inspect} (#{value.class})"
      end

      def validate_row!(state, candidate, skip_rowid: nil)
        state.columns.each do |column|
          value = candidate[column.name]
          if column.effective_not_null? && value.nil?
            raise ConstraintViolation.new(
              table: state.name,
              column: column.name,
              message: "NOT NULL constraint failed: #{state.name}.#{column.name}"
            )
          end

          next unless column.effective_unique?
          next if value.nil?

          state.rows.each do |record|
            next if record[:rowid] == skip_rowid
            next unless SqlValues.compare(record[:row][column.name], value).zero?

            constraint = column.primary_key ? "PRIMARY KEY" : "UNIQUE"
            raise ConstraintViolation.new(
              table: state.name,
              column: column.name,
              message: "#{constraint} constraint failed: #{state.name}.#{column.name}"
            )
          end
        end

        @indexes.values.each do |index|
          next unless index.unique
          next unless normalize_name(index.table) == normalize_name(state.name)

          validate_unique_index!(state, index, candidate: candidate, skip_rowid: skip_rowid)
        end
      end

      def validate_unique_index!(state, index, candidate: nil, skip_rowid: nil)
        if candidate
          candidate_key = index.columns.map { |column| candidate[real_column_name(state, column)] }
          return if candidate_key.any?(&:nil?)

          state.rows.each do |record|
            next if record[:rowid] == skip_rowid

            existing_key = index.columns.map { |column| record[:row][real_column_name(state, column)] }
            next unless compare_keys(existing_key, candidate_key).zero?

            raise ConstraintViolation.new(
              table: state.name,
              column: index.columns.join(","),
              message: "UNIQUE constraint failed: #{state.name}.#{index.columns.join(",")}"
            )
          end
          return
        end

        seen = {}
        state.rows.each do |record|
          key = index.columns.map { |column| record[:row][real_column_name(state, column)] }
          next if key.any?(&:nil?)

          comparable = key.map { |value| [SqlValues.type_rank(value), value] }
          if seen.key?(comparable)
            raise ConstraintViolation.new(
              table: state.name,
              column: index.columns.join(","),
              message: "UNIQUE constraint failed: #{state.name}.#{index.columns.join(",")}"
            )
          end
          seen[comparable] = record[:rowid]
        end
      end

      def current_record_for!(state, cursor)
        unless cursor.is_a?(TableCursor) && cursor.table_key == normalize_name(state.name)
          raise Internal, "cursor does not belong to table #{state.name.inspect}"
        end

        record = cursor.current_record
        raise Internal, "cursor is not positioned on a row" unless record

        record
      end

      def compare_keys(left, right)
        left.zip(right).each do |left_value, right_value|
          comparison = SqlValues.compare(left_value, right_value)
          return comparison unless comparison.zero?
        end
        left.length <=> right.length
      end

      def within_lower_bound?(key, lo, inclusive)
        return true if lo.nil?

        comparison = compare_keys(key, lo)
        comparison.positive? || (inclusive && comparison.zero?)
      end

      def within_upper_bound?(key, hi, inclusive)
        return true if hi.nil?

        comparison = compare_keys(key, hi)
        comparison.negative? || (inclusive && comparison.zero?)
      end

      def validate_transaction_handle!(handle)
        unless @current_transaction && handle == @current_transaction
          raise Internal, "invalid transaction handle"
        end
      end

      def savepoint_index(name)
        raise Unsupported, "savepoints outside transaction" unless @current_transaction

        index = @savepoints.rindex { |savepoint_name, _snapshot| savepoint_name == name.to_s }
        raise Internal, "savepoint not found: #{name}" unless index

        index
      end

      def snapshot_state
        {
          tables: @tables.transform_values { |state| copy_table_state(state) },
          indexes: @indexes.transform_values(&:dup),
          triggers: @triggers.transform_values(&:dup),
          triggers_by_table: @triggers_by_table.transform_values(&:dup),
          user_version: @user_version,
          schema_version: @schema_version
        }
      end

      def restore_state(snapshot)
        @tables = snapshot[:tables].transform_values { |state| copy_table_state(state) }
        @indexes = snapshot[:indexes].transform_values(&:dup)
        @triggers = snapshot[:triggers].transform_values(&:dup)
        @triggers_by_table = Hash.new { |hash, key| hash[key] = [] }
        snapshot[:triggers_by_table].each { |key, value| @triggers_by_table[key] = value.dup }
        @user_version = snapshot[:user_version]
        @schema_version = snapshot[:schema_version]
      end

      def copy_table_state(state)
        TableState.new(
          name: state.name,
          columns: state.columns.map(&:dup),
          rows: state.rows.map { |record| {rowid: record[:rowid], row: SqlBackend.copy_row(record[:row])} },
          next_rowid: state.next_rowid
        )
      end
    end

    def self.copy_sql_value(value)
      return NO_DEFAULT if value.equal?(NO_DEFAULT)

      case value
      when Blob
        Blob.new(value.bytes)
      when String
        value.dup
      else
        value
      end
    end

    def self.copy_row(row)
      row.to_h.transform_keys(&:to_s).transform_values { |value| copy_sql_value(value) }
    end
  end
end

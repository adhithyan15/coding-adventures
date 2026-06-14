local M = {}

M.VERSION = "0.1.0"

local NO_DEFAULT = {}
M.NO_DEFAULT = NO_DEFAULT
local unpack_values = table.unpack or unpack

local function raise(err)
  error(err, 0)
end

local function normalize_name(name)
  return string.lower(tostring(name))
end

local function copy_array(values)
  local out = {}
  for i, value in ipairs(values or {}) do out[i] = value end
  return out
end

local function backend_error(kind, message, fields)
  fields = fields or {}
  fields.kind = kind
  fields.message = message
  return fields
end

function M.table_not_found(table_name)
  return backend_error("TableNotFound", "table not found: " .. tostring(table_name), { table = table_name })
end

function M.table_already_exists(table_name)
  return backend_error("TableAlreadyExists", "table already exists: " .. tostring(table_name), { table = table_name })
end

function M.column_not_found(table_name, column)
  return backend_error("ColumnNotFound", "column not found: " .. tostring(table_name) .. "." .. tostring(column),
    { table = table_name, column = column })
end

function M.column_already_exists(table_name, column)
  return backend_error("ColumnAlreadyExists", "column already exists: " .. tostring(table_name) .. "." .. tostring(column),
    { table = table_name, column = column })
end

function M.constraint_violation(table_name, column, message)
  return backend_error("ConstraintViolation", message, { table = table_name, column = column })
end

function M.unsupported(operation)
  return backend_error("Unsupported", "operation not supported: " .. tostring(operation), { operation = operation })
end

function M.internal(message)
  return backend_error("Internal", tostring(message), {})
end

function M.index_already_exists(index)
  return backend_error("IndexAlreadyExists", "index already exists: " .. tostring(index), { index = index })
end

function M.index_not_found(index)
  return backend_error("IndexNotFound", "index not found: " .. tostring(index), { index = index })
end

function M.trigger_already_exists(name)
  return backend_error("TriggerAlreadyExists", "trigger already exists: " .. tostring(name), { name = name })
end

function M.trigger_not_found(name)
  return backend_error("TriggerNotFound", "trigger not found: " .. tostring(name), { name = name })
end

function M.blob(bytes)
  return { __sql_backend_blob = true, bytes = tostring(bytes or "") }
end

local function is_blob(value)
  return type(value) == "table" and value.__sql_backend_blob == true
end

M.is_blob = is_blob

function M.is_sql_value(value)
  local value_type = type(value)
  return value == nil
    or value_type == "boolean"
    or value_type == "number"
    or value_type == "string"
    or is_blob(value)
end

function M.sql_type_name(value)
  local value_type = type(value)
  if value == nil then return "NULL" end
  if value_type == "boolean" then return "BOOLEAN" end
  if value_type == "number" then
    return math.type and math.type(value) == "integer" and "INTEGER" or "REAL"
  end
  if value_type == "string" then return "TEXT" end
  if is_blob(value) then return "BLOB" end
  raise(M.internal("not a SqlValue: " .. tostring(value)))
end

local function type_rank(value)
  local value_type = type(value)
  if value == nil then return 0 end
  if value_type == "boolean" then return 1 end
  if value_type == "number" then return 2 end
  if value_type == "string" then return 3 end
  if is_blob(value) then return 4 end
  raise(M.internal("not a SqlValue: " .. tostring(value)))
end

function M.compare(left, right)
  local left_rank, right_rank = type_rank(left), type_rank(right)
  if left_rank ~= right_rank then return left_rank < right_rank and -1 or 1 end
  if left == nil then return 0 end
  if type(left) == "boolean" then
    if left == right then return 0 end
    return (left and 1 or 0) < (right and 1 or 0) and -1 or 1
  end
  local left_value = is_blob(left) and left.bytes or left
  local right_value = is_blob(right) and right.bytes or right
  if left_value == right_value then return 0 end
  return left_value < right_value and -1 or 1
end

function M.copy_value(value)
  if is_blob(value) then return M.blob(value.bytes) end
  return value
end

function M.copy_row(row)
  local out = {}
  for key, value in pairs(row or {}) do out[tostring(key)] = M.copy_value(value) end
  return out
end

function M.column_def(opts)
  opts = opts or {}
  return {
    name = tostring(opts.name),
    type_name = tostring(opts.type_name or ""),
    not_null = opts.not_null == true,
    primary_key = opts.primary_key == true,
    unique = opts.unique == true,
    autoincrement = opts.autoincrement == true,
    has_default = opts.has_default == true or opts.default ~= nil,
    default = opts.default,
    check_expression = opts.check_expression,
    foreign_key = opts.foreign_key,
  }
end

function M.effective_not_null(column)
  return column.not_null == true or column.primary_key == true
end

function M.effective_unique(column)
  return column.unique == true or column.primary_key == true
end

local function copy_column(column)
  return M.column_def({
    name = column.name,
    type_name = column.type_name,
    not_null = column.not_null,
    primary_key = column.primary_key,
    unique = column.unique,
    autoincrement = column.autoincrement,
    has_default = column.has_default,
    default = M.copy_value(column.default),
    check_expression = column.check_expression,
    foreign_key = column.foreign_key,
  })
end

function M.index_def(opts)
  opts = opts or {}
  return {
    name = tostring(opts.name),
    table = tostring(opts.table),
    columns = copy_array(opts.columns),
    unique = opts.unique == true,
    auto = opts.auto == true,
  }
end

local function copy_index(index)
  return M.index_def({
    name = index.name,
    table = index.table,
    columns = index.columns,
    unique = index.unique,
    auto = index.auto,
  })
end

function M.trigger_def(opts)
  opts = opts or {}
  return {
    name = tostring(opts.name),
    table = tostring(opts.table),
    timing = string.upper(tostring(opts.timing)),
    event = string.upper(tostring(opts.event)),
    body = tostring(opts.body or ""),
  }
end

local function copy_trigger(trigger)
  return M.trigger_def(trigger)
end

local ListRowIterator = {}
ListRowIterator.__index = ListRowIterator

function ListRowIterator.new(rows)
  local copied = {}
  for i, row in ipairs(rows or {}) do copied[i] = M.copy_row(row) end
  return setmetatable({ rows = copied, index = 1, closed = false }, ListRowIterator)
end

function ListRowIterator:next()
  if self.closed or self.index > #self.rows then return nil end
  local row = M.copy_row(self.rows[self.index])
  self.index = self.index + 1
  return row
end

function ListRowIterator:close()
  self.closed = true
end

function ListRowIterator:to_table()
  local rows = {}
  while true do
    local row = self:next()
    if not row then break end
    rows[#rows + 1] = row
  end
  self:close()
  return rows
end

M.ListRowIterator = ListRowIterator

local ListCursor = {}
ListCursor.__index = ListCursor

function ListCursor.new(rows, table_key)
  local copied = {}
  for i, row in ipairs(rows or {}) do copied[i] = M.copy_row(row) end
  return setmetatable({ rows = copied, index = 0, table_key = table_key }, ListCursor)
end

function ListCursor:next()
  self.index = self.index + 1
  return self:current_row()
end

function ListCursor:current_row()
  local row = self.rows[self.index]
  return row and M.copy_row(row) or nil
end

function ListCursor:current_index()
  return self.index
end

function ListCursor:adjust_after_delete()
  if self.index > 0 then self.index = self.index - 1 end
end

M.ListCursor = ListCursor

local TableCursor = {}
TableCursor.__index = TableCursor

function TableCursor.new(table_key, state)
  return setmetatable({ table_key = table_key, state = state, index = 0 }, TableCursor)
end

function TableCursor:next()
  self.index = self.index + 1
  return self:current_row()
end

function TableCursor:current_record()
  return self.state.rows[self.index]
end

function TableCursor:current_row()
  local record = self:current_record()
  return record and M.copy_row(record.row) or nil
end

function TableCursor:current_index()
  return self.index
end

function TableCursor:adjust_after_delete()
  if self.index > 0 then self.index = self.index - 1 end
end

local InMemoryBackend = {}
InMemoryBackend.__index = InMemoryBackend

local function copy_table_state(state)
  local columns, rows = {}, {}
  for i, column in ipairs(state.columns) do columns[i] = copy_column(column) end
  for i, record in ipairs(state.rows) do rows[i] = { rowid = record.rowid, row = M.copy_row(record.row) } end
  return { name = state.name, columns = columns, rows = rows, next_rowid = state.next_rowid }
end

function InMemoryBackend.new()
  return setmetatable({
    tables_by_key = {},
    indexes_by_key = {},
    triggers_by_key = {},
    triggers_by_table = {},
    user_version = 0,
    schema_version = 0,
    transaction_snapshot = nil,
    current_transaction = nil,
    next_transaction = 1,
    savepoints = {},
  }, InMemoryBackend)
end

function InMemoryBackend:bump_schema_version()
  self.schema_version = self.schema_version + 1
end

function InMemoryBackend:table_state(table_name)
  local state = self.tables_by_key[normalize_name(table_name)]
  if not state then raise(M.table_not_found(table_name)) end
  return state
end

function InMemoryBackend:tables()
  local names = {}
  for _, state in pairs(self.tables_by_key) do names[#names + 1] = state.name end
  table.sort(names)
  return names
end

function InMemoryBackend:columns(table_name)
  local state = self:table_state(table_name)
  local columns = {}
  for i, column in ipairs(state.columns) do columns[i] = copy_column(column) end
  return columns
end

function InMemoryBackend:scan(table_name)
  local state = self:table_state(table_name)
  local rows = {}
  for i, record in ipairs(state.rows) do rows[i] = record.row end
  return ListRowIterator.new(rows)
end

function InMemoryBackend:open_cursor(table_name)
  local key = normalize_name(table_name)
  return TableCursor.new(key, self:table_state(table_name))
end

function InMemoryBackend:find_column(state, column_name)
  local wanted = normalize_name(column_name)
  for _, column in ipairs(state.columns) do
    if normalize_name(column.name) == wanted then return column end
  end
  return nil
end

function InMemoryBackend:real_column_name(state, column_name)
  local column = self:find_column(state, column_name)
  if not column then raise(M.column_not_found(state.name, column_name)) end
  return column.name
end

function InMemoryBackend:next_autoincrement_value(state, column)
  local max_value = 0
  for _, record in ipairs(state.rows) do
    local value = record.row[column.name]
    if type(value) == "number" and value > max_value then max_value = value end
  end
  return max_value + 1
end

function InMemoryBackend:materialize_row(state, row)
  local candidate, seen = {}, {}
  for _, column in ipairs(state.columns) do
    local present, value = false, nil
    for key, row_value in pairs(row or {}) do
      if normalize_name(key) == normalize_name(column.name) then
        present, value = true, row_value
        break
      end
    end
    if not present then
      if column.autoincrement and column.primary_key then
        value = self:next_autoincrement_value(state, column)
      elseif column.has_default then
        value = column.default
      end
    end
    if not M.is_sql_value(value) then raise(M.internal("not a SqlValue: " .. tostring(value))) end
    candidate[column.name] = M.copy_value(value)
  end
  for key, _ in pairs(row or {}) do
    local column = self:find_column(state, key)
    if not column then raise(M.column_not_found(state.name, key)) end
    seen[normalize_name(column.name)] = true
  end
  return candidate
end

local function key_has_null(key)
  for i = 1, key.n do
    if key[i] == nil then return true end
  end
  return false
end

local function make_key(state, index, row, real_column_name)
  local key = { n = #index.columns }
  for i, column_name in ipairs(index.columns) do
    key[i] = row[real_column_name(state, column_name)]
  end
  return key
end

local function compare_keys(left, right)
  local max_len = math.max(left.n or #left, right.n or #right)
  for i = 1, max_len do
    local comparison = M.compare(left[i], right[i])
    if comparison ~= 0 then return comparison end
  end
  return 0
end

local function serialize_key(key)
  local parts = {}
  for i = 1, key.n do
    local value = key[i]
    if is_blob(value) then
      parts[i] = "blob:" .. value.bytes
    else
      parts[i] = M.sql_type_name(value) .. ":" .. tostring(value)
    end
  end
  return table.concat(parts, "\0")
end

function InMemoryBackend:validate_unique_index(state, index, candidate, skip_rowid)
  local real_name = function(s, c) return self:real_column_name(s, c) end
  if candidate then
    local candidate_key = make_key(state, index, candidate, real_name)
    if key_has_null(candidate_key) then return end
    for _, record in ipairs(state.rows) do
      if record.rowid ~= skip_rowid then
        local existing_key = make_key(state, index, record.row, real_name)
        if compare_keys(existing_key, candidate_key) == 0 then
          raise(M.constraint_violation(state.name, table.concat(index.columns, ","),
            "UNIQUE constraint failed: " .. state.name .. "." .. table.concat(index.columns, ",")))
        end
      end
    end
    return
  end

  local seen = {}
  for _, record in ipairs(state.rows) do
    local key = make_key(state, index, record.row, real_name)
    if not key_has_null(key) then
      local serialized = serialize_key(key)
      if seen[serialized] then
        raise(M.constraint_violation(state.name, table.concat(index.columns, ","),
          "UNIQUE constraint failed: " .. state.name .. "." .. table.concat(index.columns, ",")))
      end
      seen[serialized] = true
    end
  end
end

function InMemoryBackend:validate_row(state, candidate, skip_rowid)
  for _, column in ipairs(state.columns) do
    local value = candidate[column.name]
    if M.effective_not_null(column) and value == nil then
      raise(M.constraint_violation(state.name, column.name, "NOT NULL constraint failed: " .. state.name .. "." .. column.name))
    end
    if M.effective_unique(column) and value ~= nil then
      for _, record in ipairs(state.rows) do
        if record.rowid ~= skip_rowid and M.compare(record.row[column.name], value) == 0 then
          local constraint = column.primary_key and "PRIMARY KEY" or "UNIQUE"
          raise(M.constraint_violation(state.name, column.name, constraint .. " constraint failed: " .. state.name .. "." .. column.name))
        end
      end
    end
  end
  for _, index in pairs(self.indexes_by_key) do
    if index.unique and normalize_name(index.table) == normalize_name(state.name) then
      self:validate_unique_index(state, index, candidate, skip_rowid)
    end
  end
end

function InMemoryBackend:insert(table_name, row)
  local state = self:table_state(table_name)
  local candidate = self:materialize_row(state, row)
  self:validate_row(state, candidate, nil)
  state.rows[#state.rows + 1] = { rowid = state.next_rowid, row = candidate }
  state.next_rowid = state.next_rowid + 1
end

function InMemoryBackend:current_record_for(state, cursor)
  if getmetatable(cursor) ~= TableCursor or cursor.table_key ~= normalize_name(state.name) then
    raise(M.internal("cursor does not belong to table " .. state.name))
  end
  local record = cursor:current_record()
  if not record then raise(M.internal("cursor is not positioned on a row")) end
  return record
end

function InMemoryBackend:update(table_name, cursor, assignments)
  local state = self:table_state(table_name)
  local record = self:current_record_for(state, cursor)
  local candidate = M.copy_row(record.row)
  for name, value in pairs(assignments or {}) do
    local column = self:find_column(state, name)
    if not column then raise(M.column_not_found(state.name, name)) end
    if not M.is_sql_value(value) then raise(M.internal("not a SqlValue: " .. tostring(value))) end
    candidate[column.name] = M.copy_value(value)
  end
  self:validate_row(state, candidate, record.rowid)
  record.row = candidate
end

function InMemoryBackend:delete(table_name, cursor)
  local state = self:table_state(table_name)
  local record = self:current_record_for(state, cursor)
  for i, candidate in ipairs(state.rows) do
    if candidate == record then
      table.remove(state.rows, i)
      cursor:adjust_after_delete()
      return
    end
  end
end

function InMemoryBackend:create_table(table_name, columns, options)
  options = options or {}
  local key = normalize_name(table_name)
  if self.tables_by_key[key] then
    if options.if_not_exists then return end
    raise(M.table_already_exists(table_name))
  end
  local copied, seen = {}, {}
  for i, column in ipairs(columns or {}) do
    local column_key = normalize_name(column.name)
    if seen[column_key] then raise(M.column_already_exists(table_name, column.name)) end
    seen[column_key] = true
    copied[i] = copy_column(column)
  end
  self.tables_by_key[key] = { name = tostring(table_name), columns = copied, rows = {}, next_rowid = 1 }
  self:bump_schema_version()
end

function InMemoryBackend:drop_table(table_name, options)
  options = options or {}
  local key = normalize_name(table_name)
  if not self.tables_by_key[key] then
    if options.if_exists then return end
    raise(M.table_not_found(table_name))
  end
  self.tables_by_key[key] = nil
  for index_key, index in pairs(self.indexes_by_key) do
    if normalize_name(index.table) == key then self.indexes_by_key[index_key] = nil end
  end
  self.triggers_by_table[key] = nil
  for trigger_key, trigger in pairs(self.triggers_by_key) do
    if normalize_name(trigger.table) == key then self.triggers_by_key[trigger_key] = nil end
  end
  self:bump_schema_version()
end

function InMemoryBackend:add_column(table_name, column)
  local state = self:table_state(table_name)
  if self:find_column(state, column.name) then raise(M.column_already_exists(state.name, column.name)) end
  if M.effective_not_null(column) and not column.has_default and #state.rows > 0 then
    raise(M.constraint_violation(state.name, column.name, "NOT NULL constraint failed: " .. state.name .. "." .. column.name))
  end
  local copied = copy_column(column)
  state.columns[#state.columns + 1] = copied
  for _, record in ipairs(state.rows) do record.row[copied.name] = M.copy_value(copied.default) end
  self:bump_schema_version()
end

function InMemoryBackend:create_index(index)
  local key = normalize_name(index.name)
  if self.indexes_by_key[key] then raise(M.index_already_exists(index.name)) end
  local state = self:table_state(index.table)
  for _, column in ipairs(index.columns) do
    if not self:find_column(state, column) then raise(M.column_not_found(state.name, column)) end
  end
  local copied = copy_index(index)
  if copied.unique then self:validate_unique_index(state, copied, nil, nil) end
  self.indexes_by_key[key] = copied
  self:bump_schema_version()
end

function InMemoryBackend:drop_index(name, options)
  options = options or {}
  local key = normalize_name(name)
  if not self.indexes_by_key[key] then
    if options.if_exists then return end
    raise(M.index_not_found(name))
  end
  self.indexes_by_key[key] = nil
  self:bump_schema_version()
end

function InMemoryBackend:list_indexes(table_name)
  local out = {}
  local table_key = table_name and normalize_name(table_name) or nil
  for _, index in pairs(self.indexes_by_key) do
    if not table_key or normalize_name(index.table) == table_key then out[#out + 1] = copy_index(index) end
  end
  table.sort(out, function(a, b) return a.name < b.name end)
  return out
end

function InMemoryBackend:scan_index(index_name, lo, hi, options)
  options = options or {}
  local index = self.indexes_by_key[normalize_name(index_name)]
  if not index then raise(M.index_not_found(index_name)) end
  local state = self:table_state(index.table)
  local real_name = function(s, c) return self:real_column_name(s, c) end
  local entries = {}
  for _, record in ipairs(state.rows) do
    entries[#entries + 1] = { key = make_key(state, index, record.row, real_name), rowid = record.rowid }
  end
  table.sort(entries, function(left, right)
    local comparison = compare_keys(left.key, right.key)
    if comparison == 0 then return left.rowid < right.rowid end
    return comparison < 0
  end)
  local lo_key = lo and { n = #lo, unpack_values(lo) } or nil
  local hi_key = hi and { n = #hi, unpack_values(hi) } or nil
  local rowids = {}
  for _, entry in ipairs(entries) do
    local after_lo = true
    local before_hi = true
    if lo_key then
      local comparison = compare_keys(entry.key, lo_key)
      after_lo = comparison > 0 or ((options.lo_inclusive ~= false) and comparison == 0)
    end
    if hi_key then
      local comparison = compare_keys(entry.key, hi_key)
      before_hi = comparison < 0 or ((options.hi_inclusive ~= false) and comparison == 0)
    end
    if after_lo and before_hi then rowids[#rowids + 1] = entry.rowid end
  end
  return rowids
end

function InMemoryBackend:scan_by_rowids(table_name, rowids)
  local state = self:table_state(table_name)
  local by_rowid, rows = {}, {}
  for _, record in ipairs(state.rows) do by_rowid[record.rowid] = record.row end
  for _, rowid in ipairs(rowids or {}) do
    if by_rowid[rowid] then rows[#rows + 1] = by_rowid[rowid] end
  end
  return ListRowIterator.new(rows)
end

function InMemoryBackend:snapshot_state()
  local tables, indexes, triggers, triggers_by_table = {}, {}, {}, {}
  for key, state in pairs(self.tables_by_key) do tables[key] = copy_table_state(state) end
  for key, index in pairs(self.indexes_by_key) do indexes[key] = copy_index(index) end
  for key, trigger in pairs(self.triggers_by_key) do triggers[key] = copy_trigger(trigger) end
  for key, trigger_keys in pairs(self.triggers_by_table) do triggers_by_table[key] = copy_array(trigger_keys) end
  return {
    tables_by_key = tables,
    indexes_by_key = indexes,
    triggers_by_key = triggers,
    triggers_by_table = triggers_by_table,
    user_version = self.user_version,
    schema_version = self.schema_version,
  }
end

function InMemoryBackend:restore_state(snapshot)
  self.tables_by_key, self.indexes_by_key = {}, {}
  self.triggers_by_key, self.triggers_by_table = {}, {}
  for key, state in pairs(snapshot.tables_by_key) do self.tables_by_key[key] = copy_table_state(state) end
  for key, index in pairs(snapshot.indexes_by_key) do self.indexes_by_key[key] = copy_index(index) end
  for key, trigger in pairs(snapshot.triggers_by_key) do self.triggers_by_key[key] = copy_trigger(trigger) end
  for key, trigger_keys in pairs(snapshot.triggers_by_table) do self.triggers_by_table[key] = copy_array(trigger_keys) end
  self.user_version = snapshot.user_version
  self.schema_version = snapshot.schema_version
end

function InMemoryBackend:begin_transaction()
  if self.current_transaction then raise(M.unsupported("nested transactions")) end
  self.transaction_snapshot = self:snapshot_state()
  self.current_transaction = self.next_transaction
  self.next_transaction = self.next_transaction + 1
  return self.current_transaction
end

function InMemoryBackend:validate_transaction(handle)
  if self.current_transaction ~= handle then raise(M.internal("invalid transaction handle")) end
end

function InMemoryBackend:commit(handle)
  self:validate_transaction(handle)
  self.transaction_snapshot = nil
  self.current_transaction = nil
  self.savepoints = {}
end

function InMemoryBackend:rollback(handle)
  self:validate_transaction(handle)
  self:restore_state(self.transaction_snapshot)
  self.transaction_snapshot = nil
  self.current_transaction = nil
  self.savepoints = {}
end

function InMemoryBackend:create_savepoint(name)
  if not self.current_transaction then raise(M.unsupported("savepoints outside transaction")) end
  self.savepoints[#self.savepoints + 1] = { name = tostring(name), snapshot = self:snapshot_state() }
end

function InMemoryBackend:savepoint_index(name)
  if not self.current_transaction then raise(M.unsupported("savepoints outside transaction")) end
  for i = #self.savepoints, 1, -1 do
    if self.savepoints[i].name == tostring(name) then return i end
  end
  raise(M.internal("savepoint not found: " .. tostring(name)))
end

function InMemoryBackend:release_savepoint(name)
  local index = self:savepoint_index(name)
  for i = #self.savepoints, index, -1 do table.remove(self.savepoints, i) end
end

function InMemoryBackend:rollback_to_savepoint(name)
  local index = self:savepoint_index(name)
  self:restore_state(self.savepoints[index].snapshot)
  for i = #self.savepoints, index + 1, -1 do table.remove(self.savepoints, i) end
end

function InMemoryBackend:create_trigger(trigger)
  local key = normalize_name(trigger.name)
  if self.triggers_by_key[key] then raise(M.trigger_already_exists(trigger.name)) end
  local state = self:table_state(trigger.table)
  self.triggers_by_key[key] = copy_trigger(trigger)
  local table_key = normalize_name(state.name)
  self.triggers_by_table[table_key] = self.triggers_by_table[table_key] or {}
  table.insert(self.triggers_by_table[table_key], key)
  self:bump_schema_version()
end

function InMemoryBackend:drop_trigger(name, options)
  options = options or {}
  local key = normalize_name(name)
  local trigger = self.triggers_by_key[key]
  if not trigger then
    if options.if_exists then return end
    raise(M.trigger_not_found(name))
  end
  self.triggers_by_key[key] = nil
  local table_key = normalize_name(trigger.table)
  local keys = self.triggers_by_table[table_key] or {}
  for i = #keys, 1, -1 do
    if keys[i] == key then table.remove(keys, i) end
  end
  self:bump_schema_version()
end

function InMemoryBackend:list_triggers(table_name)
  local keys = self.triggers_by_table[normalize_name(table_name)] or {}
  local out = {}
  for _, key in ipairs(keys) do
    if self.triggers_by_key[key] then out[#out + 1] = copy_trigger(self.triggers_by_key[key]) end
  end
  return out
end

M.InMemoryBackend = InMemoryBackend

function M.backend_as_schema_provider(backend)
  return {
    columns = function(_, table_name)
      local columns = backend:columns(table_name)
      local names = {}
      for i, column in ipairs(columns) do names[i] = column.name end
      return names
    end,
    list_indexes = function(_, table_name)
      return backend:list_indexes(table_name)
    end,
  }
end

return M

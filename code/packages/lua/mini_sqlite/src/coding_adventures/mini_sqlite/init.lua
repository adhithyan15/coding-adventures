local sql_engine = require("coding_adventures.sql_execution_engine")

local M = {}

M.apilevel = "2.0"
M.threadsafety = 1
M.paramstyle = "qmark"

local ROW_ID_COLUMN = "__mini_sqlite_rowid"

local function err(kind, message)
  return { kind = kind, message = message }
end

local function trim(s)
  return (s:gsub("^%s+", ""):gsub("%s+$", ""))
end

local function normalize_name(name)
  return string.lower(name)
end

local function keyword_pattern(word)
  return (word:gsub("%a", function(c)
    return "[" .. string.lower(c) .. string.upper(c) .. "]"
  end))
end

local KW = {}
for _, word in ipairs({
  "CREATE", "TABLE", "IF", "NOT", "EXISTS", "DROP", "INSERT", "INTO",
  "VALUES", "UPDATE", "SET", "DELETE", "FROM", "WHERE"
}) do
  KW[word] = keyword_pattern(word)
end

local function strip_trailing_semicolon(sql)
  return trim(sql):gsub(";%s*$", "")
end

local function copy_row(row)
  local out = {}
  for k, v in pairs(row) do out[k] = v end
  return out
end

local function copy_table_data(table_data)
  local rows = {}
  for i, row in ipairs(table_data.rows) do rows[i] = copy_row(row) end
  local columns = {}
  for i, col in ipairs(table_data.columns) do columns[i] = col end
  return { columns = columns, rows = rows }
end

local function is_boundary_char(ch)
  return ch == "" or not ch:match("[%w_]")
end

local function first_keyword(sql)
  return (trim(sql):match("^([%a_]+)") or ""):upper()
end

local function quote_sql_string(value)
  return "'" .. tostring(value):gsub("'", "''") .. "'"
end

local function to_sql_literal(value)
  local t = type(value)
  if value == nil then return "NULL" end
  if t == "boolean" then return value and "TRUE" or "FALSE" end
  if t == "number" then return tostring(value) end
  if t == "string" then return quote_sql_string(value) end
  return nil, err("ProgrammingError", "unsupported parameter type: " .. t)
end

local function read_quoted(sql, i, quote)
  i = i + 1
  while i <= #sql do
    local ch = sql:sub(i, i)
    if ch == quote then
      if sql:sub(i + 1, i + 1) == quote then
        i = i + 2
      else
        return i + 1
      end
    else
      i = i + 1
    end
  end
  return #sql + 1
end

local function bind_parameters(sql, params)
  params = params or {}
  local out, index, i = {}, 1, 1
  while i <= #sql do
    local ch = sql:sub(i, i)
    if ch == "'" or ch == '"' then
      local next_i = read_quoted(sql, i, ch)
      out[#out + 1] = sql:sub(i, next_i - 1)
      i = next_i
    elseif ch == "-" and sql:sub(i, i + 1) == "--" then
      local next_i = i + 2
      while next_i <= #sql and sql:sub(next_i, next_i) ~= "\n" do next_i = next_i + 1 end
      out[#out + 1] = sql:sub(i, next_i - 1)
      i = next_i
    elseif ch == "/" and sql:sub(i, i + 1) == "/*" then
      local next_i = i + 2
      while next_i + 1 <= #sql and sql:sub(next_i, next_i + 1) ~= "*/" do next_i = next_i + 1 end
      next_i = math.min(next_i + 2, #sql + 1)
      out[#out + 1] = sql:sub(i, next_i - 1)
      i = next_i
    elseif ch == "?" then
      if index > #params then
        return nil, err("ProgrammingError", "not enough parameters for SQL statement")
      end
      local literal, literal_err = to_sql_literal(params[index])
      if not literal then return nil, literal_err end
      out[#out + 1] = literal
      index = index + 1
      i = i + 1
    else
      out[#out + 1] = ch
      i = i + 1
    end
  end
  if index <= #params then
    return nil, err("ProgrammingError", "too many parameters for SQL statement")
  end
  return table.concat(out)
end

local function split_top_level(text, delimiter)
  local parts, current = {}, {}
  local depth, quote, i = 0, nil, 1
  while i <= #text do
    local ch = text:sub(i, i)
    if quote then
      current[#current + 1] = ch
      if ch == quote then
        if text:sub(i + 1, i + 1) == quote then
          i = i + 1
          current[#current + 1] = text:sub(i, i)
        else
          quote = nil
        end
      end
    elseif ch == "'" or ch == '"' then
      quote = ch
      current[#current + 1] = ch
    elseif ch == "(" then
      depth = depth + 1
      current[#current + 1] = ch
    elseif ch == ")" then
      depth = math.max(depth - 1, 0)
      current[#current + 1] = ch
    elseif depth == 0 and ch == delimiter then
      local part = trim(table.concat(current))
      if part ~= "" then parts[#parts + 1] = part end
      current = {}
    else
      current[#current + 1] = ch
    end
    i = i + 1
  end
  local part = trim(table.concat(current))
  if part ~= "" then parts[#parts + 1] = part end
  return parts
end

local function split_top_level_keyword(text, keyword)
  local upper, key_len = string.upper(text), #keyword
  local depth, quote, i = 0, nil, 1
  while i <= #text do
    local ch = text:sub(i, i)
    if quote then
      if ch == quote then
        if text:sub(i + 1, i + 1) == quote then i = i + 1 else quote = nil end
      end
    elseif ch == "'" or ch == '"' then
      quote = ch
    elseif ch == "(" then
      depth = depth + 1
    elseif ch == ")" then
      depth = math.max(depth - 1, 0)
    elseif depth == 0 and upper:sub(i, i + key_len - 1) == keyword
      and is_boundary_char(text:sub(i - 1, i - 1))
      and is_boundary_char(text:sub(i + key_len, i + key_len)) then
      return trim(text:sub(1, i - 1)), trim(text:sub(i + key_len))
    end
    i = i + 1
  end
  return trim(text), ""
end

local function find_matching_paren(text, open_index)
  local depth, quote, i = 0, nil, open_index
  while i <= #text do
    local ch = text:sub(i, i)
    if quote then
      if ch == quote then
        if text:sub(i + 1, i + 1) == quote then i = i + 1 else quote = nil end
      end
    elseif ch == "'" or ch == '"' then
      quote = ch
    elseif ch == "(" then
      depth = depth + 1
    elseif ch == ")" then
      depth = depth - 1
      if depth == 0 then return i end
    end
    i = i + 1
  end
  return nil
end

local function parse_literal(text)
  local value = trim(text)
  local upper = value:upper()
  if upper == "NULL" then return nil end
  if upper == "TRUE" then return true end
  if upper == "FALSE" then return false end
  if value:sub(1, 1) == "'" and value:sub(-1) == "'" then
    return value:sub(2, -2):gsub("''", "'")
  end
  local number = tonumber(value)
  if number ~= nil then return number end
  error("expected literal value, got: " .. text)
end

local function identifier_at_start(text)
  return trim(text):match("^([%a_][%w_]*)")
end

local function parse_create(sql)
  local s = strip_trailing_semicolon(sql)
  local table_name, defs, if_not_exists
  table_name, defs = s:match("^%s*" .. KW.CREATE .. "%s+" .. KW.TABLE .. "%s+" ..
    KW.IF .. "%s+" .. KW.NOT .. "%s+" .. KW.EXISTS .. "%s+([%a_][%w_]*)%s*%((.*)%)%s*$")
  if table_name then
    if_not_exists = true
  else
    table_name, defs = s:match("^%s*" .. KW.CREATE .. "%s+" .. KW.TABLE .. "%s+([%a_][%w_]*)%s*%((.*)%)%s*$")
    if_not_exists = false
  end
  if not table_name then error("invalid CREATE TABLE statement") end
  local columns = {}
  for _, part in ipairs(split_top_level(defs, ",")) do
    local name = identifier_at_start(part)
    if name then columns[#columns + 1] = name end
  end
  if #columns == 0 then error("CREATE TABLE requires at least one column") end
  return { table = table_name, columns = columns, if_not_exists = if_not_exists }
end

local function parse_drop(sql)
  local s = strip_trailing_semicolon(sql)
  local table_name = s:match("^%s*" .. KW.DROP .. "%s+" .. KW.TABLE .. "%s+" ..
    KW.IF .. "%s+" .. KW.EXISTS .. "%s+([%a_][%w_]*)%s*$")
  if table_name then return { table = table_name, if_exists = true } end
  table_name = s:match("^%s*" .. KW.DROP .. "%s+" .. KW.TABLE .. "%s+([%a_][%w_]*)%s*$")
  if not table_name then error("invalid DROP TABLE statement") end
  return { table = table_name, if_exists = false }
end

local function parse_value_rows(sql)
  local rest, rows = trim(sql), {}
  while rest ~= "" do
    if rest:sub(1, 1) ~= "(" then error("INSERT VALUES rows must be parenthesized") end
    local close = find_matching_paren(rest, 1)
    if not close then error("unterminated INSERT VALUES row") end
    local inside, row = rest:sub(2, close - 1), {}
    for _, part in ipairs(split_top_level(inside, ",")) do row[#row + 1] = parse_literal(part) end
    if #row == 0 then error("INSERT row requires at least one value") end
    rows[#rows + 1] = row
    rest = trim(rest:sub(close + 1))
    if rest:sub(1, 1) == "," then rest = trim(rest:sub(2))
    elseif rest ~= "" then error("invalid text after INSERT row") end
  end
  if #rows == 0 then error("INSERT requires at least one row") end
  return rows
end

local function parse_insert(sql)
  local s = strip_trailing_semicolon(sql)
  local table_name, columns_sql, rows_sql = s:match("^%s*" .. KW.INSERT .. "%s+" ..
    KW.INTO .. "%s+([%a_][%w_]*)%s*%(([^)]*)%)%s+" .. KW.VALUES .. "%s+(.+)%s*$")
  local columns = {}
  if table_name then
    for _, part in ipairs(split_top_level(columns_sql, ",")) do columns[#columns + 1] = trim(part) end
  else
    table_name, rows_sql = s:match("^%s*" .. KW.INSERT .. "%s+" ..
      KW.INTO .. "%s+([%a_][%w_]*)%s+" .. KW.VALUES .. "%s+(.+)%s*$")
  end
  if not table_name then error("invalid INSERT statement") end
  return { table = table_name, columns = columns, rows = parse_value_rows(rows_sql) }
end

local function parse_update(sql)
  local s = strip_trailing_semicolon(sql)
  local table_name, rest = s:match("^%s*" .. KW.UPDATE .. "%s+([%a_][%w_]*)%s+" ..
    KW.SET .. "%s+(.+)%s*$")
  if not table_name then error("invalid UPDATE statement") end
  local assign_sql, where_sql = split_top_level_keyword(rest, "WHERE")
  local assignments = {}
  for _, assignment in ipairs(split_top_level(assign_sql, ",")) do
    local parts = split_top_level(assignment, "=")
    if #parts ~= 2 then error("invalid assignment: " .. assignment) end
    local col = trim(parts[1])
    if col:match("^[%a_][%w_]*$") == nil then error("invalid identifier: " .. col) end
    assignments[#assignments + 1] = { column = col, value = parse_literal(parts[2]) }
  end
  if #assignments == 0 then error("UPDATE requires at least one assignment") end
  return { table = table_name, assignments = assignments, where = where_sql }
end

local function parse_delete(sql)
  local s = strip_trailing_semicolon(sql)
  local table_name, where_sql = s:match("^%s*" .. KW.DELETE .. "%s+" ..
    KW.FROM .. "%s+([%a_][%w_]*)(.*)$")
  if not table_name then error("invalid DELETE statement") end
  where_sql = trim(where_sql or "")
  if where_sql ~= "" then
    local where_match = where_sql:match("^" .. KW.WHERE .. "%s+(.+)$")
    if not where_match then error("invalid DELETE statement") end
    where_sql = trim(where_match)
  end
  return { table = table_name, where = where_sql }
end

local Database = {}
Database.__index = Database

function Database.new()
  return setmetatable({ tables = {} }, Database)
end

function Database:snapshot()
  local out = { tables = {} }
  for name, table_data in pairs(self.tables) do out.tables[name] = copy_table_data(table_data) end
  return out
end

function Database:restore(snapshot)
  self.tables = {}
  for name, table_data in pairs(snapshot.tables) do self.tables[name] = copy_table_data(table_data) end
end

function Database:schema(table_name)
  local table_data = self.tables[normalize_name(table_name)]
  if not table_data then error("no such table: " .. tostring(table_name)) end
  local columns = {}
  for i, col in ipairs(table_data.columns) do columns[i] = col end
  return columns
end

function Database:scan(table_name)
  local table_data = self.tables[normalize_name(table_name)]
  if not table_data then error("no such table: " .. tostring(table_name)) end
  local rows = {}
  for i, row in ipairs(table_data.rows) do rows[i] = copy_row(row) end
  return rows
end

local function canonical_column(table_data, column)
  local wanted = normalize_name(column)
  for _, candidate in ipairs(table_data.columns) do
    if normalize_name(candidate) == wanted then return candidate end
  end
  error("no such column: " .. column)
end

function Database:create(stmt)
  local key = normalize_name(stmt.table)
  if self.tables[key] then
    if stmt.if_not_exists then return { columns = {}, rows = {}, rows_affected = 0 } end
    error("table already exists: " .. stmt.table)
  end
  local seen = {}
  for _, column in ipairs(stmt.columns) do
    local normalized = normalize_name(column)
    if seen[normalized] then error("duplicate column: " .. column) end
    seen[normalized] = true
  end
  self.tables[key] = { columns = stmt.columns, rows = {} }
  return { columns = {}, rows = {}, rows_affected = 0 }
end

function Database:drop(stmt)
  local key = normalize_name(stmt.table)
  if not self.tables[key] then
    if stmt.if_exists then return { columns = {}, rows = {}, rows_affected = 0 } end
    error("no such table: " .. stmt.table)
  end
  self.tables[key] = nil
  return { columns = {}, rows = {}, rows_affected = 0 }
end

function Database:insert(stmt)
  local table_data = self.tables[normalize_name(stmt.table)]
  if not table_data then error("no such table: " .. stmt.table) end
  local columns = {}
  if #stmt.columns == 0 then
    for i, col in ipairs(table_data.columns) do columns[i] = col end
  else
    for i, col in ipairs(stmt.columns) do columns[i] = canonical_column(table_data, col) end
  end
  for _, values in ipairs(stmt.rows) do
    if #values ~= #columns then error("INSERT expected " .. #columns .. " values, got " .. #values) end
    local row = {}
    for i, col in ipairs(columns) do row[col] = values[i] end
    table_data.rows[#table_data.rows + 1] = row
  end
  return { columns = {}, rows = {}, rows_affected = #stmt.rows }
end

local RowIdSource = {}
RowIdSource.__index = RowIdSource

function RowIdSource.new(db, table_name)
  return setmetatable({ db = db, table_name = table_name }, RowIdSource)
end

function RowIdSource:schema(table_name)
  if normalize_name(table_name) ~= normalize_name(self.table_name) then error("no such table: " .. table_name) end
  local cols = self.db:schema(table_name)
  cols[#cols + 1] = ROW_ID_COLUMN
  return cols
end

function RowIdSource:scan(table_name)
  if normalize_name(table_name) ~= normalize_name(self.table_name) then error("no such table: " .. table_name) end
  local rows = self.db:scan(table_name)
  for i, row in ipairs(rows) do row[ROW_ID_COLUMN] = i end
  return rows
end

function Database:matching_row_ids(table_name, where_sql)
  local table_data = self.tables[normalize_name(table_name)]
  if not table_data then error("no such table: " .. table_name) end
  if trim(where_sql or "") == "" then
    local ids = {}
    for i = 1, #table_data.rows do ids[#ids + 1] = i end
    return ids
  end
  local source = RowIdSource.new(self, table_name)
  local ok, result = sql_engine.execute("SELECT " .. ROW_ID_COLUMN .. " FROM " .. table_name .. " WHERE " .. where_sql, source)
  if not ok then error(result) end
  local ids = {}
  for _, row in ipairs(result.rows) do ids[#ids + 1] = row[1] end
  return ids
end

function Database:update(stmt)
  local table_data = self.tables[normalize_name(stmt.table)]
  if not table_data then error("no such table: " .. stmt.table) end
  local assignments = {}
  for i, assignment in ipairs(stmt.assignments) do
    assignments[i] = { column = canonical_column(table_data, assignment.column), value = assignment.value }
  end
  local ids, idset = self:matching_row_ids(stmt.table, stmt.where), {}
  for _, id in ipairs(ids) do idset[id] = true end
  for id, row in ipairs(table_data.rows) do
    if idset[id] then
      for _, assignment in ipairs(assignments) do row[assignment.column] = assignment.value end
    end
  end
  return { columns = {}, rows = {}, rows_affected = #ids }
end

function Database:delete(stmt)
  local table_data = self.tables[normalize_name(stmt.table)]
  if not table_data then error("no such table: " .. stmt.table) end
  local ids, idset = self:matching_row_ids(stmt.table, stmt.where), {}
  for _, id in ipairs(ids) do idset[id] = true end
  local rows = {}
  for id, row in ipairs(table_data.rows) do
    if not idset[id] then rows[#rows + 1] = row end
  end
  table_data.rows = rows
  return { columns = {}, rows = {}, rows_affected = #ids }
end

function Database:select_sql(sql)
  local ok, result = sql_engine.execute(sql, self)
  if not ok then error(result) end
  result.rows_affected = -1
  return result
end

local Cursor = {}
Cursor.__index = Cursor

function Cursor.new(conn)
  return setmetatable({
    conn = conn, description = {}, rowcount = -1, lastrowid = nil,
    arraysize = 1, rows = {}, offset = 1, closed = false,
  }, Cursor)
end

function Cursor:execute(sql, params)
  if self.closed then return nil, err("ProgrammingError", "cursor is closed") end
  local result, exec_err = self.conn:execute_bound(sql, params or {})
  if not result then return nil, exec_err end
  self.rows = result.rows or {}
  self.offset = 1
  self.rowcount = result.rows_affected or -1
  self.description = {}
  for i, col in ipairs(result.columns or {}) do self.description[i] = { name = col } end
  return self
end

function Cursor:executemany(sql, params_seq)
  local total = 0
  for _, params in ipairs(params_seq or {}) do
    local ok, exec_err = self:execute(sql, params)
    if not ok then return nil, exec_err end
    if self.rowcount > 0 then total = total + self.rowcount end
  end
  if params_seq and #params_seq > 0 then self.rowcount = total end
  return self
end

function Cursor:fetchone()
  if self.closed or self.offset > #self.rows then return nil end
  local row = self.rows[self.offset]
  self.offset = self.offset + 1
  return row
end

function Cursor:fetchmany(size)
  if self.closed then return {} end
  size = size or self.arraysize
  local rows = {}
  for _ = 1, size do
    local row = self:fetchone()
    if not row then break end
    rows[#rows + 1] = row
  end
  return rows
end

function Cursor:fetchall()
  if self.closed then return {} end
  local rows = {}
  while true do
    local row = self:fetchone()
    if not row then break end
    rows[#rows + 1] = row
  end
  return rows
end

function Cursor:close()
  self.closed = true
  self.rows = {}
  self.description = {}
end

local Connection = {}
Connection.__index = Connection

function Connection.new(options)
  return setmetatable({
    db = Database.new(),
    autocommit = options and options.autocommit or false,
    snapshot = nil,
    closed = false,
  }, Connection)
end

function Connection:assert_open()
  if self.closed then error("connection is closed") end
end

function Connection:ensure_snapshot()
  if not self.autocommit and self.snapshot == nil then self.snapshot = self.db:snapshot() end
end

function Connection:cursor()
  self:assert_open()
  return Cursor.new(self)
end

function Connection:execute(sql, params)
  local cursor = self:cursor()
  return cursor:execute(sql, params or {})
end

function Connection:executemany(sql, params_seq)
  local cursor = self:cursor()
  return cursor:executemany(sql, params_seq or {})
end

function Connection:commit()
  self:assert_open()
  self.snapshot = nil
  return true
end

function Connection:rollback()
  self:assert_open()
  if self.snapshot then
    self.db:restore(self.snapshot)
    self.snapshot = nil
  end
  return true
end

function Connection:close()
  if self.closed then return true end
  if self.snapshot then self.db:restore(self.snapshot) end
  self.snapshot = nil
  self.closed = true
  return true
end

function Connection:execute_bound(sql, params)
  local bound, bind_err = bind_parameters(sql, params or {})
  if not bound then return nil, bind_err end
  local keyword = first_keyword(bound)
  local ok, result = pcall(function()
    if keyword == "BEGIN" then
      self:ensure_snapshot()
      return { columns = {}, rows = {}, rows_affected = 0 }
    elseif keyword == "COMMIT" then
      self:commit()
      return { columns = {}, rows = {}, rows_affected = 0 }
    elseif keyword == "ROLLBACK" then
      self:rollback()
      return { columns = {}, rows = {}, rows_affected = 0 }
    elseif keyword == "SELECT" then
      return self.db:select_sql(bound)
    elseif keyword == "CREATE" then
      self:ensure_snapshot()
      return self.db:create(parse_create(bound))
    elseif keyword == "DROP" then
      self:ensure_snapshot()
      return self.db:drop(parse_drop(bound))
    elseif keyword == "INSERT" then
      self:ensure_snapshot()
      return self.db:insert(parse_insert(bound))
    elseif keyword == "UPDATE" then
      self:ensure_snapshot()
      return self.db:update(parse_update(bound))
    elseif keyword == "DELETE" then
      self:ensure_snapshot()
      return self.db:delete(parse_delete(bound))
    else
      error("unsupported SQL statement")
    end
  end)
  if ok then return result end
  return nil, err("OperationalError", tostring(result))
end

function M.connect(database, options)
  if database ~= ":memory:" then
    return nil, err("NotSupportedError", "Lua mini-sqlite supports only :memory: in Level 0")
  end
  return Connection.new(options or {})
end

M.Connection = Connection
M.Cursor = Cursor

return M

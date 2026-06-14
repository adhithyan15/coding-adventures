local csv_parser = require("coding_adventures.csv_parser")
local sql_engine = require("coding_adventures.sql_execution_engine")

local M = {}
M.VERSION = "0.1.0"

local CsvDataSource = {}
CsvDataSource.__index = CsvDataSource

local function trim(value)
  return (value:gsub("^%s+", ""):gsub("%s+$", ""))
end

local function join_path(directory, file_name)
  local base = tostring(directory or "")
  if base:sub(-1) == "/" or base:sub(-1) == "\\" then
    return base .. file_name
  end
  return base .. "/" .. file_name
end

local function read_file(path, table_name)
  local handle = io.open(path, "rb")
  if not handle then
    error("table not found: " .. tostring(table_name))
  end
  local content = handle:read("*a")
  handle:close()
  return content
end

local function resolve_path(directory, table_name)
  return join_path(directory, tostring(table_name) .. ".csv")
end

function M.coerce(value)
  if value == "" then
    return nil
  end

  local lower = string.lower(value)
  if lower == "true" then
    return true
  end
  if lower == "false" then
    return false
  end

  if value:match("^%-?%d+$") then
    return tonumber(value)
  end
  if value:match("^%-?%d+%.%d+$") then
    return tonumber(value)
  end

  return value
end

function CsvDataSource.new(directory)
  return setmetatable({ directory = tostring(directory) }, CsvDataSource)
end

function CsvDataSource:schema(table_name)
  local content = read_file(resolve_path(self.directory, table_name), table_name)
  local rows = csv_parser.parse(content)
  local header = rows[1] or {}
  local schema = {}
  for index, name in ipairs(header) do
    schema[index] = trim(name)
  end
  return schema
end

function CsvDataSource:scan(table_name)
  local content = read_file(resolve_path(self.directory, table_name), table_name)
  local parsed = csv_parser.parse(content)
  local header = parsed[1] or {}
  local rows = {}

  for row_index = 2, #parsed do
    local parsed_row = parsed[row_index]
    local row = {}
    for col_index, name in ipairs(header) do
      row[trim(name)] = M.coerce(parsed_row[col_index] or "")
    end
    rows[#rows + 1] = row
  end

  return rows
end

function M.execute_csv(sql, directory)
  local source = CsvDataSource.new(directory)
  return sql_engine.execute(sql, source)
end

M.CsvDataSource = CsvDataSource

return M

-- Fixture-only trace adapter, run temporarily by the Lua package BUILD.
local simulator = require("unix_history")

local function quote(value)
  return '"' .. value:gsub('[%z\1-\31\\"]', function(char)
    return string.format("\\u%04x", string.byte(char))
  end) .. '"'
end

-- The trace has only strings, numbers, event records and arrays.
local function encode(value, array)
  local kind = type(value)
  if kind == "string" then return quote(value) end
  if kind == "number" then return tostring(value) end
  local items = {}
  if array then
    for i, item in ipairs(value) do items[i] = encode(item, false) end
    return "[" .. table.concat(items, ",") .. "]"
  end
  for key, item in pairs(value) do
    local is_array = key == "events" or key == "names"
    items[#items + 1] = quote(key) .. ":" .. encode(item, is_array)
  end
  return "{" .. table.concat(items, ",") .. "}"
end

local command = #arg > 0 and table.concat(arg, " ") or "cat /hello"
print(encode(simulator.run(command), false))

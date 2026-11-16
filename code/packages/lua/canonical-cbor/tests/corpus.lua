-- ============================================================================
-- Test helper: minimal JSON decoder and hex converter for canonical-cbor-v1
-- (code/specs/fixtures/canonical-cbor-v1/cases.json).
--
-- Pure Lua, zero external dependencies.
-- ============================================================================

local M = {}

local function decode_json(text)
  local pos = 1

  local function skip_whitespace()
    pos = text:find("[^ \t\r\n]", pos) or (#text + 1)
  end

  local value

  local function string_value()
    local stop = pos + 1
    local parts = {}
    while true do
      local c = text:sub(stop, stop)
      if c == "" then
        error("unterminated string in JSON", 0)
      end
      if c == '"' then
        break
      end
      if c == "\\" then
        stop = stop + 1
        c = text:sub(stop, stop)
      end
      parts[#parts + 1] = c
      stop = stop + 1
    end
    pos = stop + 1
    return table.concat(parts)
  end

  function value()
    skip_whitespace()
    local c = text:sub(pos, pos)
    if c == "{" then
      local obj = {}
      pos = pos + 1
      skip_whitespace()
      if text:sub(pos, pos) == "}" then
        pos = pos + 1
        return obj
      end
      while true do
        skip_whitespace()
        local key = string_value()
        skip_whitespace()
        assert(text:sub(pos, pos) == ":", "expected ':' in JSON object")
        pos = pos + 1
        obj[key] = value()
        skip_whitespace()
        local d = text:sub(pos, pos)
        pos = pos + 1
        if d == "}" then
          return obj
        end
        assert(d == ",", "expected ',' or '}' in JSON object")
      end
    elseif c == "[" then
      local arr = {}
      pos = pos + 1
      skip_whitespace()
      if text:sub(pos, pos) == "]" then
        pos = pos + 1
        return arr
      end
      while true do
        arr[#arr + 1] = value()
        skip_whitespace()
        local d = text:sub(pos, pos)
        pos = pos + 1
        if d == "]" then
          return arr
        end
        assert(d == ",", "expected ',' or ']' in JSON array")
      end
    elseif c == '"' then
      return string_value()
    elseif text:sub(pos, pos + 3) == "true" then
      pos = pos + 4
      return true
    elseif text:sub(pos, pos + 4) == "false" then
      pos = pos + 5
      return false
    elseif text:sub(pos, pos + 3) == "null" then
      pos = pos + 4
      return nil
    else
      local number = text:match("^-?%d+", pos)
      assert(number ~= nil, "unexpected JSON token at offset " .. pos)
      pos = pos + #number
      return math.tointeger(tonumber(number))
    end
  end

  local result = value()
  skip_whitespace()
  assert(pos == #text + 1, "trailing non-whitespace content in JSON")
  return result
end

M.decode_json = decode_json

--- Convert lowercase hex string into raw binary Lua string.
--- @param hex string
--- @return string
function M.hex_to_bytes(hex)
  assert(#hex % 2 == 0, "odd-length hex string: " .. tostring(hex))
  return (hex:gsub("..", function(byte_str)
    return string.char(tonumber(byte_str, 16))
  end))
end

--- Convert raw binary Lua string into lowercase hex string.
--- @param s string
--- @return string
function M.bytes_to_hex(s)
  return (s:gsub(".", function(c)
    return string.format("%02x", string.byte(c))
  end))
end

--- Load the canonical CBOR v1 test suite fixture.
--- @return table
function M.load_corpus()
  local path = "../../../../specs/fixtures/canonical-cbor-v1/cases.json"
  local f = assert(io.open(path, "rb"), "cannot open " .. path)
  local text = f:read("a")
  f:close()
  return decode_json(text)
end

return M

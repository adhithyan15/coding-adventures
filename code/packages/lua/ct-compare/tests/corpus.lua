-- Test-only helper: a minimal JSON reader for the CT01 corpus
-- (code/specs/fixtures/ct-compare-v1/cases.json).  Lua's standard library has
-- no JSON decoder and the library under test must stay dependency free, so the
-- tests carry a small one.  It handles exactly what the corpus uses: objects,
-- arrays, strings without escapes beyond \" and \\, true/false and integers.

local M = {}

local function decode(text)
  local pos = 1

  local function skip()
    pos = text:find("[^ \t\r\n]", pos) or (#text + 1)
  end

  local value

  local function string_value()
    local stop = pos + 1
    local parts = {}
    while true do
      local c = text:sub(stop, stop)
      if c == "" then error("unterminated string") end
      if c == '"' then break end
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
    skip()
    local c = text:sub(pos, pos)
    if c == "{" then
      local obj = {}
      pos = pos + 1
      skip()
      if text:sub(pos, pos) == "}" then pos = pos + 1 return obj end
      while true do
        skip()
        local key = string_value()
        skip()
        assert(text:sub(pos, pos) == ":", "expected ':'")
        pos = pos + 1
        obj[key] = value()
        skip()
        local d = text:sub(pos, pos)
        pos = pos + 1
        if d == "}" then return obj end
        assert(d == ",", "expected ',' or '}'")
      end
    elseif c == "[" then
      local arr = {}
      pos = pos + 1
      skip()
      if text:sub(pos, pos) == "]" then pos = pos + 1 return arr end
      while true do
        arr[#arr + 1] = value()
        skip()
        local d = text:sub(pos, pos)
        pos = pos + 1
        if d == "]" then return arr end
        assert(d == ",", "expected ',' or ']'")
      end
    elseif c == '"' then
      return string_value()
    elseif text:sub(pos, pos + 3) == "true" then
      pos = pos + 4
      return true
    elseif text:sub(pos, pos + 4) == "false" then
      pos = pos + 5
      return false
    else
      local number = text:match("^-?%d+", pos)
      assert(number, "unexpected JSON at offset " .. pos)
      pos = pos + #number
      return math.tointeger(tonumber(number))
    end
  end

  local result = value()
  skip()
  assert(pos == #text + 1, "trailing JSON content")
  return result
end

M.decode = decode

--- Decode lowercase hex into a Lua byte string.
function M.hex_to_bytes(hex)
  assert(#hex % 2 == 0, "odd-length hex")
  return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

--- Decode exactly 16 hex digits into a Lua integer with the same 64 bits.
-- tonumber(s, 16) accumulates modulo 2^64, so 'ffffffffffffffff' is -1.
function M.hex_to_u64(hex)
  assert(#hex == 16, "u64 must be 16 hex digits")
  return tonumber(hex, 16)
end

function M.bytes_to_hex(s)
  return (s:gsub(".", function(c) return string.format("%02x", c:byte()) end))
end

--- Load the shared corpus relative to the tests directory.
function M.load()
  local path = "../../../../specs/fixtures/ct-compare-v1/cases.json"
  local f = assert(io.open(path, "rb"), "cannot open " .. path)
  local text = f:read("a")
  f:close()
  return decode(text)
end

return M

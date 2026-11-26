--- A strict JSON reader for the conformance corpus (VER05o section 4).
--
-- Lua has no JSON in its standard library, and a lenient parser would let a
-- corrupted corpus through, so this one is strict:
--
--   * nesting deeper than 64 (the 65th container is refused BEFORE recursing),
--     documents over 8 MiB, more than 200,000 values (object keys count);
--   * invalid UTF-8 (overlong forms, surrogates, > U+10FFFF) and a BOM are refused;
--   * raw control characters in strings, lone or reversed surrogate escapes,
--     leading zeros, `+1`, `NaN`, comments, trailing commas and TRAILING DATA are
--     refused;
--   * duplicate object keys are refused, compared by their decoded bytes;
--   * NUMBERS STAY TEXT: a 30-digit integer is never converted to a Lua number.
--
-- Value model:
--   null    StrictJson.NULL            true / false   booleans
--   string  a Lua string               number  { kind = "number", text = "12" }
--   array   { kind = "array", [1..n] } object  { kind = "object", keys = {..in order..}, map = {k -> v} }
--
-- `parse(data)` returns `value` or `nil, message`; it never raises on bad input.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local s_byte, s_find, s_format, s_sub, t_concat, u_char, u_len = string.byte, string.find, string.format, string.sub, table.concat, utf8.char, utf8.len
local type, pcall, error, rawget, setmetatable, tonumber = type, pcall, error, rawget, setmetatable, tonumber

local StrictJson = {}

StrictJson.MAX_DEPTH = 64
StrictJson.MAX_BYTES = 8 * 1024 * 1024
StrictJson.MAX_NODES = 200000

local NULL = { kind = "null" }
StrictJson.NULL = NULL

local FAIL = {} -- identity of "our" parse failures

local ESCAPES = {
  [0x22] = '"',
  [0x5C] = "\\",
  [0x2F] = "/",
  [0x62] = "\b",
  [0x66] = "\f",
  [0x6E] = "\n",
  [0x72] = "\r",
  [0x74] = "\t",
}

local Parser = {}
Parser.__index = Parser

function Parser:fail(message)
  error({ [FAIL] = true, message = message .. " at byte " .. (self.pos - 1) }, 0)
end

function Parser:peek()
  return s_byte(self.s, self.pos)
end

function Parser:skip_ws()
  local _, e = s_find(self.s, "^[ \t\n\r]*", self.pos)
  self.pos = e + 1
end

function Parser:count_node()
  self.nodes = self.nodes + 1
  if self.nodes > StrictJson.MAX_NODES then
    self:fail("more than " .. StrictJson.MAX_NODES .. " values")
  end
end

function Parser:literal(word, value)
  if s_sub(self.s, self.pos, self.pos + #word - 1) ~= word then
    self:fail("invalid literal")
  end
  self.pos = self.pos + #word
  return value
end

function Parser:hex4()
  local text = s_sub(self.s, self.pos, self.pos + 3)
  if #text ~= 4 or not s_find(text, "^%x%x%x%x$") then
    self:fail("invalid \\u escape")
  end
  self.pos = self.pos + 4
  return tonumber(text, 16)
end

function Parser:escape()
  local code = self:peek()
  if code == nil then
    self:fail("unterminated escape")
  end
  self.pos = self.pos + 1
  if ESCAPES[code] then
    return ESCAPES[code]
  end
  if code ~= 0x75 then
    self:fail("invalid escape")
  end
  local unit = self:hex4()
  if unit >= 0xD800 and unit <= 0xDBFF then
    if not (self:peek() == 0x5C and s_byte(self.s, self.pos + 1) == 0x75) then
      self:fail("lone high surrogate")
    end
    self.pos = self.pos + 2
    local low = self:hex4()
    if low < 0xDC00 or low > 0xDFFF then
      self:fail("invalid surrogate pair")
    end
    return u_char(0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00))
  elseif unit >= 0xDC00 and unit <= 0xDFFF then
    self:fail("lone low surrogate")
  end
  return u_char(unit)
end

function Parser:parse_string()
  self.pos = self.pos + 1
  local parts = {}
  while true do
    local i = s_find(self.s, '[\0-\31"\\]', self.pos)
    if i == nil then
      self.pos = #self.s + 1
      self:fail("unterminated string")
    end
    parts[#parts + 1] = s_sub(self.s, self.pos, i - 1)
    local byte = s_byte(self.s, i)
    if byte == 0x22 then
      self.pos = i + 1
      return t_concat(parts)
    elseif byte < 0x20 then
      self.pos = i
      self:fail("raw control character in string")
    end
    self.pos = i + 1
    parts[#parts + 1] = self:escape()
    if #parts > 1024 then
      parts = { t_concat(parts) } -- flush: 8 MiB of escapes must not build a 4M-entry table
    end
  end
end

function Parser:parse_number()
  local s, start = self.s, self.pos
  local pos = start
  if s_byte(s, pos) == 0x2D then
    pos = pos + 1
  end
  local first = s_byte(s, pos)
  if first == 0x30 then
    pos = pos + 1
    local nxt = s_byte(s, pos)
    if nxt and nxt >= 0x30 and nxt <= 0x39 then
      self.pos = pos
      self:fail("leading zero")
    end
  elseif first and first >= 0x31 and first <= 0x39 then
    local _, e = s_find(s, "^%d+", pos)
    pos = e + 1
  else
    self.pos = pos
    self:fail("invalid number")
  end
  if s_byte(s, pos) == 0x2E then
    local _, e = s_find(s, "^%d+", pos + 1)
    if not e then
      self.pos = pos + 1
      self:fail("digits required after .")
    end
    pos = e + 1
  end
  local c = s_byte(s, pos)
  if c == 0x65 or c == 0x45 then
    local p = pos + 1
    local sign = s_byte(s, p)
    if sign == 0x2B or sign == 0x2D then
      p = p + 1
    end
    local _, e = s_find(s, "^%d+", p)
    if not e then
      self.pos = p
      self:fail("digits required in exponent")
    end
    pos = e + 1
  end
  self.pos = pos
  return { kind = "number", text = s_sub(s, start, pos - 1) }
end

function Parser:parse_array(depth)
  if depth + 1 > StrictJson.MAX_DEPTH then
    self:fail("nesting deeper than " .. StrictJson.MAX_DEPTH)
  end
  self.pos = self.pos + 1
  local out = { kind = "array" }
  self:skip_ws()
  if self:peek() == 0x5D then
    self.pos = self.pos + 1
    return out
  end
  while true do
    self:skip_ws()
    out[#out + 1] = self:parse_value(depth + 1)
    self:skip_ws()
    local c = self:peek()
    if c == 0x2C then
      self.pos = self.pos + 1
    elseif c == 0x5D then
      self.pos = self.pos + 1
      return out
    else
      self:fail("expected , or ]")
    end
  end
end

function Parser:parse_object(depth)
  if depth + 1 > StrictJson.MAX_DEPTH then
    self:fail("nesting deeper than " .. StrictJson.MAX_DEPTH)
  end
  self.pos = self.pos + 1
  local out = { kind = "object", keys = {}, map = {} }
  self:skip_ws()
  if self:peek() == 0x7D then
    self.pos = self.pos + 1
    return out
  end
  while true do
    self:skip_ws()
    if self:peek() ~= 0x22 then
      self:fail("expected a string key")
    end
    self:count_node()
    local key = self:parse_string()
    if out.map[key] ~= nil then
      self:fail("duplicate key")
    end
    self:skip_ws()
    if self:peek() ~= 0x3A then
      self:fail("expected :")
    end
    self.pos = self.pos + 1
    self:skip_ws()
    out.keys[#out.keys + 1] = key
    out.map[key] = self:parse_value(depth + 1)
    self:skip_ws()
    local c = self:peek()
    if c == 0x2C then
      self.pos = self.pos + 1
    elseif c == 0x7D then
      self.pos = self.pos + 1
      return out
    else
      self:fail("expected , or }")
    end
  end
end

function Parser:parse_value(depth)
  self:count_node()
  local c = self:peek()
  if c == 0x7B then
    return self:parse_object(depth)
  elseif c == 0x5B then
    return self:parse_array(depth)
  elseif c == 0x22 then
    return self:parse_string()
  elseif c == 0x74 then
    return self:literal("true", true)
  elseif c == 0x66 then
    return self:literal("false", false)
  elseif c == 0x6E then
    return self:literal("null", NULL)
  elseif c == 0x2D or (c and c >= 0x30 and c <= 0x39) then
    return self:parse_number()
  elseif c == nil then
    self:fail("unexpected end of input")
  end
  self:fail(s_format("unexpected byte 0x%02x", c))
end

--- Parses `data` (a string).  Returns the value, or `nil, message`.
function StrictJson.parse(data)
  if type(data) ~= "string" then
    return nil, "JSON input must be a string"
  end
  if #data > StrictJson.MAX_BYTES then
    return nil, "document of " .. #data .. " bytes exceeds " .. StrictJson.MAX_BYTES
  end
  if not u_len(data) then
    return nil, "document is not valid UTF-8"
  end
  if s_sub(data, 1, 3) == "\239\187\191" then
    return nil, "a byte-order mark is not allowed"
  end
  local p = setmetatable({ s = data, pos = 1, nodes = 0 }, Parser)
  local ok, result = pcall(function()
    p:skip_ws()
    local value = p:parse_value(0)
    p:skip_ws()
    if p.pos <= #data then
      p:fail("trailing data after the document")
    end
    return value
  end)
  if ok then
    return result
  end
  if type(result) == "table" and rawget(result, FAIL) then
    return nil, rawget(result, "message")
  end
  error(result, 0)
end

return StrictJson

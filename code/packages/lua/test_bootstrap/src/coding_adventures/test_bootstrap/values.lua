--- Strict decoding of corpus values and kind-sensitive equality (VER05o 4).
--
-- Encoding (JSON):
--   {"kind":"int","value":"-12"}     an Int (decimal text, any width up to 1,000 digits)
--   {"kind":"bytes","hex":"00ff"}    a byte string (a Lua string)
--   true / false                     a boolean
--   [ ... ]                          a list (a Lua array) of the above
--   "text"                           TEXT (a wrapped record: types, property names)
--
-- Lua strings are byte strings, so text must be told apart from bytes by wrapping
-- (see Canon.text).  Equality never conflates true with 1, bytes with text, an
-- integer with a float, or a list with anything else.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_type, s_char, s_find, s_gsub, s_sub = math.type, string.char, string.find, string.gsub, string.sub
local type, error, rawget, rawlen, setmetatable, tonumber = type, error, rawget, rawlen, setmetatable, tonumber

local Int = require("coding_adventures.test_bootstrap.int")
local Canon = require("coding_adventures.test_bootstrap.canon")

local Values = {}

Values.MAX_SIZE = 10000
Values.MAX_INT_DIGITS = 1000
Values.MAX_DEPTH = 64
Values.MAX_FRAGMENT_LEN = 200
Values.MAX_PROBLEM_LEN = 400

-- A malformed case (not an engine mismatch): a registered error value, so the
-- corpus runner can print its message without touching any other error.
local CASE_ERRORS = setmetatable({}, { __mode = "k" })

function Values.case_error(message)
  local err = { message = message }
  CASE_ERRORS[err] = true
  return err
end

--- Raises a case error.
function Values.fail(message)
  error(Values.case_error(message), 0)
end

--- A printable, bounded description of ANY error value (string messages are
-- clipped; our own case errors print their message; everything else its type).
function Values.error_message(err)
  if type(err) == "string" then
    return Canon.clip(Canon.escape(err), Values.MAX_FRAGMENT_LEN)
  elseif type(err) == "table" and CASE_ERRORS[err] then
    return Canon.clip(Canon.escape(rawget(err, "message")), Values.MAX_FRAGMENT_LEN)
  end
  return type(err)
end

--- Decimal digits only, optional leading "-", no leading zeros, no "-0".
function Values.canonical_int(text)
  return text == "0" or s_find(text, "^%-?[1-9]%d*$") ~= nil
end

function Values.lower_hex(text)
  return #text % 2 == 0 and s_find(text, "^[0-9a-f]*$") ~= nil
end

local function hex_to_bytes(text)
  return (s_gsub(text, "..", function(h)
    return s_char(tonumber(h, 16))
  end))
end

--- A short, bounded rendering for problem lines.
function Values.show(value)
  return Canon.clip(Canon.render(value), Values.MAX_FRAGMENT_LEN)
end

local function keys_are(node, a, b)
  local keys = node.keys
  return #keys == 2 and ((keys[1] == a and keys[2] == b) or (keys[1] == b and keys[2] == a))
end

local function decode_object(node)
  local kind = node.map.kind
  if kind == "int" and keys_are(node, "kind", "value") then
    local text = node.map.value
    if type(text) ~= "string" or not Values.canonical_int(text) then
      Values.fail("non-canonical int " .. Values.show(text))
    end
    local digits = #text - (s_sub(text, 1, 1) == "-" and 1 or 0)
    if digits > Values.MAX_INT_DIGITS then
      Values.fail("int exceeds " .. Values.MAX_INT_DIGITS .. " digits")
    end
    return Int.parse(text)
  elseif kind == "bytes" and keys_are(node, "kind", "hex") then
    local text = node.map.hex
    if type(text) ~= "string" or not Values.lower_hex(text) then
      Values.fail("bytes hex must be even-length lowercase: " .. Values.show(text))
    end
    if #text > 2 * Values.MAX_SIZE then
      Values.fail("bytes exceed " .. Values.MAX_SIZE .. " bytes")
    end
    return hex_to_bytes(text)
  end
  Values.fail("unrecognised value node")
end

--- Decodes one JSON node (from StrictJson) into an engine value.
function Values.decode(node, depth)
  depth = depth or 0
  if depth > Values.MAX_DEPTH then
    Values.fail("value nesting deeper than " .. Values.MAX_DEPTH)
  end
  local ty = type(node)
  if ty == "boolean" then
    return node
  elseif ty == "string" then
    return Canon.text(node)
  elseif ty == "table" then
    if node.kind == "array" then
      if #node > Values.MAX_SIZE then
        Values.fail("list exceeds " .. Values.MAX_SIZE .. " elements")
      end
      local out = {}
      for i = 1, #node do
        out[i] = Values.decode(node[i], depth + 1)
      end
      return out
    elseif node.kind == "object" then
      return decode_object(node)
    end
  end
  Values.fail("unsupported JSON value")
end

-- The "kind" of an engine value, for kind-sensitive equality.
local function kind_of(v)
  local ty = type(v)
  if ty == "boolean" then
    return v and "yes" or "no"
  elseif ty == "string" then
    return "bytes"
  elseif ty == "number" then
    return m_type(v) == "integer" and "int" or "float"
  elseif ty == "table" then
    if Int.is_big(v) then
      return "int"
    elseif Canon.is_text(v) then
      return "text"
    end
    return "list"
  end
  return "other"
end

local function same(a, b, depth, state)
  if depth > Values.MAX_DEPTH then
    return false
  end
  state.nodes = state.nodes + 1
  if state.nodes > Values.MAX_SIZE then
    return false
  end
  local ka = kind_of(a)
  if ka ~= kind_of(b) then
    return false
  end
  if ka == "int" then
    return Int.eq(a, b)
  elseif ka == "float" then
    if a ~= a and b ~= b then
      return true
    end
    return a == b and (a ~= 0 or 1 / a == 1 / b)
  elseif ka == "text" then
    return Canon.text_value(a) == Canon.text_value(b)
  elseif ka == "list" then
    local n = rawlen(a)
    if n ~= rawlen(b) then
      return false
    end
    for i = 1, n do
      if not same(rawget(a, i), rawget(b, i), depth + 1, state) then
        return false
      end
    end
    return true
  elseif ka == "bytes" then
    return a == b
  end
  return ka == "yes" or ka == "no"
end

--- Equality that does not conflate true with 1, bytes with text, integers with
-- floats, or lists with anything else.  Reads tables with rawlen/rawget only.
function Values.same(a, b)
  return same(a, b, 0, { nodes = 0 })
end

return Values

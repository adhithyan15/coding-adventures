--- Strict validator for TFV v1 text, the line format that carries test corpora
-- between languages (TF01 section 2).  This is a VALIDATOR, not a reader: its one
-- job here is to let the corpus runner check the pbt-v1 `corpus_reject` cases with
-- a real parser instead of skipping them.
--
--   tfv 1
--   # comment
--   <id> R<n> K<field> <value> K<field> <value> ...
--
-- A value is one of
--
--   N  T  F                 null, true, false
--   I<int>                  canonical decimal integer, at most 1000 digits
--   D<16 hex>               IEEE-754 bit pattern, lowercase
--   X<hex>  S<hex>          bytes / UTF-8 text as even-length lowercase hex
--   L<n> v1 ... vn          list
--   M<n> k1 v1 ... kn vn    map; no two keys deep-equal
--
-- Tokens are separated by exactly one ASCII space.  Rejected: uppercase or
-- odd-length hex, `I-0`, `I007`, a count that disagrees with the tokens, trailing
-- tokens, doubled spaces, a duplicate field or record id, invalid UTF-8 inside S, a
-- carriage return in a record line, nesting deeper than 64 containers and integers
-- of more than 1000 digits.
--
-- Worked example: "tfv 1\nr R1 Kop S706274 Kdata XFF\n" fails (XFF is uppercase and
-- odd length); "bad header\n" fails (the first line must be exactly "tfv 1").

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local s_byte, s_char, s_find, s_gsub, s_rep, s_sub, t_concat, t_sort, u_len = string.byte, string.char, string.find, string.gsub, string.rep, string.sub, table.concat, table.sort, utf8.len
local type, pcall, error, rawget, setmetatable, ipairs, tonumber = type, pcall, error, rawget, setmetatable, ipairs, tonumber

local Canon = require("coding_adventures.test_bootstrap.canon")

local Tfv = {}

Tfv.HEADER = "tfv 1"
Tfv.MAX_DEPTH = 64
Tfv.MAX_INT_DIGITS = 1000
Tfv.MAX_INPUT = 1 << 20

local FAIL = {}

local function q(text)
  local shown = #text > 64 and (s_sub(text, 1, 64) .. "...") or text
  return '"' .. Canon.escape(shown, true) .. '"'
end

local function fail(message)
  error({ [FAIL] = true, message = message }, 0)
end

local function split(s, sep)
  local out, pos = {}, 1
  while true do
    local i = s_find(s, sep, pos, true)
    if not i then
      out[#out + 1] = s_sub(s, pos)
      return out
    end
    out[#out + 1] = s_sub(s, pos, i - 1)
    pos = i + #sep
  end
end

local function lower_hex_bytes(text)
  if #text % 2 ~= 0 or not s_find(text, "^[0-9a-f]*$") then
    return nil
  end
  return (s_gsub(text, "..", function(h)
    return s_char(tonumber(h, 16))
  end))
end

local function canonical_nat(text)
  if text == "" then
    fail("empty count")
  end
  if not s_find(text, "^%d+$") then
    fail("non-digit in count " .. q(text))
  end
  if #text > 1 and s_sub(text, 1, 1) == "0" then
    fail("leading zero in count " .. q(text))
  end
  if #text > 9 then
    fail("count " .. q(text) .. " too large")
  end
  return tonumber(text)
end

local function valid_int(body)
  local neg = s_sub(body, 1, 1) == "-"
  local digits = neg and s_sub(body, 2) or body
  if digits == "" then
    fail("empty integer")
  end
  if not s_find(digits, "^%d+$") then
    fail("non-digit in integer " .. q(body))
  end
  if s_sub(digits, 1, 1) == "0" and (#digits > 1 or neg) then
    fail("non-canonical integer " .. q(body))
  end
  if #digits > Tfv.MAX_INT_DIGITS then
    fail("integer of " .. #digits .. " digits exceeds " .. Tfv.MAX_INT_DIGITS)
  end
end

-- Walks the tokens of one record.  `value` returns a deep-equality key: equal for
-- two values exactly when TF01 3.1 calls them deep-equal.
local Reader = {}
Reader.__index = Reader

function Reader:done()
  return self.pos > #self.tokens
end

function Reader:next_token()
  if self.pos > #self.tokens then
    fail("record ends early")
  end
  local tok = self.tokens[self.pos]
  self.pos = self.pos + 1
  return tok
end

function Reader:container(tag, body, depth)
  if depth + 1 > Tfv.MAX_DEPTH then
    fail("nesting deeper than " .. Tfv.MAX_DEPTH)
  end
  local count = canonical_nat(body)
  if tag == "L" then
    local items = {}
    for _ = 1, count do -- never reserve memory from a count
      items[#items + 1] = self:value(depth + 1)
    end
    return "L[" .. t_concat(items, ",") .. ",]"
  end
  local keys, entries = {}, {}
  for _ = 1, count do
    local k = self:value(depth + 1)
    local v = self:value(depth + 1)
    if keys[k] then
      fail("duplicate map key")
    end
    keys[k] = true
    entries[#entries + 1] = "(" .. k .. "=>" .. v .. ")"
  end
  t_sort(entries, Canon.byte_less)
  return "M{" .. t_concat(entries) .. "}"
end

function Reader:value(depth)
  local tok = self:next_token()
  local tag, body = s_sub(tok, 1, 1), s_sub(tok, 2)
  if tag == "N" or tag == "T" or tag == "F" then
    if body ~= "" then
      fail("unexpected body in " .. q(tok))
    end
    return tag
  elseif tag == "I" then
    valid_int(body)
    return tok
  elseif tag == "D" then
    local raw = (#body == 16) and lower_hex_bytes(body) or nil
    if not raw then
      fail("float needs exactly 16 lowercase hex digits: " .. q(tok))
    end
    local b1, b2 = s_byte(raw, 1), s_byte(raw, 2)
    local exp = ((b1 & 0x7F) << 4) | (b2 >> 4)
    local mantissa_zero = (b2 & 0x0F) == 0 and s_sub(raw, 3, 8) == s_rep("\0", 6)
    -- Every NaN equals every NaN; -0 and +0 stay distinct.
    if exp == 0x7FF and not mantissa_zero then
      return "Dnan"
    end
    return tok
  elseif tag == "X" then
    if not lower_hex_bytes(body) then
      fail("bytes need even-length lowercase hex: " .. q(tok))
    end
    return tok
  elseif tag == "S" then
    local raw = lower_hex_bytes(body)
    if not raw then
      fail("text needs even-length lowercase hex: " .. q(tok))
    end
    if not u_len(raw) then
      fail("text is not strict UTF-8: " .. q(tok))
    end
    return tok
  elseif tag == "L" or tag == "M" then
    return self:container(tag, body, depth)
  end
  fail("unknown tag in " .. q(tok))
end

local function validate_record(line)
  if s_find(line, "\r", 1, true) then
    fail("carriage return in a record line")
  end
  local tokens = split(line, " ")
  for _, t in ipairs(tokens) do
    if t == "" then
      fail("leading, trailing or doubled space")
    end
  end
  if #tokens < 2 then
    fail("a record needs an id and R<n>")
  end
  local id = tokens[1]
  if not s_find(id, "^[A-Za-z0-9._/%-]+$") then
    fail("bad record id " .. q(id))
  end
  if s_sub(tokens[2], 1, 1) ~= "R" then
    fail("expected R<n>, got " .. q(tokens[2]))
  end
  local count = canonical_nat(s_sub(tokens[2], 2))
  local reader = setmetatable({ tokens = tokens, pos = 3 }, Reader)
  local seen = {}
  for _ = 1, count do
    local key_tok = reader:next_token()
    if s_sub(key_tok, 1, 1) ~= "K" or not s_find(s_sub(key_tok, 2), "^[a-z][a-z0-9_]*$") then
      fail("bad field key " .. q(key_tok))
    end
    if seen[key_tok] then
      fail("duplicate field " .. q(s_sub(key_tok, 2)))
    end
    seen[key_tok] = true
    reader:value(0)
  end
  if not reader:done() then
    fail("trailing tokens after the last field")
  end
  return id
end

--- Returns true for a well-formed document, `false, message` otherwise.  Reserves
-- no memory from a count in the input.
function Tfv.validate(text)
  local ok, err = pcall(function()
    if type(text) ~= "string" then
      fail("input must be a string")
    end
    if #text > Tfv.MAX_INPUT then
      fail("input of " .. #text .. " bytes exceeds " .. Tfv.MAX_INPUT)
    end
    if not u_len(text) then
      fail("document is not valid UTF-8")
    end
    local lines = split(text, "\n")
    if lines[#lines] == "" then
      lines[#lines] = nil -- the final newline is optional
    end
    if #lines == 0 or lines[1] ~= Tfv.HEADER then
      fail("the first line must be exactly " .. q(Tfv.HEADER))
    end
    local seen_ids = {}
    for n = 2, #lines do
      local line = lines[n]
      if line ~= "" and s_sub(line, 1, 1) ~= "#" then
        local ok2, id = pcall(validate_record, line)
        if not ok2 then
          if type(id) == "table" and rawget(id, FAIL) then
            fail("line " .. n .. ": " .. rawget(id, "message"))
          end
          error(id, 0)
        end
        if seen_ids[id] then
          fail("line " .. n .. ": duplicate record id " .. q(id))
        end
        seen_ids[id] = true
      end
    end
  end)
  if ok then
    return true
  end
  if type(err) == "table" and rawget(err, FAIL) then
    return false, "invalid TFV: " .. rawget(err, "message")
  end
  error(err, 0)
end

--- True when `text` is a valid TFV v1 document.
function Tfv.valid(text)
  return (Tfv.validate(text))
end

return Tfv

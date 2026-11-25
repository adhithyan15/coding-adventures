--- A stable, TOTAL text form of a witness (used in messages and the CI digest),
-- VER05o section 4.
--
-- It never raises, and it never runs user code.  A Lua value can be a table whose
-- metatable defines `__index`, `__len`, `__pairs`, `__tostring`, `__eq`, `__lt`,
-- `__concat`, `__gc` or `__close` as traps; this module reads tables ONLY with
-- `next`, `rawget`, `rawlen` and `rawequal` (none of which consult a metatable)
-- and never calls `tostring` on a user value.
--
--   nil / true / false      nil true false
--   integer                 decimal (above 100,000 bits: "<bigint N bits>")
--   float                   shortest round-trip text that still looks like a float
--                           (3.0, never 3); nan, inf, -inf
--   string, valid UTF-8     "quoted", \" \\ \n \r \t short forms, \uXXXX otherwise
--   string, other           0x<lowercase hex>
--   table, keys exactly 1..n  [a,b,c]
--   any other table         %{k=>v,...}   sorted by the BYTES of "k=>v"
--   function/userdata/thread  #Function  #Userdata  #Thread
--
-- Caps, enforced while the value is traversed: nesting deeper than 16 renders
-- "...", more than 10,000 nodes or 1 MiB of output truncates (cut on a character
-- boundary, then "..."), and a failure while rendering turns the WHOLE witness
-- into "<unrenderable>".  A table that contains itself is just a deep table.
--
-- The escape set is copied exactly from the Elixir lane's Canon (VER05m): every C0
-- control, DEL, the C1 controls, U+2028/9, the bidirectional controls and the
-- invisible formatting characters are escaped, so a witness can never forge a log
-- line or reorder the text around it.

-- Standard functions are captured at load so that a property that patches the
-- string metatable or a library table cannot make the engine raise (VER05o 4).
local m_max, m_min, s_byte, s_find, s_gsub, s_sub, t_concat, t_sort, u_char, u_codes, u_len = math.max, math.min, string.byte, string.find, string.gsub, string.sub, table.concat, table.sort, utf8.char, utf8.codes, utf8.len
local type, pcall, error, next, rawget, rawequal, setmetatable, tonumber = type, pcall, error, next, rawget, rawequal, setmetatable, tonumber

local Int = require("coding_adventures.test_bootstrap.int")

local Canon = {}

local MAX_DEPTH = 16
local MAX_NODES = 10000
local MAX_OUTPUT = 1 << 20
local MAX_BIGINT_BITS = 100000
local UNRENDERABLE = "<unrenderable>"
local TRUNC = {} -- sentinel thrown when a cap is crossed

Canon.UNRENDERABLE = UNRENDERABLE
Canon.MAX_OUTPUT = MAX_OUTPUT

local mtype = math.type
local fmt = string.format

-- Text values: a registry of wrapper records, so corpus text can be told from
-- bytes (both are Lua strings).  Identity-keyed, no metatable on the wrapper.
local TEXTS = setmetatable({}, { __mode = "k" })

--- Wraps `s` as TEXT (as opposed to a byte string).
function Canon.text(s)
  local t = { s = s }
  TEXTS[t] = true
  return t
end

function Canon.is_text(v)
  return type(v) == "table" and TEXTS[v] == true
end

--- The string inside a text wrapper.
function Canon.text_value(t)
  return rawget(t, "s")
end

-- String comparison is ALWAYS bytewise.  Lua's `<` on strings uses strcoll, which depends
-- on the host locale; a digest must not, so sorting uses this comparator and the package
-- never queries or sets the locale (it needs no environment access).
local function byte_less(a, b)
  local n = m_min(#a, #b)
  for i = 1, n do
    local x, y = s_byte(a, i), s_byte(b, i)
    if x ~= y then
      return x < y
    end
  end
  return #a < #b
end
local less = byte_less
Canon.byte_less = byte_less

-- -------------------------------------------------------------------------
-- Escaping
-- -------------------------------------------------------------------------

local function u(c)
  if c > 0xFFFF then
    return fmt("\\u{%x}", c)
  end
  return fmt("\\u%04x", c)
end

local function escape_cp(c, quote)
  if c == 10 then
    return "\\n"
  elseif c == 13 then
    return "\\r"
  elseif c == 9 then
    return "\\t"
  elseif quote and c == 92 then
    return "\\\\"
  elseif quote and c == 34 then
    return '\\"'
  elseif c < 0x20 or c == 0x7F then
    return u(c)
  elseif c >= 0x80 and c <= 0x9F then
    return u(c)
  elseif c == 0x2028 or c == 0x2029 or c == 0x061C or c == 0x200E or c == 0x200F then
    return u(c)
  elseif (c >= 0x202A and c <= 0x202E) or (c >= 0x2066 and c <= 0x2069) then
    return u(c)
  elseif c == 0xAD or c == 0x34F or c == 0x180E or c == 0xFEFF then
    return u(c)
  elseif (c >= 0x200B and c <= 0x200D) or (c >= 0x2060 and c <= 0x2064) then
    return u(c)
  elseif c >= 0xFFF9 and c <= 0xFFFB then
    return u(c)
  elseif c >= 0xE0000 and c <= 0xE007F then
    return u(c)
  end
  return u_char(c)
end

-- Drops up to 3 trailing bytes of a character cut in half.
local function trim_partial(s)
  for _ = 0, 3 do
    if #s == 0 or u_len(s) then
      return s
    end
    s = s_sub(s, 1, #s - 1)
  end
  return s
end

local function hex(s)
  return (s_gsub(s, ".", function(c)
    return fmt("%02x", s_byte(c))
  end))
end

--- Escapes control and invisible characters (and, with `quote`, backslash and the
-- double quote) so a string can never forge a log line.  A string that is not
-- valid UTF-8 becomes `0x<hex>`.  `limit` stops the work once the output exceeds it.
function Canon.escape(text, quote, limit)
  if not u_len(text) then
    return "0x" .. hex(limit and s_sub(text, 1, limit) or text)
  end
  local out, size = {}, 0
  for _, c in u_codes(text) do
    local piece = escape_cp(c, quote)
    out[#out + 1] = piece
    size = size + #piece
    if limit and size > limit then
      break
    end
  end
  return t_concat(out)
end

--- A bounded fragment for reports: fixture-derived text must never make a problem
-- line (or a log) unboundedly large.  Cut on a character boundary.
function Canon.clip(text, limit)
  if #text > limit then
    return trim_partial(s_sub(text, 1, limit)) .. "..."
  end
  return text
end

-- -------------------------------------------------------------------------
-- Rendering
-- -------------------------------------------------------------------------

local function float_text(v)
  if v ~= v then
    return "nan"
  elseif v == math.huge then
    return "inf"
  elseif v == -math.huge then
    return "-inf"
  end
  local s
  for digits = 15, 17 do
    s = fmt("%." .. digits .. "g", v)
    if tonumber(s) == v then
      break
    end
  end
  if not s_find(s, "[%.eEn]") then
    s = s .. ".0"
  end
  return s
end

local function leaf(ctx, out, text)
  out[#out + 1] = text
  ctx.size = ctx.size + #text
  if ctx.size > MAX_OUTPUT then
    error(TRUNC, 0)
  end
end

local function count_node(ctx)
  ctx.nodes = ctx.nodes + 1
  if ctx.nodes > MAX_NODES then
    error(TRUNC, 0)
  end
end

local function string_text(s, ctx)
  -- The cap is applied BEFORE any allocation: clip to what the output may still
  -- hold (plus a little, so the overflow is noticed) before validity checking,
  -- escaping or hex encoding.
  local remaining = m_max(0, MAX_OUTPUT - ctx.size) + 4
  local cut = false
  if #s > remaining then
    s = trim_partial(s_sub(s, 1, remaining))
    cut = true
  end
  local text
  if u_len(s) then
    text = '"' .. Canon.escape(s, true, remaining) .. '"'
  else
    text = "0x" .. hex(s_sub(s, 1, remaining // 2 + 2))
  end
  if cut and #text <= remaining then
    text = text .. ".." -- guarantee the cap is crossed so the output is truncated
  end
  return text
end

local emit

local function collect(t)
  -- At most MAX_NODES + 1 entries; the node counter truncates the rest.
  local keys, vals, n = {}, {}, 0
  for k, v in next, t do
    n = n + 1
    keys[n], vals[n] = k, v
    if n > MAX_NODES then
      break
    end
  end
  return keys, vals, n
end

local function is_sequence(t, n)
  if n == 0 then
    return true
  end
  -- `n` is at most MAX_NODES + 1, so this check is bounded too
  for i = 1, n do
    if rawget(t, i) == nil then
      return false
    end
  end
  return true
end

local function render_sub(value, depth, ctx)
  local sub = {}
  emit(value, depth, ctx, sub)
  return t_concat(sub)
end

local function emit_table(t, depth, ctx, out)
  if depth >= MAX_DEPTH then
    return leaf(ctx, out, "...")
  end
  local keys, vals, n = collect(t)
  if is_sequence(t, n) then
    leaf(ctx, out, "[")
    for i = 1, n do
      if i > 1 then
        leaf(ctx, out, ",")
      end
      emit(rawget(t, i), depth + 1, ctx, out)
    end
    return leaf(ctx, out, "]")
  end
  local entries = {}
  for i = 1, n do
    count_node(ctx)
    local k = render_sub(keys[i], depth + 1, ctx)
    local v = render_sub(vals[i], depth + 1, ctx)
    entries[i] = k .. "=>" .. v
  end
  t_sort(entries, less)
  leaf(ctx, out, "%{")
  leaf(ctx, out, t_concat(entries, ","))
  return leaf(ctx, out, "}")
end

function emit(value, depth, ctx, out)
  count_node(ctx)
  local ty = type(value)
  if value == nil then
    return leaf(ctx, out, "nil")
  elseif ty == "boolean" then
    return leaf(ctx, out, value and "true" or "false")
  elseif ty == "number" then
    if mtype(value) == "integer" then
      return leaf(ctx, out, fmt("%d", value))
    end
    return leaf(ctx, out, float_text(value))
  elseif ty == "string" then
    return leaf(ctx, out, string_text(value, ctx))
  elseif ty == "function" then
    return leaf(ctx, out, "#Function")
  elseif ty == "userdata" then
    return leaf(ctx, out, "#Userdata")
  elseif ty == "thread" then
    return leaf(ctx, out, "#Thread")
  end
  -- a table
  if Int.is_big(value) then
    local bits = Int.bit_length(value)
    if bits > MAX_BIGINT_BITS then
      return leaf(ctx, out, fmt("<bigint %d bits>", bits))
    end
    -- decimal text costs about bits * 0.31 characters: refuse before converting
    if ctx.size + bits // 3 > MAX_OUTPUT then
      error(TRUNC, 0)
    end
    return leaf(ctx, out, Int.tostring(value))
  elseif Canon.is_text(value) then
    return leaf(ctx, out, string_text(Canon.text_value(value), ctx))
  end
  return emit_table(value, depth, ctx, out)
end

local function truncate(parts)
  local s = t_concat(parts)
  if #s > MAX_OUTPUT then
    s = s_sub(s, 1, MAX_OUTPUT)
  end
  return trim_partial(s) .. "..."
end

--- Renders any value; never raises, never runs user code.
function Canon.render(value)
  local ctx = { nodes = 0, size = 0 }
  local out = {}
  local ok, err = pcall(emit, value, 0, ctx, out)
  if ok then
    return t_concat(out)
  end
  if rawequal(err, TRUNC) then
    return truncate(out)
  end
  return UNRENDERABLE
end

return Canon

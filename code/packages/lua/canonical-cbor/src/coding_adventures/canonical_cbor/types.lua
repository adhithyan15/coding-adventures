-- ============================================================================
-- coding_adventures.canonical_cbor.types
-- ============================================================================
-- Distinct algebraic model, immutable container classes, span types, and
-- unforgeable type branding for deterministic canonical CBOR (CBR01).
--
-- Security guarantees:
-- 1. Unforgeable private registries: instance identity and branding are tracked
--    exclusively in module-private weak-keyed tables (VALUE_REGISTRY,
--    MAP_ENTRY_REGISTRY, SPAN_REGISTRY, SPANNED_ENTRY_REGISTRY, RESULT_REGISTRY,
--    ERROR_REGISTRY). Forged tables and metatable spoofing cannot masquerade as
--    valid CBOR values.
-- 2. Instances are protected with read-only proxy metatables (__metatable = false,
--    __newindex raising error on existing and new fields).
-- 3. Equality operators only inspect genuine registered instances via private
--    data lookups, preventing untrusted objects from triggering custom metamethods.
-- 4. In Lua 5.4, 64-bit unsigned integers are held in native integers; all
--    floats (including 1.0, math.huge, 0/0) are strictly rejected.
-- 5. Text strings are validated against strict 3-argument utf8.len(s, 1, -1).
-- 6. SpannedMapEntry redacts keys and values with messages.REDACTED_PLACEHOLDER.
-- 7. All error diagnostics use messages.lua (Standing Rule #20).
-- ============================================================================

local constants = require("coding_adventures.canonical_cbor.constants")
local messages = require("coding_adventures.canonical_cbor.messages")

local M = {}

local math_type = math.type

-- ----------------------------------------------------------------------------
-- Module-Private Unforgeable Type Registries
-- ----------------------------------------------------------------------------

local VALUE_REGISTRY = setmetatable({}, { __mode = "k" })
local MAP_ENTRY_REGISTRY = setmetatable({}, { __mode = "k" })
local SPAN_REGISTRY = setmetatable({}, { __mode = "k" })
local SPANNED_ENTRY_REGISTRY = setmetatable({}, { __mode = "k" })
local RESULT_REGISTRY = setmetatable({}, { __mode = "k" })
local ERROR_REGISTRY = setmetatable({}, { __mode = "k" })

local function make_immutable_proxy(data, extra_meta)
  local proxy = {}
  local meta = {
    __index = function(_, k)
      if extra_meta and extra_meta[k] ~= nil then
        return extra_meta[k]
      end
      return data[k]
    end,
    __newindex = function()
      error(messages.IMMUTABLE_OBJECT_MESSAGE, 0)
    end,
    __pairs = function()
      return pairs(data)
    end,
    __metatable = false,
  }
  if extra_meta then
    for k, v in pairs(extra_meta) do
      if k:sub(1, 2) == "__" then
        meta[k] = v
      end
    end
  end
  return setmetatable(proxy, meta)
end

--- Create a closed payload-blind CborError table.
--- @param id string
--- @return table
function M.CborError(id)
  local msg = messages.message_for(id)
  local data = {
    id = id,
    message = msg,
  }
  local proxy = make_immutable_proxy(data, {
    __tostring = function()
      return msg
    end,
    __eq = function(a, b)
      local data_b = ERROR_REGISTRY[b]
      if not data_b then
        return false
      end
      return ERROR_REGISTRY[a].id == data_b.id
    end,
  })
  ERROR_REGISTRY[proxy] = data
  return proxy
end

--- Raise a closed payload-blind CborError immediately.
--- @param id string
function M.raise_error(id)
  error(M.CborError(id), 0)
end

function M.is_cbor_error(v)
  return ERROR_REGISTRY[v] ~= nil
end

--- Validate and normalize byte-string or byte-array input into a Lua binary string.
--- Defends against metatable TOCTOU spoofing.
--- @param data string|table
--- @param error_id string
--- @return string
function M.normalize_bytes(data, error_id)
  local t = type(data)
  if t == "string" then
    return data
  end
  if t == "table" and getmetatable(data) == nil then
    local n = #data
    local chunks = {}
    for i = 1, n do
      local b = data[i]
      if math_type(b) ~= "integer" or (b & ~constants.BYTE_MASK) ~= 0 then
        M.raise_error(error_id)
      end
      chunks[i] = string.char(b)
    end
    for k in pairs(data) do
      if math_type(k) ~= "integer" or k < 1 or k > n then
        M.raise_error(error_id)
      end
    end
    return table.concat(chunks)
  end
  M.raise_error(error_id)
end

local function make_read_only_list(list)
  local proxy = {}
  local meta = {
    __index = list,
    __len = function()
      return #list
    end,
    __ipairs = function()
      return ipairs(list)
    end,
    __pairs = function()
      return pairs(list)
    end,
    __newindex = function()
      error(messages.IMMUTABLE_OBJECT_MESSAGE, 0)
    end,
    __metatable = false,
  }
  return setmetatable(proxy, meta)
end

-- ----------------------------------------------------------------------------
-- Span
-- ----------------------------------------------------------------------------

--- Construct a Span holding 0-based byte offsets [start_pos, finish_pos).
--- @param start_pos integer
--- @param finish_pos integer
--- @return table
function M.Span(start_pos, finish_pos)
  if math_type(start_pos) ~= "integer" or math_type(finish_pos) ~= "integer" or
     start_pos < 0 or finish_pos < start_pos then
    M.raise_error("unsupported-simple")
  end
  local data = {
    start = start_pos,
    finish = finish_pos,
  }
  local proxy = make_immutable_proxy(data, {
    length = function(self)
      return self.finish - self.start
    end,
    __tostring = function(self)
      return "[" .. self.start .. ", " .. self.finish .. ")"
    end,
    __eq = function(a, b)
      local data_b = SPAN_REGISTRY[b]
      if not data_b then
        return false
      end
      local data_a = SPAN_REGISTRY[a]
      return data_a.start == data_b.start and data_a.finish == data_b.finish
    end,
  })
  SPAN_REGISTRY[proxy] = data
  return proxy
end

function M.is_span(v)
  return SPAN_REGISTRY[v] ~= nil
end

-- ----------------------------------------------------------------------------
-- SpannedMapEntry
-- ----------------------------------------------------------------------------

--- Construct a SpannedMapEntry holding key, value, and the value's exact wire Span.
--- @param key table
--- @param value table
--- @param value_span table
--- @return table
function M.SpannedMapEntry(key, value, value_span)
  if not M.is_cbor_value(key) or not M.is_cbor_value(value) or not M.is_span(value_span) then
    M.raise_error("unsupported-simple")
  end
  local data = {
    key = key,
    value = value,
    value_span = value_span,
  }
  local proxy = make_immutable_proxy(data, {
    __tostring = function(self)
      return "SpannedMapEntry(key: " .. messages.REDACTED_PLACEHOLDER ..
             ", value: " .. messages.REDACTED_PLACEHOLDER ..
             ", value_span: " .. tostring(self.value_span) .. ")"
    end,
    __eq = function(a, b)
      local data_b = SPANNED_ENTRY_REGISTRY[b]
      if not data_b then
        return false
      end
      local data_a = SPANNED_ENTRY_REGISTRY[a]
      return data_a.key == data_b.key and data_a.value == data_b.value and data_a.value_span == data_b.value_span
    end,
  })
  SPANNED_ENTRY_REGISTRY[proxy] = data
  return proxy
end

function M.is_spanned_map_entry(v)
  return SPANNED_ENTRY_REGISTRY[v] ~= nil
end

-- ----------------------------------------------------------------------------
-- DecodeMapSpansResult
-- ----------------------------------------------------------------------------

local function new_decode_map_spans_result(entries)
  local data = {
    entries = entries,
  }
  local proxy = make_immutable_proxy(data, {
    is_map = function(self)
      return self.entries ~= nil
    end,
    __tostring = function(self)
      if self.entries == nil then
        return "DecodeMapSpansResult.non_map()"
      end
      return "DecodeMapSpansResult.map(#" .. #self.entries .. ")"
    end,
    __eq = function(a, b)
      local data_b = RESULT_REGISTRY[b]
      if not data_b then
        return false
      end
      local data_a = RESULT_REGISTRY[a]
      if (data_a.entries == nil) ~= (data_b.entries == nil) then
        return false
      end
      if data_a.entries == nil then
        return true
      end
      if #data_a.entries ~= #data_b.entries then
        return false
      end
      for i = 1, #data_a.entries do
        if data_a.entries[i] ~= data_b.entries[i] then
          return false
        end
      end
      return true
    end,
  })
  RESULT_REGISTRY[proxy] = data
  return proxy
end

M.DecodeMapSpansResult = {}

function M.DecodeMapSpansResult.map(entries)
  if type(entries) ~= "table" or getmetatable(entries) ~= nil then
    M.raise_error("unsupported-simple")
  end
  local n = #entries
  local copy = {}
  for i = 1, n do
    local entry = entries[i]
    if not M.is_spanned_map_entry(entry) then
      M.raise_error("unsupported-simple")
    end
    copy[i] = entry
  end
  for k in pairs(entries) do
    if math_type(k) ~= "integer" or k < 1 or k > n then
      M.raise_error("unsupported-simple")
    end
  end
  return new_decode_map_spans_result(make_read_only_list(copy))
end

function M.DecodeMapSpansResult.non_map()
  return new_decode_map_spans_result(nil)
end

function M.is_decode_map_spans_result(v)
  return RESULT_REGISTRY[v] ~= nil
end

M.DecodeMapSpansResult = make_immutable_proxy(M.DecodeMapSpansResult, nil)

-- ----------------------------------------------------------------------------
-- CborValue Hierarchy (Strictly the 9 Genuine CBR01 Kinds)
-- ----------------------------------------------------------------------------

function M.is_cbor_value(v)
  return VALUE_REGISTRY[v] ~= nil
end

local function deep_equals_cbor_value(a, b)
  local data_b = VALUE_REGISTRY[b]
  if not data_b then
    return false
  end
  local data_a = VALUE_REGISTRY[a]
  if data_a.kind ~= data_b.kind then
    return false
  end
  local k = data_a.kind
  if k == "unsigned" or k == "negative" or k == "boolean" then
    return data_a.value == data_b.value
  elseif k == "bytes" then
    return data_a.bytes == data_b.bytes
  elseif k == "text" then
    return data_a.text == data_b.text
  elseif k == "null" then
    return true
  elseif k == "tag" then
    return data_a.tag == data_b.tag and data_a.value == data_b.value
  elseif k == "array" then
    if #data_a.items ~= #data_b.items then
      return false
    end
    for i = 1, #data_a.items do
      if data_a.items[i] ~= data_b.items[i] then
        return false
      end
    end
    return true
  elseif k == "map" then
    if #data_a.entries ~= #data_b.entries then
      return false
    end
    for i = 1, #data_a.entries do
      if data_a.entries[i] ~= data_b.entries[i] then
        return false
      end
    end
    return true
  end
  return false
end

local function make_cbor_value(kind_name, data, tostring_fn)
  data.kind = kind_name
  local proxy = make_immutable_proxy(data, {
    __tostring = tostring_fn,
    __eq = deep_equals_cbor_value,
  })
  VALUE_REGISTRY[proxy] = data
  return proxy
end

function M.CborUnsigned(value)
  if math_type(value) ~= "integer" then
    M.raise_error("unsupported-simple")
  end
  return make_cbor_value("unsigned", { value = value }, function(self)
    return "CborUnsigned(" .. tostring(self.value) .. ")"
  end)
end

function M.CborNegative(value)
  if math_type(value) ~= "integer" then
    M.raise_error("unsupported-simple")
  end
  return make_cbor_value("negative", { value = value }, function(self)
    return "CborNegative(" .. tostring(self.value) .. ")"
  end)
end

function M.CborBytes(bytes_or_table)
  local bs = M.normalize_bytes(bytes_or_table, "unsupported-simple")
  return make_cbor_value("bytes", { bytes = bs }, function(self)
    return "CborBytes(#" .. #self.bytes .. ")"
  end)
end

function M.CborText(text)
  if type(text) ~= "string" then
    M.raise_error("unsupported-simple")
  end
  -- Strict 3-argument utf8.len(s, 1, -1) rejects invalid UTF-8 and surrogates.
  if utf8.len(text, 1, -1) == nil then
    M.raise_error("invalid-utf8")
  end
  return make_cbor_value("text", { text = text }, function(self)
    return "CborText(" .. string.format("%q", self.text) .. ")"
  end)
end

function M.CborArray(items)
  if type(items) ~= "table" or getmetatable(items) ~= nil then
    M.raise_error("unsupported-simple")
  end
  local n = #items
  local copy = {}
  for i = 1, n do
    local item = items[i]
    if not M.is_cbor_value(item) then
      M.raise_error("unsupported-simple")
    end
    copy[i] = item
  end
  for k in pairs(items) do
    if math_type(k) ~= "integer" or k < 1 or k > n then
      M.raise_error("unsupported-simple")
    end
  end
  return make_cbor_value("array", { items = make_read_only_list(copy) }, function(self)
    return "CborArray(#" .. #self.items .. ")"
  end)
end

-- ----------------------------------------------------------------------------
-- CborMapEntry (Pair Model — NOT a CborValue)
-- ----------------------------------------------------------------------------

local function deep_equals_map_entry(a, b)
  local data_b = MAP_ENTRY_REGISTRY[b]
  if not data_b then
    return false
  end
  local data_a = MAP_ENTRY_REGISTRY[a]
  return data_a.key == data_b.key and data_a.value == data_b.value
end

function M.CborMapEntry(key, value)
  if not M.is_cbor_value(key) or not M.is_cbor_value(value) then
    M.raise_error("unsupported-simple")
  end
  local data = {
    kind = "map_entry",
    key = key,
    value = value,
  }
  local proxy = make_immutable_proxy(data, {
    __tostring = function(self)
      return "CborMapEntry(" .. tostring(self.key) .. " => " .. tostring(self.value) .. ")"
    end,
    __eq = deep_equals_map_entry,
  })
  MAP_ENTRY_REGISTRY[proxy] = data
  return proxy
end

function M.is_cbor_map_entry(v)
  return MAP_ENTRY_REGISTRY[v] ~= nil
end

function M.CborMap(entries)
  if type(entries) ~= "table" or getmetatable(entries) ~= nil then
    M.raise_error("unsupported-simple")
  end
  local n = #entries
  local copy = {}
  for i = 1, n do
    local entry = entries[i]
    if not M.is_cbor_map_entry(entry) then
      M.raise_error("unsupported-simple")
    end
    copy[i] = entry
  end
  for k in pairs(entries) do
    if math_type(k) ~= "integer" or k < 1 or k > n then
      M.raise_error("unsupported-simple")
    end
  end
  return make_cbor_value("map", { entries = make_read_only_list(copy) }, function(self)
    return "CborMap(#" .. #self.entries .. ")"
  end)
end

function M.CborTag(tag, value)
  if math_type(tag) ~= "integer" or not M.is_cbor_value(value) then
    M.raise_error("unsupported-simple")
  end
  return make_cbor_value("tag", { tag = tag, value = value }, function(self)
    return "CborTag(" .. tostring(self.tag) .. ", " .. tostring(self.value) .. ")"
  end)
end

function M.CborBool(value)
  if type(value) ~= "boolean" then
    M.raise_error("unsupported-simple")
  end
  return make_cbor_value("boolean", { value = value }, function(self)
    return "CborBool(" .. tostring(self.value) .. ")"
  end)
end

function M.CborNull()
  return make_cbor_value("null", {}, function()
    return "CborNull"
  end)
end

return setmetatable(M, {
  __newindex = function()
    error(messages.READ_ONLY_MODULE_MESSAGE, 0)
  end,
  __metatable = false,
})

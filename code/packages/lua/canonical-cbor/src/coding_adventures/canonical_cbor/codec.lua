-- ============================================================================
-- coding_adventures.canonical_cbor.codec
-- ============================================================================
-- Deterministic canonical CBOR encoder and decoder (RFC 8949 CBR01 profile).
--
-- Knuth-style literate implementation guaranteeing:
-- 1. Length-first canonical map key ordering (§4.2.3).
-- 2. Minimal integer argument representation (§4.2.1).
-- 3. Strict 64-bit unsigned comparison (math.ult) to defeat Lua 5.4 signed
--    integer wraparound vulnerabilities (CVE-1).
-- 4. Pre-allocation bounds validation (check_length) to defeat OOM bombs (CVE-2).
-- 5. Strict nesting depth caps (MAX_NESTING_DEPTH = 128) against stack overflow (CVE-3).
-- 6. Atomic encode_into_checked rollback on failure.
-- 7. Zero inline diagnostic strings (all via messages.lua).
-- ============================================================================

local constants = require("coding_adventures.canonical_cbor.constants")
local messages = require("coding_adventures.canonical_cbor.messages")
local types = require("coding_adventures.canonical_cbor.types")

local M = {}

-- ----------------------------------------------------------------------------
-- Internal Buffer for Encoding
-- ----------------------------------------------------------------------------

local function new_buffer()
  return {
    length = 0,
    chunks = {},
  }
end

local function push_byte(buffer, b)
  if buffer.length >= constants.MAX_ENCODED_BYTES then
    types.raise_error("encode-too-large")
  end
  buffer.length = buffer.length + 1
  buffer.chunks[#buffer.chunks + 1] = string.char(b & constants.BYTE_MASK)
end

local function push_bytes(buffer, s)
  local slen = #s
  if slen > constants.MAX_ENCODED_BYTES - buffer.length then
    types.raise_error("encode-too-large")
  end
  buffer.length = buffer.length + slen
  buffer.chunks[#buffer.chunks + 1] = s
end

local function write_big_endian_uint(buffer, value, byte_count)
  for i = byte_count - 1, 0, -1 do
    local shift = i * constants.BITS_PER_BYTE
    push_byte(buffer, (value >> shift) & constants.BYTE_MASK)
  end
end

-- Write a CBOR header (major type + argument) with minimal representation.
-- CRITICAL: Uses math.ult to prevent 64-bit signed comparison bypass.
local function write_type_and_argument(buffer, major, arg)
  local major_bits = major << constants.MAJOR_TYPE_SHIFT
  if not math.ult(constants.INFO_INLINE_MAX, arg) then
    -- arg <=_u 23
    push_byte(buffer, major_bits | arg)
  elseif not math.ult(constants.MAX_UINT8, arg) then
    -- arg <=_u 0xFF
    push_byte(buffer, major_bits | constants.INFO_1_BYTE)
    push_byte(buffer, arg)
  elseif not math.ult(constants.MAX_UINT16, arg) then
    -- arg <=_u 0xFFFF
    push_byte(buffer, major_bits | constants.INFO_2_BYTES)
    write_big_endian_uint(buffer, arg, constants.ARG_WIDTH_2_BYTES)
  elseif not math.ult(constants.MAX_UINT32, arg) then
    -- arg <=_u 0xFFFFFFFF
    push_byte(buffer, major_bits | constants.INFO_4_BYTES)
    write_big_endian_uint(buffer, arg, constants.ARG_WIDTH_4_BYTES)
  else
    -- 64-bit integer
    push_byte(buffer, major_bits | constants.INFO_8_BYTES)
    write_big_endian_uint(buffer, arg, constants.ARG_WIDTH_8_BYTES)
  end
end

-- ----------------------------------------------------------------------------
-- Recursive Encoding
-- ----------------------------------------------------------------------------

local encode_value

local function encode_map(entries, buffer, depth)
  local encoded_entries = {}
  for i = 1, #entries do
    local entry = entries[i]
    local key_buf = new_buffer()
    encode_value(entry.key, key_buf, depth + 1)
    encoded_entries[i] = {
      key_bytes = table.concat(key_buf.chunks),
      value = entry.value,
    }
  end

  -- RFC 8949 §4.2.3 Length-first map key ordering:
  table.sort(encoded_entries, function(a, b)
    local la, lb = #a.key_bytes, #b.key_bytes
    if la ~= lb then
      return la < lb
    end
    return a.key_bytes < b.key_bytes
  end)

  -- Detect duplicate canonical keys:
  for i = 2, #encoded_entries do
    if encoded_entries[i - 1].key_bytes == encoded_entries[i].key_bytes then
      types.raise_error("duplicate-map-key")
    end
  end

  write_type_and_argument(buffer, constants.MAJOR_MAP, #encoded_entries)
  for i = 1, #encoded_entries do
    local entry = encoded_entries[i]
    push_bytes(buffer, entry.key_bytes)
    encode_value(entry.value, buffer, depth + 1)
  end
end

function encode_value(v, buffer, depth)
  if depth > constants.MAX_NESTING_DEPTH then
    types.raise_error("encode-too-deep")
  end
  if not types.is_cbor_value(v) then
    types.raise_error("unsupported-simple")
  end

  local k = v.kind
  if k == "unsigned" then
    write_type_and_argument(buffer, constants.MAJOR_UNSIGNED, v.value)
  elseif k == "negative" then
    write_type_and_argument(buffer, constants.MAJOR_NEGATIVE, v.value)
  elseif k == "bytes" then
    local b = v.bytes
    write_type_and_argument(buffer, constants.MAJOR_BYTES, #b)
    push_bytes(buffer, b)
  elseif k == "text" then
    local t = v.text
    write_type_and_argument(buffer, constants.MAJOR_TEXT, #t)
    push_bytes(buffer, t)
  elseif k == "array" then
    local items = v.items
    write_type_and_argument(buffer, constants.MAJOR_ARRAY, #items)
    for i = 1, #items do
      encode_value(items[i], buffer, depth + 1)
    end
  elseif k == "map" then
    encode_map(v.entries, buffer, depth)
  elseif k == "tag" then
    write_type_and_argument(buffer, constants.MAJOR_TAG, v.tag)
    encode_value(v.value, buffer, depth + 1)
  elseif k == "boolean" then
    push_byte(buffer, v.value and constants.WIRE_BYTE_TRUE or constants.WIRE_BYTE_FALSE)
  else
    -- k == "null"
    push_byte(buffer, constants.WIRE_BYTE_NULL)
  end
end

--- Deterministically encode a CborValue into a canonical CBOR byte string.
--- @param value table
--- @return string
function M.encode_checked(value)
  local buf = new_buffer()
  encode_value(value, buf, constants.ROOT_DEPTH)
  return table.concat(buf.chunks)
end

--- Deterministically encode a CborValue and append the resulting chunk to dst_table.
--- Atomic: dst_table is validated and remains completely unmodified if encoding fails.
--- @param value table
--- @param dst_table table
function M.encode_into_checked(value, dst_table)
  if type(dst_table) ~= "table" or getmetatable(dst_table) ~= nil then
    types.raise_error("unsupported-simple")
  end
  local n = #dst_table
  for k in pairs(dst_table) do
    if math.type(k) ~= "integer" or k < 1 or k > n then
      types.raise_error("unsupported-simple")
    end
  end
  local encoded = M.encode_checked(value)
  dst_table[n + 1] = encoded
end

-- ----------------------------------------------------------------------------
-- Decoding Helpers
-- ----------------------------------------------------------------------------

local function read_u8(cursor)
  if cursor.pos > #cursor.data then
    types.raise_error("unexpected-eof")
  end
  local b = string.byte(cursor.data, cursor.pos)
  cursor.pos = cursor.pos + 1
  return b
end

local function read_n(cursor, n)
  local finish = cursor.pos + n - 1
  if finish > #cursor.data then
    types.raise_error("unexpected-eof")
  end
  local s = string.sub(cursor.data, cursor.pos, finish)
  cursor.pos = finish + 1
  return s
end

local function read_big_endian_uint(cursor, byte_count)
  local val = 0
  for _ = 1, byte_count do
    val = (val << constants.BITS_PER_BYTE) | read_u8(cursor)
  end
  return val
end

-- Verify that declared length does not exceed remaining buffer capacity.
-- CRITICAL: Uses math.ult so that 64-bit signed negative lengths (e.g. 0xFFFFFFFFFFFFFFFF == -1)
-- cannot bypass the check.
local function check_length(declared, remaining, min_bytes_per_unit)
  local max_allowed = remaining // min_bytes_per_unit
  if math.ult(max_allowed, declared) then
    types.raise_error("length-too-large")
  end
  return declared
end

-- Verify RFC 8949 Section 4.2.1 minimal integer encoding.
local function validate_minimal_argument(major, info, arg)
  if major == constants.MAJOR_SIMPLE_OR_FLOAT then
    return
  end
  if info == constants.INFO_1_BYTE then
    if not math.ult(constants.INFO_INLINE_MAX, arg) then
      types.raise_error("non-minimal-integer")
    end
  elseif info == constants.INFO_2_BYTES then
    if not math.ult(constants.MAX_UINT8, arg) then
      types.raise_error("non-minimal-integer")
    end
  elseif info == constants.INFO_4_BYTES then
    if not math.ult(constants.MAX_UINT16, arg) then
      types.raise_error("non-minimal-integer")
    end
  elseif info == constants.INFO_8_BYTES then
    if not math.ult(constants.MAX_UINT32, arg) then
      types.raise_error("non-minimal-integer")
    end
  end
end

local function read_header(cursor)
  local header_byte = read_u8(cursor)
  local major = header_byte >> constants.MAJOR_TYPE_SHIFT
  local info = header_byte & constants.ADDITIONAL_INFO_MASK
  local arg

  if info <= constants.INFO_INLINE_MAX then
    arg = info
  elseif info == constants.INFO_1_BYTE then
    arg = read_u8(cursor)
  elseif info == constants.INFO_2_BYTES then
    arg = read_big_endian_uint(cursor, constants.ARG_WIDTH_2_BYTES)
  elseif info == constants.INFO_4_BYTES then
    arg = read_big_endian_uint(cursor, constants.ARG_WIDTH_4_BYTES)
  elseif info == constants.INFO_8_BYTES then
    arg = read_big_endian_uint(cursor, constants.ARG_WIDTH_8_BYTES)
  elseif info >= constants.INFO_RESERVED_28 and info <= constants.INFO_RESERVED_30 then
    types.raise_error("reserved")
  else
    -- info == constants.INFO_INDEFINITE (31)
    types.raise_error("indefinite")
  end

  validate_minimal_argument(major, info, arg)
  return major, info, arg
end

local function key_strictly_less(prev_key_bytes, cur_key_bytes)
  local lp = #prev_key_bytes
  local lc = #cur_key_bytes
  if lp ~= lc then
    return lp < lc
  end
  return prev_key_bytes < cur_key_bytes
end

-- ----------------------------------------------------------------------------
-- Recursive Decoding
-- ----------------------------------------------------------------------------

local read_value

-- Shared private helper: reads canonical map entries and creates SpannedMapEntry items.
local function read_spanned_map_entries(cursor, depth, entry_count)
  local entries = {}
  local prev_key_bytes = nil

  for i = 1, entry_count do
    local key_start_1 = cursor.pos
    local key = read_value(cursor, depth + 1)
    local key_finish_1 = cursor.pos - 1
    local cur_key_bytes = string.sub(cursor.data, key_start_1, key_finish_1)

    if prev_key_bytes and not key_strictly_less(prev_key_bytes, cur_key_bytes) then
      types.raise_error("non-canonical-map-order")
    end
    prev_key_bytes = cur_key_bytes

    local val_start_1 = cursor.pos
    local val = read_value(cursor, depth + 1)
    local val_finish_1 = cursor.pos - 1

    -- 0-based offset span [start, finish):
    local val_span = types.Span(val_start_1 - 1, val_finish_1)
    entries[i] = types.SpannedMapEntry(key, val, val_span)
  end

  return entries
end

function read_value(cursor, depth)
  if depth > constants.MAX_NESTING_DEPTH then
    types.raise_error("too-deep")
  end

  local major, info, arg = read_header(cursor)

  if major == constants.MAJOR_UNSIGNED then
    return types.CborUnsigned(arg)
  elseif major == constants.MAJOR_NEGATIVE then
    return types.CborNegative(arg)
  elseif major == constants.MAJOR_BYTES then
    local remaining = #cursor.data - cursor.pos + 1
    local len = check_length(arg, remaining, constants.MIN_BYTES_PER_ELEMENT)
    local bs = read_n(cursor, len)
    return types.CborBytes(bs)
  elseif major == constants.MAJOR_TEXT then
    local remaining = #cursor.data - cursor.pos + 1
    local len = check_length(arg, remaining, constants.MIN_BYTES_PER_ELEMENT)
    local bs = read_n(cursor, len)
    return types.CborText(bs)
  elseif major == constants.MAJOR_ARRAY then
    local remaining = #cursor.data - cursor.pos + 1
    local count = check_length(arg, remaining, constants.MIN_BYTES_PER_ELEMENT)
    local items = {}
    for i = 1, count do
      items[i] = read_value(cursor, depth + 1)
    end
    return types.CborArray(items)
  elseif major == constants.MAJOR_MAP then
    local remaining = #cursor.data - cursor.pos + 1
    local count = check_length(arg, remaining, constants.MIN_BYTES_PER_MAP_ENTRY)
    local spanned = read_spanned_map_entries(cursor, depth, count)
    local map_entries = {}
    for i = 1, #spanned do
      map_entries[i] = types.CborMapEntry(spanned[i].key, spanned[i].value)
    end
    return types.CborMap(map_entries)
  elseif major == constants.MAJOR_TAG then
    local inner = read_value(cursor, depth + 1)
    return types.CborTag(arg, inner)
  else
    -- major == constants.MAJOR_SIMPLE_OR_FLOAT
    if info == constants.SIMPLE_FALSE then
      return types.CborBool(false)
    elseif info == constants.SIMPLE_TRUE then
      return types.CborBool(true)
    elseif info == constants.SIMPLE_NULL then
      return types.CborNull()
    elseif info == constants.SIMPLE_FLOAT_16 or
           info == constants.SIMPLE_FLOAT_32 or
           info == constants.SIMPLE_FLOAT_64 then
      types.raise_error("float-not-supported")
    else
      types.raise_error("unsupported-simple")
    end
  end
end

--- Decode a canonical CBOR byte string or array of byte integers.
--- Verifies that no trailing bytes remain after the root item.
--- @param data string|table
--- @return table
function M.decode(data)
  local bytes = types.normalize_bytes(data, "unexpected-eof")
  local cursor = {
    data = bytes,
    pos = 1,
  }
  local value = read_value(cursor, constants.ROOT_DEPTH)
  if cursor.pos <= #cursor.data then
    types.raise_error("trailing-bytes")
  end
  return value
end

--- Decode canonical CBOR and return SpannedMapEntry records with 0-based byte offsets
--- if the root item is a map, or DecodeMapSpansResult.non_map() otherwise.
--- @param data string|table
--- @return table
function M.decode_map_spans(data)
  local bytes = types.normalize_bytes(data, "unexpected-eof")
  if #bytes == 0 then
    types.raise_error("unexpected-eof")
  end

  local header_byte = string.byte(bytes, 1)
  local major = header_byte >> constants.MAJOR_TYPE_SHIFT
  if major ~= constants.MAJOR_MAP then
    -- Validate that non-map root is well-formed CBOR without trailing bytes:
    M.decode(bytes)
    return types.DecodeMapSpansResult.non_map()
  end

  local cursor = {
    data = bytes,
    pos = 1,
  }
  local _, _, arg = read_header(cursor)
  local remaining = #cursor.data - cursor.pos + 1
  local count = check_length(arg, remaining, constants.MIN_BYTES_PER_MAP_ENTRY)
  local entries = read_spanned_map_entries(cursor, constants.ROOT_DEPTH, count)

  if cursor.pos <= #cursor.data then
    types.raise_error("trailing-bytes")
  end

  return types.DecodeMapSpansResult.map(entries)
end

return setmetatable(M, {
  __newindex = function()
    error(messages.READ_ONLY_MODULE_MESSAGE, 0)
  end,
  __metatable = false,
})

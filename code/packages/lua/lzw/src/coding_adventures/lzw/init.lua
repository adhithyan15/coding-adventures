-- =============================================================================
-- CodingAdventures.LZW
-- =============================================================================
--
-- LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is LZW?
-- ------------
--
-- LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
-- added before encoding begins (codes 0-255). This eliminates LZ78's mandatory
-- next_char byte -- every symbol is already in the dictionary, so the encoder
-- can emit pure codes.
--
-- With only codes to transmit, LZW uses variable-width bit-packing: codes start
-- at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
--
-- Reserved Codes
-- --------------
--
--   0-255:  Pre-seeded single-byte entries.
--   256:    CLEAR_CODE -- reset to initial 256-entry state.
--   257:    STOP_CODE  -- end of code stream.
--   258+:   Dynamically added entries.
--
-- Wire Format (CMP03)
-- -------------------
--
--   Bytes 0-3:  original_length (big-endian uint32)
--   Bytes 4+:   bit-packed variable-width codes, LSB-first
--
-- The Tricky Token
-- ----------------
--
-- During decoding the decoder may receive code C == next_code (not yet added).
-- This happens when the input has the form xyx...x. The fix:
--
--   entry = dict[prev_code] + string.char(dict[prev_code]:sub(1,1):byte())
--
-- The Series: CMP00 -> CMP05
-- --------------------------
--
--   CMP00 (LZ77,    1977) -- Sliding-window backreferences.
--   CMP01 (LZ78,    1978) -- Explicit dictionary (trie).
--   CMP02 (LZSS,    1982) -- LZ77 + flag bits; no wasted literals.
--   CMP03 (LZW,     1984) -- LZ78 + pre-initialized dict; GIF. (this module)
--   CMP04 (Huffman, 1952) -- Entropy coding; prerequisite for DEFLATE.
--   CMP05 (DEFLATE, 1996) -- LZ77 + Huffman; ZIP/gzip/PNG/zlib.
-- =============================================================================

local M = {}

-- ---------------------------------------------------------------------------
-- Constants
-- ---------------------------------------------------------------------------

M.CLEAR_CODE        = 256
M.STOP_CODE         = 257
M.INITIAL_NEXT_CODE = 258
M.INITIAL_CODE_SIZE = 9
M.MAX_CODE_SIZE     = 16

-- ---------------------------------------------------------------------------
-- Bit I/O helpers
-- ---------------------------------------------------------------------------

--- Create a new BitWriter state.
-- State: { buf=0, bit_pos=0, bytes={} }
local function bw_new()
  return { buf = 0, bit_pos = 0, bytes = {} }
end

--- Write `code` using exactly `code_size` bits, LSB-first.
local function bw_write(w, code, code_size)
  -- Use floating-point multiplication to avoid integer overflow for large shifts.
  w.buf = w.buf + code * (2 ^ w.bit_pos)
  w.bit_pos = w.bit_pos + code_size
  while w.bit_pos >= 8 do
    w.bytes[#w.bytes + 1] = w.buf % 256
    w.buf = math.floor(w.buf / 256)
    w.bit_pos = w.bit_pos - 8
  end
end

--- Flush any remaining bits as a final partial byte.
local function bw_flush(w)
  if w.bit_pos > 0 then
    w.bytes[#w.bytes + 1] = w.buf % 256
    w.buf = 0
    w.bit_pos = 0
  end
end

--- Convert BitWriter output to a string.
local function bw_to_string(w)
  local chars = {}
  for _, b in ipairs(w.bytes) do
    chars[#chars + 1] = string.char(b)
  end
  return table.concat(chars)
end

--- Create a new BitReader state.
local function br_new(data)
  return { data = data, pos = 1, buf = 0, bit_pos = 0 }
end

--- Read the next `code_size`-bit code from the BitReader.
-- Returns the code, or nil if the stream is exhausted.
local function br_read(r, code_size)
  while r.bit_pos < code_size do
    if r.pos > #r.data then
      return nil
    end
    local byte = string.byte(r.data, r.pos)
    r.buf = r.buf + byte * (2 ^ r.bit_pos)
    r.pos = r.pos + 1
    r.bit_pos = r.bit_pos + 8
  end
  local mask = (2 ^ code_size) - 1
  local code = math.floor(r.buf) % (mask + 1)
  r.buf = math.floor(r.buf / (2 ^ code_size))
  r.bit_pos = r.bit_pos - code_size
  return code
end

local function br_exhausted(r)
  return r.pos > #r.data and r.bit_pos == 0
end

-- ---------------------------------------------------------------------------
-- Encoder
-- ---------------------------------------------------------------------------

--- Encode a byte string into a list of LZW codes.
-- Returns codes (table of integers) and original_length (integer).
function M.encode_codes(data)
  local original_length = #data
  local enc_dict = {}
  for b = 0, 255 do
    enc_dict[string.char(b)] = b
  end

  local next_code = M.INITIAL_NEXT_CODE
  local max_entries = 2 ^ M.MAX_CODE_SIZE
  local codes = { M.CLEAR_CODE }
  local w = ""

  for i = 1, #data do
    local byte = string.sub(data, i, i)
    local wb = w .. byte
    if enc_dict[wb] ~= nil then
      w = wb
    else
      codes[#codes + 1] = enc_dict[w]

      if next_code < max_entries then
        enc_dict[wb] = next_code
        next_code = next_code + 1
      elseif next_code == max_entries then
        -- Dictionary full -- emit CLEAR and reset.
        codes[#codes + 1] = M.CLEAR_CODE
        enc_dict = {}
        for b = 0, 255 do
          enc_dict[string.char(b)] = b
        end
        next_code = M.INITIAL_NEXT_CODE
      end

      w = byte
    end
  end

  if #w > 0 then
    codes[#codes + 1] = enc_dict[w]
  end
  codes[#codes + 1] = M.STOP_CODE

  return codes, original_length
end

-- ---------------------------------------------------------------------------
-- Decoder
-- ---------------------------------------------------------------------------

--- Decode a list of LZW codes back to a byte string.
-- Handles CLEAR_CODE, STOP_CODE, and the tricky-token edge case.
function M.decode_codes(codes)
  -- Decode dictionary: index (0-based code) -> string sequence.
  -- Lua tables are 1-indexed so we store code->entry in a hash map.
  local dec_dict = {}
  for b = 0, 255 do
    dec_dict[b] = string.char(b)
  end
  dec_dict[M.CLEAR_CODE] = ""
  dec_dict[M.STOP_CODE]  = ""

  local next_code = M.INITIAL_NEXT_CODE
  local max_entries = 2 ^ M.MAX_CODE_SIZE
  local output = {}
  local prev_code = nil

  for _, code in ipairs(codes) do
    if code == M.CLEAR_CODE then
      dec_dict = {}
      for b = 0, 255 do
        dec_dict[b] = string.char(b)
      end
      dec_dict[M.CLEAR_CODE] = ""
      dec_dict[M.STOP_CODE]  = ""
      next_code = M.INITIAL_NEXT_CODE
      prev_code = nil

    elseif code == M.STOP_CODE then
      break

    else
      local entry
      if dec_dict[code] ~= nil then
        entry = dec_dict[code]
      elseif code == next_code and prev_code ~= nil then
        -- Tricky token: code not yet in dict.
        local prev_entry = dec_dict[prev_code]
        entry = prev_entry .. string.char(string.byte(prev_entry, 1))
      else
        -- Invalid code -- skip.
        goto continue
      end

      output[#output + 1] = entry

      if prev_code ~= nil and next_code < max_entries then
        local prev_entry = dec_dict[prev_code]
        dec_dict[next_code] = prev_entry .. string.char(string.byte(entry, 1))
        next_code = next_code + 1
      end

      prev_code = code
    end

    ::continue::
  end

  return table.concat(output)
end

-- ---------------------------------------------------------------------------
-- Serialisation
-- ---------------------------------------------------------------------------

--- Pack a list of LZW codes into the CMP03 wire format.
-- Header: 4-byte big-endian original_length.
-- Body:   LSB-first variable-width bit-packed codes.
function M.pack_codes(codes, original_length)
  local writer = bw_new()
  local code_size = M.INITIAL_CODE_SIZE
  local next_code = M.INITIAL_NEXT_CODE
  local max = 2 ^ M.MAX_CODE_SIZE

  for _, code in ipairs(codes) do
    bw_write(writer, code, code_size)

    if code == M.CLEAR_CODE then
      code_size = M.INITIAL_CODE_SIZE
      next_code = M.INITIAL_NEXT_CODE
    elseif code ~= M.STOP_CODE then
      if next_code < max then
        next_code = next_code + 1
        if next_code > (2 ^ code_size) and code_size < M.MAX_CODE_SIZE then
          code_size = code_size + 1
        end
      end
    end
  end
  bw_flush(writer)

  local body = bw_to_string(writer)

  -- Big-endian uint32 header.
  local n = original_length
  local b4 = n % 256; n = math.floor(n / 256)
  local b3 = n % 256; n = math.floor(n / 256)
  local b2 = n % 256; n = math.floor(n / 256)
  local b1 = n % 256
  local header = string.char(b1, b2, b3, b4)

  return header .. body
end

--- Unpack CMP03 wire-format bytes into a list of LZW codes.
-- Returns codes (table) and original_length (integer).
function M.unpack_codes(data)
  if #data < 4 then
    return { M.CLEAR_CODE, M.STOP_CODE }, 0
  end

  -- Read big-endian uint32 header.
  local b1, b2, b3, b4 = string.byte(data, 1, 4)
  local original_length = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4

  local reader = br_new(string.sub(data, 5))
  local codes = {}
  local code_size = M.INITIAL_CODE_SIZE
  local next_code = M.INITIAL_NEXT_CODE
  local max = 2 ^ M.MAX_CODE_SIZE

  while not br_exhausted(reader) do
    local code = br_read(reader, code_size)
    if code == nil then break end

    codes[#codes + 1] = code

    if code == M.STOP_CODE then
      break
    elseif code == M.CLEAR_CODE then
      code_size = M.INITIAL_CODE_SIZE
      next_code = M.INITIAL_NEXT_CODE
    elseif next_code < max then
      next_code = next_code + 1
      if next_code > (2 ^ code_size) and code_size < M.MAX_CODE_SIZE then
        code_size = code_size + 1
      end
    end
  end

  return codes, original_length
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

--- Compress a byte string using LZW. Returns CMP03 wire-format bytes.
function M.compress(data)
  local codes, original_length = M.encode_codes(data)
  return M.pack_codes(codes, original_length)
end

--- Decompress CMP03 wire-format bytes. Returns the original byte string.
function M.decompress(data)
  local codes, original_length = M.unpack_codes(data)
  local result = M.decode_codes(codes)
  return string.sub(result, 1, original_length)
end

return M

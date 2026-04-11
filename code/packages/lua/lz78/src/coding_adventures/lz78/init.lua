-- ============================================================================
-- CodingAdventures.LZ78
-- ============================================================================
--
-- LZ78 lossless compression algorithm (Lempel & Ziv, 1978).
-- Part of the CMP compression series in the coding-adventures monorepo.
--
-- What Is LZ78?
-- -------------
--
-- LZ78 builds an explicit trie-based dictionary of byte sequences as it
-- encodes. Both encoder and decoder build the same dictionary independently —
-- no dictionary is transmitted on the wire.
--
-- Token: {dict_index, next_char}
-- --------------------------------
--
--   dict_index — ID of the longest dictionary prefix matched (0 = literal)
--   next_char  — The byte following the match (0 = flush sentinel)
--
-- How It Differs from LZ77
-- -------------------------
--
-- LZ77 (CMP00) uses a *sliding window*: it forgets bytes that fall off the
-- back. LZ78 grows a *global dictionary* that never forgets, making it
-- better for repetitive data spread across a file.
--
-- Dictionary entries are (parent_id, byte) pairs. Decoding reconstructs any
-- sequence by walking the parent chain from tip to root and reversing.
--
-- Wire Format (CMP01)
-- --------------------
--
--   Bytes 0–3:  original length (big-endian uint32)
--   Bytes 4–7:  token count    (big-endian uint32)
--   Bytes 8+:   N × 4 bytes each:
--                 [0..1]  dict_index (big-endian uint16)
--                 [2]     next_char  (uint8)
--                 [3]     reserved   (0x00)
--
-- Series
-- ------
--
--   CMP00 (LZ77,    1977) — Sliding-window backreferences.
--   CMP01 (LZ78,    1978) — Explicit dictionary (trie). ← this module
--   CMP02 (LZSS,    1982) — LZ77 + flag bits.
--   CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
--   CMP04 (Huffman, 1952) — Entropy coding.
--   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG.
--
-- ============================================================================

local M = {}

-- ─── TrieCursor ──────────────────────────────────────────────────────────────
--
-- A step-by-step cursor for navigating a byte-keyed trie.
--
-- Unlike a full trie API (which operates on complete keys), TrieCursor
-- maintains a current position and advances one byte at a time. This is
-- the core abstraction for streaming dictionary algorithms:
--
--   LZ78 (CMP01): step(cursor, byte) → emit token on miss, insert new entry
--   LZW  (CMP03): same pattern with a pre-seeded 256-entry alphabet
--
-- Trie storage
-- ------------
--
-- The trie is an arena: a flat array of nodes, each a {dict_id, children}
-- pair. Node 1 is the root (Lua tables are 1-indexed). Children is a
-- table mapping byte value (0–255) to child node index.
--
-- Usage
-- -----
--
--   local cursor = M.TrieCursor.new()
--   for i = 1, #data do
--     local byte = data:byte(i)
--     if not M.TrieCursor.step(cursor, byte) then
--       emit_token(M.TrieCursor.dict_id(cursor), byte)
--       M.TrieCursor.insert(cursor, byte, next_id)
--       M.TrieCursor.reset(cursor)
--       next_id = next_id + 1
--     end
--   end
--   if not M.TrieCursor.at_root(cursor) then
--     emit flush token
--   end

M.TrieCursor = {}
M.TrieCursor.__index = M.TrieCursor

--- Create a new TrieCursor with an empty trie. Cursor starts at root.
-- @return cursor
function M.TrieCursor.new()
  local self = setmetatable({}, M.TrieCursor)
  -- Arena: arena[i] = {dict_id=n, children={[byte]=node_idx, ...}}
  -- Node 1 = root (dict_id=0, children={})
  self.arena   = { {dict_id = 0, children = {}} }
  self.current = 1  -- 1-indexed; 1 = root
  return self
end

--- Try to follow the child edge for `byte` from the current position.
-- Returns true and advances if the child exists; false otherwise (cursor
-- stays at current position).
-- @param  byte  integer (0-255)
-- @return bool
function M.TrieCursor:step(byte)
  local child_idx = self.arena[self.current].children[byte]
  if child_idx then
    self.current = child_idx
    return true
  end
  return false
end

--- Add a child edge for `byte` at the current position with `dict_id`.
-- Does not advance the cursor — call reset() to return to root.
-- @param  byte     integer (0-255)
-- @param  dict_id  integer
function M.TrieCursor:insert(byte, dict_id)
  local new_idx = #self.arena + 1
  self.arena[new_idx] = {dict_id = dict_id, children = {}}
  self.arena[self.current].children[byte] = new_idx
end

--- Reset the cursor to the trie root.
function M.TrieCursor:reset()
  self.current = 1
end

--- Dictionary ID at the current cursor position (0 when at root).
-- @return integer
function M.TrieCursor:dict_id()
  return self.arena[self.current].dict_id
end

--- Returns true if the cursor is at the root node.
-- @return bool
function M.TrieCursor:at_root()
  return self.current == 1
end

--- Iterate all (path, dict_id) pairs in the trie (DFS order).
-- @return iterator yielding (path_table, dict_id)
function M.TrieCursor:entries()
  local results = {}
  local function dfs(node_idx, path)
    local node = self.arena[node_idx]
    if node.dict_id > 0 then
      local p = {}
      for i = 1, #path do p[i] = path[i] end
      results[#results + 1] = {path = p, dict_id = node.dict_id}
    end
    for byte, child_idx in pairs(node.children) do
      path[#path + 1] = byte
      dfs(child_idx, path)
      path[#path] = nil
    end
  end
  dfs(1, {})
  table.sort(results, function(a, b) return a.dict_id < b.dict_id end)
  local i = 0
  return function()
    i = i + 1
    if results[i] then
      return results[i].path, results[i].dict_id
    end
  end
end

-- ─── Encoder ──────────────────────────────────────────────────────────────────

--- Encode a string into an LZ78 token array.
--
-- Uses TrieCursor to walk the dictionary one byte at a time. When step()
-- returns false (no child edge), emits a token for the current dict_id plus
-- byte, records the new sequence, and resets to root.
--
-- If input ends mid-match, a flush token with next_char=0 is emitted.
--
-- @param  data          string  Binary input.
-- @param  max_dict_size int     Maximum dictionary entries (default 65536).
-- @return table  Array of {dict_index=n, next_char=n} tables.
--
-- Example:
--   local tokens = M.encode("ABCDE")
--   -- all tokens have dict_index=0 (all literals)
function M.encode(data, max_dict_size)
  max_dict_size = max_dict_size or 65536
  local cursor  = M.TrieCursor.new()
  local next_id = 1
  local tokens  = {}

  for i = 1, #data do
    local byte = data:byte(i)
    if not cursor:step(byte) then
      tokens[#tokens + 1] = {dict_index = cursor:dict_id(), next_char = byte}
      if next_id < max_dict_size then
        cursor:insert(byte, next_id)
        next_id = next_id + 1
      end
      cursor:reset()
    end
  end

  -- Flush partial match at end of stream.
  if not cursor:at_root() then
    tokens[#tokens + 1] = {dict_index = cursor:dict_id(), next_char = 0}
  end

  return tokens
end

-- ─── Decoder ──────────────────────────────────────────────────────────────────

-- Walk the parent chain to reconstruct a dictionary entry.
-- Returns a table of bytes in correct forward order.
local function reconstruct(dict_table, index)
  if index == 0 then return {} end
  local rev = {}
  local idx = index
  while idx ~= 0 do
    local entry = dict_table[idx + 1]  -- +1: Lua is 1-indexed; dict_ids are 0-based
    rev[#rev + 1] = entry[2]  -- byte
    idx = entry[1]             -- parent_id
  end
  -- Reverse to get forward order.
  local fwd = {}
  for i = #rev, 1, -1 do
    fwd[#fwd + 1] = rev[i]
  end
  return fwd
end

--- Decode an LZ78 token array back into the original bytes.
--
-- Mirrors encode(): maintains a parallel dictionary as an array of
-- {parent_id, byte} pairs. For each token, reconstructs the sequence for
-- dict_index, emits it, emits next_char, then adds a new dictionary entry.
--
-- @param  tokens          table   Token array from encode().
-- @param  original_length int     If set, truncates output to this length.
--                                 Pass nil to return all bytes.
-- @return string  Reconstructed binary string.
function M.decode(tokens, original_length)
  -- dict_table[i] = {parent_id, byte}. Entry 1 = root sentinel.
  local dict_table = {{0, 0}}
  local out = {}

  for _, tok in ipairs(tokens) do
    local seq = reconstruct(dict_table, tok.dict_index)
    for _, b in ipairs(seq) do
      out[#out + 1] = b
    end

    if original_length == nil or #out < original_length then
      out[#out + 1] = tok.next_char
    end

    dict_table[#dict_table + 1] = {tok.dict_index, tok.next_char}
  end

  if original_length ~= nil and #out > original_length then
    while #out > original_length do
      out[#out] = nil
    end
  end

  -- Convert byte array to string.
  local chars = {}
  for i, b in ipairs(out) do
    chars[i] = string.char(b)
  end
  return table.concat(chars)
end

-- ─── Serialisation ────────────────────────────────────────────────────────────

-- Write a big-endian uint32 to a table of chars.
local function write_u32(buf, n)
  buf[#buf + 1] = string.char(math.floor(n / 0x1000000) % 0x100)
  buf[#buf + 1] = string.char(math.floor(n / 0x10000) % 0x100)
  buf[#buf + 1] = string.char(math.floor(n / 0x100) % 0x100)
  buf[#buf + 1] = string.char(n % 0x100)
end

-- Write a big-endian uint16 to a table of chars.
local function write_u16(buf, n)
  buf[#buf + 1] = string.char(math.floor(n / 0x100) % 0x100)
  buf[#buf + 1] = string.char(n % 0x100)
end

-- Read a big-endian uint32 from string at byte offset (1-indexed).
local function read_u32(s, pos)
  local a, b, c, d = s:byte(pos, pos + 3)
  return a * 0x1000000 + b * 0x10000 + c * 0x100 + d
end

-- Read a big-endian uint16 from string at byte offset (1-indexed).
local function read_u16(s, pos)
  local a, b = s:byte(pos, pos + 1)
  return a * 0x100 + b
end

--- Serialise tokens to the CMP01 wire format.
-- @param  tokens          table   Token array.
-- @param  original_length int     Original byte count.
-- @return string  Binary wire-format bytes.
function M.serialise_tokens(tokens, original_length)
  local buf = {}
  write_u32(buf, original_length)
  write_u32(buf, #tokens)
  for _, tok in ipairs(tokens) do
    write_u16(buf, tok.dict_index)
    buf[#buf + 1] = string.char(tok.next_char)
    buf[#buf + 1] = string.char(0)
  end
  return table.concat(buf)
end

--- Deserialise CMP01 wire-format bytes back into tokens and original length.
-- @param  data  string  Wire-format binary.
-- @return tokens table, original_length int
function M.deserialise_tokens(data)
  if #data < 8 then return {}, 0 end
  local original_length = read_u32(data, 1)
  local token_count     = read_u32(data, 5)
  local tokens = {}
  for i = 0, token_count - 1 do
    local base = 9 + i * 4  -- 1-indexed: bytes 9..12 are token 0
    if base + 3 > #data then break end
    local dict_index = read_u16(data, base)
    local next_char  = data:byte(base + 2)
    tokens[#tokens + 1] = {dict_index = dict_index, next_char = next_char}
  end
  return tokens, original_length
end

-- ─── One-shot API ─────────────────────────────────────────────────────────────

--- Compress a string using LZ78, returning the CMP01 wire format.
--
-- @param  data          string  Input binary.
-- @param  max_dict_size int     Maximum dictionary entries (default 65536).
-- @return string  Compressed bytes.
--
-- Example:
--   local compressed = M.compress("hello hello hello")
--   assert(M.decompress(compressed) == "hello hello hello")
function M.compress(data, max_dict_size)
  local tokens = M.encode(data, max_dict_size)
  return M.serialise_tokens(tokens, #data)
end

--- Decompress bytes that were compressed with compress().
--
-- @param  data  string  CMP01 wire-format bytes.
-- @return string  Decompressed original bytes.
function M.decompress(data)
  local tokens, original_length = M.deserialise_tokens(data)
  return M.decode(tokens, original_length)
end

return M

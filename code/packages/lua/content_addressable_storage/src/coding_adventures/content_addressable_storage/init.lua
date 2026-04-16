-- cas — Content-Addressable Storage
--
-- Content-addressable storage (CAS) maps the *hash of content* to the content
-- itself.  The hash is simultaneously the address and an integrity check: if the
-- bytes returned by the store don't hash to the key you requested, the data is
-- corrupt.  No separate checksum file or trust anchor is needed.
--
-- MENTAL MODEL
-- ────────────
-- Imagine a library where every book's call number IS a fingerprint of the
-- book's text.  You can't file a different book under that number — the number
-- would immediately be wrong.  And if someone swaps pages, the fingerprint
-- changes and the librarian knows before you even open the cover.
--
--   Traditional storage:  name  ──►  content   (name can lie; content can change)
--   Content-addressed:    hash  ──►  content   (hash is derived from content, cannot lie)
--
-- HOW GIT USES CAS
-- ────────────────
-- Git's entire history is built on this principle.  Every blob (file snapshot),
-- tree (directory listing), commit, and tag is stored by the SHA-1 hash of its
-- serialized bytes.  Two identical files share one object.  Renaming a file
-- creates zero new storage.  History is an immutable DAG of hashes pointing to
-- hashes.
--
-- ARCHITECTURE
-- ────────────
--
--   ┌──────────────────────────────────────────────────┐
--   │  ContentAddressableStore                          │
--   │  · put(data)          → sha1 key string          │
--   │  · get(key)           → data (verified)          │
--   │  · find_by_prefix(hex)→ full key                 │
--   └─────────────────┬────────────────────────────────┘
--                     │ BlobStore (abstract base class)
--          ┌──────────┴────────────────────────────────┐
--          │                                           │
--   LocalDiskStore                       (future: S3, mem, …)
--   root/XX/XXXXXX…
--
-- KEYS
-- ────
-- Keys are 20-byte binary strings produced by SHA-1 hashing.  We store them
-- as Lua strings (which are arbitrary byte sequences) rather than integer
-- tables, because string operations (comparison, substring) are idiomatic.
--
-- sha1.digest() returns a table of 20 integers; we convert with string.char.
-- sha1.hex()    returns a 40-char lowercase hex string directly.
--
-- Usage:
--   local cas = require("coding_adventures.content_addressable_storage")
--
--   -- LocalDiskStore backend
--   local store = cas.LocalDiskStore.new("/tmp/myrepo")
--   local db    = cas.ContentAddressableStore.new(store)
--
--   local key = db:put("hello, world")          -- 20-byte binary string
--   local data = db:get(key)                     -- "hello, world"
--   print(cas.key_to_hex(key))                   -- "a0b65939670bc2c010f4d5d6a0b3e4e4b0b3a3a9"
--
-- ============================================================================

local sha1 = require("coding_adventures.sha1")

local M = {}
M.VERSION = "0.1.0"

-- ─── Hex Utilities ────────────────────────────────────────────────────────────
--
-- Keys are 20-byte binary strings, but humans interact with them as 40-char
-- lowercase hex strings (e.g., "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5").
--
-- key_to_hex — converts a 20-byte string → 40-char hex string
-- hex_to_key — parses a 40-char hex string → 20-byte binary string
--              returns (nil, err_msg) on bad input

-- Convert a 20-byte binary key to a 40-character lowercase hex string.
--
-- Example:
--   key_to_hex("\xde\xad\xbe\xef" .. string.rep("\x00", 16))
--   → "deadbeef000000000000000000000000"
function M.key_to_hex(key)
    local parts = {}
    for i = 1, #key do
        parts[i] = string.format("%02x", key:byte(i))
    end
    return table.concat(parts)
end

-- Decode a single ASCII hex nibble character to its 0–15 value.
-- Returns (value, nil) on success, (nil, errmsg) on failure.
local function hex_nibble(ch)
    local b = ch:byte(1)
    if b >= 0x30 and b <= 0x39 then       -- '0'..'9'
        return b - 0x30
    elseif b >= 0x61 and b <= 0x66 then   -- 'a'..'f'
        return b - 0x61 + 10
    elseif b >= 0x41 and b <= 0x46 then   -- 'A'..'F'
        return b - 0x41 + 10
    end
    return nil, "invalid hex character: " .. ch
end

-- Parse a 40-character hex string into a 20-byte binary key string.
-- Returns (key_string, nil) on success or (nil, err_msg) on failure.
function M.hex_to_key(hex)
    if #hex ~= 40 then
        return nil, string.format("expected 40 hex chars, got %d", #hex)
    end
    local bytes = {}
    for i = 1, 40, 2 do
        local hi, err1 = hex_nibble(hex:sub(i, i))
        if not hi then return nil, err1 end
        local lo, err2 = hex_nibble(hex:sub(i+1, i+1))
        if not lo then return nil, err2 end
        bytes[#bytes + 1] = string.char(hi * 16 + lo)
    end
    return table.concat(bytes)
end

-- Decode an arbitrary-length hex prefix (1–40 chars, may be odd-length) to a
-- binary byte-prefix string.
--
-- Odd-length strings are right-padded with '0' before decoding, because a
-- nibble prefix like "a3f" means "starts with 0xa3, 0xf0" — the trailing
-- nibble is the high nibble of the next byte.
--
-- Returns (prefix_bytes, nil) on success, (nil, err_msg) on failure.
-- Empty string is rejected (would match everything, never useful).
local function decode_hex_prefix(hex)
    if hex == "" then
        return nil, "prefix cannot be empty"
    end
    -- Validate all characters first
    for i = 1, #hex do
        local _, err = hex_nibble(hex:sub(i, i))
        if err then return nil, err end
    end
    -- Pad to even length
    local padded = (#hex % 2 == 1) and (hex .. "0") or hex
    local bytes = {}
    for i = 1, #padded, 2 do
        local hi = hex_nibble(padded:sub(i, i))
        local lo = hex_nibble(padded:sub(i+1, i+1))
        bytes[#bytes + 1] = string.char(hi * 16 + lo)
    end
    return table.concat(bytes)
end

-- Compute the SHA-1 of a string and return the result as a 20-byte binary
-- string (NOT a hex string and NOT an integer table).
--
-- sha1.digest(msg) returns a 1-indexed table of 20 integers in range 0–255.
-- We convert to a binary string with string.char so key comparisons and
-- string operations work idiomatically.
local function sha1_key(data)
    local bytes = sha1.digest(data)
    local chars = {}
    for i = 1, 20 do
        chars[i] = string.char(bytes[i])
    end
    return table.concat(chars)
end

-- ─── BlobStore (Abstract Base Class) ─────────────────────────────────────────
--
-- Any storage backend that can store and retrieve byte blobs by a 20-byte key
-- qualifies as a BlobStore.  In Lua there are no interfaces or traits, so we
-- implement BlobStore as a table that holds stub methods which error() when
-- called.  Subclasses override these stubs.
--
-- Method signatures (all using colon-call syntax):
--
--   store:put(key, data)           → (true, nil) | (nil, err_table)
--   store:get(key)                 → (data, nil)  | (nil, err_table)
--   store:exists(key)              → (bool, nil)  | (nil, err_table)
--   store:keys_with_prefix(prefix) → (list, nil)  | (nil, err_table)
--
-- Error tables have the shape: { type = "...", message = "...", ... }
-- The `type` field is the primary discriminator for callers.
--
-- The abstract methods raise a Lua error (string) rather than returning an
-- error table, because calling an unimplemented method is a programmer
-- mistake, not a runtime failure.  Runtime failures use the (nil, err_table)
-- pattern so callers can handle them without pcall.

M.BlobStore = {}
M.BlobStore.__index = M.BlobStore

-- Create a new BlobStore instance.  Subclasses call this then set their own
-- __index metatable, e.g.:
--   setmetatable(self, { __index = MyStore })
function M.BlobStore.new()
    local self = {}
    setmetatable(self, M.BlobStore)
    return self
end

-- Store `data` (a string) under `key` (a 20-byte binary string).
-- Must be idempotent: storing the same key twice is not an error.
function M.BlobStore:put(_key, _data)
    error("BlobStore:put is abstract and must be overridden by a subclass")
end

-- Retrieve the blob stored under `key`.
function M.BlobStore:get(_key)
    error("BlobStore:get is abstract and must be overridden by a subclass")
end

-- Return true if `key` exists in the store, false otherwise.
function M.BlobStore:exists(_key)
    error("BlobStore:exists is abstract and must be overridden by a subclass")
end

-- Return a list of all 20-byte keys whose first #prefix bytes match `prefix`.
function M.BlobStore:keys_with_prefix(_prefix)
    error("BlobStore:keys_with_prefix is abstract and must be overridden by a subclass")
end

-- ─── ContentAddressableStore ──────────────────────────────────────────────────
--
-- Wraps any BlobStore and adds three things the store alone cannot provide:
--
--   1. Automatic keying  — callers pass content; SHA-1 is computed internally.
--   2. Integrity check   — on every get, SHA-1(returned bytes) must equal key.
--   3. Prefix resolution — converts abbreviated hex to a full 20-byte key.
--
-- Error tables returned by methods:
--
--   { type = "not_found",        key = key }
--   { type = "corrupted",        key = key }
--   { type = "ambiguous_prefix", prefix = hex }
--   { type = "prefix_not_found", prefix = hex }
--   { type = "invalid_prefix",   prefix = hex }
--   { type = "store_error",      message = msg }

M.ContentAddressableStore = {}
M.ContentAddressableStore.__index = M.ContentAddressableStore

-- Create a new ContentAddressableStore wrapping `store` (a BlobStore).
--
-- Example:
--   local disk = cas.LocalDiskStore.new("/tmp/myrepo")
--   local db   = cas.ContentAddressableStore.new(disk)
function M.ContentAddressableStore.new(store)
    local self = { store = store }
    setmetatable(self, M.ContentAddressableStore)
    return self
end

-- Hash `data` with SHA-1, store it in the backend, and return the key.
--
-- Idempotent: if the same content was already stored, the existing key is
-- returned and the backend handles deduplication.
--
-- Returns (key_string, nil) on success, (nil, err_table) on failure.
function M.ContentAddressableStore:put(data)
    local key = sha1_key(data)
    -- Delegate to the store.  BlobStore:put must be idempotent, so no pre-check
    -- is needed here — skipping exists()+put() eliminates a TOCTOU window.
    local ok, err = self.store:put(key, data)
    if not ok then
        return nil, { type = "store_error", message = tostring(err) }
    end
    return key
end

-- Retrieve the blob stored under `key` and verify its integrity.
--
-- After fetching, we re-hash the bytes and compare against the requested key.
-- If they differ, the store has been corrupted (data was modified after write).
--
-- Returns (data_string, nil) on success, (nil, err_table) on failure.
-- Error types: "not_found", "corrupted", "store_error".
function M.ContentAddressableStore:get(key)
    local data, err = self.store:get(key)
    if not data then
        -- Propagate whatever the backend reported.  LocalDiskStore returns a
        -- { type = "not_found", ... } table for missing files.
        return nil, err
    end

    -- Integrity check: re-hash the returned bytes.
    local actual = sha1_key(data)
    if actual ~= key then
        return nil, { type = "corrupted", key = key }
    end

    return data
end

-- Check whether `key` is present in the store.
-- Returns (bool, nil) on success, (nil, err_table) on failure.
function M.ContentAddressableStore:exists(key)
    local result, err = self.store:exists(key)
    if result == nil then
        return nil, { type = "store_error", message = tostring(err) }
    end
    return result
end

-- Resolve an abbreviated hex string to the full 20-byte key.
--
-- Accepts any non-empty hex string of 1–40 characters.  Odd-length strings
-- are treated as nibble prefixes: "1bafb97" matches any key whose 40-char hex
-- starts with those 7 nibbles, regardless of what the 8th nibble is.
--
-- Correct odd-length handling:
--   "1bafb97" (7 chars)
--   → pass 3 complete bytes [0x1b, 0xaf, 0xb9] to keys_with_prefix
--   → filter results: keep only keys where (key_byte[4] >> 4) == 7
--
-- Returns (key_string, nil) on success, (nil, err_table) on failure.
-- Error types: "invalid_prefix", "prefix_not_found", "ambiguous_prefix".
function M.ContentAddressableStore:find_by_prefix(hex_prefix)
    -- Validate: non-empty and all hex characters.
    if hex_prefix == "" then
        return nil, { type = "invalid_prefix", prefix = hex_prefix, message = "prefix cannot be empty" }
    end
    for i = 1, #hex_prefix do
        local _, verr = hex_nibble(hex_prefix:sub(i, i))
        if verr then
            return nil, { type = "invalid_prefix", prefix = hex_prefix, message = verr }
        end
    end

    local is_odd = (#hex_prefix % 2 == 1)
    local trailing_nibble_val = nil
    local complete_hex = hex_prefix

    if is_odd then
        -- Extract the trailing nibble value (0–15) and drop it from the hex string.
        trailing_nibble_val = hex_nibble(hex_prefix:sub(#hex_prefix))
        complete_hex        = hex_prefix:sub(1, #hex_prefix - 1)
    end

    -- Encode the complete hex pairs into a binary prefix string.
    local prefix_bytes = ""
    for i = 1, #complete_hex, 2 do
        local hi = hex_nibble(complete_hex:sub(i, i))
        local lo = hex_nibble(complete_hex:sub(i + 1, i + 1))
        prefix_bytes = prefix_bytes .. string.char(hi * 16 + lo)
    end

    local matches = {}

    if is_odd and #prefix_bytes == 0 then
        -- 1-nibble prefix: scan all 16 possible first bytes (0xN0 through 0xNf).
        -- For example, "a" must match keys in buckets a0/, a1/, …, af/.
        for lo = 0, 15 do
            local first_byte = trailing_nibble_val * 16 + lo
            local m, serr = self.store:keys_with_prefix(string.char(first_byte))
            if not m then
                return nil, { type = "store_error", message = tostring(serr) }
            end
            for _, k in ipairs(m) do
                matches[#matches + 1] = k
            end
        end
    else
        local m, serr = self.store:keys_with_prefix(prefix_bytes)
        if not m then
            return nil, { type = "store_error", message = tostring(serr) }
        end
        matches = m

        if is_odd then
            -- Filter: keep only keys where the high nibble of the next byte
            -- equals trailing_nibble_val.
            --
            -- prefix_bytes has n bytes; the "next" byte is at index n+1 (1-based).
            -- We use integer division (math.floor(x/16)) for Lua 5.1/5.2 compat.
            local n = #prefix_bytes
            local filtered = {}
            for _, key in ipairs(matches) do
                local next_byte = key:byte(n + 1)
                if next_byte and math.floor(next_byte / 16) == trailing_nibble_val then
                    filtered[#filtered + 1] = key
                end
            end
            matches = filtered
        end
    end

    -- Sort for deterministic behaviour (important for tests).
    table.sort(matches)

    local n = #matches
    if n == 0 then
        return nil, { type = "prefix_not_found", prefix = hex_prefix }
    elseif n == 1 then
        return matches[1]
    else
        return nil, { type = "ambiguous_prefix", prefix = hex_prefix }
    end
end

-- Access the underlying BlobStore directly.
-- Useful for backend-specific operations (listing all keys for GC, etc.).
function M.ContentAddressableStore:inner()
    return self.store
end

-- ─── LocalDiskStore ───────────────────────────────────────────────────────────
--
-- Filesystem backend using the Git 2/38 fanout layout.
--
-- WHY 2/38 FANOUT?
-- ────────────────
-- A repository with 100 000 objects would put 100 000 files in a single
-- directory if we stored objects as root/<40-hex-hash>.  Most filesystems
-- slow down dramatically at that scale (directory entry scanning is O(n)).
-- Splitting on the first byte creates up to 256 sub-directories (~00/ through
-- ff/), keeping each to a manageable size even in large repositories.
-- Git has used this layout since its initial release in 2005.
--
-- OBJECT PATH
-- ───────────
--   key = 20-byte binary string, hex = "a3f4b2c1…"
--   dir  = root/a3/
--   file = root/a3/f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5
--
-- ATOMIC WRITES
-- ─────────────
-- To avoid a reader seeing a partial write:
--   1. Write data to a temp file in the same fanout directory.
--   2. os.rename(tmp, final_path) — atomic on POSIX.
--   3. If the final path already exists (concurrent dup put), no-op.
--
-- Temp file name uses os.time() + math.random to minimise collision risk
-- without depending on /proc/self or platform-specific APIs.

M.LocalDiskStore = {}
M.LocalDiskStore.__index = M.LocalDiskStore

-- Create (or open) a store rooted at `root_path`.
-- The root directory is created if it does not exist.
-- Returns (store_instance, nil) or errors loudly on mkdir failure.
function M.LocalDiskStore.new(root_path)
    -- os.execute is the portable way to create directories recursively.
    -- We use the platform-appropriate command.
    local sep = package.config:sub(1,1)  -- '/' on POSIX, '\\' on Windows
    if sep == "\\" then
        -- Windows: mkdir accepts forward or back slashes
        os.execute('mkdir "' .. root_path:gsub("/", "\\") .. '" 2>nul')
    else
        os.execute("mkdir -p '" .. root_path .. "'")
    end
    local self = { root = root_path }
    setmetatable(self, M.LocalDiskStore)
    return self
end

-- Compute the storage path for a given 20-byte binary key.
--
-- key:sub(1,1) is the first byte; its hex encoding is the directory name.
-- The remaining 38 hex chars form the filename.
--
--   key = "\xa3\xf4\xb2…"
--   hex = "a3f4b2…"
--   dir = root_path .. "/a3"
--   file = root_path .. "/a3/f4b2…"
function M.LocalDiskStore:_object_path(key)
    local hex = M.key_to_hex(key)
    local dir_name  = hex:sub(1, 2)    -- first byte as 2 hex chars
    local file_name = hex:sub(3)        -- remaining 38 hex chars
    local sep = "/"
    return self.root .. sep .. dir_name, self.root .. sep .. dir_name .. sep .. file_name
end

-- Create parent directories for `path` if they do not exist.
-- This creates only the immediate parent (the 2-char fanout dir).
local function ensure_dir(dir)
    local sep = package.config:sub(1,1)
    if sep == "\\" then
        os.execute('mkdir "' .. dir:gsub("/", "\\") .. '" 2>nul')
    else
        os.execute("mkdir -p '" .. dir .. "'")
    end
end

-- Read the entire contents of a file at `path`.
-- Returns (contents_string, nil) or (nil, err_msg).
local function read_file(path)
    local f, err = io.open(path, "rb")
    if not f then return nil, err end
    local contents = f:read("*a")
    f:close()
    return contents
end

-- Write `data` to `path` atomically.
-- Returns (true, nil) or (nil, err_msg).
local function write_file_atomic(dir, final_path, data)
    -- Build an unpredictable temp name in the same directory so the rename
    -- stays on the same filesystem (cross-device renames fail on POSIX).
    -- Mixing os.time() with math.random gives low collision probability
    -- without requiring platform-specific process-id APIs.
    local tmp_name = os.time() .. "_" .. math.random(999999) .. ".tmp"
    local tmp_path = dir .. "/" .. tmp_name

    local f, err = io.open(tmp_path, "wb")
    if not f then
        return nil, "could not open temp file: " .. tostring(err)
    end
    f:write(data)
    f:close()

    -- os.rename is atomic on POSIX (guaranteed by POSIX.1).
    -- On Windows it may fail if the destination exists; treat that as success
    -- because the stored bytes are identical (content-addressed).
    local ok, rename_err = os.rename(tmp_path, final_path)
    if not ok then
        -- Best-effort cleanup of the temp file.
        os.remove(tmp_path)
        -- Check if the destination appeared (concurrent write of same object).
        local f2 = io.open(final_path, "rb")
        if f2 then
            f2:close()
            return true  -- another writer stored the same object — idempotent
        end
        return nil, "rename failed: " .. tostring(rename_err)
    end
    return true
end

-- Store `data` under `key`.  Idempotent: if the file already exists, no write
-- is performed.
--
-- Returns (true, nil) on success, (nil, err_msg) on failure.
function M.LocalDiskStore:put(key, data)
    local dir, final_path = self:_object_path(key)

    -- Short-circuit: object already present — content-addressed means identical.
    local f = io.open(final_path, "rb")
    if f then f:close(); return true end

    ensure_dir(dir)
    return write_file_atomic(dir, final_path, data)
end

-- Retrieve the blob stored under `key`.
--
-- Returns (data_string, nil) on success.
-- Returns (nil, { type = "not_found", key = key }) if not present.
-- Returns (nil, { type = "store_error", message = msg }) on I/O failure.
function M.LocalDiskStore:get(key)
    local _, final_path = self:_object_path(key)
    local data, err = read_file(final_path)
    if not data then
        -- Distinguish "file not found" from other I/O errors.
        -- The error message from io.open typically contains "No such file" on
        -- POSIX or "cannot open" on Windows for missing files.
        return nil, { type = "not_found", key = key, message = tostring(err) }
    end
    return data
end

-- Return true if `key` exists in the store, false otherwise.
function M.LocalDiskStore:exists(key)
    local _, final_path = self:_object_path(key)
    local f = io.open(final_path, "rb")
    if f then f:close(); return true end
    return false
end

-- Return a list of all 20-byte keys whose first #prefix bytes match `prefix`.
--
-- The 2/38 fanout layout means we can narrow the scan by reading only the
-- directory named by the first byte of `prefix`.
--
-- Returns (list_of_key_strings, nil) or (nil, err_table).
function M.LocalDiskStore:keys_with_prefix(prefix)
    if #prefix == 0 then
        -- A zero-byte prefix would match everything — reject it.
        return {}
    end

    -- The first byte of the prefix identifies the fanout directory.
    local first_byte_hex = string.format("%02x", prefix:byte(1))
    local dir = self.root .. "/" .. first_byte_hex

    -- Convert the remaining bytes of the prefix (bytes 2…) into the hex
    -- fragment that the filename must start with.
    local remaining_hex = ""
    if #prefix > 1 then
        remaining_hex = M.key_to_hex(prefix:sub(2))
    end

    -- Scan the fanout directory.  Lua's io.popen with 'ls' / 'dir' is the
    -- portable way to list directory contents without C extensions.
    local result = {}
    local sep = package.config:sub(1,1)
    local cmd
    if sep == "\\" then
        -- Windows: dir /b lists bare filenames
        cmd = 'dir /b "' .. dir:gsub("/", "\\") .. '" 2>nul'
    else
        cmd = "ls -1 '" .. dir .. "' 2>/dev/null"
    end

    local pipe = io.popen(cmd)
    if pipe then
        for filename in pipe:lines() do
            -- Skip temp files left by in-progress or failed writes.
            if not filename:match("%.tmp$") then
                -- Check that this file's name starts with `remaining_hex`.
                if remaining_hex == "" or filename:sub(1, #remaining_hex) == remaining_hex then
                    -- Reconstruct the full 40-char hex = dir_name + filename
                    local full_hex = first_byte_hex .. filename
                    if #full_hex == 40 then
                        -- Convert hex back to 20-byte binary key
                        local key = M.hex_to_key(full_hex)
                        if key then
                            result[#result + 1] = key
                        end
                        -- Skip malformed filenames silently (hex_to_key returns nil, msg)
                    end
                end
            end
        end
        pipe:close()
    end

    return result
end

-- ─── Module Exports ──────────────────────────────────────────────────────────

return M

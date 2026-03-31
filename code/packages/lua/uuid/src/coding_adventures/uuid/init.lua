-- ============================================================================
-- uuid — Pure Lua UUID v1/v3/v4/v5/v7 generation and parsing
-- ============================================================================
--
-- A UUID (Universally Unique Identifier) is a 128-bit label used to identify
-- information in computer systems. UUIDs are defined by RFC 4122 and ITU-T
-- X.667. Their canonical textual form is 32 hexadecimal digits grouped as:
--
--     xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
--     8        4    4    4    12   hex digits
--
-- Where M is the "version" nibble (1–7) and N is the "variant" bits (10xx
-- for RFC 4122 UUIDs, giving N ∈ {8,9,a,b}).
--
-- ## Version Overview
--
-- | Version | Based On                        | Use Case                           |
-- |---------|---------------------------------|------------------------------------|
-- | v1      | Current time + MAC address      | Time-ordered IDs, database keys    |
-- | v3      | MD5(namespace + name)           | Deterministic IDs from names       |
-- | v4      | Random                          | General-purpose unique IDs         |
-- | v5      | SHA-1(namespace + name)         | Deterministic IDs (preferred v3)   |
-- | v7      | Unix ms timestamp + random      | Sortable by time, better than v1   |
--
-- ## Well-Known Namespaces (RFC 4122 §4.3)
--
-- RFC 4122 defines four standard namespace UUIDs that should be used as the
-- namespace argument to v3/v5:
--
--   NAMESPACE_DNS  = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
--   NAMESPACE_URL  = "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
--   NAMESPACE_OID  = "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
--   NAMESPACE_X500 = "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
--
-- ## Implementation Notes
--
-- Randomness: We seed math.random with os.time() + os.clock() on module load.
-- This is adequate for general-purpose use but not cryptographically secure.
-- For cryptographic applications, use a CSPRNG.
--
-- v1 Node: We generate a random 48-bit node (simulating a MAC address) rather
-- than reading the real MAC address. RFC 4122 §4.5 explicitly permits this.
--
-- v7 Time: We use os.time() for the Unix epoch seconds. Millisecond precision
-- is simulated by combining os.time() * 1000 + (os.clock() * 1000 % 1000).
--
-- ## RFC 4122 Test Vectors
--
-- These MUST pass for v3 and v5 (from the RFC Appendix B):
--
--   v3(NAMESPACE_DNS, "www.example.com") = "5df41881-3aed-3515-88a7-2f4a814cf09e"
--   v5(NAMESPACE_DNS, "www.example.com") = "2ed6657d-e927-568b-95e1-2665a8aea6a2"
--   (verified against Python's uuid.uuid5() reference implementation)
--
-- Usage:
--
--   local uuid = require("coding_adventures.uuid")
--
--   print(uuid.generate_v4())
--   -- "550e8400-e29b-41d4-a716-446655440000"  (example)
--
--   local u = uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com")
--   print(u)  -- "2ed6657d-e927-568b-95e3-af9f787f5a91"
--
--   local info = uuid.parse(u)
--   print(info.version)  -- 5
--
-- ============================================================================

local md5  = require("coding_adventures.md5")
local sha1 = require("coding_adventures.sha1")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Well-known namespace UUIDs (RFC 4122 §4.3)
-- ============================================================================

--- NAMESPACE_DNS is the standard namespace for DNS hostnames.
-- Use this when generating a name-based UUID from a DNS name.
M.NAMESPACE_DNS  = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

--- NAMESPACE_URL is the standard namespace for URLs.
M.NAMESPACE_URL  = "6ba7b811-9dad-11d1-80b4-00c04fd430c8"

--- NAMESPACE_OID is the standard namespace for ISO OIDs.
M.NAMESPACE_OID  = "6ba7b812-9dad-11d1-80b4-00c04fd430c8"

--- NAMESPACE_X500 is the standard namespace for X.500 DNs.
M.NAMESPACE_X500 = "6ba7b814-9dad-11d1-80b4-00c04fd430c8"

-- ============================================================================
-- Seed the random number generator
-- ============================================================================

-- We combine os.time() (seconds since epoch) with os.clock() (CPU time) to
-- get a more unique seed than os.time() alone. This fallback is used only
-- when /dev/urandom is unavailable (e.g., Windows without a CSPRNG adapter).
math.randomseed(os.time() + math.floor(os.clock() * 1e6))

-- Discard the first few values — many LCG-based PRNGs have poor initial
-- distribution. This is a well-known "warm-up" technique.
for _ = 1, 3 do math.random() end

-- ============================================================================
-- Internal: random byte generation
-- ============================================================================

--- random_bytes(n) → table of n random integers in [0, 255]
--
-- Attempts to read from /dev/urandom (a cryptographically secure source
-- available on Linux, macOS, and *BSD). Falls back to math.random if
-- /dev/urandom is not available (e.g., Windows).
--
-- NOTE: When using UUID v4 as security tokens (session IDs, CSRF tokens,
-- password-reset links), ensure /dev/urandom is available. math.random is
-- a predictable PRNG seeded from wall-clock time and is NOT suitable for
-- security-sensitive UUID generation.
local function random_bytes(n)
    -- Try /dev/urandom first (CSPRNG on POSIX systems)
    local f = io.open("/dev/urandom", "rb")
    if f then
        local data = f:read(n)
        f:close()
        if data and #data == n then
            local t = {}
            for i = 1, n do
                t[i] = data:byte(i)
            end
            return t
        end
    end
    -- Fallback: math.random (not cryptographically secure)
    local t = {}
    for i = 1, n do
        t[i] = math.random(0, 255)
    end
    return t
end

-- ============================================================================
-- Internal: UUID formatting
-- ============================================================================

--- bytes_to_uuid_string(bytes) → UUID string
--
-- Takes a 16-byte table and formats it as a standard UUID string:
--
--     xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
--     bytes 1-4  5-6  7-8  9-10  11-16
--
-- The bytes are formatted as lowercase hex digits, two per byte.
local function bytes_to_uuid_string(bytes)
    -- Each group: bytes 1-4, 5-6, 7-8, 9-10, 11-16
    local function hex(b) return string.format("%02x", b) end

    local parts = {}
    for i = 1, 4  do parts[#parts+1] = hex(bytes[i]) end
    parts[#parts+1] = "-"
    for i = 5, 6  do parts[#parts+1] = hex(bytes[i]) end
    parts[#parts+1] = "-"
    for i = 7, 8  do parts[#parts+1] = hex(bytes[i]) end
    parts[#parts+1] = "-"
    for i = 9, 10 do parts[#parts+1] = hex(bytes[i]) end
    parts[#parts+1] = "-"
    for i = 11, 16 do parts[#parts+1] = hex(bytes[i]) end

    return table.concat(parts)
end

--- set_version_and_variant(bytes, version) → bytes (mutated in-place)
--
-- Sets the RFC 4122 version and variant bits in a 16-byte UUID:
--
--   Version bits: byte[7], top nibble (bits 15-12 of the time_hi_and_version
--   field). Set to 0100 (v4), 0011 (v3), 0101 (v5), etc.
--
--   Variant bits: byte[9], top two bits must be 10xx (RFC 4122 variant).
--   We mask the byte to 0x3f and OR with 0x80 to set bits 7-6 = "10".
--
-- This follows the UUID byte layout defined in RFC 4122 §4.1.2:
--
--     time_low               (4 bytes) = bytes 1-4
--     time_mid               (2 bytes) = bytes 5-6
--     time_hi_and_version    (2 bytes) = bytes 7-8
--     clock_seq_hi_and_reserved (1 byte) = byte 9
--     clock_seq_low          (1 byte) = byte 10
--     node                   (6 bytes) = bytes 11-16
local function set_version_and_variant(bytes, version)
    -- byte[7] = time_hi_and_version
    -- Clear top nibble (mask 0x0f), then OR version nibble shifted left 4
    bytes[7] = (bytes[7] & 0x0f) | (version << 4)

    -- byte[9] = clock_seq_hi_and_reserved
    -- Clear top 2 bits (mask 0x3f), then set to "10" (OR 0x80)
    bytes[9] = (bytes[9] & 0x3f) | 0x80

    return bytes
end

-- ============================================================================
-- Internal: UUID parsing helpers
-- ============================================================================

--- parse_uuid_to_bytes(uuid_str) → table of 16 bytes, or nil on error
--
-- Converts a UUID string like "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
-- into a 16-element table of integer bytes [0..255].
--
-- We strip dashes, then parse each pair of hex digits as one byte.
local function parse_uuid_to_bytes(uuid_str)
    -- Remove dashes
    local hex_str = uuid_str:gsub("-", "")

    -- Must be exactly 32 hex characters
    if #hex_str ~= 32 then return nil end
    if hex_str:find("[^0-9a-fA-F]") then return nil end

    local bytes = {}
    for i = 1, 32, 2 do
        local byte_hex = hex_str:sub(i, i+1)
        bytes[#bytes+1] = tonumber(byte_hex, 16)
    end

    return bytes
end

-- ============================================================================
-- nil_uuid
-- ============================================================================

--- nil_uuid() → "00000000-0000-0000-0000-000000000000"
--
-- The nil UUID is the UUID consisting of all zeros. It is used to represent
-- "no UUID" or an uninitialized UUID field (RFC 4122 §4.1.7).
function M.nil_uuid()
    return "00000000-0000-0000-0000-000000000000"
end

-- ============================================================================
-- validate
-- ============================================================================

--- validate(uuid_str) → boolean
--
-- Returns true if the string is a syntactically valid UUID in the canonical
-- 8-4-4-4-12 format with lowercase or uppercase hex digits.
--
-- This only checks format, not whether the version/variant bits are set
-- correctly. For example, the nil UUID "00000000-0000-0000-0000-000000000000"
-- passes validation even though it has version 0.
function M.validate(uuid_str)
    if type(uuid_str) ~= "string" then return false end
    -- Canonical UUID pattern: 8-4-4-4-12 hex digits with dashes
    local pattern = "^%x%x%x%x%x%x%x%x%-%x%x%x%x%-%x%x%x%x%-%x%x%x%x%-%x%x%x%x%x%x%x%x%x%x%x%x$"
    return uuid_str:match(pattern) ~= nil
end

-- ============================================================================
-- parse
-- ============================================================================

--- parse(uuid_str) → table with fields, or nil, error_string
--
-- Parses a UUID string and returns a table with:
--
--   {
--     version  = integer (1-8, or 0 for nil UUID),
--     variant  = string ("rfc4122", "reserved_microsoft", "reserved_future", "ncs"),
--     bytes    = table of 16 integers,
--   }
--
-- Returns nil, "error message" if the string is not a valid UUID.
function M.parse(uuid_str)
    if not M.validate(uuid_str) then
        return nil, "invalid UUID format: " .. tostring(uuid_str)
    end

    local bytes = parse_uuid_to_bytes(uuid_str)
    if not bytes then
        return nil, "could not parse UUID bytes from: " .. uuid_str
    end

    -- Extract version: top nibble of byte 7 (time_hi_and_version)
    local version = (bytes[7] >> 4) & 0x0f

    -- Extract variant from top bits of byte 9 (clock_seq_hi_and_reserved)
    -- RFC 4122 variant encoding:
    --   0xxxxxxx = NCS backward compatibility
    --   10xxxxxx = RFC 4122 (IETF) variant
    --   110xxxxx = Microsoft COM/DCOM backward compatibility
    --   111xxxxx = Reserved for future use
    local b9 = bytes[9]
    local variant
    if (b9 & 0x80) == 0 then
        variant = "ncs"
    elseif (b9 & 0xc0) == 0x80 then
        variant = "rfc4122"
    elseif (b9 & 0xe0) == 0xc0 then
        variant = "reserved_microsoft"
    else
        variant = "reserved_future"
    end

    return {
        version = version,
        variant = variant,
        bytes   = bytes,
    }
end

-- ============================================================================
-- generate_v4 — Random UUID
-- ============================================================================

--- generate_v4() → UUID string
--
-- ## How UUID v4 Works
--
-- RFC 4122 §4.4 specifies UUID v4 as follows:
--
--   1. Set all 128 bits to pseudo-random values.
--   2. Set the version: bits 15-12 of byte[7] = 0100 (version 4).
--   3. Set the variant: bits 7-6 of byte[9] = 10 (RFC 4122 variant).
--
-- This leaves 122 bits of actual randomness (128 - 6 version/variant bits).
-- The probability of a collision across 2^61 UUIDs is roughly 50% — in
-- practice, you would need to generate trillions of UUIDs before collision
-- becomes remotely likely.
--
-- ## Example
--
--     550e8400-e29b-41d4-a716-446655440000
--                    ^                      ← '4' = version 4
--                         ^                 ← 'a' = 1010 = RFC 4122 variant
--
-- @return UUID string in canonical 8-4-4-4-12 format
function M.generate_v4()
    local bytes = random_bytes(16)
    set_version_and_variant(bytes, 4)
    return bytes_to_uuid_string(bytes)
end

-- ============================================================================
-- generate_v1 — Time-based UUID
-- ============================================================================

--- generate_v1() → UUID string
--
-- ## How UUID v1 Works
--
-- UUID v1 encodes the current time and node (MAC address) into the UUID:
--
--   1. time: 60-bit count of 100-nanosecond intervals since 15 October 1582
--      (the start of the Gregorian calendar). This is the UUID epoch.
--
--   2. clock_seq: 14-bit value to handle clock resets or multiple UUIDs
--      generated in the same 100ns interval.
--
--   3. node: 48-bit MAC address (or random if not available).
--
-- ## Simplifications in This Implementation
--
-- We use os.time() (Unix epoch seconds) converted to the UUID epoch, and
-- os.clock() for sub-second precision. Since Lua's os.time() only has
-- 1-second resolution, we add random bits for sub-second portions.
--
-- We generate a random node (48 bits) with the multicast bit set (bit 40 of
-- the node = 1), which RFC 4122 §4.5 defines as the "locally administered"
-- marker to distinguish from real MAC addresses.
--
-- @return UUID string in canonical format
function M.generate_v1()
    -- UUID v1 epoch offset: number of 100ns intervals from 15 Oct 1582 to Unix epoch
    -- = 122192928000000000 (derived from the exact date difference)
    local UUID_EPOCH_OFFSET = 122192928000000000

    -- Current time in 100ns intervals since UUID epoch.
    -- os.time() gives seconds since Unix epoch, multiply by 10^7 for 100ns units.
    -- We add random low bits to simulate sub-second resolution.
    local unix_seconds = os.time()
    local sub_second   = math.random(0, 9999999)  -- up to 10^7 - 1 (100ns intervals in 1s)
    local timestamp    = UUID_EPOCH_OFFSET + unix_seconds * 10000000 + sub_second

    -- Extract the three timestamp fields:
    --   time_low  (32 bits): bits 0-31  of timestamp
    --   time_mid  (16 bits): bits 32-47 of timestamp
    --   time_hi   (12 bits): bits 48-59 of timestamp (version nibble takes top 4 bits)
    local time_low  = timestamp & 0xFFFFFFFF
    local time_mid  = (timestamp >> 32) & 0xFFFF
    local time_hi   = (timestamp >> 48) & 0x0FFF  -- 12 bits, version nibble added later

    -- Clock sequence: 14 random bits to handle clock adjustments
    local clock_seq = math.random(0, 0x3FFF)  -- 14 bits

    -- Node: 48-bit random, with multicast bit set (RFC 4122 §4.5)
    local node_bytes = random_bytes(6)
    node_bytes[1] = node_bytes[1] | 0x01  -- set multicast bit = locally administered

    -- Assemble 16 bytes:
    --   bytes 1-4:   time_low (big-endian)
    --   bytes 5-6:   time_mid (big-endian)
    --   bytes 7-8:   time_hi_and_version (big-endian, version = 1)
    --   byte  9:     clock_seq_hi_and_reserved (top 2 bits = 10)
    --   byte  10:    clock_seq_low
    --   bytes 11-16: node
    local bytes = {
        (time_low >> 24) & 0xFF,
        (time_low >> 16) & 0xFF,
        (time_low >>  8) & 0xFF,
         time_low        & 0xFF,
        (time_mid >>  8) & 0xFF,
         time_mid        & 0xFF,
        -- time_hi_and_version: top nibble = 0001 (version 1)
        0x10 | ((time_hi >> 8) & 0x0F),
         time_hi         & 0xFF,
        -- clock_seq_hi_and_reserved: top 2 bits = 10 (RFC 4122 variant)
        0x80 | ((clock_seq >> 8) & 0x3F),
         clock_seq       & 0xFF,
        -- node
        node_bytes[1], node_bytes[2], node_bytes[3],
        node_bytes[4], node_bytes[5], node_bytes[6],
    }

    return bytes_to_uuid_string(bytes)
end

-- ============================================================================
-- Internal: name-based UUID helper (used by v3 and v5)
-- ============================================================================

--- name_based_uuid(hash_bytes, version) → UUID string
--
-- Common logic for v3 (MD5) and v5 (SHA-1) UUIDs.
-- Takes the first 16 bytes of a hash output, sets the version and variant bits,
-- and formats as a UUID string.
--
-- Why only 16 bytes? RFC 4122 §4.3 specifies using the first 16 bytes of the
-- hash, discarding the remaining bytes. SHA-1 produces 20 bytes, so we drop
-- the last 4. MD5 produces exactly 16 bytes.
--
-- @param hash_bytes  table of at least 16 bytes
-- @param version     UUID version number (3 or 5)
-- @return            UUID string
local function name_based_uuid(hash_bytes, version)
    -- Take the first 16 bytes of the hash output
    local bytes = {}
    for i = 1, 16 do
        bytes[i] = hash_bytes[i]
    end

    -- Apply RFC 4122 version and variant bits
    set_version_and_variant(bytes, version)

    return bytes_to_uuid_string(bytes)
end

-- ============================================================================
-- generate_v3 — MD5-based name UUID
-- ============================================================================

--- generate_v3(namespace_uuid_str, name) → UUID string
--
-- ## How UUID v3 Works
--
-- UUID v3 generates a deterministic UUID from a (namespace, name) pair:
--
--   1. Convert the namespace UUID string to 16 bytes.
--   2. Concatenate namespace_bytes + name_bytes (UTF-8/raw bytes of the name).
--   3. Compute MD5(concatenated_bytes).
--   4. Take the first 16 bytes of the MD5 output.
--   5. Set version bits to 0011 (3) in byte[7].
--   6. Set variant bits to 10xx in byte[9].
--
-- ## Determinism
--
-- The same (namespace, name) pair always produces the same UUID. This is
-- useful for generating stable identifiers for well-known resources (e.g.,
-- the UUID for "www.example.com" in the DNS namespace is always the same).
--
-- ## v3 vs v5
--
-- v5 (SHA-1) is preferred over v3 (MD5) for new applications because:
--   - MD5 has known collision vulnerabilities (though not exploitable here)
--   - SHA-1 provides a larger internal state, reducing theoretical collisions
--
-- ## Test Vector (RFC 4122 Appendix B)
--
--   generate_v3(NAMESPACE_DNS, "www.example.com")
--   → "5df41881-3aed-3515-88a7-2f4a814cf09e"
--
-- @param namespace_uuid_str  namespace UUID string (use M.NAMESPACE_* constants)
-- @param name                name string to hash
-- @return                    UUID string, or nil, error on bad namespace
function M.generate_v3(namespace_uuid_str, name)
    -- Parse namespace UUID into bytes
    local ns_bytes = parse_uuid_to_bytes(namespace_uuid_str)
    if not ns_bytes then
        return nil, "generate_v3: invalid namespace UUID: " .. tostring(namespace_uuid_str)
    end

    -- Build the input to hash: namespace_bytes concatenated with name bytes
    -- We convert both to a Lua string for the md5 module (which expects a string)
    local input_bytes = {}
    for i = 1, 16 do
        input_bytes[#input_bytes+1] = ns_bytes[i]
    end
    for i = 1, #name do
        input_bytes[#input_bytes+1] = string.byte(name, i)
    end

    -- Convert byte table to string for md5.digest
    local input_str = ""
    for _, b in ipairs(input_bytes) do
        input_str = input_str .. string.char(b)
    end

    -- Compute MD5 hash — returns table of 16 bytes
    local hash = md5.digest(input_str)

    return name_based_uuid(hash, 3)
end

-- ============================================================================
-- generate_v5 — SHA-1-based name UUID
-- ============================================================================

--- generate_v5(namespace_uuid_str, name) → UUID string
--
-- ## How UUID v5 Works
--
-- Identical to v3, but uses SHA-1 instead of MD5:
--
--   1. Convert the namespace UUID string to 16 bytes.
--   2. Concatenate namespace_bytes + name_bytes.
--   3. Compute SHA-1(concatenated_bytes).
--   4. Take the first 16 bytes of the 20-byte SHA-1 output.
--   5. Set version bits to 0101 (5) in byte[7].
--   6. Set variant bits to 10xx in byte[9].
--
-- ## Test Vector (RFC 4122 Appendix B, verified against Python uuid.uuid5())
--
--   generate_v5(NAMESPACE_DNS, "www.example.com")
--   → "2ed6657d-e927-568b-95e1-2665a8aea6a2"
--
-- @param namespace_uuid_str  namespace UUID string
-- @param name                name string to hash
-- @return                    UUID string, or nil, error on bad namespace
function M.generate_v5(namespace_uuid_str, name)
    local ns_bytes = parse_uuid_to_bytes(namespace_uuid_str)
    if not ns_bytes then
        return nil, "generate_v5: invalid namespace UUID: " .. tostring(namespace_uuid_str)
    end

    -- Build input: namespace bytes + name bytes
    local input_bytes = {}
    for i = 1, 16 do
        input_bytes[#input_bytes+1] = ns_bytes[i]
    end
    for i = 1, #name do
        input_bytes[#input_bytes+1] = string.byte(name, i)
    end

    -- Convert to string for sha1.digest
    local input_str = ""
    for _, b in ipairs(input_bytes) do
        input_str = input_str .. string.char(b)
    end

    -- Compute SHA-1 hash — returns table of 20 bytes; we use only the first 16
    local hash = sha1.digest(input_str)

    return name_based_uuid(hash, 5)
end

-- ============================================================================
-- generate_v7 — Unix Epoch time-ordered UUID
-- ============================================================================

--- generate_v7() → UUID string
--
-- ## How UUID v7 Works
--
-- UUID v7 (proposed in draft-ietf-uuidrev-rfc4122bis) combines a Unix
-- millisecond timestamp with random bits to produce a monotonically
-- increasing, sortable UUID.
--
-- Bit layout of the 128 bits:
--
--   unix_ts_ms (48 bits): bits 127-80 — milliseconds since Unix epoch
--   ver        ( 4 bits): bits 79-76  — version 0111 (7)
--   rand_a     (12 bits): bits 75-64  — random bits
--   var        ( 2 bits): bits 63-62  — variant 10 (RFC 4122)
--   rand_b     (62 bits): bits 61-0   — random bits
--
-- ## Why v7 over v1?
--
-- - v1 stores time in a fragmented way across three fields (time_low,
--   time_mid, time_hi) that are not lexicographically sortable.
-- - v7 stores the timestamp in the most significant bits, so UUIDs sort
--   chronologically by both byte comparison and string comparison.
-- - v7 uses the Unix epoch (1970) rather than the Gregorian epoch (1582).
--
-- @return UUID string with 48-bit ms timestamp in the high bytes
function M.generate_v7()
    -- Unix timestamp in milliseconds
    -- os.time() gives seconds; we need ms precision.
    -- We approximate sub-second precision using os.clock() fractional part.
    local seconds     = os.time()
    local frac_ms     = math.floor((os.clock() % 1.0) * 1000)
    local unix_ms     = seconds * 1000 + frac_ms

    -- Clamp to 48-bit range (year 10889 problem is far away)
    unix_ms = unix_ms & 0xFFFFFFFFFFFF

    -- Generate 10 random bytes for the remaining fields
    local rand_bytes = random_bytes(10)

    -- Build 16 bytes:
    -- Bytes 1-6: unix_ts_ms (48 bits, big-endian)
    local bytes = {
        (unix_ms >> 40) & 0xFF,
        (unix_ms >> 32) & 0xFF,
        (unix_ms >> 24) & 0xFF,
        (unix_ms >> 16) & 0xFF,
        (unix_ms >>  8) & 0xFF,
         unix_ms        & 0xFF,
        -- Bytes 7-8: ver(4) | rand_a(12) — version nibble + 12 random bits
        0x70 | (rand_bytes[1] & 0x0F),
        rand_bytes[2],
        -- Bytes 9-16: variant(2) | rand_b(62)
        0x80 | (rand_bytes[3] & 0x3F),
        rand_bytes[4],
        rand_bytes[5],
        rand_bytes[6],
        rand_bytes[7],
        rand_bytes[8],
        rand_bytes[9],
        rand_bytes[10],
    }

    return bytes_to_uuid_string(bytes)
end

return M

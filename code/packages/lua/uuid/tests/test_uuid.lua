-- ============================================================================
-- Tests for uuid — UUID v1/v3/v4/v5/v7 generation and parsing
-- ============================================================================
--
-- ## Testing Strategy
--
-- UUID tests must balance two concerns:
--
--   1. Correctness: The RFC 4122 v3 and v5 test vectors must match exactly.
--      These are deterministic hashes with known expected outputs.
--
--   2. Statistical: v1, v4, and v7 are random, so we check format, version
--      bits, variant bits, and uniqueness across multiple samples rather than
--      exact values.
--
-- ## UUID Structure Recap
--
-- Each UUID has the form: xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
--
--   M = version nibble (in the 7th byte's high nibble)
--   N = variant high bits (in the 9th byte): must be 10xx for RFC 4122

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
-- Also make md5 and sha1 available (they are installed as luarocks packages
-- by the BUILD script, but the path below covers running from tests/ directly)
package.path = "../../md5/src/?.lua;" .. "../../md5/src/?/init.lua;" .. package.path
package.path = "../../sha1/src/?.lua;" .. "../../sha1/src/?/init.lua;" .. package.path

local uuid = require("coding_adventures.uuid")

-- ============================================================================
-- Helpers
-- ============================================================================

--- uuid_version(s) — extract the version digit (character position 15)
-- In "xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx", M is at index 15 (1-based).
local function uuid_version(s)
    -- Position 15: after "8char-4char-" = 8+1+4+1 = 14 chars, then char 15
    return tonumber(s:sub(15, 15), 16)
end

--- uuid_variant_char(s) — the first character of the 4th group
-- In "xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx", N is at position 20 (1-based).
-- For RFC 4122 variant, N must be 8, 9, a, or b (binary: 10xx)
local function uuid_variant_ok(s)
    local c = s:sub(20, 20):lower()
    return c == "8" or c == "9" or c == "a" or c == "b"
end

--- is_valid_format(s) — checks the canonical UUID format
local function is_valid_format(s)
    return s:match("^%x%x%x%x%x%x%x%x%-%x%x%x%x%-%x%x%x%x%-%x%x%x%x%-%x%x%x%x%x%x%x%x%x%x%x%x$") ~= nil
end

-- ============================================================================
-- nil_uuid
-- ============================================================================

describe("nil_uuid", function()
    it("returns the all-zeros UUID", function()
        assert.are.equal(uuid.nil_uuid(), "00000000-0000-0000-0000-000000000000")
    end)

    it("validates as a valid format", function()
        assert.is_true(uuid.validate(uuid.nil_uuid()))
    end)
end)

-- ============================================================================
-- validate
-- ============================================================================

describe("validate", function()
    it("accepts a valid lowercase UUID", function()
        assert.is_true(uuid.validate("550e8400-e29b-41d4-a716-446655440000"))
    end)

    it("accepts a valid uppercase UUID", function()
        assert.is_true(uuid.validate("550E8400-E29B-41D4-A716-446655440000"))
    end)

    it("accepts the nil UUID", function()
        assert.is_true(uuid.validate("00000000-0000-0000-0000-000000000000"))
    end)

    it("rejects a UUID that is too short", function()
        assert.is_false(uuid.validate("550e8400-e29b-41d4-a716-44665544000"))
    end)

    it("rejects a UUID with wrong dash positions", function()
        assert.is_false(uuid.validate("550e8400e29b-41d4-a716-446655440000"))
    end)

    it("rejects a UUID with invalid hex characters", function()
        assert.is_false(uuid.validate("550e8400-e29b-41d4-a716-44665544000g"))
    end)

    it("rejects nil input", function()
        assert.is_false(uuid.validate(nil))
    end)

    it("rejects non-string input", function()
        assert.is_false(uuid.validate(42))
    end)

    it("rejects empty string", function()
        assert.is_false(uuid.validate(""))
    end)
end)

-- ============================================================================
-- parse
-- ============================================================================

describe("parse", function()
    it("returns nil + error for invalid UUID", function()
        local result, err = uuid.parse("not-a-uuid")
        assert.is_nil(result)
        assert.is_truthy(err)
    end)

    it("parses version 4 UUID correctly", function()
        local u = uuid.generate_v4()
        local info = uuid.parse(u)
        assert.are.equal(info.version, 4)
        assert.are.equal(info.variant, "rfc4122")
    end)

    it("parses version 5 test vector correctly", function()
        local u = uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com")
        local info = uuid.parse(u)
        assert.are.equal(info.version, 5)
        assert.are.equal(info.variant, "rfc4122")
    end)

    it("parses version 3 test vector correctly", function()
        local u = uuid.generate_v3(uuid.NAMESPACE_DNS, "www.example.com")
        local info = uuid.parse(u)
        assert.are.equal(info.version, 3)
        assert.are.equal(info.variant, "rfc4122")
    end)

    it("parses nil UUID: version 0, variant ncs", function()
        local info = uuid.parse(uuid.nil_uuid())
        assert.are.equal(info.version, 0)
        -- All zeros → byte[9] = 0x00 → top bit = 0 → NCS variant
        assert.are.equal(info.variant, "ncs")
    end)

    it("returns bytes table of length 16", function()
        local u = uuid.generate_v4()
        local info = uuid.parse(u)
        assert.are.equal(#info.bytes, 16)
    end)

    it("all bytes are in 0..255 range", function()
        local u = uuid.generate_v4()
        local info = uuid.parse(u)
        for _, b in ipairs(info.bytes) do
            assert.is_true(b >= 0 and b <= 255)
        end
    end)
end)

-- ============================================================================
-- generate_v4
-- ============================================================================

describe("generate_v4", function()
    it("produces a valid UUID format", function()
        local u = uuid.generate_v4()
        assert.is_true(is_valid_format(u))
    end)

    it("has version 4 in the correct position", function()
        local u = uuid.generate_v4()
        assert.are.equal(uuid_version(u), 4)
    end)

    it("has RFC 4122 variant bits", function()
        local u = uuid.generate_v4()
        assert.is_true(uuid_variant_ok(u))
    end)

    it("produces unique UUIDs on repeated calls", function()
        local uuids = {}
        for i = 1, 20 do
            local u = uuid.generate_v4()
            -- Ensure no duplicates
            for _, existing in ipairs(uuids) do
                assert.are_not.equal(u, existing)
            end
            uuids[i] = u
        end
    end)

    it("validates with the validate function", function()
        assert.is_true(uuid.validate(uuid.generate_v4()))
    end)
end)

-- ============================================================================
-- generate_v1
-- ============================================================================

describe("generate_v1", function()
    it("produces a valid UUID format", function()
        local u = uuid.generate_v1()
        assert.is_true(is_valid_format(u))
    end)

    it("has version 1 in the correct position", function()
        local u = uuid.generate_v1()
        assert.are.equal(uuid_version(u), 1)
    end)

    it("has RFC 4122 variant bits", function()
        local u = uuid.generate_v1()
        assert.is_true(uuid_variant_ok(u))
    end)

    it("produces unique UUIDs on repeated calls", function()
        local uuids = {}
        for i = 1, 10 do
            local u = uuid.generate_v1()
            for _, existing in ipairs(uuids) do
                assert.are_not.equal(u, existing)
            end
            uuids[i] = u
        end
    end)
end)

-- ============================================================================
-- generate_v3 — RFC 4122 test vectors (MUST match exactly)
-- ============================================================================

describe("generate_v3", function()
    it("matches RFC 4122 test vector: DNS + www.example.com", function()
        -- This is the AUTHORITATIVE test. Any other result is a bug.
        local result = uuid.generate_v3(uuid.NAMESPACE_DNS, "www.example.com")
        assert.are.equal(result, "5df41881-3aed-3515-88a7-2f4a814cf09e")
    end)

    it("is deterministic: same inputs always give same output", function()
        local u1 = uuid.generate_v3(uuid.NAMESPACE_DNS, "test.example.org")
        local u2 = uuid.generate_v3(uuid.NAMESPACE_DNS, "test.example.org")
        assert.are.equal(u1, u2)
    end)

    it("different names give different UUIDs", function()
        local u1 = uuid.generate_v3(uuid.NAMESPACE_DNS, "foo.example.com")
        local u2 = uuid.generate_v3(uuid.NAMESPACE_DNS, "bar.example.com")
        assert.are_not.equal(u1, u2)
    end)

    it("different namespaces give different UUIDs for same name", function()
        local u1 = uuid.generate_v3(uuid.NAMESPACE_DNS, "example.com")
        local u2 = uuid.generate_v3(uuid.NAMESPACE_URL, "example.com")
        assert.are_not.equal(u1, u2)
    end)

    it("has version 3 in the correct position", function()
        local u = uuid.generate_v3(uuid.NAMESPACE_DNS, "www.example.com")
        assert.are.equal(uuid_version(u), 3)
    end)

    it("has RFC 4122 variant bits", function()
        local u = uuid.generate_v3(uuid.NAMESPACE_DNS, "www.example.com")
        assert.is_true(uuid_variant_ok(u))
    end)

    it("returns nil + error for invalid namespace UUID", function()
        local result, err = uuid.generate_v3("not-a-uuid", "name")
        assert.is_nil(result)
        assert.is_truthy(err)
    end)

    it("handles empty name", function()
        -- Should not error, just produce a deterministic UUID
        local u = uuid.generate_v3(uuid.NAMESPACE_DNS, "")
        assert.is_true(is_valid_format(u))
    end)
end)

-- ============================================================================
-- generate_v5 — RFC 4122 test vectors (MUST match exactly)
-- ============================================================================

describe("generate_v5", function()
    it("matches RFC 4122 test vector: DNS + www.example.com", function()
        -- This is the AUTHORITATIVE test value, verified against Python's
        -- uuid.uuid5() reference implementation (RFC 4122 §Appendix B).
        local result = uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com")
        assert.are.equal(result, "2ed6657d-e927-568b-95e1-2665a8aea6a2")
    end)

    it("is deterministic: same inputs always give same output", function()
        local u1 = uuid.generate_v5(uuid.NAMESPACE_DNS, "test.example.org")
        local u2 = uuid.generate_v5(uuid.NAMESPACE_DNS, "test.example.org")
        assert.are.equal(u1, u2)
    end)

    it("different names give different UUIDs", function()
        local u1 = uuid.generate_v5(uuid.NAMESPACE_DNS, "foo.example.com")
        local u2 = uuid.generate_v5(uuid.NAMESPACE_DNS, "bar.example.com")
        assert.are_not.equal(u1, u2)
    end)

    it("different namespaces give different UUIDs for same name", function()
        local u1 = uuid.generate_v5(uuid.NAMESPACE_DNS, "example.com")
        local u2 = uuid.generate_v5(uuid.NAMESPACE_URL, "example.com")
        assert.are_not.equal(u1, u2)
    end)

    it("has version 5 in the correct position", function()
        local u = uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com")
        assert.are.equal(uuid_version(u), 5)
    end)

    it("has RFC 4122 variant bits", function()
        local u = uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com")
        assert.is_true(uuid_variant_ok(u))
    end)

    it("returns nil + error for invalid namespace", function()
        local result, err = uuid.generate_v5("bad-uuid", "name")
        assert.is_nil(result)
        assert.is_truthy(err)
    end)

    it("v5 differs from v3 for the same inputs", function()
        local u3 = uuid.generate_v3(uuid.NAMESPACE_DNS, "www.example.com")
        local u5 = uuid.generate_v5(uuid.NAMESPACE_DNS, "www.example.com")
        assert.are_not.equal(u3, u5)
    end)
end)

-- ============================================================================
-- generate_v7
-- ============================================================================

describe("generate_v7", function()
    it("produces a valid UUID format", function()
        local u = uuid.generate_v7()
        assert.is_true(is_valid_format(u))
    end)

    it("has version 7 in the correct position", function()
        local u = uuid.generate_v7()
        assert.are.equal(uuid_version(u), 7)
    end)

    it("has RFC 4122 variant bits", function()
        local u = uuid.generate_v7()
        assert.is_true(uuid_variant_ok(u))
    end)

    it("produces unique UUIDs on repeated calls", function()
        local uuids = {}
        for i = 1, 10 do
            local u = uuid.generate_v7()
            for _, existing in ipairs(uuids) do
                assert.are_not.equal(u, existing)
            end
            uuids[i] = u
        end
    end)

    it("validates with the validate function", function()
        assert.is_true(uuid.validate(uuid.generate_v7()))
    end)

    it("the high 12 hex characters contain a recent timestamp (sanity check)", function()
        -- The first 12 hex chars (bytes 1-6) represent the Unix ms timestamp.
        -- We check that the top 48 bits encode a time within ±1 day of now.
        local u     = uuid.generate_v7()
        local hex   = u:gsub("-", ""):sub(1, 12)
        local ms    = tonumber(hex, 16)
        local now   = os.time() * 1000
        local one_day_ms = 86400 * 1000
        assert.is_true(math.abs(ms - now) < one_day_ms)
    end)
end)

-- ============================================================================
-- Namespace constants
-- ============================================================================

describe("namespace constants", function()
    it("NAMESPACE_DNS validates correctly", function()
        assert.is_true(uuid.validate(uuid.NAMESPACE_DNS))
    end)

    it("NAMESPACE_URL validates correctly", function()
        assert.is_true(uuid.validate(uuid.NAMESPACE_URL))
    end)

    it("NAMESPACE_DNS has the correct well-known value", function()
        assert.are.equal(uuid.NAMESPACE_DNS, "6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    end)

    it("NAMESPACE_URL has the correct well-known value", function()
        assert.are.equal(uuid.NAMESPACE_URL, "6ba7b811-9dad-11d1-80b4-00c04fd430c8")
    end)
end)

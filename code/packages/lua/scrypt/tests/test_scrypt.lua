-- Tests for coding_adventures.scrypt — RFC 7914
--
-- Test coverage:
--   - RFC 7914 official test vectors (vectors 1 and 2)
--   - Input validation (N power-of-2, N range, r, p, dk_len)
--   - Output properties (length, determinism, sensitivity)
--   - Edge cases (empty password, empty salt, single-byte output)
--   - Hex output format

-- Wire up all transitive dependencies so tests can be run without luarocks install.
-- The package.path entries are relative to the tests/ directory where busted runs.
package.path =
  "../src/?.lua;"      ..
  "../src/?/init.lua;" ..
  "../../hmac/src/?.lua;"      ..
  "../../hmac/src/?/init.lua;" ..
  "../../sha256/src/?.lua;"    ..
  "../../sha256/src/?/init.lua;" ..
  "../../sha512/src/?.lua;"    ..
  "../../sha512/src/?/init.lua;" ..
  "../../md5/src/?.lua;"       ..
  "../../md5/src/?/init.lua;"  ..
  "../../sha1/src/?.lua;"      ..
  "../../sha1/src/?/init.lua;" ..
  package.path

local scrypt = require("coding_adventures.scrypt")

-- ─── RFC 7914 Test Vectors ────────────────────────────────────────────────────
-- These are the authoritative test vectors from §11 of RFC 7914.
-- Passing them guarantees we have not introduced a subtraction/rotation/XOR bug.

describe("RFC 7914 test vectors", function()

  it("vector 1 — empty password and empty salt, N=16, r=1, p=1, dkLen=64", function()
    -- RFC 7914 §11, Test Vector 1.
    -- Password: "" (empty)   Salt: ""   N=16   r=1   p=1   dkLen=64
    --
    -- This vector specifically tests the empty-password path.
    -- Standard HMAC-SHA256 public APIs reject empty keys as a security guard,
    -- so scrypt must use the raw HMAC engine internally.
    local dk = scrypt.scrypt_hex("", "", 16, 1, 1, 64)
    assert.are.equal(
      "77d6576238657b203b19ca42c18a0497" ..
      "f16b4844e3074ae8dfdffa3fede21442" ..
      "fcd0069ded0948f8326a753a0fc81f17" ..
      "e8d3e0fb2e0d3628cf35e20c38d18906",
      dk
    )
  end)

  it("vector 2 — 'password'/'NaCl', N=1024, r=8, p=16, dkLen=64", function()
    -- RFC 7914 §11, Test Vector 2.
    -- This is the primary functional correctness test for all three layers:
    -- Salsa20/8, BlockMix, and ROMix. It is significantly slower than vector 1
    -- because N=1024 and p=16 require 16 independent ROMix passes of 1024 steps.
    local dk = scrypt.scrypt_hex("password", "NaCl", 1024, 8, 16, 64)
    assert.are.equal(
      "fdbabe1c9d3472007856e7190d01e9fe" ..
      "7c6ad7cbc8237830e77376634b373162" ..
      "2eaf30d92e22a3886ff109279d9830da" ..
      "c727afb94a83ee6d8360cbdfa2cc0640",
      dk
    )
  end)

end)

-- ─── Input Validation ─────────────────────────────────────────────────────────
-- Every invalid parameter combination must raise a Lua error.

describe("input validation", function()

  it("N=3 (not a power of 2) raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 3, 1, 1, 32)
    end)
  end)

  it("N=1 (less than 2) raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 1, 1, 1, 32)
    end)
  end)

  it("N=0 raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 0, 1, 1, 32)
    end)
  end)

  it("N > 2^20 raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 2 ^ 21, 1, 1, 32)
    end)
  end)

  it("r=0 raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 16, 0, 1, 32)
    end)
  end)

  it("p=0 raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 16, 1, 0, 32)
    end)
  end)

  it("dk_len=0 raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 16, 1, 1, 0)
    end)
  end)

  it("dk_len > 2^20 raises error", function()
    assert.has_error(function()
      scrypt.scrypt("pw", "salt", 16, 1, 1, 2 ^ 20 + 1)
    end)
  end)

end)

-- ─── Output Properties ────────────────────────────────────────────────────────

describe("output properties", function()

  it("output length matches dk_len (single block)", function()
    local dk = scrypt.scrypt("password", "salt", 16, 1, 1, 32)
    assert.are.equal(32, #dk)
  end)

  it("output length matches dk_len (multi-block, dk_len=48)", function()
    -- 48 bytes spans two 32-byte PBKDF2 output blocks
    local dk = scrypt.scrypt("password", "salt", 16, 1, 1, 48)
    assert.are.equal(48, #dk)
  end)

  it("output length matches dk_len (single byte)", function()
    local dk = scrypt.scrypt("password", "salt", 16, 1, 1, 1)
    assert.are.equal(1, #dk)
  end)

  it("is deterministic — same inputs always produce same output", function()
    local a = scrypt.scrypt("secret", "nacl", 16, 1, 1, 32)
    local b = scrypt.scrypt("secret", "nacl", 16, 1, 1, 32)
    assert.are.equal(a, b)
  end)

  it("different passwords produce different keys", function()
    local a = scrypt.scrypt("password1", "salt", 16, 1, 1, 32)
    local b = scrypt.scrypt("password2", "salt", 16, 1, 1, 32)
    assert.are_not.equal(a, b)
  end)

  it("different salts produce different keys", function()
    local a = scrypt.scrypt("password", "salt1", 16, 1, 1, 32)
    local b = scrypt.scrypt("password", "salt2", 16, 1, 1, 32)
    assert.are_not.equal(a, b)
  end)

  it("different N produce different keys", function()
    local a = scrypt.scrypt("password", "salt", 16, 1, 1, 32)
    local b = scrypt.scrypt("password", "salt", 32, 1, 1, 32)
    assert.are_not.equal(a, b)
  end)

  it("prefix consistency: shorter dk_len is prefix of longer", function()
    -- scrypt is built on PBKDF2 whose output is sequential blocks.
    -- The first 32 bytes of a 64-byte output must equal the 32-byte output.
    local short = scrypt.scrypt("password", "salt", 16, 1, 1, 32)
    local full  = scrypt.scrypt("password", "salt", 16, 1, 1, 64)
    assert.are.equal(short, full:sub(1, 32))
  end)

end)

-- ─── Edge Cases ───────────────────────────────────────────────────────────────

describe("edge cases", function()

  it("empty salt is allowed", function()
    -- Only N=16 vector has empty salt in RFC; but empty salt should not error.
    local dk = scrypt.scrypt("password", "", 16, 1, 1, 32)
    assert.are.equal(32, #dk)
  end)

  it("empty password is allowed (RFC 7914 vector 1)", function()
    -- Confirms that scrypt internally handles empty passwords correctly.
    local dk = scrypt.scrypt("", "salt", 16, 1, 1, 32)
    assert.are.equal(32, #dk)
  end)

  it("binary password with null bytes is allowed", function()
    local dk = scrypt.scrypt("pass\x00word", "salt", 16, 1, 1, 32)
    assert.are.equal(32, #dk)
  end)

  it("binary salt with null bytes is allowed", function()
    local dk = scrypt.scrypt("password", "sa\x00lt", 16, 1, 1, 32)
    assert.are.equal(32, #dk)
  end)

end)

-- ─── Hex Output ───────────────────────────────────────────────────────────────

describe("scrypt_hex output", function()

  it("hex output is twice the dk_len in characters", function()
    local hex = scrypt.scrypt_hex("pw", "salt", 16, 1, 1, 32)
    assert.are.equal(64, #hex)
  end)

  it("hex output is lowercase", function()
    local hex = scrypt.scrypt_hex("pw", "salt", 16, 1, 1, 32)
    assert.is_truthy(hex:match("^[0-9a-f]+$"))
  end)

  it("hex output matches manual encoding of raw output", function()
    local raw = scrypt.scrypt("pw", "salt", 16, 1, 1, 32)
    local hex = scrypt.scrypt_hex("pw", "salt", 16, 1, 1, 32)
    local expected = raw:gsub(".", function(c)
      return string.format("%02x", string.byte(c))
    end)
    assert.are.equal(expected, hex)
  end)

end)

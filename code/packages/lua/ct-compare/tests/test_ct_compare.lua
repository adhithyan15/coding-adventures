-- Tests for coding_adventures.ct_compare (CT01).
--
-- Part 1 replays every case of the shared CT01 corpus
-- (code/specs/fixtures/ct-compare-v1/cases.json).  Part 2 adds exhaustive
-- checks the corpus only samples: every byte position, all 256 select byte
-- values, length mismatches, all 64 single-bit flips, and input validation.

package.path = "./?.lua;../src/?.lua;../src/?/init.lua;" .. package.path

local ct = require("coding_adventures.ct_compare")
local corpus = require("corpus")

local function bytes(...) return string.char(...) end

-- ============================================================================
-- Shared CT01 corpus
-- ============================================================================

describe("CT01 corpus", function()
  local doc = corpus.load()

  it("is the ct-compare-v1 profile with 21 cases", function()
    assert.are.equal("ct-compare-v1", doc.profile)
    assert.are.equal(21, #doc.cases)
  end)

  for _, case in ipairs(doc.cases) do
    it(case.id, function()
      local op = case.operation
      if op == "eq" or op == "eq_fixed" then
        local l = corpus.hex_to_bytes(case.left_hex)
        local r = corpus.hex_to_bytes(case.right_hex)
        local got, bit
        if op == "eq" then
          got, bit = ct.ct_eq(l, r), ct.ct_eq_bit(l, r)
        else
          assert.are.equal(case.fixed_size_bytes, #l)
          got, bit = ct.ct_eq_fixed(l, r), ct.ct_eq_fixed_bit(l, r)
        end
        assert.are.equal(case.expected.equal, got)
        -- corpus values are public, so this boolean-to-bit mapping is fine
        assert.are.equal(case.expected.equal and 1 or 0, bit)
      elseif op == "select_bytes" then
        local l = corpus.hex_to_bytes(case.left_hex)
        local r = corpus.hex_to_bytes(case.right_hex)
        local choice = case.choice and 1 or 0
        if case.expected.error_id then
          assert.are.equal("CT_SELECT_LENGTH_MISMATCH", case.expected.error_id)
          local ok, err = pcall(ct.ct_select_bytes, l, r, choice)
          assert.is_false(ok)
          assert.is_truthy(tostring(err):find("CT_SELECT_LENGTH_MISMATCH", 1, true))
        else
          local out = ct.ct_select_bytes(l, r, choice)
          assert.are.equal(case.expected.output_hex, corpus.bytes_to_hex(out))
          -- inputs unchanged
          assert.are.equal(case.left_hex, corpus.bytes_to_hex(l))
          assert.are.equal(case.right_hex, corpus.bytes_to_hex(r))
        end
      elseif op == "eq_u64" then
        local l = corpus.hex_to_u64(case.left_u64)
        local r = corpus.hex_to_u64(case.right_u64)
        assert.are.equal(case.expected.equal, ct.ct_eq_u64(l, r))
        assert.are.equal(case.expected.equal and 1 or 0, ct.ct_eq_u64_bit(l, r))
      else
        error("unknown corpus operation " .. tostring(op))
      end
    end)
  end
end)

-- ============================================================================
-- Equality beyond the corpus
-- ============================================================================

describe("ct_eq", function()
  it("accepts equal and empty strings", function()
    assert.is_true(ct.ct_eq("", ""))
    assert.is_true(ct.ct_eq("secret", "secret"))
  end)

  it("detects a mismatch at every byte position of a 64-byte string", function()
    local base = {}
    for i = 1, 64 do base[i] = (i * 7) % 256 end
    local a = string.char(table.unpack(base))
    for pos = 1, 64 do
      local copy = { table.unpack(base) }
      copy[pos] = copy[pos] ~ 0x01
      local b = string.char(table.unpack(copy))
      assert.is_false(ct.ct_eq(a, b), "position " .. pos)
      assert.are.equal(0, ct.ct_eq_bit(a, b))
      assert.is_false(ct.ct_eq_fixed(a, b))
    end
  end)

  it("detects every single-bit difference in every byte value", function()
    for v = 0, 255 do
      for bit = 0, 7 do
        assert.is_false(ct.ct_eq(bytes(v), bytes(v ~ (1 << bit))))
      end
      assert.is_true(ct.ct_eq(bytes(v), bytes(v)))
    end
  end)

  it("treats a length mismatch as false in both directions", function()
    assert.is_false(ct.ct_eq("a", "ab"))
    assert.is_false(ct.ct_eq("ab", "a"))
    assert.is_false(ct.ct_eq("", "a"))
    assert.are.equal(0, ct.ct_eq_bit("", "a"))
    assert.is_false(ct.ct_eq_fixed("abc", "abcd"))
  end)

  it("returns a bit that is exactly 0 or 1", function()
    assert.are.equal(1, ct.ct_eq_bit("x", "x"))
    assert.are.equal(1, ct.ct_eq_fixed_bit("", ""))
    assert.are.equal(0, ct.ct_eq_bit("x", "y"))
    assert.are.equal(0, ct.ct_eq_fixed_bit("x", "y"))
  end)

  it("treats embedded NUL bytes as data", function()
    assert.is_false(ct.ct_eq("a\0b", "a\0c"))
    assert.is_true(ct.ct_eq("a\0b", "a\0b"))
  end)

  it("accepts arrays of byte values", function()
    assert.is_true(ct.ct_eq({ 1, 2, 3 }, "\1\2\3"))
    assert.is_true(ct.ct_eq({}, ""))
    assert.is_false(ct.ct_eq({ 1, 2, 3 }, { 1, 2, 4 }))
  end)

  it("rejects malformed byte inputs instead of coercing them", function()
    assert.has_error(function() ct.ct_eq({ 256 }, "\0") end)
    assert.has_error(function() ct.ct_eq({ -1 }, "\255") end)
    assert.has_error(function() ct.ct_eq({ 97.0 }, "a") end)
    assert.has_error(function() ct.ct_eq({ 97.9 }, "a") end)
    assert.has_error(function() ct.ct_eq({ "a" }, "a") end)
    assert.has_error(function() ct.ct_eq({ true }, "\1") end)
    assert.has_error(function() ct.ct_eq(nil, "") end)
    assert.has_error(function() ct.ct_eq(3, "\0\0\0") end)
    assert.has_error(function() ct.ct_eq("", false) end)
    assert.has_error(function() ct.ct_eq(setmetatable({}, {}), "") end)
  end)
end)

-- ============================================================================
-- Selection
-- ============================================================================

describe("ct_select_bytes", function()
  it("picks left for 1 and right for 0", function()
    assert.are.equal("AB", ct.ct_select_bytes("AB", "CD", 1))
    assert.are.equal("CD", ct.ct_select_bytes("AB", "CD", 0))
  end)

  it("is correct for all 256 byte values against all 256 byte values", function()
    -- Every (left, right) byte pair, both choices: 2 * 65536 outcomes,
    -- batched as 256-byte strings so the test stays fast.
    for lv = 0, 255 do
      local left = string.rep(string.char(lv), 256)
      local rights = {}
      for rv = 0, 255 do rights[rv + 1] = string.char(rv) end
      local right = table.concat(rights)
      assert.are.equal(left, ct.ct_select_bytes(left, right, 1))
      assert.are.equal(right, ct.ct_select_bytes(left, right, 0))
    end
  end)

  it("covers every byte value in one pass", function()
    local all = {}
    for v = 0, 255 do all[v + 1] = string.char(v) end
    local ascending = table.concat(all)
    local descending = ascending:reverse()
    assert.are.equal(ascending, ct.ct_select_bytes(ascending, descending, 1))
    assert.are.equal(descending, ct.ct_select_bytes(ascending, descending, 0))
  end)

  it("handles empty inputs", function()
    assert.are.equal("", ct.ct_select_bytes("", "", 1))
    assert.are.equal("", ct.ct_select_bytes("", "", 0))
  end)

  it("raises CT_SELECT_LENGTH_MISMATCH for unequal lengths", function()
    for _, args in ipairs({ { "a", "ab" }, { "ab", "a" }, { "", "a" } }) do
      local ok, err = pcall(ct.ct_select_bytes, args[1], args[2], 1)
      assert.is_false(ok)
      assert.is_truthy(tostring(err):find("CT_SELECT_LENGTH_MISMATCH", 1, true))
    end
  end)

  it("rejects every choice other than the integers 0 and 1", function()
    local bad = { 2, -1, 3, 255, 256, math.mininteger, math.maxinteger,
                  1.0, 0.0, true, false, "1", "0", {}, print }
    for _, choice in ipairs(bad) do
      assert.has_error(function() ct.ct_select_bytes("a", "b", choice) end,
        nil, "choice " .. tostring(choice))
    end
    assert.has_error(function() ct.ct_select_bytes("a", "b", nil) end)
  end)

  it("validates choice before reading the inputs' contents", function()
    -- A bad choice with a bad input still reports an error; neither succeeds.
    assert.has_error(function() ct.ct_select_bytes({ 256 }, "a", 5) end)
  end)

  it("composes with ct_eq_bit without any boolean conversion", function()
    local pick = ct.ct_select_bytes("yes", "no!", ct.ct_eq_bit("tag", "tag"))
    assert.are.equal("yes", pick)
    pick = ct.ct_select_bytes("yes", "no!", ct.ct_eq_bit("tag", "tab"))
    assert.are.equal("no!", pick)
    pick = ct.ct_select_bytes("yes", "no!", ct.ct_eq_u64_bit(5, 6))
    assert.are.equal("no!", pick)
  end)

  it("accepts byte arrays and returns a string", function()
    assert.are.equal("\1\2", ct.ct_select_bytes({ 1, 2 }, { 3, 4 }, 1))
    assert.are.equal("\3\4", ct.ct_select_bytes({ 1, 2 }, { 3, 4 }, 0))
  end)

  it("does not mutate array inputs", function()
    local l, r = { 1, 2 }, { 3, 4 }
    ct.ct_select_bytes(l, r, 1)
    assert.are.same({ 1, 2 }, l)
    assert.are.same({ 3, 4 }, r)
  end)
end)

-- ============================================================================
-- Unsigned 64-bit equality
-- ============================================================================

describe("ct_eq_u64", function()
  it("is true for equal values including 0, max and sign bit", function()
    for _, v in ipairs({ 0, 1, -1, math.mininteger, math.maxinteger, 0x0123456789ABCDEF }) do
      assert.is_true(ct.ct_eq_u64(v, v))
      assert.are.equal(1, ct.ct_eq_u64_bit(v, v))
    end
  end)

  it("detects each of the 64 single-bit flips from several bases", function()
    local bases = { 0, -1, 0x5555555555555555, 0x0123456789ABCDEF, math.mininteger }
    for _, base in ipairs(bases) do
      for bit = 0, 63 do
        local flipped = base ~ (1 << bit)
        assert.is_false(ct.ct_eq_u64(base, flipped), "bit " .. bit)
        assert.is_false(ct.ct_eq_u64(flipped, base))
        assert.are.equal(0, ct.ct_eq_u64_bit(base, flipped))
      end
    end
  end)

  it("distinguishes 0 from the sign bit and from the maximum", function()
    assert.is_false(ct.ct_eq_u64(0, math.mininteger))
    assert.is_false(ct.ct_eq_u64(0, -1))
    assert.is_false(ct.ct_eq_u64(math.maxinteger, -1))
  end)

  it("rejects non-integers", function()
    assert.has_error(function() ct.ct_eq_u64(1.0, 1) end)
    assert.has_error(function() ct.ct_eq_u64(1, 1.5) end)
    assert.has_error(function() ct.ct_eq_u64("1", 1) end)
    assert.has_error(function() ct.ct_eq_u64(1, nil) end)
    assert.has_error(function() ct.ct_eq_u64(true, true) end)
  end)
end)

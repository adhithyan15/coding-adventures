-- ============================================================================
-- Tests for coding_adventures.canonical_cbor (CBR01).
--
-- Knuth-style literate test suite covering:
-- 1. Value model, constructors, type safety, and defensive immutability.
-- 2. Error diagnostics and clean code (all 14 closed error IDs).
-- 3. Convenience wrappers and atomic rollback.
-- 4. Shared CBR01 conformance corpus (all 55 cases from cases.json).
-- 5. Non-minimal integer header widening across major types 0..6.
-- 6. DecodeMapSpans accuracy, 0-based offset verification, and CBR01 invariant.
-- 7. 7-class Lua CVE Resilience Showcase:
--    CVE-1: Lua 5.4 64-bit 0xFFFFFFFFFFFFFFFF == -1 signed-comparison bypass.
--    CVE-2: Pre-allocation OOM bomb defense.
--    CVE-3: Deep nesting recursion exhaustion (depth 128 vs 129).
--    CVE-4: Canonical length-first map key ordering & duplicate key defense.
--    CVE-5: UTF-8 overlong and surrogate smuggling defense.
--    CVE-6: Trailing-byte smuggling and metatable TOCTOU spoofing.
--    CVE-7: Secret payload reflection leak defense & SpannedMapEntry redaction.
-- 8. Deterministic PRNG round-trip and map-permutation property tests.
-- 9. Natural encode-too-large buffer overflow trigger.
-- 10. 10,000-iteration adversarial byte-mutation fuzzing loop.
-- ============================================================================

package.path = "./?.lua;../src/?.lua;../src/?/init.lua;" .. package.path

local cbor = require("coding_adventures.canonical_cbor")
local constants = cbor.constants
local messages = cbor.messages
local types = require("coding_adventures.canonical_cbor.types")
local codec = require("coding_adventures.canonical_cbor.codec")
local corpus = require("corpus")

local hex_to_bytes = corpus.hex_to_bytes
local bytes_to_hex = corpus.bytes_to_hex

local function build_nested_array(n)
  local cur = cbor.CborArray({})
  for _ = 1, n do
    cur = cbor.CborArray({ cur })
  end
  return cur
end

local function build_nested_array_wire(n)
  return string.rep("\x81", n) .. "\x80"
end

local function make_prng(seed)
  local state = seed
  local function next_u32()
    state = (state * 1664525 + 1013904223) & 0xFFFFFFFF
    return state
  end
  local function next_int(max)
    return next_u32() % max
  end
  local function next_u64()
    local hi = next_u32()
    local lo = next_u32()
    return (hi << 32) | lo
  end
  local function next_bytes(len)
    local t = {}
    for i = 1, len do
      t[i] = string.char(next_int(256))
    end
    return table.concat(t)
  end
  return {
    next_u32 = next_u32,
    next_int = next_int,
    next_u64 = next_u64,
    next_bytes = next_bytes,
  }
end

describe("coding_adventures.canonical_cbor", function()

  -- =========================================================================
  -- 1. Value Model, Constructors, Type Safety, and Immutability
  -- =========================================================================

  describe("Value model and immutability", function()
    it("handles CborUnsigned constructor, equality, and immutability", function()
      local u = cbor.CborUnsigned(42)
      assert.are.equal("unsigned", u.kind)
      assert.are.equal(42, u.value)
      assert.are.equal("CborUnsigned(42)", tostring(u))

      local u2 = cbor.CborUnsigned(42)
      assert.are.equal(u, u2)
      assert.are_not.equal(u, cbor.CborUnsigned(43))

      -- Read-only metatable:
      assert.is_false(getmetatable(u))
      local ok = pcall(function() u.value = 99 end)
      assert.is_false(ok)

      -- Reject non-integers and floats:
      assert.has_error(function() cbor.CborUnsigned(1.0) end)
      assert.has_error(function() cbor.CborUnsigned("42") end)
      assert.has_error(function() cbor.CborUnsigned(nil) end)
    end)

    it("handles CborNegative constructor, equality, and immutability", function()
      local n = cbor.CborNegative(41) -- represents -42
      assert.are.equal("negative", n.kind)
      assert.are.equal(41, n.value)
      assert.are.equal("CborNegative(41)", tostring(n))

      local n2 = cbor.CborNegative(41)
      assert.are.equal(n, n2)
      assert.are_not.equal(n, cbor.CborNegative(42))

      assert.is_false(getmetatable(n))
      assert.has_error(function() n.value = 0 end)
      assert.has_error(function() cbor.CborNegative(1.5) end)
      assert.has_error(function() cbor.CborNegative(nil) end)
    end)

    it("handles CborBytes constructor, equality, and immutability", function()
      local b = cbor.CborBytes("\x01\x02\x03")
      assert.are.equal("bytes", b.kind)
      assert.are.equal("\x01\x02\x03", b.bytes)
      assert.are.equal("CborBytes(#3)", tostring(b))

      -- From integer array:
      local b_arr = cbor.CborBytes({ 1, 2, 3 })
      assert.are.equal(b, b_arr)

      assert.is_false(getmetatable(b))
      assert.has_error(function() b.bytes = "other" end)
      assert.has_error(function() cbor.CborBytes(123) end)
      assert.has_error(function() cbor.CborBytes({ -1 }) end)
      assert.has_error(function() cbor.CborBytes({ 256 }) end)
    end)

    it("handles CborText constructor, strict UTF-8, and immutability", function()
      local t = cbor.CborText("hello")
      assert.are.equal("text", t.kind)
      assert.are.equal("hello", t.text)
      assert.are.equal(t, cbor.CborText("hello"))
      assert.are_not.equal(t, cbor.CborText("world"))

      assert.is_false(getmetatable(t))
      assert.has_error(function() t.text = "bye" end)
      assert.has_error(function() cbor.CborText(123) end)
      -- Invalid UTF-8:
      assert.has_error(function() cbor.CborText("\xFF") end)
      assert.has_error(function() cbor.CborText("\xED\xA0\x80") end)
    end)

    it("handles CborArray constructor, equality, and immutability", function()
      local arr = cbor.CborArray({ cbor.CborUnsigned(1), cbor.CborBool(true) })
      assert.are.equal("array", arr.kind)
      assert.are.equal(2, #arr.items)
      assert.are.equal("CborArray(#2)", tostring(arr))

      local arr2 = cbor.CborArray({ cbor.CborUnsigned(1), cbor.CborBool(true) })
      assert.are.equal(arr, arr2)
      assert.are_not.equal(arr, cbor.CborArray({ cbor.CborUnsigned(1) }))

      assert.is_false(getmetatable(arr))
      assert.has_error(function() arr.items = {} end)
      assert.has_error(function() arr.items[1] = cbor.CborNull() end)

      -- Reject non-CBOR items:
      assert.has_error(function() cbor.CborArray({ 123 }) end)
      assert.has_error(function() cbor.CborArray({ "raw string" }) end)
      assert.has_error(function() cbor.CborArray("not a table") end)
    end)

    it("handles CborMapEntry and CborMap constructor, equality, and immutability", function()
      local entry = cbor.CborMapEntry(cbor.CborText("k"), cbor.CborUnsigned(1))
      assert.are.equal("map_entry", entry.kind)
      assert.are.equal(cbor.CborText("k"), entry.key)
      assert.are.equal(cbor.CborUnsigned(1), entry.value)
      assert.are.equal(entry, cbor.CborMapEntry(cbor.CborText("k"), cbor.CborUnsigned(1)))

      local map_val = cbor.CborMap({ entry })
      assert.are.equal("map", map_val.kind)
      assert.are.equal(1, #map_val.entries)
      assert.are.equal("CborMap(#1)", tostring(map_val))

      assert.is_false(getmetatable(entry))
      assert.is_false(getmetatable(map_val))
      assert.has_error(function() entry.key = cbor.CborNull() end)
      assert.has_error(function() map_val.entries = {} end)

      -- Reject invalid items:
      assert.has_error(function() cbor.CborMapEntry("raw", cbor.CborNull()) end)
      assert.has_error(function() cbor.CborMapEntry(cbor.CborNull(), "raw") end)
      assert.has_error(function() cbor.CborMap({ "not an entry" }) end)
    end)

    it("handles CborTag, CborBool, and CborNull constructors and immutability", function()
      local tag = cbor.CborTag(24, cbor.CborUnsigned(100))
      assert.are.equal("tag", tag.kind)
      assert.are.equal(24, tag.tag)
      assert.are.equal(cbor.CborUnsigned(100), tag.value)
      assert.are.equal("CborTag(24, CborUnsigned(100))", tostring(tag))
      assert.are.equal(tag, cbor.CborTag(24, cbor.CborUnsigned(100)))

      local b_t = cbor.CborBool(true)
      local b_f = cbor.CborBool(false)
      assert.are.equal("boolean", b_t.kind)
      assert.is_true(b_t.value)
      assert.is_false(b_f.value)
      assert.are_not.equal(b_t, b_f)

      local null_val = cbor.CborNull()
      assert.are.equal("null", null_val.kind)
      assert.are.equal("CborNull", tostring(null_val))
      assert.are.equal(null_val, cbor.CborNull())

      assert.has_error(function() cbor.CborTag(24.5, cbor.CborNull()) end)
      assert.has_error(function() cbor.CborTag(24, "raw") end)
      assert.has_error(function() cbor.CborBool("true") end)
    end)

    it("handles Span and SpannedMapEntry constructors, redaction, and immutability", function()
      local span = cbor.Span(10, 25)
      assert.are.equal(10, span.start)
      assert.are.equal(25, span.finish)
      assert.are.equal(15, span:length())
      assert.are.equal("[10, 25)", tostring(span))
      assert.are.equal(span, cbor.Span(10, 25))

      local spanned = cbor.SpannedMapEntry(cbor.CborText("secret_key"), cbor.CborText("secret_val"), span)
      assert.are.equal(cbor.CborText("secret_key"), spanned.key)
      assert.are.equal(cbor.CborText("secret_val"), spanned.value)
      assert.are.equal(span, spanned.value_span)
      assert.are.equal(spanned, cbor.SpannedMapEntry(cbor.CborText("secret_key"), cbor.CborText("secret_val"), span))

      -- Redaction check:
      local rep = tostring(spanned)
      assert.is_nil(rep:find("secret_key", 1, true))
      assert.is_nil(rep:find("secret_val", 1, true))
      assert.is_truthy(rep:find("<redacted>", 1, true))

      assert.is_false(getmetatable(span))
      assert.is_false(getmetatable(spanned))
      assert.has_error(function() cbor.Span(-1, 5) end)
      assert.has_error(function() cbor.Span(5, 3) end)
    end)

    it("handles DecodeMapSpansResult constructors and immutability", function()
      local res_non_map = cbor.DecodeMapSpansResult.non_map()
      assert.is_false(res_non_map:is_map())
      assert.is_nil(res_non_map.entries)
      assert.are.equal("DecodeMapSpansResult.non_map()", tostring(res_non_map))

      local entry = cbor.SpannedMapEntry(cbor.CborUnsigned(1), cbor.CborBool(true), cbor.Span(0, 1))
      local res_map = cbor.DecodeMapSpansResult.map({ entry })
      assert.is_true(res_map:is_map())
      assert.are.equal(1, #res_map.entries)
      assert.are.equal(entry, res_map.entries[1])
      assert.are.equal("DecodeMapSpansResult.map(#1)", tostring(res_map))

      assert.is_false(getmetatable(res_map))
      assert.is_false(getmetatable(res_non_map))
      assert.has_error(function() res_map.entries = nil end)
      assert.has_error(function() cbor.DecodeMapSpansResult.map({ "not spanned" }) end)
    end)
  end)

  -- =========================================================================
  -- 2. Error Diagnostics & Clean Code (All 14 Closed Error IDs)
  -- =========================================================================

  describe("Error diagnostics and clean code", function()
    it("provides all 14 closed payload-blind error diagnostics", function()
      local expected_ids = {
        "unexpected-eof",
        "trailing-bytes",
        "reserved",
        "indefinite",
        "non-minimal-integer",
        "invalid-utf8",
        "non-canonical-map-order",
        "unsupported-simple",
        "float-not-supported",
        "too-deep",
        "length-too-large",
        "duplicate-map-key",
        "encode-too-deep",
        "encode-too-large",
      }

      assert.are.equal("<redacted>", messages.REDACTED_PLACEHOLDER)

      for _, id in ipairs(expected_ids) do
        assert.is_true(messages.is_valid_id(id))
        local msg = messages.message_for(id)
        assert.is_truthy(msg:find("^canonical%-cbor: "))

        local err = cbor.CborError(id)
        assert.are.equal(id, err.id)
        assert.are.equal(msg, err.message)
        assert.are.equal(msg, tostring(err))

        local err_same = cbor.CborError(id)
        assert.are.equal(err, err_same)
      end

      -- Unknown ID error:
      assert.has_error(function() messages.message_for("unknown-id") end)
    end)
  end)

  -- =========================================================================
  -- 3. Convenience Wrappers & Atomic Rollback
  -- =========================================================================

  describe("Convenience wrappers and atomic rollback", function()
    it("ensures encode and encode_into match checked counterparts", function()
      local val = cbor.CborUnsigned(123)
      local enc1 = cbor.encode_checked(val)
      local enc2 = cbor.encode(val)
      assert.are.equal(enc1, enc2)

      local dst1 = {}
      cbor.encode_into_checked(val, dst1)
      assert.are.equal(1, #dst1)
      assert.are.equal(enc1, dst1[1])

      local dst2 = {}
      cbor.encode_into(val, dst2)
      assert.are.equal(1, #dst2)
      assert.are.equal(enc1, dst2[1])

      -- Reject non-table or metatable dst:
      assert.has_error(function() cbor.encode_into_checked(val, "not table") end)
      local spoofed = setmetatable({}, { __newindex = function() end })
      assert.has_error(function() cbor.encode_into_checked(val, spoofed) end)
    end)

    it("guarantees atomic rollback of dst_table on encoding failure", function()
      local dst = { "initial_payload" }
      local bad_map = cbor.CborMap({
        cbor.CborMapEntry(cbor.CborText("key"), cbor.CborUnsigned(1)),
        cbor.CborMapEntry(cbor.CborText("key"), cbor.CborUnsigned(2)),
      })

      local ok, err = pcall(function()
        cbor.encode_into_checked(bad_map, dst)
      end)
      assert.is_false(ok)
      assert.are.equal("duplicate-map-key", err.id)
      assert.are.equal(1, #dst)
      assert.are.equal("initial_payload", dst[1])
    end)
  end)

  -- =========================================================================
  -- 4. Shared CBR01 Conformance Corpus (cases.json)
  -- =========================================================================

  describe("CBR01 Conformance Corpus", function()
    local doc = corpus.load_corpus()

    it("verifies fixture profile and case count", function()
      assert.are.equal("rfc8949-section-4.2.3-length-first", doc.profile)
      assert.are.equal(55, #doc.cases)
    end)

    for _, cs in ipairs(doc.cases) do
      it(cs.id .. " (" .. cs.operation .. ")", function()
        local op = cs.operation
        local input = cs.input
        local expected = cs.expected

        if op == "round-trip" then
          local in_bytes = hex_to_bytes(input)
          local val = cbor.decode(in_bytes)
          local out_bytes = cbor.encode_checked(val)
          assert.are.equal(expected, bytes_to_hex(out_bytes))
          local val2 = cbor.decode(out_bytes)
          assert.are.equal(val, val2)

        elseif op == "encode-map" then
          local entries = {}
          for pair in input:gmatch("[^;]+") do
            local k_hex, v_hex = pair:match("^([^=]+)=([^=]+)$")
            local k = cbor.decode(hex_to_bytes(k_hex))
            local v = cbor.decode(hex_to_bytes(v_hex))
            entries[#entries + 1] = cbor.CborMapEntry(k, v)
          end
          local map_val = cbor.CborMap(entries)
          local out_bytes = cbor.encode_checked(map_val)
          assert.are.equal(expected, bytes_to_hex(out_bytes))

        elseif op == "generated-round-trip" then
          if input == "nested-array:128" then
            local val = build_nested_array(128)
            local wire = build_nested_array_wire(128)
            local enc = cbor.encode_checked(val)
            assert.are.equal(wire, enc)
            local dec = cbor.decode(wire)
            assert.are.equal(val, dec)
          elseif input == "bytes-repeat:1048571:00" then
            local payload = string.rep("\x00", 1048571)
            local val = cbor.CborBytes(payload)
            local enc = cbor.encode_checked(val)
            assert.are.equal(1048576, #enc)
            local dec = cbor.decode(enc)
            assert.are.equal(val, dec)
          else
            error("unknown generated-round-trip input: " .. tostring(input), 0)
          end

        elseif op == "decode-error" then
          local wire
          if input == "nested-array-wire:129" then
            wire = build_nested_array_wire(129)
          else
            wire = hex_to_bytes(input)
          end
          local ok, err = pcall(function()
            cbor.decode(wire)
          end)
          assert.is_false(ok)
          assert.are.equal(expected, err.id)
          assert.is_truthy(err.message:find("^canonical%-cbor: "))

        elseif op == "encode-error" then
          local val
          if input == "duplicate-map-key" then
            val = cbor.CborMap({
              cbor.CborMapEntry(cbor.CborText("dup"), cbor.CborUnsigned(1)),
              cbor.CborMapEntry(cbor.CborText("dup"), cbor.CborUnsigned(2)),
            })
          elseif input == "nested-array:129" then
            val = build_nested_array(129)
          elseif input == "bytes-repeat:1048572:00" then
            val = cbor.CborBytes(string.rep("\x00", 1048572))
          else
            error("unknown encode-error input: " .. tostring(input), 0)
          end
          local ok, err = pcall(function()
            cbor.encode_checked(val)
          end)
          assert.is_false(ok)
          assert.are.equal(expected, err.id)
          assert.is_truthy(err.message:find("^canonical%-cbor: "))

        else
          error("unknown operation: " .. tostring(op), 0)
        end
      end)
    end
  end)

  -- =========================================================================
  -- 5. Non-Minimal Header Widening Rejection Across Major Types
  -- =========================================================================

  describe("Non-minimal header widening rejection", function()
    it("rejects 1-byte, 2-byte, 4-byte, and 8-byte non-minimal encodings across major types 0..6", function()
      for major = 0, 6 do
        local major_shift = major << 5

        -- 1-byte widening for value 23 (info code 24):
        local payload_23 = ""
        if major == 2 or major == 3 then
          payload_23 = string.rep("\x00", 23)
        elseif major == 4 then
          payload_23 = string.rep("\xF6", 23)
        elseif major == 5 then
          payload_23 = string.rep("\xF6", 46)
        elseif major == 6 then
          payload_23 = "\x00"
        end
        local wire_1 = string.char(major_shift | 24, 23) .. payload_23
        local ok1, err1 = pcall(cbor.decode, wire_1)
        assert.is_false(ok1)
        assert.are.equal("non-minimal-integer", err1.id)

        -- 2-byte widening for value 255 (info code 25):
        local payload_255 = ""
        if major == 2 or major == 3 then
          payload_255 = string.rep("\x00", 255)
        elseif major == 4 then
          payload_255 = string.rep("\xF6", 255)
        elseif major == 5 then
          payload_255 = string.rep("\xF6", 510)
        elseif major == 6 then
          payload_255 = "\x00"
        end
        local wire_2 = string.char(major_shift | 25, 0x00, 0xFF) .. payload_255
        local ok2, err2 = pcall(cbor.decode, wire_2)
        assert.is_false(ok2)
        assert.are.equal("non-minimal-integer", err2.id)

        -- 4-byte widening for value 65535 (info code 26):
        local wire_4 = string.char(major_shift | 26, 0x00, 0x00, 0xFF, 0xFF)
        local ok4, err4 = pcall(cbor.decode, wire_4)
        assert.is_false(ok4)
        -- Can be non-minimal-integer or unexpected-eof (due to omitted payload), but minimal check runs first!
        assert.are.equal("non-minimal-integer", err4.id)

        -- 8-byte widening for value 4294967295 (info code 27):
        local wire_8 = string.char(major_shift | 27, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF)
        local ok8, err8 = pcall(cbor.decode, wire_8)
        assert.is_false(ok8)
        assert.are.equal("non-minimal-integer", err8.id)
      end
    end)
  end)

  -- =========================================================================
  -- 6. DecodeMapSpans Accuracy & Pass-Through Invariant
  -- =========================================================================

  describe("decode_map_spans", function()
    it("extracts exact 0-based spans matching sub-string slices", function()
      -- wire: a2 61 61 01 61 62 82 03 04 -> {"a": 1, "b": [3, 4]}
      -- idx: 1  2  3  4  5  6  7  8  9
      -- 'a' wire: 61 61 (indices 2,3). value 1: 01 (index 4). 0-based span: [3, 4)
      -- 'b' wire: 61 62 (indices 5,6). value [3, 4]: 82 03 04 (indices 7,8,9). 0-based span: [6, 9)
      local wire = hex_to_bytes("a26161016162820304")
      local res = cbor.decode_map_spans(wire)
      assert.is_true(res:is_map())
      local entries = res.entries
      assert.are.equal(2, #entries)

      local e1 = entries[1]
      assert.are.equal(cbor.CborText("a"), e1.key)
      assert.are.equal(cbor.CborUnsigned(1), e1.value)
      assert.are.equal(3, e1.value_span.start)
      assert.are.equal(4, e1.value_span.finish)
      local slice1 = string.sub(wire, e1.value_span.start + 1, e1.value_span.finish)
      assert.are.equal(slice1, cbor.encode_checked(e1.value))

      local e2 = entries[2]
      assert.are.equal(cbor.CborText("b"), e2.key)
      assert.are.equal(cbor.CborArray({ cbor.CborUnsigned(3), cbor.CborUnsigned(4) }), e2.value)
      assert.are.equal(6, e2.value_span.start)
      assert.are.equal(9, e2.value_span.finish)
      local slice2 = string.sub(wire, e2.value_span.start + 1, e2.value_span.finish)
      assert.are.equal(slice2, cbor.encode_checked(e2.value))
    end)

    it("handles non-map root items cleanly", function()
      local res1 = cbor.decode_map_spans(hex_to_bytes("01"))
      assert.is_false(res1:is_map())

      local res2 = cbor.decode_map_spans(hex_to_bytes("820102"))
      assert.is_false(res2:is_map())
    end)

    it("rejects invalid wire inputs and headers in decode_map_spans", function()
      -- Empty input:
      local ok_e, err_e = pcall(cbor.decode_map_spans, "")
      assert.is_false(ok_e)
      assert.are.equal("unexpected-eof", err_e.id)

      -- Non-canonical map key order:
      local ok_o, err_o = pcall(cbor.decode_map_spans, hex_to_bytes("a2616202616101"))
      assert.is_false(ok_o)
      assert.are.equal("non-canonical-map-order", err_o.id)

      -- Trailing bytes:
      local ok_t, err_t = pcall(cbor.decode_map_spans, hex_to_bytes("a1616101ff"))
      assert.is_false(ok_t)
      assert.are.equal("trailing-bytes", err_t.id)

      -- Indefinite map:
      local ok_i, err_i = pcall(cbor.decode_map_spans, hex_to_bytes("bf616101ff"))
      assert.is_false(ok_i)
      assert.are.equal("indefinite", err_i.id)

      -- Non-minimal integer header:
      local ok_m, err_m = pcall(cbor.decode_map_spans, hex_to_bytes("b800"))
      assert.is_false(ok_m)
      assert.are.equal("non-minimal-integer", err_m.id)

      -- Reserved:
      local ok_r, err_r = pcall(cbor.decode_map_spans, hex_to_bytes("bc"))
      assert.is_false(ok_r)
      assert.are.equal("reserved", err_r.id)
    end)

    it("verifies CBR01 pass-through invariant for large map value slices", function()
      local big_bytes = string.rep("\x55", 1048580)
      local val_wire = string.char(0x5A, 0x00, 0x10, 0x00, 0x04) .. big_bytes
      local key_wire = "\x61\x61" -- text "a"
      local full_wire = "\xA1" .. key_wire .. val_wire

      local res = cbor.decode_map_spans(full_wire)
      assert.is_true(res:is_map())
      assert.are.equal(1, #res.entries)
      local span = res.entries[1].value_span
      assert.are.equal(#full_wire - #val_wire, span.start)
      assert.are.equal(#full_wire, span.finish)
    end)
  end)

  -- =========================================================================
  -- 7. 7-Class Lua CVE Resilience Showcase
  -- =========================================================================

  describe("7-Class Lua CVE Resilience Showcase", function()
    it("CVE-1: prevents 0xFFFFFFFFFFFFFFFF == -1 signed-comparison bypass", function()
      -- In Lua 5.4, 0xFFFFFFFFFFFFFFFF wraps to -1.
      -- If signed '<=' was used on declared lengths, -1 <= remaining would be true!
      local overflow_wires = {
        hex_to_bytes("5bffffffffffffffff"), -- Major 2 (bytes) length 2^64-1
        hex_to_bytes("7bffffffffffffffff"), -- Major 3 (text) length 2^64-1
        hex_to_bytes("9bffffffffffffffff"), -- Major 4 (array) count 2^64-1
        hex_to_bytes("bbffffffffffffffff"), -- Major 5 (map) count 2^64-1
        hex_to_bytes("ba40000001"),          -- Major 5 count > max / 2
      }
      for _, vec in ipairs(overflow_wires) do
        local ok, err = pcall(cbor.decode, vec)
        assert.is_false(ok)
        assert.are.equal("length-too-large", err.id)
      end

      -- Unsigned 64-bit integer max representation (-1 in Lua 5.4) must serialize as 1bffffffffffffffff:
      local u_max = cbor.CborUnsigned(-1)
      local enc = cbor.encode_checked(u_max)
      assert.are.equal("1bffffffffffffffff", bytes_to_hex(enc))
      local dec = cbor.decode(enc)
      assert.are.equal(u_max, dec)
    end)

    it("CVE-2: pre-allocation OOM bomb defense", function()
      local oom_vectors = {
        hex_to_bytes("9a3b9aca00"), -- 1 billion element array in 5 bytes
        hex_to_bytes("ba3b9aca00"), -- 1 billion entry map in 5 bytes
        hex_to_bytes("5a05f5e100"), -- 100 million byte string in 5 bytes
        hex_to_bytes("7a05f5e100"), -- 100 million char text in 5 bytes
      }
      for _, vec in ipairs(oom_vectors) do
        local ok, err = pcall(cbor.decode, vec)
        assert.is_false(ok)
        assert.are.equal("length-too-large", err.id)
      end
    end)

    it("CVE-3: deep nesting recursion exhaustion defense (depth 128 vs 129)", function()
      -- Exactly 128 depth passes:
      local val_128 = build_nested_array(128)
      local wire_128 = build_nested_array_wire(128)
      assert.are.equal(wire_128, cbor.encode_checked(val_128))
      assert.are.equal(val_128, cbor.decode(wire_128))

      -- Depth 129 rejects on decode (too-deep):
      local wire_129 = build_nested_array_wire(129)
      local ok_dec, err_dec = pcall(cbor.decode, wire_129)
      assert.is_false(ok_dec)
      assert.are.equal("too-deep", err_dec.id)

      -- Depth 129 rejects on encode (encode-too-deep):
      local val_129 = build_nested_array(129)
      local ok_enc, err_enc = pcall(cbor.encode_checked, val_129)
      assert.is_false(ok_enc)
      assert.are.equal("encode-too-deep", err_enc.id)
    end)

    it("CVE-4: canonical length-first map key ordering & duplicate key defense", function()
      -- Non-minimal integer:
      local ok_min, err_min = pcall(cbor.decode, hex_to_bytes("1800"))
      assert.is_false(ok_min)
      assert.are.equal("non-minimal-integer", err_min.id)

      -- Out of order keys {"b": 1, "a": 2}:
      local ok_ord, err_ord = pcall(cbor.decode, hex_to_bytes("a2616201616102"))
      assert.is_false(ok_ord)
      assert.are.equal("non-canonical-map-order", err_ord.id)

      -- Duplicate keys on wire {"a": 1, "a": 2}:
      local ok_dup, err_dup = pcall(cbor.decode, hex_to_bytes("a2616101616102"))
      assert.is_false(ok_dup)
      assert.are.equal("non-canonical-map-order", err_dup.id)

      -- Duplicate keys rejected on encode:
      local dup_map = cbor.CborMap({
        cbor.CborMapEntry(cbor.CborText("role"), cbor.CborText("guest")),
        cbor.CborMapEntry(cbor.CborText("role"), cbor.CborText("admin")),
      })
      local ok_enc, err_enc = pcall(cbor.encode_checked, dup_map)
      assert.is_false(ok_enc)
      assert.are.equal("duplicate-map-key", err_enc.id)
    end)

    it("CVE-5: UTF-8 overlong and surrogate smuggling defense", function()
      local smuggling_vectors = {
        hex_to_bytes("62c080"),     -- Overlong 2-byte NUL
        hex_to_bytes("63e080af"),   -- Overlong 3-byte slash
        hex_to_bytes("64f08080af"), -- Overlong 4-byte slash
        hex_to_bytes("63eda080"),   -- Lone surrogate U+D800
        hex_to_bytes("63edbfbf"),   -- Lone surrogate U+DFFF
        hex_to_bytes("61ff"),       -- Invalid 0xFF byte
      }
      for _, vec in ipairs(smuggling_vectors) do
        local ok, err = pcall(cbor.decode, vec)
        assert.is_false(ok)
        assert.are.equal("invalid-utf8", err.id)
      end

      -- CborText constructor rejects surrogates and invalid bytes:
      assert.has_error(function() cbor.CborText("\xED\xA0\x80") end)
      assert.has_error(function() cbor.CborText("\xFF") end)

      -- Multi-byte emoji round-trip:
      local emoji = "🚀"
      local t_emoji = cbor.CborText(emoji)
      local enc_emoji = cbor.encode_checked(t_emoji)
      assert.are.equal(t_emoji, cbor.decode(enc_emoji))
    end)

    it("CVE-6: trailing-byte smuggling and metatable TOCTOU spoofing", function()
      local valid_wire = hex_to_bytes("01")
      local smuggled = valid_wire .. "\xDE\xAD\xBE\xEF"
      local ok_trail, err_trail = pcall(cbor.decode, smuggled)
      assert.is_false(ok_trail)
      assert.are.equal("trailing-bytes", err_trail.id)

      -- Reject table with metatable (spoofing __index / __len):
      local spoofed = setmetatable({ 1, 2, 3 }, {
        __index = function() return 99 end,
        __len = function() return 100 end,
      })
      assert.has_error(function() cbor.CborArray(spoofed) end)
      assert.has_error(function() cbor.CborMap(spoofed) end)
      assert.has_error(function() cbor.CborBytes(spoofed) end)
      assert.has_error(function() cbor.decode(spoofed) end)
      assert.has_error(function() cbor.encode_into_checked(cbor.CborNull(), spoofed) end)
    end)

    it("CVE-7: secret payload reflection leak defense and SpannedMapEntry redaction", function()
      local secret_token = "SECRET_API_TOKEN_XYZ_12345"
      local bad_map = cbor.CborMap({
        cbor.CborMapEntry(cbor.CborText(secret_token), cbor.CborUnsigned(1)),
        cbor.CborMapEntry(cbor.CborText(secret_token), cbor.CborUnsigned(2)),
      })

      local ok, err = pcall(cbor.encode_checked, bad_map)
      assert.is_false(ok)
      local msg = err.message
      assert.is_nil(msg:find(secret_token, 1, true))
      assert.is_truthy(msg:find("^canonical%-cbor: "))

      local spanned = cbor.SpannedMapEntry(
        cbor.CborText(secret_token),
        cbor.CborText("PASSWORD_HASH_DO_NOT_PRINT"),
        cbor.Span(0, 10)
      )
      local rep = tostring(spanned)
      assert.is_nil(rep:find(secret_token, 1, true))
      assert.is_nil(rep:find("PASSWORD_HASH_DO_NOT_PRINT", 1, true))
      assert.is_truthy(rep:find("<redacted>", 1, true))
    end)
  end)

  -- =========================================================================
  -- 8. Deterministic Property Tests, Overflow, and Fuzzing
  -- =========================================================================

  describe("Deterministic property tests and fuzzing", function()
    it("preserves round-trip equality under PRNG fuzzing (500 iterations)", function()
      local prng = make_prng(12345)
      for _ = 1, 500 do
        local type_choice = prng.next_int(7)
        local val
        if type_choice == 0 then
          val = cbor.CborUnsigned(prng.next_u64())
        elseif type_choice == 1 then
          val = cbor.CborNegative(prng.next_u64())
        elseif type_choice == 2 then
          val = cbor.CborBytes(prng.next_bytes(prng.next_int(32)))
        elseif type_choice == 3 then
          val = cbor.CborText("text_" .. prng.next_u32())
        elseif type_choice == 4 then
          val = cbor.CborArray({
            cbor.CborUnsigned(prng.next_int(100)),
            cbor.CborBool(prng.next_int(2) == 1),
            cbor.CborNull(),
          })
        elseif type_choice == 5 then
          val = cbor.CborTag(prng.next_int(100), cbor.CborUnsigned(prng.next_int(1000)))
        else
          val = cbor.CborBool(prng.next_int(2) == 0)
        end

        local encoded = cbor.encode_checked(val)
        assert.is_true(#encoded <= constants.MAX_ENCODED_BYTES)
        local decoded = cbor.decode(encoded)
        assert.are.equal(val, decoded)
        local re_encoded = cbor.encode_checked(decoded)
        assert.are.equal(encoded, re_encoded)
      end
    end)

    it("triggers natural buffer overflow via push_byte (encode-too-large)", function()
      -- Array of 2 items: 1 byte header 0x82.
      -- First item: CborBytes with 5 bytes header (0x5A + 4 bytes length) + payload.
      -- Total before second item: 1 + 5 + payload = MAX_ENCODED_BYTES.
      -- Second item (CborNull) pushes 1 byte and triggers encode-too-large!
      local payload = string.rep("\x00", constants.MAX_ENCODED_BYTES - 6)
      local val = cbor.CborArray({
        cbor.CborBytes(payload),
        cbor.CborNull(),
      })
      local ok, err = pcall(cbor.encode_checked, val)
      assert.is_false(ok)
      assert.are.equal("encode-too-large", err.id)
    end)

    it("survives 10,000-iteration adversarial byte-mutation fuzz loop", function()
      local prng = make_prng(99999)
      local iters = 10000
      for _ = 1, iters do
        local len = prng.next_int(64)
        local raw = prng.next_bytes(len)

        local ok, res = pcall(cbor.decode, raw)
        if ok then
          if #raw > constants.MAX_ENCODED_BYTES then
            local ok_e, err_e = pcall(cbor.encode_checked, res)
            assert.is_false(ok_e)
            assert.are.equal("encode-too-large", err_e.id)
          else
            local re_enc = cbor.encode_checked(res)
            assert.are.equal(raw, re_enc)
            local res2 = cbor.decode(re_enc)
            assert.are.equal(res, res2)
          end
        else
          assert.is_truthy(res.message:find("^canonical%-cbor: "))
          assert.is_true(messages.is_valid_id(res.id))
        end
      end
    end)
  end)

  -- =========================================================================
  -- 9. Exhaustive Coverage of Equality, Immutability, and Defensive Branches
  -- =========================================================================

  describe("Exhaustive coverage of equality, immutability, and defensive branches", function()
    it("tests immutability on all module exports", function()
      local types_mod = require("coding_adventures.canonical_cbor.types")
      local codec_mod = require("coding_adventures.canonical_cbor.codec")

      assert.has_error(function() constants.NEW_KEY = 1 end)
      assert.has_error(function() messages.NEW_KEY = 1 end)
      assert.has_error(function() messages.ERROR_MESSAGES["foo"] = "bar" end)
      assert.has_error(function() types_mod.NEW_KEY = 1 end)
      assert.has_error(function() codec_mod.NEW_KEY = 1 end)
      assert.has_error(function() cbor.NEW_KEY = 1 end)
      assert.has_error(function() cbor.DecodeMapSpansResult.NEW_KEY = 1 end)
    end)

    it("tests Span methods and equality edge cases", function()
      local s1 = cbor.Span(0, 10)
      local s2 = cbor.Span(0, 10)
      local s3 = cbor.Span(0, 11)
      local s4 = cbor.Span(1, 10)

      assert.are.equal(10, s1:length())
      assert.are.equal(s1, s2)
      assert.are_not.equal(s1, s3)
      assert.are_not.equal(s1, s4)
      assert.are_not.equal(s1, "not a span")

      local seen_keys = {}
      for k, v in pairs(s1) do
        seen_keys[k] = v
      end
      assert.are.equal(0, seen_keys.start)
      assert.are.equal(10, seen_keys.finish)
    end)

    it("tests SpannedMapEntry equality edge cases", function()
      local e1 = cbor.SpannedMapEntry(cbor.CborUnsigned(1), cbor.CborBool(true), cbor.Span(0, 1))
      local e2 = cbor.SpannedMapEntry(cbor.CborUnsigned(1), cbor.CborBool(true), cbor.Span(0, 1))
      local e3 = cbor.SpannedMapEntry(cbor.CborUnsigned(2), cbor.CborBool(true), cbor.Span(0, 1))
      local e4 = cbor.SpannedMapEntry(cbor.CborUnsigned(1), cbor.CborBool(false), cbor.Span(0, 1))
      local e5 = cbor.SpannedMapEntry(cbor.CborUnsigned(1), cbor.CborBool(true), cbor.Span(0, 2))

      assert.are.equal(e1, e2)
      assert.are_not.equal(e1, e3)
      assert.are_not.equal(e1, e4)
      assert.are_not.equal(e1, e5)
      assert.are_not.equal(e1, "not spanned entry")
    end)

    it("tests DecodeMapSpansResult equality edge cases", function()
      local e1 = cbor.SpannedMapEntry(cbor.CborUnsigned(1), cbor.CborBool(true), cbor.Span(0, 1))
      local e2 = cbor.SpannedMapEntry(cbor.CborUnsigned(2), cbor.CborBool(false), cbor.Span(0, 1))

      local res1 = cbor.DecodeMapSpansResult.map({ e1 })
      local res2 = cbor.DecodeMapSpansResult.map({ e1 })
      local res3 = cbor.DecodeMapSpansResult.map({ e2 })
      local res4 = cbor.DecodeMapSpansResult.map({ e1, e2 })
      local non1 = cbor.DecodeMapSpansResult.non_map()
      local non2 = cbor.DecodeMapSpansResult.non_map()

      assert.are.equal(res1, res2)
      assert.are_not.equal(res1, res3)
      assert.are_not.equal(res1, res4)
      assert.are_not.equal(res1, non1)
      assert.are_not.equal(non1, res1)
      assert.are.equal(non1, non2)
      assert.are_not.equal(res1, "not a result")
    end)

    it("tests deep equality across all CBOR value kinds and mismatch branches", function()
      local u = cbor.CborUnsigned(1)
      local n = cbor.CborNegative(1)
      local b = cbor.CborBytes("\x01")
      local t = cbor.CborText("1")
      local arr1 = cbor.CborArray({ u })
      local arr2 = cbor.CborArray({ u, u })
      local arr3 = cbor.CborArray({ n })
      local me1 = cbor.CborMapEntry(t, u)
      local me2 = cbor.CborMapEntry(t, n)
      local me3 = cbor.CborMapEntry(cbor.CborText("2"), u)
      local map1 = cbor.CborMap({ me1 })
      local map2 = cbor.CborMap({ me1, me2 })
      local map3 = cbor.CborMap({ me2 })
      local tag1 = cbor.CborTag(1, u)
      local tag2 = cbor.CborTag(2, u)
      local tag3 = cbor.CborTag(1, n)
      local b_t = cbor.CborBool(true)
      local b_f = cbor.CborBool(false)
      local null_v = cbor.CborNull()

      -- Cross-kind inequalities:
      assert.are_not.equal(u, n)
      assert.are_not.equal(u, "raw")
      assert.are_not.equal(b, t)
      assert.are_not.equal(b_t, null_v)
      assert.are_not.equal(tag1, arr1)
      assert.are_not.equal(arr1, map1)

      -- Array inequality:
      assert.are_not.equal(arr1, arr2)
      assert.are_not.equal(arr1, arr3)

      -- MapEntry inequality:
      assert.are_not.equal(me1, me2)
      assert.are_not.equal(me1, me3)

      -- Map inequality:
      assert.are_not.equal(map1, map2)
      assert.are_not.equal(map1, map3)

      -- Tag inequality:
      assert.are_not.equal(tag1, tag2)
      assert.are_not.equal(tag1, tag3)

      -- Bool inequality:
      assert.are_not.equal(b_t, b_f)

      -- Read-only list pairs/ipairs:
      local count = 0
      for _, item in ipairs(arr1.items) do
        count = count + 1
        assert.are.equal(u, item)
      end
      assert.are.equal(1, count)

      local pcount = 0
      for _ in pairs(arr1.items) do
        pcount = pcount + 1
      end
      assert.are.equal(1, pcount)
    end)

    it("enforces unforgeable type branding and rejects spoofed tables and improper CborMapEntry usage", function()
      -- Forged table attempting to masquerade as CborText:
      local forged_text = {
        kind = "text",
        text = "\xFF\xFE",
      }
      assert.is_false(types.is_cbor_value(forged_text))
      local ok_e, err_e = pcall(cbor.encode_checked, forged_text)
      assert.is_false(ok_e)
      assert.are.equal("unsupported-simple", err_e.id)

      -- Metatabled table attempting __index spoofing:
      local spoofed = setmetatable({}, {
        __index = function(_, k)
          if k == "kind" then return "unsigned" end
          if k == "value" then return 42 end
        end,
      })
      assert.is_false(types.is_cbor_value(spoofed))
      local ok_s, err_s = pcall(cbor.encode_checked, spoofed)
      assert.is_false(ok_s)
      assert.are.equal("unsupported-simple", err_s.id)

      -- CborMapEntry is a pair model, NOT a CborValue:
      local entry = cbor.CborMapEntry(cbor.CborUnsigned(1), cbor.CborText("a"))
      assert.is_true(types.is_cbor_map_entry(entry))
      assert.is_false(types.is_cbor_value(entry))

      -- Reject CborMapEntry inside CborArray:
      local ok_a, err_a = pcall(cbor.CborArray, { entry })
      assert.is_false(ok_a)
      assert.are.equal("unsupported-simple", err_a.id)

      -- Reject CborMapEntry inside CborTag:
      local ok_t, err_t = pcall(cbor.CborTag, 1, entry)
      assert.is_false(ok_t)
      assert.are.equal("unsupported-simple", err_t.id)

      -- Reject CborMapEntry passed directly to encode_checked:
      local ok_me, err_me = pcall(cbor.encode_checked, entry)
      assert.is_false(ok_me)
      assert.are.equal("unsupported-simple", err_me.id)

      -- Reject non-CBOR values:
      local ok_r, err_r = pcall(cbor.encode_checked, 123)
      assert.is_false(ok_r)
      assert.are.equal("unsupported-simple", err_r.id)

      -- Deep equality defense against non-registered objects:
      local u = cbor.CborUnsigned(10)
      assert.are_not.equal(u, { kind = "unsigned", value = 10 })
      assert.are_not.equal(entry, { key = cbor.CborUnsigned(1), value = cbor.CborText("a") })
    end)

    it("validates dst_table sequentially and performs atomic rollback on all encode errors", function()
      -- Non-table dst_table:
      local ok1, err1 = pcall(cbor.encode_into_checked, cbor.CborUnsigned(1), "not-a-table")
      assert.is_false(ok1)
      assert.are.equal("unsupported-simple", err1.id)

      -- Metatabled dst_table:
      local ok2, err2 = pcall(cbor.encode_into_checked, cbor.CborUnsigned(1), setmetatable({}, {}))
      assert.is_false(ok2)
      assert.are.equal("unsupported-simple", err2.id)

      -- Hash/string-keyed dst_table:
      local ok3, err3 = pcall(cbor.encode_into_checked, cbor.CborUnsigned(1), { a = 1 })
      assert.is_false(ok3)
      assert.are.equal("unsupported-simple", err3.id)

      -- Sparse dst_table with holes:
      local ok4, err4 = pcall(cbor.encode_into_checked, cbor.CborUnsigned(1), { [1] = "x", [3] = "y" })
      assert.is_false(ok4)
      assert.are.equal("unsupported-simple", err4.id)

      -- Atomic rollback on duplicate-map-key:
      local dst = { "initial_entry" }
      local dup_map = cbor.CborMap({
        cbor.CborMapEntry(cbor.CborUnsigned(1), cbor.CborText("a")),
        cbor.CborMapEntry(cbor.CborUnsigned(1), cbor.CborText("b")),
      })
      local ok_dup, err_dup = pcall(cbor.encode_into_checked, dup_map, dst)
      assert.is_false(ok_dup)
      assert.are.equal("duplicate-map-key", err_dup.id)
      assert.are.equal(1, #dst)
      assert.are.equal("initial_entry", dst[1])

      -- Atomic rollback on encode-too-deep:
      local deep_val = cbor.CborUnsigned(1)
      for _ = 1, 130 do
        deep_val = cbor.CborArray({ deep_val })
      end
      local deep_map = cbor.CborMap({
        cbor.CborMapEntry(cbor.CborUnsigned(1), deep_val),
      })
      local ok_deep, err_deep = pcall(cbor.encode_into_checked, deep_map, dst)
      assert.is_false(ok_deep)
      assert.are.equal("encode-too-deep", err_deep.id)
      assert.are.equal(1, #dst)
      assert.are.equal("initial_entry", dst[1])

      -- Atomic rollback on encode-too-large:
      local huge_bytes = cbor.CborBytes(string.rep("x", 1048576))
      local huge_map = cbor.CborMap({
        cbor.CborMapEntry(cbor.CborUnsigned(1), huge_bytes),
      })
      local ok_large, err_large = pcall(cbor.encode_into_checked, huge_map, dst)
      assert.is_false(ok_large)
      assert.are.equal("encode-too-large", err_large.id)
      assert.are.equal(1, #dst)
      assert.are.equal("initial_entry", dst[1])
    end)

    it("validates malformed map headers and trailing bytes in decode_map_spans", function()
      -- Indefinite-length map header (0xBF)
      local ok_indef, err_indef = pcall(cbor.decode_map_spans, "\xBF\xFF")
      assert.is_false(ok_indef)
      assert.are.equal("indefinite", err_indef.id)

      -- Non-minimal 1-byte length map header (0xB800)
      local ok_nonmin1, err_nonmin1 = pcall(cbor.decode_map_spans, "\xB8\x00")
      assert.is_false(ok_nonmin1)
      assert.are.equal("non-minimal-integer", err_nonmin1.id)

      -- Non-minimal 2-byte length map header (0xB90001)
      local ok_nonmin2, err_nonmin2 = pcall(cbor.decode_map_spans, "\xB9\x00\x01\x01\x02")
      assert.is_false(ok_nonmin2)
      assert.are.equal("non-minimal-integer", err_nonmin2.id)

      -- Non-minimal 4-byte length map header (0xBA00000001)
      local ok_nonmin4, err_nonmin4 = pcall(cbor.decode_map_spans, "\xBA\x00\x00\x00\x01\x01\x02")
      assert.is_false(ok_nonmin4)
      assert.are.equal("non-minimal-integer", err_nonmin4.id)

      -- Reserved additional info in map header (0xBC)
      local ok_res, err_res = pcall(cbor.decode_map_spans, "\xBC")
      assert.is_false(ok_res)
      assert.are.equal("reserved", err_res.id)

      -- Trailing bytes after map in decode_map_spans
      local ok_trail, err_trail = pcall(cbor.decode_map_spans, "\xA0\xFF")
      assert.is_false(ok_trail)
      assert.are.equal("trailing-bytes", err_trail.id)

      -- Pre-allocation length exceeds remaining bytes
      local ok_len, err_len = pcall(cbor.decode_map_spans, "\xA1\x01")
      assert.is_false(ok_len)
      assert.are.equal("length-too-large", err_len.id)

      -- Truncated value payload in map entry triggers unexpected-eof
      local ok_trunc, err_trunc = pcall(cbor.decode_map_spans, "\xA1\x01\x18")
      assert.is_false(ok_trunc)
      assert.are.equal("unexpected-eof", err_trunc.id)
    end)

    it("enforces module-level immutability across all submodules with payload-blind errors", function()
      -- constants module is read-only:
      local ok_c, err_c = pcall(function() constants.NEW_KEY = 1 end)
      assert.is_false(ok_c)
      assert.are.equal(messages.READ_ONLY_MODULE_MESSAGE, err_c)

      -- messages module is read-only:
      local ok_m, err_m = pcall(function() messages.NEW_KEY = 1 end)
      assert.is_false(ok_m)
      assert.are.equal(messages.READ_ONLY_MODULE_MESSAGE, err_m)

      -- messages.ERROR_MESSAGES is immutable:
      local ok_em, err_em = pcall(function() messages.ERROR_MESSAGES["foo"] = "bar" end)
      assert.is_false(ok_em)
      assert.are.equal(messages.IMMUTABLE_ERROR_MESSAGES, err_em)

      -- types module is read-only:
      local ok_t, err_t = pcall(function() types.NEW_KEY = 1 end)
      assert.is_false(ok_t)
      assert.are.equal(messages.READ_ONLY_MODULE_MESSAGE, err_t)

      -- codec module is read-only:
      local ok_cd, err_cd = pcall(function() codec.NEW_KEY = 1 end)
      assert.is_false(ok_cd)
      assert.are.equal(messages.READ_ONLY_MODULE_MESSAGE, err_cd)

      -- root cbor facade module is read-only:
      local ok_rt, err_rt = pcall(function() cbor.NEW_KEY = 1 end)
      assert.is_false(ok_rt)
      assert.are.equal(messages.READ_ONLY_MODULE_MESSAGE, err_rt)

      -- message_for with unknown error id:
      local ok_un, err_un = pcall(messages.message_for, "unrecognized-id")
      assert.is_false(ok_un)
      assert.are.equal(messages.UNKNOWN_ERROR_ID_MESSAGE, err_un)
    end)

    it("verifies defensive equality, formatting, and helper branches", function()
      -- CborError equality with non-CborError:
      local err1 = types.CborError("unexpected-eof")
      assert.is_true(types.is_cbor_error(err1))
      assert.is_false(types.is_cbor_error({ id = "unexpected-eof" }))
      assert.are_not.equal(err1, "unexpected-eof")
      assert.are_not.equal(err1, { id = "unexpected-eof" })

      -- Span equality with non-Span:
      local sp = types.Span(0, 5)
      assert.is_true(types.is_span(sp))
      assert.is_false(types.is_span({ start = 0, finish = 5 }))
      assert.are_not.equal(sp, { start = 0, finish = 5 })

      -- SpannedMapEntry equality with non-SpannedMapEntry:
      local k = cbor.CborUnsigned(1)
      local v = cbor.CborText("test")
      local sme = types.SpannedMapEntry(k, v, sp)
      assert.is_true(types.is_spanned_map_entry(sme))
      assert.is_false(types.is_spanned_map_entry({ key = k, value = v, value_span = sp }))
      assert.are_not.equal(sme, { key = k, value = v, value_span = sp })

      -- SpannedMapEntry constructor rejection with invalid types:
      local ok_sme1, err_sme1 = pcall(types.SpannedMapEntry, "raw", v, sp)
      assert.is_false(ok_sme1)
      assert.are.equal("unsupported-simple", err_sme1.id)

      -- DecodeMapSpansResult equality and validation:
      local dmsr = types.DecodeMapSpansResult.map({ sme })
      assert.is_true(types.is_decode_map_spans_result(dmsr))
      assert.is_false(types.is_decode_map_spans_result({ is_map = true }))
      assert.are_not.equal(dmsr, { is_map = true })

      local ok_dm1, err_dm1 = pcall(types.DecodeMapSpansResult.map, "not-a-table")
      assert.is_false(ok_dm1)
      assert.are.equal("unsupported-simple", err_dm1.id)

      local ok_dm2, err_dm2 = pcall(types.DecodeMapSpansResult.map, { [1] = sme, [3] = sme })
      assert.is_false(ok_dm2)
      assert.are.equal("unsupported-simple", err_dm2.id)

      -- Non-sequential table rejection in constructors:
      local ok_arr, err_arr = pcall(cbor.CborArray, { [1] = k, [3] = k })
      assert.is_false(ok_arr)
      assert.are.equal("unsupported-simple", err_arr.id)

      local ok_map, err_map = pcall(cbor.CborMap, { [1] = cbor.CborMapEntry(k, v), [3] = cbor.CborMapEntry(k, v) })
      assert.is_false(ok_map)
      assert.are.equal("unsupported-simple", err_map.id)

      local ok_norm, err_norm = pcall(types.normalize_bytes, { [1] = 65, [3] = 66 }, "unsupported-simple")
      assert.is_false(ok_norm)
      assert.are.equal("unsupported-simple", err_norm.id)

      -- String representations:
      assert.is_truthy(tostring(cbor.CborText("abc")):find("CborText"))
      assert.is_truthy(tostring(cbor.CborBool(false)):find("CborBool%(false%)"))
      assert.is_truthy(tostring(cbor.CborMapEntry(k, v)):find("CborMapEntry"))

      -- Map inequality when entry count differs:
      local m1 = cbor.CborMap({ cbor.CborMapEntry(k, v) })
      local m2 = cbor.CborMap({ cbor.CborMapEntry(k, v), cbor.CborMapEntry(cbor.CborUnsigned(2), v) })
      assert.are_not.equal(m1, m2)
    end)
  end)
end)

-- Tests for coding_adventures.wasm_types
--
-- Covers: ValType, RefType, BlockType, ExternType constants,
-- is_val_type, is_ref_type, val_type_name,
-- encode_val_type, decode_val_type,
-- encode_limits, decode_limits,
-- encode_func_type, decode_func_type

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local wt = require("coding_adventures.wasm_types")

-- Helper: compare two byte arrays
local function bytes_equal(a, b)
    if #a ~= #b then return false end
    for i = 1, #a do
        if a[i] ~= b[i] then return false end
    end
    return true
end

describe("wasm_types", function()

    -- -----------------------------------------------------------------------
    -- Meta
    -- -----------------------------------------------------------------------

    it("has VERSION 0.1.0", function()
        assert.equals("0.1.0", wt.VERSION)
    end)

    -- -----------------------------------------------------------------------
    -- ValType constants
    -- -----------------------------------------------------------------------

    describe("ValType constants", function()
        it("i32 = 0x7F", function() assert.equals(0x7F, wt.ValType.i32) end)
        it("i64 = 0x7E", function() assert.equals(0x7E, wt.ValType.i64) end)
        it("f32 = 0x7D", function() assert.equals(0x7D, wt.ValType.f32) end)
        it("f64 = 0x7C", function() assert.equals(0x7C, wt.ValType.f64) end)
        it("v128 = 0x7B", function() assert.equals(0x7B, wt.ValType.v128) end)
        it("funcref = 0x70", function() assert.equals(0x70, wt.ValType.funcref) end)
        it("externref = 0x6F", function() assert.equals(0x6F, wt.ValType.externref) end)
    end)

    -- -----------------------------------------------------------------------
    -- RefType constants
    -- -----------------------------------------------------------------------

    describe("RefType constants", function()
        it("funcref = 0x70", function() assert.equals(0x70, wt.RefType.funcref) end)
        it("externref = 0x6F", function() assert.equals(0x6F, wt.RefType.externref) end)
    end)

    -- -----------------------------------------------------------------------
    -- BlockType constants
    -- -----------------------------------------------------------------------

    describe("BlockType constants", function()
        it("empty = 0x40", function() assert.equals(0x40, wt.BlockType.empty) end)
    end)

    -- -----------------------------------------------------------------------
    -- ExternType constants
    -- -----------------------------------------------------------------------

    describe("ExternType constants", function()
        it("func = 0",   function() assert.equals(0, wt.ExternType.func) end)
        it("table = 1",  function() assert.equals(1, wt.ExternType.table) end)
        it("mem = 2",    function() assert.equals(2, wt.ExternType.mem) end)
        it("global = 3", function() assert.equals(3, wt.ExternType.global) end)
    end)

    -- -----------------------------------------------------------------------
    -- is_val_type
    -- -----------------------------------------------------------------------

    describe("is_val_type", function()
        it("returns true for all valid val types", function()
            assert.is_true(wt.is_val_type(0x7F))  -- i32
            assert.is_true(wt.is_val_type(0x7E))  -- i64
            assert.is_true(wt.is_val_type(0x7D))  -- f32
            assert.is_true(wt.is_val_type(0x7C))  -- f64
            assert.is_true(wt.is_val_type(0x7B))  -- v128
            assert.is_true(wt.is_val_type(0x70))  -- funcref
            assert.is_true(wt.is_val_type(0x6F))  -- externref
        end)

        it("returns false for invalid bytes", function()
            assert.is_false(wt.is_val_type(0x00))
            assert.is_false(wt.is_val_type(0x40))  -- BlockType.empty, not a ValType
            assert.is_false(wt.is_val_type(0x60))  -- FuncType magic, not a ValType
            assert.is_false(wt.is_val_type(0x7A))  -- between v128 and funcref
            assert.is_false(wt.is_val_type(0xFF))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- is_ref_type
    -- -----------------------------------------------------------------------

    describe("is_ref_type", function()
        it("returns true for funcref and externref", function()
            assert.is_true(wt.is_ref_type(0x70))
            assert.is_true(wt.is_ref_type(0x6F))
        end)

        it("returns false for non-reference types", function()
            assert.is_false(wt.is_ref_type(0x7F))  -- i32
            assert.is_false(wt.is_ref_type(0x7E))  -- i64
            assert.is_false(wt.is_ref_type(0x7D))  -- f32
            assert.is_false(wt.is_ref_type(0x7C))  -- f64
            assert.is_false(wt.is_ref_type(0x7B))  -- v128
            assert.is_false(wt.is_ref_type(0x00))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- val_type_name
    -- -----------------------------------------------------------------------

    describe("val_type_name", function()
        it("returns 'i32' for 0x7F",       function() assert.equals("i32",      wt.val_type_name(0x7F)) end)
        it("returns 'i64' for 0x7E",       function() assert.equals("i64",      wt.val_type_name(0x7E)) end)
        it("returns 'f32' for 0x7D",       function() assert.equals("f32",      wt.val_type_name(0x7D)) end)
        it("returns 'f64' for 0x7C",       function() assert.equals("f64",      wt.val_type_name(0x7C)) end)
        it("returns 'v128' for 0x7B",      function() assert.equals("v128",     wt.val_type_name(0x7B)) end)
        it("returns 'funcref' for 0x70",   function() assert.equals("funcref",  wt.val_type_name(0x70)) end)
        it("returns 'externref' for 0x6F", function() assert.equals("externref",wt.val_type_name(0x6F)) end)

        it("returns 'unknown_0xXX' for unrecognized bytes", function()
            assert.equals("unknown_0x00", wt.val_type_name(0x00))
            assert.equals("unknown_0x42", wt.val_type_name(0x42))
            assert.equals("unknown_0x60", wt.val_type_name(0x60))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- encode_val_type
    -- -----------------------------------------------------------------------

    describe("encode_val_type", function()
        it("encodes i32 as {0x7F}", function()
            assert.is_true(bytes_equal({0x7F}, wt.encode_val_type(0x7F)))
        end)
        it("encodes i64 as {0x7E}", function()
            assert.is_true(bytes_equal({0x7E}, wt.encode_val_type(0x7E)))
        end)
        it("encodes funcref as {0x70}", function()
            assert.is_true(bytes_equal({0x70}, wt.encode_val_type(0x70)))
        end)
        it("encodes externref as {0x6F}", function()
            assert.is_true(bytes_equal({0x6F}, wt.encode_val_type(0x6F)))
        end)
        it("returns exactly one byte", function()
            assert.equals(1, #wt.encode_val_type(0x7F))
        end)
        it("errors on invalid val type", function()
            assert.has_error(function() wt.encode_val_type(0x42) end)
            assert.has_error(function() wt.encode_val_type(0x00) end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- decode_val_type
    -- -----------------------------------------------------------------------

    describe("decode_val_type", function()
        it("decodes i32 from {0x7F}", function()
            local r = wt.decode_val_type({0x7F}, 1)
            assert.equals(0x7F, r.type)
            assert.equals(1, r.bytes_consumed)
        end)
        it("decodes i64 from {0x7E}", function()
            local r = wt.decode_val_type({0x7E})
            assert.equals(0x7E, r.type)
            assert.equals(1, r.bytes_consumed)
        end)
        it("decodes at offset", function()
            local r = wt.decode_val_type({0x00, 0x7F}, 2)
            assert.equals(0x7F, r.type)
            assert.equals(1, r.bytes_consumed)
        end)
        it("errors on invalid val type byte", function()
            assert.has_error(function() wt.decode_val_type({0x42}) end)
        end)
        it("errors when offset is out of range", function()
            assert.has_error(function() wt.decode_val_type({0x7F}, 5) end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- encode_limits
    -- -----------------------------------------------------------------------

    describe("encode_limits", function()
        it("no max: {0x00} + min LEB128", function()
            -- min=0: LEB128(0) = {0x00}
            assert.is_true(bytes_equal({0x00, 0x00}, wt.encode_limits({min=0})))
        end)

        it("no max, min=1: {0x00, 0x01}", function()
            assert.is_true(bytes_equal({0x00, 0x01}, wt.encode_limits({min=1, max=nil})))
        end)

        it("has max: {0x01} + min + max", function()
            -- min=1, max=16: {0x01, 0x01, 0x10}
            assert.is_true(bytes_equal({0x01, 0x01, 0x10}, wt.encode_limits({min=1, max=16})))
        end)

        it("min=0, max=0: {0x01, 0x00, 0x00}", function()
            assert.is_true(bytes_equal({0x01, 0x00, 0x00}, wt.encode_limits({min=0, max=0})))
        end)

        it("large values use multi-byte LEB128", function()
            -- min=128: LEB128(128) = {0x80, 0x01} (two bytes)
            local result = wt.encode_limits({min=128})
            assert.equals(0x00, result[1])  -- no-max flag
            assert.equals(0x80, result[2])  -- LEB128 first byte of 128
            assert.equals(0x01, result[3])  -- LEB128 second byte of 128
        end)
    end)

    -- -----------------------------------------------------------------------
    -- decode_limits
    -- -----------------------------------------------------------------------

    describe("decode_limits", function()
        it("decodes no-max limits: {0x00, 0x00} → min=0, max=nil", function()
            local r = wt.decode_limits({0x00, 0x00})
            assert.equals(0, r.limits.min)
            assert.is_nil(r.limits.max)
            assert.equals(2, r.bytes_consumed)
        end)

        it("decodes no-max limits with min=5: {0x00, 0x05}", function()
            local r = wt.decode_limits({0x00, 0x05})
            assert.equals(5, r.limits.min)
            assert.is_nil(r.limits.max)
        end)

        it("decodes bounded limits: {0x01, 0x01, 0x10}", function()
            local r = wt.decode_limits({0x01, 0x01, 0x10})
            assert.equals(1, r.limits.min)
            assert.equals(16, r.limits.max)
            assert.equals(3, r.bytes_consumed)
        end)

        it("round-trips encode/decode for no-max", function()
            local orig = {min=42, max=nil}
            local encoded = wt.encode_limits(orig)
            local decoded = wt.decode_limits(encoded)
            assert.equals(orig.min, decoded.limits.min)
            assert.is_nil(decoded.limits.max)
        end)

        it("round-trips encode/decode for bounded limits", function()
            local orig = {min=10, max=200}
            local encoded = wt.encode_limits(orig)
            local decoded = wt.decode_limits(encoded)
            assert.equals(orig.min, decoded.limits.min)
            assert.equals(orig.max, decoded.limits.max)
        end)

        it("errors on invalid flag byte", function()
            assert.has_error(function() wt.decode_limits({0x02, 0x00}) end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- encode_func_type
    -- -----------------------------------------------------------------------

    describe("encode_func_type", function()
        it("encodes empty func type () → (): {0x60, 0x00, 0x00}", function()
            local ft = {params={}, results={}}
            assert.is_true(bytes_equal({0x60, 0x00, 0x00}, wt.encode_func_type(ft)))
        end)

        it("encodes (i32) → (): {0x60, 0x01, 0x7F, 0x00}", function()
            local ft = {params={0x7F}, results={}}
            assert.is_true(bytes_equal({0x60, 0x01, 0x7F, 0x00}, wt.encode_func_type(ft)))
        end)

        it("encodes (i32, i32) → i64: {0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E}", function()
            local ft = {params={0x7F, 0x7F}, results={0x7E}}
            assert.is_true(bytes_equal({0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E}, wt.encode_func_type(ft)))
        end)

        it("starts with 0x60 magic byte", function()
            local ft = {params={0x7F}, results={0x7F}}
            assert.equals(0x60, wt.encode_func_type(ft)[1])
        end)
    end)

    -- -----------------------------------------------------------------------
    -- decode_func_type
    -- -----------------------------------------------------------------------

    describe("decode_func_type", function()
        it("decodes () → () from {0x60, 0x00, 0x00}", function()
            local r = wt.decode_func_type({0x60, 0x00, 0x00})
            assert.equals(0, #r.func_type.params)
            assert.equals(0, #r.func_type.results)
            assert.equals(3, r.bytes_consumed)
        end)

        it("decodes (i32) → () from {0x60, 0x01, 0x7F, 0x00}", function()
            local r = wt.decode_func_type({0x60, 0x01, 0x7F, 0x00})
            assert.equals(1, #r.func_type.params)
            assert.equals(0x7F, r.func_type.params[1])
            assert.equals(0, #r.func_type.results)
            assert.equals(4, r.bytes_consumed)
        end)

        it("decodes (i32, i32) → i64", function()
            local r = wt.decode_func_type({0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E})
            assert.equals(2, #r.func_type.params)
            assert.equals(0x7F, r.func_type.params[1])
            assert.equals(0x7F, r.func_type.params[2])
            assert.equals(1, #r.func_type.results)
            assert.equals(0x7E, r.func_type.results[1])
        end)

        it("errors if magic byte is not 0x60", function()
            assert.has_error(function() wt.decode_func_type({0x61, 0x00, 0x00}) end)
        end)

        it("round-trips encode/decode", function()
            local orig = {params={0x7F, 0x7C}, results={0x7E, 0x7D}}
            local encoded = wt.encode_func_type(orig)
            local decoded = wt.decode_func_type(encoded)
            assert.equals(#orig.params, #decoded.func_type.params)
            assert.equals(#orig.results, #decoded.func_type.results)
            for i = 1, #orig.params do
                assert.equals(orig.params[i], decoded.func_type.params[i])
            end
            for i = 1, #orig.results do
                assert.equals(orig.results[i], decoded.func_type.results[i])
            end
        end)
    end)

end)

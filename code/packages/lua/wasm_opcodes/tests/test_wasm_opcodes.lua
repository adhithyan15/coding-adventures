-- Tests for coding_adventures.wasm_opcodes
--
-- Covers: OPCODES table, opcode_name, is_valid_opcode, get_opcode_info

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local op = require("coding_adventures.wasm_opcodes")

describe("wasm_opcodes", function()

    -- -----------------------------------------------------------------------
    -- Meta
    -- -----------------------------------------------------------------------

    it("has VERSION 0.1.0", function()
        assert.equals("0.1.0", op.VERSION)
    end)

    it("exposes OPCODES table", function()
        assert.is_table(op.OPCODES)
    end)

    it("OPCODES table is non-empty", function()
        local count = 0
        for _ in pairs(op.OPCODES) do count = count + 1 end
        assert.is_true(count >= 40, "expected at least 40 opcodes, got " .. count)
    end)

    -- -----------------------------------------------------------------------
    -- Control flow opcodes
    -- -----------------------------------------------------------------------

    describe("control flow", function()
        it("0x00 is 'unreachable'", function()
            assert.equals("unreachable", op.opcode_name(0x00))
        end)
        it("0x01 is 'nop'", function()
            assert.equals("nop", op.opcode_name(0x01))
        end)
        it("0x02 is 'block'", function()
            assert.equals("block", op.opcode_name(0x02))
        end)
        it("0x03 is 'loop'", function()
            assert.equals("loop", op.opcode_name(0x03))
        end)
        it("0x04 is 'if'", function()
            assert.equals("if", op.opcode_name(0x04))
        end)
        it("0x05 is 'else'", function()
            assert.equals("else", op.opcode_name(0x05))
        end)
        it("0x0b is 'end'", function()
            assert.equals("end", op.opcode_name(0x0b))
        end)
        it("0x0c is 'br'", function()
            assert.equals("br", op.opcode_name(0x0c))
        end)
        it("0x0d is 'br_if'", function()
            assert.equals("br_if", op.opcode_name(0x0d))
        end)
        it("0x0e is 'br_table'", function()
            assert.equals("br_table", op.opcode_name(0x0e))
        end)
        it("0x0f is 'return'", function()
            assert.equals("return", op.opcode_name(0x0f))
        end)
        it("0x10 is 'call'", function()
            assert.equals("call", op.opcode_name(0x10))
        end)
        it("0x11 is 'call_indirect'", function()
            assert.equals("call_indirect", op.opcode_name(0x11))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Parametric opcodes
    -- -----------------------------------------------------------------------

    describe("parametric", function()
        it("0x1a is 'drop'", function()
            assert.equals("drop", op.opcode_name(0x1a))
        end)
        it("0x1b is 'select'", function()
            assert.equals("select", op.opcode_name(0x1b))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Variable opcodes
    -- -----------------------------------------------------------------------

    describe("variable", function()
        it("0x20 is 'local.get'", function()
            assert.equals("local.get", op.opcode_name(0x20))
        end)
        it("0x21 is 'local.set'", function()
            assert.equals("local.set", op.opcode_name(0x21))
        end)
        it("0x22 is 'local.tee'", function()
            assert.equals("local.tee", op.opcode_name(0x22))
        end)
        it("0x23 is 'global.get'", function()
            assert.equals("global.get", op.opcode_name(0x23))
        end)
        it("0x24 is 'global.set'", function()
            assert.equals("global.set", op.opcode_name(0x24))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Memory opcodes
    -- -----------------------------------------------------------------------

    describe("memory", function()
        it("0x28 is 'i32.load'", function()
            assert.equals("i32.load", op.opcode_name(0x28))
        end)
        it("0x29 is 'i64.load'", function()
            assert.equals("i64.load", op.opcode_name(0x29))
        end)
        it("0x2a is 'f32.load'", function()
            assert.equals("f32.load", op.opcode_name(0x2a))
        end)
        it("0x2b is 'f64.load'", function()
            assert.equals("f64.load", op.opcode_name(0x2b))
        end)
        it("0x2c is 'i32.load8_s'", function()
            assert.equals("i32.load8_s", op.opcode_name(0x2c))
        end)
        it("0x2d is 'i32.load8_u'", function()
            assert.equals("i32.load8_u", op.opcode_name(0x2d))
        end)
        it("0x2e is 'i32.load16_s'", function()
            assert.equals("i32.load16_s", op.opcode_name(0x2e))
        end)
        it("0x2f is 'i32.load16_u'", function()
            assert.equals("i32.load16_u", op.opcode_name(0x2f))
        end)
        it("0x30 is 'i64.load8_s'", function()
            assert.equals("i64.load8_s", op.opcode_name(0x30))
        end)
        it("0x31 is 'i64.load8_u'", function()
            assert.equals("i64.load8_u", op.opcode_name(0x31))
        end)
        it("0x36 is 'i32.store'", function()
            assert.equals("i32.store", op.opcode_name(0x36))
        end)
        it("0x3a is 'i32.store8'", function()
            assert.equals("i32.store8", op.opcode_name(0x3a))
        end)
        it("0x3b is 'i32.store16'", function()
            assert.equals("i32.store16", op.opcode_name(0x3b))
        end)
        it("0x3f is 'memory.size'", function()
            assert.equals("memory.size", op.opcode_name(0x3f))
        end)
        it("0x40 is 'memory.grow'", function()
            assert.equals("memory.grow", op.opcode_name(0x40))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Numeric: i32
    -- -----------------------------------------------------------------------

    describe("i32 numeric", function()
        it("0x41 is 'i32.const'",  function() assert.equals("i32.const",  op.opcode_name(0x41)) end)
        it("0x45 is 'i32.eqz'",   function() assert.equals("i32.eqz",   op.opcode_name(0x45)) end)
        it("0x46 is 'i32.eq'",    function() assert.equals("i32.eq",    op.opcode_name(0x46)) end)
        it("0x47 is 'i32.ne'",    function() assert.equals("i32.ne",    op.opcode_name(0x47)) end)
        it("0x48 is 'i32.lt_s'",  function() assert.equals("i32.lt_s",  op.opcode_name(0x48)) end)
        it("0x4a is 'i32.gt_s'",  function() assert.equals("i32.gt_s",  op.opcode_name(0x4a)) end)
        it("0x6a is 'i32.add'",   function() assert.equals("i32.add",   op.opcode_name(0x6a)) end)
        it("0x6b is 'i32.sub'",   function() assert.equals("i32.sub",   op.opcode_name(0x6b)) end)
        it("0x6c is 'i32.mul'",   function() assert.equals("i32.mul",   op.opcode_name(0x6c)) end)
        it("0x6d is 'i32.div_s'", function() assert.equals("i32.div_s", op.opcode_name(0x6d)) end)
        it("0x71 is 'i32.and'",   function() assert.equals("i32.and",   op.opcode_name(0x71)) end)
        it("0x72 is 'i32.or'",    function() assert.equals("i32.or",    op.opcode_name(0x72)) end)
        it("0x73 is 'i32.xor'",   function() assert.equals("i32.xor",   op.opcode_name(0x73)) end)
        it("0x74 is 'i32.shl'",   function() assert.equals("i32.shl",   op.opcode_name(0x74)) end)
        it("0x75 is 'i32.shr_s'", function() assert.equals("i32.shr_s", op.opcode_name(0x75)) end)
    end)

    -- -----------------------------------------------------------------------
    -- Numeric: i64
    -- -----------------------------------------------------------------------

    describe("i64 numeric", function()
        it("0x42 is 'i64.const'", function() assert.equals("i64.const", op.opcode_name(0x42)) end)
        it("0x7c is 'i64.add'",   function() assert.equals("i64.add",   op.opcode_name(0x7c)) end)
        it("0x7d is 'i64.sub'",   function() assert.equals("i64.sub",   op.opcode_name(0x7d)) end)
        it("0x7e is 'i64.mul'",   function() assert.equals("i64.mul",   op.opcode_name(0x7e)) end)
    end)

    -- -----------------------------------------------------------------------
    -- Numeric: f32
    -- -----------------------------------------------------------------------

    describe("f32 numeric", function()
        it("0x43 is 'f32.const'", function() assert.equals("f32.const", op.opcode_name(0x43)) end)
        it("0x92 is 'f32.add'",   function() assert.equals("f32.add",   op.opcode_name(0x92)) end)
        it("0x93 is 'f32.sub'",   function() assert.equals("f32.sub",   op.opcode_name(0x93)) end)
        it("0x94 is 'f32.mul'",   function() assert.equals("f32.mul",   op.opcode_name(0x94)) end)
    end)

    -- -----------------------------------------------------------------------
    -- Numeric: f64
    -- -----------------------------------------------------------------------

    describe("f64 numeric", function()
        it("0x44 is 'f64.const'", function() assert.equals("f64.const", op.opcode_name(0x44)) end)
        it("0xa0 is 'f64.add'",   function() assert.equals("f64.add",   op.opcode_name(0xa0)) end)
        it("0xa1 is 'f64.sub'",   function() assert.equals("f64.sub",   op.opcode_name(0xa1)) end)
        it("0xa2 is 'f64.mul'",   function() assert.equals("f64.mul",   op.opcode_name(0xa2)) end)
    end)

    -- -----------------------------------------------------------------------
    -- Conversion instructions
    -- -----------------------------------------------------------------------

    describe("conversions", function()
        it("0xa7 is 'i32.wrap_i64'",     function() assert.equals("i32.wrap_i64",     op.opcode_name(0xa7)) end)
        it("0xa8 is 'i32.trunc_f32_s'",  function() assert.equals("i32.trunc_f32_s",  op.opcode_name(0xa8)) end)
        it("0xac is 'i64.extend_i32_s'", function() assert.equals("i64.extend_i32_s", op.opcode_name(0xac)) end)
        it("0xb6 is 'f32.demote_f64'",   function() assert.equals("f32.demote_f64",   op.opcode_name(0xb6)) end)
        it("0xbb is 'f64.promote_f32'",  function() assert.equals("f64.promote_f32",  op.opcode_name(0xbb)) end)
    end)

    -- -----------------------------------------------------------------------
    -- opcode_name — unknown bytes
    -- -----------------------------------------------------------------------

    describe("opcode_name unknown bytes", function()
        it("returns 'unknown_0xXX' for unrecognized bytes", function()
            assert.equals("unknown_0x99", op.opcode_name(0x99))
            assert.equals("unknown_0xff", op.opcode_name(0xff))
            assert.equals("unknown_0x50", op.opcode_name(0x50))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- is_valid_opcode
    -- -----------------------------------------------------------------------

    describe("is_valid_opcode", function()
        it("returns true for all defined opcodes", function()
            for byte, _ in pairs(op.OPCODES) do
                assert.is_true(op.is_valid_opcode(byte),
                    string.format("expected is_valid_opcode(0x%02x) to be true", byte))
            end
        end)

        it("returns true for specific known opcodes", function()
            assert.is_true(op.is_valid_opcode(0x00))  -- unreachable
            assert.is_true(op.is_valid_opcode(0x6a))  -- i32.add
            assert.is_true(op.is_valid_opcode(0x10))  -- call
        end)

        it("returns false for unrecognized bytes", function()
            assert.is_false(op.is_valid_opcode(0x99))
            assert.is_false(op.is_valid_opcode(0xff))
            assert.is_false(op.is_valid_opcode(0x50))
            assert.is_false(op.is_valid_opcode(0x15))  -- not in MVP
        end)
    end)

    -- -----------------------------------------------------------------------
    -- get_opcode_info
    -- -----------------------------------------------------------------------

    describe("get_opcode_info", function()
        it("returns table with name and operands for known opcode", function()
            local info = op.get_opcode_info(0x6a)
            assert.is_table(info)
            assert.equals("i32.add", info.name)
            assert.equals("none", info.operands)
        end)

        it("returns table for call with operands description", function()
            local info = op.get_opcode_info(0x10)
            assert.is_table(info)
            assert.equals("call", info.name)
            assert.is_string(info.operands)
        end)

        it("returns table for load with memarg operands", function()
            local info = op.get_opcode_info(0x28)
            assert.is_table(info)
            assert.equals("i32.load", info.name)
            assert.is_string(info.operands)
            -- Should mention memarg or align
            assert.is_true(info.operands:find("memarg") ~= nil or info.operands:find("align") ~= nil)
        end)

        it("returns nil for unrecognized byte", function()
            assert.is_nil(op.get_opcode_info(0x99))
            assert.is_nil(op.get_opcode_info(0xff))
        end)

        it("every entry in OPCODES has name and operands fields", function()
            for byte, info in pairs(op.OPCODES) do
                assert.is_string(info.name,
                    string.format("OPCODES[0x%02x].name should be a string", byte))
                assert.is_string(info.operands,
                    string.format("OPCODES[0x%02x].operands should be a string", byte))
            end
        end)

        it("all names are non-empty", function()
            for byte, info in pairs(op.OPCODES) do
                assert.is_true(#info.name > 0,
                    string.format("OPCODES[0x%02x].name is empty", byte))
            end
        end)
    end)

end)

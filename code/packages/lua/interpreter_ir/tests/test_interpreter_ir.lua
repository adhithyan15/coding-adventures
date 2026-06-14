package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local ir = require("coding_adventures.interpreter_ir")

describe("interpreter_ir", function()
    it("records feedback slot polymorphism", function()
        local slot = ir.SlotState.new()
        slot:record("u8"):record("str")
        assert.equals(ir.SlotKind.Polymorphic, slot.kind)
        assert.is_true(slot:is_polymorphic())
    end)

    it("validates functions and branch labels", function()
        local fn = ir.IirFunction.new({
            name = "main",
            return_type = ir.Types.U8,
            instructions = {
                ir.IirInstr.of("const", { dest = "x", srcs = { 1 }, type_hint = ir.Types.U8 }),
                ir.IirInstr.of("label", { srcs = { "done" } }),
                ir.IirInstr.of("ret", { srcs = { "x" } }),
            },
        })
        local mod = ir.IirModule.new({ name = "test", functions = { fn }, entry_point = "main" })
        assert.has_no.errors(function() mod:validate() end)
        assert.equals("fully_typed", fn.type_status)
    end)
end)

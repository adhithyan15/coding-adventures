package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local ir = require("coding_adventures.interpreter_ir")
local vm_core = require("coding_adventures.vm_core")

describe("vm_core", function()
    it("executes arithmetic and returns a value", function()
        local fn = ir.IirFunction.new({
            name = "main",
            return_type = ir.Types.U64,
            instructions = {
                ir.IirInstr.of("const", { dest = "a", srcs = { 20 }, type_hint = ir.Types.U64 }),
                ir.IirInstr.of("const", { dest = "b", srcs = { 22 }, type_hint = ir.Types.U64 }),
                ir.IirInstr.of("add", { dest = "c", srcs = { "a", "b" }, type_hint = ir.Types.U64 }),
                ir.IirInstr.of("ret", { srcs = { "c" } }),
            },
        })
        local mod = ir.IirModule.new({ name = "arith", functions = { fn }, entry_point = "main" })
        assert.equals(42, vm_core.VMCore.new():execute(mod))
    end)

    it("writes byte output", function()
        local fn = ir.IirFunction.new({
            name = "main",
            return_type = ir.Types.Void,
            instructions = {
                ir.IirInstr.of("const", { dest = "x", srcs = { 65 }, type_hint = ir.Types.U8 }),
                ir.IirInstr.of("io_out", { srcs = { "x" } }),
                ir.IirInstr.of("ret_void"),
            },
        })
        local mod = ir.IirModule.new({ name = "io", functions = { fn }, entry_point = "main" })
        local vm = vm_core.VMCore.new()
        vm:execute(mod)
        assert.equals("A", vm.output)
    end)
end)

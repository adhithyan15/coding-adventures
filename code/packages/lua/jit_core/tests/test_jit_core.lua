package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../../vm_core/src/?.lua;" .. "../../vm_core/src/?/init.lua;" ..
    "../../codegen_core/src/?.lua;" .. "../../codegen_core/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local ir = require("coding_adventures.interpreter_ir")
local vm_core = require("coding_adventures.vm_core")
local jit = require("coding_adventures.jit_core")

describe("jit_core", function()
    it("compiles fully typed functions to the pure VM backend", function()
        local fn = ir.IirFunction.new({
            name = "main",
            return_type = ir.Types.U8,
            instructions = {
                ir.IirInstr.of("const", { dest = "x", srcs = { 7 }, type_hint = ir.Types.U8 }),
                ir.IirInstr.of("ret", { srcs = { "x" } }),
            },
        })
        local mod = ir.IirModule.new({ name = "jit", functions = { fn }, entry_point = "main" })
        assert.equals(7, jit.JITCore.new(vm_core.VMCore.new({ u8_wrap = true })):execute_with_jit(mod))
    end)
end)

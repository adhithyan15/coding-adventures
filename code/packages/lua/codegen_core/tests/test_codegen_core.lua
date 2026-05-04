package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local ir = require("coding_adventures.interpreter_ir")
local codegen = require("coding_adventures.codegen_core")

describe("codegen_core", function()
    it("emits LANG text for registered targets", function()
        local fn = ir.IirFunction.new({
            name = "main",
            instructions = { ir.IirInstr.of("ret_void") },
            return_type = ir.Types.Void,
        })
        local mod = ir.IirModule.new({ name = "demo", functions = { fn }, entry_point = "main", language = "demo" })
        local artifact = codegen.BackendRegistry.default():compile(mod, "wasm")
        assert.equals("wasm", artifact.target)
        assert.is_truthy(artifact.body:find("%.function main"))
    end)
end)

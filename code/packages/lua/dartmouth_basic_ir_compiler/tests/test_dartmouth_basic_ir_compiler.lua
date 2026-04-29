package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../../vm_core/src/?.lua;" .. "../../vm_core/src/?/init.lua;" ..
    "../../codegen_core/src/?.lua;" .. "../../codegen_core/src/?/init.lua;" ..
    "../../jit_core/src/?.lua;" .. "../../jit_core/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local basic = require("coding_adventures.dartmouth_basic_ir_compiler")

describe("dartmouth_basic_ir_compiler", function()
    it("runs BASIC print and arithmetic through the LANG VM", function()
        assert.equals("42\n", basic.run_dartmouth_basic("10 LET A = 40\n20 PRINT A + 2\n30 END"))
    end)

    it("emits target artifacts", function()
        local artifact = basic.emit_dartmouth_basic("10 END", "clr")
        assert.equals("clr", artifact.target)
        assert.is_truthy(artifact.body:find("dartmouth%-basic"))
    end)
end)

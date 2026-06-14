package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../../vm_core/src/?.lua;" .. "../../vm_core/src/?/init.lua;" ..
    "../../codegen_core/src/?.lua;" .. "../../codegen_core/src/?/init.lua;" ..
    "../../jit_core/src/?.lua;" .. "../../jit_core/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local lang_vm = require("coding_adventures.brainfuck.lang_vm")

describe("brainfuck LANG VM", function()
    it("executes output through the shared VM", function()
        local result = lang_vm.execute_on_lang_vm("+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++.")
        assert.equals("A", result.output)
    end)

    it("supports loops", function()
        local result = lang_vm.execute_on_lang_vm("+++[>+++++++++++++++++++++<-]>++.")
        assert.equals("A", result.output)
    end)
end)

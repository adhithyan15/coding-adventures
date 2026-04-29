package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../../vm_core/src/?.lua;" .. "../../vm_core/src/?/init.lua;" ..
    "../../codegen_core/src/?.lua;" .. "../../codegen_core/src/?/init.lua;" ..
    "../../jit_core/src/?.lua;" .. "../../jit_core/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local twig = require("coding_adventures.twig")

describe("twig", function()
    it("runs arithmetic and function calls on the LANG VM", function()
        local _, value = twig.run_twig("(define (inc x) (+ x 1)) (inc 41)")
        assert.equals(42, value)
    end)

    it("captures print output", function()
        local stdout, value = twig.run_twig("(print (+ 1 2))")
        assert.equals("3\n", stdout)
        assert.is_nil(value)
    end)
end)

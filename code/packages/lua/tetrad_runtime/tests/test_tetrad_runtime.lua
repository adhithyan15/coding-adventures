package.path = "../../interpreter_ir/src/?.lua;" .. "../../interpreter_ir/src/?/init.lua;" ..
    "../../vm_core/src/?.lua;" .. "../../vm_core/src/?/init.lua;" ..
    "../../codegen_core/src/?.lua;" .. "../../codegen_core/src/?/init.lua;" ..
    "../../jit_core/src/?.lua;" .. "../../jit_core/src/?/init.lua;" ..
    "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local tetrad = require("coding_adventures.tetrad_runtime")

describe("tetrad_runtime", function()
    it("runs top-level arithmetic through the LANG VM", function()
        assert.equals(42, tetrad.run_tetrad("let x = 40; return x + 2;"))
    end)

    it("emits text for backend targets", function()
        local artifact = tetrad.emit_tetrad("return 7;", "wasm")
        assert.equals("wasm", artifact.target)
        assert.is_truthy(artifact.body:find("language=tetrad"))
    end)
end)

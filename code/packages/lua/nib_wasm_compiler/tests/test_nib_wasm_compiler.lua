package.path = "../../nib_lexer/src/?.lua;" ..
    "../../nib_lexer/src/?/init.lua;" ..
    "../../nib_parser/src/?.lua;" ..
    "../../nib_parser/src/?/init.lua;" ..
    "../../nib_type_checker/src/?.lua;" ..
    "../../nib_type_checker/src/?/init.lua;" ..
    "../../nib_ir_compiler/src/?.lua;" ..
    "../../nib_ir_compiler/src/?/init.lua;" ..
    "../../ir_to_wasm_compiler/src/?.lua;" ..
    "../../ir_to_wasm_compiler/src/?/init.lua;" ..
    "../../ir_to_wasm_validator/src/?.lua;" ..
    "../../ir_to_wasm_validator/src/?/init.lua;" ..
    "../../wasm_module_encoder/src/?.lua;" ..
    "../../wasm_module_encoder/src/?/init.lua;" ..
    "../../wasm_validator/src/?.lua;" ..
    "../../wasm_validator/src/?/init.lua;" ..
    "../../wasm_execution/src/?.lua;" ..
    "../../wasm_execution/src/?/init.lua;" ..
    "../../wasm_runtime/src/?.lua;" ..
    "../../wasm_runtime/src/?/init.lua;" ..
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    package.path

local compiler = require("coding_adventures.nib_wasm_compiler")
local wasm_runtime = require("coding_adventures.wasm_runtime")

describe("nib_wasm_compiler", function()
    it("returns pipeline artifacts for compiled source", function()
        local result, err = compiler.compile_source("fn answer() -> u4 { return 7; }")
        assert.is_nil(err)
        assert.is_true(#result.binary > 0)
        assert.is_true(#(result.raw_ir.instructions or {}) > 0)
    end)

    it("aliases pack_source to compile_source", function()
        local compiled = assert(compiler.compile_source("fn answer() -> u4 { return 7; }"))
        local packed = assert(compiler.pack_source("fn answer() -> u4 { return 7; }"))
        assert.are.equal(compiled.binary, packed.binary)
    end)

    it("writes the encoded module to disk", function()
        local output_path = os.tmpname() .. ".wasm"
        local result, err = compiler.write_wasm_file("fn answer() -> u4 { return 7; }", output_path)
        assert.is_nil(err)
        local file = assert(io.open(output_path, "rb"))
        local contents = file:read("*a")
        file:close()
        os.remove(output_path)
        assert.are.equal(result.binary, contents)
    end)

    it("runs the _start path through the wasm runtime", function()
        local result, err = compiler.compile_source([[
            fn add(a: u4, b: u4) -> u4 { return a +% b; }
            fn main() -> u4 { return add(3, 4); }
        ]])
        assert.is_nil(err)
        local runtime = wasm_runtime.WasmRuntime.new()
        local execution_result = runtime:load_and_run(result.binary, "_start", {})
        assert.are.same({ 7 }, execution_result)
    end)

    it("runs an exported loop function through the wasm runtime", function()
        local result, err = compiler.compile_source([[
            fn count_to(n: u4) -> u4 {
                let acc: u4 = 0;
                for i: u4 in 0..n {
                    acc = acc +% 1;
                }
                return acc;
            }
        ]])
        assert.is_nil(err)
        local runtime = wasm_runtime.WasmRuntime.new()
        local execution_result = runtime:load_and_run(result.binary, "count_to", { 5 })
        assert.are.same({ 5 }, execution_result)
    end)

    it("reports type errors as package errors", function()
        local result, err = compiler.compile_source("fn main() { let x: bool = 1 +% 2; }")
        assert.is_nil(result)
        assert.are.equal("type-check", err.stage)
    end)
end)

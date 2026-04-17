package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../brainfuck/src/?.lua;" .. "../../brainfuck/src/?/init.lua;" .. package.path
package.path = "../../brainfuck_ir_compiler/src/?.lua;" .. "../../brainfuck_ir_compiler/src/?/init.lua;" .. package.path
package.path = "../../compiler_ir/src/?.lua;" .. "../../compiler_ir/src/?/init.lua;" .. package.path
package.path = "../../compiler_source_map/src/?.lua;" .. "../../compiler_source_map/src/?/init.lua;" .. package.path
package.path = "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;" .. package.path
package.path = "../../state_machine/src/?.lua;" .. "../../state_machine/src/?/init.lua;" .. package.path
package.path = "../../grammar_tools/src/?.lua;" .. "../../grammar_tools/src/?/init.lua;" .. package.path
package.path = "../../lexer/src/?.lua;" .. "../../lexer/src/?/init.lua;" .. package.path
package.path = "../../parser/src/?.lua;" .. "../../parser/src/?/init.lua;" .. package.path
package.path = "../../ir_to_wasm_compiler/src/?.lua;" .. "../../ir_to_wasm_compiler/src/?/init.lua;" .. package.path
package.path = "../../ir_to_wasm_validator/src/?.lua;" .. "../../ir_to_wasm_validator/src/?/init.lua;" .. package.path
package.path = "../../wasm_module_encoder/src/?.lua;" .. "../../wasm_module_encoder/src/?/init.lua;" .. package.path
package.path = "../../wasm_runtime/src/?.lua;" .. "../../wasm_runtime/src/?/init.lua;" .. package.path
package.path = "../../wasm_validator/src/?.lua;" .. "../../wasm_validator/src/?/init.lua;" .. package.path
package.path = "../../wasm_module_parser/src/?.lua;" .. "../../wasm_module_parser/src/?/init.lua;" .. package.path
package.path = "../../wasm_execution/src/?.lua;" .. "../../wasm_execution/src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../virtual_machine/src/?.lua;" .. "../../virtual_machine/src/?/init.lua;" .. package.path

local compiler = require("coding_adventures.brainfuck_wasm_compiler")
local wasm_runtime = require("coding_adventures.wasm_runtime")

local function run(binary, stdin_text)
    local output = {}
    local offset = 1
    local host = wasm_runtime.WasiHost.new({
        stdin = function(count)
            if offset > #stdin_text then
                return ""
            end
            local chunk = stdin_text:sub(offset, offset + count - 1)
            offset = offset + #chunk
            return chunk
        end,
        stdout = function(text)
            output[#output + 1] = text
        end,
    })

    local runtime = wasm_runtime.WasmRuntime.new(host)
    local result = runtime:load_and_run(binary, "_start", {})
    return result, output
end

describe("brainfuck_wasm_compiler", function()
    it("returns pipeline artifacts for compiled Brainfuck source", function()
        local result, err = compiler.compile_source("+.")

        assert.is_nil(err)
        assert.are.equal("program.bf", result.filename)
        assert.is_true(#(result.raw_ir.instructions or {}) > 0)
        assert.is_true(#(result.optimized_ir.instructions or {}) > 0)
        assert.is_true(#result.binary > 0)
        assert.are.equal("_start", result.module.exports[2].name)
    end)

    it("aliases pack_source to compile_source", function()
        local compiled, compiled_err = compiler.compile_source("+.")
        local packed, packed_err = compiler.pack_source("+.")

        assert.is_nil(compiled_err)
        assert.is_nil(packed_err)
        assert.are.equal(compiled.binary, packed.binary)
    end)

    it("writes the encoded module to disk", function()
        local output_path = os.tmpname() .. ".wasm"
        local result, err = compiler.write_wasm_file("+.", output_path)

        assert.is_nil(err)

        local file = assert(io.open(output_path, "rb"))
        local contents = file:read("*a")
        file:close()

        os.remove(output_path)

        assert.are.equal(result.binary, contents)
        assert.are.equal(output_path, result.wasm_path)
    end)

    it("runs compiled output programs in the WASM runtime", function()
        local result, err = compiler.compile_source(string.rep("+", 65) .. ".")
        assert.is_nil(err)

        local execution_result, output = run(result.binary, "")

        assert.are.same({ 0 }, execution_result)
        assert.are.same({ "A" }, output)
    end)

    it("runs compiled input programs in the WASM runtime", function()
        local result, err = compiler.compile_source(",.")
        assert.is_nil(err)

        local execution_result, output = run(result.binary, "Z")

        assert.are.same({ 0 }, execution_result)
        assert.are.same({ "Z" }, output)
    end)

    it("runs compiled cat programs in the WASM runtime", function()
        local result, err = compiler.compile_source(",[.,]")
        assert.is_nil(err)

        local execution_result, output = run(result.binary, "Hi")

        assert.are.same({ 0 }, execution_result)
        assert.are.same({ "H", "i" }, output)
    end)

    it("honors a custom filename", function()
        local instance = compiler.BrainfuckWasmCompiler.new({
            filename = "hello.bf",
        })
        local result, err = instance:compile_source("+")

        assert.is_nil(err)
        assert.are.equal("hello.bf", result.filename)
    end)
end)

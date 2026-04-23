package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../compiler_ir/src/?.lua;" .. "../../compiler_ir/src/?/init.lua;" .. package.path
package.path = "../../compiler_source_map/src/?.lua;" .. "../../compiler_source_map/src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../grammar_tools/src/?.lua;" .. "../../grammar_tools/src/?/init.lua;" .. package.path
package.path = "../../lexer/src/?.lua;" .. "../../lexer/src/?/init.lua;" .. package.path
package.path = "../../parser/src/?.lua;" .. "../../parser/src/?/init.lua;" .. package.path
package.path = "../../brainfuck/src/?.lua;" .. "../../brainfuck/src/?/init.lua;" .. package.path
package.path = "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;" .. package.path
package.path = "../../state_machine/src/?.lua;" .. "../../state_machine/src/?/init.lua;" .. package.path
package.path = "../../brainfuck_ir_compiler/src/?.lua;" .. "../../brainfuck_ir_compiler/src/?/init.lua;" .. package.path

local bf_parser = require("coding_adventures.brainfuck.parser")
local brainfuck_ir_compiler = require("coding_adventures.brainfuck_ir_compiler")
local compiler = require("coding_adventures.ir_to_wasm_compiler")
local ir = require("coding_adventures.compiler_ir")

local function compile_brainfuck(source)
    local ast = bf_parser.parse(source)
    local result, err = brainfuck_ir_compiler.compile(ast, "echo.bf", brainfuck_ir_compiler.release_config())
    assert.is_nil(err)
    return result.program
end

describe("ir_to_wasm_compiler", function()
    it("lowers Brainfuck IR into a memory-backed module with WASI IO imports", function()
        local module, err = compiler.compile(compile_brainfuck(",."))

        assert.is_nil(err)
        assert.are.equal(1, #module.memories)
        assert.are.equal("memory", module.exports[1].name)
        assert.are.equal("_start", module.exports[2].name)
        assert.are.equal(2, #module.imports)
        assert.are.equal("fd_write", module.imports[1].name)
        assert.are.equal("fd_read", module.imports[2].name)
    end)

    it("infers exported function signatures from comment metadata", function()
        local program = ir.new_program("_fn_add")
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.COMMENT, {
            ir.new_label("function: add(x, y)")
        }, -1))
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.LABEL, {
            ir.new_label("_fn_add")
        }, -1))
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.RET, {}, 0))

        local module, err = compiler.compile(program)

        assert.is_nil(err)
        assert.are.equal(1, #module.exports)
        assert.are.equal("add", module.exports[1].name)
        assert.are.equal(2, #module.types[1].params)
    end)

    it("rejects unsupported syscalls", function()
        local program = ir.new_program("_start")
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.LABEL, {
            ir.new_label("_start")
        }, -1))
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.SYSCALL, {
            ir.new_immediate(99)
        }, 0))

        local module, err = compiler.compile(program)

        assert.is_nil(module)
        assert.matches("unsupported SYSCALL number", err)
    end)
end)

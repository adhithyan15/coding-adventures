package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../compiler_ir/src/?.lua;" .. "../../compiler_ir/src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../ir_to_wasm_compiler/src/?.lua;" .. "../../ir_to_wasm_compiler/src/?/init.lua;" .. package.path

local ir = require("coding_adventures.compiler_ir")
local validator = require("coding_adventures.ir_to_wasm_validator")

describe("ir_to_wasm_validator", function()
    it("returns no errors for a valid _start program", function()
        local program = ir.new_program("_start")
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.LABEL, {
            ir.new_label("_start")
        }, -1))
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.HALT, {}, 0))

        local errors = validator.validate(program)

        assert.are.same({}, errors)
    end)

    it("surfaces lowering errors", function()
        local program = ir.new_program("_start")
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.LABEL, {
            ir.new_label("_start")
        }, -1))
        ir.add_instruction(program, ir.new_instruction(ir.IrOp.SYSCALL, {
            ir.new_immediate(99)
        }, 0))

        local errors = validator.validate(program)

        assert.are.equal(1, #errors)
        assert.are.equal("lowering", errors[1].rule)
        assert.matches("unsupported SYSCALL number", errors[1].message)
    end)
end)

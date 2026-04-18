package.path = "../../nib_lexer/src/?.lua;" ..
    "../../nib_lexer/src/?/init.lua;" ..
    "../../nib_parser/src/?.lua;" ..
    "../../nib_parser/src/?/init.lua;" ..
    "../../nib_type_checker/src/?.lua;" ..
    "../../nib_type_checker/src/?/init.lua;" ..
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    package.path

local nib_parser = require("coding_adventures.nib_parser")
local nib_type_checker = require("coding_adventures.nib_type_checker")
local compiler = require("coding_adventures.nib_ir_compiler")
local ir = require("coding_adventures.compiler_ir")

local function compile_source(source)
    local ast = nib_parser.parse(source)
    local typed = nib_type_checker.check(ast)
    assert.is_true(typed.ok)
    return compiler.compile_nib(typed.typed_ast).program
end

describe("nib_ir_compiler", function()
    it("emits entrypoint and halt", function()
        local program = compile_source("fn main() -> u4 { return 7; }")
        local opcodes = {}
        for _, instr in ipairs(program.instructions) do
            opcodes[#opcodes + 1] = instr.opcode
        end
        assert.is_true(#opcodes > 0)
        assert.is_true(opcodes[1] == ir.IrOp.LABEL)
    end)

    it("emits call and add shapes", function()
        local program = compile_source("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }")
        local saw_call = false
        local saw_add = false
        for _, instr in ipairs(program.instructions) do
            saw_call = saw_call or instr.opcode == ir.IrOp.CALL
            saw_add = saw_add or instr.opcode == ir.IrOp.ADD or instr.opcode == ir.IrOp.ADD_IMM
        end
        assert.is_true(saw_call)
        assert.is_true(saw_add)
    end)

    it("emits loop control flow", function()
        local program = compile_source([[
            fn count_to(n: u4) -> u4 {
                let acc: u4 = 0;
                for i: u4 in 0..n {
                    acc = acc +% 1;
                }
                return acc;
            }
        ]])
        local saw_branch = false
        local saw_jump = false
        for _, instr in ipairs(program.instructions) do
            saw_branch = saw_branch or instr.opcode == ir.IrOp.BRANCH_Z
            saw_jump = saw_jump or instr.opcode == ir.IrOp.JUMP
        end
        assert.is_true(saw_branch)
        assert.is_true(saw_jump)
    end)
end)

-- Tests for bytecode_compiler
-- ============================
--
-- Comprehensive busted tests covering all three compilers:
--   1. BytecodeCompiler — the hardcoded AST-to-bytecode compiler
--   2. JVMCompiler — the JVM-style bytecode compiler
--   3. GenericCompiler — the pluggable compilation framework
--
-- Also tests supporting types: CompilerScope, ASTNode, TokenNode,
-- Instruction, CodeObject, and JVMCodeObject.

-- Add all transitive dependency paths so requires resolve correctly.
package.path = "../src/?.lua;" .. "../src/?/init.lua;"
    .. "../../parser/src/?.lua;" .. "../../parser/src/?/init.lua;"
    .. "../../virtual_machine/src/?.lua;" .. "../../virtual_machine/src/?/init.lua;"
    .. "../../lexer/src/?.lua;" .. "../../lexer/src/?/init.lua;"
    .. "../../grammar_tools/src/?.lua;" .. "../../grammar_tools/src/?/init.lua;"
    .. "../../state_machine/src/?.lua;" .. "../../state_machine/src/?/init.lua;"
    .. "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;"
    .. package.path

local bc = require("coding_adventures.bytecode_compiler")


-- =========================================================================
-- Helpers
-- =========================================================================

--- Extract just the opcodes from an array of instructions.
local function opcodes(instructions)
    local result = {}
    for i, instr in ipairs(instructions) do
        result[i] = instr.opcode
    end
    return result
end

--- Deep comparison of two tables (arrays or maps).
local function deep_equal(a, b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    -- Check all keys in a exist in b with equal values.
    for k, v in pairs(a) do
        if not deep_equal(v, b[k]) then return false end
    end
    -- Check all keys in b exist in a.
    for k, _ in pairs(b) do
        if a[k] == nil then return false end
    end
    return true
end


-- =========================================================================
-- Module-level basics
-- =========================================================================

describe("bytecode_compiler module", function()
    it("has a version", function()
        assert.are.equal("0.1.0", bc.VERSION)
    end)

    it("exports all OpCode constants", function()
        assert.are.equal(0x01, bc.OpLoadConst)
        assert.are.equal(0x02, bc.OpPop)
        assert.are.equal(0x03, bc.OpDup)
        assert.are.equal(0x10, bc.OpStoreName)
        assert.are.equal(0x11, bc.OpLoadName)
        assert.are.equal(0x12, bc.OpStoreLocal)
        assert.are.equal(0x13, bc.OpLoadLocal)
        assert.are.equal(0x20, bc.OpAdd)
        assert.are.equal(0x21, bc.OpSub)
        assert.are.equal(0x22, bc.OpMul)
        assert.are.equal(0x23, bc.OpDiv)
        assert.are.equal(0x30, bc.OpCmpEq)
        assert.are.equal(0x31, bc.OpCmpLt)
        assert.are.equal(0x32, bc.OpCmpGt)
        assert.are.equal(0x40, bc.OpJump)
        assert.are.equal(0x41, bc.OpJumpIfFalse)
        assert.are.equal(0x42, bc.OpJumpIfTrue)
        assert.are.equal(0x50, bc.OpCall)
        assert.are.equal(0x51, bc.OpReturn)
        assert.are.equal(0x60, bc.OpPrint)
        assert.are.equal(0xFF, bc.OpHalt)
    end)

    it("exports all JVM bytecode constants", function()
        assert.are.equal(0x03, bc.ICONST_0)
        assert.are.equal(0x04, bc.ICONST_1)
        assert.are.equal(0x05, bc.ICONST_2)
        assert.are.equal(0x06, bc.ICONST_3)
        assert.are.equal(0x07, bc.ICONST_4)
        assert.are.equal(0x08, bc.ICONST_5)
        assert.are.equal(0x10, bc.BIPUSH)
        assert.are.equal(0x12, bc.LDC)
        assert.are.equal(0x15, bc.ILOAD)
        assert.are.equal(0x1A, bc.ILOAD_0)
        assert.are.equal(0x1B, bc.ILOAD_1)
        assert.are.equal(0x1C, bc.ILOAD_2)
        assert.are.equal(0x1D, bc.ILOAD_3)
        assert.are.equal(0x36, bc.ISTORE)
        assert.are.equal(0x3B, bc.ISTORE_0)
        assert.are.equal(0x3C, bc.ISTORE_1)
        assert.are.equal(0x3D, bc.ISTORE_2)
        assert.are.equal(0x3E, bc.ISTORE_3)
        assert.are.equal(0x57, bc.JVM_POP)
        assert.are.equal(0x60, bc.IADD)
        assert.are.equal(0x64, bc.ISUB)
        assert.are.equal(0x68, bc.IMUL)
        assert.are.equal(0x6C, bc.IDIV)
        assert.are.equal(0xB1, bc.RETURN)
    end)
end)


-- =========================================================================
-- Instruction and CodeObject constructors
-- =========================================================================

describe("Instruction", function()
    it("creates an instruction with opcode only", function()
        local instr = bc.Instruction(bc.OpAdd)
        assert.are.equal(bc.OpAdd, instr.opcode)
        assert.is_nil(instr.operand)
    end)

    it("creates an instruction with opcode and operand", function()
        local instr = bc.Instruction(bc.OpLoadConst, 5)
        assert.are.equal(bc.OpLoadConst, instr.opcode)
        assert.are.equal(5, instr.operand)
    end)

    it("supports string operands", function()
        local instr = bc.Instruction(bc.OpLoadConst, "hello")
        assert.are.equal("hello", instr.operand)
    end)
end)

describe("CodeObject", function()
    it("creates a code object with defaults", function()
        local code = bc.CodeObject()
        assert.are.same({}, code.instructions)
        assert.are.same({}, code.constants)
        assert.are.same({}, code.names)
    end)

    it("creates a code object with provided values", function()
        local instrs = { bc.Instruction(bc.OpHalt) }
        local consts = { 42 }
        local names = { "x" }
        local code = bc.CodeObject(instrs, consts, names)
        assert.are.equal(1, #code.instructions)
        assert.are.equal(42, code.constants[1])
        assert.are.equal("x", code.names[1])
    end)
end)

describe("JVMCodeObject", function()
    it("creates a JVM code object with defaults", function()
        local code = bc.JVMCodeObject()
        assert.are.same({}, code.bytecode)
        assert.are.same({}, code.constants)
        assert.are.equal(0, code.num_locals)
        assert.are.same({}, code.local_names)
    end)

    it("creates a JVM code object with provided values", function()
        local code = bc.JVMCodeObject({ 0x03, 0xB1 }, { 42 }, 2, { "x", "y" })
        assert.are.equal(2, #code.bytecode)
        assert.are.equal(42, code.constants[1])
        assert.are.equal(2, code.num_locals)
        assert.are.equal("y", code.local_names[2])
    end)
end)


-- =========================================================================
-- AST Node constructors
-- =========================================================================

describe("AST node constructors", function()
    it("creates a Program node", function()
        local prog = bc.Program({ "stmt1" })
        assert.are.equal("Program", prog.type)
        assert.are.equal(1, #prog.statements)
    end)

    it("creates a Program with default empty statements", function()
        local prog = bc.Program()
        assert.are.same({}, prog.statements)
    end)

    it("creates an Assignment node", function()
        local target = bc.Name("x")
        local value = bc.NumberLiteral(42)
        local assign = bc.Assignment(target, value)
        assert.are.equal("Assignment", assign.type)
        assert.are.equal("x", assign.target.name)
        assert.are.equal(42, assign.value.value)
    end)

    it("creates an ExpressionStmt node", function()
        local expr = bc.NumberLiteral(1)
        local stmt = bc.ExpressionStmt(expr)
        assert.are.equal("ExpressionStmt", stmt.type)
        assert.are.equal(1, stmt.expression.value)
    end)

    it("creates a NumberLiteral node", function()
        local num = bc.NumberLiteral(3.14)
        assert.are.equal("NumberLiteral", num.type)
        assert.are.equal(3.14, num.value)
    end)

    it("creates a StringLiteral node", function()
        local str = bc.StringLiteral("hello")
        assert.are.equal("StringLiteral", str.type)
        assert.are.equal("hello", str.value)
    end)

    it("creates a Name node", function()
        local name = bc.Name("counter")
        assert.are.equal("Name", name.type)
        assert.are.equal("counter", name.name)
    end)

    it("creates a BinaryOp node", function()
        local left = bc.NumberLiteral(1)
        local right = bc.NumberLiteral(2)
        local binop = bc.BinaryOp(left, "+", right)
        assert.are.equal("BinaryOp", binop.type)
        assert.are.equal("+", binop.op)
        assert.are.equal(1, binop.left.value)
        assert.are.equal(2, binop.right.value)
    end)
end)

describe("GenericCompiler AST node constructors", function()
    it("creates an ASTNode", function()
        local node = bc.ASTNode("expression", {})
        assert.are.equal("ast", node.node_kind)
        assert.are.equal("expression", node.rule_name)
        assert.are.same({}, node.children)
    end)

    it("creates an ASTNode with default empty children", function()
        local node = bc.ASTNode("rule")
        assert.are.same({}, node.children)
    end)

    it("creates a TokenNode", function()
        local token = bc.TokenNode("NUMBER", "42")
        assert.are.equal("token", token.node_kind)
        assert.are.equal("NUMBER", token.token_type)
        assert.are.equal("42", token.value)
    end)
end)


-- =========================================================================
-- BytecodeCompiler
-- =========================================================================

describe("BytecodeCompiler", function()
    it("compiles a simple number literal expression statement", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(42))
        })

        local code = compiler:compile(prog)

        assert.are.equal(3, #code.instructions)
        assert.are.equal(bc.OpLoadConst, code.instructions[1].opcode)
        assert.are.equal(0, code.instructions[1].operand)
        assert.are.equal(bc.OpPop, code.instructions[2].opcode)
        assert.are.equal(bc.OpHalt, code.instructions[3].opcode)
        assert.are.same({ 42 }, code.constants)
    end)

    it("compiles variable assignment with arithmetic", function()
        -- x = 1 + 2
        local prog = bc.Program({
            bc.Assignment(
                bc.Name("x"),
                bc.BinaryOp(
                    bc.NumberLiteral(1),
                    "+",
                    bc.NumberLiteral(2)
                )
            )
        })

        local compiler = bc.BytecodeCompiler.new()
        local code = compiler:compile(prog)

        local expected_opcodes = {
            bc.OpLoadConst,  -- push 1
            bc.OpLoadConst,  -- push 2
            bc.OpAdd,        -- 1 + 2
            bc.OpStoreName,  -- store in x
            bc.OpHalt,
        }
        assert.are.same(expected_opcodes, opcodes(code.instructions))

        -- Verify operands.
        assert.are.equal(0, code.instructions[1].operand) -- const index 0 = 1
        assert.are.equal(1, code.instructions[2].operand) -- const index 1 = 2
        assert.are.equal(0, code.instructions[4].operand) -- name index 0 = "x"

        -- Verify pools.
        assert.are.same({ 1, 2 }, code.constants)
        assert.are.same({ "x" }, code.names)
    end)

    it("compiles string literals", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.StringLiteral("hello"))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.OpLoadConst, code.instructions[1].opcode)
        assert.are.same({ "hello" }, code.constants)
    end)

    it("compiles variable references", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.Name("x"))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.OpLoadName, code.instructions[1].opcode)
        assert.are.equal(0, code.instructions[1].operand)
        assert.are.same({ "x" }, code.names)
    end)

    it("compiles subtraction", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(10), "-", bc.NumberLiteral(3)))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.OpSub, code.instructions[3].opcode)
    end)

    it("compiles multiplication", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(4), "*", bc.NumberLiteral(5)))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.OpMul, code.instructions[3].opcode)
    end)

    it("compiles division", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(20), "/", bc.NumberLiteral(4)))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.OpDiv, code.instructions[3].opcode)
    end)

    it("deduplicates constants", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(5), "+", bc.NumberLiteral(5)))
        })

        local code = compiler:compile(prog)

        -- 5 appears twice in the source but only once in the constant pool.
        assert.are.same({ 5 }, code.constants)
        assert.are.equal(0, code.instructions[1].operand)
        assert.are.equal(0, code.instructions[2].operand)
    end)

    it("deduplicates names", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(1)),
            bc.ExpressionStmt(bc.Name("x")),
        })

        local code = compiler:compile(prog)

        -- "x" appears in assignment and reference but only once in names pool.
        assert.are.same({ "x" }, code.names)
    end)

    it("errors on unknown statement type", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            { type = "UnknownStmt" }
        })

        assert.has_error(function()
            compiler:compile(prog)
        end, "Unknown statement type: UnknownStmt")
    end)

    it("errors on unknown expression type", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt({ type = "UnknownExpr" })
        })

        assert.has_error(function()
            compiler:compile(prog)
        end, "Unknown expression type: UnknownExpr")
    end)

    it("errors on unknown operator", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(1), "^", bc.NumberLiteral(2)))
        })

        assert.has_error(function()
            compiler:compile(prog)
        end, "Unknown operator: ^")
    end)

    it("compiles multiple statements", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(10)),
            bc.Assignment(bc.Name("y"), bc.NumberLiteral(20)),
            bc.ExpressionStmt(bc.BinaryOp(bc.Name("x"), "+", bc.Name("y"))),
        })

        local code = compiler:compile(prog)

        -- x=10: LOAD_CONST, STORE_NAME
        -- y=20: LOAD_CONST, STORE_NAME
        -- x+y:  LOAD_NAME, LOAD_NAME, ADD, POP
        -- HALT
        assert.are.equal(9, #code.instructions)
        assert.are.equal(bc.OpHalt, code.instructions[9].opcode)
    end)

    it("compiles nested binary operations", function()
        -- (1 + 2) * 3
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(
                bc.BinaryOp(
                    bc.BinaryOp(bc.NumberLiteral(1), "+", bc.NumberLiteral(2)),
                    "*",
                    bc.NumberLiteral(3)
                )
            )
        })

        local code = compiler:compile(prog)

        local expected = {
            bc.OpLoadConst,  -- 1
            bc.OpLoadConst,  -- 2
            bc.OpAdd,        -- 1+2
            bc.OpLoadConst,  -- 3
            bc.OpMul,        -- (1+2)*3
            bc.OpPop,
            bc.OpHalt,
        }
        assert.are.same(expected, opcodes(code.instructions))
    end)

    it("has correct OPERATOR_MAP", function()
        assert.are.equal(bc.OpAdd, bc.BytecodeCompiler.OPERATOR_MAP["+"])
        assert.are.equal(bc.OpSub, bc.BytecodeCompiler.OPERATOR_MAP["-"])
        assert.are.equal(bc.OpMul, bc.BytecodeCompiler.OPERATOR_MAP["*"])
        assert.are.equal(bc.OpDiv, bc.BytecodeCompiler.OPERATOR_MAP["/"])
    end)

    it("compiles an empty program to just HALT", function()
        local compiler = bc.BytecodeCompiler.new()
        local prog = bc.Program({})

        local code = compiler:compile(prog)

        assert.are.equal(1, #code.instructions)
        assert.are.equal(bc.OpHalt, code.instructions[1].opcode)
    end)
end)


-- =========================================================================
-- JVMCompiler
-- =========================================================================

describe("JVMCompiler", function()
    it("compiles an empty program to just RETURN", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({})

        local code = compiler:compile(prog)

        assert.are.equal(1, #code.bytecode)
        assert.are.equal(bc.RETURN, code.bytecode[1])
    end)

    it("compiles small integer constants (0-5) with ICONST_n", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(0)),
        })

        local code = compiler:compile(prog)

        -- ICONST_0, POP, RETURN
        assert.are.equal(bc.ICONST_0, code.bytecode[1])
        assert.are.equal(bc.JVM_POP, code.bytecode[2])
    end)

    it("uses ICONST_3 for the value 3", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(3)),
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.ICONST_3, code.bytecode[1])
    end)

    it("uses ICONST_5 for the value 5", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(5)),
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.ICONST_5, code.bytecode[1])
    end)

    it("compiles byte-range integers (6-127) with BIPUSH", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(42)),
        })

        local code = compiler:compile(prog)

        -- BIPUSH, 42, POP, RETURN
        assert.are.equal(bc.BIPUSH, code.bytecode[1])
        assert.are.equal(42, code.bytecode[2])
    end)

    it("compiles negative byte-range integers with BIPUSH", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(-1)),
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.BIPUSH, code.bytecode[1])
        -- -1 & 0xFF = 255
        assert.are.equal(255, code.bytecode[2])
    end)

    it("compiles large integers with LDC", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.NumberLiteral(1000)),
        })

        local code = compiler:compile(prog)

        -- LDC, index, POP, RETURN
        assert.are.equal(bc.LDC, code.bytecode[1])
        assert.are.equal(0, code.bytecode[2])  -- constant pool index 0
        assert.are.same({ 1000 }, code.constants)
    end)

    it("compiles string literals with LDC", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.StringLiteral("hello")),
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.LDC, code.bytecode[1])
        assert.are.same({ "hello" }, code.constants)
    end)

    it("compiles addition with IADD", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(1), "+", bc.NumberLiteral(2)))
        })

        local code = compiler:compile(prog)

        -- ICONST_1, ICONST_2, IADD, POP, RETURN
        assert.are.equal(bc.ICONST_1, code.bytecode[1])
        assert.are.equal(bc.ICONST_2, code.bytecode[2])
        assert.are.equal(bc.IADD, code.bytecode[3])
    end)

    it("compiles subtraction with ISUB", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(3), "-", bc.NumberLiteral(1)))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.ISUB, code.bytecode[3])
    end)

    it("compiles multiplication with IMUL", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(4), "*", bc.NumberLiteral(5)))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.IMUL, code.bytecode[3])
    end)

    it("compiles division with IDIV", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(4), "/", bc.NumberLiteral(2)))
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.IDIV, code.bytecode[3])
    end)

    it("compiles assignment with ISTORE_n for slots 0-3", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(1)),
        })

        local code = compiler:compile(prog)

        -- ICONST_1, ISTORE_0, RETURN
        assert.are.equal(bc.ICONST_1, code.bytecode[1])
        assert.are.equal(bc.ISTORE_0, code.bytecode[2])
        assert.are.same({ "x" }, code.local_names)
        assert.are.equal(1, code.num_locals)
    end)

    it("uses ISTORE_1 for the second variable", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(1)),
            bc.Assignment(bc.Name("y"), bc.NumberLiteral(2)),
        })

        local code = compiler:compile(prog)

        assert.are.equal(bc.ISTORE_0, code.bytecode[2]) -- x in slot 0
        assert.are.equal(bc.ISTORE_1, code.bytecode[4]) -- y in slot 1
        assert.are.equal(2, code.num_locals)
    end)

    it("uses ISTORE for slots beyond 3", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("a"), bc.NumberLiteral(0)),
            bc.Assignment(bc.Name("b"), bc.NumberLiteral(1)),
            bc.Assignment(bc.Name("c"), bc.NumberLiteral(2)),
            bc.Assignment(bc.Name("d"), bc.NumberLiteral(3)),
            bc.Assignment(bc.Name("e"), bc.NumberLiteral(4)),  -- slot 4, needs ISTORE
        })

        local code = compiler:compile(prog)

        -- Last assignment: ICONST_4, ISTORE, 4, RETURN
        local n = #code.bytecode
        assert.are.equal(bc.RETURN, code.bytecode[n])
        assert.are.equal(4, code.bytecode[n - 1])       -- slot 4
        assert.are.equal(bc.ISTORE, code.bytecode[n - 2]) -- ISTORE opcode
        assert.are.equal(5, code.num_locals)
    end)

    it("compiles variable references with ILOAD_n for slots 0-3", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(1)),
            bc.ExpressionStmt(bc.Name("x")),
        })

        local code = compiler:compile(prog)

        -- ICONST_1, ISTORE_0, ILOAD_0, POP, RETURN
        assert.are.equal(bc.ILOAD_0, code.bytecode[3])
    end)

    it("uses ILOAD for slots beyond 3", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("a"), bc.NumberLiteral(0)),
            bc.Assignment(bc.Name("b"), bc.NumberLiteral(1)),
            bc.Assignment(bc.Name("c"), bc.NumberLiteral(2)),
            bc.Assignment(bc.Name("d"), bc.NumberLiteral(3)),
            bc.Assignment(bc.Name("e"), bc.NumberLiteral(4)),
            bc.ExpressionStmt(bc.Name("e")),  -- slot 4, needs ILOAD
        })

        local code = compiler:compile(prog)

        -- Near the end: ILOAD, 4, POP, RETURN
        local n = #code.bytecode
        assert.are.equal(bc.RETURN, code.bytecode[n])
        assert.are.equal(bc.JVM_POP, code.bytecode[n - 1])
        assert.are.equal(4, code.bytecode[n - 2])
        assert.are.equal(bc.ILOAD, code.bytecode[n - 3])
    end)

    it("deduplicates constants", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.StringLiteral("hello")),
            bc.ExpressionStmt(bc.StringLiteral("hello")),
        })

        local code = compiler:compile(prog)

        assert.are.same({ "hello" }, code.constants)
    end)

    it("deduplicates local variable slots", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(1)),
            bc.Assignment(bc.Name("x"), bc.NumberLiteral(2)),
        })

        local code = compiler:compile(prog)

        assert.are.equal(1, code.num_locals)
        -- Both stores should go to slot 0.
        assert.are.equal(bc.ISTORE_0, code.bytecode[2])
        assert.are.equal(bc.ISTORE_0, code.bytecode[4])
    end)

    it("errors on unknown statement type", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({ { type = "BadStmt" } })

        assert.has_error(function()
            compiler:compile(prog)
        end, "Unknown statement type: BadStmt")
    end)

    it("errors on unknown expression type", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt({ type = "BadExpr" })
        })

        assert.has_error(function()
            compiler:compile(prog)
        end, "Unknown expression type: BadExpr")
    end)

    it("errors on unknown operator", function()
        local compiler = bc.JVMCompiler.new()
        local prog = bc.Program({
            bc.ExpressionStmt(bc.BinaryOp(bc.NumberLiteral(1), "%", bc.NumberLiteral(2)))
        })

        assert.has_error(function()
            compiler:compile(prog)
        end, "Unknown operator: %")
    end)

    it("has correct OPERATOR_MAP", function()
        assert.are.equal(bc.IADD, bc.JVMCompiler.OPERATOR_MAP["+"])
        assert.are.equal(bc.ISUB, bc.JVMCompiler.OPERATOR_MAP["-"])
        assert.are.equal(bc.IMUL, bc.JVMCompiler.OPERATOR_MAP["*"])
        assert.are.equal(bc.IDIV, bc.JVMCompiler.OPERATOR_MAP["/"])
    end)
end)


-- =========================================================================
-- CompilerScope
-- =========================================================================

describe("CompilerScope", function()
    it("assigns consecutive slot indices", function()
        local scope = bc.CompilerScope.new(nil)

        local a = scope:add_local("a")
        local b = scope:add_local("b")
        local c = scope:add_local("c")

        assert.are.equal(0, a)
        assert.are.equal(1, b)
        assert.are.equal(2, c)
    end)

    it("deduplicates — same name returns same slot", function()
        local scope = bc.CompilerScope.new(nil)

        local i1 = scope:add_local("x")
        local i2 = scope:add_local("x")

        assert.are.equal(0, i1)
        assert.are.equal(0, i2)
        assert.are.equal(1, scope:num_locals())
    end)

    it("get_local returns slot for known variables", function()
        local scope = bc.CompilerScope.new(nil)
        scope:add_local("param")
        scope:add_local("local_var")

        local param, param_ok = scope:get_local("param")
        local lv, lv_ok = scope:get_local("local_var")

        assert.is_true(param_ok)
        assert.are.equal(0, param)
        assert.is_true(lv_ok)
        assert.are.equal(1, lv)
    end)

    it("get_local returns nil and false for unknown variables", function()
        local scope = bc.CompilerScope.new(nil)

        local slot, ok = scope:get_local("nonexistent")

        assert.is_nil(slot)
        assert.is_false(ok)
    end)

    it("num_locals reflects the total count", function()
        local scope = bc.CompilerScope.new(nil)
        scope:add_local("a")
        scope:add_local("b")
        scope:add_local("c")

        assert.are.equal(3, scope:num_locals())
    end)

    it("num_locals starts at 0 for empty scope", function()
        local scope = bc.CompilerScope.new(nil)
        assert.are.equal(0, scope:num_locals())
    end)

    it("stores parent reference", function()
        local parent = bc.CompilerScope.new(nil)
        local child = bc.CompilerScope.new(parent)

        assert.are.equal(parent, child.parent)
        assert.is_nil(parent.parent)
    end)

    it("params are pre-assigned before add_local", function()
        local scope = bc.CompilerScope.new(nil)
        scope:add_local("x")
        scope:add_local("y")
        local temp_slot = scope:add_local("temp")

        local x, _ = scope:get_local("x")
        local y, _ = scope:get_local("y")

        assert.are.equal(0, x)
        assert.are.equal(1, y)
        assert.are.equal(2, temp_slot)
    end)
end)


-- =========================================================================
-- GenericCompiler — Plugin registration and dispatch
-- =========================================================================

describe("GenericCompiler plugin registration", function()
    it("calls the registered handler for a matching rule_name", function()
        local compiler = bc.GenericCompiler.new()
        local called = false

        compiler:register_rule("my_rule", function(c, node)
            called = true
        end)

        compiler:compile_node(bc.ASTNode("my_rule"))
        assert.is_true(called)
    end)

    it("passes the compiler and node to the handler", function()
        local compiler = bc.GenericCompiler.new()
        local test_node = bc.ASTNode("check_args", { bc.TokenNode("NUM", "42") })
        local received_compiler, received_node

        compiler:register_rule("check_args", function(c, node)
            received_compiler = c
            received_node = node
        end)

        compiler:compile_node(test_node)
        assert.are.equal(compiler, received_compiler)
        assert.are.equal(test_node, received_node)
    end)

    it("dispatches different rules to different handlers", function()
        local compiler = bc.GenericCompiler.new()
        local log = {}

        compiler:register_rule("rule_a", function(c, node)
            table.insert(log, "a")
        end)
        compiler:register_rule("rule_b", function(c, node)
            table.insert(log, "b")
        end)

        compiler:compile_node(bc.ASTNode("rule_a"))
        compiler:compile_node(bc.ASTNode("rule_b"))

        assert.are.same({ "a", "b" }, log)
    end)

    it("later registration overwrites earlier for the same rule_name", function()
        local compiler = bc.GenericCompiler.new()
        local result = ""

        compiler:register_rule("overridable", function(c, node)
            result = "first"
        end)
        compiler:register_rule("overridable", function(c, node)
            result = "second"
        end)

        compiler:compile_node(bc.ASTNode("overridable"))
        assert.are.equal("second", result)
    end)
end)


-- =========================================================================
-- GenericCompiler — Pass-through single child nodes
-- =========================================================================

describe("GenericCompiler pass-through", function()
    it("passes through a single-child ASTNode to its child", function()
        local compiler = bc.GenericCompiler.new()
        local called = false

        compiler:register_rule("inner", function(c, node)
            called = true
        end)

        compiler:compile_node(bc.ASTNode("wrapper", { bc.ASTNode("inner") }))
        assert.is_true(called)
    end)

    it("chains through multiple levels of single-child wrappers", function()
        local compiler = bc.GenericCompiler.new()
        local called = false

        compiler:register_rule("leaf", function(c, node)
            called = true
        end)

        local tree = bc.ASTNode("level1", {
            bc.ASTNode("level2", {
                bc.ASTNode("level3", {
                    bc.ASTNode("leaf")
                })
            })
        })

        compiler:compile_node(tree)
        assert.is_true(called)
    end)
end)


-- =========================================================================
-- GenericCompiler — Unhandled multi-child error
-- =========================================================================

describe("GenericCompiler unhandled rules", function()
    it("errors for multi-child node without handler", function()
        local compiler = bc.GenericCompiler.new()
        local node = bc.ASTNode("unknown_rule", {
            bc.TokenNode("A", "a"),
            bc.TokenNode("B", "b"),
        })

        local ok, err = pcall(function()
            compiler:compile_node(node)
        end)
        assert.is_false(ok)
        assert.truthy(string.find(err, "UnhandledRuleError"))
    end)

    it("error message includes the rule name", function()
        local compiler = bc.GenericCompiler.new()
        local node = bc.ASTNode("missing_handler", {
            bc.TokenNode("X", "x"),
            bc.TokenNode("Y", "y"),
        })

        local ok, err = pcall(function()
            compiler:compile_node(node)
        end)
        assert.is_false(ok)
        assert.truthy(string.find(err, "missing_handler"))
    end)

    it("errors for zero-child node without handler", function()
        local compiler = bc.GenericCompiler.new()
        local node = bc.ASTNode("no_children", {})

        local ok, err = pcall(function()
            compiler:compile_node(node)
        end)
        assert.is_false(ok)
        assert.truthy(string.find(err, "UnhandledRuleError"))
    end)

    it("errors on unexpected node_kind", function()
        local compiler = bc.GenericCompiler.new()

        local ok, err = pcall(function()
            compiler:compile_node({ node_kind = "alien" })
        end)
        assert.is_false(ok)
        assert.truthy(string.find(err, "CompilerError"))
    end)
end)


-- =========================================================================
-- GenericCompiler — Token pass-through (no-op)
-- =========================================================================

describe("GenericCompiler token handling", function()
    it("compile_token is a no-op by default", function()
        local compiler = bc.GenericCompiler.new()
        local before = #compiler.instructions

        compiler:compile_node(bc.TokenNode("NUMBER", "42"))

        assert.are.equal(before, #compiler.instructions)
    end)

    it("single-child wrapper around token passes through silently", function()
        local compiler = bc.GenericCompiler.new()
        local node = bc.ASTNode("wrapper", { bc.TokenNode("IDENT", "x") })

        -- Should not error.
        compiler:compile_node(node)
        assert.are.equal(0, #compiler.instructions)
    end)
end)


-- =========================================================================
-- GenericCompiler — Instruction emission
-- =========================================================================

describe("GenericCompiler instruction emission", function()
    it("emit appends an instruction with opcode only", function()
        local compiler = bc.GenericCompiler.new()
        compiler:emit(bc.OpAdd)

        assert.are.equal(1, #compiler.instructions)
        assert.are.equal(bc.OpAdd, compiler.instructions[1].opcode)
        assert.is_nil(compiler.instructions[1].operand)
    end)

    it("emit appends an instruction with opcode and operand", function()
        local compiler = bc.GenericCompiler.new()
        compiler:emit(bc.OpLoadConst, 0)

        assert.are.equal(1, #compiler.instructions)
        assert.are.equal(bc.OpLoadConst, compiler.instructions[1].opcode)
        assert.are.equal(0, compiler.instructions[1].operand)
    end)

    it("emit returns sequential 0-based indices", function()
        local compiler = bc.GenericCompiler.new()

        local idx0 = compiler:emit(bc.OpLoadConst, 0)
        local idx1 = compiler:emit(bc.OpLoadConst, 1)
        local idx2 = compiler:emit(bc.OpAdd)

        assert.are.equal(0, idx0)
        assert.are.equal(1, idx1)
        assert.are.equal(2, idx2)
    end)

    it("current_offset reflects the number of emitted instructions", function()
        local compiler = bc.GenericCompiler.new()

        assert.are.equal(0, compiler:current_offset())
        compiler:emit(bc.OpAdd)
        assert.are.equal(1, compiler:current_offset())
        compiler:emit(bc.OpSub)
        assert.are.equal(2, compiler:current_offset())
    end)

    it("emit supports string operand", function()
        local compiler = bc.GenericCompiler.new()
        compiler:emit(bc.OpLoadConst, "hello")

        assert.are.equal("hello", compiler.instructions[1].operand)
    end)

    it("emit supports nil operand (no operand)", function()
        local compiler = bc.GenericCompiler.new()
        compiler:emit(bc.OpLoadConst)

        assert.is_nil(compiler.instructions[1].operand)
    end)
end)


-- =========================================================================
-- GenericCompiler — Jump patching
-- =========================================================================

describe("GenericCompiler jump patching", function()
    it("emit_jump emits a placeholder with operand 0", function()
        local compiler = bc.GenericCompiler.new()
        local idx = compiler:emit_jump(bc.OpJumpIfFalse)

        -- 0-based index, so Lua table index is idx + 1.
        assert.are.equal(bc.OpJumpIfFalse, compiler.instructions[idx + 1].opcode)
        assert.are.equal(0, compiler.instructions[idx + 1].operand)
    end)

    it("patch_jump with explicit target", function()
        local compiler = bc.GenericCompiler.new()
        local jump_idx = compiler:emit_jump(bc.OpJump)
        compiler:emit(bc.OpAdd)  -- index 1
        compiler:emit(bc.OpSub)  -- index 2

        compiler:patch_jump(jump_idx, 2)

        assert.are.equal(2, compiler.instructions[jump_idx + 1].operand)
    end)

    it("patch_jump defaults to current_offset", function()
        local compiler = bc.GenericCompiler.new()
        local jump_idx = compiler:emit_jump(bc.OpJumpIfFalse)
        compiler:emit(bc.OpAdd)  -- index 1
        compiler:emit(bc.OpSub)  -- index 2

        -- current_offset is now 3.
        compiler:patch_jump(jump_idx)

        assert.are.equal(3, compiler.instructions[jump_idx + 1].operand)
    end)

    it("patch_jump preserves the original opcode", function()
        local compiler = bc.GenericCompiler.new()
        local jump_idx = compiler:emit_jump(bc.OpJumpIfFalse)

        compiler:patch_jump(jump_idx, 10)

        assert.are.equal(bc.OpJumpIfFalse, compiler.instructions[jump_idx + 1].opcode)
        assert.are.equal(10, compiler.instructions[jump_idx + 1].operand)
    end)

    it("emit_jump returns the instruction index for later patching", function()
        local compiler = bc.GenericCompiler.new()
        compiler:emit(bc.OpLoadConst, 0) -- index 0
        local jump_idx = compiler:emit_jump(bc.OpJump) -- index 1

        assert.are.equal(1, jump_idx)
    end)

    it("patch_jump errors on out-of-range index", function()
        local compiler = bc.GenericCompiler.new()

        local ok, err = pcall(function()
            compiler:patch_jump(99, 0)
        end)
        assert.is_false(ok)
        assert.truthy(string.find(err, "CompilerError"))
    end)

    it("patch_jump errors on negative index", function()
        local compiler = bc.GenericCompiler.new()

        assert.has_error(function()
            compiler:patch_jump(-1, 0)
        end)
    end)
end)


-- =========================================================================
-- GenericCompiler — Constant pool
-- =========================================================================

describe("GenericCompiler constant pool", function()
    it("add_constant adds a new value and returns its index", function()
        local compiler = bc.GenericCompiler.new()

        local idx = compiler:add_constant(42)

        assert.are.equal(0, idx)
        assert.are.same({ 42 }, compiler.constants)
    end)

    it("add_constant deduplicates identical values", function()
        local compiler = bc.GenericCompiler.new()

        local idx1 = compiler:add_constant(42)
        local idx2 = compiler:add_constant(42)

        assert.are.equal(0, idx1)
        assert.are.equal(0, idx2)
        assert.are.same({ 42 }, compiler.constants)
    end)

    it("add_constant handles multiple distinct values", function()
        local compiler = bc.GenericCompiler.new()

        local i0 = compiler:add_constant(1)
        local i1 = compiler:add_constant("hello")
        local i2 = compiler:add_constant(2)

        assert.are.equal(0, i0)
        assert.are.equal(1, i1)
        assert.are.equal(2, i2)
    end)

    it("add_constant distinguishes numbers from strings", function()
        local compiler = bc.GenericCompiler.new()

        local i0 = compiler:add_constant(0)
        local i1 = compiler:add_constant("0")

        assert.are.equal(0, i0)
        assert.are.equal(1, i1)
    end)
end)


-- =========================================================================
-- GenericCompiler — Name pool
-- =========================================================================

describe("GenericCompiler name pool", function()
    it("add_name adds a new name and returns its index", function()
        local compiler = bc.GenericCompiler.new()

        local idx = compiler:add_name("x")

        assert.are.equal(0, idx)
        assert.are.same({ "x" }, compiler.names)
    end)

    it("add_name deduplicates identical names", function()
        local compiler = bc.GenericCompiler.new()

        local idx1 = compiler:add_name("x")
        local idx2 = compiler:add_name("x")

        assert.are.equal(0, idx1)
        assert.are.equal(0, idx2)
        assert.are.same({ "x" }, compiler.names)
    end)

    it("add_name handles multiple distinct names", function()
        local compiler = bc.GenericCompiler.new()

        local i0 = compiler:add_name("x")
        local i1 = compiler:add_name("y")
        local i2 = compiler:add_name("z")

        assert.are.equal(0, i0)
        assert.are.equal(1, i1)
        assert.are.equal(2, i2)
        assert.are.same({ "x", "y", "z" }, compiler.names)
    end)
end)


-- =========================================================================
-- GenericCompiler — Scope management
-- =========================================================================

describe("GenericCompiler scope management", function()
    it("enter_scope creates a new scope and sets it as current", function()
        local compiler = bc.GenericCompiler.new()
        assert.is_nil(compiler.scope)

        local scope = compiler:enter_scope()

        assert.are.equal(scope, compiler.scope)
        assert.is_nil(scope.parent)
    end)

    it("enter_scope with params pre-assigns local slots", function()
        local compiler = bc.GenericCompiler.new()

        local scope = compiler:enter_scope("x", "y", "z")

        local x, x_ok = scope:get_local("x")
        local y, y_ok = scope:get_local("y")
        local z, z_ok = scope:get_local("z")
        assert.is_true(x_ok)
        assert.are.equal(0, x)
        assert.is_true(y_ok)
        assert.are.equal(1, y)
        assert.is_true(z_ok)
        assert.are.equal(2, z)
        assert.are.equal(3, scope:num_locals())
    end)

    it("exit_scope restores the parent scope", function()
        local compiler = bc.GenericCompiler.new()
        compiler:enter_scope()
        local inner = compiler:enter_scope()

        local exited = compiler:exit_scope()

        assert.are.equal(inner, exited)
        assert.is_not_nil(compiler.scope)
        assert.is_nil(compiler.scope.parent)
    end)

    it("nested scopes link via parent pointers", function()
        local compiler = bc.GenericCompiler.new()
        local outer = compiler:enter_scope("a")
        local inner = compiler:enter_scope("b")

        assert.are.equal(outer, inner.parent)
        assert.is_nil(outer.parent)
    end)

    it("exit_scope errors when not in a scope", function()
        local compiler = bc.GenericCompiler.new()

        local ok, err = pcall(function()
            compiler:exit_scope()
        end)
        assert.is_false(ok)
        assert.truthy(string.find(err, "CompilerError"))
    end)

    it("exit_scope returns the exited scope for inspection", function()
        local compiler = bc.GenericCompiler.new()
        local scope = compiler:enter_scope("x", "y")
        scope:add_local("temp")

        local exited = compiler:exit_scope()

        assert.are.equal(3, exited:num_locals())
        local x, _ = exited:get_local("x")
        local temp, _ = exited:get_local("temp")
        assert.are.equal(0, x)
        assert.are.equal(2, temp)
    end)
end)


-- =========================================================================
-- GenericCompiler — Nested code object compilation
-- =========================================================================

describe("GenericCompiler nested compilation", function()
    it("compile_nested returns a separate CodeObject", function()
        local compiler = bc.GenericCompiler.new()

        compiler:register_rule("body", function(c, node)
            local idx = c:add_constant(99)
            c:emit(bc.OpLoadConst, idx)
            c:add_name("local_var")
        end)

        local nested = compiler:compile_nested(bc.ASTNode("body"))

        assert.are.equal(1, #nested.instructions)
        assert.are.equal(bc.OpLoadConst, nested.instructions[1].opcode)
        assert.are.same({ 99 }, nested.constants)
        assert.are.same({ "local_var" }, nested.names)
    end)

    it("compile_nested restores outer state", function()
        local compiler = bc.GenericCompiler.new()

        -- Set up some outer state first.
        compiler:emit(bc.OpLoadConst, compiler:add_constant(1))
        compiler:add_name("outer_var")

        local outer_instr_count = #compiler.instructions
        local outer_const_count = #compiler.constants
        local outer_name_count = #compiler.names

        compiler:register_rule("inner_body", function(c, node)
            c:emit(bc.OpAdd)
            c:add_constant(999)
            c:add_name("inner_var")
        end)

        compiler:compile_nested(bc.ASTNode("inner_body"))

        -- Outer state should be restored.
        assert.are.equal(outer_instr_count, #compiler.instructions)
        assert.are.equal(outer_const_count, #compiler.constants)
        assert.are.equal(outer_name_count, #compiler.names)
        assert.are.same({ 1 }, compiler.constants)
        assert.are.same({ "outer_var" }, compiler.names)
    end)

    it("compile_nested does not pollute outer instructions", function()
        local compiler = bc.GenericCompiler.new()
        compiler:emit(bc.OpLoadConst, 0) -- outer instruction

        compiler:register_rule("nested", function(c, node)
            c:emit(bc.OpAdd)
            c:emit(bc.OpSub)
            c:emit(bc.OpMul)
        end)

        compiler:compile_nested(bc.ASTNode("nested"))

        -- Outer should still have just the one instruction.
        assert.are.equal(1, #compiler.instructions)
        assert.are.equal(bc.OpLoadConst, compiler.instructions[1].opcode)
    end)
end)


-- =========================================================================
-- GenericCompiler — Top-level compile
-- =========================================================================

describe("GenericCompiler top-level compile", function()
    it("appends HALT instruction at the end", function()
        local compiler = bc.GenericCompiler.new()
        compiler:register_rule("program", function(c, node)
            c:emit(bc.OpLoadConst, c:add_constant(42))
        end)

        local code = compiler:compile(bc.ASTNode("program"))

        local last = code.instructions[#code.instructions]
        assert.are.equal(bc.OpHalt, last.opcode)
    end)

    it("supports custom halt opcode", function()
        local compiler = bc.GenericCompiler.new()
        compiler:register_rule("prog", function(c, node) end)

        local custom_halt = 0xFE
        local code = compiler:compile(bc.ASTNode("prog"), custom_halt)

        local last = code.instructions[#code.instructions]
        assert.are.equal(custom_halt, last.opcode)
    end)

    it("returns a CodeObject with instructions constants and names", function()
        local compiler = bc.GenericCompiler.new()
        compiler:register_rule("root", function(c, node)
            c:emit(bc.OpLoadConst, c:add_constant(10))
            c:emit(bc.OpStoreName, c:add_name("x"))
        end)

        local code = compiler:compile(bc.ASTNode("root"))

        -- LOAD_CONST, STORE_NAME, HALT = 3 instructions
        assert.are.equal(3, #code.instructions)
        assert.are.same({ 10 }, code.constants)
        assert.are.same({ "x" }, code.names)
    end)

    it("empty program produces just HALT", function()
        local compiler = bc.GenericCompiler.new()
        compiler:register_rule("empty", function(c, node) end)

        local code = compiler:compile(bc.ASTNode("empty"))

        assert.are.equal(1, #code.instructions)
        assert.are.equal(bc.OpHalt, code.instructions[1].opcode)
    end)
end)


-- =========================================================================
-- GenericCompiler — Integration tests
-- =========================================================================

describe("GenericCompiler integration", function()
    it("compiles 1 + 2 to LOAD_CONST LOAD_CONST ADD HALT", function()
        local compiler = bc.GenericCompiler.new()

        compiler:register_rule("number", function(c, node)
            local token = node.children[1]  -- Lua 1-based
            local value = tonumber(token.value)
            local idx = c:add_constant(value)
            c:emit(bc.OpLoadConst, idx)
        end)

        compiler:register_rule("addition", function(c, node)
            c:compile_node(node.children[1])  -- left operand
            c:compile_node(node.children[3])  -- right operand (skip PLUS)
            c:emit(bc.OpAdd)
        end)

        local ast = bc.ASTNode("expression", {
            bc.ASTNode("addition", {
                bc.ASTNode("number", { bc.TokenNode("NUMBER", "1") }),
                bc.TokenNode("PLUS", "+"),
                bc.ASTNode("number", { bc.TokenNode("NUMBER", "2") }),
            }),
        })

        local code = compiler:compile(ast)

        local expected_opcodes = {
            bc.OpLoadConst,
            bc.OpLoadConst,
            bc.OpAdd,
            bc.OpHalt,
        }
        assert.are.same(expected_opcodes, opcodes(code.instructions))
        assert.are.same({ 1, 2 }, code.constants)
        assert.are.equal(0, code.instructions[1].operand)
        assert.are.equal(1, code.instructions[2].operand)
    end)

    it("compiles nested 1 + 2 + 3 with left-associative grouping", function()
        local compiler = bc.GenericCompiler.new()

        compiler:register_rule("number", function(c, node)
            local token = node.children[1]
            local value = tonumber(token.value)
            c:emit(bc.OpLoadConst, c:add_constant(value))
        end)

        compiler:register_rule("addition", function(c, node)
            c:compile_node(node.children[1])
            c:compile_node(node.children[3])
            c:emit(bc.OpAdd)
        end)

        local ast = bc.ASTNode("addition", {
            bc.ASTNode("addition", {
                bc.ASTNode("number", { bc.TokenNode("NUMBER", "1") }),
                bc.TokenNode("PLUS", "+"),
                bc.ASTNode("number", { bc.TokenNode("NUMBER", "2") }),
            }),
            bc.TokenNode("PLUS", "+"),
            bc.ASTNode("number", { bc.TokenNode("NUMBER", "3") }),
        })

        local code = compiler:compile(ast)

        local expected_opcodes = {
            bc.OpLoadConst,  -- 1
            bc.OpLoadConst,  -- 2
            bc.OpAdd,        -- 1 + 2
            bc.OpLoadConst,  -- 3
            bc.OpAdd,        -- (1+2) + 3
            bc.OpHalt,
        }
        assert.are.same(expected_opcodes, opcodes(code.instructions))
        assert.are.same({ 1, 2, 3 }, code.constants)
    end)

    it("compiles variable assignment and lookup", function()
        local compiler = bc.GenericCompiler.new()

        compiler:register_rule("number", function(c, node)
            local token = node.children[1]
            local value = tonumber(token.value)
            c:emit(bc.OpLoadConst, c:add_constant(value))
        end)

        compiler:register_rule("assignment", function(c, node)
            local name_token = node.children[1]
            c:compile_node(node.children[3])  -- compile the value
            c:emit(bc.OpStoreName, c:add_name(name_token.value))
        end)

        compiler:register_rule("name_ref", function(c, node)
            local token = node.children[1]
            c:emit(bc.OpLoadName, c:add_name(token.value))
        end)

        compiler:register_rule("program", function(c, node)
            for _, child in ipairs(node.children) do
                c:compile_node(child)
            end
        end)

        local ast = bc.ASTNode("program", {
            bc.ASTNode("assignment", {
                bc.TokenNode("IDENT", "x"),
                bc.TokenNode("EQUALS", "="),
                bc.ASTNode("number", { bc.TokenNode("NUMBER", "42") }),
            }),
            bc.ASTNode("name_ref", {
                bc.TokenNode("IDENT", "x"),
            }),
        })

        local code = compiler:compile(ast)

        local expected_opcodes = {
            bc.OpLoadConst,
            bc.OpStoreName,
            bc.OpLoadName,
            bc.OpHalt,
        }
        assert.are.same(expected_opcodes, opcodes(code.instructions))
        assert.are.same({ 42 }, code.constants)
        assert.are.same({ "x" }, code.names)
    end)
end)


-- =========================================================================
-- GenericCompiler — Additional edge cases
-- =========================================================================

describe("GenericCompiler edge cases", function()
    it("starts with nil scope", function()
        local compiler = bc.GenericCompiler.new()
        assert.is_nil(compiler.scope)
    end)

    it("starts with empty instructions", function()
        local compiler = bc.GenericCompiler.new()
        assert.are.equal(0, #compiler.instructions)
    end)

    it("starts with empty constants", function()
        local compiler = bc.GenericCompiler.new()
        assert.are.equal(0, #compiler.constants)
    end)

    it("starts with empty names", function()
        local compiler = bc.GenericCompiler.new()
        assert.are.equal(0, #compiler.names)
    end)

    it("handler can emit multiple instructions", function()
        local compiler = bc.GenericCompiler.new()
        compiler:register_rule("multi", function(c, node)
            c:emit(bc.OpLoadConst, 0)
            c:emit(bc.OpLoadConst, 1)
            c:emit(bc.OpAdd)
        end)

        compiler:compile_node(bc.ASTNode("multi"))
        assert.are.equal(3, #compiler.instructions)
    end)

    it("handler can recursively compile children", function()
        local compiler = bc.GenericCompiler.new()
        local order = {}

        compiler:register_rule("parent", function(c, node)
            table.insert(order, "parent_start")
            for _, child in ipairs(node.children) do
                c:compile_node(child)
            end
            table.insert(order, "parent_end")
        end)

        compiler:register_rule("child", function(c, node)
            table.insert(order, "child")
        end)

        compiler:compile_node(bc.ASTNode("parent", {
            bc.ASTNode("child"),
            bc.ASTNode("child"),
        }))

        assert.are.same({ "parent_start", "child", "child", "parent_end" }, order)
    end)

    it("scope enter and exit can be interleaved with compilation", function()
        local compiler = bc.GenericCompiler.new()

        local scope1 = compiler:enter_scope("a")
        assert.are.equal(scope1, compiler.scope)

        local scope2 = compiler:enter_scope("b")
        assert.are.equal(scope2, compiler.scope)
        assert.are.equal(scope1, scope2.parent)

        local exited = compiler:exit_scope()
        assert.are.equal(scope2, exited)
        assert.are.equal(scope1, compiler.scope)

        compiler:exit_scope()
        assert.is_nil(compiler.scope)
    end)
end)

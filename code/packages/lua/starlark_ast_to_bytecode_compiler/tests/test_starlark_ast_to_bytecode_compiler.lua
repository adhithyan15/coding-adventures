-- Tests for coding_adventures.starlark_ast_to_bytecode_compiler
-- ================================================================
-- Run with: cd tests && busted . --verbose --pattern=test_

package.path = "../src/?.lua;" .. "../src/?/init.lua;" ..
               "../../bytecode_compiler/src/?.lua;" ..
               "../../bytecode_compiler/src/?/init.lua;" ..
               package.path

local compiler_mod = require("coding_adventures.starlark_ast_to_bytecode_compiler")

-- Shorter aliases
local C   = compiler_mod
local tok = compiler_mod.token_node
local ast = compiler_mod.ast_node

local function compile(tree)
    local compiler = C.new_compiler()
    return compiler:compile(tree)
end

-- ============================================================================
-- Opcode Constants
-- ============================================================================

describe("opcode constants", function()
    it("stack ops are correct", function()
        assert.equals(C.OP_LOAD_CONST, 0x01)
        assert.equals(C.OP_POP,        0x02)
        assert.equals(C.OP_DUP,        0x03)
        assert.equals(C.OP_LOAD_NONE,  0x04)
        assert.equals(C.OP_LOAD_TRUE,  0x05)
        assert.equals(C.OP_LOAD_FALSE, 0x06)
    end)

    it("variable ops are correct", function()
        assert.equals(C.OP_STORE_NAME,  0x10)
        assert.equals(C.OP_LOAD_NAME,   0x11)
        assert.equals(C.OP_STORE_LOCAL, 0x12)
        assert.equals(C.OP_LOAD_LOCAL,  0x13)
    end)

    it("arithmetic ops are correct", function()
        assert.equals(C.OP_ADD,       0x20)
        assert.equals(C.OP_SUB,       0x21)
        assert.equals(C.OP_MUL,       0x22)
        assert.equals(C.OP_DIV,       0x23)
        assert.equals(C.OP_FLOOR_DIV, 0x24)
        assert.equals(C.OP_MOD,       0x25)
        assert.equals(C.OP_POWER,     0x26)
        assert.equals(C.OP_NEGATE,    0x27)
    end)

    it("bitwise ops are correct", function()
        assert.equals(C.OP_BIT_AND, 0x28)
        assert.equals(C.OP_BIT_OR,  0x29)
        assert.equals(C.OP_BIT_XOR, 0x2A)
        assert.equals(C.OP_BIT_NOT, 0x2B)
        assert.equals(C.OP_LSHIFT,  0x2C)
        assert.equals(C.OP_RSHIFT,  0x2D)
    end)

    it("comparison ops are correct", function()
        assert.equals(C.OP_CMP_EQ,     0x30)
        assert.equals(C.OP_CMP_NE,     0x31)
        assert.equals(C.OP_CMP_LT,     0x32)
        assert.equals(C.OP_CMP_GT,     0x33)
        assert.equals(C.OP_CMP_LE,     0x34)
        assert.equals(C.OP_CMP_GE,     0x35)
        assert.equals(C.OP_CMP_IN,     0x36)
        assert.equals(C.OP_CMP_NOT_IN, 0x37)
        assert.equals(C.OP_LOGICAL_NOT,0x38)
    end)

    it("control flow ops are correct", function()
        assert.equals(C.OP_JUMP,                 0x40)
        assert.equals(C.OP_JUMP_IF_FALSE,        0x41)
        assert.equals(C.OP_JUMP_IF_TRUE,         0x42)
        assert.equals(C.OP_JUMP_IF_FALSE_OR_POP, 0x43)
        assert.equals(C.OP_JUMP_IF_TRUE_OR_POP,  0x44)
    end)

    it("function ops are correct", function()
        assert.equals(C.OP_MAKE_FUNCTION,    0x50)
        assert.equals(C.OP_CALL_FUNCTION,    0x51)
        assert.equals(C.OP_CALL_FUNCTION_KW, 0x52)
        assert.equals(C.OP_RETURN,           0x53)
    end)

    it("collection ops are correct", function()
        assert.equals(C.OP_BUILD_LIST,  0x60)
        assert.equals(C.OP_BUILD_DICT,  0x61)
        assert.equals(C.OP_BUILD_TUPLE, 0x62)
        assert.equals(C.OP_LIST_APPEND, 0x63)
        assert.equals(C.OP_DICT_SET,    0x64)
    end)

    it("subscript and attribute ops are correct", function()
        assert.equals(C.OP_LOAD_SUBSCRIPT,  0x70)
        assert.equals(C.OP_STORE_SUBSCRIPT, 0x71)
        assert.equals(C.OP_LOAD_ATTR,       0x72)
        assert.equals(C.OP_STORE_ATTR,      0x73)
        assert.equals(C.OP_LOAD_SLICE,      0x74)
    end)

    it("iteration ops are correct", function()
        assert.equals(C.OP_GET_ITER,        0x80)
        assert.equals(C.OP_FOR_ITER,        0x81)
        assert.equals(C.OP_UNPACK_SEQUENCE, 0x82)
    end)

    it("module and IO and halt ops are correct", function()
        assert.equals(C.OP_LOAD_MODULE,  0x90)
        assert.equals(C.OP_IMPORT_FROM,  0x91)
        assert.equals(C.OP_PRINT,        0xA0)
        assert.equals(C.OP_HALT,         0xFF)
    end)
end)

-- ============================================================================
-- Operator Maps
-- ============================================================================

describe("operator maps", function()
    it("BINARY_OP_MAP covers arithmetic operators", function()
        assert.equals(C.BINARY_OP_MAP["+"],  C.OP_ADD)
        assert.equals(C.BINARY_OP_MAP["-"],  C.OP_SUB)
        assert.equals(C.BINARY_OP_MAP["*"],  C.OP_MUL)
        assert.equals(C.BINARY_OP_MAP["/"],  C.OP_DIV)
        assert.equals(C.BINARY_OP_MAP["//"], C.OP_FLOOR_DIV)
        assert.equals(C.BINARY_OP_MAP["%"],  C.OP_MOD)
        assert.equals(C.BINARY_OP_MAP["**"], C.OP_POWER)
        assert.equals(C.BINARY_OP_MAP["&"],  C.OP_BIT_AND)
        assert.equals(C.BINARY_OP_MAP["|"],  C.OP_BIT_OR)
        assert.equals(C.BINARY_OP_MAP["^"],  C.OP_BIT_XOR)
        assert.equals(C.BINARY_OP_MAP["<<"], C.OP_LSHIFT)
        assert.equals(C.BINARY_OP_MAP[">>"], C.OP_RSHIFT)
    end)

    it("COMPARE_OP_MAP covers comparison operators", function()
        assert.equals(C.COMPARE_OP_MAP["=="],     C.OP_CMP_EQ)
        assert.equals(C.COMPARE_OP_MAP["!="],     C.OP_CMP_NE)
        assert.equals(C.COMPARE_OP_MAP["<"],      C.OP_CMP_LT)
        assert.equals(C.COMPARE_OP_MAP[">"],      C.OP_CMP_GT)
        assert.equals(C.COMPARE_OP_MAP["<="],     C.OP_CMP_LE)
        assert.equals(C.COMPARE_OP_MAP[">="],     C.OP_CMP_GE)
        assert.equals(C.COMPARE_OP_MAP["in"],     C.OP_CMP_IN)
        assert.equals(C.COMPARE_OP_MAP["not in"], C.OP_CMP_NOT_IN)
    end)

    it("AUGMENTED_ASSIGN_MAP covers all augmented operators", function()
        assert.equals(C.AUGMENTED_ASSIGN_MAP["+="],  C.OP_ADD)
        assert.equals(C.AUGMENTED_ASSIGN_MAP["-="],  C.OP_SUB)
        assert.equals(C.AUGMENTED_ASSIGN_MAP["*="],  C.OP_MUL)
        assert.equals(C.AUGMENTED_ASSIGN_MAP["//="], C.OP_FLOOR_DIV)
    end)

    it("UNARY_OP_MAP covers unary operators", function()
        assert.equals(C.UNARY_OP_MAP["-"], C.OP_NEGATE)
        assert.equals(C.UNARY_OP_MAP["~"], C.OP_BIT_NOT)
    end)
end)

-- ============================================================================
-- Helper: instruction list builder
-- ============================================================================

local function opcodes_of(code_obj)
    local result = {}
    for _, instr in ipairs(code_obj.instructions) do
        table.insert(result, instr.opcode)
    end
    return result
end

local function operands_of(code_obj)
    local result = {}
    for _, instr in ipairs(code_obj.instructions) do
        table.insert(result, instr.operand)
    end
    return result
end

-- ============================================================================
-- Compiling Constants
-- ============================================================================

describe("compiling atom literals", function()
    it("integer literal emits LOAD_CONST + HALT", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("atom", { tok("INT", "42") })
                    })
                })
            })
        })
        local co = compile(tree)
        -- expression_stmt pops the result, so: LOAD_CONST, POP, HALT
        local ops = opcodes_of(co)
        assert.equals(ops[1], C.OP_LOAD_CONST)
        assert.equals(ops[2], C.OP_POP)
        assert.equals(ops[3], C.OP_HALT)
        assert.equals(co.constants[1], 42)
    end)

    it("string literal puts stripped string in constant pool", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("atom", { tok("STRING", '"hello"') })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.constants[1], "hello")
    end)

    it("True emits LOAD_TRUE", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("atom", { tok("NAME", "True") })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_TRUE)
    end)

    it("False emits LOAD_FALSE", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("atom", { tok("NAME", "False") })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_FALSE)
    end)

    it("None emits LOAD_NONE", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("atom", { tok("NAME", "None") })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_NONE)
    end)
end)

-- ============================================================================
-- Identifier (variable reference)
-- ============================================================================

describe("compiling identifier references", function()
    it("identifier emits LOAD_NAME", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("identifier", { tok("NAME", "x") })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_NAME)
        assert.equals(co.names[1], "x")
    end)
end)

-- ============================================================================
-- Assignment: x = expr
-- ============================================================================

describe("compiling assign_stmt", function()
    it("x = 42 emits LOAD_CONST + STORE_NAME + HALT", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("assign_stmt", {
                        ast("identifier", { tok("NAME", "x") }),
                        tok("OP", "="),
                        ast("atom",       { tok("INT", "42") }),
                    })
                })
            })
        })
        local co = compile(tree)
        local ops = opcodes_of(co)
        assert.equals(ops[1], C.OP_LOAD_CONST)   -- push 42
        assert.equals(ops[2], C.OP_STORE_NAME)    -- store in x
        assert.equals(ops[3], C.OP_HALT)
        assert.equals(co.constants[1], 42)
        assert.equals(co.names[1], "x")
    end)
end)

-- ============================================================================
-- Arithmetic Expressions
-- ============================================================================

describe("compiling arith expressions", function()
    it("1 + 2 emits LOAD_CONST LOAD_CONST ADD", function()
        -- arith: [term(1), "+", term(2)]
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("arith", {
                            ast("atom", { tok("INT", "1") }),
                            tok("OP", "+"),
                            ast("atom", { tok("INT", "2") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local ops = opcodes_of(co)
        assert.equals(ops[1], C.OP_LOAD_CONST)
        assert.equals(ops[2], C.OP_LOAD_CONST)
        assert.equals(ops[3], C.OP_ADD)
        assert.equals(ops[4], C.OP_POP)
        assert.equals(co.constants[1], 1)
        assert.equals(co.constants[2], 2)
    end)

    it("a - b emits SUB", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("arith", {
                            ast("atom", { tok("INT", "5") }),
                            tok("OP", "-"),
                            ast("atom", { tok("INT", "3") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_SUB)
    end)

    it("term: a * b emits MUL", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("term", {
                            ast("atom", { tok("INT", "3") }),
                            tok("OP", "*"),
                            ast("atom", { tok("INT", "7") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_MUL)
    end)

    it("term: a // b emits FLOOR_DIV", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("term", {
                            ast("atom", { tok("INT", "7") }),
                            tok("OP", "//"),
                            ast("atom", { tok("INT", "2") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_FLOOR_DIV)
    end)

    it("term: a % b emits MOD", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("term", {
                            ast("atom", { tok("INT", "10") }),
                            tok("OP", "%"),
                            ast("atom", { tok("INT", "3") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_MOD)
    end)

    it("power_expr: a ** b emits POWER", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("power_expr", {
                            ast("atom", { tok("INT", "2") }),
                            tok("OP", "**"),
                            ast("atom", { tok("INT", "8") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_POWER)
    end)
end)

-- ============================================================================
-- Unary Expressions
-- ============================================================================

describe("compiling unary/factor", function()
    it("-x emits LOAD_NAME + NEGATE", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("factor", {
                            tok("OP", "-"),
                            ast("atom", { tok("NAME", "x") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_NAME)
        assert.equals(co.instructions[2].opcode, C.OP_NEGATE)
    end)

    it("~x emits BIT_NOT", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("factor", {
                            tok("OP", "~"),
                            ast("atom", { tok("INT", "5") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[2].opcode, C.OP_BIT_NOT)
    end)
end)

-- ============================================================================
-- Comparison Expressions
-- ============================================================================

describe("compiling comparison", function()
    it("a == b emits CMP_EQ", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("comparison", {
                            ast("atom", { tok("INT", "1") }),
                            tok("OP", "=="),
                            ast("atom", { tok("INT", "1") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_CMP_EQ)
    end)

    it("a < b emits CMP_LT", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("comparison", {
                            ast("atom", { tok("NAME", "a") }),
                            tok("OP", "<"),
                            ast("atom", { tok("NAME", "b") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_CMP_LT)
    end)

    it("not x emits LOGICAL_NOT", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("not_expr", {
                            tok("KW", "not"),
                            ast("atom", { tok("NAME", "x") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[2].opcode, C.OP_LOGICAL_NOT)
    end)
end)

-- ============================================================================
-- Boolean Short-Circuit
-- ============================================================================

describe("compiling boolean expressions", function()
    it("a or b uses JUMP_IF_TRUE_OR_POP", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("or_expr", {
                            ast("atom", { tok("NAME", "a") }),
                            tok("KW", "or"),
                            ast("atom", { tok("NAME", "b") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        -- Should have: LOAD_NAME(a), JUMP_IF_TRUE_OR_POP, LOAD_NAME(b), POP, HALT
        local has_jit = false
        for _, instr in ipairs(co.instructions) do
            if instr.opcode == C.OP_JUMP_IF_TRUE_OR_POP then
                has_jit = true
            end
        end
        assert.is_true(has_jit, "or_expr should emit JUMP_IF_TRUE_OR_POP")
    end)

    it("a and b uses JUMP_IF_FALSE_OR_POP", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("and_expr", {
                            ast("atom", { tok("NAME", "a") }),
                            tok("KW", "and"),
                            ast("atom", { tok("NAME", "b") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local has_jif = false
        for _, instr in ipairs(co.instructions) do
            if instr.opcode == C.OP_JUMP_IF_FALSE_OR_POP then
                has_jif = true
            end
        end
        assert.is_true(has_jif, "and_expr should emit JUMP_IF_FALSE_OR_POP")
    end)
end)

-- ============================================================================
-- Control Flow: if_stmt
-- ============================================================================

describe("compiling if_stmt", function()
    it("if cond: suite emits JUMP_IF_FALSE", function()
        local tree = ast("file", {
            ast("statement", {
                ast("compound_stmt", {
                    ast("if_stmt", {
                        ast("atom", { tok("NAME", "cond") }),
                        ast("suite", {
                            ast("statement", {
                                ast("simple_stmt", {
                                    ast("pass_stmt", { tok("KW", "pass") })
                                })
                            })
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local has_jif = false
        for _, instr in ipairs(co.instructions) do
            if instr.opcode == C.OP_JUMP_IF_FALSE then
                has_jif = true
            end
        end
        assert.is_true(has_jif, "if_stmt should emit JUMP_IF_FALSE")
    end)
end)

-- ============================================================================
-- pass_stmt
-- ============================================================================

describe("compiling pass_stmt", function()
    it("pass emits nothing (only HALT remains)", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("pass_stmt", { tok("KW", "pass") })
                })
            })
        })
        local co = compile(tree)
        assert.equals(#co.instructions, 1)
        assert.equals(co.instructions[1].opcode, C.OP_HALT)
    end)
end)

-- ============================================================================
-- return_stmt
-- ============================================================================

describe("compiling return_stmt", function()
    it("return 42 emits LOAD_CONST + RETURN", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("return_stmt", {
                        tok("KW", "return"),
                        ast("atom", { tok("INT", "42") }),
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_CONST)
        assert.equals(co.instructions[2].opcode, C.OP_RETURN)
        assert.equals(co.constants[1], 42)
    end)

    it("bare return emits LOAD_NONE + RETURN", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("return_stmt", {
                        tok("KW", "return"),
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_NONE)
        assert.equals(co.instructions[2].opcode, C.OP_RETURN)
    end)
end)

-- ============================================================================
-- Collections
-- ============================================================================

describe("compiling list expressions", function()
    it("empty list emits BUILD_LIST 0", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("list_expr", {})
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_BUILD_LIST)
        assert.equals(co.instructions[1].operand, 0)
    end)

    it("list with 2 elements emits 2 LOAD_CONSTs + BUILD_LIST 2", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("list_expr", {
                            ast("atom", { tok("INT", "1") }),
                            ast("atom", { tok("INT", "2") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_CONST)
        assert.equals(co.instructions[2].opcode, C.OP_LOAD_CONST)
        assert.equals(co.instructions[3].opcode, C.OP_BUILD_LIST)
        assert.equals(co.instructions[3].operand, 2)
    end)
end)

describe("compiling dict expressions", function()
    it("empty dict emits BUILD_DICT 0", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("dict_expr", {})
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_BUILD_DICT)
        assert.equals(co.instructions[1].operand, 0)
    end)

    it("dict with 1 entry emits key, value, BUILD_DICT 1", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("dict_expr", {
                            ast("dict_entry", {
                                ast("atom", { tok("STRING", '"k"') }),
                                tok("OP", ":"),
                                ast("atom", { tok("INT", "1") }),
                            })
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_BUILD_DICT)
        assert.equals(co.instructions[3].operand, 1)
    end)
end)

describe("compiling tuple_expr", function()
    it("(a, b) emits 2 loads + BUILD_TUPLE 2", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("tuple_expr", {
                            ast("atom", { tok("INT", "1") }),
                            ast("atom", { tok("INT", "2") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_BUILD_TUPLE)
        assert.equals(co.instructions[3].operand, 2)
    end)
end)

-- ============================================================================
-- for_stmt
-- ============================================================================

describe("compiling for_stmt", function()
    it("for x in items: body emits GET_ITER + FOR_ITER + JUMP", function()
        local tree = ast("file", {
            ast("statement", {
                ast("compound_stmt", {
                    ast("for_stmt", {
                        tok("KW", "for"),
                        ast("identifier", { tok("NAME", "x") }),
                        tok("KW", "in"),
                        ast("atom", { tok("NAME", "items") }),
                        tok("OP", ":"),
                        ast("suite", {
                            ast("statement", {
                                ast("simple_stmt", {
                                    ast("pass_stmt", { tok("KW", "pass") })
                                })
                            })
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local ops = {}
        for _, instr in ipairs(co.instructions) do
            table.insert(ops, instr.opcode)
        end
        local has_get_iter = false
        local has_for_iter = false
        local has_jump     = false
        for _, op in ipairs(ops) do
            if op == C.OP_GET_ITER then has_get_iter = true end
            if op == C.OP_FOR_ITER then has_for_iter = true end
            if op == C.OP_JUMP     then has_jump     = true end
        end
        assert.is_true(has_get_iter, "should emit GET_ITER")
        assert.is_true(has_for_iter, "should emit FOR_ITER")
        assert.is_true(has_jump,     "should emit JUMP back to loop")
    end)
end)

-- ============================================================================
-- def_stmt (function definitions)
-- ============================================================================

describe("compiling def_stmt", function()
    it("def f(): pass emits LOAD_CONST + MAKE_FUNCTION + STORE_NAME", function()
        local tree = ast("file", {
            ast("statement", {
                ast("compound_stmt", {
                    ast("def_stmt", {
                        tok("KW", "def"),
                        tok("NAME", "f"),
                        tok("OP", "("),
                        tok("OP", ")"),
                        tok("OP", ":"),
                        ast("suite", {
                            ast("statement", {
                                ast("simple_stmt", {
                                    ast("pass_stmt", { tok("KW", "pass") })
                                })
                            })
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local has_make_func  = false
        local has_store_name = false
        for _, instr in ipairs(co.instructions) do
            if instr.opcode == C.OP_MAKE_FUNCTION  then has_make_func  = true end
            if instr.opcode == C.OP_STORE_NAME     then has_store_name = true end
        end
        assert.is_true(has_make_func,  "should emit MAKE_FUNCTION")
        assert.is_true(has_store_name, "should emit STORE_NAME for function name")
        assert.equals(co.names[1], "f")
    end)
end)

-- ============================================================================
-- call (function calls)
-- ============================================================================

describe("compiling call", function()
    it("f() emits LOAD_NAME + CALL_FUNCTION 0", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("call", {
                            ast("atom", { tok("NAME", "f") }),
                            tok("OP", "("),
                            tok("OP", ")"),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_NAME)
        assert.equals(co.instructions[2].opcode, C.OP_CALL_FUNCTION)
        assert.equals(co.instructions[2].operand, 0)
    end)

    it("f(1, 2) emits LOAD_NAME + 2 LOAD_CONSTs + CALL_FUNCTION 2", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("call", {
                            ast("atom", { tok("NAME", "f") }),
                            tok("OP", "("),
                            ast("call_args", {
                                ast("argument", { ast("atom", { tok("INT", "1") }) }),
                                ast("argument", { ast("atom", { tok("INT", "2") }) }),
                            }),
                            tok("OP", ")"),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[1].opcode, C.OP_LOAD_NAME)
        assert.equals(co.instructions[2].opcode, C.OP_LOAD_CONST)
        assert.equals(co.instructions[3].opcode, C.OP_LOAD_CONST)
        assert.equals(co.instructions[4].opcode, C.OP_CALL_FUNCTION)
        assert.equals(co.instructions[4].operand, 2)
    end)
end)

-- ============================================================================
-- CodeObject structure
-- ============================================================================

describe("code_object structure", function()
    it("has instructions, constants, and names arrays", function()
        local tree = ast("file", {})
        local co = compile(tree)
        assert.is_table(co.instructions)
        assert.is_table(co.constants)
        assert.is_table(co.names)
    end)

    it("constants are deduplicated", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("arith", {
                            ast("atom", { tok("INT", "5") }),
                            tok("OP", "+"),
                            ast("atom", { tok("INT", "5") }),  -- same constant
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        -- Only one entry for 5 in the constant pool
        local count = 0
        for _, v in ipairs(co.constants) do
            if v == 5 then count = count + 1 end
        end
        assert.equals(count, 1)
    end)

    it("names are deduplicated", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("arith", {
                            ast("identifier", { tok("NAME", "x") }),
                            tok("OP", "+"),
                            ast("identifier", { tok("NAME", "x") }),  -- same name
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local count = 0
        for _, n in ipairs(co.names) do
            if n == "x" then count = count + 1 end
        end
        assert.equals(count, 1)
    end)
end)

-- ============================================================================
-- String helper
-- ============================================================================

describe("_strip_quotes helper", function()
    it("strips double quotes", function()
        assert.equals(C._strip_quotes('"hello"'), "hello")
    end)

    it("strips single quotes", function()
        assert.equals(C._strip_quotes("'world'"), "world")
    end)

    it("strips triple double quotes", function()
        assert.equals(C._strip_quotes('"""triple"""'), "triple")
    end)

    it("strips triple single quotes", function()
        assert.equals(C._strip_quotes("'''triple'''"), "triple")
    end)

    it("returns as-is for unquoted strings", function()
        assert.equals(C._strip_quotes("bare"), "bare")
    end)

    it("handles nil safely", function()
        assert.equals(C._strip_quotes(nil), "")
    end)
end)

-- ============================================================================
-- Integration: x = 1 + 2
-- ============================================================================

describe("integration: x = 1 + 2", function()
    it("produces correct instruction sequence", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("assign_stmt", {
                        ast("identifier", { tok("NAME", "x") }),
                        tok("OP", "="),
                        ast("arith", {
                            ast("atom", { tok("INT", "1") }),
                            tok("OP", "+"),
                            ast("atom", { tok("INT", "2") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        local ops = opcodes_of(co)
        -- Expected: LOAD_CONST(1), LOAD_CONST(2), ADD, STORE_NAME(x), HALT
        assert.equals(ops[1], C.OP_LOAD_CONST)
        assert.equals(ops[2], C.OP_LOAD_CONST)
        assert.equals(ops[3], C.OP_ADD)
        assert.equals(ops[4], C.OP_STORE_NAME)
        assert.equals(ops[5], C.OP_HALT)
        assert.equals(co.constants[1], 1)
        assert.equals(co.constants[2], 2)
        assert.equals(co.names[1], "x")
    end)
end)

-- ============================================================================
-- Integration: bitwise operations
-- ============================================================================

describe("integration: bitwise ops", function()
    it("a & b emits BIT_AND", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("bitwise_and", {
                            ast("atom", { tok("NAME", "a") }),
                            tok("OP", "&"),
                            ast("atom", { tok("NAME", "b") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_BIT_AND)
    end)

    it("a | b emits BIT_OR", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("bitwise_or", {
                            ast("atom", { tok("NAME", "a") }),
                            tok("OP", "|"),
                            ast("atom", { tok("NAME", "b") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_BIT_OR)
    end)

    it("a ^ b emits BIT_XOR", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("bitwise_xor", {
                            ast("atom", { tok("NAME", "a") }),
                            tok("OP", "^"),
                            ast("atom", { tok("NAME", "b") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_BIT_XOR)
    end)
end)

-- ============================================================================
-- Integration: shift operations
-- ============================================================================

describe("integration: shift ops", function()
    it("a << b emits LSHIFT", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("expression_stmt", {
                        ast("shift", {
                            ast("atom", { tok("INT", "1") }),
                            tok("OP", "<<"),
                            ast("atom", { tok("INT", "4") }),
                        })
                    })
                })
            })
        })
        local co = compile(tree)
        assert.equals(co.instructions[3].opcode, C.OP_LSHIFT)
    end)
end)

-- ============================================================================
-- Integration: augmented_assign_stmt
-- ============================================================================

describe("compiling augmented_assign_stmt", function()
    it("x += 1 emits LOAD_NAME + LOAD_CONST + ADD + STORE_NAME", function()
        local tree = ast("file", {
            ast("statement", {
                ast("simple_stmt", {
                    ast("augmented_assign_stmt", {
                        ast("identifier", { tok("NAME", "x") }),
                        tok("OP", "+="),
                        ast("atom", { tok("INT", "1") }),
                    })
                })
            })
        })
        local co = compile(tree)
        local ops = opcodes_of(co)
        assert.equals(ops[1], C.OP_LOAD_NAME)   -- load x
        assert.equals(ops[2], C.OP_LOAD_CONST)  -- load 1
        assert.equals(ops[3], C.OP_ADD)          -- x + 1
        assert.equals(ops[4], C.OP_STORE_NAME)   -- store back to x
    end)
end)

-- ============================================================================
-- new_compiler() and compile_ast() convenience API
-- ============================================================================

describe("convenience API", function()
    it("new_compiler() returns a compiler with compile method", function()
        local c = C.new_compiler()
        assert.is_table(c)
        assert.is_function(c.compile)
    end)

    it("compile_ast() compiles a simple tree", function()
        local tree = ast("file", {})
        local co = C.compile_ast(tree)
        assert.is_table(co)
        assert.equals(co.instructions[1].opcode, C.OP_HALT)
    end)
end)

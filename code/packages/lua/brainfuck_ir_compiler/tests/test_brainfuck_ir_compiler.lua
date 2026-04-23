-- ============================================================================
-- Tests for brainfuck_ir_compiler — Brainfuck AOT compiler frontend
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. BuildConfig — debug_config, release_config, new_build_config.
-- 2. Empty program — prologue + HALT, data declaration.
-- 3. Single commands — +, -, >, <, ., ,
-- 4. Masking — AND_IMM present/absent based on config.
-- 5. Bounds checks — CMP_GT, CMP_LT, BRANCH_NZ, __trap_oob.
-- 6. Loop compilation — labels, BRANCH_Z, JUMP.
-- 7. Nested loops — unique labels for each loop.
-- 8. Source map — SourceToAst and AstToIr entries.
-- 9. IR printer integration — print_ir produces valid text.
-- 10. Roundtrip — parse(print(result.program)) has same instruction count.
-- 11. Error handling — bad AST root, invalid tape size.
-- 12. Instruction ID uniqueness.
-- ============================================================================

-- IMPORTANT: Set package.path before all requires.
-- Include brainfuck source dir so the lexer loads from source (not the
-- luarocks-installed copy) and can locate the grammar file via debug.getinfo.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" ..
               "../../brainfuck/src/?.lua;" .. "../../brainfuck/src/?/init.lua;" ..
               "../../compiler_ir/src/?.lua;" .. "../../compiler_ir/src/?/init.lua;" ..
               "../../compiler_source_map/src/?.lua;" .. "../../compiler_source_map/src/?/init.lua;" ..
               package.path

local bic = require("coding_adventures.brainfuck_ir_compiler")
local ir  = require("coding_adventures.compiler_ir")
local bf_parser = require("coding_adventures.brainfuck.parser")

-- ============================================================================
-- Test Helpers
-- ============================================================================

--- Parse and compile a Brainfuck source string.
-- Returns result, err.
local function compile_source(source, config)
    local ok, ast_or_err = pcall(bf_parser.parse, source)
    if not ok then
        return nil, tostring(ast_or_err)
    end
    return bic.compile(ast_or_err, "test.bf", config)
end

--- Parse and compile, failing the test on error.
local function must_compile(source, config)
    local result, err = compile_source(source, config)
    assert.is_nil(err, "compile failed: " .. tostring(err))
    assert.is_not_nil(result)
    return result
end

--- Count how many instructions have the given opcode.
local function count_opcode(program, opcode)
    local n = 0
    for _, instr in ipairs(program.instructions) do
        if instr.opcode == opcode then
            n = n + 1
        end
    end
    return n
end

--- Check if the program contains a LABEL instruction with the given name.
local function has_label(program, name)
    for _, instr in ipairs(program.instructions) do
        if instr.opcode == ir.IrOp.LABEL
           and instr.operands[1]
           and instr.operands[1].kind == "label"
           and instr.operands[1].name == name then
            return true
        end
    end
    return false
end

--- Find first instruction with given opcode and optional operand check.
local function find_instruction(program, opcode, operand_check)
    for _, instr in ipairs(program.instructions) do
        if instr.opcode == opcode then
            if not operand_check or operand_check(instr.operands) then
                return instr
            end
        end
    end
    return nil
end

-- ============================================================================
-- Test Suite
-- ============================================================================

describe("brainfuck_ir_compiler", function()

    -- -----------------------------------------------------------------------
    -- BuildConfig
    -- -----------------------------------------------------------------------
    describe("debug_config", function()

        it("has insert_bounds_checks = true", function()
            assert.is_true(bic.debug_config().insert_bounds_checks)
        end)

        it("has insert_debug_locs = true", function()
            assert.is_true(bic.debug_config().insert_debug_locs)
        end)

        it("has mask_byte_arithmetic = true", function()
            assert.is_true(bic.debug_config().mask_byte_arithmetic)
        end)

        it("has tape_size = 30000", function()
            assert.equals(30000, bic.debug_config().tape_size)
        end)

    end)

    describe("release_config", function()

        it("has insert_bounds_checks = false", function()
            assert.is_false(bic.release_config().insert_bounds_checks)
        end)

        it("has insert_debug_locs = false", function()
            assert.is_false(bic.release_config().insert_debug_locs)
        end)

        it("has mask_byte_arithmetic = true", function()
            assert.is_true(bic.release_config().mask_byte_arithmetic)
        end)

        it("has tape_size = 30000", function()
            assert.equals(30000, bic.release_config().tape_size)
        end)

    end)

    describe("new_build_config", function()

        it("mask_byte_arithmetic defaults to true when not specified", function()
            local c = bic.new_build_config({})
            assert.is_true(c.mask_byte_arithmetic)
        end)

        it("mask_byte_arithmetic can be set to false", function()
            local c = bic.new_build_config({ mask_byte_arithmetic = false })
            assert.is_false(c.mask_byte_arithmetic)
        end)

        it("tape_size defaults to 30000", function()
            local c = bic.new_build_config({})
            assert.equals(30000, c.tape_size)
        end)

        it("tape_size can be customized", function()
            local c = bic.new_build_config({ tape_size = 1000 })
            assert.equals(1000, c.tape_size)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Empty program
    -- -----------------------------------------------------------------------
    describe("empty program", function()

        it("has _start label", function()
            local r = must_compile("", bic.release_config())
            assert.is_true(has_label(r.program, "_start"))
        end)

        it("has exactly 1 HALT instruction", function()
            local r = must_compile("", bic.release_config())
            assert.equals(1, count_opcode(r.program, ir.IrOp.HALT))
        end)

        it("has version 1", function()
            local r = must_compile("", bic.release_config())
            assert.equals(1, r.program.version)
        end)

        it("has entry label _start", function()
            local r = must_compile("", bic.release_config())
            assert.equals("_start", r.program.entry_label)
        end)

        it("has 1 data declaration for tape", function()
            local r = must_compile("", bic.release_config())
            assert.equals(1, #r.program.data)
            assert.equals("tape", r.program.data[1].label)
        end)

        it("tape data declaration has size 30000", function()
            local r = must_compile("", bic.release_config())
            assert.equals(30000, r.program.data[1].size)
        end)

        it("tape data declaration has init 0", function()
            local r = must_compile("", bic.release_config())
            assert.equals(0, r.program.data[1].init)
        end)

        it("custom tape size is reflected in data declaration", function()
            local c = bic.new_build_config({ tape_size = 1000 })
            local r = must_compile("", c)
            assert.equals(1000, r.program.data[1].size)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Increment (+)
    -- -----------------------------------------------------------------------
    describe("+ (INC)", function()

        it("emits LOAD_BYTE", function()
            local r = must_compile("+", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.LOAD_BYTE) >= 1)
        end)

        it("emits ADD_IMM with delta=1", function()
            local r = must_compile("+", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.ADD_IMM, function(ops)
                return ops[3] and ops[3].kind == "immediate" and ops[3].value == 1
            end)
            assert.is_not_nil(found, "expected ADD_IMM v2, v2, 1")
        end)

        it("emits AND_IMM with 255 when masking is on", function()
            local r = must_compile("+", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.AND_IMM) >= 1)
        end)

        it("emits STORE_BYTE", function()
            local r = must_compile("+", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.STORE_BYTE) >= 1)
        end)

        it("does NOT emit AND_IMM when masking is off", function()
            local c = bic.new_build_config({ mask_byte_arithmetic = false })
            local r = must_compile("+", c)
            assert.equals(0, count_opcode(r.program, ir.IrOp.AND_IMM))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Decrement (-)
    -- -----------------------------------------------------------------------
    describe("- (DEC)", function()

        it("emits ADD_IMM with delta=-1", function()
            local r = must_compile("-", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.ADD_IMM, function(ops)
                return ops[3] and ops[3].kind == "immediate" and ops[3].value == -1
            end)
            assert.is_not_nil(found, "expected ADD_IMM with -1 for DEC")
        end)

        it("emits STORE_BYTE", function()
            local r = must_compile("-", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.STORE_BYTE) >= 1)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Right (>)
    -- -----------------------------------------------------------------------
    describe("> (RIGHT)", function()

        it("emits ADD_IMM v1, v1, 1", function()
            local r = must_compile(">", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.ADD_IMM, function(ops)
                return ops[1] and ops[1].kind == "register" and ops[1].index == 1
                   and ops[3] and ops[3].kind == "immediate" and ops[3].value == 1
            end)
            assert.is_not_nil(found, "expected ADD_IMM v1, v1, 1 for RIGHT")
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Left (<)
    -- -----------------------------------------------------------------------
    describe("< (LEFT)", function()

        it("emits ADD_IMM v1, v1, -1", function()
            local r = must_compile("<", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.ADD_IMM, function(ops)
                return ops[1] and ops[1].kind == "register" and ops[1].index == 1
                   and ops[3] and ops[3].kind == "immediate" and ops[3].value == -1
            end)
            assert.is_not_nil(found, "expected ADD_IMM v1, v1, -1 for LEFT")
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Output (.)
    -- -----------------------------------------------------------------------
    describe(". (OUTPUT)", function()

        it("emits SYSCALL with value 1 (write)", function()
            local r = must_compile(".", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.SYSCALL, function(ops)
                return ops[1] and ops[1].kind == "immediate" and ops[1].value == 1
            end)
            assert.is_not_nil(found, "expected SYSCALL 1 for OUTPUT")
        end)

        it("emits LOAD_BYTE before the syscall", function()
            local r = must_compile(".", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.LOAD_BYTE) >= 1)
        end)

        it("copies the byte into v4 with ADD_IMM 0 in release mode", function()
            local r = must_compile(".", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.ADD_IMM, function(ops)
                return ops[1] and ops[1].kind == "register" and ops[1].index == 4
                   and ops[2] and ops[2].kind == "register" and ops[2].index == 2
                   and ops[3] and ops[3].kind == "immediate" and ops[3].value == 0
            end)
            assert.is_not_nil(found, "expected ADD_IMM v4, v2, 0 for OUTPUT")
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Input (,)
    -- -----------------------------------------------------------------------
    describe(", (INPUT)", function()

        it("emits SYSCALL with value 2 (read)", function()
            local r = must_compile(",", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.SYSCALL, function(ops)
                return ops[1] and ops[1].kind == "immediate" and ops[1].value == 2
            end)
            assert.is_not_nil(found, "expected SYSCALL 2 for INPUT")
        end)

        it("emits STORE_BYTE after the syscall", function()
            local r = must_compile(",", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.STORE_BYTE) >= 1)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Loop compilation
    -- -----------------------------------------------------------------------
    describe("loop", function()

        it("simple loop [-] has loop_0_start label", function()
            local r = must_compile("[-]", bic.release_config())
            assert.is_true(has_label(r.program, "loop_0_start"))
        end)

        it("simple loop [-] has loop_0_end label", function()
            local r = must_compile("[-]", bic.release_config())
            assert.is_true(has_label(r.program, "loop_0_end"))
        end)

        it("simple loop has BRANCH_Z for loop entry", function()
            local r = must_compile("[-]", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.BRANCH_Z) >= 1)
        end)

        it("simple loop has JUMP for back-edge", function()
            local r = must_compile("[-]", bic.release_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.JUMP) >= 1)
        end)

        it("empty loop [] still has loop labels", function()
            local r = must_compile("[]", bic.release_config())
            assert.is_true(has_label(r.program, "loop_0_start"))
            assert.is_true(has_label(r.program, "loop_0_end"))
        end)

        it("nested loops have distinct labels", function()
            local r = must_compile("[>[+<-]]", bic.release_config())
            assert.is_true(has_label(r.program, "loop_0_start"))
            assert.is_true(has_label(r.program, "loop_1_start"))
        end)

        it("nested loops have two BRANCH_Z instructions", function()
            local r = must_compile("[[-]]", bic.release_config())
            assert.equals(2, count_opcode(r.program, ir.IrOp.BRANCH_Z))
        end)

        it("nested loops have two JUMP instructions", function()
            local r = must_compile("[[-]]", bic.release_config())
            assert.equals(2, count_opcode(r.program, ir.IrOp.JUMP))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Bounds checking (debug mode)
    -- -----------------------------------------------------------------------
    describe("bounds checking", function()

        it("> in debug mode emits CMP_GT", function()
            local r = must_compile(">", bic.debug_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.CMP_GT) >= 1)
        end)

        it("> in debug mode emits BRANCH_NZ", function()
            local r = must_compile(">", bic.debug_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.BRANCH_NZ) >= 1)
        end)

        it("> in debug mode emits __trap_oob label", function()
            local r = must_compile(">", bic.debug_config())
            assert.is_true(has_label(r.program, "__trap_oob"))
        end)

        it("< in debug mode emits CMP_LT", function()
            local r = must_compile("<", bic.debug_config())
            assert.is_true(count_opcode(r.program, ir.IrOp.CMP_LT) >= 1)
        end)

        it("> in release mode does NOT emit CMP_GT", function()
            local r = must_compile("><", bic.release_config())
            assert.equals(0, count_opcode(r.program, ir.IrOp.CMP_GT))
        end)

        it("> in release mode does NOT emit CMP_LT", function()
            local r = must_compile("><", bic.release_config())
            assert.equals(0, count_opcode(r.program, ir.IrOp.CMP_LT))
        end)

        it("release mode does NOT emit __trap_oob label", function()
            local r = must_compile("><", bic.release_config())
            assert.is_false(has_label(r.program, "__trap_oob"))
        end)

        it("debug mode sets up v5 (max pointer) and v6 (zero)", function()
            local r = must_compile("", bic.debug_config())
            -- Two extra LOAD_IMM instructions in prologue for v5 and v6
            -- (v0 = LOAD_ADDR, v1 = LOAD_IMM 0, v5 = LOAD_IMM max, v6 = LOAD_IMM 0)
            local load_imm_count = count_opcode(r.program, ir.IrOp.LOAD_IMM)
            -- At minimum 3: v1=0, v5=tape_size-1, v6=0
            assert.is_true(load_imm_count >= 3)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Source map
    -- -----------------------------------------------------------------------
    describe("source map", function()

        it("+. produces 2 SourceToAst entries", function()
            local r = must_compile("+.", bic.release_config())
            assert.equals(2, #r.source_map.source_to_ast.entries)
        end)

        it("first source entry is at column 1 (the +)", function()
            local r = must_compile("+.", bic.release_config())
            assert.equals(1, r.source_map.source_to_ast.entries[1].pos.column)
        end)

        it("second source entry is at column 2 (the .)", function()
            local r = must_compile("+.", bic.release_config())
            assert.equals(2, r.source_map.source_to_ast.entries[2].pos.column)
        end)

        it("source entries have correct filename", function()
            local r = must_compile("+", bic.release_config())
            assert.equals("test.bf", r.source_map.source_to_ast.entries[1].pos.file)
        end)

        it("+ produces 1 AstToIr entry", function()
            local r = must_compile("+", bic.release_config())
            assert.equals(1, #r.source_map.ast_to_ir.entries)
        end)

        it("+ with masking produces 4 IR IDs (LOAD, ADD_IMM, AND_IMM, STORE)", function()
            local r = must_compile("+", bic.release_config())
            assert.equals(1, #r.source_map.ast_to_ir.entries)
            assert.equals(4, #r.source_map.ast_to_ir.entries[1].ir_ids)
        end)

        it("+ without masking produces 3 IR IDs (LOAD, ADD_IMM, STORE)", function()
            local c = bic.new_build_config({ mask_byte_arithmetic = false })
            local r = must_compile("+", c)
            assert.equals(3, #r.source_map.ast_to_ir.entries[1].ir_ids)
        end)

        it("loop and body command produce entries", function()
            local r = must_compile("[-]", bic.release_config())
            -- Loop + 1 command ("-") = at least 2 SourceToAst entries
            assert.is_true(#r.source_map.source_to_ast.entries >= 2)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IR printer integration
    -- -----------------------------------------------------------------------
    describe("IR printer integration", function()

        it("compiled IR is printable with .version 1", function()
            local r = must_compile("+.", bic.release_config())
            local text = ir.print_ir(r.program)
            assert.is_truthy(text:find("%.version 1"))
        end)

        it("printed IR contains .data tape 30000 0", function()
            local r = must_compile("", bic.release_config())
            local text = ir.print_ir(r.program)
            assert.is_truthy(text:find("%.data tape 30000 0"))
        end)

        it("printed IR contains .entry _start", function()
            local r = must_compile("", bic.release_config())
            local text = ir.print_ir(r.program)
            assert.is_truthy(text:find("%.entry _start"))
        end)

        it("printed IR contains HALT", function()
            local r = must_compile("", bic.release_config())
            local text = ir.print_ir(r.program)
            assert.is_truthy(text:find("HALT"))
        end)

        it("printed IR for + contains LOAD_BYTE", function()
            local r = must_compile("+", bic.release_config())
            local text = ir.print_ir(r.program)
            assert.is_truthy(text:find("LOAD_BYTE"))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Roundtrip
    -- -----------------------------------------------------------------------
    describe("roundtrip (print → parse)", function()

        it("parse(print(prog)) has same instruction count for ++[-].", function()
            local r = must_compile("++[-].", bic.release_config())
            local text = ir.print_ir(r.program)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err, "roundtrip parse failed: " .. tostring(err))
            assert.equals(#r.program.instructions, #prog2.instructions)
        end)

        it("parse(print(prog)) works for complex program", function()
            local r = must_compile("++++++++[>+++++++++<-]>.", bic.release_config())
            local text = ir.print_ir(r.program)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            assert.equals(#r.program.instructions, #prog2.instructions)
        end)

        it("parse(print(prog)) works for debug config", function()
            local r = must_compile("+>-<", bic.debug_config())
            local text = ir.print_ir(r.program)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            assert.equals(#r.program.instructions, #prog2.instructions)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Instruction ID uniqueness
    -- -----------------------------------------------------------------------
    describe("instruction ID uniqueness", function()

        it("all non-label instruction IDs are unique", function()
            local r = must_compile("++[>+<-].", bic.release_config())
            local seen = {}
            for _, instr in ipairs(r.program.instructions) do
                if instr.id ~= -1 then
                    assert.is_nil(seen[instr.id],
                        "duplicate instruction ID: " .. tostring(instr.id))
                    seen[instr.id] = true
                end
            end
        end)

        it("IDs start from 0", function()
            local r = must_compile("+", bic.release_config())
            local min_id = math.huge
            for _, instr in ipairs(r.program.instructions) do
                if instr.id ~= -1 and instr.id < min_id then
                    min_id = instr.id
                end
            end
            assert.equals(0, min_id)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Error handling
    -- -----------------------------------------------------------------------
    describe("error handling", function()

        it("returns error for non-program AST root", function()
            -- Build a fake AST node with wrong rule_name
            local fake_ast = { rule_name = "not_a_program", children = {} }
            local result, err = bic.compile(fake_ast, "test.bf", bic.release_config())
            assert.is_nil(result)
            assert.is_not_nil(err)
            assert.is_truthy(err:find("program"))
        end)

        it("returns error for zero tape_size", function()
            local ok, ast_or_err = pcall(bf_parser.parse, bf_parser, "")
            assert.is_true(ok)
            local c = bic.new_build_config({ tape_size = 0 })
            local result, err = bic.compile(ast_or_err, "test.bf", c)
            assert.is_nil(result)
            assert.is_not_nil(err)
        end)

        it("returns error for negative tape_size", function()
            local ok, ast_or_err = pcall(bf_parser.parse, bf_parser, "")
            assert.is_true(ok)
            local c = bic.new_build_config({ tape_size = -1 })
            local result, err = bic.compile(ast_or_err, "test.bf", c)
            assert.is_nil(result)
            assert.is_not_nil(err)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Classic programs
    -- -----------------------------------------------------------------------
    describe("classic programs", function()

        it("hello world fragment has loop_0_start", function()
            -- 72 = 8 * 9: ++++++++[>+++++++++<-]>.
            local r = must_compile("++++++++[>+++++++++<-]>.", bic.release_config())
            assert.is_true(has_label(r.program, "loop_0_start"))
        end)

        it("hello world fragment has SYSCALL 1 for output", function()
            local r = must_compile("++++++++[>+++++++++<-]>.", bic.release_config())
            local found = find_instruction(r.program, ir.IrOp.SYSCALL, function(ops)
                return ops[1] and ops[1].value == 1
            end)
            assert.is_not_nil(found, "expected SYSCALL 1 for output")
        end)

        it("cat program ,[.,] has both read and write syscalls", function()
            local r = must_compile(",[.,]", bic.release_config())
            local found_read = find_instruction(r.program, ir.IrOp.SYSCALL, function(ops)
                return ops[1] and ops[1].value == 2
            end)
            local found_write = find_instruction(r.program, ir.IrOp.SYSCALL, function(ops)
                return ops[1] and ops[1].value == 1
            end)
            assert.is_not_nil(found_read, "expected SYSCALL 2 (read)")
            assert.is_not_nil(found_write, "expected SYSCALL 1 (write)")
        end)

    end)

end)

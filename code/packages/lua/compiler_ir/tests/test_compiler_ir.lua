-- ============================================================================
-- Tests for compiler_ir — IR types, opcodes, printer, and parser
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. IrOp table — all 25 opcodes are present and have correct values.
-- 2. IrOpName — inverse mapping is correct for all opcodes.
-- 3. Operand constructors — register, immediate, label.
-- 4. operand_tostring — correct string representation.
-- 5. IrInstruction — new_instruction constructor.
-- 6. IrDataDecl — new_data_decl constructor.
-- 7. IrProgram — new_program, add_instruction, add_data.
-- 8. IDGenerator — next_id, current_id, from-start.
-- 9. Printer — print_ir produces correct text format.
-- 10. Parser — parse_ir reconstructs a program from text.
-- 11. Roundtrip — parse(print(prog)) == prog (instruction count).
-- 12. Parser error handling — malformed input returns errors.
-- ============================================================================

-- IMPORTANT: Set package.path so require works when busted runs from tests/.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local ir = require("coding_adventures.compiler_ir")

describe("compiler_ir", function()

    -- -----------------------------------------------------------------------
    -- IrOp table
    -- -----------------------------------------------------------------------
    describe("IrOp", function()

        it("has all 25 opcodes", function()
            local count = 0
            for _ in pairs(ir.IrOp) do count = count + 1 end
            assert.equals(25, count)
        end)

        it("LOAD_IMM is 0", function()
            assert.equals(0, ir.IrOp.LOAD_IMM)
        end)

        it("LOAD_ADDR is 1", function()
            assert.equals(1, ir.IrOp.LOAD_ADDR)
        end)

        it("LOAD_BYTE is 2", function()
            assert.equals(2, ir.IrOp.LOAD_BYTE)
        end)

        it("STORE_BYTE is 3", function()
            assert.equals(3, ir.IrOp.STORE_BYTE)
        end)

        it("ADD is 6", function()
            assert.equals(6, ir.IrOp.ADD)
        end)

        it("ADD_IMM is 7", function()
            assert.equals(7, ir.IrOp.ADD_IMM)
        end)

        it("AND_IMM is 10", function()
            assert.equals(10, ir.IrOp.AND_IMM)
        end)

        it("LABEL is 15", function()
            assert.equals(15, ir.IrOp.LABEL)
        end)

        it("BRANCH_Z is 17", function()
            assert.equals(17, ir.IrOp.BRANCH_Z)
        end)

        it("BRANCH_NZ is 18", function()
            assert.equals(18, ir.IrOp.BRANCH_NZ)
        end)

        it("SYSCALL is 21", function()
            assert.equals(21, ir.IrOp.SYSCALL)
        end)

        it("HALT is 22", function()
            assert.equals(22, ir.IrOp.HALT)
        end)

        it("COMMENT is 24", function()
            assert.equals(24, ir.IrOp.COMMENT)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IrOpName — inverse mapping
    -- -----------------------------------------------------------------------
    describe("IrOpName", function()

        it("maps 0 back to LOAD_IMM", function()
            assert.equals("LOAD_IMM", ir.IrOpName[0])
        end)

        it("maps 22 back to HALT", function()
            assert.equals("HALT", ir.IrOpName[22])
        end)

        it("maps 15 back to LABEL", function()
            assert.equals("LABEL", ir.IrOpName[15])
        end)

        it("has 25 entries", function()
            local count = 0
            for _ in pairs(ir.IrOpName) do count = count + 1 end
            assert.equals(25, count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Operand constructors
    -- -----------------------------------------------------------------------
    describe("new_register", function()

        it("creates a register operand with kind=register", function()
            local r = ir.new_register(0)
            assert.equals("register", r.kind)
            assert.equals(0, r.index)
        end)

        it("stores the register index", function()
            local r = ir.new_register(5)
            assert.equals(5, r.index)
        end)

    end)

    describe("new_immediate", function()

        it("creates an immediate operand with kind=immediate", function()
            local imm = ir.new_immediate(42)
            assert.equals("immediate", imm.kind)
            assert.equals(42, imm.value)
        end)

        it("handles negative values", function()
            local imm = ir.new_immediate(-1)
            assert.equals(-1, imm.value)
        end)

        it("handles zero", function()
            local imm = ir.new_immediate(0)
            assert.equals(0, imm.value)
        end)

        it("handles 255", function()
            local imm = ir.new_immediate(255)
            assert.equals(255, imm.value)
        end)

    end)

    describe("new_label", function()

        it("creates a label operand with kind=label", function()
            local lbl = ir.new_label("_start")
            assert.equals("label", lbl.kind)
            assert.equals("_start", lbl.name)
        end)

        it("stores arbitrary label names", function()
            local lbl = ir.new_label("loop_0_end")
            assert.equals("loop_0_end", lbl.name)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- operand_tostring
    -- -----------------------------------------------------------------------
    describe("operand_tostring", function()

        it("formats register v0", function()
            assert.equals("v0", ir.operand_tostring(ir.new_register(0)))
        end)

        it("formats register v5", function()
            assert.equals("v5", ir.operand_tostring(ir.new_register(5)))
        end)

        it("formats immediate 42", function()
            assert.equals("42", ir.operand_tostring(ir.new_immediate(42)))
        end)

        it("formats negative immediate -1", function()
            assert.equals("-1", ir.operand_tostring(ir.new_immediate(-1)))
        end)

        it("formats label _start", function()
            assert.equals("_start", ir.operand_tostring(ir.new_label("_start")))
        end)

        it("formats label loop_0_end", function()
            assert.equals("loop_0_end", ir.operand_tostring(ir.new_label("loop_0_end")))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IrInstruction
    -- -----------------------------------------------------------------------
    describe("new_instruction", function()

        it("stores opcode, operands, and id", function()
            local operands = { ir.new_register(0), ir.new_immediate(42) }
            local instr = ir.new_instruction(ir.IrOp.LOAD_IMM, operands, 3)
            assert.equals(ir.IrOp.LOAD_IMM, instr.opcode)
            assert.equals(2, #instr.operands)
            assert.equals(3, instr.id)
        end)

        it("defaults operands to empty table when nil", function()
            local instr = ir.new_instruction(ir.IrOp.HALT, nil, 0)
            assert.is_table(instr.operands)
            assert.equals(0, #instr.operands)
        end)

        it("accepts -1 id for labels", function()
            local instr = ir.new_instruction(
                ir.IrOp.LABEL,
                { ir.new_label("_start") },
                -1
            )
            assert.equals(-1, instr.id)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IrDataDecl
    -- -----------------------------------------------------------------------
    describe("new_data_decl", function()

        it("stores label, size, and init", function()
            local decl = ir.new_data_decl("tape", 30000, 0)
            assert.equals("tape", decl.label)
            assert.equals(30000, decl.size)
            assert.equals(0, decl.init)
        end)

        it("defaults init to 0", function()
            local decl = ir.new_data_decl("buf", 1024)
            assert.equals(0, decl.init)
        end)

        it("accepts non-zero init", function()
            local decl = ir.new_data_decl("ff_buf", 256, 255)
            assert.equals(255, decl.init)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IrProgram
    -- -----------------------------------------------------------------------
    describe("new_program", function()

        it("creates a program with the given entry label", function()
            local prog = ir.new_program("_start")
            assert.equals("_start", prog.entry_label)
        end)

        it("starts with version 1", function()
            local prog = ir.new_program("_start")
            assert.equals(1, prog.version)
        end)

        it("starts with empty instruction list", function()
            local prog = ir.new_program("_start")
            assert.equals(0, #prog.instructions)
        end)

        it("starts with empty data list", function()
            local prog = ir.new_program("_start")
            assert.equals(0, #prog.data)
        end)

    end)

    describe("add_instruction", function()

        it("appends instructions in order", function()
            local prog = ir.new_program("_start")
            local i1 = ir.new_instruction(ir.IrOp.HALT, {}, 0)
            local i2 = ir.new_instruction(ir.IrOp.NOP, {}, 1)
            ir.add_instruction(prog, i1)
            ir.add_instruction(prog, i2)
            assert.equals(2, #prog.instructions)
            assert.equals(ir.IrOp.HALT, prog.instructions[1].opcode)
            assert.equals(ir.IrOp.NOP, prog.instructions[2].opcode)
        end)

    end)

    describe("add_data", function()

        it("appends data declarations in order", function()
            local prog = ir.new_program("_start")
            ir.add_data(prog, ir.new_data_decl("tape", 30000, 0))
            assert.equals(1, #prog.data)
            assert.equals("tape", prog.data[1].label)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- IDGenerator
    -- -----------------------------------------------------------------------
    describe("IDGenerator", function()

        it("starts at 0", function()
            local gen = ir.new_id_generator()
            assert.equals(0, ir.current_id(gen))
        end)

        it("next_id returns 0 on first call", function()
            local gen = ir.new_id_generator()
            assert.equals(0, ir.next_id(gen))
        end)

        it("next_id increments counter", function()
            local gen = ir.new_id_generator()
            ir.next_id(gen)
            assert.equals(1, ir.current_id(gen))
        end)

        it("returns monotonically increasing IDs", function()
            local gen = ir.new_id_generator()
            local ids = {}
            for _ = 1, 10 do
                ids[#ids + 1] = ir.next_id(gen)
            end
            for i = 1, #ids - 1 do
                assert.is_true(ids[i] < ids[i + 1])
            end
        end)

        it("IDs are all unique", function()
            local gen = ir.new_id_generator()
            local seen = {}
            for _ = 1, 100 do
                local id = ir.next_id(gen)
                assert.is_nil(seen[id], "duplicate ID: " .. tostring(id))
                seen[id] = true
            end
        end)

        it("new_id_generator_from starts at given value", function()
            local gen = ir.new_id_generator_from(50)
            assert.equals(50, ir.next_id(gen))
            assert.equals(51, ir.next_id(gen))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Printer
    -- -----------------------------------------------------------------------
    describe("print_ir", function()

        local function make_simple_prog()
            local prog = ir.new_program("_start")
            ir.add_data(prog, ir.new_data_decl("tape", 30000, 0))
            -- _start label
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LABEL, { ir.new_label("_start") }, -1))
            -- LOAD_ADDR v0, tape
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LOAD_ADDR,
                    { ir.new_register(0), ir.new_label("tape") }, 0))
            -- HALT
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.HALT, {}, 1))
            return prog
        end

        it("output contains .version 1", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("%.version 1"))
        end)

        it("output contains .data tape 30000 0", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("%.data tape 30000 0"))
        end)

        it("output contains .entry _start", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("%.entry _start"))
        end)

        it("output contains _start label", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("_start:"))
        end)

        it("output contains LOAD_ADDR instruction", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("LOAD_ADDR"))
        end)

        it("output contains HALT instruction", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("HALT"))
        end)

        it("output contains instruction ID comment", function()
            local text = ir.print_ir(make_simple_prog())
            assert.is_truthy(text:find("; #0"))
            assert.is_truthy(text:find("; #1"))
        end)

        it("COMMENT instruction emits as ; text", function()
            local prog = ir.new_program("_start")
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.COMMENT,
                    { ir.new_label("hello world") }, -1))
            local text = ir.print_ir(prog)
            assert.is_truthy(text:find("; hello world"))
        end)

        it("output ends with newline", function()
            local text = ir.print_ir(make_simple_prog())
            assert.equals("\n", text:sub(-1))
        end)

        it("operands are comma-separated", function()
            local prog = ir.new_program("_start")
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.ADD_IMM,
                    { ir.new_register(1), ir.new_register(1), ir.new_immediate(1) },
                    0))
            local text = ir.print_ir(prog)
            assert.is_truthy(text:find("v1, v1, 1"))
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Parser
    -- -----------------------------------------------------------------------
    describe("parse_ir", function()

        it("parses .version directive", function()
            local prog, err = ir.parse_ir(".version 1\n.entry _start\n")
            assert.is_nil(err)
            assert.equals(1, prog.version)
        end)

        it("parses .data directive", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.data tape 30000 0\n.entry _start\n")
            assert.is_nil(err)
            assert.equals(1, #prog.data)
            assert.equals("tape", prog.data[1].label)
            assert.equals(30000, prog.data[1].size)
            assert.equals(0, prog.data[1].init)
        end)

        it("parses .entry directive", function()
            local prog, err = ir.parse_ir(".version 1\n.entry _start\n")
            assert.is_nil(err)
            assert.equals("_start", prog.entry_label)
        end)

        it("parses label definition", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n_start:\n")
            assert.is_nil(err)
            assert.equals(1, #prog.instructions)
            assert.equals(ir.IrOp.LABEL, prog.instructions[1].opcode)
        end)

        it("parses HALT instruction", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n  HALT  ; #0\n")
            assert.is_nil(err)
            assert.equals(1, #prog.instructions)
            assert.equals(ir.IrOp.HALT, prog.instructions[1].opcode)
            assert.equals(0, prog.instructions[1].id)
        end)

        it("parses instruction with register operands", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n  LOAD_BYTE v2, v0, v1  ; #5\n")
            assert.is_nil(err)
            local instr = prog.instructions[1]
            assert.equals(ir.IrOp.LOAD_BYTE, instr.opcode)
            assert.equals("register", instr.operands[1].kind)
            assert.equals(2, instr.operands[1].index)
            assert.equals(5, instr.id)
        end)

        it("parses instruction with immediate operand", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n  ADD_IMM v1, v1, 1  ; #2\n")
            assert.is_nil(err)
            local instr = prog.instructions[1]
            assert.equals(ir.IrOp.ADD_IMM, instr.opcode)
            assert.equals("immediate", instr.operands[3].kind)
            assert.equals(1, instr.operands[3].value)
        end)

        it("parses instruction with negative immediate", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n  ADD_IMM v1, v1, -1  ; #3\n")
            assert.is_nil(err)
            local instr = prog.instructions[1]
            assert.equals(-1, instr.operands[3].value)
        end)

        it("parses instruction with label operand", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n  LOAD_ADDR v0, tape  ; #0\n")
            assert.is_nil(err)
            local instr = prog.instructions[1]
            assert.equals(ir.IrOp.LOAD_ADDR, instr.opcode)
            assert.equals("label", instr.operands[2].kind)
            assert.equals("tape", instr.operands[2].name)
        end)

        it("returns error for unknown opcode", function()
            local prog, err = ir.parse_ir(
                ".version 1\n.entry _start\n  FOOBAR v0  ; #0\n")
            assert.is_nil(prog)
            assert.is_not_nil(err)
            assert.is_truthy(err:find("unknown opcode"))
        end)

        it("returns error for invalid .version directive", function()
            local prog, err = ir.parse_ir(".version\n")
            assert.is_nil(prog)
            assert.is_not_nil(err)
        end)

        it("skips blank lines", function()
            local prog, err = ir.parse_ir(
                ".version 1\n\n.entry _start\n\n  HALT  ; #0\n")
            assert.is_nil(err)
            assert.equals(1, #prog.instructions)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Roundtrip: print_ir → parse_ir
    -- -----------------------------------------------------------------------
    describe("roundtrip", function()

        local function make_full_prog()
            local prog = ir.new_program("_start")
            ir.add_data(prog, ir.new_data_decl("tape", 30000, 0))
            local gen = ir.new_id_generator()

            -- _start label
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LABEL, { ir.new_label("_start") }, -1))
            -- LOAD_ADDR v0, tape
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LOAD_ADDR,
                    { ir.new_register(0), ir.new_label("tape") },
                    ir.next_id(gen)))
            -- LOAD_IMM v1, 0
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LOAD_IMM,
                    { ir.new_register(1), ir.new_immediate(0) },
                    ir.next_id(gen)))
            -- loop_0_start label
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LABEL,
                    { ir.new_label("loop_0_start") }, -1))
            -- LOAD_BYTE v2, v0, v1
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LOAD_BYTE,
                    { ir.new_register(2), ir.new_register(0), ir.new_register(1) },
                    ir.next_id(gen)))
            -- BRANCH_Z v2, loop_0_end
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.BRANCH_Z,
                    { ir.new_register(2), ir.new_label("loop_0_end") },
                    ir.next_id(gen)))
            -- ADD_IMM v2, v2, 1
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.ADD_IMM,
                    { ir.new_register(2), ir.new_register(2), ir.new_immediate(1) },
                    ir.next_id(gen)))
            -- AND_IMM v2, v2, 255
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.AND_IMM,
                    { ir.new_register(2), ir.new_register(2), ir.new_immediate(255) },
                    ir.next_id(gen)))
            -- STORE_BYTE v2, v0, v1
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.STORE_BYTE,
                    { ir.new_register(2), ir.new_register(0), ir.new_register(1) },
                    ir.next_id(gen)))
            -- JUMP loop_0_start
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.JUMP,
                    { ir.new_label("loop_0_start") },
                    ir.next_id(gen)))
            -- loop_0_end label
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.LABEL,
                    { ir.new_label("loop_0_end") }, -1))
            -- HALT
            ir.add_instruction(prog,
                ir.new_instruction(ir.IrOp.HALT, {}, ir.next_id(gen)))

            return prog
        end

        it("parse(print(prog)) has same instruction count", function()
            local prog = make_full_prog()
            local text = ir.print_ir(prog)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err, "parse error: " .. tostring(err))
            assert.equals(#prog.instructions, #prog2.instructions)
        end)

        it("parse(print(prog)) preserves data declarations", function()
            local prog = make_full_prog()
            local text = ir.print_ir(prog)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            assert.equals(#prog.data, #prog2.data)
            assert.equals(prog.data[1].label, prog2.data[1].label)
            assert.equals(prog.data[1].size, prog2.data[1].size)
        end)

        it("parse(print(prog)) preserves entry label", function()
            local prog = make_full_prog()
            local text = ir.print_ir(prog)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            assert.equals(prog.entry_label, prog2.entry_label)
        end)

        it("parse(print(prog)) preserves version", function()
            local prog = make_full_prog()
            local text = ir.print_ir(prog)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            assert.equals(prog.version, prog2.version)
        end)

        it("parse(print(prog)) preserves instruction opcodes", function()
            local prog = make_full_prog()
            local text = ir.print_ir(prog)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            for i = 1, #prog.instructions do
                assert.equals(prog.instructions[i].opcode, prog2.instructions[i].opcode,
                    "opcode mismatch at instruction " .. i)
            end
        end)

        it("parse(print(prog)) preserves instruction IDs", function()
            local prog = make_full_prog()
            local text = ir.print_ir(prog)
            local prog2, err = ir.parse_ir(text)
            assert.is_nil(err)
            for i = 1, #prog.instructions do
                assert.equals(prog.instructions[i].id, prog2.instructions[i].id,
                    "id mismatch at instruction " .. i)
            end
        end)

    end)

end)

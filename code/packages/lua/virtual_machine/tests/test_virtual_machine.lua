-- Tests for virtual-machine.
--
-- Comprehensive busted test suite covering both VirtualMachine (hard-coded
-- opcodes) and GenericVM (handler-based pluggable opcodes).  Targets 95%+
-- line coverage.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local vm_mod = require("coding_adventures.virtual_machine")

-- Shorthand aliases for readability
local I = vm_mod.instruction
local AC = vm_mod.assemble_code
local VM = vm_mod.VirtualMachine
local GVM = vm_mod.GenericVM

-- Opcode aliases
local LOAD_CONST   = vm_mod.OP_LOAD_CONST
local POP          = vm_mod.OP_POP
local DUP          = vm_mod.OP_DUP
local STORE_NAME   = vm_mod.OP_STORE_NAME
local LOAD_NAME    = vm_mod.OP_LOAD_NAME
local STORE_LOCAL  = vm_mod.OP_STORE_LOCAL
local LOAD_LOCAL   = vm_mod.OP_LOAD_LOCAL
local ADD          = vm_mod.OP_ADD
local SUB          = vm_mod.OP_SUB
local MUL          = vm_mod.OP_MUL
local DIV          = vm_mod.OP_DIV
local CMP_EQ       = vm_mod.OP_CMP_EQ
local CMP_LT       = vm_mod.OP_CMP_LT
local CMP_GT       = vm_mod.OP_CMP_GT
local JUMP         = vm_mod.OP_JUMP
local JUMP_IF_FALSE = vm_mod.OP_JUMP_IF_FALSE
local JUMP_IF_TRUE  = vm_mod.OP_JUMP_IF_TRUE
local CALL         = vm_mod.OP_CALL
local RETURN       = vm_mod.OP_RETURN
local PRINT        = vm_mod.OP_PRINT
local HALT         = vm_mod.OP_HALT


-- =========================================================================
-- MODULE BASICS
-- =========================================================================

describe("module", function()
    it("has a version", function()
        assert.are.equal("0.1.0", vm_mod.VERSION)
    end)

    it("exports all opcode constants", function()
        assert.are.equal(0x01, LOAD_CONST)
        assert.are.equal(0x02, POP)
        assert.are.equal(0x03, DUP)
        assert.are.equal(0x10, STORE_NAME)
        assert.are.equal(0x11, LOAD_NAME)
        assert.are.equal(0x12, STORE_LOCAL)
        assert.are.equal(0x13, LOAD_LOCAL)
        assert.are.equal(0x20, ADD)
        assert.are.equal(0x21, SUB)
        assert.are.equal(0x22, MUL)
        assert.are.equal(0x23, DIV)
        assert.are.equal(0x30, CMP_EQ)
        assert.are.equal(0x31, CMP_LT)
        assert.are.equal(0x32, CMP_GT)
        assert.are.equal(0x40, JUMP)
        assert.are.equal(0x41, JUMP_IF_FALSE)
        assert.are.equal(0x42, JUMP_IF_TRUE)
        assert.are.equal(0x50, CALL)
        assert.are.equal(0x51, RETURN)
        assert.are.equal(0x60, PRINT)
        assert.are.equal(0xFF, HALT)
    end)
end)


-- =========================================================================
-- INSTRUCTION
-- =========================================================================

describe("instruction", function()
    it("creates an instruction with opcode and operand", function()
        local instr = I(LOAD_CONST, 1)
        assert.are.equal(LOAD_CONST, instr.opcode)
        assert.are.equal(1, instr.operand)
    end)

    it("creates an instruction with opcode only", function()
        local instr = I(ADD)
        assert.are.equal(ADD, instr.opcode)
        assert.is_nil(instr.operand)
    end)

    it("formats with operand", function()
        local s = vm_mod.instruction_tostring(I(LOAD_CONST, 1))
        assert.are.equal("Instruction(0x01, 1)", s)
    end)

    it("formats without operand", function()
        local s = vm_mod.instruction_tostring(I(ADD))
        assert.are.equal("Instruction(0x20)", s)
    end)
end)


-- =========================================================================
-- CODE OBJECT (assemble_code)
-- =========================================================================

describe("assemble_code", function()
    it("bundles instructions, constants, and names", function()
        local code = AC(
            { I(LOAD_CONST, 1), I(HALT) },
            { 42 },
            { "x" }
        )
        assert.are.equal(2, #code.instructions)
        assert.are.equal(42, code.constants[1])
        assert.are.equal("x", code.names[1])
    end)

    it("defaults constants and names to empty tables", function()
        local code = AC({ I(HALT) })
        assert.are.same({}, code.constants)
        assert.are.same({}, code.names)
    end)

    it("defaults everything to empty tables when called with nil", function()
        local code = AC()
        assert.are.same({}, code.instructions)
        assert.are.same({}, code.constants)
        assert.are.same({}, code.names)
    end)
end)


-- =========================================================================
-- HELPER FUNCTIONS
-- =========================================================================

describe("is_falsy", function()
    it("nil is falsy", function()
        assert.is_true(vm_mod.is_falsy(nil))
    end)

    it("0 is falsy", function()
        assert.is_true(vm_mod.is_falsy(0))
    end)

    it("empty string is falsy", function()
        assert.is_true(vm_mod.is_falsy(""))
    end)

    it("1 is truthy", function()
        assert.is_false(vm_mod.is_falsy(1))
    end)

    it("-1 is truthy", function()
        assert.is_false(vm_mod.is_falsy(-1))
    end)

    it("non-empty string is truthy", function()
        assert.is_false(vm_mod.is_falsy("hello"))
    end)

    it("table is truthy", function()
        assert.is_false(vm_mod.is_falsy({}))
    end)
end)

describe("vm_add", function()
    it("adds two numbers", function()
        assert.are.equal(7, vm_mod.vm_add(3, 4))
    end)

    it("concatenates two strings", function()
        assert.are.equal("foobar", vm_mod.vm_add("foo", "bar"))
    end)

    it("errors on mixed types", function()
        assert.has_error(function() vm_mod.vm_add(1, "bar") end, "TypeError: Cannot add number and string")
    end)
end)

describe("copy_array", function()
    it("returns a shallow copy", function()
        local orig = {1, 2, 3}
        local c = vm_mod.copy_array(orig)
        assert.are.same(orig, c)
        -- Verify it is a different table
        c[1] = 99
        assert.are.equal(1, orig[1])
    end)

    it("copies empty array", function()
        assert.are.same({}, vm_mod.copy_array({}))
    end)
end)

describe("copy_map", function()
    it("returns a shallow copy", function()
        local orig = { a = 1, b = 2 }
        local c = vm_mod.copy_map(orig)
        assert.are.same(orig, c)
        c.a = 99
        assert.are.equal(1, orig.a)
    end)
end)


-- =========================================================================
-- VIRTUAL MACHINE -- Opcode Tests
-- =========================================================================

describe("VirtualMachine", function()
    local vm

    before_each(function()
        vm = VM.new()
    end)

    -- ----- Stack manipulation -----

    describe("LOAD_CONST", function()
        it("pushes a constant onto the stack", function()
            local code = AC({ I(LOAD_CONST, 1), I(HALT) }, { 42 })
            vm:execute(code)
            assert.are.equal(42, vm.stack[1])
        end)

        it("errors on out-of-bounds index", function()
            local code = AC({ I(LOAD_CONST, 5), I(HALT) }, { 42 })
            assert.has_error(function() vm:execute(code) end, "InvalidOperandError: LOAD_CONST 5 out of bounds")
        end)

        it("errors on index 0", function()
            local code = AC({ I(LOAD_CONST, 0), I(HALT) }, { 42 })
            assert.has_error(function() vm:execute(code) end, "InvalidOperandError: LOAD_CONST 0 out of bounds")
        end)
    end)

    describe("POP", function()
        it("discards top of stack", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(POP),
                I(HALT),
            }, { 10, 20 })
            vm:execute(code)
            assert.are.equal(1, #vm.stack)
            assert.are.equal(10, vm.stack[1])
        end)

        it("errors on empty stack", function()
            local code = AC({ I(POP), I(HALT) })
            assert.has_error(function() vm:execute(code) end, "StackUnderflowError")
        end)
    end)

    describe("DUP", function()
        it("duplicates top of stack", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(DUP),
                I(HALT),
            }, { 7 })
            vm:execute(code)
            assert.are.equal(2, #vm.stack)
            assert.are.equal(7, vm.stack[1])
            assert.are.equal(7, vm.stack[2])
        end)

        it("errors on empty stack", function()
            local code = AC({ I(DUP), I(HALT) })
            assert.has_error(function() vm:execute(code) end, "StackUnderflowError: DUP empty stack")
        end)
    end)

    -- ----- Variable access -----

    describe("STORE_NAME / LOAD_NAME", function()
        it("stores and loads a variable", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(STORE_NAME, 1),
                I(LOAD_NAME, 1),
                I(HALT),
            }, { 99 }, { "x" })
            vm:execute(code)
            assert.are.equal(99, vm.stack[1])
            assert.are.equal(99, vm.variables["x"])
        end)

        it("errors on undefined variable", function()
            local code = AC({
                I(LOAD_NAME, 1),
                I(HALT),
            }, {}, { "undefined_var" })
            assert.has_error(function() vm:execute(code) end,
                "UndefinedNameError: Variable 'undefined_var' is not defined")
        end)
    end)

    describe("STORE_LOCAL / LOAD_LOCAL", function()
        it("stores and loads a local slot", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(STORE_LOCAL, 1),
                I(LOAD_LOCAL, 1),
                I(HALT),
            }, { 55 })
            vm:execute(code)
            assert.are.equal(55, vm.stack[1])
        end)

        it("extends locals array if needed", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(STORE_LOCAL, 3),
                I(HALT),
            }, { 77 })
            vm:execute(code)
            assert.are.equal(77, vm.locals[3])
        end)

        it("errors on uninitialized local", function()
            local code = AC({
                I(LOAD_LOCAL, 5),
                I(HALT),
            })
            assert.has_error(function() vm:execute(code) end,
                "InvalidOperandError: LOAD_LOCAL 5 uninitialized")
        end)

        it("errors on index 0 for LOAD_LOCAL", function()
            local code = AC({
                I(LOAD_LOCAL, 0),
                I(HALT),
            })
            assert.has_error(function() vm:execute(code) end,
                "InvalidOperandError: LOAD_LOCAL 0 uninitialized")
        end)
    end)

    -- ----- Arithmetic -----

    describe("ADD", function()
        it("adds two numbers", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(ADD),
                I(HALT),
            }, { 3, 4 })
            vm:execute(code)
            assert.are.equal(7, vm.stack[1])
        end)

        it("concatenates two strings", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(ADD),
                I(HALT),
            }, { "hello", " world" })
            vm:execute(code)
            assert.are.equal("hello world", vm.stack[1])
        end)

        it("errors on mixed types", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(ADD),
                I(HALT),
            }, { 1, "two" })
            assert.has_error(function() vm:execute(code) end)
        end)
    end)

    describe("SUB", function()
        it("subtracts two numbers", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(SUB),
                I(HALT),
            }, { 10, 3 })
            vm:execute(code)
            assert.are.equal(7, vm.stack[1])
        end)
    end)

    describe("MUL", function()
        it("multiplies two numbers", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(MUL),
                I(HALT),
            }, { 6, 7 })
            vm:execute(code)
            assert.are.equal(42, vm.stack[1])
        end)
    end)

    describe("DIV", function()
        it("divides two numbers (integer division)", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(DIV),
                I(HALT),
            }, { 10, 3 })
            vm:execute(code)
            assert.are.equal(3, vm.stack[1])
        end)

        it("errors on division by zero", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(DIV),
                I(HALT),
            }, { 10, 0 })
            assert.has_error(function() vm:execute(code) end,
                "DivisionByZeroError: Division by zero")
        end)
    end)

    -- ----- Comparison -----

    describe("CMP_EQ", function()
        it("pushes 1 when equal", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(CMP_EQ),
                I(HALT),
            }, { 5, 5 })
            vm:execute(code)
            assert.are.equal(1, vm.stack[1])
        end)

        it("pushes 0 when not equal", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(CMP_EQ),
                I(HALT),
            }, { 5, 6 })
            vm:execute(code)
            assert.are.equal(0, vm.stack[1])
        end)
    end)

    describe("CMP_LT", function()
        it("pushes 1 when a < b", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(CMP_LT),
                I(HALT),
            }, { 3, 5 })
            vm:execute(code)
            assert.are.equal(1, vm.stack[1])
        end)

        it("pushes 0 when a >= b", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(CMP_LT),
                I(HALT),
            }, { 5, 3 })
            vm:execute(code)
            assert.are.equal(0, vm.stack[1])
        end)
    end)

    describe("CMP_GT", function()
        it("pushes 1 when a > b", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(CMP_GT),
                I(HALT),
            }, { 5, 3 })
            vm:execute(code)
            assert.are.equal(1, vm.stack[1])
        end)

        it("pushes 0 when a <= b", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(CMP_GT),
                I(HALT),
            }, { 3, 5 })
            vm:execute(code)
            assert.are.equal(0, vm.stack[1])
        end)
    end)

    -- ----- Control flow -----

    describe("JUMP", function()
        it("jumps to target instruction", function()
            -- Jump over the second LOAD_CONST
            local code = AC({
                I(LOAD_CONST, 1),   -- 1
                I(JUMP, 3),         -- 2: jump to 3
                I(LOAD_CONST, 2),   -- 3: this gets skipped... wait
                I(HALT),            -- 4
            }, { 10, 20 })
            -- JUMP to 3 means PC = 3, which is LOAD_CONST 20 (skipped nothing useful)
            -- Let's actually skip instruction 3 by jumping to 4
            code = AC({
                I(LOAD_CONST, 1),   -- 1
                I(JUMP, 4),         -- 2: jump to instruction 4
                I(LOAD_CONST, 2),   -- 3: SKIPPED
                I(HALT),            -- 4
            }, { 10, 20 })
            vm:execute(code)
            assert.are.equal(1, #vm.stack)
            assert.are.equal(10, vm.stack[1])
        end)
    end)

    describe("JUMP_IF_FALSE", function()
        it("jumps when value is falsy (0)", function()
            local code = AC({
                I(LOAD_CONST, 1),        -- 1: push 0
                I(JUMP_IF_FALSE, 4),     -- 2: if falsy, jump to 4
                I(LOAD_CONST, 2),        -- 3: SKIPPED
                I(HALT),                 -- 4
            }, { 0, 99 })
            vm:execute(code)
            assert.are.equal(0, #vm.stack)
        end)

        it("falls through when value is truthy", function()
            local code = AC({
                I(LOAD_CONST, 1),        -- 1: push 1
                I(JUMP_IF_FALSE, 4),     -- 2: if falsy, jump (but 1 is truthy)
                I(LOAD_CONST, 2),        -- 3: NOT skipped
                I(HALT),                 -- 4
            }, { 1, 99 })
            vm:execute(code)
            assert.are.equal(1, #vm.stack)
            assert.are.equal(99, vm.stack[1])
        end)
    end)

    describe("JUMP_IF_TRUE", function()
        it("jumps when value is truthy", function()
            local code = AC({
                I(LOAD_CONST, 1),       -- 1: push 1
                I(JUMP_IF_TRUE, 4),     -- 2: if truthy, jump to 4
                I(LOAD_CONST, 2),       -- 3: SKIPPED
                I(HALT),                -- 4
            }, { 1, 99 })
            vm:execute(code)
            assert.are.equal(0, #vm.stack)
        end)

        it("falls through when value is falsy", function()
            local code = AC({
                I(LOAD_CONST, 1),       -- 1: push 0
                I(JUMP_IF_TRUE, 4),     -- 2: if truthy, jump (but 0 is falsy)
                I(LOAD_CONST, 2),       -- 3: NOT skipped
                I(HALT),                -- 4
            }, { 0, 99 })
            vm:execute(code)
            assert.are.equal(1, #vm.stack)
            assert.are.equal(99, vm.stack[1])
        end)
    end)

    -- ----- Functions -----

    describe("CALL / RETURN", function()
        it("calls a function and returns", function()
            -- Define a function that pushes 42 and returns
            local func_code = AC({
                I(LOAD_CONST, 1),
                I(RETURN),
            }, { 42 })

            -- Main program: store the function, call it
            local main_code = AC({
                I(LOAD_CONST, 1),     -- 1: push func_code
                I(STORE_NAME, 1),     -- 2: store as "my_func"
                I(CALL, 1),           -- 3: call "my_func"
                I(HALT),              -- 4
            }, { func_code }, { "my_func" })

            vm:execute(main_code)
            -- The function pushed 42 onto the stack
            assert.are.equal(42, vm.stack[1])
        end)

        it("errors on calling undefined function", function()
            local code = AC({
                I(CALL, 1),
                I(HALT),
            }, {}, { "nonexistent" })
            assert.has_error(function() vm:execute(code) end,
                "UndefinedNameError: Function 'nonexistent' is not defined")
        end)

        it("errors on calling non-callable object", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(STORE_NAME, 1),
                I(CALL, 1),
                I(HALT),
            }, { 42 }, { "not_a_func" })
            assert.has_error(function() vm:execute(code) end,
                "VMError: Object is not callable")
        end)

        it("RETURN at top level halts the VM", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(RETURN),
                I(LOAD_CONST, 2),  -- should not execute
                I(HALT),
            }, { 10, 20 })
            vm:execute(code)
            assert.is_true(vm.halted)
            assert.are.equal(1, #vm.stack)
            assert.are.equal(10, vm.stack[1])
        end)
    end)

    -- ----- I/O -----

    describe("PRINT", function()
        it("pops value and appends to output", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(PRINT),
                I(HALT),
            }, { 42 })
            vm:execute(code)
            assert.are.equal(0, #vm.stack)
            assert.are.equal("42", vm.output[1])
        end)

        it("prints strings", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(PRINT),
                I(HALT),
            }, { "hello" })
            vm:execute(code)
            assert.are.equal("hello", vm.output[1])
        end)
    end)

    -- ----- HALT -----

    describe("HALT", function()
        it("sets halted flag", function()
            local code = AC({ I(HALT) })
            vm:execute(code)
            assert.is_true(vm.halted)
        end)
    end)

    -- ----- Unknown opcode -----

    describe("unknown opcode", function()
        it("errors on unregistered opcode", function()
            local code = AC({ I(0xEE), I(HALT) })
            assert.has_error(function() vm:execute(code) end,
                "InvalidOpcodeError: Unknown opcode 0xee")
        end)
    end)

    -- ----- Traces -----

    describe("traces", function()
        it("returns one trace per executed instruction", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(PRINT),
                I(HALT),
            }, { 42 })
            local traces = vm:execute(code)
            assert.are.equal(3, #traces)
        end)

        it("traces record pc, stack_before, stack_after, variables", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(STORE_NAME, 1),
                I(HALT),
            }, { 99 }, { "x" })
            local traces = vm:execute(code)

            -- First trace: LOAD_CONST
            assert.are.equal(1, traces[1].pc)
            assert.are.same({}, traces[1].stack_before)
            assert.are.same({99}, traces[1].stack_after)

            -- Second trace: STORE_NAME
            assert.are.equal(2, traces[2].pc)
            assert.are.same({99}, traces[2].stack_before)
            assert.are.same({}, traces[2].stack_after)
            assert.are.equal(99, traces[2].variables["x"])
        end)

        it("trace output is set for PRINT, nil for others", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(PRINT),
                I(HALT),
            }, { 42 })
            local traces = vm:execute(code)
            assert.is_nil(traces[1].output)
            assert.are.equal("42", traces[2].output)
            assert.is_nil(traces[3].output)
        end)

        it("trace description is set", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(HALT),
            }, { 42 })
            local traces = vm:execute(code)
            assert.is_not_nil(traces[1].description)
            assert.is_not_nil(traces[2].description)
        end)
    end)

    -- ----- Coroutine stepper -----

    describe("create_stepper", function()
        it("yields one trace per step", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(ADD),
                I(HALT),
            }, { 3, 4 })
            local stepper = vm:create_stepper(code)
            local traces = {}
            while true do
                local ok, trace = coroutine.resume(stepper)
                if not ok or trace == nil then break end
                traces[#traces + 1] = trace
            end
            assert.are.equal(4, #traces)
            assert.are.equal(1, traces[1].pc) -- LOAD_CONST
            assert.are.equal(4, traces[4].pc) -- HALT
        end)

        it("can pause and resume mid-program", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(ADD),
                I(HALT),
            }, { 10, 20 })
            local stepper = vm:create_stepper(code)

            -- Step once
            local ok, trace = coroutine.resume(stepper)
            assert.is_true(ok)
            assert.are.equal(1, trace.pc)
            assert.are.same({10}, vm.stack)

            -- Step again
            ok, trace = coroutine.resume(stepper)
            assert.is_true(ok)
            assert.are.equal(2, trace.pc)
            assert.are.same({10, 20}, vm.stack)

            -- Step: ADD
            ok, trace = coroutine.resume(stepper)
            assert.is_true(ok)
            assert.are.equal(3, trace.pc)
            assert.are.same({30}, vm.stack)
        end)
    end)

    -- ----- Integration: full programs -----

    describe("integration", function()
        it("computes factorial of 5 using a loop", function()
            -- Pseudo-code:
            --   result = 1; n = 5
            --   while n > 1: result *= n; n -= 1
            --   print result
            local code = AC({
                I(LOAD_CONST, 1),       -- 1: push 1
                I(STORE_NAME, 1),       -- 2: result = 1
                I(LOAD_CONST, 2),       -- 3: push 5
                I(STORE_NAME, 2),       -- 4: n = 5
                I(LOAD_NAME, 2),        -- 5: push n       (loop top)
                I(LOAD_CONST, 1),       -- 6: push 1
                I(CMP_GT),              -- 7: n > 1?
                I(JUMP_IF_FALSE, 18),   -- 8: exit loop -> 18
                I(LOAD_NAME, 1),        -- 9: push result
                I(LOAD_NAME, 2),        -- 10: push n
                I(MUL),                 -- 11: result * n
                I(STORE_NAME, 1),       -- 12: result = ...
                I(LOAD_NAME, 2),        -- 13: push n
                I(LOAD_CONST, 1),       -- 14: push 1
                I(SUB),                 -- 15: n - 1
                I(STORE_NAME, 2),       -- 16: n = ...
                I(JUMP, 5),             -- 17: back to loop
                I(LOAD_NAME, 1),        -- 18: push result
                I(PRINT),              -- 19
                I(HALT),                -- 20
            }, { 1, 5 }, { "result", "n" })

            vm:execute(code)
            assert.are.equal("120", vm.output[1])
        end)

        it("runs program that terminates by falling off the end", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(PRINT),
            }, { "done" })
            vm:execute(code)
            assert.are.equal("done", vm.output[1])
            assert.is_false(vm.halted)  -- halted flag not set, just ran out of instructions
        end)

        it("handles multiple prints", function()
            local code = AC({
                I(LOAD_CONST, 1),
                I(PRINT),
                I(LOAD_CONST, 2),
                I(PRINT),
                I(HALT),
            }, { "hello", "world" })
            vm:execute(code)
            assert.are.equal(2, #vm.output)
            assert.are.equal("hello", vm.output[1])
            assert.are.equal("world", vm.output[2])
        end)
    end)
end)


-- =========================================================================
-- GENERIC VM
-- =========================================================================

describe("GenericVM", function()
    local gvm

    before_each(function()
        gvm = GVM.new()
    end)

    -- ----- Constructor -----

    describe("constructor", function()
        it("initializes with empty state", function()
            assert.are.same({}, gvm.stack)
            assert.are.same({}, gvm.variables)
            assert.are.same({}, gvm.locals)
            assert.are.equal(1, gvm.pc)
            assert.is_false(gvm.halted)
            assert.are.same({}, gvm.output)
            assert.are.same({}, gvm.call_stack)
        end)
    end)

    -- ----- Stack operations -----

    describe("push / pop / peek", function()
        it("push and pop work correctly", function()
            gvm:push(10)
            gvm:push(20)
            assert.are.equal(20, gvm:pop())
            assert.are.equal(10, gvm:pop())
        end)

        it("pop errors on empty stack", function()
            assert.has_error(function() gvm:pop() end, "StackUnderflowError")
        end)

        it("peek returns top without removing", function()
            gvm:push(42)
            assert.are.equal(42, gvm:peek())
            assert.are.equal(42, gvm:peek()) -- still there
            assert.are.equal(1, #gvm.stack)
        end)

        it("peek errors on empty stack", function()
            assert.has_error(function() gvm:peek() end, "StackUnderflowError")
        end)
    end)

    -- ----- Call stack operations -----

    describe("push_frame / pop_frame", function()
        it("pushes and pops frames", function()
            local frame = { return_addr = 5 }
            gvm:push_frame(frame)
            assert.are.equal(1, #gvm.call_stack)
            local popped = gvm:pop_frame()
            assert.are.equal(5, popped.return_addr)
            assert.are.equal(0, #gvm.call_stack)
        end)

        it("pop_frame errors on empty call stack", function()
            assert.has_error(function() gvm:pop_frame() end, "CallStackUnderflowError")
        end)

        it("respects max recursion depth", function()
            gvm:set_max_recursion_depth(2)
            gvm:push_frame({ a = 1 })
            gvm:push_frame({ a = 2 })
            assert.has_error(function() gvm:push_frame({ a = 3 }) end, "MaxRecursionError")
        end)

        it("max depth 0 disallows all calls", function()
            gvm:set_max_recursion_depth(0)
            assert.has_error(function() gvm:push_frame({}) end, "MaxRecursionError")
        end)

        it("nil max depth allows unlimited calls", function()
            gvm:set_max_recursion_depth(nil)
            for i = 1, 50 do
                gvm:push_frame({ i = i })
            end
            assert.are.equal(50, #gvm.call_stack)
        end)
    end)

    -- ----- PC control -----

    describe("advance_pc / jump_to", function()
        it("advance_pc increments by 1", function()
            assert.are.equal(1, gvm.pc)
            gvm:advance_pc()
            assert.are.equal(2, gvm.pc)
        end)

        it("jump_to sets PC to target", function()
            gvm:jump_to(10)
            assert.are.equal(10, gvm.pc)
        end)
    end)

    -- ----- Configuration -----

    describe("configuration", function()
        it("get/set max recursion depth", function()
            assert.is_nil(gvm:get_max_recursion_depth())
            gvm:set_max_recursion_depth(100)
            assert.are.equal(100, gvm:get_max_recursion_depth())
            gvm:set_max_recursion_depth(nil)
            assert.is_nil(gvm:get_max_recursion_depth())
        end)

        it("freeze prevents registration", function()
            gvm:set_frozen(true)
            assert.is_true(gvm:is_frozen())
            assert.has_error(function()
                gvm:register_opcode(0x01, function() end)
            end, "FrozenVMError: cannot register opcodes on a frozen VM")
            assert.has_error(function()
                gvm:register_builtin("len", function() end)
            end, "FrozenVMError: cannot register builtins on a frozen VM")
        end)

        it("unfreeze re-enables registration", function()
            gvm:set_frozen(true)
            gvm:set_frozen(false)
            assert.is_false(gvm:is_frozen())
            -- Should not error
            gvm:register_opcode(0x01, function() end)
        end)
    end)

    -- ----- Builtin registration -----

    describe("builtins", function()
        it("registers and retrieves a builtin", function()
            gvm:register_builtin("len", function(...)
                local args = {...}
                return #args[1]
            end)
            local b = gvm:get_builtin("len")
            assert.is_not_nil(b)
            assert.are.equal("len", b.name)
            assert.are.equal(5, b.implementation("hello"))
        end)

        it("returns nil for unregistered builtin", function()
            assert.is_nil(gvm:get_builtin("nonexistent"))
        end)
    end)

    -- ----- Handler registration and execution -----

    describe("register_opcode and execution", function()
        it("dispatches to registered handler", function()
            gvm:register_opcode(LOAD_CONST, function(vm, instr, code)
                local val = code.constants[instr.operand]
                vm:push(val)
                vm:advance_pc()
                return nil
            end)
            gvm:register_opcode(HALT, function(vm, instr, code)
                vm.halted = true
                return nil
            end)

            local code = AC({ I(LOAD_CONST, 1), I(HALT) }, { 42 })
            gvm:execute(code)
            assert.are.equal(42, gvm.stack[1])
            assert.is_true(gvm.halted)
        end)

        it("errors on unregistered opcode", function()
            local code = AC({ I(0xAB) })
            assert.has_error(function() gvm:execute(code) end,
                "InvalidOpcodeError: no handler registered for opcode 0xab")
        end)

        it("handler can produce output", function()
            gvm:register_opcode(PRINT, function(vm, instr, code)
                local val = vm:pop()
                vm:advance_pc()
                return tostring(val)
            end)
            gvm:register_opcode(LOAD_CONST, function(vm, instr, code)
                vm:push(code.constants[instr.operand])
                vm:advance_pc()
                return nil
            end)
            gvm:register_opcode(HALT, function(vm, instr, code)
                vm.halted = true
                return nil
            end)

            local code = AC({ I(LOAD_CONST, 1), I(PRINT), I(HALT) }, { 99 })
            local traces = gvm:execute(code)
            assert.are.equal("99", gvm.output[1])
            assert.are.equal("99", traces[2].output)
        end)
    end)

    -- ----- Traces -----

    describe("traces", function()
        it("returns traces with correct format", function()
            gvm:register_opcode(LOAD_CONST, function(vm, instr, code)
                vm:push(code.constants[instr.operand])
                vm:advance_pc()
                return nil
            end)
            gvm:register_opcode(HALT, function(vm, instr, code)
                vm.halted = true
                return nil
            end)

            local code = AC({ I(LOAD_CONST, 1), I(HALT) }, { 77 })
            local traces = gvm:execute(code)
            assert.are.equal(2, #traces)

            -- First trace
            assert.are.equal(1, traces[1].pc)
            assert.are.same({}, traces[1].stack_before)
            assert.are.same({77}, traces[1].stack_after)
            assert.are.equal("Executed opcode 0x01", traces[1].description)
            assert.is_nil(traces[1].output)

            -- Second trace
            assert.are.equal(2, traces[2].pc)
            assert.are.equal("Executed opcode 0xff", traces[2].description)
        end)
    end)

    -- ----- Reset -----

    describe("reset", function()
        it("clears runtime state but preserves handlers and config", function()
            gvm:register_opcode(LOAD_CONST, function(vm, instr, code)
                vm:push(code.constants[instr.operand])
                vm:advance_pc()
                return nil
            end)
            gvm:register_opcode(HALT, function(vm, instr, code)
                vm.halted = true
                return nil
            end)
            gvm:set_max_recursion_depth(50)
            gvm:set_frozen(true)

            local code = AC({ I(LOAD_CONST, 1), I(HALT) }, { 42 })
            -- Unfreeze temporarily to run
            gvm:set_frozen(false)
            gvm:execute(code)
            assert.are.equal(42, gvm.stack[1])
            assert.is_true(gvm.halted)

            gvm:reset()

            -- Runtime state cleared
            assert.are.same({}, gvm.stack)
            assert.are.same({}, gvm.variables)
            assert.are.same({}, gvm.locals)
            assert.are.equal(1, gvm.pc)
            assert.is_false(gvm.halted)
            assert.are.same({}, gvm.output)
            assert.are.same({}, gvm.call_stack)

            -- Config preserved
            assert.are.equal(50, gvm:get_max_recursion_depth())

            -- Handlers preserved: can execute again
            gvm:execute(code)
            assert.are.equal(42, gvm.stack[1])
        end)
    end)

    -- ----- Coroutine stepper -----

    describe("create_stepper", function()
        it("yields one trace per step", function()
            gvm:register_opcode(LOAD_CONST, function(vm, instr, code)
                vm:push(code.constants[instr.operand])
                vm:advance_pc()
                return nil
            end)
            gvm:register_opcode(ADD, function(vm, instr, code)
                local b = vm:pop()
                local a = vm:pop()
                vm:push(a + b)
                vm:advance_pc()
                return nil
            end)
            gvm:register_opcode(HALT, function(vm, instr, code)
                vm.halted = true
                return nil
            end)

            local code = AC({
                I(LOAD_CONST, 1),
                I(LOAD_CONST, 2),
                I(ADD),
                I(HALT),
            }, { 10, 20 })

            local stepper = gvm:create_stepper(code)
            local traces = {}
            while true do
                local ok, trace = coroutine.resume(stepper)
                if not ok or trace == nil then break end
                traces[#traces + 1] = trace
            end
            assert.are.equal(4, #traces)
            assert.are.equal(30, gvm.stack[1])
        end)
    end)

    -- ----- Integration: full program with GenericVM -----

    describe("integration", function()
        -- Register a standard set of handlers for a complete test
        local function register_standard_handlers(vm)
            vm:register_opcode(LOAD_CONST, function(v, instr, code)
                v:push(code.constants[instr.operand])
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(POP, function(v, instr, code)
                v:pop()
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(DUP, function(v, instr, code)
                v:push(v:peek())
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(STORE_NAME, function(v, instr, code)
                local name = code.names[instr.operand]
                v.variables[name] = v:pop()
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(LOAD_NAME, function(v, instr, code)
                local name = code.names[instr.operand]
                v:push(v.variables[name])
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(ADD, function(v, instr, code)
                local b = v:pop()
                local a = v:pop()
                v:push(a + b)
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(SUB, function(v, instr, code)
                local b = v:pop()
                local a = v:pop()
                v:push(a - b)
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(MUL, function(v, instr, code)
                local b = v:pop()
                local a = v:pop()
                v:push(a * b)
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(CMP_GT, function(v, instr, code)
                local b = v:pop()
                local a = v:pop()
                v:push((a > b) and 1 or 0)
                v:advance_pc()
                return nil
            end)
            vm:register_opcode(JUMP, function(v, instr, code)
                v:jump_to(instr.operand)
                return nil
            end)
            vm:register_opcode(JUMP_IF_FALSE, function(v, instr, code)
                local val = v:pop()
                if val == 0 or val == nil or val == "" then
                    v:jump_to(instr.operand)
                else
                    v:advance_pc()
                end
                return nil
            end)
            vm:register_opcode(PRINT, function(v, instr, code)
                local val = v:pop()
                v:advance_pc()
                return tostring(val)
            end)
            vm:register_opcode(HALT, function(v, instr, code)
                v.halted = true
                return nil
            end)
        end

        it("computes sum 1+2+3+4+5 with a loop", function()
            register_standard_handlers(gvm)

            -- sum = 0; i = 1
            -- while 6 > i (i.e. i <= 5): sum += i; i += 1
            -- print sum
            --
            -- We use constant 6 instead of 5 because CMP_GT is strict:
            --   6 > 5 = true  (include i=5)
            --   6 > 6 = false (exit when i=6)
            local code = AC({
                I(LOAD_CONST, 1),       -- 1
                I(STORE_NAME, 1),       -- 2: sum = 0
                I(LOAD_CONST, 2),       -- 3
                I(STORE_NAME, 2),       -- 4: i = 1
                I(LOAD_CONST, 3),       -- 5: push 6
                I(LOAD_NAME, 2),        -- 6: push i
                I(CMP_GT),              -- 7: 6 > i?
                I(JUMP_IF_FALSE, 18),   -- 8: exit
                I(LOAD_NAME, 1),        -- 9
                I(LOAD_NAME, 2),        -- 10
                I(ADD),                 -- 11
                I(STORE_NAME, 1),       -- 12: sum += i
                I(LOAD_NAME, 2),        -- 13
                I(LOAD_CONST, 2),       -- 14
                I(ADD),                 -- 15
                I(STORE_NAME, 2),       -- 16: i += 1
                I(JUMP, 5),             -- 17: loop
                I(LOAD_NAME, 1),        -- 18
                I(PRINT),              -- 19
                I(HALT),                -- 20
            }, { 0, 1, 6 }, { "sum", "i" })

            gvm:execute(code)
            assert.are.equal("15", gvm.output[1])
        end)

        it("can reset and rerun", function()
            register_standard_handlers(gvm)
            local code = AC({
                I(LOAD_CONST, 1), I(PRINT), I(HALT),
            }, { "first" })
            gvm:execute(code)
            assert.are.equal("first", gvm.output[1])

            gvm:reset()
            code = AC({
                I(LOAD_CONST, 1), I(PRINT), I(HALT),
            }, { "second" })
            gvm:execute(code)
            assert.are.equal("second", gvm.output[1])
            assert.are.equal(1, #gvm.output)
        end)
    end)
end)


-- =========================================================================
-- EDGE CASES AND ERROR PATHS
-- =========================================================================

describe("edge cases", function()
    it("VirtualMachine step can be called directly", function()
        local vm = VM.new()
        local code = AC({ I(LOAD_CONST, 1), I(HALT) }, { 10 })
        local trace = vm:step(code)
        assert.are.equal(1, trace.pc)
        assert.are.same({10}, vm.stack)
    end)

    it("GenericVM step can be called directly", function()
        local gvm = GVM.new()
        gvm:register_opcode(LOAD_CONST, function(v, instr, code)
            v:push(code.constants[instr.operand])
            v:advance_pc()
            return nil
        end)
        local code = AC({ I(LOAD_CONST, 1) }, { 10 })
        local trace = gvm:step(code)
        assert.are.equal(1, trace.pc)
        assert.are.same({10}, gvm.stack)
    end)

    it("overwriting a handler in GenericVM replaces the old one", function()
        local gvm = GVM.new()
        gvm:register_opcode(HALT, function(vm) vm.halted = true; return nil end)
        gvm:register_opcode(HALT, function(vm)
            vm:push(999)
            vm.halted = true
            return nil
        end)
        local code = AC({ I(HALT) })
        gvm:execute(code)
        assert.are.equal(999, gvm.stack[1])
    end)

    it("GenericVM with builtin used from handler", function()
        local gvm = GVM.new()
        gvm:register_builtin("double", function(...)
            local args = {...}
            return args[1] * 2
        end)
        gvm:register_opcode(LOAD_CONST, function(vm, instr, code)
            local val = code.constants[instr.operand]
            local dbl = vm:get_builtin("double")
            vm:push(dbl.implementation(val))
            vm:advance_pc()
            return nil
        end)
        gvm:register_opcode(HALT, function(vm) vm.halted = true; return nil end)

        local code = AC({ I(LOAD_CONST, 1), I(HALT) }, { 21 })
        gvm:execute(code)
        assert.are.equal(42, gvm.stack[1])
    end)

    it("VirtualMachine with function that modifies variables", function()
        local vm = VM.new()
        local func_code = AC({
            I(LOAD_CONST, 1),  -- push 100
            I(STORE_NAME, 1),  -- store as "result" (but func uses main code's names)
            I(RETURN),
        }, { 100 }, { "result" })

        -- Store function and call it -- but note: CALL uses the main code's names
        -- to look up the function name, and the function body uses its own CodeObject
        local main_code = AC({
            I(LOAD_CONST, 1),   -- push func_code
            I(STORE_NAME, 1),   -- store as "my_func"
            I(CALL, 1),         -- call "my_func"
            I(LOAD_NAME, 2),    -- push "result" -- but this is index 2
            I(PRINT),
            I(HALT),
        }, { func_code }, { "my_func", "result" })

        vm:execute(main_code)
        -- The function stores 100 into "result" in the VM's variables
        assert.are.equal("100", vm.output[1])
    end)
end)

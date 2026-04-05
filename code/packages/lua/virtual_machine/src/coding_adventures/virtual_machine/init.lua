-- virtual_machine -- Stack-based bytecode interpreter with eval loop, value
-- stack, and variable environment.
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 5 in the computing stack.
--
-- =========================================================================
-- WHAT IS A VIRTUAL MACHINE?
-- =========================================================================
--
-- A virtual machine (VM) is a software CPU.  Real CPUs execute machine code
-- (binary instructions like ADD, LOAD, JUMP).  A VM does the same thing, but
-- the "machine code" is a sequence of Lua tables instead of raw bytes, and
-- the "CPU registers" are Lua variables.
--
-- Why bother?  Because a VM lets you run programs written in *any* language
-- -- Python, Ruby, your own invention -- as long as you have a compiler that
-- translates source code into the VM's instruction set (bytecode).
--
-- This module provides TWO virtual machines:
--
--   1. VirtualMachine  -- every opcode is hard-coded in a big dispatch table.
--                         Simple, fast, and easy to follow.
--
--   2. GenericVM        -- opcodes are registered as callback functions.
--                         Language-agnostic: plug in different opcode sets
--                         to emulate Python bytecode, Ruby bytecode, or
--                         whatever you dream up next weekend.
--
-- Both VMs share the same Instruction, CodeObject, and VMTrace types.
--
-- =========================================================================
-- ARCHITECTURE OVERVIEW
-- =========================================================================
--
--   +---------------------------------------------------+
--   |                   VirtualMachine                   |
--   |                                                    |
--   |   +---------+   +-----------+   +--------+        |
--   |   |  Stack  |   | Variables |   | Locals |        |
--   |   | (LIFO)  |   |  (table)  |   | (array)|        |
--   |   +---------+   +-----------+   +--------+        |
--   |                                                    |
--   |   PC --> fetch instruction --> dispatch by opcode  |
--   |          --> modify state --> record trace          |
--   +---------------------------------------------------+
--
--   +---------------------------------------------------+
--   |                    GenericVM                        |
--   |                                                    |
--   |   +---------+   +-----------+   +--------+        |
--   |   |  Stack  |   | Variables |   | Locals |        |
--   |   | (LIFO)  |   |  (table)  |   | (array)|        |
--   |   +---------+   +-----------+   +--------+        |
--   |                                                    |
--   |   +-------------------------------------------+   |
--   |   |  Handler Table                            |   |
--   |   |  0x01 -> LoadConst handler                |   |
--   |   |  0x20 -> Add handler                      |   |
--   |   |  0xFF -> Halt handler                     |   |
--   |   +-------------------------------------------+   |
--   |                                                    |
--   |   +-------------------------------------------+   |
--   |   |  Builtins Table                           |   |
--   |   |  "len"  -> length function                |   |
--   |   |  "abs"  -> absolute value function        |   |
--   |   +-------------------------------------------+   |
--   |                                                    |
--   |   PC --> fetch --> lookup handler --> call handler  |
--   |          --> record trace                          |
--   +---------------------------------------------------+
--
-- =========================================================================
-- THE EXECUTION CYCLE (Fetch-Decode-Execute)
-- =========================================================================
--
-- Every real CPU and every virtual machine follows the same basic loop:
--
--   1. FETCH:   Read the instruction at the current Program Counter (PC).
--   2. DECODE:  Figure out what the instruction means (look up handler or
--               dispatch table entry).
--   3. EXECUTE: Do the work (modify VM state: push, pop, jump, etc.).
--   4. REPEAT:  Go back to step 1 unless halted.
--
-- Both VirtualMachine.execute() and GenericVM.execute() implement exactly
-- this loop.  The step() method performs one iteration and returns a VMTrace
-- so you can inspect what happened at each instruction.
--
-- =========================================================================
-- COROUTINE-BASED STEP-THROUGH DEBUGGING
-- =========================================================================
--
-- Lua coroutines provide a natural mechanism for pause-and-resume execution.
-- Both VMs offer a create_stepper() method that returns a coroutine-based
-- iterator.  Each call to coroutine.resume() executes one instruction and
-- yields the VMTrace.  This is ideal for debuggers, visualizers, and
-- educational tools that want to show execution one step at a time.
--
--   local stepper = vm:create_stepper(code)
--   while true do
--       local ok, trace = coroutine.resume(stepper)
--       if not ok or trace == nil then break end
--       print(trace.description)
--   end
--

local M = {}

M.VERSION = "0.1.0"


-- =========================================================================
-- OPCODE CONSTANTS
-- =========================================================================
--
-- Each opcode is a numeric constant that tells the VM what operation to
-- perform.  We use the same hex values as the Go implementation so that
-- bytecode is interchangeable.
--
-- Opcodes are grouped by category:
--
--   0x01-0x03  Stack manipulation (load, pop, dup)
--   0x10-0x13  Variable access (store/load global and local)
--   0x20-0x23  Arithmetic (add, sub, mul, div)
--   0x30-0x32  Comparison (eq, lt, gt)
--   0x40-0x42  Control flow (jump, conditional jumps)
--   0x50-0x51  Functions (call, return)
--   0x60       I/O (print)
--   0xFF       Halt
--

M.OP_LOAD_CONST   = 0x01
M.OP_POP          = 0x02
M.OP_DUP          = 0x03
M.OP_STORE_NAME   = 0x10
M.OP_LOAD_NAME    = 0x11
M.OP_STORE_LOCAL  = 0x12
M.OP_LOAD_LOCAL   = 0x13
M.OP_ADD          = 0x20
M.OP_SUB          = 0x21
M.OP_MUL          = 0x22
M.OP_DIV          = 0x23
M.OP_CMP_EQ       = 0x30
M.OP_CMP_LT       = 0x31
M.OP_CMP_GT       = 0x32
M.OP_JUMP         = 0x40
M.OP_JUMP_IF_FALSE = 0x41
M.OP_JUMP_IF_TRUE  = 0x42
M.OP_CALL         = 0x50
M.OP_RETURN       = 0x51
M.OP_PRINT        = 0x60
M.OP_HALT         = 0xFF


-- =========================================================================
-- INSTRUCTION
-- =========================================================================
--
-- An Instruction is the smallest unit of work in a bytecode program.
-- It has an opcode (what to do) and an optional operand (what to do it to).
--
-- Examples:
--   { opcode = 0x01, operand = 0 }   -- LOAD_CONST: push constants[0]
--   { opcode = 0x20 }                -- ADD: pop two, push sum
--   { opcode = 0x40, operand = 5 }   -- JUMP: set PC to 5
--

--- Create a new Instruction table.
-- @param opcode  number  The opcode constant (e.g. M.OP_ADD).
-- @param operand any     Optional operand (index, jump target, etc.).
-- @return table  The instruction.
function M.instruction(opcode, operand)
    return { opcode = opcode, operand = operand }
end

--- Format an Instruction as a human-readable string.
-- @param instr table  The instruction to format.
-- @return string
function M.instruction_tostring(instr)
    if instr.operand ~= nil then
        return string.format("Instruction(0x%02x, %s)", instr.opcode, tostring(instr.operand))
    end
    return string.format("Instruction(0x%02x)", instr.opcode)
end


-- =========================================================================
-- CODE OBJECT
-- =========================================================================
--
-- A CodeObject bundles everything needed to run a bytecode program:
--
--   instructions  -- the sequence of Instruction tables (1-indexed in Lua)
--   constants     -- the constant pool (numbers, strings, etc.)
--   names         -- the name pool (variable/function names)
--
-- Think of it like an executable file: instructions are the code segment,
-- constants are the data segment, and names are the symbol table.
--

--- Assemble a CodeObject from instructions, constants, and names.
--
-- This is the primary way to create a CodeObject.  It fills in
-- sensible defaults for constants and names if omitted.
--
-- @param instructions table  Array of Instruction tables.
-- @param constants    table  (optional) Array of constant values.
-- @param names        table  (optional) Array of name strings.
-- @return table  The CodeObject.
function M.assemble_code(instructions, constants, names)
    return {
        instructions = instructions or {},
        constants = constants or {},
        names = names or {},
    }
end


-- =========================================================================
-- VM TRACE
-- =========================================================================
--
-- A VMTrace is a snapshot of one instruction's execution.  It records:
--
--   pc            -- the program counter BEFORE the instruction ran
--   instruction   -- the instruction that was executed
--   stack_before  -- copy of the stack BEFORE execution
--   stack_after   -- copy of the stack AFTER execution
--   variables     -- copy of the variable environment AFTER execution
--   output        -- string if the instruction produced output, nil otherwise
--   description   -- human-readable explanation of what happened
--
-- Traces are invaluable for debugging and education.  You can replay
-- a program's execution step by step, inspecting the stack and variables
-- at every point.
--

--- Create a VMTrace table.
-- @return table
local function make_trace(pc, instruction, stack_before, stack_after, variables, output, description)
    return {
        pc = pc,
        instruction = instruction,
        stack_before = stack_before,
        stack_after = stack_after,
        variables = variables,
        output = output,
        description = description,
    }
end


-- =========================================================================
-- HELPER FUNCTIONS
-- =========================================================================
--
-- Small utilities used by both VirtualMachine and GenericVM.

--- Shallow-copy an array (integer-indexed table).
-- @param arr table  The array to copy.
-- @return table  A new table with the same elements.
local function copy_array(arr)
    local c = {}
    for i = 1, #arr do
        c[i] = arr[i]
    end
    return c
end

--- Shallow-copy a dictionary (string-keyed table).
-- @param t table  The table to copy.
-- @return table  A new table with the same key-value pairs.
local function copy_map(t)
    local c = {}
    for k, v in pairs(t) do
        c[k] = v
    end
    return c
end

--- Test whether a value is "falsy" in VM semantics.
--
-- Falsiness rules (matching the Go implementation):
--   nil        -> true  (absence of a value)
--   0          -> true  (C-style: zero is false)
--   ""         -> true  (empty string is false)
--   everything -> false (any other value is truthy)
--
-- Note: this differs from Lua's own truthiness rules, where only nil and
-- false are falsy.  Our VM follows C/Python conventions where 0 is also
-- falsy.
--
-- Truth table:
--   Value       | is_falsy?
--   ------------|----------
--   nil         | true
--   0           | true
--   ""          | true
--   1           | false
--   -1          | false
--   "hello"     | false
--   {}          | false
--
-- @param val any  The value to test.
-- @return boolean  True if the value is falsy.
local function is_falsy(val)
    if val == nil then return true end
    if val == 0 then return true end
    if val == "" then return true end
    return false
end

--- Pop the top value from a stack array.
-- Panics with "StackUnderflowError" if the stack is empty.
-- @param stack table  The stack array.
-- @return any  The popped value.
-- @return table  The stack (same table, mutated).
local function stack_pop(stack)
    if #stack == 0 then
        error("StackUnderflowError")
    end
    local val = stack[#stack]
    stack[#stack] = nil
    return val
end

--- Add two values with type-aware dispatch.
--
-- Supports:
--   number + number -> arithmetic addition
--   string + string -> concatenation
--   anything else   -> TypeError
--
-- @param a any  Left operand.
-- @param b any  Right operand.
-- @return any  The result.
local function vm_add(a, b)
    if type(a) == "number" and type(b) == "number" then
        return a + b
    end
    if type(a) == "string" and type(b) == "string" then
        return a .. b
    end
    error("TypeError: Cannot add " .. type(a) .. " and " .. type(b))
end


-- =========================================================================
-- VIRTUAL MACHINE (Hard-Coded Opcodes)
-- =========================================================================
--
-- The VirtualMachine dispatches opcodes through a Lua table lookup (the
-- dispatch table).  Each opcode maps to a handler function.  This is
-- equivalent to the Go switch statement but idiomatic in Lua.
--
-- Usage:
--
--   local vm = M.VirtualMachine.new()
--   local code = M.assemble_code(
--       { M.instruction(M.OP_LOAD_CONST, 1),
--         M.instruction(M.OP_PRINT),
--         M.instruction(M.OP_HALT) },
--       { 42 }
--   )
--   local traces = vm:execute(code)
--   print(vm.output[1])  --> "42"
--

local VirtualMachine = {}
VirtualMachine.__index = VirtualMachine

--- Create a new VirtualMachine with empty state.
-- @return VirtualMachine
function VirtualMachine.new()
    local self = setmetatable({}, VirtualMachine)
    self.stack = {}         -- operand stack (LIFO)
    self.variables = {}     -- named variables (global scope)
    self.locals = {}        -- slot-indexed local variables
    self.pc = 1             -- program counter (1-indexed for Lua)
    self.halted = false     -- when true, execution loop stops
    self.output = {}        -- accumulated print output
    self.call_stack = {}    -- saved CallFrame entries for call/return
    return self
end

--- Pop the top value from the operand stack.
-- @return any  The popped value.
function VirtualMachine:pop()
    return stack_pop(self.stack)
end

-- =========================================================================
-- DISPATCH TABLE
-- =========================================================================
--
-- Instead of a giant if/elseif chain, we use a table mapping each opcode
-- to a handler function.  The handler receives (vm, instr, code) and
-- returns (output_string_or_nil, description_string).
--
-- This is idiomatic Lua and avoids deep nesting.  Each handler is a small
-- self-contained function that modifies the VM state.
--

local dispatch = {}

-- LOAD_CONST: Push a constant from the constant pool onto the stack.
--
-- The operand is a 1-based index into code.constants.
-- If the index is out of bounds, we panic with InvalidOperandError.
dispatch[M.OP_LOAD_CONST] = function(vm, instr, code)
    local idx = instr.operand
    if idx < 1 or idx > #code.constants then
        error(string.format("InvalidOperandError: LOAD_CONST %d out of bounds", idx))
    end
    local val = code.constants[idx]
    vm.stack[#vm.stack + 1] = val
    vm.pc = vm.pc + 1
    return nil, string.format("Push constant %s onto the stack", tostring(val))
end

-- POP: Discard the top value from the stack.
dispatch[M.OP_POP] = function(vm, instr, code)
    vm:pop()
    vm.pc = vm.pc + 1
    return nil, "Discard top of stack"
end

-- DUP: Duplicate the top value on the stack.
--
-- Before: [a, b, c]
-- After:  [a, b, c, c]
dispatch[M.OP_DUP] = function(vm, instr, code)
    if #vm.stack == 0 then
        error("StackUnderflowError: DUP empty stack")
    end
    vm.stack[#vm.stack + 1] = vm.stack[#vm.stack]
    vm.pc = vm.pc + 1
    return nil, "Duplicate top of stack"
end

-- STORE_NAME: Pop a value and store it in a named variable.
--
-- The operand is a 1-based index into code.names.
dispatch[M.OP_STORE_NAME] = function(vm, instr, code)
    local idx = instr.operand
    local name = code.names[idx]
    local val = vm:pop()
    vm.variables[name] = val
    vm.pc = vm.pc + 1
    return nil, string.format("Store %s into variable '%s'", tostring(val), name)
end

-- LOAD_NAME: Push a named variable's value onto the stack.
--
-- If the variable is not defined, panic with UndefinedNameError.
dispatch[M.OP_LOAD_NAME] = function(vm, instr, code)
    local idx = instr.operand
    local name = code.names[idx]
    local val = vm.variables[name]
    if val == nil and not (vm.variables[name] ~= nil) then
        -- We need to distinguish "stored nil" from "never stored".
        -- Since Lua tables don't store nil, an undefined name simply
        -- doesn't exist as a key.
        local found = false
        for k, _ in pairs(vm.variables) do
            if k == name then found = true; break end
        end
        if not found then
            error(string.format("UndefinedNameError: Variable '%s' is not defined", name))
        end
    end
    vm.stack[#vm.stack + 1] = val
    vm.pc = vm.pc + 1
    return nil, string.format("Push variable '%s' onto the stack", name)
end

-- STORE_LOCAL: Pop a value and store it in a local slot.
--
-- The operand is a 1-based slot index.  If the locals array is too
-- small, we extend it.
dispatch[M.OP_STORE_LOCAL] = function(vm, instr, code)
    local idx = instr.operand
    local val = vm:pop()
    -- Extend locals array if needed.
    -- Note: we use false as a placeholder rather than nil, because Lua
    -- tables do not store nil values and #t would not reflect the new length.
    -- We then immediately overwrite the target slot with the real value.
    while #vm.locals < idx do
        vm.locals[#vm.locals + 1] = false
    end
    vm.locals[idx] = val
    vm.pc = vm.pc + 1
    return nil, string.format("Store %s into local slot %d", tostring(val), idx)
end

-- LOAD_LOCAL: Push a local slot's value onto the stack.
--
-- Panics if the slot index is out of bounds.
dispatch[M.OP_LOAD_LOCAL] = function(vm, instr, code)
    local idx = instr.operand
    if idx < 1 or idx > #vm.locals then
        error(string.format("InvalidOperandError: LOAD_LOCAL %d uninitialized", idx))
    end
    vm.stack[#vm.stack + 1] = vm.locals[idx]
    vm.pc = vm.pc + 1
    return nil, string.format("Push local slot %d onto the stack", idx)
end

-- ADD: Pop two values, push their sum.
--
-- Supports number+number and string+string (concatenation).
-- Stack effect: [... a b] -> [... (a+b)]
-- Note: b is on top, a is below.  We compute a + b.
dispatch[M.OP_ADD] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    local res = vm_add(a, b)
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Pop %s and %s, push sum %s", tostring(b), tostring(a), tostring(res))
end

-- SUB: Pop two numbers, push their difference (a - b).
dispatch[M.OP_SUB] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    local res = a - b
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Pop %s and %s, push difference %s", tostring(b), tostring(a), tostring(res))
end

-- MUL: Pop two numbers, push their product (a * b).
dispatch[M.OP_MUL] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    local res = a * b
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Pop %s and %s, push product %s", tostring(b), tostring(a), tostring(res))
end

-- DIV: Pop two numbers, push their integer quotient (a / b).
--
-- Division by zero panics with DivisionByZeroError.
-- We use Lua's integer division operator (//) for integer semantics.
dispatch[M.OP_DIV] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    if b == 0 then
        error("DivisionByZeroError: Division by zero")
    end
    local res = a // b
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Pop %s and %s, push quotient %s", tostring(b), tostring(a), tostring(res))
end

-- CMP_EQ: Pop two values, push 1 if equal, 0 if not.
--
-- Uses Lua's == operator for comparison.
-- Result is always an integer (1 or 0), matching C-style booleans.
dispatch[M.OP_CMP_EQ] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    local res = (a == b) and 1 or 0
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Compare %s == %s", tostring(a), tostring(b))
end

-- CMP_LT: Pop two numbers, push 1 if a < b, 0 otherwise.
dispatch[M.OP_CMP_LT] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    local res = (a < b) and 1 or 0
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Compare %s < %s", tostring(a), tostring(b))
end

-- CMP_GT: Pop two numbers, push 1 if a > b, 0 otherwise.
dispatch[M.OP_CMP_GT] = function(vm, instr, code)
    local b = vm:pop()
    local a = vm:pop()
    local res = (a > b) and 1 or 0
    vm.stack[#vm.stack + 1] = res
    vm.pc = vm.pc + 1
    return nil, string.format("Compare %s > %s", tostring(a), tostring(b))
end

-- JUMP: Unconditional jump to a target address.
--
-- The operand is the target PC value (1-indexed).
dispatch[M.OP_JUMP] = function(vm, instr, code)
    local target = instr.operand
    vm.pc = target
    return nil, string.format("Jump to instruction %d", target)
end

-- JUMP_IF_FALSE: Pop a value; if falsy, jump to target; else advance.
--
-- This is the foundation of if/else and while loops:
--   if condition is false -> jump past the body
--   if condition is true  -> fall through to the body
dispatch[M.OP_JUMP_IF_FALSE] = function(vm, instr, code)
    local target = instr.operand
    local val = vm:pop()
    if is_falsy(val) then
        vm.pc = target
    else
        vm.pc = vm.pc + 1
    end
    return nil, string.format("Pop %s, conditional jump", tostring(val))
end

-- JUMP_IF_TRUE: Pop a value; if truthy, jump to target; else advance.
dispatch[M.OP_JUMP_IF_TRUE] = function(vm, instr, code)
    local target = instr.operand
    local val = vm:pop()
    if not is_falsy(val) then
        vm.pc = target
    else
        vm.pc = vm.pc + 1
    end
    return nil, string.format("Pop %s, conditional jump", tostring(val))
end

-- CALL: Invoke a named function stored as a CodeObject in variables.
--
-- The process:
--   1. Look up the function name in code.names[operand].
--   2. Retrieve the CodeObject from vm.variables[name].
--   3. Save the current state (return address, variables, locals) in a frame.
--   4. Execute the function's instructions until RETURN or end.
--   5. Restore the saved state from the frame.
dispatch[M.OP_CALL] = function(vm, instr, code)
    local idx = instr.operand
    local func_name = code.names[idx]
    local func_obj = vm.variables[func_name]
    if func_obj == nil then
        error(string.format("UndefinedNameError: Function '%s' is not defined", func_name))
    end
    if type(func_obj) ~= "table" or func_obj.instructions == nil then
        error("VMError: Object is not callable")
    end

    -- Save current frame
    local frame = {
        return_address = vm.pc + 1,
        saved_variables = copy_map(vm.variables),
        saved_locals = copy_array(vm.locals),
    }
    vm.call_stack[#vm.call_stack + 1] = frame

    -- Execute the function body
    vm.locals = {}
    vm.pc = 1
    while not vm.halted and vm.pc <= #func_obj.instructions do
        if func_obj.instructions[vm.pc].opcode == M.OP_RETURN then
            break
        end
        vm:step(func_obj)
    end

    -- Restore from frame
    local popped = vm.call_stack[#vm.call_stack]
    vm.call_stack[#vm.call_stack] = nil
    vm.pc = popped.return_address
    vm.locals = popped.saved_locals
    return nil, string.format("Call function '%s'", func_name)
end

-- RETURN: Return from the current function.
--
-- If we're inside a function call:
--   - pop the call frame and restore state
-- If we're at the top level:
--   - halt the VM (program is done)
dispatch[M.OP_RETURN] = function(vm, instr, code)
    if #vm.call_stack > 0 then
        local popped = vm.call_stack[#vm.call_stack]
        vm.call_stack[#vm.call_stack] = nil
        vm.pc = popped.return_address
        vm.locals = popped.saved_locals
    else
        vm.halted = true
    end
    return nil, "Return from function"
end

-- PRINT: Pop a value and append its string representation to output.
dispatch[M.OP_PRINT] = function(vm, instr, code)
    local val = vm:pop()
    local str_val = tostring(val)
    vm.output[#vm.output + 1] = str_val
    vm.pc = vm.pc + 1
    return str_val, string.format("Print %s", tostring(val))
end

-- HALT: Stop execution.
dispatch[M.OP_HALT] = function(vm, instr, code)
    vm.halted = true
    return nil, "Halt execution"
end


-- =========================================================================
-- STEP AND EXECUTE
-- =========================================================================

--- Execute one instruction and return a VMTrace.
--
-- This is the core fetch-decode-execute cycle for a single instruction.
--
-- @param code table  The CodeObject to execute from.
-- @return table  A VMTrace describing what happened.
function VirtualMachine:step(code)
    local instr = code.instructions[self.pc]
    local pc_before = self.pc
    local stack_before = copy_array(self.stack)

    -- Look up the handler in the dispatch table
    local handler = dispatch[instr.opcode]
    if handler == nil then
        error(string.format("InvalidOpcodeError: Unknown opcode 0x%02x", instr.opcode))
    end

    -- Execute the handler
    local output_val, desc = handler(self, instr, code)

    return make_trace(
        pc_before, instr, stack_before,
        copy_array(self.stack), copy_map(self.variables),
        output_val, desc
    )
end

--- Execute a CodeObject from start to finish, collecting traces.
--
-- Runs until the VM is halted or the PC moves past the last instruction.
--
-- @param code table  The CodeObject to execute.
-- @return table  Array of VMTrace tables, one per instruction executed.
function VirtualMachine:execute(code)
    local traces = {}
    while not self.halted and self.pc <= #code.instructions do
        traces[#traces + 1] = self:step(code)
    end
    return traces
end


-- =========================================================================
-- COROUTINE-BASED STEPPER
-- =========================================================================
--
-- Lua coroutines give us cooperative multitasking for free.  A coroutine
-- is like a function that can pause itself (yield) and be resumed later.
--
-- create_stepper() wraps the execution loop in a coroutine.  Each call
-- to coroutine.resume() runs exactly one instruction, then yields the
-- VMTrace back to the caller.  This is perfect for:
--
--   - Step-through debuggers ("next" / "step into")
--   - Execution visualizers that animate each instruction
--   - Test harnesses that need to inspect state mid-program
--
-- Example:
--
--   local vm = VirtualMachine.new()
--   local stepper = vm:create_stepper(code)
--   local ok, trace = coroutine.resume(stepper)
--   -- trace contains the VMTrace for the first instruction
--   ok, trace = coroutine.resume(stepper)
--   -- trace contains the VMTrace for the second instruction
--   -- ...continues until trace is nil (execution complete)
--

--- Create a coroutine-based stepper for step-through debugging.
--
-- @param code table  The CodeObject to execute.
-- @return thread  A Lua coroutine that yields VMTrace on each step.
function VirtualMachine:create_stepper(code)
    return coroutine.create(function()
        while not self.halted and self.pc <= #code.instructions do
            local trace = self:step(code)
            coroutine.yield(trace)
        end
    end)
end


M.VirtualMachine = VirtualMachine


-- =========================================================================
-- GENERIC VM (Handler-Based, Pluggable Opcodes)
-- =========================================================================
--
-- The GenericVM takes a different approach: it knows NOTHING about
-- individual opcodes.  Instead, you register handler functions for each
-- opcode, and the VM dispatches to them at execution time.
--
-- Think of it like a telephone switchboard.  The VM is the operator who
-- connects calls, but doesn't know what the callers are saying.  The
-- handlers are the callers who do the actual work.
--
-- This separation makes it trivial to:
--   - Add new opcodes without touching the VM core.
--   - Swap opcode sets to emulate different bytecode formats.
--   - Test individual opcodes in isolation.
--   - Freeze a VM configuration so no more opcodes can be registered
--     (useful for sandboxing).
--
-- CALL STACK AND RECURSION
-- ========================
--
-- When a function calls another function, the VM needs to remember
-- where to return to.  This is the call stack -- a stack of "frames",
-- each holding saved state as a generic Lua table.
--
-- The handler for CALL decides what to put in the frame, and the handler
-- for RETURN decides how to restore it.  This keeps the VM generic.
--
-- To prevent infinite recursion, you can set max_recursion_depth.
-- If the call stack exceeds this depth, push_frame errors with
-- "MaxRecursionError".
--
--   nil -> unlimited recursion (default)
--   0   -> no function calls allowed at all
--   100 -> up to 100 nested calls
--

local GenericVM = {}
GenericVM.__index = GenericVM

--- Create a new GenericVM with empty state and no registered handlers.
--
-- You must register opcode handlers before executing any code.
--
-- @return GenericVM
function GenericVM.new()
    local self = setmetatable({}, GenericVM)
    self.stack = {}
    self.variables = {}
    self.locals = {}
    self.pc = 1
    self.halted = false
    self.output = {}
    self.call_stack = {}
    self._handlers = {}     -- opcode -> handler function
    self._builtins = {}     -- name -> { name=..., implementation=... }
    self._max_recursion_depth = nil   -- nil = unlimited
    self._frozen = false

    -- =====================================================================
    -- TYPED STACK
    -- =====================================================================
    --
    -- WebAssembly is statically typed: every value on the operand stack has
    -- a known type (i32, i64, f32, f64, funcref, externref). The "typed
    -- stack" is a parallel array of {value=V, type=T} entries that tracks
    -- the type of each stack slot at runtime.
    --
    -- Why a separate stack?  The original GenericVM.stack holds raw Lua
    -- values (numbers, strings, etc.) for simplicity and backward
    -- compatibility. The typed_stack adds type annotations for runtimes
    -- (like Wasm) that require type-aware operations.
    --
    -- Usage:
    --   vm:push_typed(42, "i32")      -- push an i32 value
    --   local entry = vm:pop_typed()  -- returns {value=42, type="i32"}
    --   local top = vm:peek_typed()   -- look without removing
    --
    self.typed_stack = {}

    -- =====================================================================
    -- INSTRUCTION HOOKS
    -- =====================================================================
    --
    -- Hooks run before/after every instruction in execute_with_context.
    -- They are useful for tracing, profiling, breakpoints, and debugging.
    --
    -- Each hook is a function(vm, instr, code) that can inspect or modify
    -- the VM state. Returning a truthy value from pre_instruction_hook
    -- skips the instruction (useful for breakpoints that want to pause).
    --
    self.pre_instruction_hook = nil   -- function(vm, instr, code) or nil
    self.post_instruction_hook = nil  -- function(vm, instr, code) or nil

    -- =====================================================================
    -- CONTEXT HANDLERS
    -- =====================================================================
    --
    -- Context opcodes are a second dispatch table for instructions that
    -- need access to external runtime context (linear memory, globals,
    -- tables, host functions). Regular opcodes modify only the VM's
    -- internal state; context opcodes can reach outside the VM into the
    -- execution environment.
    --
    -- In WebAssembly, instructions like memory.load, global.get, and
    -- call_indirect need access to the module instance's linear memory,
    -- global variable store, and function table respectively. Rather than
    -- baking these into the VM, we let the execution layer register
    -- context handlers that receive the runtime context as an extra
    -- argument.
    --
    -- Handler signature:
    --   function(vm, instr, code, context) -> string_or_nil
    --
    -- The `context` parameter is an opaque table provided by the caller
    -- of execute_with_context. It typically holds references to memory,
    -- globals, tables, and other module instance state.
    --
    self._context_handlers = {}

    return self
end


-- =========================================================================
-- REGISTRATION -- Adding opcodes and builtins
-- =========================================================================

--- Register an opcode handler function.
--
-- When the VM encounters this opcode during execution, it will call
-- the handler.  The handler signature is:
--
--   function(vm, instr, code) -> string_or_nil
--
-- The handler receives the GenericVM instance, the current instruction,
-- and the full CodeObject.  It should modify the VM state (stack, PC,
-- variables) and return a string if it produced output, or nil otherwise.
--
-- If the VM is frozen, this errors with "FrozenVMError".
-- Registering the same opcode twice silently overwrites the previous handler.
--
-- @param opcode  number          The opcode constant.
-- @param handler function        The handler function.
function GenericVM:register_opcode(opcode, handler)
    if self._frozen then
        error("FrozenVMError: cannot register opcodes on a frozen VM")
    end
    self._handlers[opcode] = handler
end

--- Register a named built-in function.
--
-- Built-in functions are a "standard library" that opcode handlers can
-- look up by name.  They are stored separately from user variables.
--
-- @param name string       The built-in's name (e.g. "len", "abs").
-- @param impl function     The implementation function.
function GenericVM:register_builtin(name, impl)
    if self._frozen then
        error("FrozenVMError: cannot register builtins on a frozen VM")
    end
    self._builtins[name] = {
        name = name,
        implementation = impl,
    }
end

--- Retrieve a registered built-in function by name.
--
-- @param name string  The built-in's name.
-- @return table|nil   The builtin table { name, implementation }, or nil.
function GenericVM:get_builtin(name)
    return self._builtins[name]
end


-- =========================================================================
-- STACK OPERATIONS
-- =========================================================================
--
-- The stack is the heart of a stack-based VM.  Almost every operation
-- works by pushing values onto the stack or popping them off.
--
-- Analogy: think of a stack of plates in a cafeteria.
--   Push = put a plate on top.
--   Pop  = take the top plate off.
--   Peek = look at the top plate without removing it.
--
-- If you try to pop or peek when the stack is empty, that's an error
-- (you can't take a plate from an empty stack), so we error out.

--- Push a value onto the operand stack.
-- @param value any  The value to push.
function GenericVM:push(value)
    self.stack[#self.stack + 1] = value
end

--- Pop and return the top value from the operand stack.
-- Errors with "StackUnderflowError" if the stack is empty.
-- @return any  The popped value.
function GenericVM:pop()
    return stack_pop(self.stack)
end

--- Return the top value from the operand stack WITHOUT removing it.
-- Errors with "StackUnderflowError" if the stack is empty.
-- @return any  The top value.
function GenericVM:peek()
    if #self.stack == 0 then
        error("StackUnderflowError")
    end
    return self.stack[#self.stack]
end


-- =========================================================================
-- TYPED STACK OPERATIONS
-- =========================================================================
--
-- The typed stack stores {value, type} pairs. It mirrors the regular
-- stack but carries explicit type tags. WebAssembly needs this because
-- the same Lua number could represent an i32, i64, f32, or f64 — the
-- type determines how arithmetic wraps and how comparisons behave.
--
-- The typed stack is independent of the regular stack. You can use
-- either or both depending on your needs:
--   - Regular stack: simple, for non-typed bytecodes
--   - Typed stack: for WebAssembly and other typed VMs

--- Push a typed value onto the typed stack.
--
-- @param value any     The value to push.
-- @param val_type string  The type tag (e.g., "i32", "i64", "f32", "f64").
function GenericVM:push_typed(value, val_type)
    self.typed_stack[#self.typed_stack + 1] = { value = value, type = val_type }
end

--- Pop and return the top typed entry from the typed stack.
--
-- Returns a table with {value=V, type=T}.
-- Errors with "StackUnderflowError" if the typed stack is empty.
--
-- @return table  {value=V, type=T}
function GenericVM:pop_typed()
    if #self.typed_stack == 0 then
        error("StackUnderflowError: typed stack is empty")
    end
    local entry = self.typed_stack[#self.typed_stack]
    self.typed_stack[#self.typed_stack] = nil
    return entry
end

--- Peek at the top typed entry without removing it.
--
-- Returns a table with {value=V, type=T}.
-- Errors with "StackUnderflowError" if the typed stack is empty.
--
-- @return table  {value=V, type=T}
function GenericVM:peek_typed()
    if #self.typed_stack == 0 then
        error("StackUnderflowError: typed stack is empty")
    end
    return self.typed_stack[#self.typed_stack]
end


-- =========================================================================
-- CALL STACK OPERATIONS
-- =========================================================================
--
-- Each frame is a generic Lua table (like a map).  The handler decides
-- what to save (return address, variables, locals, etc.) and the RETURN
-- handler decides how to restore it.
--
-- Why a generic table instead of a fixed struct?  Because different
-- languages save different things.  A generic frame keeps the VM
-- language-agnostic.

--- Push a frame onto the call stack.
--
-- If max_recursion_depth is set and the call stack would exceed it,
-- errors with "MaxRecursionError".
--
-- Truth table:
--   max_recursion_depth | #call_stack | Action
--   --------------------|-------------|----------
--   nil                 | any         | allow
--   0                   | 0           | error
--   3                   | 2           | allow
--   3                   | 3           | error
--
-- @param frame table  The frame to save.
function GenericVM:push_frame(frame)
    if self._max_recursion_depth ~= nil then
        if #self.call_stack >= self._max_recursion_depth then
            error("MaxRecursionError")
        end
    end
    self.call_stack[#self.call_stack + 1] = frame
end

--- Pop and return the top frame from the call stack.
-- Errors with "CallStackUnderflowError" if the call stack is empty.
-- @return table  The popped frame.
function GenericVM:pop_frame()
    if #self.call_stack == 0 then
        error("CallStackUnderflowError")
    end
    local frame = self.call_stack[#self.call_stack]
    self.call_stack[#self.call_stack] = nil
    return frame
end


-- =========================================================================
-- PROGRAM COUNTER CONTROL
-- =========================================================================

--- Advance the program counter by 1 (normal sequential execution).
function GenericVM:advance_pc()
    self.pc = self.pc + 1
end

--- Set the program counter to an arbitrary target address.
-- Used by jump/branch instructions.
-- @param target number  The target PC value (1-indexed).
function GenericVM:jump_to(target)
    self.pc = target
end


-- =========================================================================
-- CONFIGURATION
-- =========================================================================

--- Set the maximum call stack depth.
-- Pass nil for unlimited recursion.  Pass 0 to disallow any function calls.
-- @param depth number|nil  The maximum depth, or nil for unlimited.
function GenericVM:set_max_recursion_depth(depth)
    self._max_recursion_depth = depth
end

--- Get the current max recursion depth setting.
-- @return number|nil  The current depth, or nil if unlimited.
function GenericVM:get_max_recursion_depth()
    return self._max_recursion_depth
end

--- Freeze or unfreeze the VM's handler/builtin registration.
-- A frozen VM will error if you try to register new opcodes or builtins.
-- This is useful for sandboxing.
-- @param frozen boolean  True to freeze, false to unfreeze.
function GenericVM:set_frozen(frozen)
    self._frozen = frozen
end

--- Check whether the VM is currently frozen.
-- @return boolean
function GenericVM:is_frozen()
    return self._frozen
end


-- =========================================================================
-- EXECUTION
-- =========================================================================

--- Execute one instruction and return a VMTrace.
--
-- The process:
--   1. FETCH:    Read code.instructions[vm.pc].
--   2. SNAPSHOT: Copy the stack before execution.
--   3. DECODE:   Look up the handler for instr.opcode.
--   4. EXECUTE:  Call the handler.
--   5. TRACE:    Build a VMTrace with before/after snapshots.
--
-- If no handler is registered for the opcode, errors with
-- "InvalidOpcodeError".
--
-- @param code table  The CodeObject to execute from.
-- @return table  A VMTrace describing what happened.
function GenericVM:step(code)
    -- 1. FETCH
    local instr = code.instructions[self.pc]
    local pc_before = self.pc

    -- 2. SNAPSHOT
    local stack_before = copy_array(self.stack)

    -- 3. DECODE
    local handler = self._handlers[instr.opcode]
    if handler == nil then
        error(string.format("InvalidOpcodeError: no handler registered for opcode 0x%02x", instr.opcode))
    end

    -- 4. EXECUTE
    local output_val = handler(self, instr, code)

    -- Record output if the handler produced any
    if output_val ~= nil then
        self.output[#self.output + 1] = output_val
    end

    -- 5. TRACE
    return make_trace(
        pc_before, instr, stack_before,
        copy_array(self.stack), copy_map(self.variables),
        output_val,
        string.format("Executed opcode 0x%02x", instr.opcode)
    )
end

--- Execute a CodeObject from start to finish, collecting traces.
--
-- Runs until the VM is halted or the PC moves past the last instruction.
--
-- @param code table  The CodeObject to execute.
-- @return table  Array of VMTrace tables.
function GenericVM:execute(code)
    local traces = {}
    while not self.halted and self.pc <= #code.instructions do
        traces[#traces + 1] = self:step(code)
    end
    return traces
end


-- =========================================================================
-- CONTEXT OPCODE REGISTRATION
-- =========================================================================
--
-- Context opcodes extend the GenericVM with handlers that receive an
-- additional `context` parameter. This is the mechanism by which the
-- WebAssembly execution engine connects memory loads, global accesses,
-- and function calls to the module instance's runtime state.
--
-- When the VM encounters an opcode during execute_with_context, it
-- first checks the context handler table, then falls back to the
-- regular handler table. This means context handlers take priority
-- for opcodes that appear in both tables.

--- Register a context-aware opcode handler.
--
-- Context handlers have the signature:
--   function(vm, instr, code, context) -> string_or_nil
--
-- The `context` argument is the opaque table passed to execute_with_context.
--
-- @param opcode  number    The opcode constant.
-- @param handler function  The context-aware handler function.
function GenericVM:register_context_opcode(opcode, handler)
    if self._frozen then
        error("FrozenVMError: cannot register context opcodes on a frozen VM")
    end
    self._context_handlers[opcode] = handler
end

-- =========================================================================
-- EXECUTE WITH CONTEXT
-- =========================================================================
--
-- execute_with_context is the main execution loop for typed/contextual
-- bytecodes like WebAssembly. It differs from plain execute() in three
-- ways:
--
--   1. Context handlers are checked before regular handlers.
--   2. pre_instruction_hook and post_instruction_hook are called.
--   3. The context table is passed to context handlers.
--
-- This method does NOT collect traces for performance — Wasm execution
-- can be millions of instructions, and allocating a trace per instruction
-- would be prohibitively expensive. Use the hooks for debugging instead.

--- Execute a CodeObject with a runtime context.
--
-- @param code    table  The CodeObject (instructions, constants, names).
-- @param context table  Opaque context table passed to context handlers.
function GenericVM:execute_with_context(code, context)
    while not self.halted and self.pc <= #code.instructions do
        local instr = code.instructions[self.pc]

        -- Pre-instruction hook (for debugging, tracing, breakpoints)
        if self.pre_instruction_hook then
            local skip = self.pre_instruction_hook(self, instr, code)
            if skip then
                -- Hook requested we skip this instruction (e.g., breakpoint)
                goto continue
            end
        end

        -- Look up context handler first, then regular handler
        local ctx_handler = self._context_handlers[instr.opcode]
        if ctx_handler then
            local output_val = ctx_handler(self, instr, code, context)
            if output_val ~= nil then
                self.output[#self.output + 1] = output_val
            end
        else
            local handler = self._handlers[instr.opcode]
            if handler == nil then
                error(string.format(
                    "InvalidOpcodeError: no handler registered for opcode 0x%02x",
                    instr.opcode))
            end
            local output_val = handler(self, instr, code)
            if output_val ~= nil then
                self.output[#self.output + 1] = output_val
            end
        end

        -- Post-instruction hook
        if self.post_instruction_hook then
            self.post_instruction_hook(self, instr, code)
        end

        ::continue::
    end
end


-- =========================================================================
-- COROUTINE-BASED STEPPER (GenericVM)
-- =========================================================================

--- Create a coroutine-based stepper for step-through debugging.
--
-- Works identically to VirtualMachine:create_stepper().
--
-- @param code table  The CodeObject to execute.
-- @return thread  A Lua coroutine that yields VMTrace on each step.
function GenericVM:create_stepper(code)
    return coroutine.create(function()
        while not self.halted and self.pc <= #code.instructions do
            local trace = self:step(code)
            coroutine.yield(trace)
        end
    end)
end


-- =========================================================================
-- RESET
-- =========================================================================

--- Reset all runtime state but preserve registered handlers, builtins,
-- and configuration (max_recursion_depth, frozen).
--
-- This lets you reuse a configured VM to run multiple programs without
-- re-registering all the opcodes.
function GenericVM:reset()
    self.stack = {}
    self.variables = {}
    self.locals = {}
    self.pc = 1
    self.halted = false
    self.output = {}
    self.call_stack = {}
end


M.GenericVM = GenericVM


-- =========================================================================
-- EXPORTED HELPERS
-- =========================================================================
--
-- We export the helper functions so that opcode handlers registered with
-- GenericVM can use them, and so tests can exercise them directly.

M.is_falsy = is_falsy
M.vm_add = vm_add
M.copy_array = copy_array
M.copy_map = copy_map
M.instruction_tostring = M.instruction_tostring

return M

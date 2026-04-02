--- coding_adventures.jvm_simulator — JVM bytecode interpreter
--
-- # What is the JVM?
--
-- The Java Virtual Machine (1995) is the most widely deployed virtual machine
-- in history. It runs Java, Kotlin, Scala, Clojure, and Groovy. Like the CLR
-- and our own VM, it is a **stack-based machine** — instructions operate on
-- an operand stack rather than CPU registers.
--
-- # Typed Opcodes: A Key JVM Design Choice
--
-- Unlike our VM (which has one ADD instruction) or the CLR (which has `add`
-- and infers the type), the JVM has **separate opcodes for each type**:
--
--   `iadd` — integer add (32-bit signed)
--   `ladd` — long add   (64-bit signed)
--   `fadd` — float add  (32-bit IEEE 754)
--   `dadd` — double add (64-bit IEEE 754)
--
-- This means the JVM can verify type safety at load time without needing to
-- track types at runtime — a performance and security win.
--
-- For this educational simulator, we implement the `i` (integer) variants only.
-- Real JVM programs targeting Java integers use these exclusively for basic
-- integer arithmetic.
--
-- # Local Variable Array
--
-- Every JVM method frame has both an **operand stack** AND a **local variable
-- array**. The array is zero-indexed (slot 0 is usually `this` for instance
-- methods). Like the CLR, short-form opcodes (iload_0-3, istore_0-3) avoid
-- encoding a slot number for the first four slots.
--
-- # Constant Pool
--
-- The JVM has a class-level "constant pool" — a table of string literals,
-- numeric constants, class/method references. The `ldc` instruction loads
-- a constant pool entry by 1-byte index. Our simulator accepts a Lua array
-- as the constant pool.
--
-- # Bytecode Encoding
--
-- JVM bytecode is variable-width. Branch instructions (`goto`, `if_icmpeq`,
-- `if_icmpgt`) use a **big-endian 16-bit signed offset**. This differs from
-- the CLR which uses small 8-bit offsets for short branches.
--
-- Example: `x = 1 + 2`
--
--   iconst_1  → 0x04       push integer 1
--   iconst_2  → 0x05       push integer 2
--   iadd      → 0x60       pop two ints, push sum
--   istore_0  → 0x3B       store int in local[0]
--   return    → 0xB1       return void
--
-- @module coding_adventures.jvm_simulator

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Opcode Constants (real JVM hex values for educational accuracy)
-- ============================================================================

-- Push integer constants (short forms 0-5)
-- These are consecutive: iconst_0=0x03, iconst_1=0x04, ..., iconst_5=0x08
M.ICONST_0   = 0x03   -- push int 0
M.ICONST_1   = 0x04   -- push int 1
M.ICONST_2   = 0x05   -- push int 2
M.ICONST_3   = 0x06   -- push int 3
M.ICONST_4   = 0x07   -- push int 4
M.ICONST_5   = 0x08   -- push int 5

-- Push constants with operands
M.BIPUSH     = 0x10   -- push signed byte as int (1-byte operand)
M.SIPUSH     = 0x11   -- push signed short as int (2-byte big-endian operand)
M.LDC        = 0x12   -- push value from constant pool (1-byte index)

-- Load integer from local variable
-- Short forms for slots 0-3; generic form takes a 1-byte slot index.
M.ILOAD      = 0x15   -- load int from local[N]  (1-byte operand)
M.ILOAD_0    = 0x1A   -- load int from local[0]
M.ILOAD_1    = 0x1B   -- load int from local[1]
M.ILOAD_2    = 0x1C   -- load int from local[2]
M.ILOAD_3    = 0x1D   -- load int from local[3]

-- Store integer to local variable
M.ISTORE     = 0x36   -- store int to local[N]  (1-byte operand)
M.ISTORE_0   = 0x3B   -- store int to local[0]
M.ISTORE_1   = 0x3C   -- store int to local[1]
M.ISTORE_2   = 0x3D   -- store int to local[2]
M.ISTORE_3   = 0x3E   -- store int to local[3]

-- Integer arithmetic
M.IADD       = 0x60   -- pop b, pop a, push a+b (truncated to int32)
M.ISUB       = 0x64   -- pop b, pop a, push a-b
M.IMUL       = 0x68   -- pop b, pop a, push a*b
M.IDIV       = 0x6C   -- pop b, pop a, push trunc(a/b) — raises on b=0

-- Control flow
-- JVM branches use 16-bit big-endian offsets relative to the branch
-- instruction's own PC (not the next PC, unlike most other VMs).
M.IF_ICMPEQ  = 0x9F   -- pop b, pop a; branch if a == b (2-byte offset)
M.IF_ICMPGT  = 0xA3   -- pop b, pop a; branch if a > b  (2-byte offset)
M.GOTO       = 0xA7   -- unconditional branch (2-byte signed offset)

-- Return
M.IRETURN    = 0xAC   -- pop int, halt with return_value set
M.RETURN     = 0xB1   -- void return (no value)

-- ============================================================================
-- Trace Record
-- ============================================================================

--- Build a trace record for one executed instruction.
-- Each trace captures the full before/after state so you can replay or debug.
local function make_trace(pc, opcode, before, sim, description)
    local locals_copy = {}
    for i, v in ipairs(sim.locals) do
        locals_copy[i] = v
    end
    return {
        pc           = pc,
        opcode       = opcode,
        stack_before = before,
        stack_after  = {table.unpack(sim.stack)},
        locals       = locals_copy,
        description  = description,
    }
end

-- ============================================================================
-- Simulator Constructor
-- ============================================================================

--- Create a new JVM simulator instance.
--
-- The JVM simulator is a pure value type. Each step() call returns a NEW
-- simulator state — the original is not modified.
--
-- Fields:
--   sim.stack        — operand stack (array, #stack = top)
--   sim.locals       — local variable array (1-indexed in Lua; slot 0 = index 1)
--   sim.constants    — constant pool (array, 1-indexed; ldc index 0 = constants[1])
--   sim.pc           — program counter (0-based byte index)
--   sim.bytecode     — array of byte integers (0-255)
--   sim.halted       — true after return/ireturn
--   sim.return_value — integer return value (set by ireturn) or nil
--
-- @return table
function M.new()
    return {
        stack        = {},
        locals       = {},
        constants    = {},
        pc           = 0,
        bytecode     = {},
        halted       = false,
        return_value = nil,
    }
end

--- Load bytecode into the simulator.
--
-- @param sim       table  — simulator instance
-- @param bytecode  table  — array of byte integers
-- @param opts      table  — optional { constants={}, num_locals=16 }
-- @return table — new simulator state
function M.load(sim, bytecode, opts)
    opts = opts or {}
    local num_locals = opts.num_locals or 16
    local constants  = opts.constants or {}
    -- Use a __len metatable so that #sim.locals returns num_locals even when
    -- all slots are nil. Plain Lua tables with nil values return 0 from #.
    local new_locals = setmetatable({}, {__len = function() return num_locals end})
    for i = 1, num_locals do
        new_locals[i] = nil
    end
    return {
        stack        = {},
        locals       = new_locals,
        constants    = constants,
        pc           = 0,
        bytecode     = bytecode,
        halted       = false,
        return_value = nil,
    }
end

-- ============================================================================
-- Byte Reading Helpers
-- ============================================================================

local function byte_at(bytecode, pos)
    return bytecode[pos + 1]
end

local function signed_byte_at(bytecode, pos)
    local v = bytecode[pos + 1]
    if v >= 128 then return v - 256 end
    return v
end

-- Big-endian 16-bit signed integer (used by goto, if_icmpeq, if_icmpgt).
-- JVM branches compute: target = instruction_pc + offset
local function big_signed16_at(bytecode, pos)
    local hi = bytecode[pos + 1]
    local lo = bytecode[pos + 2]
    local v = hi * 256 + lo
    if v >= 32768 then v = v - 65536 end
    return v
end

-- ============================================================================
-- Stack Helpers
-- ============================================================================

local function pop(sim)
    if #sim.stack == 0 then
        error("Stack underflow")
    end
    local val = sim.stack[#sim.stack]
    local new_stack = {}
    for i = 1, #sim.stack - 1 do
        new_stack[i] = sim.stack[i]
    end
    sim.stack = new_stack
    return sim, val
end

local function push(sim, val)
    local new_stack = {table.unpack(sim.stack)}
    new_stack[#new_stack + 1] = val
    sim.stack = new_stack
    return sim
end

-- ============================================================================
-- Integer Overflow Helper
-- ============================================================================

-- The JVM operates on 32-bit signed integers. Arithmetic wraps at 32 bits.
-- Lua's default integer is 64-bit, so we must clamp results.
local function to_i32(value)
    value = value % 4294967296  -- mask to 32 bits
    if value >= 2147483648 then
        value = value - 4294967296
    end
    return value
end

-- ============================================================================
-- Execute One Instruction
-- ============================================================================

--- Execute one JVM bytecode instruction.
--
-- @param sim  table — current simulator state
-- @return sim, trace — updated state and trace record
function M.step(sim)
    if sim.halted then
        error("JVM simulator has halted")
    end
    if sim.pc >= #sim.bytecode then
        error(string.format("PC (%d) past end of bytecode (%d bytes)", sim.pc, #sim.bytecode))
    end

    local stack_before = {table.unpack(sim.stack)}
    local opcode = byte_at(sim.bytecode, sim.pc)

    -- ---- iconst_0 through iconst_5 ------------------------------------------
    -- Six short-form integer constants. JVM is more limited than CLR here
    -- (CLR goes from 0 to 8). Values outside 0-5 need bipush or ldc.
    if opcode >= M.ICONST_0 and opcode <= M.ICONST_5 then
        local pc = sim.pc
        local value = opcode - M.ICONST_0
        sim = push(sim, value)
        sim.pc = pc + 1
        return sim, make_trace(pc, string.format("iconst_%d", value), stack_before, sim,
            string.format("push %d", value))

    -- ---- bipush -------------------------------------------------------------
    -- Push a signed byte (-128 to 127) as a 32-bit integer.
    elseif opcode == M.BIPUSH then
        local pc = sim.pc
        local value = signed_byte_at(sim.bytecode, pc + 1)
        sim = push(sim, value)
        sim.pc = pc + 2
        return sim, make_trace(pc, "bipush", stack_before, sim,
            string.format("push %d", value))

    -- ---- sipush -------------------------------------------------------------
    -- Push a signed 16-bit short (-32768 to 32767) as a 32-bit integer.
    elseif opcode == M.SIPUSH then
        local pc = sim.pc
        local value = big_signed16_at(sim.bytecode, pc + 1)
        sim = push(sim, value)
        sim.pc = pc + 3
        return sim, make_trace(pc, "sipush", stack_before, sim,
            string.format("push %d", value))

    -- ---- ldc ----------------------------------------------------------------
    -- Load a constant pool entry by 1-byte index.
    -- The constant pool is 0-indexed at the JVM level, but our Lua array is
    -- 1-indexed, so constants[index + 1] is the correct lookup.
    elseif opcode == M.LDC then
        local pc = sim.pc
        local index = byte_at(sim.bytecode, pc + 1)
        if index >= #sim.constants then
            error(string.format("Constant pool index %d out of range", index))
        end
        local value = sim.constants[index + 1]
        if type(value) ~= "number" then
            error(string.format("ldc: constant pool entry %d is not a number", index))
        end
        sim = push(sim, value)
        sim.pc = pc + 2
        return sim, make_trace(pc, "ldc", stack_before, sim,
            string.format("push constant[%d] = %s", index, tostring(value)))

    -- ---- iload_0 through iload_3 --------------------------------------------
    elseif opcode >= M.ILOAD_0 and opcode <= M.ILOAD_3 then
        local pc = sim.pc
        local slot = opcode - M.ILOAD_0
        local value = sim.locals[slot + 1]
        if value == nil then
            error(string.format("Local variable %d has not been initialized", slot))
        end
        sim = push(sim, value)
        sim.pc = pc + 1
        return sim, make_trace(pc, string.format("iload_%d", slot), stack_before, sim,
            string.format("push locals[%d] = %d", slot, value))

    -- ---- iload (with slot operand) ------------------------------------------
    elseif opcode == M.ILOAD then
        local pc = sim.pc
        local slot = byte_at(sim.bytecode, pc + 1)
        local value = sim.locals[slot + 1]
        if value == nil then
            error(string.format("Local variable %d has not been initialized", slot))
        end
        sim = push(sim, value)
        sim.pc = pc + 2
        return sim, make_trace(pc, "iload", stack_before, sim,
            string.format("push locals[%d] = %d", slot, value))

    -- ---- istore_0 through istore_3 ------------------------------------------
    elseif opcode >= M.ISTORE_0 and opcode <= M.ISTORE_3 then
        local pc = sim.pc
        local slot = opcode - M.ISTORE_0
        sim, value = pop(sim)
        sim.locals[slot + 1] = value
        sim.pc = pc + 1
        return sim, make_trace(pc, string.format("istore_%d", slot), stack_before, sim,
            string.format("pop %d, store in locals[%d]", value, slot))

    -- ---- istore (with slot operand) -----------------------------------------
    elseif opcode == M.ISTORE then
        local pc = sim.pc
        local slot = byte_at(sim.bytecode, pc + 1)
        sim, value = pop(sim)
        sim.locals[slot + 1] = value
        sim.pc = pc + 2
        return sim, make_trace(pc, "istore", stack_before, sim,
            string.format("pop %d, store in locals[%d]", value, slot))

    -- ---- iadd ---------------------------------------------------------------
    -- Pop b then a (LIFO), push a+b clamped to int32.
    elseif opcode == M.IADD then
        local pc = sim.pc
        sim, b = pop(sim)
        sim, a = pop(sim)
        local result = to_i32(a + b)
        sim = push(sim, result)
        sim.pc = pc + 1
        return sim, make_trace(pc, "iadd", stack_before, sim,
            string.format("pop %d and %d, push %d", b, a, result))

    -- ---- isub ---------------------------------------------------------------
    elseif opcode == M.ISUB then
        local pc = sim.pc
        sim, b = pop(sim)
        sim, a = pop(sim)
        local result = to_i32(a - b)
        sim = push(sim, result)
        sim.pc = pc + 1
        return sim, make_trace(pc, "isub", stack_before, sim,
            string.format("pop %d and %d, push %d", b, a, result))

    -- ---- imul ---------------------------------------------------------------
    elseif opcode == M.IMUL then
        local pc = sim.pc
        sim, b = pop(sim)
        sim, a = pop(sim)
        local result = to_i32(a * b)
        sim = push(sim, result)
        sim.pc = pc + 1
        return sim, make_trace(pc, "imul", stack_before, sim,
            string.format("pop %d and %d, push %d", b, a, result))

    -- ---- idiv ---------------------------------------------------------------
    -- Integer division truncates toward zero.
    -- Raises ArithmeticException on division by zero.
    elseif opcode == M.IDIV then
        local pc = sim.pc
        sim, b = pop(sim)
        sim, a = pop(sim)
        if b == 0 then
            error("ArithmeticException: / by zero")
        end
        local result = to_i32(math.tointeger and math.tointeger(a / b) or math.floor(a / b))
        if result == nil then result = math.floor(a / b) end
        sim = push(sim, result)
        sim.pc = pc + 1
        return sim, make_trace(pc, "idiv", stack_before, sim,
            string.format("pop %d and %d, push %d", b, a, result))

    -- ---- goto ---------------------------------------------------------------
    -- Unconditional branch. Offset is relative to the goto instruction's own PC.
    -- target = instruction_pc + offset  (NOT next_pc + offset like CLR!)
    elseif opcode == M.GOTO then
        local pc = sim.pc
        local offset = big_signed16_at(sim.bytecode, pc + 1)
        local target = pc + offset
        sim.pc = target
        return sim, make_trace(pc, "goto", stack_before, sim,
            string.format("jump to PC=%d (offset %s%d)",
                target, offset >= 0 and "+" or "", offset))

    -- ---- if_icmpeq ----------------------------------------------------------
    -- Pop b then a; if a == b, branch (offset relative to this instruction).
    elseif opcode == M.IF_ICMPEQ then
        local pc = sim.pc
        local offset = big_signed16_at(sim.bytecode, pc + 1)
        sim, b = pop(sim)
        sim, a = pop(sim)
        local taken = (a == b)
        if taken then
            sim.pc = pc + offset
            return sim, make_trace(pc, "if_icmpeq", stack_before, sim,
                string.format("pop %d and %d, %d == %d is true, jump to PC=%d",
                    b, a, a, b, sim.pc))
        else
            sim.pc = pc + 3
            return sim, make_trace(pc, "if_icmpeq", stack_before, sim,
                string.format("pop %d and %d, %d == %d is false, fall through",
                    b, a, a, b))
        end

    -- ---- if_icmpgt ----------------------------------------------------------
    -- Pop b then a; if a > b, branch.
    elseif opcode == M.IF_ICMPGT then
        local pc = sim.pc
        local offset = big_signed16_at(sim.bytecode, pc + 1)
        sim, b = pop(sim)
        sim, a = pop(sim)
        local taken = (a > b)
        if taken then
            sim.pc = pc + offset
            return sim, make_trace(pc, "if_icmpgt", stack_before, sim,
                string.format("pop %d and %d, %d > %d is true, jump to PC=%d",
                    b, a, a, b, sim.pc))
        else
            sim.pc = pc + 3
            return sim, make_trace(pc, "if_icmpgt", stack_before, sim,
                string.format("pop %d and %d, %d > %d is false, fall through",
                    b, a, a, b))
        end

    -- ---- ireturn ------------------------------------------------------------
    -- Pop the top int and halt with it as the return value.
    elseif opcode == M.IRETURN then
        local pc = sim.pc
        sim, value = pop(sim)
        sim.return_value = value
        sim.halted = true
        sim.pc = pc + 1
        return sim, make_trace(pc, "ireturn", stack_before, sim,
            string.format("return %d", value))

    -- ---- return (void) ------------------------------------------------------
    elseif opcode == M.RETURN then
        local pc = sim.pc
        sim.halted = true
        sim.pc = pc + 1
        return sim, make_trace(pc, "return", stack_before, sim, "return void")

    else
        error(string.format("Unknown JVM opcode: 0x%02X at PC=%d", opcode, sim.pc))
    end
end

-- ============================================================================
-- Run to Completion
-- ============================================================================

--- Run the simulator until halted or max_steps reached.
--
-- @param sim   table — loaded simulator
-- @param opts  table — optional { max_steps=10000 }
-- @return sim, traces
function M.run(sim, opts)
    opts = opts or {}
    local max_steps = opts.max_steps or 10000
    local traces = {}
    for _ = 1, max_steps do
        if sim.halted then break end
        local trace
        sim, trace = M.step(sim)
        traces[#traces + 1] = trace
    end
    return sim, traces
end

-- ============================================================================
-- Assembly Helpers
-- ============================================================================

--- Encode an integer constant in the most compact JVM form.
--
--   0-5:      iconst_N  (1 byte)
--   -128..127 (outside 0-5): bipush N  (2 bytes)
--   (values outside ±128 need ldc or sipush)
--
-- @param n  integer — value to push
-- @return table — array of byte integers
function M.encode_iconst(n)
    if n >= 0 and n <= 5 then
        return { M.ICONST_0 + n }
    elseif n >= -128 and n <= 127 then
        local b = n < 0 and (n + 256) or n
        return { M.BIPUSH, b }
    else
        error(string.format("encode_iconst: value %d outside bipush range; use sipush or ldc", n))
    end
end

--- Encode an istore instruction for the given slot.
-- @param slot  integer — local variable index (0-based)
-- @return table — array of byte integers
function M.encode_istore(slot)
    if slot >= 0 and slot <= 3 then
        return { M.ISTORE_0 + slot }
    else
        return { M.ISTORE, slot }
    end
end

--- Encode an iload instruction for the given slot.
-- @param slot  integer — local variable index (0-based)
-- @return table — array of byte integers
function M.encode_iload(slot)
    if slot >= 0 and slot <= 3 then
        return { M.ILOAD_0 + slot }
    else
        return { M.ILOAD, slot }
    end
end

--- Assemble multiple byte arrays into a single flat byte array.
-- @param parts  table — array of byte arrays
-- @return table — flat array of byte integers
function M.assemble(parts)
    local result = {}
    for _, part in ipairs(parts) do
        for _, byte in ipairs(part) do
            result[#result + 1] = byte
        end
    end
    return result
end

return M

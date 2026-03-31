--- coding_adventures.clr_simulator — CLR IL (Intermediate Language) simulator
--
-- # What is the CLR?
--
-- The Common Language Runtime (CLR) is Microsoft's execution engine, introduced
-- in 2002 with .NET Framework. It runs C#, F#, VB.NET, and many other languages.
-- Like the JVM, it is a **stack-based virtual machine** — instructions operate
-- on a stack rather than CPU registers.
--
-- # CLR vs JVM: A Key Difference
--
-- The JVM uses typed opcodes: `iadd` (int add), `ladd` (long add), `fadd`
-- (float add). The CLR uses type-neutral opcodes: just `add` — the runtime
-- infers the type from what is on the stack.
--
--   JVM:  iconst_1 / iconst_2 / iadd     ← type in the opcode name
--   CLR:  ldc.i4.1 / ldc.i4.2 / add      ← type inferred from stack top
--
-- This makes CLR bytecode more compact, but requires type tracking at runtime.
--
-- # Bytecode Encoding
--
-- CLR bytecode is a sequence of variable-width bytes. Most instructions are
-- 1 byte (just the opcode). Some have 1-byte or 4-byte operands. A few
-- instructions (ceq, cgt, clt) use a 2-byte opcode: 0xFE followed by a
-- second byte.
--
-- Example: `x = 1 + 2`
--
--   ldc.i4.1  → 0x17          push int32 constant 1
--   ldc.i4.2  → 0x18          push int32 constant 2
--   add       → 0x58          pop two, push sum
--   stloc.0   → 0x0A          store to local variable 0
--   ret       → 0x2A          return
--
-- # Stack Layout
--
-- The CLR operand stack holds typed values. For this educational simulator,
-- we only implement int32 (and null for ldnull). The stack grows right
-- (new values are appended to the array):
--
--   After ldc.i4.3: stack = [3]
--   After ldc.i4.5: stack = [3, 5]
--   After add:       stack = [8]
--
-- # Local Variables
--
-- Each method has an array of local variable slots. Up to 4 slots have
-- short encodings (stloc.0 – stloc.3, ldloc.0 – ldloc.3). More slots use
-- the "s" variants: stloc.s N and ldloc.s N (1-byte operand).
--
-- # Multi-byte Opcodes
--
-- The compare instructions (ceq, cgt, clt) use a 0xFE prefix byte followed
-- by a second byte that identifies the specific instruction:
--
--   ceq  →  0xFE 0x01   (push 1 if equal, 0 otherwise)
--   cgt  →  0xFE 0x02   (push 1 if a > b, 0 otherwise)
--   clt  →  0xFE 0x04   (push 1 if a < b, 0 otherwise)
--
-- The two-byte encoding is unusual among stack machines and was chosen by
-- Microsoft to leave room for future extensions without wasting single-byte
-- opcodes.
--
-- @module coding_adventures.clr_simulator

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Opcode Constants (real CLR IL hex values — educational accuracy!)
-- ============================================================================

-- No-op and null
M.NOP        = 0x00   -- no operation
M.LDNULL     = 0x01   -- push null

-- Load/store short local variable slots (0-3)
M.LDLOC_0    = 0x06   -- push local[0]
M.LDLOC_1    = 0x07   -- push local[1]
M.LDLOC_2    = 0x08   -- push local[2]
M.LDLOC_3    = 0x09   -- push local[3]
M.STLOC_0    = 0x0A   -- pop → local[0]
M.STLOC_1    = 0x0B   -- pop → local[1]
M.STLOC_2    = 0x0C   -- pop → local[2]
M.STLOC_3    = 0x0D   -- pop → local[3]

-- Load/store with 1-byte slot operand
M.LDLOC_S    = 0x11   -- push local[N]  (operand = slot index)
M.STLOC_S    = 0x13   -- pop → local[N] (operand = slot index)

-- Load integer constants (short forms 0-8)
M.LDC_I4_0   = 0x16   -- push 0
M.LDC_I4_1   = 0x17   -- push 1
M.LDC_I4_2   = 0x18   -- push 2
M.LDC_I4_3   = 0x19   -- push 3
M.LDC_I4_4   = 0x1A   -- push 4
M.LDC_I4_5   = 0x1B   -- push 5
M.LDC_I4_6   = 0x1C   -- push 6
M.LDC_I4_7   = 0x1D   -- push 7
M.LDC_I4_8   = 0x1E   -- push 8
M.LDC_I4_S   = 0x1F   -- push signed int8 (1-byte operand)
M.LDC_I4     = 0x20   -- push int32 (4-byte little-endian operand)

-- Flow control
M.RET        = 0x2A   -- return
M.BR_S       = 0x2B   -- unconditional short branch (1-byte signed offset)
M.BRFALSE_S  = 0x2C   -- branch if top == 0/null (short)
M.BRTRUE_S   = 0x2D   -- branch if top != 0/null (short)

-- Arithmetic (type inferred from stack — no iadd/ladd split like JVM!)
M.ADD        = 0x58   -- pop b, pop a, push a+b
M.SUB        = 0x59   -- pop b, pop a, push a-b
M.MUL        = 0x5A   -- pop b, pop a, push a*b
M.DIV        = 0x5B   -- pop b, pop a, push trunc(a/b)

-- Two-byte prefix for compare instructions
M.PREFIX_FE  = 0xFE   -- first byte of ceq/cgt/clt
M.CEQ_BYTE   = 0x01   -- ceq second byte: push 1 if equal
M.CGT_BYTE   = 0x02   -- cgt second byte: push 1 if a > b
M.CLT_BYTE   = 0x04   -- clt second byte: push 1 if a < b

-- ============================================================================
-- Trace Record
-- ============================================================================

--- Create a trace record capturing one instruction's execution.
-- A trace lets you replay, debug, and inspect every step of execution:
--
--   trace.pc            — program counter before the instruction
--   trace.opcode        — human-readable opcode name (e.g. "ldc.i4.3")
--   trace.stack_before  — stack contents before execution
--   trace.stack_after   — stack contents after execution
--   trace.locals        — local variable array snapshot after execution
--   trace.description   — plain-English description of what happened
--
-- @param pc          integer — the PC at which the instruction executed
-- @param opcode      string  — opcode mnemonic
-- @param before      table   — copy of stack before execution
-- @param sim         table   — simulator state after execution
-- @param description string  — plain-English description
-- @return table (trace record)
local function make_trace(pc, opcode, before, sim, description)
    -- Deep copy locals (use numeric for-loop since ipairs stops at nil slots).
    local locals_copy = setmetatable({}, {__len = function() return #sim.locals end})
    for i = 1, #sim.locals do
        locals_copy[i] = sim.locals[i]
    end
    return {
        pc           = pc,
        opcode       = opcode,
        stack_before = before,
        stack_after  = copy_stack(sim.stack),
        locals       = locals_copy,
        description  = description,
    }
end

-- ============================================================================
-- Simulator Constructor
-- ============================================================================

--- Create a new CLR simulator instance.
--
-- The simulator is a pure value object — there is no mutable global state.
-- Each call to step() returns a NEW simulator with the updated state.
--
-- Fields:
--   sim.stack    — operand stack (array, index 1 = bottom, #stack = top)
--   sim.locals   — local variable array (1-indexed; slot 0 = locals[1])
--   sim.pc       — program counter (0-based byte index into bytecode)
--   sim.bytecode — raw byte string (array of integer 0-255 values)
--   sim.halted   — true after ret executes
--
-- @return table — new simulator instance
function M.new()
    return {
        stack    = new_stack(),
        locals   = {},
        pc       = 0,
        bytecode = {},
        halted   = false,
    }
end

--- Load bytecode and initialize locals.
--
-- @param sim       table   — simulator instance
-- @param bytecode  table   — array of byte integers (0-255)
-- @param opts      table   — optional { num_locals=16 }
-- @return table — updated simulator (new value)
function M.load(sim, bytecode, opts)
    opts = opts or {}
    local num_locals = opts.num_locals or 16
    -- We need a locals table whose # operator returns num_locals even when all
    -- slots are nil. Lua's # operator stops at the first nil in a sequence, so
    -- a table of all-nil slots returns 0. The fix: attach a metatable with a
    -- __len metamethod that always returns the declared slot count.
    local new_locals = setmetatable({}, {__len = function() return num_locals end})
    for i = 1, num_locals do
        new_locals[i] = nil
    end
    return {
        stack    = new_stack(),
        locals   = new_locals,
        pc       = 0,
        bytecode = bytecode,
        halted   = false,
    }
end

-- ============================================================================
-- Byte Reading Helpers
-- ============================================================================

-- Read an unsigned byte at position pos (0-based).
local function byte_at(bytecode, pos)
    return bytecode[pos + 1]  -- Lua arrays are 1-indexed
end

-- Read a signed byte at position pos (0-based).
-- CLR uses signed bytes for short branch offsets and ldc.i4.s operands.
local function signed_byte_at(bytecode, pos)
    local v = bytecode[pos + 1]
    if v >= 128 then return v - 256 end
    return v
end

-- Read a little-endian signed 32-bit integer starting at position pos.
-- CLR encodes ldc.i4 operands in little-endian order (LSB first).
local function little_signed32_at(bytecode, pos)
    local b0 = bytecode[pos + 1]
    local b1 = bytecode[pos + 2]
    local b2 = bytecode[pos + 3]
    local b3 = bytecode[pos + 4]
    local v = b0 + b1 * 256 + b2 * 65536 + b3 * 16777216
    -- Convert to signed 32-bit
    if v >= 2147483648 then v = v - 4294967296 end
    return v
end

-- ============================================================================
-- Stack Helpers
-- ============================================================================
--
-- The CLR stack must support nil as a valid value (via ldnull). Plain Lua
-- tables cannot count nil entries with '#'. We solve this by storing a '_n'
-- field that tracks the logical size, and attaching a __len metamethod so
-- that '#stack' returns '_n' even when some slots hold nil.
--
-- Example: after ldnull, stack[1]==nil but #stack==1.

local STACK_MT = {__len = function(t) return t._n end}

local function new_stack()
    return setmetatable({_n = 0}, STACK_MT)
end

local function copy_stack(s)
    local c = setmetatable({_n = s._n}, STACK_MT)
    for i = 1, s._n do c[i] = s[i] end
    return c
end

-- Pop the top value from the stack; raises on underflow.
local function pop(sim)
    if sim.stack._n == 0 then
        error("Stack underflow")
    end
    local s = copy_stack(sim.stack)
    local val = s[s._n]
    s[s._n] = nil  -- clear slot (optional but tidy)
    s._n = s._n - 1
    sim.stack = s
    return sim, val
end

-- Push a value onto the stack (nil-safe via explicit index).
local function push(sim, val)
    local s = copy_stack(sim.stack)
    s._n = s._n + 1
    s[s._n] = val
    sim.stack = s
    return sim
end

-- ============================================================================
-- Arithmetic Helper
-- ============================================================================

-- Perform a two-operand arithmetic operation.
-- The CLR pops b first, then a (LIFO), then pushes a op b.
local function do_arithmetic(sim, mnemonic, stack_before, op_fn)
    local pc = sim.pc
    sim, b = pop(sim)
    sim, a = pop(sim)
    local result = op_fn(a, b)
    sim = push(sim, result)
    sim.pc = pc + 1
    return sim, make_trace(pc, mnemonic, stack_before, sim,
        string.format("pop %s and %s, push %s", tostring(b), tostring(a), tostring(result)))
end

-- ============================================================================
-- Execute One Instruction
-- ============================================================================

--- Execute one instruction, advancing the PC.
--
-- The CLR step function:
--   1. Read the opcode at sim.pc.
--   2. Decode operands (if any).
--   3. Execute the instruction (modify stack, locals, pc).
--   4. Return updated simulator + trace record.
--
-- @param sim  table — current simulator state
-- @return sim, trace — updated simulator and trace record
function M.step(sim)
    if sim.halted then
        error("CLR simulator has halted")
    end
    if sim.pc >= #sim.bytecode then
        error(string.format("PC (%d) is beyond end of bytecode (%d bytes)", sim.pc, #sim.bytecode))
    end

    local stack_before = copy_stack(sim.stack)
    local opcode = byte_at(sim.bytecode, sim.pc)

    -- ---- nop ----------------------------------------------------------------
    if opcode == M.NOP then
        local pc = sim.pc
        sim.pc = pc + 1
        return sim, make_trace(pc, "nop", stack_before, sim, "no operation")

    -- ---- ldnull -------------------------------------------------------------
    elseif opcode == M.LDNULL then
        local pc = sim.pc
        sim = push(sim, nil)
        sim.pc = pc + 1
        return sim, make_trace(pc, "ldnull", stack_before, sim, "push null")

    -- ---- ldc.i4.0 through ldc.i4.8 -----------------------------------------
    -- The eight short-form integer push opcodes are consecutive in the opcode
    -- table: 0x16 (0) through 0x1E (8). The value is (opcode - 0x16).
    elseif opcode >= M.LDC_I4_0 and opcode <= M.LDC_I4_8 then
        local pc = sim.pc
        local value = opcode - M.LDC_I4_0
        sim = push(sim, value)
        sim.pc = pc + 1
        return sim, make_trace(pc, string.format("ldc.i4.%d", value), stack_before, sim,
            string.format("push %d", value))

    -- ---- ldc.i4.s -----------------------------------------------------------
    -- Push a signed int8 as int32. Useful for small constants outside 0-8.
    elseif opcode == M.LDC_I4_S then
        local pc = sim.pc
        local value = signed_byte_at(sim.bytecode, pc + 1)
        sim = push(sim, value)
        sim.pc = pc + 2
        return sim, make_trace(pc, "ldc.i4.s", stack_before, sim,
            string.format("push %d", value))

    -- ---- ldc.i4 -------------------------------------------------------------
    -- Push a full 32-bit signed integer (4-byte little-endian operand).
    elseif opcode == M.LDC_I4 then
        local pc = sim.pc
        local value = little_signed32_at(sim.bytecode, pc + 1)
        sim = push(sim, value)
        sim.pc = pc + 5
        return sim, make_trace(pc, "ldc.i4", stack_before, sim,
            string.format("push %d", value))

    -- ---- ldloc.0 through ldloc.3 -------------------------------------------
    -- Short-form load from local variable slots 0-3.
    -- Opcodes 0x06-0x09 (ldloc.0-3).
    elseif opcode >= M.LDLOC_0 and opcode <= M.LDLOC_3 then
        local pc = sim.pc
        local slot = opcode - M.LDLOC_0
        local value = sim.locals[slot + 1]  -- Lua 1-indexed
        if value == nil then
            error(string.format("Local variable %d is uninitialized", slot))
        end
        sim = push(sim, value)
        sim.pc = pc + 1
        return sim, make_trace(pc, string.format("ldloc.%d", slot), stack_before, sim,
            string.format("push locals[%d] = %s", slot, tostring(value)))

    -- ---- stloc.0 through stloc.3 -------------------------------------------
    -- Short-form store to local variable slots 0-3.
    -- Opcodes 0x0A-0x0D (stloc.0-3).
    elseif opcode >= M.STLOC_0 and opcode <= M.STLOC_3 then
        local pc = sim.pc
        local slot = opcode - M.STLOC_0
        sim, value = pop(sim)
        sim.locals[slot + 1] = value
        sim.pc = pc + 1
        return sim, make_trace(pc, string.format("stloc.%d", slot), stack_before, sim,
            string.format("pop %s, store in locals[%d]", tostring(value), slot))

    -- ---- ldloc.s ------------------------------------------------------------
    -- Load local variable with 1-byte slot index (for slots 4+).
    elseif opcode == M.LDLOC_S then
        local pc = sim.pc
        local slot = byte_at(sim.bytecode, pc + 1)
        local value = sim.locals[slot + 1]
        if value == nil then
            error(string.format("Local variable %d is uninitialized", slot))
        end
        sim = push(sim, value)
        sim.pc = pc + 2
        return sim, make_trace(pc, "ldloc.s", stack_before, sim,
            string.format("push locals[%d] = %s", slot, tostring(value)))

    -- ---- stloc.s ------------------------------------------------------------
    -- Store to local variable with 1-byte slot index (for slots 4+).
    elseif opcode == M.STLOC_S then
        local pc = sim.pc
        local slot = byte_at(sim.bytecode, pc + 1)
        sim, value = pop(sim)
        sim.locals[slot + 1] = value
        sim.pc = pc + 2
        return sim, make_trace(pc, "stloc.s", stack_before, sim,
            string.format("pop %s, store in locals[%d]", tostring(value), slot))

    -- ---- add / sub / mul ----------------------------------------------------
    elseif opcode == M.ADD then
        return do_arithmetic(sim, "add", stack_before, function(a, b) return a + b end)
    elseif opcode == M.SUB then
        return do_arithmetic(sim, "sub", stack_before, function(a, b) return a - b end)
    elseif opcode == M.MUL then
        return do_arithmetic(sim, "mul", stack_before, function(a, b) return a * b end)

    -- ---- div ----------------------------------------------------------------
    -- CLR integer division truncates toward zero (like C's / operator).
    elseif opcode == M.DIV then
        local pc = sim.pc
        sim, b = pop(sim)
        sim, a = pop(sim)
        if b == 0 then
            error("System.DivideByZeroException: division by zero")
        end
        local result = math.tointeger and math.tointeger(a / b) or math.floor(a / b)
        if result == nil then result = math.floor(a / b) end
        sim = push(sim, result)
        sim.pc = pc + 1
        return sim, make_trace(pc, "div", stack_before, sim,
            string.format("pop %d and %d, push %d", b, a, result))

    -- ---- ret ----------------------------------------------------------------
    -- Return ends the method. The simulator halts.
    elseif opcode == M.RET then
        local pc = sim.pc
        sim.pc = pc + 1
        sim.halted = true
        return sim, make_trace(pc, "ret", stack_before, sim, "return")

    -- ---- br.s ---------------------------------------------------------------
    -- Unconditional short branch. The offset is relative to the instruction
    -- after the br.s (i.e., next_pc = pc + 2, then target = next_pc + offset).
    elseif opcode == M.BR_S then
        local pc = sim.pc
        local offset = signed_byte_at(sim.bytecode, pc + 1)
        local next_pc = pc + 2
        local target = next_pc + offset
        sim.pc = target
        return sim, make_trace(pc, "br.s", stack_before, sim,
            string.format("branch to PC=%d (offset %s%d)",
                target, offset >= 0 and "+" or "", offset))

    -- ---- brfalse.s ----------------------------------------------------------
    -- Branch if top of stack is 0 (false) or null.
    elseif opcode == M.BRFALSE_S then
        local pc = sim.pc
        local offset = signed_byte_at(sim.bytecode, pc + 1)
        local next_pc = pc + 2
        sim, value = pop(sim)
        local numeric = (value == nil) and 0 or value
        local taken = (numeric == 0)
        if taken then
            sim.pc = next_pc + offset
            return sim, make_trace(pc, "brfalse.s", stack_before, sim,
                string.format("pop %s, branch taken to PC=%d", tostring(value), sim.pc))
        else
            sim.pc = next_pc
            return sim, make_trace(pc, "brfalse.s", stack_before, sim,
                string.format("pop %s, branch not taken", tostring(value)))
        end

    -- ---- brtrue.s -----------------------------------------------------------
    -- Branch if top of stack is non-zero (true) and non-null.
    elseif opcode == M.BRTRUE_S then
        local pc = sim.pc
        local offset = signed_byte_at(sim.bytecode, pc + 1)
        local next_pc = pc + 2
        sim, value = pop(sim)
        local numeric = (value == nil) and 0 or value
        local taken = (numeric ~= 0)
        if taken then
            sim.pc = next_pc + offset
            return sim, make_trace(pc, "brtrue.s", stack_before, sim,
                string.format("pop %s, branch taken to PC=%d", tostring(value), sim.pc))
        else
            sim.pc = next_pc
            return sim, make_trace(pc, "brtrue.s", stack_before, sim,
                string.format("pop %s, branch not taken", tostring(value)))
        end

    -- ---- 0xFE prefix: ceq, cgt, clt -----------------------------------------
    -- Two-byte compare instructions. They pop two values and push 1 or 0.
    --
    --   ceq: push 1 if a == b  (equality comparison)
    --   cgt: push 1 if a >  b  (greater-than comparison)
    --   clt: push 1 if a <  b  (less-than comparison)
    --
    -- Note: a and b are popped from the LIFO stack, so b is popped first.
    elseif opcode == M.PREFIX_FE then
        local pc = sim.pc
        if pc + 1 >= #sim.bytecode then
            error(string.format("Incomplete two-byte opcode at PC=%d", pc))
        end
        local second = byte_at(sim.bytecode, pc + 1)
        sim, b = pop(sim)
        sim, a = pop(sim)
        local mnemonic, result, desc
        if second == M.CEQ_BYTE then
            mnemonic = "ceq"
            result = (a == b) and 1 or 0
            desc = string.format("pop %s and %s, push %d (%s == %s)", tostring(b), tostring(a), result, tostring(a), tostring(b))
        elseif second == M.CGT_BYTE then
            mnemonic = "cgt"
            result = (a > b) and 1 or 0
            desc = string.format("pop %s and %s, push %d (%s > %s)", tostring(b), tostring(a), result, tostring(a), tostring(b))
        elseif second == M.CLT_BYTE then
            mnemonic = "clt"
            result = (a < b) and 1 or 0
            desc = string.format("pop %s and %s, push %d (%s < %s)", tostring(b), tostring(a), result, tostring(a), tostring(b))
        else
            error(string.format("Unknown two-byte opcode: 0xFE 0x%02X at PC=%d", second, pc))
        end
        sim = push(sim, result)
        sim.pc = pc + 2
        return sim, make_trace(pc, mnemonic, stack_before, sim, desc)

    else
        error(string.format("Unknown CLR opcode: 0x%02X at PC=%d", opcode, sim.pc))
    end
end

-- ============================================================================
-- Run to Completion
-- ============================================================================

--- Run the simulator until it halts or the step limit is reached.
--
-- @param sim       table — simulator instance with loaded bytecode
-- @param opts      table — optional { max_steps=10000 }
-- @return sim, traces — final simulator state and array of trace records
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
-- Bytecode Assembly Helpers
-- ============================================================================

--- Encode an integer constant as the shortest CLR form.
--
--   0-8:     single-byte ldc.i4.N  (0x16-0x1E)
--   -128..127 (outside 0-8): ldc.i4.s N  (2 bytes)
--   otherwise: ldc.i4 N  (5 bytes, little-endian 32-bit)
--
-- @param n  integer
-- @return table — array of byte integers
function M.encode_ldc_i4(n)
    if n >= 0 and n <= 8 then
        return { M.LDC_I4_0 + n }
    elseif n >= -128 and n <= 127 then
        local b = n < 0 and (n + 256) or n
        return { M.LDC_I4_S, b }
    else
        local v = n
        if v < 0 then v = v + 4294967296 end
        local b0 = v % 256
        local b1 = math.floor(v / 256) % 256
        local b2 = math.floor(v / 65536) % 256
        local b3 = math.floor(v / 16777216) % 256
        return { M.LDC_I4, b0, b1, b2, b3 }
    end
end

--- Encode a stloc instruction for the given slot.
-- @param slot  integer — local variable index (0-based)
-- @return table — array of byte integers
function M.encode_stloc(slot)
    if slot >= 0 and slot <= 3 then
        return { M.STLOC_0 + slot }
    else
        return { M.STLOC_S, slot }
    end
end

--- Encode a ldloc instruction for the given slot.
-- @param slot  integer — local variable index (0-based)
-- @return table — array of byte integers
function M.encode_ldloc(slot)
    if slot >= 0 and slot <= 3 then
        return { M.LDLOC_0 + slot }
    else
        return { M.LDLOC_S, slot }
    end
end

--- Assemble multiple byte arrays (parts) into a single flat byte array.
--
-- This lets you write bytecode like:
--
--   local code = M.assemble({
--     M.encode_ldc_i4(10),
--     M.encode_ldc_i4(20),
--     { M.ADD },
--     M.encode_stloc(0),
--     { M.RET },
--   })
--
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

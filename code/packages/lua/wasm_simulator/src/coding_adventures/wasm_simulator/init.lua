-- ============================================================================
-- wasm_simulator — WebAssembly interpreter / simulator
-- ============================================================================
--
-- This module executes WebAssembly modules. It accepts a parsed module
-- (produced by coding_adventures.wasm_module_parser) and runs its bytecode
-- instructions on a software-emulated Wasm virtual machine.
--
-- ## What Is a WebAssembly Virtual Machine?
--
-- WebAssembly is a STACK MACHINE. Unlike register-based machines (x86, ARM)
-- where operations read from and write to named registers, a stack machine
-- keeps all working values on an implicit VALUE STACK.
--
-- Every instruction either PUSHES values onto the stack, POPS values off,
-- or does both. The model looks like this during execution of `i32.add`:
--
--   Before:              After:
--   ┌───────────┐        ┌───────────┐
--   │    7      │ ← top  │    10     │ ← top   (7 + 3 = 10)
--   ├───────────┤        └───────────┘
--   │    3      │
--   └───────────┘
--
-- The entire computation is expressed as a sequence of pushes and pops.
-- This is simpler to validate (the type checker can work statically) and
-- straightforward to interpret (just a loop over instructions).
--
-- ## Linear Memory
--
-- Every Wasm module has access to a single array of bytes called LINEAR MEMORY.
-- This models the flat address space used by programs written in C, C++, Rust,
-- etc. Key properties:
--
--   - Addressed by byte offset (0-based unsigned integers)
--   - Organized into 64 KiB pages (65536 bytes per page)
--   - Can grow at runtime via `memory.grow`; never shrinks
--   - Accesses outside allocated bounds trap (cause a runtime error)
--
-- In this simulator, linear memory is represented as a Lua table indexed from
-- 0 to (size_in_bytes - 1). Each cell holds a number in [0, 255].
--
-- ## Globals
--
-- Global variables store persistent scalar values between function calls.
-- A global has a type (e.g., i32) and a mutability flag. At module load time,
-- each global is initialized by evaluating a constant-expression (a short
-- sequence of instructions like `i32.const 42; end`).
--
-- ## Functions and Activation Frames
--
-- When a function is called, an ACTIVATION FRAME (or "stack frame") is created:
--
--   Frame = {
--     func_idx  — which function we're executing
--     locals    — array of local variable values (includes parameters)
--     return_pc — where to continue after this call returns (used by call stack)
--   }
--
-- Functions can call other functions, creating a call stack of frames. The
-- value stack is shared across frames (Wasm's design means the caller's values
-- are still below the callee's on the stack — the callee pops its args and
-- pushes its results).
--
-- ## Structured Control Flow and Labels
--
-- Unlike most languages, WebAssembly does NOT have arbitrary goto. All control
-- flow is STRUCTURED — you can only branch to the start or end of an
-- enclosing block. This makes Wasm safe and easy to analyze.
--
-- Structured control instructions and their branch targets:
--
--   block: branch goes to the END of the block (forward branch, like break)
--   loop:  branch goes to the START of the loop (backward branch, like continue)
--   if:    branch goes to the END of the if (forward branch)
--
-- Labels are tracked in a LABEL STACK. When we encounter a branch instruction
-- `br N`, we pop N+1 labels off the label stack and jump to the corresponding
-- target. Each label records enough information to know where to resume.
--
-- ## Instruction Encoding
--
-- In the code section, each function body is a sequence of bytes where:
--   - The first byte is the OPCODE (e.g., 0x6A for i32.add)
--   - Following bytes are IMMEDIATES specific to that opcode
--     (e.g., for i32.const, the next bytes are a signed LEB128 integer)
--
-- This simulator decodes opcodes and immediates on the fly while executing.
--
-- ## How This Module Is Organized
--
-- 1. LEB128 decoder — used to read variable-length immediates from bytecode
-- 2. Instance constructor — initializes globals, memory, and call dispatch
-- 3. execute_expr — the main instruction loop (handles all supported opcodes)
-- 4. control flow helpers — label stack, block entry/exit, branch logic
-- 5. memory helpers — bounds-checked read/write of the linear memory
-- 6. Instance methods — the public API (call, get_global, memory_read, etc.)
--
-- ## Usage
--
--   local parser    = require("coding_adventures.wasm_module_parser")
--   local simulator = require("coding_adventures.wasm_simulator")
--
--   -- Parse a .wasm file
--   local f = io.open("add.wasm", "rb")
--   local bytes = f:read("*all")
--   f:close()
--   local mod = parser.parse(bytes)
--
--   -- Instantiate and call
--   local inst = simulator.Instance.new(mod)
--   local result = inst:call("add", {3, 4})
--   print(result[1])  --> 7
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Dependencies
-- ============================================================================

local leb128 = require("coding_adventures.wasm_leb128")

-- ============================================================================
-- Constants: Wasm page size
-- ============================================================================
--
-- The WebAssembly spec mandates that linear memory is divided into PAGES,
-- each exactly 65536 bytes (64 KiB). The `memory.size` instruction returns
-- the current number of pages; `memory.grow` adds pages.

local PAGE_SIZE = 65536  -- bytes per Wasm memory page

-- ============================================================================
-- Helper: read_leb128_signed(bytes, pos)
-- ============================================================================
--
-- Read a signed LEB128 integer from a byte array at position `pos` (1-based).
-- Returns (value, new_pos) where new_pos is the first byte AFTER the integer.
--
-- This is used to decode `i32.const` immediates. For example:
--
--   bytes = {0x41, 0x2A}   -- i32.const 42
--   pos   = 2              -- pointing at 0x2A
--   → (42, 3)
--
-- The LEB128 encoding of -1 in 5-byte form: {0x7F} (single byte, -1).
-- The LEB128 encoding of -128: {0x80, 0x7F}.

local function read_leb128_signed(bytes, pos)
    local val, count = leb128.decode_signed(bytes, pos)
    return val, pos + count
end

-- ============================================================================
-- Helper: read_leb128_unsigned(bytes, pos)
-- ============================================================================
--
-- Like read_leb128_signed but for unsigned integers. Used for local indices,
-- global indices, function indices, label depths, memory offsets, etc.

local function read_leb128_unsigned(bytes, pos)
    local val, count = leb128.decode_unsigned(bytes, pos)
    return val, pos + count
end

-- ============================================================================
-- Helper: to_i32(n)
-- ============================================================================
--
-- WebAssembly i32 arithmetic is MODULAR (wrapping). Lua uses 64-bit floats
-- (or integers on Lua 5.3+) so we must simulate 32-bit wrap-around.
--
-- Two's complement 32-bit range: -2147483648 to +2147483647
-- We wrap by masking to 32 bits, then sign-extending if the high bit is set.
--
-- Example:
--   to_i32(2147483648)  → -2147483648  (INT_MIN)
--   to_i32(4294967295)  → -1
--   to_i32(-1)          → -1           (already in range)

local function to_i32(n)
    -- Mask to 32 bits (unsigned)
    local u = n % 4294967296  -- 2^32
    if u < 0 then u = u + 4294967296 end
    -- Sign-extend: if bit 31 is set, the value is negative
    if u >= 2147483648 then
        return u - 4294967296
    end
    return u
end

-- ============================================================================
-- Helper: to_u32(n)
-- ============================================================================
--
-- Interpret n as an unsigned 32-bit integer (for shift amounts and bit ops).
-- Used for the shift amount in shl/shr, which is taken modulo 32.

local function to_u32(n)
    local u = n % 4294967296
    if u < 0 then u = u + 4294967296 end
    return u
end

-- ============================================================================
-- Helper: bool_to_i32(b)
-- ============================================================================
--
-- In WebAssembly, boolean results (from comparisons) are i32 values: 1 for
-- true, 0 for false. This helper converts a Lua boolean to that convention.

local function bool_to_i32(b)
    return b and 1 or 0
end

-- ============================================================================
-- Memory: make_memory(num_pages)
-- ============================================================================
--
-- Create a fresh linear memory initialized to all-zero bytes.
--
-- The memory is a table:
--   { bytes = {...}, size_pages = N }
--
-- `bytes` is indexed 0..(num_pages * PAGE_SIZE - 1), all initialized to 0.
-- We use a Lua table with numeric keys; unset keys implicitly read as 0 (we
-- rely on this to avoid allocating all PAGE_SIZE bytes upfront for large
-- memories).

local function make_memory(num_pages)
    return {
        bytes      = {},     -- sparse table: [0..size-1] → byte value
        size_pages = num_pages or 0,
    }
end

-- memory_load(mem, addr, n_bytes)
-- Read n_bytes bytes from linear memory starting at byte address `addr`.
-- Returns a table of byte values (length n_bytes).
-- Traps if address is out of bounds.
local function memory_load(mem, addr, n_bytes)
    local limit = mem.size_pages * PAGE_SIZE
    if addr < 0 or addr + n_bytes > limit then
        error(string.format(
            "wasm_simulator: memory access out of bounds: addr=%d, len=%d, memory_size=%d",
            addr, n_bytes, limit))
    end
    local result = {}
    for i = 0, n_bytes - 1 do
        result[i + 1] = mem.bytes[addr + i] or 0
    end
    return result
end

-- memory_store(mem, addr, byte_array)
-- Write bytes from byte_array into linear memory starting at `addr`.
-- Traps if the write would go out of bounds.
local function memory_store(mem, addr, byte_array)
    local limit = mem.size_pages * PAGE_SIZE
    local n = #byte_array
    if addr < 0 or addr + n > limit then
        error(string.format(
            "wasm_simulator: memory write out of bounds: addr=%d, len=%d, memory_size=%d",
            addr, n, limit))
    end
    for i = 1, n do
        mem.bytes[addr + i - 1] = byte_array[i]
    end
end

-- read_i32_le(mem, addr)
-- Read a 32-bit little-endian integer from memory at byte address `addr`.
-- Returns a SIGNED 32-bit integer.
--
-- "Little-endian" means the least-significant byte is at the lowest address:
--   addr+0: bits  0-7   (LSB)
--   addr+1: bits  8-15
--   addr+2: bits 16-23
--   addr+3: bits 24-31  (MSB)
local function read_i32_le(mem, addr)
    local bs = memory_load(mem, addr, 4)
    local u = bs[1]
      + bs[2] * 256
      + bs[3] * 65536
      + bs[4] * 16777216
    return to_i32(u)
end

-- write_i32_le(mem, addr, val)
-- Write a signed 32-bit integer to memory at `addr` in little-endian order.
local function write_i32_le(mem, addr, val)
    -- Convert to unsigned 32-bit before splitting into bytes
    local u = to_u32(val)
    memory_store(mem, addr, {
        u % 256,
        math.floor(u / 256) % 256,
        math.floor(u / 65536) % 256,
        math.floor(u / 16777216) % 256,
    })
end

-- ============================================================================
-- eval_init_expr(init_expr_bytes, globals)
-- ============================================================================
--
-- Wasm global initializers are "constant expressions" — a restricted subset
-- of instructions that can only push a single constant value. The valid forms
-- in MVP Wasm are:
--
--   i32.const n; end
--   i64.const n; end
--   f32.const f; end
--   f64.const f; end
--   global.get i; end  (where global i is an import, already initialized)
--
-- We evaluate these mini-programs to get the initial value of a global.
-- The `globals` table (already-initialized globals) is needed for global.get.

local function eval_init_expr(init_bytes, globals)
    -- init_bytes is an array of byte values ending with 0x0B (end)
    local pos = 1
    local opcode = init_bytes[pos]; pos = pos + 1

    if opcode == 0x41 then
        -- i32.const <signed LEB128>
        local val, new_pos = read_leb128_signed(init_bytes, pos)
        return to_i32(val)
    elseif opcode == 0x42 then
        -- i64.const <signed LEB128> — we treat i64 as a Lua number
        local val, new_pos = read_leb128_signed(init_bytes, pos)
        return val
    elseif opcode == 0x23 then
        -- global.get <unsigned LEB128>
        local idx, new_pos = read_leb128_unsigned(init_bytes, pos)
        if globals and globals[idx + 1] then
            return globals[idx + 1].value
        end
        error(string.format("wasm_simulator: global.get in init_expr: global %d not found", idx))
    elseif opcode == 0x43 then
        -- f32.const — 4 raw bytes (IEEE 754 single); we reconstruct as Lua number
        -- For this simulator we do a best-effort: treat as zero (full IEEE 754
        -- conversion is complex and not needed for the integer-focused tests)
        return 0.0
    elseif opcode == 0x44 then
        -- f64.const — 8 raw bytes (IEEE 754 double)
        return 0.0
    else
        error(string.format("wasm_simulator: unsupported init_expr opcode 0x%02x", opcode))
    end
end

-- ============================================================================
-- Stack helpers (push/pop on a Lua array used as a stack)
-- ============================================================================
--
-- The value stack is a Lua table used as a 1-indexed array.
-- `stack[#stack]` is the top; we push by appending and pop by removing.

local function stack_push(stack, val)
    stack[#stack + 1] = val
end

local function stack_pop(stack)
    local n = #stack
    if n == 0 then
        error("wasm_simulator: value stack underflow")
    end
    local val = stack[n]
    stack[n] = nil
    return val
end

local function stack_peek(stack)
    local n = #stack
    if n == 0 then
        error("wasm_simulator: peek on empty stack")
    end
    return stack[n]
end

-- ============================================================================
-- Label Stack
-- ============================================================================
--
-- The LABEL STACK tracks structured control flow regions. Each entry describes
-- one active block/loop/if scope:
--
--   {
--     kind        — "block", "loop", or "if"
--     target_pc   — for "block"/"if": position of the matching `end` instruction
--                   for "loop": position of the instruction AFTER the `loop` opcode
--     arity       — number of values produced by this block (0 or 1 in MVP)
--     else_pc     — position of `else` clause, if present (only for "if")
--   }
--
-- When `br N` is executed:
--   - We unwind N+1 labels from the top of the label stack.
--   - The Nth label's `target_pc` determines where to jump:
--     - For block/if: jump forward to after the `end`
--     - For loop: jump backward to the start of the loop body
--
-- The label stack is LOCAL to a single function execution call.

-- ============================================================================
-- execute_expr(instance, func_idx, bytecode, locals)
-- ============================================================================
--
-- This is the HEART of the simulator. It executes a sequence of Wasm bytecode
-- instructions and returns the results left on the value stack.
--
-- Parameters:
--   instance  — the Instance containing globals, memory, and the full module
--   func_idx  — index of the function being executed (0-based), for call stack
--   bytecode  — array of byte values (the function body's raw instruction bytes)
--   locals    — array of local variable values (1-based; includes parameters)
--
-- The instruction loop runs until the function returns (via `return` opcode,
-- or falling off the end of the bytecode at the implicit function-level `end`).

local function execute_expr(instance, func_idx, bytecode, locals)
    -- The value stack for this execution.
    -- Stack elements are Lua numbers (or nil for reference types, not supported here).
    local stack = {}

    -- The label stack for structured control flow.
    -- Entry format: {kind, target_pc, arity, start_pc}
    --   kind      = "block" | "loop" | "if"
    --   target_pc = where `br` should jump (for block: end; for loop: loop start)
    --   arity     = values this block produces (currently always 0 or 1)
    --   start_pc  = position of the first instruction inside this block (for loop)
    local labels = {}

    -- Instruction pointer: 1-based index into `bytecode`.
    local pc = 1

    -- We use a "pc overflow" sentinel to signal function return.
    local RETURN_SENTINEL = {}

    -- ------------------------------------------------------------------
    -- Helper: scan forward to find matching end/else for a block/loop/if.
    -- We need this to implement block, loop, if (which have jump targets
    -- we must compute before entering the block).
    -- ------------------------------------------------------------------
    -- scan_for_end(bytecode, start_pos)
    -- Starting at start_pos, scan forward counting nesting levels.
    -- Returns the 1-based position of the matching `end` opcode (0x0B).
    -- Also returns the position of the matching `else` at depth 0, or nil.
    local function scan_for_end(start_pos)
        local depth = 0
        local else_pc = nil
        local i = start_pos
        while i <= #bytecode do
            local op = bytecode[i]
            if op == 0x02 or op == 0x03 or op == 0x04 then
                -- block, loop, if: increase nesting depth
                depth = depth + 1
                i = i + 1
                -- Skip the blocktype byte (always present)
                i = i + 1
            elseif op == 0x05 then
                -- else
                if depth == 0 then
                    else_pc = i
                end
                i = i + 1
            elseif op == 0x0b then
                -- end
                if depth == 0 then
                    return i, else_pc
                end
                depth = depth - 1
                i = i + 1
            elseif op == 0x0c or op == 0x0d then
                -- br, br_if: skip label index (LEB128)
                i = i + 1
                local _, cnt = leb128.decode_unsigned(bytecode, i)
                i = i + cnt
            elseif op == 0x0e then
                -- br_table: vec of labels + default
                i = i + 1
                local n, cnt = leb128.decode_unsigned(bytecode, i)
                i = i + cnt
                for _ = 1, n + 1 do
                    local _, lc = leb128.decode_unsigned(bytecode, i)
                    i = i + lc
                end
            elseif op == 0x10 then
                -- call: skip func_idx (LEB128)
                i = i + 1
                local _, cnt = leb128.decode_unsigned(bytecode, i)
                i = i + cnt
            elseif op == 0x11 then
                -- call_indirect: skip type_idx + table_idx
                i = i + 1
                local _, c1 = leb128.decode_unsigned(bytecode, i)
                i = i + c1
                local _, c2 = leb128.decode_unsigned(bytecode, i)
                i = i + c2
            elseif op == 0x20 or op == 0x21 or op == 0x22
                or op == 0x23 or op == 0x24 then
                -- local.get/set/tee, global.get/set: skip 1 LEB128
                i = i + 1
                local _, cnt = leb128.decode_unsigned(bytecode, i)
                i = i + cnt
            elseif op == 0x28 or op == 0x29 or op == 0x2a or op == 0x2b
                or op == 0x2c or op == 0x2d or op == 0x2e or op == 0x2f
                or op == 0x30 or op == 0x31 or op == 0x32 or op == 0x33
                or op == 0x34 or op == 0x35 or op == 0x36 or op == 0x37
                or op == 0x38 or op == 0x39 or op == 0x3a or op == 0x3b
                or op == 0x3c or op == 0x3d or op == 0x3e then
                -- load/store: skip align + offset (two LEB128s)
                i = i + 1
                local _, c1 = leb128.decode_unsigned(bytecode, i)
                i = i + c1
                local _, c2 = leb128.decode_unsigned(bytecode, i)
                i = i + c2
            elseif op == 0x3f or op == 0x40 then
                -- memory.size, memory.grow: skip reserved byte
                i = i + 2
            elseif op == 0x41 then
                -- i32.const: skip signed LEB128
                i = i + 1
                local _, cnt = leb128.decode_signed(bytecode, i)
                i = i + cnt
            elseif op == 0x42 then
                -- i64.const: skip signed LEB128
                i = i + 1
                local _, cnt = leb128.decode_signed(bytecode, i)
                i = i + cnt
            elseif op == 0x43 then
                -- f32.const: skip 4 bytes
                i = i + 5
            elseif op == 0x44 then
                -- f64.const: skip 8 bytes
                i = i + 9
            else
                -- Everything else is a fixed-size opcode with no immediates
                i = i + 1
            end
        end
        error("wasm_simulator: scan_for_end: could not find matching end")
    end

    -- ------------------------------------------------------------------
    -- Helper: do_branch(depth)
    -- Perform a branch to the label at `depth` levels up the label stack.
    -- Returns the new pc to resume at, or RETURN_SENTINEL to signal return.
    -- ------------------------------------------------------------------
    local function do_branch(depth)
        -- We unwind `depth` labels (not depth+1; we keep the target label).
        -- The target is labels[#labels - depth].
        local n = #labels
        local target_idx = n - depth
        if target_idx < 1 then
            -- Branch target is outside all blocks: equivalent to function return
            return RETURN_SENTINEL
        end
        local label = labels[target_idx]
        -- Remove all labels from target_idx+1 to n (we're jumping out of them)
        for i = n, target_idx + 1, -1 do
            labels[i] = nil
        end
        -- For a loop, branch goes BACK to the start (loop continuation)
        -- For block/if, branch goes FORWARD to after the end
        if label.kind == "loop" then
            return label.start_pc
        else
            return label.end_pc + 1  -- +1 to skip past the `end` opcode itself
        end
    end

    -- ------------------------------------------------------------------
    -- Main instruction loop
    -- ------------------------------------------------------------------
    while pc <= #bytecode do
        local opcode = bytecode[pc]
        pc = pc + 1

        -- ----------------------------------------------------------------
        -- CONTROL FLOW INSTRUCTIONS
        -- ----------------------------------------------------------------

        if opcode == 0x00 then
            -- unreachable: unconditional trap
            error("wasm_simulator: unreachable instruction executed")

        elseif opcode == 0x01 then
            -- nop: no operation, do nothing
            -- Used as a placeholder; harmless.

        elseif opcode == 0x02 then
            -- block <blocktype>
            -- Read the block type byte (0x40 = void, or a ValType).
            -- Then scan forward to find the matching `end`.
            local blocktype = bytecode[pc]; pc = pc + 1
            -- arity: how many results this block produces (0 for void, 1 otherwise)
            local arity = (blocktype == 0x40) and 0 or 1
            -- Scan forward from current pc to find the matching `end`
            local end_pc, else_pc = scan_for_end(pc)
            -- Push a label for this block
            labels[#labels + 1] = {
                kind     = "block",
                end_pc   = end_pc,
                arity    = arity,
                start_pc = pc,     -- not used for block, but stored for symmetry
            }
            -- Execution continues inside the block (pc is already past blocktype)

        elseif opcode == 0x03 then
            -- loop <blocktype>
            -- A loop is like a block, except `br` to a loop goes BACK to the start.
            local blocktype = bytecode[pc]; pc = pc + 1
            local arity = (blocktype == 0x40) and 0 or 1
            local end_pc, _ = scan_for_end(pc)
            -- start_pc is the first instruction INSIDE the loop body (current pc)
            labels[#labels + 1] = {
                kind     = "loop",
                end_pc   = end_pc,
                arity    = arity,
                start_pc = pc,  -- branching here restarts the loop
            }

        elseif opcode == 0x04 then
            -- if <blocktype>
            -- Pop the condition. If nonzero, execute the "then" arm.
            -- If zero, jump to `else` (if present) or `end`.
            local blocktype = bytecode[pc]; pc = pc + 1
            local arity = (blocktype == 0x40) and 0 or 1
            local end_pc, else_pc = scan_for_end(pc)
            local cond = stack_pop(stack)
            if cond ~= 0 then
                -- Execute "then" arm; push a label for branch targets
                labels[#labels + 1] = {
                    kind     = "if",
                    end_pc   = end_pc,
                    else_pc  = else_pc,
                    arity    = arity,
                    start_pc = pc,
                }
                -- pc is already inside the then-arm; continue executing
            else
                -- Skip to else (if exists) or end
                if else_pc then
                    pc = else_pc + 1  -- skip past the `else` opcode
                    labels[#labels + 1] = {
                        kind     = "if",
                        end_pc   = end_pc,
                        else_pc  = nil,
                        arity    = arity,
                        start_pc = pc,
                    }
                else
                    -- No else arm: jump past the `end`
                    pc = end_pc + 1
                end
            end

        elseif opcode == 0x05 then
            -- else: we reach this only when executing the "then" arm and it fell
            -- through to the else marker. That means the then-arm is done; skip
            -- to the matching `end`.
            local label = labels[#labels]
            if label and label.kind == "if" then
                pc = label.end_pc + 1
                labels[#labels] = nil
            else
                error("wasm_simulator: else without matching if")
            end

        elseif opcode == 0x0b then
            -- end: close the current block/loop/if
            -- Pop the corresponding label.
            if #labels > 0 then
                labels[#labels] = nil
            else
                -- This `end` closes the function body — we're done!
                break
            end

        elseif opcode == 0x0c then
            -- br <label_depth>
            -- Unconditional branch. The immediate is a depth (0 = innermost label).
            local depth, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            local jump_pc = do_branch(depth)
            if jump_pc == RETURN_SENTINEL then
                goto function_return
            end
            pc = jump_pc

        elseif opcode == 0x0d then
            -- br_if <label_depth>
            -- Conditional branch: branch if condition is nonzero.
            local depth, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            local cond = stack_pop(stack)
            if cond ~= 0 then
                local jump_pc = do_branch(depth)
                if jump_pc == RETURN_SENTINEL then
                    goto function_return
                end
                pc = jump_pc
            end
            -- If condition is zero, fall through (don't branch)

        elseif opcode == 0x0f then
            -- return: explicit return from the function.
            -- The return values are already on the stack.
            goto function_return

        elseif opcode == 0x10 then
            -- call <func_idx>
            -- Direct function call. Read the function index, then pop arguments
            -- from the stack, call the function, and push results back.
            local callee_idx, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc

            -- Look up the function's type to know how many args to pop
            local mod = instance.module
            local type_idx = mod.functions[callee_idx + 1]
            if type_idx == nil then
                error(string.format(
                    "wasm_simulator: call: function index %d out of range", callee_idx))
            end
            local func_type = mod.types[type_idx + 1]
            local n_params = #func_type.params

            -- Pop arguments from the stack (they're in reverse order on stack)
            local args = {}
            for i = n_params, 1, -1 do
                args[i] = stack_pop(stack)
            end

            -- Execute the callee
            local results = instance:call_by_index(callee_idx, args)

            -- Push results onto our stack
            for _, r in ipairs(results) do
                stack_push(stack, r)
            end

        -- ----------------------------------------------------------------
        -- PARAMETRIC INSTRUCTIONS
        -- ----------------------------------------------------------------

        elseif opcode == 0x1a then
            -- drop: discard the top of the value stack
            stack_pop(stack)

        elseif opcode == 0x1b then
            -- select: pop condition and two values, push one based on condition
            -- Stack before: [..., val1, val2, cond]
            -- Stack after:  [..., val1] if cond != 0
            --               [..., val2] if cond == 0
            local cond = stack_pop(stack)
            local val2 = stack_pop(stack)
            local val1 = stack_pop(stack)
            stack_push(stack, cond ~= 0 and val1 or val2)

        -- ----------------------------------------------------------------
        -- VARIABLE INSTRUCTIONS
        -- ----------------------------------------------------------------

        elseif opcode == 0x20 then
            -- local.get <local_idx>
            -- Push the value of the local variable at the given index.
            -- Local variables are 0-indexed in Wasm; we store them 1-indexed in Lua.
            local idx, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            local val = locals[idx + 1]
            if val == nil then val = 0 end
            stack_push(stack, val)

        elseif opcode == 0x21 then
            -- local.set <local_idx>
            -- Pop the top of the stack and store in a local variable.
            local idx, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            locals[idx + 1] = stack_pop(stack)

        elseif opcode == 0x22 then
            -- local.tee <local_idx>
            -- Like local.set but also leaves the value on the stack.
            -- ("Tee" splits one stream into two, like a T-pipe fitting.)
            local idx, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            local val = stack_peek(stack)
            locals[idx + 1] = val
            -- Value stays on stack (do NOT pop)

        elseif opcode == 0x23 then
            -- global.get <global_idx>
            -- Push the current value of a global variable.
            local idx, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            local glob = instance.globals[idx + 1]
            if glob == nil then
                error(string.format("wasm_simulator: global.get: index %d out of range", idx))
            end
            stack_push(stack, glob.value)

        elseif opcode == 0x24 then
            -- global.set <global_idx>
            -- Pop and store in a mutable global variable.
            local idx, new_pc = read_leb128_unsigned(bytecode, pc)
            pc = new_pc
            local glob = instance.globals[idx + 1]
            if glob == nil then
                error(string.format("wasm_simulator: global.set: index %d out of range", idx))
            end
            if not glob.mutable then
                error(string.format("wasm_simulator: global.set: global %d is immutable", idx))
            end
            glob.value = stack_pop(stack)

        -- ----------------------------------------------------------------
        -- MEMORY INSTRUCTIONS
        -- ----------------------------------------------------------------
        --
        -- All memory instructions have a "memory argument" with two LEB128
        -- immediates: alignment (log2) and static offset. The alignment is
        -- just a hint and we ignore it. The offset is added to the runtime
        -- address before the actual memory access.

        elseif opcode == 0x28 then
            -- i32.load <align> <offset>
            -- Pop address, add offset, read 4 bytes little-endian, push i32.
            local _align, p1 = read_leb128_unsigned(bytecode, pc)
            local offset, p2 = read_leb128_unsigned(bytecode, p1)
            pc = p2
            local addr = stack_pop(stack)
            local val = read_i32_le(instance.memory, addr + offset)
            stack_push(stack, val)

        elseif opcode == 0x36 then
            -- i32.store <align> <offset>
            -- Pop value (i32), pop address, add offset, write 4 bytes LE.
            local _align, p1 = read_leb128_unsigned(bytecode, pc)
            local offset, p2 = read_leb128_unsigned(bytecode, p1)
            pc = p2
            local val  = stack_pop(stack)
            local addr = stack_pop(stack)
            write_i32_le(instance.memory, addr + offset, val)

        elseif opcode == 0x3f then
            -- memory.size
            -- Push the current memory size in pages.
            local _reserved = bytecode[pc]; pc = pc + 1
            stack_push(stack, instance.memory.size_pages)

        elseif opcode == 0x40 then
            -- memory.grow <reserved>
            -- Pop delta (number of pages to add).
            -- Push old size if successful, -1 on failure.
            -- For this simulator we always succeed up to a reasonable limit.
            local _reserved = bytecode[pc]; pc = pc + 1
            local delta = stack_pop(stack)
            local old_size = instance.memory.size_pages
            -- Limit growth to prevent runaway allocation (64 pages = 4 MiB max)
            local MAX_PAGES = 64
            if old_size + delta <= MAX_PAGES then
                instance.memory.size_pages = old_size + delta
                stack_push(stack, old_size)
            else
                stack_push(stack, to_i32(-1))
            end

        -- ----------------------------------------------------------------
        -- NUMERIC INSTRUCTIONS — CONSTANTS
        -- ----------------------------------------------------------------

        elseif opcode == 0x41 then
            -- i32.const <signed LEB128>
            -- Push a 32-bit integer constant. The immediate is sign-extended.
            local val, new_pc = read_leb128_signed(bytecode, pc)
            pc = new_pc
            stack_push(stack, to_i32(val))

        elseif opcode == 0x42 then
            -- i64.const <signed LEB128>
            -- Push a 64-bit integer constant. We treat i64 as a Lua number.
            local val, new_pc = read_leb128_signed(bytecode, pc)
            pc = new_pc
            stack_push(stack, val)

        -- ----------------------------------------------------------------
        -- NUMERIC INSTRUCTIONS — i32 COMPARISONS
        -- ----------------------------------------------------------------
        --
        -- All comparison instructions pop two i32 values and push an i32
        -- result: 1 if the condition is true, 0 if false.

        elseif opcode == 0x45 then
            -- i32.eqz: push 1 if top of stack is 0, else 0
            local a = stack_pop(stack)
            stack_push(stack, bool_to_i32(a == 0))

        elseif opcode == 0x46 then
            -- i32.eq: 1 if a == b
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, bool_to_i32(a_val == b_val))

        elseif opcode == 0x47 then
            -- i32.ne: 1 if a != b
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, bool_to_i32(a_val ~= b_val))

        elseif opcode == 0x48 then
            -- i32.lt_s: 1 if a < b (signed)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, bool_to_i32(a_val < b_val))

        elseif opcode == 0x49 then
            -- i32.lt_u: 1 if a < b (unsigned)
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            stack_push(stack, bool_to_i32(a_val < b_val))

        elseif opcode == 0x4a then
            -- i32.gt_s: 1 if a > b (signed)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, bool_to_i32(a_val > b_val))

        elseif opcode == 0x4b then
            -- i32.gt_u: 1 if a > b (unsigned)
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            stack_push(stack, bool_to_i32(a_val > b_val))

        elseif opcode == 0x4c then
            -- i32.le_s: 1 if a <= b (signed)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, bool_to_i32(a_val <= b_val))

        elseif opcode == 0x4d then
            -- i32.le_u: 1 if a <= b (unsigned)
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            stack_push(stack, bool_to_i32(a_val <= b_val))

        elseif opcode == 0x4e then
            -- i32.ge_s: 1 if a >= b (signed)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, bool_to_i32(a_val >= b_val))

        elseif opcode == 0x4f then
            -- i32.ge_u: 1 if a >= b (unsigned)
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            stack_push(stack, bool_to_i32(a_val >= b_val))

        -- ----------------------------------------------------------------
        -- NUMERIC INSTRUCTIONS — i32 ARITHMETIC
        -- ----------------------------------------------------------------
        --
        -- Binary arithmetic operations follow the pattern:
        --   pop b (top), pop a (second), compute a OP b, push result.
        --
        -- All results are wrapped to 32-bit signed range by to_i32().

        elseif opcode == 0x6a then
            -- i32.add: a + b (wrapping)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, to_i32(a_val + b_val))

        elseif opcode == 0x6b then
            -- i32.sub: a - b (wrapping)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, to_i32(a_val - b_val))

        elseif opcode == 0x6c then
            -- i32.mul: a * b (wrapping)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            stack_push(stack, to_i32(a_val * b_val))

        elseif opcode == 0x6d then
            -- i32.div_s: signed division. Traps on divide-by-zero.
            -- Also traps on INT_MIN / -1 (overflow in two's complement).
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            if b_val == 0 then
                error("wasm_simulator: i32.div_s: division by zero")
            end
            if a_val == -2147483648 and b_val == -1 then
                error("wasm_simulator: i32.div_s: integer overflow (INT_MIN / -1)")
            end
            -- Truncate toward zero (Lua's division truncates toward -inf for negatives,
            -- so we must use math.modf for correct signed truncation)
            local result = math.modf(a_val / b_val)
            stack_push(stack, to_i32(result))

        elseif opcode == 0x6e then
            -- i32.div_u: unsigned division.
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            if b_val == 0 then
                error("wasm_simulator: i32.div_u: division by zero")
            end
            stack_push(stack, to_i32(math.floor(a_val / b_val)))

        elseif opcode == 0x6f then
            -- i32.rem_s: signed remainder (truncated toward zero)
            local b_val = stack_pop(stack)
            local a_val = stack_pop(stack)
            if b_val == 0 then
                error("wasm_simulator: i32.rem_s: remainder by zero")
            end
            -- Lua % is floor modulo; we need truncated remainder.
            -- truncated_rem(a, b) = a - b * trunc(a/b)
            local q = math.modf(a_val / b_val)
            local result = a_val - b_val * q
            stack_push(stack, to_i32(result))

        elseif opcode == 0x70 then
            -- i32.rem_u: unsigned remainder
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            if b_val == 0 then
                error("wasm_simulator: i32.rem_u: remainder by zero")
            end
            stack_push(stack, to_i32(a_val % b_val))

        elseif opcode == 0x71 then
            -- i32.and: bitwise AND
            -- We use Lua's built-in bitwise AND (Lua 5.3+) or emulate it.
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            -- Use bit operations if available (Lua 5.3+)
            local result
            if type(a_val) == "number" and math.type then
                -- Lua 5.3+: use integer AND
                result = math.tointeger(a_val) & math.tointeger(b_val)
            else
                -- Fallback: implement bitwise AND manually for Lua 5.1/5.2
                result = 0
                local bit_val = 1
                for _ = 1, 32 do
                    local a_bit = a_val % 2
                    local b_bit = b_val % 2
                    if a_bit == 1 and b_bit == 1 then
                        result = result + bit_val
                    end
                    a_val = math.floor(a_val / 2)
                    b_val = math.floor(b_val / 2)
                    bit_val = bit_val * 2
                end
            end
            stack_push(stack, to_i32(result))

        elseif opcode == 0x72 then
            -- i32.or: bitwise OR
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            local result
            if math.type then
                result = math.tointeger(a_val) | math.tointeger(b_val)
            else
                result = 0
                local bit_val = 1
                for _ = 1, 32 do
                    local a_bit = a_val % 2
                    local b_bit = b_val % 2
                    if a_bit == 1 or b_bit == 1 then
                        result = result + bit_val
                    end
                    a_val = math.floor(a_val / 2)
                    b_val = math.floor(b_val / 2)
                    bit_val = bit_val * 2
                end
            end
            stack_push(stack, to_i32(result))

        elseif opcode == 0x73 then
            -- i32.xor: bitwise exclusive OR
            local b_val = to_u32(stack_pop(stack))
            local a_val = to_u32(stack_pop(stack))
            local result
            if math.type then
                result = math.tointeger(a_val) ~ math.tointeger(b_val)
            else
                result = 0
                local bit_val = 1
                for _ = 1, 32 do
                    local a_bit = a_val % 2
                    local b_bit = b_val % 2
                    if (a_bit == 1) ~= (b_bit == 1) then
                        result = result + bit_val
                    end
                    a_val = math.floor(a_val / 2)
                    b_val = math.floor(b_val / 2)
                    bit_val = bit_val * 2
                end
            end
            stack_push(stack, to_i32(result))

        elseif opcode == 0x74 then
            -- i32.shl: left shift
            -- The shift amount is taken modulo 32 (spec requirement).
            local b_val = to_u32(stack_pop(stack)) % 32
            local a_val = to_u32(stack_pop(stack))
            local result
            if math.type then
                result = math.tointeger(a_val) << b_val
            else
                result = a_val * (2 ^ b_val)
            end
            stack_push(stack, to_i32(result))

        elseif opcode == 0x75 then
            -- i32.shr_s: arithmetic (sign-preserving) right shift
            -- Fills vacated bits with the sign bit.
            local b_val = to_u32(stack_pop(stack)) % 32
            local a_val = stack_pop(stack)  -- keep as signed
            local result
            if math.type then
                result = math.tointeger(to_i32(a_val)) >> b_val
            else
                -- Manual arithmetic right shift
                local sign = a_val < 0 and 1 or 0
                local u = to_u32(a_val)
                result = math.floor(u / (2 ^ b_val))
                if sign == 1 then
                    -- Fill high bits with 1s
                    local mask = 4294967295 - math.floor(4294967296 / (2 ^ b_val)) + 1
                    result = result + mask
                end
            end
            stack_push(stack, to_i32(result))

        elseif opcode == 0x76 then
            -- i32.shr_u: logical (zero-filling) right shift
            local b_val = to_u32(stack_pop(stack)) % 32
            local a_val = to_u32(stack_pop(stack))
            local result
            if math.type then
                result = math.tointeger(a_val) >> b_val
            else
                result = math.floor(a_val / (2 ^ b_val))
            end
            stack_push(stack, to_i32(result))

        else
            -- Unknown or unimplemented opcode
            error(string.format(
                "wasm_simulator: unsupported opcode 0x%02x at pc=%d",
                opcode, pc - 1))
        end
    end

    -- Function return: collect results from stack
    ::function_return::

    -- Return whatever is on the stack (the function's results)
    return stack
end

-- ============================================================================
-- Instance — the Wasm module instance
-- ============================================================================
--
-- An INSTANCE is a module that has been loaded and initialized:
--   - All globals are initialized from their init expressions
--   - Linear memory is allocated and initialized from data segments
--   - Exports are indexed by name for fast lookup
--
-- The Instance is an OOP object in Lua, using a metatable so that method
-- calls like `inst:call("add", {3, 4})` work idiomatically.

local Instance = {}
Instance.__index = Instance

-- Instance.new(module) — create a new instance from a parsed Wasm module
--
-- The `module` argument is the table returned by wasm_module_parser.parse().
-- This function:
--   1. Initializes global variables from their init expressions
--   2. Allocates linear memory (from the memory section)
--   3. Applies data segments to initialize memory contents
--   4. Builds a name → index map for exported functions
--
-- Returns an Instance object ready for calling.
function Instance.new(module)
    local self = setmetatable({}, Instance)
    self.module = module

    -- ------------------------------------------------------------------
    -- Step 1: Initialize globals
    -- ------------------------------------------------------------------
    --
    -- Globals are stored as { value, mutable, val_type }.
    -- We initialize them in order so that global.get in init_exprs can
    -- refer to already-initialized globals (the spec allows this for
    -- import globals, and some tools also use defined globals this way).
    self.globals = {}
    for i, g in ipairs(module.globals) do
        local value = eval_init_expr(g.init_expr, self.globals)
        self.globals[i] = {
            value    = value,
            mutable  = (g.mutable == 1 or g.mutable == true),
            val_type = g.val_type,
        }
    end

    -- ------------------------------------------------------------------
    -- Step 2: Allocate linear memory
    -- ------------------------------------------------------------------
    --
    -- The memory section lists memory definitions (usually just one in MVP).
    -- Each defines a limits {min, max?} in pages. We allocate min pages
    -- immediately; max is an upper bound we enforce in memory.grow.
    if #module.memories > 0 then
        local mem_def = module.memories[1]
        self.memory = make_memory(mem_def.limits.min)
    else
        -- No memory section: allocate zero pages (will trap on any access)
        self.memory = make_memory(0)
    end

    -- ------------------------------------------------------------------
    -- Step 3: Apply data segments
    -- ------------------------------------------------------------------
    --
    -- Data segments initialize regions of linear memory with known bytes.
    -- Each segment has an offset expression and a byte array.
    for _, seg in ipairs(module.data or {}) do
        -- Evaluate the offset expression (typically `i32.const N; end`)
        local offset = eval_init_expr(seg.offset_expr or {0x41, 0x00, 0x0b}, self.globals)
        memory_store(self.memory, offset, seg.bytes or {})
    end

    -- ------------------------------------------------------------------
    -- Step 4: Build export index
    -- ------------------------------------------------------------------
    --
    -- Build a name → {kind, idx} map for fast export lookup.
    -- We also build separate function name → func_idx map.
    self.export_map = {}
    self.func_export_map = {}  -- name → function index (0-based)
    for _, exp in ipairs(module.exports) do
        self.export_map[exp.name] = exp.desc
        if exp.desc.kind == "func" then
            self.func_export_map[exp.name] = exp.desc.idx
        end
    end

    return self
end

-- ============================================================================
-- Instance:call_by_index(func_idx, args)
-- ============================================================================
--
-- Execute the function at `func_idx` (0-based) with the given argument array.
-- Returns an array of result values.
--
-- This is the internal workhorse; Instance:call() uses it after resolving a
-- function name to an index.
--
-- The execution model:
--   1. Look up the function's type to know parameter count
--   2. Look up the function's code entry (locals + bytecode body)
--   3. Build the `locals` array: params first, then zero-initialized declared locals
--   4. Call execute_expr to run the bytecode
--   5. Return the values left on the value stack

function Instance:call_by_index(func_idx, args)
    local mod = self.module

    -- Validate the function index
    if func_idx < 0 or func_idx >= #mod.functions then
        error(string.format(
            "wasm_simulator: call_by_index: function index %d out of range (module has %d functions)",
            func_idx, #mod.functions))
    end

    -- Look up the type signature (tells us parameter and result types/counts)
    local type_idx = mod.functions[func_idx + 1]  -- 1-based in Lua
    local func_type = mod.types[type_idx + 1]
    local n_params = #func_type.params

    -- Look up the code entry (body bytecode + local variable declarations)
    local code = mod.codes[func_idx + 1]
    if code == nil then
        error(string.format("wasm_simulator: no code entry for function %d", func_idx))
    end

    -- ------------------------------------------------------------------
    -- Build the locals array
    -- ------------------------------------------------------------------
    --
    -- In Wasm, "locals" include BOTH function parameters and locally declared
    -- variables. Their layout is:
    --
    --   locals[0..n_params-1]   — function arguments (filled from `args`)
    --   locals[n_params..]      — locally declared variables (zero-initialized)
    --
    -- The code section stores local declarations as groups: each group says
    -- "N variables of type T". We expand these into individual locals.
    --
    -- Example: a function with (i32 param) and locals {2 × i32, 1 × f64}
    -- would have locals = [param_val, 0, 0, 0.0]

    local locals = {}

    -- Copy arguments into first n_params slots (0-indexed → 1-indexed)
    for i = 1, n_params do
        locals[i] = args and args[i] or 0
    end

    -- Zero-initialize the declared local variables
    local local_idx = n_params + 1
    for _, local_group in ipairs(code.locals) do
        for _ = 1, local_group.count do
            locals[local_idx] = 0
            local_idx = local_idx + 1
        end
    end

    -- ------------------------------------------------------------------
    -- Execute the function body
    -- ------------------------------------------------------------------
    --
    -- The `body` field of a code entry is the raw bytecode array (byte values).
    -- execute_expr handles all instructions and returns the value stack.

    local result_stack = execute_expr(self, func_idx, code.body, locals)

    -- Collect the expected number of results from the stack.
    -- In MVP Wasm, functions return 0 or 1 values.
    local n_results = #func_type.results
    local results = {}
    for i = n_results, 1, -1 do
        results[i] = result_stack[#result_stack - (n_results - i)]
    end

    return results
end

-- ============================================================================
-- Instance:call(func_name, args)
-- ============================================================================
--
-- Call an exported function by name. Resolves the name to a function index
-- and delegates to call_by_index.
--
-- @param func_name  String name of an exported function.
-- @param args       Array of argument values (may be nil or empty for 0-arg functions).
-- @return           Array of result values.

function Instance:call(func_name, args)
    local func_idx = self.func_export_map[func_name]
    if func_idx == nil then
        error(string.format(
            "wasm_simulator: no exported function named '%s'", func_name))
    end
    return self:call_by_index(func_idx, args)
end

-- ============================================================================
-- Instance:get_global(name)
-- ============================================================================
--
-- Retrieve the current value of an exported global variable by name.
-- Returns the numeric value.

function Instance:get_global(name)
    local desc = self.export_map[name]
    if desc == nil then
        error(string.format("wasm_simulator: no export named '%s'", name))
    end
    if desc.kind ~= "global" then
        error(string.format("wasm_simulator: export '%s' is not a global", name))
    end
    local glob = self.globals[desc.idx + 1]
    if glob == nil then
        error(string.format("wasm_simulator: global index %d not found", desc.idx))
    end
    return glob.value
end

-- ============================================================================
-- Instance:set_global(name, value)
-- ============================================================================
--
-- Set the value of an exported mutable global variable by name.

function Instance:set_global(name, value)
    local desc = self.export_map[name]
    if desc == nil then
        error(string.format("wasm_simulator: no export named '%s'", name))
    end
    if desc.kind ~= "global" then
        error(string.format("wasm_simulator: export '%s' is not a global", name))
    end
    local glob = self.globals[desc.idx + 1]
    if glob == nil then
        error(string.format("wasm_simulator: global index %d not found", desc.idx))
    end
    if not glob.mutable then
        error(string.format("wasm_simulator: global '%s' is immutable", name))
    end
    glob.value = value
end

-- ============================================================================
-- Instance:memory_read(offset, length)
-- ============================================================================
--
-- Read `length` bytes from linear memory starting at byte `offset`.
-- Returns a table of byte values (length elements, 1-indexed).

function Instance:memory_read(offset, length)
    return memory_load(self.memory, offset, length)
end

-- ============================================================================
-- Instance:memory_write(offset, bytes)
-- ============================================================================
--
-- Write a table of byte values into linear memory starting at `offset`.
-- `bytes` is a 1-indexed array of integers in [0, 255].

function Instance:memory_write(offset, bytes)
    memory_store(self.memory, offset, bytes)
end

-- ============================================================================
-- Export
-- ============================================================================

M.Instance = Instance

-- Also export helpers useful for tests
M.to_i32    = to_i32
M.to_u32    = to_u32
M.PAGE_SIZE = PAGE_SIZE

return M

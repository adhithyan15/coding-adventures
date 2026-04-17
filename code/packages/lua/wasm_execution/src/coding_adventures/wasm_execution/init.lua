-- wasm_execution -- WebAssembly 1.0 execution engine
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
--
-- ============================================================================
-- ARCHITECTURE
-- ============================================================================
--
-- The WASM execution engine interprets validated WASM modules using the
-- GenericVM from the virtual_machine package. The flow for calling a function:
--
--   WasmExecutionEngine:call_function(func_index, args)
--     1. Look up the function body
--     2. Decode bytecodes into Instruction tables
--     3. Build the control flow map
--     4. Initialize locals (args + zero-initialized declared locals)
--     5. Create the WasmExecutionContext
--     6. Run GenericVM:execute_with_context(code, context)
--     7. Collect return values from the typed stack
--
-- ============================================================================
-- LUA 5.3+ NOTES
-- ============================================================================
--
-- Lua 5.3 introduced native 64-bit integers alongside IEEE 754 doubles.
-- We use these features extensively:
--
--   - i32 masking: val & 0xFFFFFFFF then sign-extend via signed interpretation
--   - i64: native Lua integers (64-bit signed)
--   - f32: string.pack("f", v) / string.unpack("f", ...) for IEEE 754 single
--   - f64: Lua numbers (IEEE 754 doubles)
--   - Bitwise operators: &, |, ~, <<, >>, ~ (unary NOT)
--
-- ============================================================================
-- Usage
-- ============================================================================
--
--   local we = require("coding_adventures.wasm_execution")
--
--   local engine = we.WasmExecutionEngine.new({
--     memory = nil,
--     tables = {},
--     globals = {},
--     global_types = {},
--     func_types = { {params = {0x7F}, results = {0x7F}} },
--     func_bodies = { {locals = {}, body = {...}} },
--     host_functions = {nil},
--   })
--
--   local results = engine:call_function(1, { we.i32(5) })
--   -- results[1].value == 25 for a square function
--
-- ============================================================================

local wasm_leb128 = require("coding_adventures.wasm_leb128")
local wasm_types = require("coding_adventures.wasm_types")
local wasm_opcodes = require("coding_adventures.wasm_opcodes")
local virtual_machine = require("coding_adventures.virtual_machine")

local M = {}

M.VERSION = "0.1.0"


-- ============================================================================
-- VALUE TYPE CONSTANTS
-- ============================================================================
--
-- WebAssembly value types, mirroring the binary encoding from wasm_types.
-- These constants are used throughout the execution engine for type tagging.

local I32 = 0x7F
local I64 = 0x7E
local F32 = 0x7D
local F64 = 0x7C

M.I32 = I32
M.I64 = I64
M.F32 = F32
M.F64 = F64


-- ============================================================================
-- TRAP ERROR
-- ============================================================================
--
-- In WASM, a "trap" is an unrecoverable runtime error. Traps occur on:
--   - Out-of-bounds memory access
--   - Division by zero (integer only)
--   - Unreachable instruction executed
--   - Type mismatch in call_indirect
--   - Stack overflow
--
-- We model traps by calling error() with a string prefixed by "TrapError: ".
-- Callers can catch this with pcall/xpcall and check the prefix.

local function trap(msg)
    error("TrapError: " .. msg, 2)
end

M.trap = trap


-- ============================================================================
-- WASM VALUE CONSTRUCTORS
-- ============================================================================
--
-- Every value in WebAssembly is typed. A WasmValue is a table with two fields:
--   type  -- one of I32, I64, F32, F64
--   value -- the numeric payload
--
-- Constructor functions enforce wrapping semantics:
--   i32: wraps to signed 32-bit range [-2^31, 2^31-1]
--   i64: Lua integers are already 64-bit signed
--   f32: rounds to IEEE 754 single precision via string.pack/unpack
--   f64: Lua numbers are already IEEE 754 doubles

--- Wrap a number to signed 32-bit integer range.
--
-- Lua 5.3+ integers are 64-bit, so we must mask to 32 bits and then
-- sign-extend. The technique:
--   1. Mask to low 32 bits: val & 0xFFFFFFFF
--   2. If bit 31 is set, subtract 2^32 to get the negative value.
--
-- Truth table:
--   Input              | & 0xFFFFFFFF  | Sign-extended
--   -------------------|---------------|---------------
--   42                 | 42            | 42
--   -1                 | 0xFFFFFFFF    | -1
--   0x80000000         | 0x80000000    | -2147483648
--   0x100000000 (2^32) | 0             | 0
--   3.7                | 3             | 3
--
local function to_i32(val)
    -- Convert to integer first (truncate float), then mask to 32 bits.
    val = math.tointeger(val) or math.floor(val)
    val = val & 0xFFFFFFFF
    -- Sign-extend: if bit 31 is set, the value is negative in i32.
    if val >= 0x80000000 then
        val = val - 0x100000000
    end
    return val
end

--- Create an i32 WasmValue.
-- @param value number  The integer value (will be wrapped to 32 bits).
-- @return table  {type=I32, value=signed_i32}
function M.i32(value)
    return { type = I32, value = to_i32(value) }
end

--- Create an i64 WasmValue.
-- @param value number  The integer value (Lua native 64-bit integer).
-- @return table  {type=I64, value=integer}
function M.i64(value)
    return { type = I64, value = math.tointeger(value) or math.floor(value) }
end

--- Create an f32 WasmValue.
-- Rounds to IEEE 754 single precision using string.pack/unpack.
-- @param value number  The float value.
-- @return table  {type=F32, value=rounded_float}
function M.f32(value)
    local packed = string.pack("f", value)
    local rounded = string.unpack("f", packed)
    return { type = F32, value = rounded }
end

--- Create an f64 WasmValue.
-- Lua numbers are already IEEE 754 doubles, no conversion needed.
-- @param value number  The float value.
-- @return table  {type=F64, value=float}
function M.f64(value)
    return { type = F64, value = value + 0.0 }
end

--- Create a zero-initialized WasmValue for a given type code.
-- Used when initializing local variables in a function frame.
-- @param val_type number  The value type constant (I32, I64, F32, F64).
-- @return table  A zero-valued WasmValue.
function M.default_value(val_type)
    if val_type == I32 then return M.i32(0) end
    if val_type == I64 then return M.i64(0) end
    if val_type == F32 then return M.f32(0.0) end
    if val_type == F64 then return M.f64(0.0) end
    trap("unknown value type: 0x" .. string.format("%02x", val_type))
end

-- ============================================================================
-- TYPE-SAFE EXTRACTION HELPERS
-- ============================================================================
--
-- These functions extract the raw value from a WasmValue with a type check.
-- A type mismatch indicates a bug in the execution engine or the compiler.

local type_names = {
    [I32] = "i32",
    [I64] = "i64",
    [F32] = "f32",
    [F64] = "f64",
}

function M.as_i32(v)
    if v.type ~= I32 then
        trap("type mismatch: expected i32, got " .. (type_names[v.type] or "unknown"))
    end
    return v.value
end

function M.as_i64(v)
    if v.type ~= I64 then
        trap("type mismatch: expected i64, got " .. (type_names[v.type] or "unknown"))
    end
    return v.value
end

function M.as_f32(v)
    if v.type ~= F32 then
        trap("type mismatch: expected f32, got " .. (type_names[v.type] or "unknown"))
    end
    return v.value
end

function M.as_f64(v)
    if v.type ~= F64 then
        trap("type mismatch: expected f64, got " .. (type_names[v.type] or "unknown"))
    end
    return v.value
end


-- ============================================================================
-- LINEAR MEMORY
-- ============================================================================
--
-- WASM linear memory is a contiguous, byte-addressable array measured in
-- 64 KiB pages. In Lua, we use a Lua string as the backing store and
-- string.pack/unpack for typed reads/writes. For mutability we keep a table
-- of bytes internally and convert to/from strings as needed.
--
-- Actually, for performance we store memory as a flat array of integers
-- (bytes 0-255) and use string.pack/unpack on temporary strings for
-- multi-byte operations.
--
-- WASM uses LITTLE-ENDIAN byte order for all memory operations.

local PAGE_SIZE = 65536  -- 64 KiB per page

local LinearMemory = {}
LinearMemory.__index = LinearMemory
M.LinearMemory = LinearMemory
M.PAGE_SIZE = PAGE_SIZE

--- Create a new LinearMemory.
-- @param initial_pages number  Number of pages to allocate initially.
-- @param max_pages number|nil  Optional upper bound on page count.
-- @return LinearMemory
function LinearMemory.new(initial_pages, max_pages)
    local self = setmetatable({}, LinearMemory)
    self.current_pages = initial_pages
    self.max_pages = max_pages
    -- Initialize memory as a table of zeros.
    self.data = {}
    local total_bytes = initial_pages * PAGE_SIZE
    for i = 1, total_bytes do
        self.data[i] = 0
    end
    return self
end

--- Bounds check for memory access.
-- @param offset number  Byte offset into memory.
-- @param width number   Number of bytes to access.
local function mem_bounds_check(self, offset, width)
    local byte_length = self.current_pages * PAGE_SIZE
    if offset < 0 or offset + width > byte_length then
        trap(string.format(
            "out of bounds memory access: offset=%d, size=%d, memory size=%d",
            offset, width, byte_length))
    end
end

--- Load a signed 32-bit integer from memory (little-endian).
function LinearMemory:load_i32(offset)
    mem_bounds_check(self, offset, 4)
    -- Lua arrays are 1-based, memory offsets are 0-based.
    local b0 = self.data[offset + 1]
    local b1 = self.data[offset + 2]
    local b2 = self.data[offset + 3]
    local b3 = self.data[offset + 4]
    local val = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    return to_i32(val)
end

--- Load a signed 64-bit integer from memory (little-endian).
function LinearMemory:load_i64(offset)
    mem_bounds_check(self, offset, 8)
    local val = 0
    for i = 7, 0, -1 do
        val = (val << 8) | self.data[offset + i + 1]
    end
    return val
end

--- Load a 32-bit float from memory (little-endian).
function LinearMemory:load_f32(offset)
    mem_bounds_check(self, offset, 4)
    local bytes = string.char(
        self.data[offset + 1], self.data[offset + 2],
        self.data[offset + 3], self.data[offset + 4])
    return (string.unpack("<f", bytes))
end

--- Load a 64-bit float from memory (little-endian).
function LinearMemory:load_f64(offset)
    mem_bounds_check(self, offset, 8)
    local bytes = string.char(
        self.data[offset + 1], self.data[offset + 2],
        self.data[offset + 3], self.data[offset + 4],
        self.data[offset + 5], self.data[offset + 6],
        self.data[offset + 7], self.data[offset + 8])
    return (string.unpack("<d", bytes))
end

--- Load 1 byte, sign-extend to i32.
function LinearMemory:load_i32_8s(offset)
    mem_bounds_check(self, offset, 1)
    local val = self.data[offset + 1]
    if val >= 128 then val = val - 256 end
    return val
end

--- Load 1 byte, zero-extend to i32.
function LinearMemory:load_i32_8u(offset)
    mem_bounds_check(self, offset, 1)
    return self.data[offset + 1]
end

--- Load 2 bytes (LE), sign-extend to i32.
function LinearMemory:load_i32_16s(offset)
    mem_bounds_check(self, offset, 2)
    local val = self.data[offset + 1] | (self.data[offset + 2] << 8)
    if val >= 32768 then val = val - 65536 end
    return val
end

--- Load 2 bytes (LE), zero-extend to i32.
function LinearMemory:load_i32_16u(offset)
    mem_bounds_check(self, offset, 2)
    return self.data[offset + 1] | (self.data[offset + 2] << 8)
end

--- Store a 32-bit integer (little-endian).
function LinearMemory:store_i32(offset, value)
    mem_bounds_check(self, offset, 4)
    value = value & 0xFFFFFFFF
    self.data[offset + 1] = value & 0xFF
    self.data[offset + 2] = (value >> 8) & 0xFF
    self.data[offset + 3] = (value >> 16) & 0xFF
    self.data[offset + 4] = (value >> 24) & 0xFF
end

--- Store a 64-bit integer (little-endian).
function LinearMemory:store_i64(offset, value)
    mem_bounds_check(self, offset, 8)
    for i = 0, 7 do
        self.data[offset + i + 1] = value & 0xFF
        value = value >> 8
    end
end

--- Store a 32-bit float (little-endian).
function LinearMemory:store_f32(offset, value)
    mem_bounds_check(self, offset, 4)
    local packed = string.pack("<f", value)
    for i = 1, 4 do
        self.data[offset + i] = string.byte(packed, i)
    end
end

--- Store a 64-bit float (little-endian).
function LinearMemory:store_f64(offset, value)
    mem_bounds_check(self, offset, 8)
    local packed = string.pack("<d", value)
    for i = 1, 8 do
        self.data[offset + i] = string.byte(packed, i)
    end
end

--- Store low 8 bits of an i32.
function LinearMemory:store_i32_8(offset, value)
    mem_bounds_check(self, offset, 1)
    self.data[offset + 1] = value & 0xFF
end

--- Store low 16 bits of an i32 (little-endian).
function LinearMemory:store_i32_16(offset, value)
    mem_bounds_check(self, offset, 2)
    self.data[offset + 1] = value & 0xFF
    self.data[offset + 2] = (value >> 8) & 0xFF
end

--- Grow memory by delta pages. Returns old page count or -1 on failure.
function LinearMemory:grow(delta_pages)
    local old_pages = self.current_pages
    local new_pages = old_pages + delta_pages

    if self.max_pages ~= nil and new_pages > self.max_pages then
        return -1
    end
    if new_pages > 65536 then
        return -1
    end

    -- Extend the data array with zeros.
    local old_len = old_pages * PAGE_SIZE
    local new_len = new_pages * PAGE_SIZE
    for i = old_len + 1, new_len do
        self.data[i] = 0
    end
    self.current_pages = new_pages
    return old_pages
end

--- Return current memory size in pages.
function LinearMemory:size()
    return self.current_pages
end

--- Return current memory size in bytes.
function LinearMemory:byte_length()
    return self.current_pages * PAGE_SIZE
end

--- Write raw bytes into memory at the given offset.
-- Used during instantiation to apply data segments.
-- @param offset number  Byte offset.
-- @param bytes table|string  Array of byte values or a string.
function LinearMemory:write_bytes(offset, bytes)
    if type(bytes) == "string" then
        mem_bounds_check(self, offset, #bytes)
        for i = 1, #bytes do
            self.data[offset + i] = string.byte(bytes, i)
        end
    else
        mem_bounds_check(self, offset, #bytes)
        for i = 1, #bytes do
            self.data[offset + i] = bytes[i]
        end
    end
end


-- ============================================================================
-- TABLE
-- ============================================================================
--
-- A WASM table is an array of nullable function indices. In WASM 1.0, tables
-- hold funcref values used for indirect function calls (call_indirect).
--
-- Table elements are either a valid function index or nil (uninitialized).
-- Accessing a nil element via call_indirect traps.

local Table = {}
Table.__index = Table
M.Table = Table

--- Create a new Table.
-- @param initial_size number  Number of entries, all initialized to nil.
-- @param max_size number|nil  Optional upper bound.
-- @return Table
function Table.new(initial_size, max_size)
    local self = setmetatable({}, Table)
    self.elements = {}
    -- Initialize all entries to nil (sparse table in Lua).
    self._size = initial_size
    self.max_size = max_size
    return self
end

--- Get the function index at the given table index (0-based).
function Table:get(index)
    if index < 0 or index >= self._size then
        trap(string.format(
            "out of bounds table access: index=%d, table size=%d",
            index, self._size))
    end
    return self.elements[index]  -- nil if uninitialized
end

--- Set the function index at the given table index (0-based).
function Table:set(index, func_index)
    if index < 0 or index >= self._size then
        trap(string.format(
            "out of bounds table access: index=%d, table size=%d",
            index, self._size))
    end
    self.elements[index] = func_index
end

--- Return the current table size.
function Table:size()
    return self._size
end

--- Grow the table by delta entries. Returns old size or -1 on failure.
function Table:grow(delta)
    local old_size = self._size
    local new_size = old_size + delta
    if self.max_size ~= nil and new_size > self.max_size then
        return -1
    end
    self._size = new_size
    return old_size
end


-- ============================================================================
-- DECODER
-- ============================================================================
--
-- The decoder converts variable-length WASM bytecodes into fixed-format
-- instruction tables that the GenericVM can dispatch. Each decoded instruction
-- has:
--   opcode  -- the opcode byte
--   operand -- decoded immediate(s) or nil
--
-- The decoder also builds the control flow map, which maps each block/loop/if
-- instruction index to its matching end (and else for if instructions).

--- Decode a single immediate value from bytecodes.
-- @param code table  Array of bytes (1-indexed).
-- @param offset number  Current position in the byte array (1-indexed).
-- @param imm_type string  The immediate type from opcode metadata.
-- @return value, bytes_consumed
local function decode_single_immediate(code, offset, imm_type)
    if imm_type == "none" then
        return nil, 0

    elseif imm_type == "i32" or imm_type == "i32:s" or imm_type == "i32:i32" then
        -- Signed LEB128 for i32.const
        local val, consumed = wasm_leb128.decode_signed(code, offset)
        return val, consumed

    elseif imm_type == "i64:s" or imm_type == "i64:i64" then
        -- Signed LEB128 for i64.const (up to 10 bytes)
        local val, consumed = wasm_leb128.decode_signed(code, offset)
        return val, consumed

    elseif imm_type == "local_idx:u32" or imm_type == "global_idx:u32"
        or imm_type == "func_idx:u32" or imm_type == "label_idx:u32"
        or imm_type == "label:u32"
        or imm_type == "mem_idx:u32" then
        -- Unsigned LEB128 index
        local val, consumed = wasm_leb128.decode_unsigned(code, offset)
        return val, consumed

    elseif imm_type == "blocktype" then
        -- Block type: 0x40 (empty), value type byte, or signed LEB128 type index
        local byte = code[offset]
        if byte == 0x40 or byte == I32 or byte == I64 or byte == F32 or byte == F64 then
            return byte, 1
        end
        local val, consumed = wasm_leb128.decode_signed(code, offset)
        return val, consumed

    elseif imm_type == "memarg" or imm_type == "memarg(align:u32, offset:u32)" then
        -- Memory argument: align (unsigned LEB128) + offset (unsigned LEB128)
        local align, a_size = wasm_leb128.decode_unsigned(code, offset)
        local mem_offset, o_size = wasm_leb128.decode_unsigned(code, offset + a_size)
        return { align = align, offset = mem_offset }, a_size + o_size

    elseif imm_type == "f32:ieee754" then
        -- 4 bytes little-endian IEEE 754 float
        local bytes = string.char(code[offset], code[offset+1], code[offset+2], code[offset+3])
        local val = string.unpack("<f", bytes)
        return val, 4

    elseif imm_type == "f64:ieee754" then
        -- 8 bytes little-endian IEEE 754 double
        local bytes = string.char(
            code[offset], code[offset+1], code[offset+2], code[offset+3],
            code[offset+4], code[offset+5], code[offset+6], code[offset+7])
        local val = string.unpack("<d", bytes)
        return val, 8

    elseif imm_type == "vec(label:u32)+default:u32" then
        -- br_table: count + labels + default
        local count, c_size = wasm_leb128.decode_unsigned(code, offset)
        local pos = offset + c_size
        local labels = {}
        for i = 1, count do
            local label, l_size = wasm_leb128.decode_unsigned(code, pos)
            labels[i] = label
            pos = pos + l_size
        end
        local default_label, d_size = wasm_leb128.decode_unsigned(code, pos)
        pos = pos + d_size
        return { labels = labels, default_label = default_label }, pos - offset

    elseif imm_type == "type_idx:u32 table_idx:u32" then
        -- call_indirect: type index + table index
        local type_idx, t_size = wasm_leb128.decode_unsigned(code, offset)
        local table_idx, tb_size = wasm_leb128.decode_unsigned(code, offset + t_size)
        return { type_idx = type_idx, table_idx = table_idx }, t_size + tb_size
    end

    return nil, 0
end

--- Decode all instructions in a function body's bytecodes.
-- @param body table  {locals=..., body=byte_array}
-- @return table  Array of {opcode, operand, offset, size}
function M.decode_function_body(body)
    local code = body.body or body
    local instructions = {}
    local offset = 1  -- Lua 1-indexed

    while offset <= #code do
        local start_offset = offset
        local opcode_byte = code[offset]
        offset = offset + 1

        -- Look up opcode metadata for immediate types.
        local info = wasm_opcodes.get_opcode_info(opcode_byte)
        local operand = nil
        local imm_consumed = 0

        if info and info.operands and info.operands ~= "none" then
            operand, imm_consumed = decode_single_immediate(code, offset, info.operands)
            offset = offset + imm_consumed
        end

        instructions[#instructions + 1] = {
            opcode = opcode_byte,
            operand = operand,
        }
    end

    return instructions
end

--- Build the control flow map for a decoded instruction array.
--
-- Maps each block/loop/if instruction index to { end_pc, else_pc }.
-- Uses a stack-based algorithm: push on block/loop/if, pop on end.
--
-- @param instructions table  Array of decoded instructions.
-- @return table  Map from instruction index to {end_pc, else_pc}.
function M.build_control_flow_map(instructions)
    local cfmap = {}
    local stack = {}  -- { index, opcode, else_pc }

    for i, instr in ipairs(instructions) do
        local op = instr.opcode

        if op == 0x02 or op == 0x03 or op == 0x04 then
            -- block, loop, if
            stack[#stack + 1] = { index = i, opcode = op, else_pc = nil }

        elseif op == 0x05 then
            -- else
            if #stack > 0 then
                stack[#stack].else_pc = i
            end

        elseif op == 0x0B then
            -- end
            if #stack > 0 then
                local opener = stack[#stack]
                stack[#stack] = nil
                cfmap[opener.index] = {
                    end_pc = i,
                    else_pc = opener.else_pc,
                }
            end
            -- If stack is empty, this is the function's trailing end.
        end
    end

    return cfmap
end

--- Convert decoded instructions to GenericVM instruction format.
-- @param decoded table  Array of {opcode, operand}.
-- @return table  Array of {opcode, operand} (same format, just explicit).
function M.to_vm_instructions(decoded)
    local result = {}
    for i, d in ipairs(decoded) do
        result[i] = { opcode = d.opcode, operand = d.operand }
    end
    return result
end


-- ============================================================================
-- CONSTANT EXPRESSION EVALUATOR
-- ============================================================================
--
-- Constant expressions appear in global initializers, data segment offsets,
-- and element segment offsets. They consist of a single "const" instruction
-- followed by "end" (0x0B). We evaluate them to produce a WasmValue.

function M.evaluate_const_expr(expr_bytes, globals)
    globals = globals or {}
    if type(expr_bytes) ~= "table" or #expr_bytes == 0 then
        return M.i32(0)
    end

    local opcode = expr_bytes[1]
    if opcode == 0x41 then
        -- i32.const
        local val, _ = wasm_leb128.decode_signed(expr_bytes, 2)
        return M.i32(val)
    elseif opcode == 0x42 then
        -- i64.const
        local val, _ = wasm_leb128.decode_signed(expr_bytes, 2)
        return M.i64(val)
    elseif opcode == 0x43 then
        -- f32.const
        local bytes = string.char(expr_bytes[2], expr_bytes[3], expr_bytes[4], expr_bytes[5])
        local val = string.unpack("<f", bytes)
        return M.f32(val)
    elseif opcode == 0x44 then
        -- f64.const
        local bytes = string.char(
            expr_bytes[2], expr_bytes[3], expr_bytes[4], expr_bytes[5],
            expr_bytes[6], expr_bytes[7], expr_bytes[8], expr_bytes[9])
        local val = string.unpack("<d", bytes)
        return M.f64(val)
    elseif opcode == 0x23 then
        -- global.get
        local idx, _ = wasm_leb128.decode_unsigned(expr_bytes, 2)
        if globals[idx + 1] then
            return globals[idx + 1]
        end
        return M.i32(0)
    end

    return M.i32(0)
end

M.evaluate_const_expr = M.evaluate_const_expr


-- ============================================================================
-- BLOCK TYPE RESOLUTION
-- ============================================================================
--
-- Resolves a block type operand to its result arity.
--
-- WASM 1.0 block types:
--   0x40           -> empty (0 results)
--   0x7F/7E/7D/7C -> single value type (1 result)
--   >= 0           -> type index (multi-value)

local function block_arity(block_type, func_types)
    if block_type == nil or block_type == 0x40 then return 0 end
    if block_type == I32 or block_type == I64
        or block_type == F32 or block_type == F64 then
        return 1
    end
    -- Type index for multi-value blocks
    if block_type >= 0 and func_types[block_type + 1] then
        return #func_types[block_type + 1].results
    end
    return 0
end


-- ============================================================================
-- BRANCH EXECUTION
-- ============================================================================
--
-- Core branching primitive used by br, br_if, and br_table.

local function execute_branch(vm, ctx, label_index)
    local label_stack_index = #ctx.label_stack - label_index
    if label_stack_index < 1 then
        trap("branch target " .. label_index .. " out of range")
    end

    local label = ctx.label_stack[label_stack_index]

    -- For loops, branch carries 0 params (MVP). For blocks, carries arity results.
    local arity = label.is_loop and 0 or label.arity

    -- Save result values from the top of the stack.
    local results = {}
    for j = 1, arity do
        results[arity - j + 1] = vm:pop_typed()
    end

    -- Unwind the typed stack to the label's recorded height.
    while #vm.typed_stack > label.stack_height do
        vm:pop_typed()
    end

    -- Push results back.
    for _, v in ipairs(results) do
        vm.typed_stack[#vm.typed_stack + 1] = v
    end

    -- Pop labels down to and including the target.
    for i = #ctx.label_stack, label_stack_index, -1 do
        ctx.label_stack[i] = nil
    end

    -- Jump to the label's target PC.
    vm:jump_to(label.target_pc)
end


-- ============================================================================
-- INSTRUCTION HANDLER REGISTRATION
-- ============================================================================
--
-- All WASM instruction handlers are registered as context opcodes on the
-- GenericVM. They receive (vm, instr, code, ctx) where ctx is the
-- WasmExecutionContext table.

local function register_all_instructions(vm)

    -- ====================================================================
    -- CONTROL FLOW
    -- ====================================================================

    -- unreachable (0x00)
    vm:register_context_opcode(0x00, function(vm, instr, code, ctx)
        trap("unreachable instruction executed")
    end)

    -- nop (0x01)
    vm:register_context_opcode(0x01, function(vm, instr, code, ctx)
        vm:advance_pc()
    end)

    -- block (0x02)
    vm:register_context_opcode(0x02, function(vm, instr, code, ctx)
        local block_type = instr.operand
        local arity = block_arity(block_type, ctx.func_types)
        local target = ctx.control_flow_map[vm.pc]
        local end_pc = target and target.end_pc or (vm.pc + 1)

        ctx.label_stack[#ctx.label_stack + 1] = {
            arity = arity,
            target_pc = end_pc,
            stack_height = #vm.typed_stack,
            is_loop = false,
        }
        vm:advance_pc()
    end)

    -- loop (0x03)
    vm:register_context_opcode(0x03, function(vm, instr, code, ctx)
        local block_type = instr.operand
        local arity = block_arity(block_type, ctx.func_types)

        ctx.label_stack[#ctx.label_stack + 1] = {
            arity = arity,
            target_pc = vm.pc,  -- Loop branches back to start!
            stack_height = #vm.typed_stack,
            is_loop = true,
        }
        vm:advance_pc()
    end)

    -- if (0x04)
    vm:register_context_opcode(0x04, function(vm, instr, code, ctx)
        local block_type = instr.operand
        local arity = block_arity(block_type, ctx.func_types)
        local condition = vm:pop_typed().value

        local target = ctx.control_flow_map[vm.pc]
        local end_pc = target and target.end_pc or (vm.pc + 1)
        local else_pc = target and target.else_pc or nil

        ctx.label_stack[#ctx.label_stack + 1] = {
            arity = arity,
            target_pc = end_pc,
            stack_height = #vm.typed_stack,
            is_loop = false,
        }

        if condition ~= 0 then
            vm:advance_pc()
        else
            vm:jump_to(else_pc and (else_pc + 1) or end_pc)
        end
    end)

    -- else (0x05)
    vm:register_context_opcode(0x05, function(vm, instr, code, ctx)
        local label = ctx.label_stack[#ctx.label_stack]
        vm:jump_to(label.target_pc)
    end)

    -- end (0x0B)
    vm:register_context_opcode(0x0B, function(vm, instr, code, ctx)
        if #ctx.label_stack > 0 then
            ctx.label_stack[#ctx.label_stack] = nil
            vm:advance_pc()
        else
            -- Only the final end of the function should halt execution.
            -- Structured control can legally jump to later end opcodes and
            -- continue with more instructions afterward.
            if vm.pc >= #code.instructions then
                ctx.returned = true
                vm.halted = true
            else
                vm:advance_pc()
            end
        end
    end)

    -- br (0x0C)
    vm:register_context_opcode(0x0C, function(vm, instr, code, ctx)
        execute_branch(vm, ctx, instr.operand)
    end)

    -- br_if (0x0D)
    vm:register_context_opcode(0x0D, function(vm, instr, code, ctx)
        local condition = vm:pop_typed().value
        if condition ~= 0 then
            execute_branch(vm, ctx, instr.operand)
        else
            vm:advance_pc()
        end
    end)

    -- br_table (0x0E)
    vm:register_context_opcode(0x0E, function(vm, instr, code, ctx)
        local tbl = instr.operand
        local index = vm:pop_typed().value
        local target_label
        if index >= 0 and index < #tbl.labels then
            target_label = tbl.labels[index + 1]
        else
            target_label = tbl.default_label
        end
        execute_branch(vm, ctx, target_label)
    end)

    -- return (0x0F)
    vm:register_context_opcode(0x0F, function(vm, instr, code, ctx)
        ctx.returned = true
        vm.halted = true
    end)

    -- call (0x10)
    vm:register_context_opcode(0x10, function(vm, instr, code, ctx)
        local func_index = instr.operand
        call_function(vm, ctx, func_index)
    end)

    -- call_indirect (0x11)
    vm:register_context_opcode(0x11, function(vm, instr, code, ctx)
        local operand = instr.operand
        local type_idx = operand.type_idx
        local table_idx = operand.table_idx or 0
        local elem_index = vm:pop_typed().value

        local tbl = ctx.tables[table_idx + 1]
        if not tbl then trap("undefined table") end

        local func_index = tbl:get(elem_index)
        if func_index == nil then trap("uninitialized table element") end

        -- Type check
        local expected = ctx.func_types[type_idx + 1]
        local actual = ctx.func_types[func_index + 1]
        if not expected or not actual then trap("undefined type") end
        if #expected.params ~= #actual.params or #expected.results ~= #actual.results then
            trap("indirect call type mismatch")
        end
        for i = 1, #expected.params do
            if expected.params[i] ~= actual.params[i] then
                trap("indirect call type mismatch")
            end
        end
        for i = 1, #expected.results do
            if expected.results[i] ~= actual.results[i] then
                trap("indirect call type mismatch")
            end
        end

        call_function(vm, ctx, func_index)
    end)

    -- ====================================================================
    -- VARIABLE INSTRUCTIONS
    -- ====================================================================

    -- local.get (0x20)
    vm:register_context_opcode(0x20, function(vm, instr, code, ctx)
        local index = instr.operand
        vm.typed_stack[#vm.typed_stack + 1] = ctx.typed_locals[index + 1]
        vm:advance_pc()
    end)

    -- local.set (0x21)
    vm:register_context_opcode(0x21, function(vm, instr, code, ctx)
        local index = instr.operand
        ctx.typed_locals[index + 1] = vm:pop_typed()
        vm:advance_pc()
    end)

    -- local.tee (0x22) -- set without popping
    vm:register_context_opcode(0x22, function(vm, instr, code, ctx)
        local index = instr.operand
        ctx.typed_locals[index + 1] = vm:peek_typed()
        vm:advance_pc()
    end)

    -- global.get (0x23)
    vm:register_context_opcode(0x23, function(vm, instr, code, ctx)
        local index = instr.operand
        vm.typed_stack[#vm.typed_stack + 1] = ctx.globals[index + 1]
        vm:advance_pc()
    end)

    -- global.set (0x24)
    vm:register_context_opcode(0x24, function(vm, instr, code, ctx)
        local index = instr.operand
        ctx.globals[index + 1] = vm:pop_typed()
        vm:advance_pc()
    end)

    -- ====================================================================
    -- PARAMETRIC INSTRUCTIONS
    -- ====================================================================

    -- drop (0x1A)
    vm:register_context_opcode(0x1A, function(vm, instr, code, ctx)
        vm:pop_typed()
        vm:advance_pc()
    end)

    -- select (0x1B)
    vm:register_context_opcode(0x1B, function(vm, instr, code, ctx)
        local condition = vm:pop_typed().value
        local val2 = vm:pop_typed()
        local val1 = vm:pop_typed()
        if condition ~= 0 then
            vm.typed_stack[#vm.typed_stack + 1] = val1
        else
            vm.typed_stack[#vm.typed_stack + 1] = val2
        end
        vm:advance_pc()
    end)

    -- ====================================================================
    -- i32 NUMERIC INSTRUCTIONS
    -- ====================================================================

    -- i32.const (0x41)
    vm:register_context_opcode(0x41, function(vm, instr, code, ctx)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(instr.operand)
        vm:advance_pc()
    end)

    -- i32.eqz (0x45)
    vm:register_context_opcode(0x45, function(vm, instr, code, ctx)
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a == 0 and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.eq (0x46)
    vm:register_context_opcode(0x46, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a == b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.ne (0x47)
    vm:register_context_opcode(0x47, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a ~= b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.lt_s (0x48)
    vm:register_context_opcode(0x48, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a < b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.lt_u (0x49)
    vm:register_context_opcode(0x49, function(vm, instr, code, ctx)
        local b = vm:pop_typed().value & 0xFFFFFFFF
        local a = vm:pop_typed().value & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a < b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.gt_s (0x4A)
    vm:register_context_opcode(0x4A, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a > b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.gt_u (0x4B)
    vm:register_context_opcode(0x4B, function(vm, instr, code, ctx)
        local b = vm:pop_typed().value & 0xFFFFFFFF
        local a = vm:pop_typed().value & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a > b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.le_s (0x4C)
    vm:register_context_opcode(0x4C, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a <= b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.le_u (0x4D)
    vm:register_context_opcode(0x4D, function(vm, instr, code, ctx)
        local b = vm:pop_typed().value & 0xFFFFFFFF
        local a = vm:pop_typed().value & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a <= b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.ge_s (0x4E)
    vm:register_context_opcode(0x4E, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a >= b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.ge_u (0x4F)
    vm:register_context_opcode(0x4F, function(vm, instr, code, ctx)
        local b = vm:pop_typed().value & 0xFFFFFFFF
        local a = vm:pop_typed().value & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a >= b and 1 or 0)
        vm:advance_pc()
    end)

    -- i32.clz (0x67)
    vm:register_context_opcode(0x67, function(vm, instr, code, ctx)
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local count = 0
        if a == 0 then
            count = 32
        else
            while (a & 0x80000000) == 0 do
                count = count + 1
                a = a << 1
            end
        end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(count)
        vm:advance_pc()
    end)

    -- i32.ctz (0x68)
    vm:register_context_opcode(0x68, function(vm, instr, code, ctx)
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local count = 0
        if a == 0 then
            count = 32
        else
            while (a & 1) == 0 do
                count = count + 1
                a = a >> 1
            end
        end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(count)
        vm:advance_pc()
    end)

    -- i32.popcnt (0x69)
    vm:register_context_opcode(0x69, function(vm, instr, code, ctx)
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local count = 0
        while a ~= 0 do
            count = count + (a & 1)
            a = a >> 1
        end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(count)
        vm:advance_pc()
    end)

    -- i32.add (0x6A)
    vm:register_context_opcode(0x6A, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a + b)
        vm:advance_pc()
    end)

    -- i32.sub (0x6B)
    vm:register_context_opcode(0x6B, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a - b)
        vm:advance_pc()
    end)

    -- i32.mul (0x6C)
    vm:register_context_opcode(0x6C, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a * b)
        vm:advance_pc()
    end)

    -- i32.div_s (0x6D)
    vm:register_context_opcode(0x6D, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        if b == 0 then trap("integer divide by zero") end
        if a == -2147483648 and b == -1 then trap("integer overflow") end
        -- Truncate toward zero (C-style division)
        local result
        if (a < 0) ~= (b < 0) then
            result = -math.floor(math.abs(a) / math.abs(b))
        else
            result = math.floor(math.abs(a) / math.abs(b))
        end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(result)
        vm:advance_pc()
    end)

    -- i32.div_u (0x6E)
    vm:register_context_opcode(0x6E, function(vm, instr, code, ctx)
        local b = vm:pop_typed().value & 0xFFFFFFFF
        local a = vm:pop_typed().value & 0xFFFFFFFF
        if b == 0 then trap("integer divide by zero") end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a // b)
        vm:advance_pc()
    end)

    -- i32.rem_s (0x6F)
    vm:register_context_opcode(0x6F, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        if b == 0 then trap("integer divide by zero") end
        -- WASM rem_s: result has the sign of the dividend (a), like C99 %.
        local result = a % b
        -- Lua's % follows floor division. We need truncation-toward-zero remainder.
        if result ~= 0 and ((a < 0) ~= (result < 0)) then
            result = result - b
        end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(result)
        vm:advance_pc()
    end)

    -- i32.rem_u (0x70)
    vm:register_context_opcode(0x70, function(vm, instr, code, ctx)
        local b = vm:pop_typed().value & 0xFFFFFFFF
        local a = vm:pop_typed().value & 0xFFFFFFFF
        if b == 0 then trap("integer divide by zero") end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a % b)
        vm:advance_pc()
    end)

    -- i32.and (0x71)
    vm:register_context_opcode(0x71, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a & b)
        vm:advance_pc()
    end)

    -- i32.or (0x72)
    vm:register_context_opcode(0x72, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a | b)
        vm:advance_pc()
    end)

    -- i32.xor (0x73)
    vm:register_context_opcode(0x73, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed())
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a ~ b)
        vm:advance_pc()
    end)

    -- i32.shl (0x74)
    vm:register_context_opcode(0x74, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed()) & 31
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a << b)
        vm:advance_pc()
    end)

    -- i32.shr_s (0x75) -- arithmetic shift right
    vm:register_context_opcode(0x75, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed()) & 31
        local a = M.as_i32(vm:pop_typed())
        -- Arithmetic right shift preserves the sign bit.
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a >> b)
        vm:advance_pc()
    end)

    -- i32.shr_u (0x76) -- logical shift right
    vm:register_context_opcode(0x76, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed()) & 31
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a >> b)
        vm:advance_pc()
    end)

    -- i32.rotl (0x77)
    vm:register_context_opcode(0x77, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed()) & 31
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local result = ((a << b) | (a >> (32 - b))) & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(result)
        vm:advance_pc()
    end)

    -- i32.rotr (0x78)
    vm:register_context_opcode(0x78, function(vm, instr, code, ctx)
        local b = M.as_i32(vm:pop_typed()) & 31
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local result = ((a >> b) | (a << (32 - b))) & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(result)
        vm:advance_pc()
    end)

    -- ====================================================================
    -- i64 NUMERIC INSTRUCTIONS (subset)
    -- ====================================================================

    -- i64.const (0x42)
    vm:register_context_opcode(0x42, function(vm, instr, code, ctx)
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(instr.operand)
        vm:advance_pc()
    end)

    -- i64.eqz (0x50)
    vm:register_context_opcode(0x50, function(vm, instr, code, ctx)
        local a = M.as_i64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a == 0 and 1 or 0)
        vm:advance_pc()
    end)

    -- i64.eq (0x51)
    vm:register_context_opcode(0x51, function(vm, instr, code, ctx)
        local b = M.as_i64(vm:pop_typed())
        local a = M.as_i64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a == b and 1 or 0)
        vm:advance_pc()
    end)

    -- i64.add (0x7C)
    vm:register_context_opcode(0x7C, function(vm, instr, code, ctx)
        local b = M.as_i64(vm:pop_typed())
        local a = M.as_i64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(a + b)
        vm:advance_pc()
    end)

    -- i64.sub (0x7D)
    vm:register_context_opcode(0x7D, function(vm, instr, code, ctx)
        local b = M.as_i64(vm:pop_typed())
        local a = M.as_i64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(a - b)
        vm:advance_pc()
    end)

    -- i64.mul (0x7E)
    vm:register_context_opcode(0x7E, function(vm, instr, code, ctx)
        local b = M.as_i64(vm:pop_typed())
        local a = M.as_i64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(a * b)
        vm:advance_pc()
    end)

    -- ====================================================================
    -- f32 NUMERIC INSTRUCTIONS (subset)
    -- ====================================================================

    -- f32.const (0x43)
    vm:register_context_opcode(0x43, function(vm, instr, code, ctx)
        vm.typed_stack[#vm.typed_stack + 1] = M.f32(instr.operand)
        vm:advance_pc()
    end)

    -- f32.add (0x92)
    vm:register_context_opcode(0x92, function(vm, instr, code, ctx)
        local b = M.as_f32(vm:pop_typed())
        local a = M.as_f32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f32(a + b)
        vm:advance_pc()
    end)

    -- f32.sub (0x93)
    vm:register_context_opcode(0x93, function(vm, instr, code, ctx)
        local b = M.as_f32(vm:pop_typed())
        local a = M.as_f32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f32(a - b)
        vm:advance_pc()
    end)

    -- f32.mul (0x94)
    vm:register_context_opcode(0x94, function(vm, instr, code, ctx)
        local b = M.as_f32(vm:pop_typed())
        local a = M.as_f32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f32(a * b)
        vm:advance_pc()
    end)

    -- f32.div (0x95)
    vm:register_context_opcode(0x95, function(vm, instr, code, ctx)
        local b = M.as_f32(vm:pop_typed())
        local a = M.as_f32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f32(a / b)
        vm:advance_pc()
    end)

    -- ====================================================================
    -- f64 NUMERIC INSTRUCTIONS (subset)
    -- ====================================================================

    -- f64.const (0x44)
    vm:register_context_opcode(0x44, function(vm, instr, code, ctx)
        vm.typed_stack[#vm.typed_stack + 1] = M.f64(instr.operand)
        vm:advance_pc()
    end)

    -- f64.add (0xA0)
    vm:register_context_opcode(0xA0, function(vm, instr, code, ctx)
        local b = M.as_f64(vm:pop_typed())
        local a = M.as_f64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f64(a + b)
        vm:advance_pc()
    end)

    -- f64.sub (0xA1)
    vm:register_context_opcode(0xA1, function(vm, instr, code, ctx)
        local b = M.as_f64(vm:pop_typed())
        local a = M.as_f64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f64(a - b)
        vm:advance_pc()
    end)

    -- f64.mul (0xA2)
    vm:register_context_opcode(0xA2, function(vm, instr, code, ctx)
        local b = M.as_f64(vm:pop_typed())
        local a = M.as_f64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f64(a * b)
        vm:advance_pc()
    end)

    -- f64.div (0xA3)
    vm:register_context_opcode(0xA3, function(vm, instr, code, ctx)
        local b = M.as_f64(vm:pop_typed())
        local a = M.as_f64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.f64(a / b)
        vm:advance_pc()
    end)

    -- ====================================================================
    -- CONVERSION INSTRUCTIONS (subset)
    -- ====================================================================

    -- i32.wrap_i64 (0xA7)
    vm:register_context_opcode(0xA7, function(vm, instr, code, ctx)
        local a = M.as_i64(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(a)
        vm:advance_pc()
    end)

    -- i64.extend_i32_s (0xAC)
    vm:register_context_opcode(0xAC, function(vm, instr, code, ctx)
        local a = M.as_i32(vm:pop_typed())
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(a)
        vm:advance_pc()
    end)

    -- i64.extend_i32_u (0xAD)
    vm:register_context_opcode(0xAD, function(vm, instr, code, ctx)
        local a = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(a)
        vm:advance_pc()
    end)

    -- ====================================================================
    -- MEMORY INSTRUCTIONS (subset)
    -- ====================================================================

    -- i32.load (0x28)
    vm:register_context_opcode(0x28, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(ctx.memory:load_i32(addr))
        vm:advance_pc()
    end)

    -- i64.load (0x29)
    vm:register_context_opcode(0x29, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.i64(ctx.memory:load_i64(addr))
        vm:advance_pc()
    end)

    -- f32.load (0x2A)
    vm:register_context_opcode(0x2A, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.f32(ctx.memory:load_f32(addr))
        vm:advance_pc()
    end)

    -- f64.load (0x2B)
    vm:register_context_opcode(0x2B, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.f64(ctx.memory:load_f64(addr))
        vm:advance_pc()
    end)

    -- i32.load8_s (0x2C)
    vm:register_context_opcode(0x2C, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(ctx.memory:load_i32_8s(addr))
        vm:advance_pc()
    end)

    -- i32.load8_u (0x2D)
    vm:register_context_opcode(0x2D, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(ctx.memory:load_i32_8u(addr))
        vm:advance_pc()
    end)

    -- i32.load16_s (0x2E)
    vm:register_context_opcode(0x2E, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(ctx.memory:load_i32_16s(addr))
        vm:advance_pc()
    end)

    -- i32.load16_u (0x2F)
    vm:register_context_opcode(0x2F, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(ctx.memory:load_i32_16u(addr))
        vm:advance_pc()
    end)

    -- i32.store (0x36)
    vm:register_context_opcode(0x36, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local value = M.as_i32(vm:pop_typed())
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        ctx.memory:store_i32(addr, value)
        vm:advance_pc()
    end)

    -- i64.store (0x37)
    vm:register_context_opcode(0x37, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local value = M.as_i64(vm:pop_typed())
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        ctx.memory:store_i64(addr, value)
        vm:advance_pc()
    end)

    -- f32.store (0x38)
    vm:register_context_opcode(0x38, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local value = M.as_f32(vm:pop_typed())
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        ctx.memory:store_f32(addr, value)
        vm:advance_pc()
    end)

    -- f64.store (0x39)
    vm:register_context_opcode(0x39, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local value = M.as_f64(vm:pop_typed())
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        ctx.memory:store_f64(addr, value)
        vm:advance_pc()
    end)

    -- i32.store8 (0x3A)
    vm:register_context_opcode(0x3A, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local value = M.as_i32(vm:pop_typed())
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        ctx.memory:store_i32_8(addr, value)
        vm:advance_pc()
    end)

    -- i32.store16 (0x3B)
    vm:register_context_opcode(0x3B, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local memarg = instr.operand
        local value = M.as_i32(vm:pop_typed())
        local base = M.as_i32(vm:pop_typed()) & 0xFFFFFFFF
        local addr = base + (memarg and memarg.offset or 0)
        ctx.memory:store_i32_16(addr, value)
        vm:advance_pc()
    end)

    -- memory.size (0x3F)
    vm:register_context_opcode(0x3F, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(ctx.memory:size())
        vm:advance_pc()
    end)

    -- memory.grow (0x40)
    vm:register_context_opcode(0x40, function(vm, instr, code, ctx)
        if not ctx.memory then trap("no memory") end
        local delta = M.as_i32(vm:pop_typed())
        local result = ctx.memory:grow(delta)
        vm.typed_stack[#vm.typed_stack + 1] = M.i32(result)
        vm:advance_pc()
    end)
end


-- ============================================================================
-- FUNCTION CALL IMPLEMENTATION
-- ============================================================================
--
-- Handles both module-defined and host (imported) functions. For module
-- functions, saves the caller's state, sets up the callee's frame, and
-- reconfigures the VM to execute the callee's bytecodes.

function call_function(vm, ctx, func_index)
    local func_type = ctx.func_types[func_index + 1]
    if not func_type then
        trap("undefined function " .. func_index)
    end

    -- Pop arguments (in reverse order).
    local args = {}
    for j = #func_type.params, 1, -1 do
        args[j] = vm:pop_typed()
    end

    -- Check if this is a host function.
    local host_func = ctx.host_functions[func_index + 1]
    if host_func then
        local results = host_func.call(args)
        for _, r in ipairs(results) do
            vm.typed_stack[#vm.typed_stack + 1] = r
        end
        vm:advance_pc()
        return
    end

    -- Module-defined function: set up a new frame.
    local body = ctx.func_bodies[func_index + 1]
    if not body then
        trap("no body for function " .. func_index)
    end

    -- Save the caller's state.
    ctx.saved_frames[#ctx.saved_frames + 1] = {
        locals = {},
        label_stack = ctx.label_stack,
        stack_height = #vm.typed_stack,
        control_flow_map = ctx.control_flow_map,
        return_pc = vm.pc + 1,
        return_arity = #func_type.results,
        code = ctx.current_code,
    }
    -- Copy the caller's locals.
    for i, v in ipairs(ctx.typed_locals) do
        ctx.saved_frames[#ctx.saved_frames].locals[i] = v
    end

    -- Initialize the callee's locals: args + zero-initialized declared locals.
    local new_locals = {}
    for i, a in ipairs(args) do
        new_locals[i] = a
    end
    -- Expand declared locals from the function body.
    local local_decls = body.locals or {}
    for _, decl in ipairs(local_decls) do
        for _ = 1, decl.count do
            new_locals[#new_locals + 1] = M.default_value(decl.type)
        end
    end
    ctx.typed_locals = new_locals

    -- Clear label stack for the new frame.
    ctx.label_stack = {}

    -- Decode the callee's body and build control flow map.
    local decoded = M.decode_function_body(body)
    local cfmap = M.build_control_flow_map(decoded)
    local vm_instructions = M.to_vm_instructions(decoded)

    ctx.control_flow_map = cfmap
    ctx.current_code = {
        instructions = vm_instructions,
        constants = {},
        names = {},
    }

    -- Reset execution state for the callee.
    ctx.returned = false
    vm.halted = false
    vm:jump_to(1)  -- Lua 1-indexed

    -- Execute the callee within the same context (recursive call).
    vm:execute_with_context(ctx.current_code, ctx)

    -- After callee returns, restore the caller's state.
    local frame = ctx.saved_frames[#ctx.saved_frames]
    ctx.saved_frames[#ctx.saved_frames] = nil

    -- Collect return values from the typed stack.
    local return_values = {}
    for i = 1, frame.return_arity do
        if #vm.typed_stack > 0 then
            return_values[frame.return_arity - i + 1] = vm:pop_typed()
        end
    end

    -- Unwind any extra values left by the callee above the frame boundary.
    while #vm.typed_stack > frame.stack_height do
        vm:pop_typed()
    end

    -- Push return values onto the caller's stack.
    for _, v in ipairs(return_values) do
        vm.typed_stack[#vm.typed_stack + 1] = v
    end

    -- Restore caller state.
    ctx.typed_locals = frame.locals
    ctx.label_stack = frame.label_stack
    ctx.control_flow_map = frame.control_flow_map
    ctx.current_code = frame.code

    -- Resume the caller.
    ctx.returned = false
    vm.halted = false
    vm:jump_to(frame.return_pc)
end


-- ============================================================================
-- WASM EXECUTION ENGINE
-- ============================================================================
--
-- The top-level execution engine. Takes the module's runtime state (memory,
-- tables, globals, functions) and provides call_function to invoke WASM
-- functions by index.

local WasmExecutionEngine = {}
WasmExecutionEngine.__index = WasmExecutionEngine
M.WasmExecutionEngine = WasmExecutionEngine

local MAX_CALL_DEPTH = 1024

--- Create a new WasmExecutionEngine.
-- @param config table  Configuration with module runtime state.
-- @return WasmExecutionEngine
function WasmExecutionEngine.new(config)
    local self = setmetatable({}, WasmExecutionEngine)
    self.memory = config.memory
    self.tables = config.tables or {}
    self.globals = config.globals or {}
    self.global_types = config.global_types or {}
    self.func_types = config.func_types or {}
    self.func_bodies = config.func_bodies or {}
    self.host_functions = config.host_functions or {}

    -- Create and configure the GenericVM.
    self.vm = virtual_machine.GenericVM.new()
    self.vm:set_max_recursion_depth(MAX_CALL_DEPTH)

    -- Register all WASM instruction handlers.
    register_all_instructions(self.vm)

    return self
end

--- Call a WASM function by index (0-based).
--
-- @param func_index number  The function index (0-based, imports first).
-- @param args table  Array of WasmValue tables.
-- @return table  Array of WasmValue results.
function WasmExecutionEngine:call_function(func_index, args)
    local func_type = self.func_types[func_index + 1]
    if not func_type then
        trap("undefined function index " .. func_index)
    end

    -- Validate argument count.
    if #args ~= #func_type.params then
        trap(string.format(
            "function %d expects %d arguments, got %d",
            func_index, #func_type.params, #args))
    end

    -- Check if this is a host function.
    local host_func = self.host_functions[func_index + 1]
    if host_func then
        return host_func.call(args)
    end

    -- Module-defined function.
    local body = self.func_bodies[func_index + 1]
    if not body then
        trap("no body for function " .. func_index)
    end

    -- Decode the function body.
    local decoded = M.decode_function_body(body)
    local control_flow_map = M.build_control_flow_map(decoded)
    local vm_instructions = M.to_vm_instructions(decoded)

    -- Initialize locals: arguments + zero-initialized declared locals.
    local typed_locals = {}
    for i, a in ipairs(args) do
        typed_locals[i] = a
    end
    local local_decls = body.locals or {}
    for _, decl in ipairs(local_decls) do
        for _ = 1, decl.count do
            typed_locals[#typed_locals + 1] = M.default_value(decl.type)
        end
    end

    -- Build the execution context.
    local code = {
        instructions = vm_instructions,
        constants = {},
        names = {},
    }

    local ctx = {
        memory = self.memory,
        tables = self.tables,
        globals = self.globals,
        global_types = self.global_types,
        func_types = self.func_types,
        func_bodies = self.func_bodies,
        host_functions = self.host_functions,
        typed_locals = typed_locals,
        label_stack = {},
        control_flow_map = control_flow_map,
        saved_frames = {},
        returned = false,
        return_values = {},
        current_code = code,
    }

    -- Reset the VM and execute.
    self.vm:reset()
    self.vm:execute_with_context(code, ctx)

    -- Collect return values from the typed stack.
    local result_count = #func_type.results
    local results = {}
    for i = 1, result_count do
        if #self.vm.typed_stack > 0 then
            results[result_count - i + 1] = self.vm:pop_typed()
        end
    end

    return results
end


return M

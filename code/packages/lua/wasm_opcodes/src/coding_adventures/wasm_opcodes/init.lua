-- ============================================================================
-- wasm_opcodes — WebAssembly opcode definitions
-- ============================================================================
--
-- An "opcode" (operation code) is the numeric identifier for a single
-- instruction in a virtual machine's instruction set. WebAssembly's binary
-- format encodes each instruction as one or more bytes, starting with an
-- opcode byte that identifies which operation to perform.
--
-- ## How WebAssembly Instructions Are Encoded
--
-- In the WebAssembly binary format, instruction encoding follows this pattern:
--
--   [opcode_byte] [optional_immediate_bytes...]
--
-- For most instructions, the opcode fits in a single byte (0x00–0xFF).
-- Multi-byte opcodes (used for SIMD and GC proposals) start with 0xFD, 0xFE,
-- or 0xFF followed by a LEB128 extension — those are NOT included here.
--
-- ## Instruction Categories
--
-- WebAssembly instructions are grouped into categories:
--
--   CONTROL FLOW
--   ─────────────
--   unreachable (0x00) — Trap unconditionally. Used after code that should
--     never be reached (e.g., after a function that always panics).
--   nop (0x01) — No operation. Useful as a placeholder.
--   block (0x02) — Begin a block expression. Block type follows.
--   loop (0x03) — Begin a loop (branch target is the start, not the end).
--   if (0x04) — Begin conditional block. Pops a condition from the stack.
--   else (0x05) — Else branch of an if. Only valid between if and end.
--   end (0x0b) — End a block, loop, if, or function.
--   br (0x0c) — Unconditional branch. Immediate: label index (LEB128).
--   br_if (0x0d) — Conditional branch. Pops condition; branches if nonzero.
--   br_table (0x0e) — Indirect branch table. Immediate: vec(label) + default.
--   return (0x0f) — Return from current function.
--   call (0x10) — Call a function by index. Immediate: func index (LEB128).
--   call_indirect (0x11) — Call through a function table. Immediates: type
--     index, table index.
--
--   PARAMETRIC
--   ───────────
--   drop (0x1a) — Discard the top of the value stack.
--   select (0x1b) — Conditional select: pops condition and two values,
--     pushes the first if condition is nonzero, the second otherwise.
--
--   VARIABLE ACCESS
--   ────────────────
--   local.get (0x20) — Push value of local variable. Immediate: local index.
--   local.set (0x21) — Pop and store in local variable.
--   local.tee (0x22) — Copy top of stack into local, but do NOT pop it.
--   global.get (0x23) — Push value of global variable.
--   global.set (0x24) — Pop and store in global variable.
--
--   MEMORY INSTRUCTIONS
--   ────────────────────
--   Load/store instructions carry a "memory argument" with two LEB128 values:
--     alignment hint (log₂ of alignment, e.g., 2 = 4-byte alignment)
--     byte offset (added to the address before accessing memory)
--
--   NUMERIC INSTRUCTIONS
--   ─────────────────────
--   Each numeric type (i32, i64, f32, f64) has its own opcode range.
--   Constants (const) take an immediate value in LEB128 (or IEEE 754 bytes).
--   Comparisons pop two values and push an i32 (0 or 1).
--   Arithmetic pops two values (or one for unary operations) and pushes one.
--
-- ## This Module's Role
--
-- This module is a reference table — it maps byte values to human-readable
-- names and brief descriptions. Higher-level tools (assemblers, disassemblers,
-- JIT compilers, validators) use this table to:
--   1. Look up what an opcode does without consulting the spec.
--   2. Generate error messages mentioning instruction names.
--   3. Validate that an opcode byte is recognized.
--
-- ## Usage
--
--   local op = require("coding_adventures.wasm_opcodes")
--
--   op.opcode_name(0x00)    --> "unreachable"
--   op.opcode_name(0x6a)    --> "i32.add"
--   op.opcode_name(0x99)    --> "unknown_0x99"
--
--   op.is_valid_opcode(0x01) --> true
--   op.is_valid_opcode(0x03) --> true
--   op.is_valid_opcode(0x99) --> false
--
--   local info = op.get_opcode_info(0x28)
--   -- {name="i32.load", operands="memarg(align:u32, offset:u32)"}
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- OPCODES — The Master Table
-- ============================================================================
--
-- Each entry maps:   byte_value → {name, operands}
--
-- The `operands` field is a descriptive string explaining what immediate bytes
-- (if any) follow the opcode in the binary encoding. This is documentation for
-- humans, not a machine-parseable format.
--
-- Operand notation conventions:
--   "none"                    — no immediates
--   "blocktype"               — a block type (0x40 or a ValType byte)
--   "label:u32"               — a label index as unsigned LEB128
--   "func_idx:u32"            — a function index as unsigned LEB128
--   "local_idx:u32"           — a local variable index as unsigned LEB128
--   "global_idx:u32"          — a global variable index as unsigned LEB128
--   "memarg(align:u32, offset:u32)" — memory alignment and byte offset
--   "i32:i32"                 — a signed 32-bit LEB128 immediate
--   "i64:i64"                 — a signed 64-bit LEB128 immediate
--   "f32:ieee754"             — 4 bytes IEEE 754 single-precision float
--   "f64:ieee754"             — 8 bytes IEEE 754 double-precision float
--   "vec(label:u32)+default:u32" — br_table: vector of labels + default label
--   "type_idx:u32 table_idx:u32" — call_indirect: type and table indices

M.OPCODES = {
    -- ========================================================================
    -- Control Flow Instructions
    -- ========================================================================

    -- unreachable: Trap immediately. Used to mark code that should be
    -- unreachable (e.g., after a noreturn function call). If execution reaches
    -- this instruction, it raises a runtime trap.
    [0x00] = { name = "unreachable", operands = "none" },

    -- nop: No operation. Does nothing, consumes no stack values, produces none.
    -- Useful as a placeholder during code generation.
    [0x01] = { name = "nop", operands = "none" },

    -- block: Begin a "block" structured control instruction.
    -- A block introduces a new label. Exiting the block (via end or br) is
    -- a forward branch. Blocks can produce values (determined by their blocktype).
    [0x02] = { name = "block", operands = "blocktype" },

    -- loop: Begin a "loop" structured control instruction.
    -- Like block, but the label refers to the BEGINNING of the loop body.
    -- Branching to a loop label re-executes the loop from the start.
    [0x03] = { name = "loop", operands = "blocktype" },

    -- if: Begin a conditional block. Pops an i32 condition from the stack.
    -- If nonzero, executes the "then" arm; if zero, jumps to else/end.
    [0x04] = { name = "if", operands = "blocktype" },

    -- else: Marks the beginning of the "else" arm of an if instruction.
    -- Must appear between if and end.
    [0x05] = { name = "else", operands = "none" },

    -- end: Terminates a block, loop, if, or else. Also terminates a function
    -- body (the implicit block wrapping the entire function).
    [0x0b] = { name = "end", operands = "none" },

    -- br: Unconditional branch to a label. The label index is a depth value:
    -- 0 means the immediately enclosing block, 1 means the one outside that, etc.
    [0x0c] = { name = "br", operands = "label:u32" },

    -- br_if: Conditional branch. Pops an i32 condition. Branches to the label
    -- if the condition is nonzero; falls through otherwise.
    [0x0d] = { name = "br_if", operands = "label:u32" },

    -- br_table: Indexed branch. Pops an i32 index, branches to labels[index]
    -- if in range, or to the default label otherwise.
    [0x0e] = { name = "br_table", operands = "vec(label:u32)+default:u32" },

    -- return: Return from the current function. Pops the function's result
    -- values from the stack (as defined by the function's type).
    [0x0f] = { name = "return", operands = "none" },

    -- call: Direct function call. Pops argument values, pushes result values.
    -- The function index is the immediate.
    [0x10] = { name = "call", operands = "func_idx:u32" },

    -- call_indirect: Indirect call through a function table. The type index
    -- gives the expected function signature; the table index identifies which
    -- table to look in. Pops the element index, then pops arguments.
    [0x11] = { name = "call_indirect", operands = "type_idx:u32 table_idx:u32" },

    -- ========================================================================
    -- Parametric Instructions
    -- ========================================================================

    -- drop: Discard the top value on the value stack.
    -- Useful for calling functions whose return values you don't need.
    [0x1a] = { name = "drop", operands = "none" },

    -- select: Conditional selection.
    -- Stack before: [..., val1, val2, condition:i32]
    -- Stack after:  [..., val1]  if condition != 0
    --               [..., val2]  if condition == 0
    -- (Note: val1 and val2 must have the same type.)
    [0x1b] = { name = "select", operands = "none" },

    -- ========================================================================
    -- Variable Instructions
    -- ========================================================================

    -- local.get: Push the value of local variable at the given index.
    -- Local variables include function parameters (indices 0..n-1) followed
    -- by locally declared variables (indices n..m).
    [0x20] = { name = "local.get", operands = "local_idx:u32" },

    -- local.set: Pop the top of the stack and store it in a local variable.
    [0x21] = { name = "local.set", operands = "local_idx:u32" },

    -- local.tee: Like local.set, but also leaves the value on the stack.
    -- ("Tee" is named after a T-pipe fitting that splits one flow into two.)
    [0x22] = { name = "local.tee", operands = "local_idx:u32" },

    -- global.get: Push the value of a global variable.
    [0x23] = { name = "global.get", operands = "global_idx:u32" },

    -- global.set: Pop and store in a mutable global variable.
    -- (Immutable globals cannot be the target of global.set.)
    [0x24] = { name = "global.set", operands = "global_idx:u32" },

    -- ========================================================================
    -- Memory Load Instructions
    -- ========================================================================
    --
    -- All load instructions take a memory argument: alignment hint + byte offset.
    --   alignment: log₂ of the alignment (e.g., 2 means 4-byte aligned)
    --   offset: static byte offset added to the runtime address
    --
    -- The WebAssembly spec guarantees that linear memory access outside the
    -- allocated bounds causes a trap.

    -- i32.load: Load 4 bytes from memory, interpret as i32.
    [0x28] = { name = "i32.load", operands = "memarg(align:u32, offset:u32)" },

    -- i64.load: Load 8 bytes from memory, interpret as i64.
    [0x29] = { name = "i64.load", operands = "memarg(align:u32, offset:u32)" },

    -- f32.load: Load 4 bytes from memory, interpret as IEEE 754 f32.
    [0x2a] = { name = "f32.load", operands = "memarg(align:u32, offset:u32)" },

    -- f64.load: Load 8 bytes from memory, interpret as IEEE 754 f64.
    [0x2b] = { name = "f64.load", operands = "memarg(align:u32, offset:u32)" },

    -- i32.load8_s: Load 1 byte, sign-extend to i32.
    [0x2c] = { name = "i32.load8_s", operands = "memarg(align:u32, offset:u32)" },

    -- i32.load8_u: Load 1 byte, zero-extend to i32.
    [0x2d] = { name = "i32.load8_u", operands = "memarg(align:u32, offset:u32)" },

    -- i32.load16_s: Load 2 bytes, sign-extend to i32.
    [0x2e] = { name = "i32.load16_s", operands = "memarg(align:u32, offset:u32)" },

    -- i32.load16_u: Load 2 bytes, zero-extend to i32.
    [0x2f] = { name = "i32.load16_u", operands = "memarg(align:u32, offset:u32)" },

    -- i64.load8_s: Load 1 byte, sign-extend to i64.
    [0x30] = { name = "i64.load8_s", operands = "memarg(align:u32, offset:u32)" },

    -- i64.load8_u: Load 1 byte, zero-extend to i64.
    [0x31] = { name = "i64.load8_u", operands = "memarg(align:u32, offset:u32)" },

    -- ========================================================================
    -- Memory Store Instructions
    -- ========================================================================

    -- i32.store: Pop i32, pop address, write 4 bytes to memory.
    [0x36] = { name = "i32.store", operands = "memarg(align:u32, offset:u32)" },

    -- i32.store8: Write only the low 8 bits of an i32 to memory.
    [0x3a] = { name = "i32.store8", operands = "memarg(align:u32, offset:u32)" },

    -- i32.store16: Write only the low 16 bits of an i32 to memory.
    [0x3b] = { name = "i32.store16", operands = "memarg(align:u32, offset:u32)" },

    -- ========================================================================
    -- Memory Size Instructions
    -- ========================================================================

    -- memory.size: Push the current size of linear memory in pages (64 KiB each).
    -- Immediate: 0x00 (reserved, must be zero in MVP).
    [0x3f] = { name = "memory.size", operands = "reserved:u8" },

    -- memory.grow: Grow linear memory by the given number of pages.
    -- Pops the delta (number of pages to add), pushes the old size or -1 on failure.
    [0x40] = { name = "memory.grow", operands = "reserved:u8" },

    -- ========================================================================
    -- i32 Numeric Instructions
    -- ========================================================================

    -- i32.const: Push an i32 constant onto the stack.
    -- The immediate is a signed 32-bit LEB128 integer.
    [0x41] = { name = "i32.const", operands = "i32:i32" },

    -- i32.eqz: Test if top of stack (i32) is zero. Push 1 if zero, 0 otherwise.
    -- This is the only unary comparison; there is no i32.nez.
    [0x45] = { name = "i32.eqz", operands = "none" },

    -- i32.eq: Pop two i32s, push 1 if equal, 0 otherwise.
    [0x46] = { name = "i32.eq", operands = "none" },

    -- i32.ne: Pop two i32s, push 1 if not equal, 0 otherwise.
    [0x47] = { name = "i32.ne", operands = "none" },

    -- i32.lt_s: Less-than comparison treating both operands as signed.
    [0x48] = { name = "i32.lt_s", operands = "none" },

    -- i32.gt_s: Greater-than comparison treating both operands as signed.
    [0x4A] = { name = "i32.gt_s", operands = "none" },

    -- i32.add: Integer addition. Wraps on overflow (modular 2³²).
    [0x6a] = { name = "i32.add", operands = "none" },

    -- i32.sub: Integer subtraction. Wraps on underflow.
    [0x6b] = { name = "i32.sub", operands = "none" },

    -- i32.mul: Integer multiplication. Wraps on overflow.
    [0x6c] = { name = "i32.mul", operands = "none" },

    -- i32.div_s: Signed integer division. Traps on divide-by-zero or overflow
    -- (INT_MIN / -1 overflows in two's complement).
    [0x6d] = { name = "i32.div_s", operands = "none" },

    -- i32.and: Bitwise AND.
    [0x71] = { name = "i32.and", operands = "none" },

    -- i32.or: Bitwise OR.
    [0x72] = { name = "i32.or", operands = "none" },

    -- i32.xor: Bitwise exclusive OR.
    [0x73] = { name = "i32.xor", operands = "none" },

    -- i32.shl: Left shift. The shift amount is taken modulo 32.
    [0x74] = { name = "i32.shl", operands = "none" },

    -- i32.shr_s: Arithmetic (sign-extending) right shift. Preserves the sign bit.
    [0x75] = { name = "i32.shr_s", operands = "none" },

    -- ========================================================================
    -- i64 Numeric Instructions
    -- ========================================================================

    -- i64.const: Push an i64 constant. Immediate: signed 64-bit LEB128.
    [0x42] = { name = "i64.const", operands = "i64:i64" },

    -- i64.add: 64-bit wrapping addition.
    [0x7c] = { name = "i64.add", operands = "none" },

    -- i64.sub: 64-bit wrapping subtraction.
    [0x7d] = { name = "i64.sub", operands = "none" },

    -- i64.mul: 64-bit wrapping multiplication.
    [0x7e] = { name = "i64.mul", operands = "none" },

    -- ========================================================================
    -- f32 Numeric Instructions
    -- ========================================================================

    -- f32.const: Push an f32 constant. Immediate: 4 bytes IEEE 754.
    [0x43] = { name = "f32.const", operands = "f32:ieee754" },

    -- f32.add: IEEE 754 single-precision addition.
    [0x92] = { name = "f32.add", operands = "none" },

    -- f32.sub: IEEE 754 single-precision subtraction.
    [0x93] = { name = "f32.sub", operands = "none" },

    -- f32.mul: IEEE 754 single-precision multiplication.
    [0x94] = { name = "f32.mul", operands = "none" },

    -- ========================================================================
    -- f64 Numeric Instructions
    -- ========================================================================

    -- f64.const: Push an f64 constant. Immediate: 8 bytes IEEE 754.
    [0x44] = { name = "f64.const", operands = "f64:ieee754" },

    -- f64.add: IEEE 754 double-precision addition.
    [0xa0] = { name = "f64.add", operands = "none" },

    -- f64.sub: IEEE 754 double-precision subtraction.
    [0xa1] = { name = "f64.sub", operands = "none" },

    -- f64.mul: IEEE 754 double-precision multiplication.
    [0xa2] = { name = "f64.mul", operands = "none" },

    -- ========================================================================
    -- Conversion Instructions
    -- ========================================================================
    --
    -- WebAssembly provides explicit conversion instructions because there are
    -- no implicit type coercions. You must always be explicit about how you
    -- convert between types.

    -- i32.wrap_i64: Discard the high 32 bits of an i64, producing an i32.
    -- This is "wrap" because the value wraps modulo 2³².
    [0xa7] = { name = "i32.wrap_i64", operands = "none" },

    -- i32.trunc_f32_s: Convert f32 to i32, rounding toward zero (truncation).
    -- Signed interpretation. Traps if the value is out of i32 range or NaN.
    [0xa8] = { name = "i32.trunc_f32_s", operands = "none" },

    -- i64.extend_i32_s: Sign-extend an i32 to i64.
    -- The high 32 bits are filled with copies of the i32's sign bit.
    [0xac] = { name = "i64.extend_i32_s", operands = "none" },

    -- f32.demote_f64: Convert f64 to f32, losing precision if necessary.
    -- Values too large become ±inf; NaN is preserved (possibly with changed payload).
    [0xb6] = { name = "f32.demote_f64", operands = "none" },

    -- f64.promote_f32: Convert f32 to f64 without loss of precision.
    -- Every representable f32 value is exactly representable as f64.
    [0xbb] = { name = "f64.promote_f32", operands = "none" },
}

-- ============================================================================
-- opcode_name(byte) — Get the mnemonic name for an opcode
-- ============================================================================

--- Return the human-readable name of a WebAssembly opcode byte.
--
-- For recognized opcodes, returns the standard mnemonic (e.g., "i32.add").
-- For unrecognized bytes, returns "unknown_0xXX" where XX is the hex value.
--
-- @param byte  Integer opcode byte (0x00–0xFF).
-- @return      String mnemonic or "unknown_0xXX".
function M.opcode_name(byte)
    local entry = M.OPCODES[byte]
    if entry then
        return entry.name
    end
    return string.format("unknown_0x%02x", byte)
end

-- ============================================================================
-- is_valid_opcode(byte) — Check if a byte is a known opcode
-- ============================================================================

--- Return true if `byte` is a recognized WebAssembly opcode.
--
-- @param byte  Integer to test.
-- @return      true if the byte maps to a known opcode; false otherwise.
function M.is_valid_opcode(byte)
    return M.OPCODES[byte] ~= nil
end

-- ============================================================================
-- get_opcode_info(byte) — Retrieve full opcode metadata
-- ============================================================================

--- Return the full info table for an opcode, or nil if not recognized.
--
-- The returned table has the fields:
--   name     — string mnemonic (e.g., "i32.add")
--   operands — string describing immediates (e.g., "none", "local_idx:u32")
--
-- Returns nil for unrecognized bytes.
--
-- @param byte  Integer opcode byte.
-- @return      Table {name, operands} or nil.
function M.get_opcode_info(byte)
    return M.OPCODES[byte]
end

return M

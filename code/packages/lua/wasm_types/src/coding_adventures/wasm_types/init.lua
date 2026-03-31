-- ============================================================================
-- wasm_types — WebAssembly value types and fundamental type definitions
-- ============================================================================
--
-- WebAssembly (Wasm) is a binary instruction format that serves as a portable
-- compilation target for languages like C, C++, Rust, and Go. Understanding
-- its type system is fundamental to implementing any Wasm tool (compiler,
-- interpreter, verifier, linker).
--
-- ## The Wasm Type System at a Glance
--
-- WebAssembly is a STATICALLY typed language. Every value has a type known at
-- compile time, and every function signature is checked during module
-- validation. The type system is intentionally minimal:
--
--   INTEGER TYPES:
--     i32 — 32-bit integer (used for both signed and unsigned operations)
--     i64 — 64-bit integer
--
--   FLOATING-POINT TYPES:
--     f32 — 32-bit IEEE 754 float
--     f64 — 64-bit IEEE 754 float
--
--   VECTOR TYPE:
--     v128 — 128-bit SIMD vector (added in the SIMD proposal)
--
--   REFERENCE TYPES:
--     funcref   — reference to a function (opaque; can be null)
--     externref — reference to an external/host object (opaque; can be null)
--
-- ## Binary Encoding
--
-- In the WebAssembly binary format, types are encoded as single bytes:
--
--   i32      0x7F   (decimal 127, as a signed byte: -1)
--   i64      0x7E   (decimal 126, as a signed byte: -2)
--   f32      0x7D   (decimal 125, as a signed byte: -3)
--   f64      0x7C   (decimal 124, as a signed byte: -4)
--   v128     0x7B   (decimal 123, as a signed byte: -5)
--   funcref  0x70   (decimal 112, as a signed byte: -16)
--   externref 0x6F  (decimal 111, as a signed byte: -17)
--
-- This "negative in sign-extended form" pattern is deliberate: Wasm's binary
-- format uses signed LEB128 for type codes, and these values were chosen to
-- encode as single bytes in signed LEB128 (values -1 through -64 do).
--
-- ## Composite Types
--
-- Beyond individual value types, WebAssembly defines several compound types:
--
--   FuncType   — maps a tuple of parameter types to a tuple of result types
--                encoded with a 0x60 prefix byte
--   Limits     — a range [min, max] used for memory and table sizing
--   MemType    — wraps Limits for linear memory
--   TableType  — wraps a RefType + Limits for element tables
--   GlobalType — a ValType with a mutability flag
--
-- ## Dependency: wasm_leb128
--
-- This module uses LEB128 (Little-Endian Base-128) variable-length encoding
-- for integer values within type encodings (e.g., parameter counts, limit
-- values). See coding_adventures.wasm_leb128 for details on that encoding.
--
-- ## Usage
--
--   local wt = require("coding_adventures.wasm_types")
--
--   -- Check if a byte is a valid value type
--   wt.is_val_type(0x7F)  --> true   (i32)
--   wt.is_val_type(0x42)  --> false  (not a type byte)
--
--   -- Get a human-readable name
--   wt.val_type_name(0x7E) --> "i64"
--
--   -- Encode/decode a value type
--   wt.encode_val_type(wt.ValType.i32)  --> {0x7F}
--   wt.decode_val_type({0x7F}, 1)       --> {type=0x7F, bytes_consumed=1}
--
--   -- Encode/decode limits
--   wt.encode_limits({min=0, max=nil})      --> {0x00, 0x00}
--   wt.encode_limits({min=1, max=16})       --> {0x01, 0x01, 0x10}
--   wt.decode_limits({0x00, 0x00}, 1)       --> {limits={min=0, max=nil}, bytes_consumed=2}
--
--   -- Encode/decode a function type
--   local ft = {params={wt.ValType.i32, wt.ValType.i32}, results={wt.ValType.i64}}
--   wt.encode_func_type(ft)  --> {0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E}
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Dependencies
-- ============================================================================

local leb128 = require("coding_adventures.wasm_leb128")

-- ============================================================================
-- ValType — WebAssembly Value Types
-- ============================================================================
--
-- These are the byte codes used in the WebAssembly binary format to identify
-- value types. They appear in function type signatures, global type
-- definitions, table element types, and block type annotations.
--
-- The encoding follows the signed LEB128 convention of the WebAssembly spec:
-- these byte values are in the range 0x6F–0x7F (111–127 decimal), which all
-- encode as single-byte signed LEB128.

--- ValType table: maps human-readable names to their byte values.
--
-- Usage:
--   wt.ValType.i32      -- 0x7F
--   wt.ValType.funcref  -- 0x70
M.ValType = {
    i32      = 0x7F,  -- 32-bit integer
    i64      = 0x7E,  -- 64-bit integer
    f32      = 0x7D,  -- 32-bit IEEE 754 float
    f64      = 0x7C,  -- 64-bit IEEE 754 float
    v128     = 0x7B,  -- 128-bit SIMD vector
    funcref  = 0x70,  -- function reference (opaque, nullable)
    externref = 0x6F, -- external/host reference (opaque, nullable)
}

-- ============================================================================
-- RefType — Reference Types (subset of ValType)
-- ============================================================================
--
-- Reference types are a subset of value types that represent opaque
-- references. They are used in tables and as operands to reference
-- instructions. In the current spec there are exactly two:
--   funcref  — a reference to a function (may be null)
--   externref — a reference to a host object (may be null)

--- RefType table: the two reference types.
M.RefType = {
    funcref  = 0x70,
    externref = 0x6F,
}

-- ============================================================================
-- BlockType — Block Type Encoding
-- ============================================================================
--
-- WebAssembly structured control instructions (block, loop, if) carry a
-- "block type" annotation that describes what the block consumes and produces.
--
-- There are two cases:
--   1. Empty (void): encoded as 0x40. The block neither consumes nor produces
--      any values on the value stack.
--   2. A single ValType: encoded as the value type byte. The block produces
--      exactly one value of that type (and typically consumes zero).
--
-- (The multi-value proposal added type indices to block types, but that is not
-- covered here — this module focuses on the single-value case.)

--- BlockType.empty: the void/epsilon block type (0x40).
M.BlockType = {
    empty = 0x40,  -- no values (also called "epsilon" in the spec)
}

-- ============================================================================
-- ExternType — External Type Codes
-- ============================================================================
--
-- The "external" type codes appear in the import and export sections of a
-- WebAssembly module. They identify what kind of entity is being imported
-- or exported.

--- ExternType codes as used in import/export section encodings.
M.ExternType = {
    func   = 0,  -- a function
    table  = 1,  -- a table
    mem    = 2,  -- a linear memory
    global = 3,  -- a global variable
}

-- ============================================================================
-- is_val_type(byte) — Predicate: is this byte a valid ValType?
-- ============================================================================

--- Return true if `byte` is a valid WebAssembly value type byte.
--
-- Valid value type bytes are: 0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x70, 0x6F
--
-- @param byte  Integer to test.
-- @return      true if it is a valid value type; false otherwise.
function M.is_val_type(byte)
    return byte == 0x7F  -- i32
        or byte == 0x7E  -- i64
        or byte == 0x7D  -- f32
        or byte == 0x7C  -- f64
        or byte == 0x7B  -- v128
        or byte == 0x70  -- funcref
        or byte == 0x6F  -- externref
end

-- ============================================================================
-- is_ref_type(byte) — Predicate: is this byte a valid RefType?
-- ============================================================================

--- Return true if `byte` is a valid WebAssembly reference type byte.
--
-- Reference types are the subset of value types that represent opaque
-- references: funcref (0x70) and externref (0x6F).
--
-- @param byte  Integer to test.
-- @return      true if it is funcref or externref; false otherwise.
function M.is_ref_type(byte)
    return byte == 0x70  -- funcref
        or byte == 0x6F  -- externref
end

-- ============================================================================
-- val_type_name(byte) — Human-readable name for a ValType byte
-- ============================================================================

--- Return the string name of a WebAssembly value type.
--
-- Examples:
--   val_type_name(0x7F) --> "i32"
--   val_type_name(0x70) --> "funcref"
--   val_type_name(0x42) --> "unknown_0x42"
--
-- @param byte  The value type byte code.
-- @return      A human-readable string name.
function M.val_type_name(byte)
    local names = {
        [0x7F] = "i32",
        [0x7E] = "i64",
        [0x7D] = "f32",
        [0x7C] = "f64",
        [0x7B] = "v128",
        [0x70] = "funcref",
        [0x6F] = "externref",
    }
    return names[byte] or string.format("unknown_0x%02x", byte)
end

-- ============================================================================
-- encode_val_type(val_type) — Encode a ValType as a single-byte array
-- ============================================================================

--- Encode a value type byte into a 1-element byte array.
--
-- In the WebAssembly binary format, a value type is always encoded as a single
-- byte. This function validates the input and wraps it in an array.
--
-- @param val_type  A valid ValType byte (e.g., 0x7F for i32).
-- @return          A 1-element array containing the byte, e.g., {0x7F}.
-- @error           If val_type is not a recognized value type byte.
function M.encode_val_type(val_type)
    if not M.is_val_type(val_type) then
        error(string.format("wasm_types.encode_val_type: invalid val_type 0x%02x", val_type))
    end
    return {val_type}
end

-- ============================================================================
-- decode_val_type(bytes, offset) — Decode a ValType from a byte array
-- ============================================================================

--- Decode a value type from a byte array starting at `offset` (1-based).
--
-- Reads exactly one byte and validates it as a value type.
--
-- @param bytes   Array of byte values.
-- @param offset  1-based starting index (defaults to 1).
-- @return        Table {type=<byte>, bytes_consumed=1}.
-- @error         If offset is out of range or the byte is not a valid ValType.
function M.decode_val_type(bytes, offset)
    offset = offset or 1
    if offset > #bytes then
        error("wasm_types.decode_val_type: offset out of range")
    end
    local byte = bytes[offset]
    if not M.is_val_type(byte) then
        error(string.format("wasm_types.decode_val_type: invalid val_type 0x%02x at offset %d", byte, offset))
    end
    return { type = byte, bytes_consumed = 1 }
end

-- ============================================================================
-- encode_limits(limits) — Encode a Limits structure as a byte array
-- ============================================================================
--
-- A Limits structure specifies the minimum (and optionally maximum) size of
-- a memory or table. In the binary format:
--
--   If no maximum:  0x00 followed by min (unsigned LEB128)
--   If maximum:     0x01 followed by min (unsigned LEB128), max (unsigned LEB128)
--
-- ### Example
--
--   encode_limits({min=0})           → {0x00, 0x00}
--   encode_limits({min=1, max=nil})  → {0x00, 0x01}
--   encode_limits({min=1, max=16})   → {0x01, 0x01, 0x10}
--
-- The min and max values represent page counts (for memory, 1 page = 64 KiB)
-- or element counts (for tables).

--- Encode a limits structure as a byte array.
--
-- @param limits  Table with fields:
--                  min (integer, required) — minimum size
--                  max (integer or nil, optional) — maximum size; nil means unbounded
-- @return        Byte array.
function M.encode_limits(limits)
    local result = {}

    if limits.max == nil then
        -- No maximum: flag byte 0x00 + min as unsigned LEB128
        result[#result + 1] = 0x00
        local min_bytes = leb128.encode_unsigned(limits.min)
        for _, b in ipairs(min_bytes) do
            result[#result + 1] = b
        end
    else
        -- Has maximum: flag byte 0x01 + min + max, both unsigned LEB128
        result[#result + 1] = 0x01
        local min_bytes = leb128.encode_unsigned(limits.min)
        for _, b in ipairs(min_bytes) do
            result[#result + 1] = b
        end
        local max_bytes = leb128.encode_unsigned(limits.max)
        for _, b in ipairs(max_bytes) do
            result[#result + 1] = b
        end
    end

    return result
end

-- ============================================================================
-- decode_limits(bytes, offset) — Decode a Limits structure from a byte array
-- ============================================================================

--- Decode a limits structure from a byte array starting at `offset` (1-based).
--
-- Reads a flag byte:
--   0x00 → unbounded (no max): read one LEB128 integer for min
--   0x01 → bounded (has max): read two LEB128 integers (min, max)
--
-- @param bytes   Array of byte values.
-- @param offset  1-based starting index (defaults to 1).
-- @return        Table {limits={min=N, max=M|nil}, bytes_consumed=K}.
-- @error         If the flag byte is not 0x00 or 0x01.
function M.decode_limits(bytes, offset)
    offset = offset or 1
    local consumed = 0

    -- Read flag byte
    local flag = bytes[offset + consumed]
    consumed = consumed + 1

    if flag == 0x00 then
        -- No maximum: read min
        local min_val, min_count = leb128.decode_unsigned(bytes, offset + consumed)
        consumed = consumed + min_count
        return {
            limits = { min = min_val, max = nil },
            bytes_consumed = consumed,
        }
    elseif flag == 0x01 then
        -- Has maximum: read min then max
        local min_val, min_count = leb128.decode_unsigned(bytes, offset + consumed)
        consumed = consumed + min_count
        local max_val, max_count = leb128.decode_unsigned(bytes, offset + consumed)
        consumed = consumed + max_count
        return {
            limits = { min = min_val, max = max_val },
            bytes_consumed = consumed,
        }
    else
        error(string.format("wasm_types.decode_limits: invalid limits flag 0x%02x", flag))
    end
end

-- ============================================================================
-- encode_func_type(func_type) — Encode a function type signature
-- ============================================================================
--
-- A function type (also called a "type entry" in the Wasm type section) maps
-- a list of parameter types to a list of result types. In the binary format:
--
--   0x60                           -- magic prefix for function types
--   param_count (unsigned LEB128)  -- number of parameters
--   param_type_1, param_type_2, …  -- each a single byte ValType
--   result_count (unsigned LEB128) -- number of results
--   result_type_1, result_type_2, … -- each a single byte ValType
--
-- ### Example: (i32, i32) → i64
--
--   {params={0x7F, 0x7F}, results={0x7E}}
--   encodes as: {0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E}
--
-- ### Why 0x60?
--
-- The byte 0x60 was chosen because it doesn't conflict with any ValType byte.
-- It serves as a "this is a function type" sentinel in type section entries.

--- Encode a function type as a byte array.
--
-- @param func_type  Table with fields:
--                     params  (array of ValType bytes)
--                     results (array of ValType bytes)
-- @return           Byte array starting with 0x60.
function M.encode_func_type(func_type)
    local result = {0x60}  -- function type magic byte

    -- Parameter count (unsigned LEB128) then each param type (1 byte each)
    local param_count_bytes = leb128.encode_unsigned(#func_type.params)
    for _, b in ipairs(param_count_bytes) do result[#result + 1] = b end
    for _, vt in ipairs(func_type.params) do result[#result + 1] = vt end

    -- Result count (unsigned LEB128) then each result type (1 byte each)
    local result_count_bytes = leb128.encode_unsigned(#func_type.results)
    for _, b in ipairs(result_count_bytes) do result[#result + 1] = b end
    for _, vt in ipairs(func_type.results) do result[#result + 1] = vt end

    return result
end

-- ============================================================================
-- decode_func_type(bytes, offset) — Decode a function type signature
-- ============================================================================

--- Decode a function type from a byte array starting at `offset` (1-based).
--
-- Expects the byte at `offset` to be 0x60 (the function type magic byte),
-- followed by parameter count (LEB128), param types, result count (LEB128),
-- result types.
--
-- @param bytes   Array of byte values.
-- @param offset  1-based starting index (defaults to 1).
-- @return        Table {func_type={params=[...], results=[...]}, bytes_consumed=K}.
-- @error         If the first byte is not 0x60, or if invalid ValType bytes found.
function M.decode_func_type(bytes, offset)
    offset = offset or 1
    local consumed = 0

    -- Validate the function type magic byte
    local magic = bytes[offset + consumed]
    consumed = consumed + 1
    if magic ~= 0x60 then
        error(string.format("wasm_types.decode_func_type: expected 0x60, got 0x%02x", magic))
    end

    -- Read param count
    local param_count, pc = leb128.decode_unsigned(bytes, offset + consumed)
    consumed = consumed + pc

    -- Read each param type (one byte each)
    local params = {}
    for _ = 1, param_count do
        local vt = bytes[offset + consumed]
        consumed = consumed + 1
        if not M.is_val_type(vt) then
            error(string.format("wasm_types.decode_func_type: invalid param type 0x%02x", vt))
        end
        params[#params + 1] = vt
    end

    -- Read result count
    local result_count, rc = leb128.decode_unsigned(bytes, offset + consumed)
    consumed = consumed + rc

    -- Read each result type
    local results = {}
    for _ = 1, result_count do
        local vt = bytes[offset + consumed]
        consumed = consumed + 1
        if not M.is_val_type(vt) then
            error(string.format("wasm_types.decode_func_type: invalid result type 0x%02x", vt))
        end
        results[#results + 1] = vt
    end

    return {
        func_type = { params = params, results = results },
        bytes_consumed = consumed,
    }
end

return M

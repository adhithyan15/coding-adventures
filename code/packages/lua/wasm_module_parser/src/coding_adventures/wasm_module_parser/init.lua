-- ============================================================================
-- wasm_module_parser — WebAssembly binary module parser
-- ============================================================================
--
-- WebAssembly (Wasm) is a binary instruction format designed as a portable
-- compilation target for languages like C, C++, Rust, and Go. It runs in web
-- browsers, on servers (via WASI), and embedded environments.
--
-- This module parses the BINARY format of a .wasm file into a structured Lua
-- table. It implements the module binary structure defined at:
--   https://webassembly.github.io/spec/core/binary/modules.html
--
-- ## The Wasm Binary Format — High-Level View
--
-- A Wasm binary file looks like this:
--
--   ┌─────────────────────────────────────────────────────────────────────┐
--   │  MAGIC NUMBER  │  VERSION  │  SECTION 0  │  SECTION 1  │  ...      │
--   │  4 bytes       │  4 bytes  │  variable   │  variable   │           │
--   └─────────────────────────────────────────────────────────────────────┘
--
-- Magic number: the bytes 0x00 0x61 0x73 0x6D, which as ASCII reads "\0asm".
-- This is a standard "magic bytes" pattern used by file format parsers to
-- quickly verify they have the right kind of file.
--
-- Version: the 4-byte little-endian integer 0x00000001. This is version 1 of
-- the Wasm binary format, which is the current standard.
--
-- After the 8-byte header, the file contains zero or more SECTIONS.
--
-- ## Sections
--
-- Each section has the structure:
--
--   ┌──────────────┬─────────────────────┬───────────────────────────────┐
--   │  Section ID  │  Content Length     │  Content bytes                │
--   │  1 byte      │  unsigned LEB128    │  (length bytes)               │
--   └──────────────┴─────────────────────┴───────────────────────────────┘
--
-- The section ID is a single byte from 0 to 11. Each ID corresponds to a
-- specific kind of information (types, imports, function bodies, etc.).
-- The content length tells us how many bytes to read for this section's data.
--
-- ## Section IDs
--
--   ID  Name        Purpose
--   ──  ──────────  ──────────────────────────────────────────────────────
--    0  Custom      Arbitrary name+data; tooling metadata, debug info, etc.
--    1  Type        Function type signatures: param types → result types
--    2  Import      External symbols: functions, tables, memories, globals
--    3  Function    Maps each function to a type signature (by type index)
--    4  Table       Tables of references (used with indirect function calls)
--    5  Memory      Linear memory specifications (size limits)
--    6  Global      Global variables with their types and initial values
--    7  Export      Symbols exported to the host (function, table, mem, global)
--    8  Start       Index of the "start function" called on module load
--    9  Element     Initialization data for tables
--   10  Code        Function bodies (locals + bytecode instructions)
--   11  Data        Initialization data for linear memory
--
-- ## LEB128 Integers
--
-- The WebAssembly binary format uses LEB128 (Little-Endian Base 128) for most
-- integer values. LEB128 is a variable-length encoding where:
-- - Small numbers (0–127) take 1 byte
-- - Larger numbers take more bytes
-- - The high bit of each byte is a "continuation bit" (1 = more bytes follow)
--
-- This module delegates LEB128 decoding to coding_adventures.wasm_leb128.
--
-- ## Usage
--
--   local parser = require("coding_adventures.wasm_module_parser")
--
--   -- Parse a binary string (from file or constructed in code)
--   local module = parser.parse("\0asm\1\0\0\0")  -- minimal empty module
--
--   -- Access parsed sections
--   print(module.version)        --> 1
--   print(#module.types)         --> 0 (no type section)
--   print(#module.exports)       --> 0 (no export section)
--
--   -- Find functions that return i32
--   for i, ft in ipairs(module.types) do
--     if #ft.results == 1 and ft.results[1] == 0x7F then
--       print("type " .. i .. " returns i32")
--     end
--   end
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Dependencies
-- ============================================================================

local leb128 = require("coding_adventures.wasm_leb128")
local wt     = require("coding_adventures.wasm_types")

-- ============================================================================
-- Module-level constants
-- ============================================================================

-- MODULE_MAGIC: the 4-byte magic number that starts every valid Wasm binary.
-- "\0asm" in ASCII. The null byte prefix ensures it can't be confused with
-- plain text; the "asm" part gives humans a hint about the file type.
M.MODULE_MAGIC = "\0asm"

-- MODULE_VERSION: the Wasm binary format version. Currently always 1.
-- Stored as 4 little-endian bytes: 0x01 0x00 0x00 0x00.
M.MODULE_VERSION = 1

-- ============================================================================
-- Section ID constants
--
-- These numeric codes appear as the first byte of each section. They identify
-- what kind of data the section contains.
-- ============================================================================

M.SECTION_CUSTOM   = 0   -- Custom section (name + arbitrary bytes)
M.SECTION_TYPE     = 1   -- Type section (function signatures)
M.SECTION_IMPORT   = 2   -- Import section (external dependencies)
M.SECTION_FUNCTION = 3   -- Function section (type indices for functions)
M.SECTION_TABLE    = 4   -- Table section (reference table specs)
M.SECTION_MEMORY   = 5   -- Memory section (linear memory specs)
M.SECTION_GLOBAL   = 6   -- Global section (global variable definitions)
M.SECTION_EXPORT   = 7   -- Export section (public interface)
M.SECTION_START    = 8   -- Start section (startup function index)
M.SECTION_ELEMENT  = 9   -- Element section (table initialization data)
M.SECTION_CODE     = 10  -- Code section (function bodies)
M.SECTION_DATA     = 11  -- Data section (memory initialization data)

-- Human-readable names for section IDs (useful for error messages)
local SECTION_NAMES = {
    [0]  = "custom",
    [1]  = "type",
    [2]  = "import",
    [3]  = "function",
    [4]  = "table",
    [5]  = "memory",
    [6]  = "global",
    [7]  = "export",
    [8]  = "start",
    [9]  = "element",
    [10] = "code",
    [11] = "data",
}

-- ============================================================================
-- bytes_from_string(s) — convert binary string to integer array
--
-- The wasm_leb128 module operates on Lua arrays of integers (where each
-- element is a byte value 0–255). The WebAssembly binary format is naturally
-- handled as a binary string. This helper converts the binary string to an
-- integer array once at the start of parsing.
--
-- Lua's string.byte(s, i) returns the numeric value of the byte at position i
-- (1-based). We use this to build the array.
--
-- Example:
--   bytes_from_string("\0asm\1\0\0\0")
--   --> {0, 97, 115, 109, 1, 0, 0, 0}
--       (0x00='\0', 0x61='a', 0x73='s', 0x6D='m', then version bytes)
-- ============================================================================
local function bytes_from_string(s)
    local t = {}
    for i = 1, #s do
        t[i] = string.byte(s, i)
    end
    return t
end

-- ============================================================================
-- read_bytes(bytes, pos, count) — extract a sub-array of bytes
--
-- Returns a new array containing `count` bytes starting at position `pos`.
-- Also returns the new position after reading.
--
-- This is used when we need to pass a chunk of bytes to a sub-parser or
-- when we want to preserve section content as raw bytes.
-- ============================================================================
local function read_bytes(bytes, pos, count)
    local result = {}
    for i = 1, count do
        result[i] = bytes[pos + i - 1]
    end
    return result, pos + count
end

-- ============================================================================
-- read_string(bytes, pos) — read a LEB128-prefixed UTF-8 string
--
-- In the WebAssembly binary format, names (for imports, exports, custom
-- sections) are encoded as:
--   - A length (unsigned LEB128)
--   - That many bytes of UTF-8 encoded text
--
-- This function reads such a name, returning (string_value, new_pos).
--
-- Example:
--   The export name "add" is encoded as: 0x03, 0x61, 0x64, 0x64
--   (length=3, then 'a'=0x61, 'd'=0x64, 'd'=0x64)
-- ============================================================================
local function read_string(bytes, pos)
    local length, count = leb128.decode_unsigned(bytes, pos)
    pos = pos + count
    local chars = {}
    for i = 1, length do
        chars[i] = string.char(bytes[pos + i - 1])
    end
    pos = pos + length
    return table.concat(chars), pos
end

-- ============================================================================
-- parse_type_section(bytes, pos, end_pos) — decode the Type section
--
-- The Type section lists all the function signatures used in the module.
-- Other sections (Function, Import, Export) refer to these by index.
--
-- Binary layout:
--   count (unsigned LEB128)        — number of type entries
--   For each type entry:
--     0x60                         — "function type" sentinel byte
--     param_count (unsigned LEB128)
--     param_type_1, …              — each a 1-byte ValType
--     result_count (unsigned LEB128)
--     result_type_1, …             — each a 1-byte ValType
--
-- The 0x60 byte was chosen as the function type marker because it doesn't
-- conflict with any ValType byte (which are all >= 0x6F). This allows the
-- type section to potentially include other composite types in future
-- WebAssembly proposals.
--
-- Returns: array of {params=[...], results=[...]} tables, one per type entry.
-- ============================================================================
local function parse_type_section(bytes, pos, end_pos)
    local types = {}

    -- Read the count of type entries
    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for _ = 1, count do
        -- Expect the function type magic byte 0x60
        local marker = bytes[pos]
        if marker ~= 0x60 then
            error(string.format(
                "wasm_module_parser: expected 0x60 (func type) at pos %d, got 0x%02x",
                pos, marker))
        end
        pos = pos + 1

        -- Read parameter types
        local param_count, pc = leb128.decode_unsigned(bytes, pos)
        pos = pos + pc
        local params = {}
        for i = 1, param_count do
            params[i] = bytes[pos]
            pos = pos + 1
        end

        -- Read result types
        local result_count, rc = leb128.decode_unsigned(bytes, pos)
        pos = pos + rc
        local results = {}
        for i = 1, result_count do
            results[i] = bytes[pos]
            pos = pos + 1
        end

        types[#types + 1] = { params = params, results = results }
    end

    return types
end

-- ============================================================================
-- parse_import_section(bytes, pos, end_pos) — decode the Import section
--
-- The Import section lists all external symbols the module needs from its
-- environment. Imports can be functions, tables, memories, or globals.
--
-- Binary layout:
--   count (unsigned LEB128)
--   For each import:
--     module_name (length-prefixed UTF-8 string)
--     field_name  (length-prefixed UTF-8 string)
--     import_desc — a 1-byte type tag followed by type-specific data:
--       0x00: function  → type_index (unsigned LEB128)
--       0x01: table     → ref_type (1 byte), limits
--       0x02: memory    → limits
--       0x03: global    → val_type (1 byte), mutability (1 byte: 0=const, 1=var)
--
-- EXAMPLE — importing `env.memory` as a memory:
--   module = "env", field = "memory", desc = {kind="mem", min=1, max=nil}
--
-- WHY IMPORTS?
-- WebAssembly modules are sandboxed. They cannot access anything outside
-- themselves unless the host explicitly provides it through imports. This is
-- the foundation of Wasm's security model: the module declares what it needs,
-- and the host decides what to allow.
--
-- Returns: array of {mod, name, desc} tables, where desc is:
--   {kind="func", type_idx=N}
--   {kind="table", ref_type=T, limits={min=N, max=M}}
--   {kind="mem", limits={min=N, max=M}}
--   {kind="global", val_type=T, mutable=bool}
-- ============================================================================
local function parse_import_section(bytes, pos, end_pos)
    local imports = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for _ = 1, count do
        -- Read module name and field name
        local mod_name, new_pos = read_string(bytes, pos)
        pos = new_pos
        local field_name
        field_name, new_pos = read_string(bytes, pos)
        pos = new_pos

        -- Read the import descriptor tag
        local tag = bytes[pos]
        pos = pos + 1

        local desc
        if tag == 0x00 then
            -- Function import: refers to a type index in the type section
            local type_idx, tc = leb128.decode_unsigned(bytes, pos)
            pos = pos + tc
            desc = { kind = "func", type_idx = type_idx }

        elseif tag == 0x01 then
            -- Table import: a reference type + limits
            local ref_type = bytes[pos]
            pos = pos + 1
            local lim_result = wt.decode_limits(bytes, pos)
            pos = pos + lim_result.bytes_consumed
            desc = { kind = "table", ref_type = ref_type, limits = lim_result.limits }

        elseif tag == 0x02 then
            -- Memory import: just limits
            local lim_result = wt.decode_limits(bytes, pos)
            pos = pos + lim_result.bytes_consumed
            desc = { kind = "mem", limits = lim_result.limits }

        elseif tag == 0x03 then
            -- Global import: a value type + mutability flag
            local val_type = bytes[pos]
            pos = pos + 1
            local mut = bytes[pos]
            pos = pos + 1
            desc = { kind = "global", val_type = val_type, mutable = (mut == 1) }

        else
            error(string.format(
                "wasm_module_parser: unknown import descriptor tag 0x%02x at pos %d",
                tag, pos - 1))
        end

        imports[#imports + 1] = { mod = mod_name, name = field_name, desc = desc }
    end

    return imports
end

-- ============================================================================
-- parse_function_section(bytes, pos, end_pos) — decode the Function section
--
-- The Function section is a compact array of type indices. Each entry says
-- which function type signature (from the Type section) applies to the
-- corresponding locally-defined function.
--
-- This section does NOT contain the actual code — that's the Code section.
-- Instead, it acts as a "table of contents" that links each function to its
-- type signature.
--
-- Binary layout:
--   count (unsigned LEB128)
--   For each function:
--     type_index (unsigned LEB128)  — index into the Type section
--
-- EXAMPLE: if the Function section contains [2, 0, 1], then:
--   - Local function 0 has type signature types[2]
--   - Local function 1 has type signature types[0]
--   - Local function 2 has type signature types[1]
--
-- Returns: array of type_index integers.
-- ============================================================================
local function parse_function_section(bytes, pos, end_pos)
    local functions = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for i = 1, count do
        local type_idx, tc = leb128.decode_unsigned(bytes, pos)
        pos = pos + tc
        functions[i] = type_idx
    end

    return functions
end

-- ============================================================================
-- parse_table_section(bytes, pos, end_pos) — decode the Table section
--
-- Tables in WebAssembly are arrays of reference types (funcref or externref).
-- They are the mechanism for indirect function calls: instead of calling a
-- function by its compile-time index, you call through a table entry that
-- can be changed at runtime.
--
-- This is how C function pointers are implemented in Wasm.
--
-- Binary layout:
--   count (unsigned LEB128)
--   For each table:
--     ref_type (1 byte)   — 0x70 for funcref, 0x6F for externref
--     limits              — min and optional max element count
--
-- Returns: array of {ref_type=T, limits={min=N, max=M}} tables.
-- ============================================================================
local function parse_table_section(bytes, pos, end_pos)
    local tables = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for i = 1, count do
        local ref_type = bytes[pos]
        pos = pos + 1
        local lim_result = wt.decode_limits(bytes, pos)
        pos = pos + lim_result.bytes_consumed
        tables[i] = { ref_type = ref_type, limits = lim_result.limits }
    end

    return tables
end

-- ============================================================================
-- parse_memory_section(bytes, pos, end_pos) — decode the Memory section
--
-- WebAssembly has a single "linear memory": a resizable, flat byte array
-- accessible via load/store instructions. The Memory section specifies the
-- initial and maximum size of this memory in 64KiB "pages".
--
--   1 page = 65,536 bytes = 64 KiB
--
-- Most Wasm modules have at most one memory (the multi-memory proposal allows
-- more, but this parser handles the common single-memory case).
--
-- Binary layout:
--   count (unsigned LEB128)
--   For each memory:
--     limits (flag byte + LEB128 values)
--       flag 0x00: min only (unbounded growth)
--       flag 0x01: min and max (bounded growth)
--
-- Returns: array of {limits={min=N, max=M}} tables.
-- ============================================================================
local function parse_memory_section(bytes, pos, end_pos)
    local memories = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for i = 1, count do
        local lim_result = wt.decode_limits(bytes, pos)
        pos = pos + lim_result.bytes_consumed
        memories[i] = { limits = lim_result.limits }
    end

    return memories
end

-- ============================================================================
-- parse_init_expr(bytes, pos) — parse a constant expression
--
-- Global initializers and element segment offsets use "constant expressions":
-- short sequences of instructions that evaluate to a single value. These
-- must be constant (no function calls, no control flow).
--
-- Common init expressions:
--   i32.const N  → 0x41, signed_LEB128(N), 0x0B
--   i64.const N  → 0x42, signed_LEB128(N), 0x0B
--   f32.const V  → 0x43, 4 bytes (IEEE 754), 0x0B
--   f64.const V  → 0x44, 8 bytes (IEEE 754), 0x0B
--   global.get I → 0x23, unsigned_LEB128(I), 0x0B
--   ref.null T   → 0xD0, type byte, 0x0B
--   ref.func I   → 0xD2, unsigned_LEB128(I), 0x0B
--
-- All end with 0x0B (the "end" opcode).
--
-- We parse the init expression by collecting bytes until we see the end opcode.
-- This preserves the raw bytes so callers can interpret them as needed.
--
-- Returns: (bytes_array, new_pos) where bytes_array contains the full
-- expression including the terminal 0x0B.
-- ============================================================================
local function parse_init_expr(bytes, pos)
    local expr_bytes = {}

    -- Read bytes until we hit the end opcode (0x0B)
    -- We need to handle the most common cases:
    --   i32.const (0x41) + signed LEB128 + end
    --   i64.const (0x42) + signed LEB128 + end
    --   f32.const (0x43) + 4 raw bytes + end
    --   f64.const (0x44) + 8 raw bytes + end
    --   global.get (0x23) + unsigned LEB128 + end
    -- For robustness, we read byte-by-byte until we see 0x0B.
    -- Note: 0x0B can appear as part of LEB128 values, so this is a
    -- simplification. For a production parser we'd need a full instruction
    -- decoder here. For our purposes (structural parsing) this is sufficient
    -- for well-formed modules.
    while true do
        local b = bytes[pos]
        expr_bytes[#expr_bytes + 1] = b
        pos = pos + 1
        if b == 0x0B then  -- "end" opcode
            break
        end
    end

    return expr_bytes, pos
end

-- ============================================================================
-- parse_global_section(bytes, pos, end_pos) — decode the Global section
--
-- Globals are module-level variables accessible by all functions in the module.
-- They have a type (value type), a mutability flag, and an initial value
-- expressed as a constant expression.
--
-- Binary layout:
--   count (unsigned LEB128)
--   For each global:
--     val_type   (1 byte)  — the type of the global (i32, i64, f32, f64, etc.)
--     mutability (1 byte)  — 0x00 = const (immutable), 0x01 = var (mutable)
--     init_expr  — constant expression giving the initial value
--
-- Returns: array of {val_type=T, mutable=bool, init_expr=[...]} tables.
-- ============================================================================
local function parse_global_section(bytes, pos, end_pos)
    local globals = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for i = 1, count do
        local val_type = bytes[pos]
        pos = pos + 1
        local mut = bytes[pos]
        pos = pos + 1

        local init_bytes, new_pos = parse_init_expr(bytes, pos)
        pos = new_pos

        globals[i] = {
            val_type  = val_type,
            mutable   = (mut == 1),
            init_expr = init_bytes,
        }
    end

    return globals
end

-- ============================================================================
-- parse_export_section(bytes, pos, end_pos) — decode the Export section
--
-- The Export section defines the module's public interface — what it makes
-- available to the host or to other modules that might link with it.
--
-- Binary layout:
--   count (unsigned LEB128)
--   For each export:
--     name (length-prefixed UTF-8 string)
--     export_desc:
--       tag (1 byte):
--         0x00 → function index (unsigned LEB128)
--         0x01 → table index (unsigned LEB128)
--         0x02 → memory index (unsigned LEB128)
--         0x03 → global index (unsigned LEB128)
--
-- EXAMPLE: a module that exports a function named "add":
--   "add" (0x03, 0x61, 0x64, 0x64), tag=0x00, index=0
--
-- Returns: array of {name=S, desc={kind=K, idx=I}} tables.
-- ============================================================================
local function parse_export_section(bytes, pos, end_pos)
    local exports = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    local EXPORT_KINDS = { [0]="func", [1]="table", [2]="mem", [3]="global" }

    for _ = 1, count do
        local name, new_pos = read_string(bytes, pos)
        pos = new_pos

        local tag = bytes[pos]
        pos = pos + 1

        local idx, ic = leb128.decode_unsigned(bytes, pos)
        pos = pos + ic

        local kind = EXPORT_KINDS[tag] or string.format("unknown_%d", tag)
        exports[#exports + 1] = { name = name, desc = { kind = kind, idx = idx } }
    end

    return exports
end

-- ============================================================================
-- parse_start_section(bytes, pos, end_pos) — decode the Start section
--
-- The optional Start section specifies the index of a function that the
-- runtime should call automatically when the module is instantiated.
-- This is similar to a "main" function or a constructor.
--
-- The start function must have the type () → () (no parameters, no results).
-- It runs before any exported functions are called.
--
-- Binary layout:
--   function_index (unsigned LEB128)
--
-- Returns: an integer (the function index).
-- ============================================================================
local function parse_start_section(bytes, pos, end_pos)
    local func_idx, _ = leb128.decode_unsigned(bytes, pos)
    return func_idx
end

-- ============================================================================
-- parse_code_section(bytes, pos, end_pos) — decode the Code section
--
-- The Code section contains the actual bytecode for each locally-defined
-- function. The number of entries here must match the number of entries in
-- the Function section.
--
-- Binary layout:
--   count (unsigned LEB128)         — number of function bodies
--   For each function body:
--     body_size (unsigned LEB128)   — total size of this function body in bytes
--     local_decls_count (unsigned LEB128)  — number of local variable groups
--     For each local group:
--       n (unsigned LEB128)   — count of locals of this type
--       type (1 byte)         — the ValType for these locals
--     [instruction bytes]     — the function body bytecode
--     0x0B                    — "end" opcode (terminates the function)
--
-- LOCAL VARIABLE GROUPS
-- Local variables are declared as groups to compress the common case where
-- you have multiple locals of the same type. For example, instead of:
--   local 0: i32
--   local 1: i32
--   local 2: i32
-- You write one group: count=3, type=i32.
-- This is more compact for the common case of multiple same-typed locals.
--
-- WHY SEPARATE TYPE AND CODE SECTIONS?
-- The Function section tells you what type each function has. The Code section
-- gives the actual bytecode. This separation allows type-checking (validation)
-- to be done without loading the function bodies — important for streaming
-- compilation where you want to validate types before you've even received
-- all the code bytes.
--
-- Returns: array of {locals=[{count=N, type=T},...], body=[...]} tables.
-- The body field contains the raw instruction bytes (including the trailing 0x0B).
-- ============================================================================
local function parse_code_section(bytes, pos, end_pos)
    local codes = {}

    local count, c = leb128.decode_unsigned(bytes, pos)
    pos = pos + c

    for _ = 1, count do
        -- Each function body is prefixed with its byte length.
        -- This allows skipping over function bodies you don't need to parse.
        local body_size, bc = leb128.decode_unsigned(bytes, pos)
        pos = pos + bc
        local body_start = pos

        -- Read local variable declarations
        local local_count, lc = leb128.decode_unsigned(bytes, pos)
        pos = pos + lc

        local locals = {}
        for i = 1, local_count do
            local n, nc = leb128.decode_unsigned(bytes, pos)
            pos = pos + nc
            local t = bytes[pos]
            pos = pos + 1
            locals[i] = { count = n, type = t }
        end

        -- The rest of the function body (up to body_start + body_size) is
        -- the raw bytecode instructions. We preserve it as a byte array.
        local instr_size = body_size - (pos - body_start)
        local instr_bytes, new_pos = read_bytes(bytes, pos, instr_size)
        pos = new_pos

        codes[#codes + 1] = { locals = locals, body = instr_bytes }
    end

    return codes
end

-- ============================================================================
-- parse_custom_section(bytes, pos, section_size) — decode a Custom section
--
-- Custom sections are extension points in the Wasm binary format. They have
-- an arbitrary name and arbitrary byte content. They are used for:
--   - Debug information (DWARF, Wasm-specific debug formats)
--   - Source maps
--   - Name sections (mapping function indices to human-readable names)
--   - Producer information (what compiler generated this module)
--
-- Binary layout:
--   name (length-prefixed UTF-8 string)
--   data (remaining bytes in the section)
--
-- Custom sections are OPTIONAL — runtimes that don't understand a custom
-- section can safely skip it. This is the extension mechanism for the format.
--
-- Returns: {name=S, data=[...]} table.
-- ============================================================================
local function parse_custom_section(bytes, pos, section_end)
    local name, new_pos = read_string(bytes, pos)
    pos = new_pos

    -- The data is all remaining bytes in the section
    local data_size = section_end - pos + 1
    local data = {}
    if data_size > 0 then
        for i = 1, data_size do
            data[i] = bytes[pos + i - 1]
        end
    end

    return { name = name, data = data }
end

-- ============================================================================
-- M.parse_header(bytes, pos) — validate the Wasm module header
--
-- Every valid Wasm binary must start with:
--   1. The magic number: 0x00 0x61 0x73 0x6D ("\0asm")
--   2. The version number: 0x01 0x00 0x00 0x00 (version 1, little-endian)
--
-- The magic number prevents accidentally treating a non-Wasm file as Wasm.
-- The version number allows the format to evolve — if a new incompatible
-- format is defined someday, it would use version 2.
--
-- Arguments:
--   bytes — integer array (the full module bytes)
--   pos   — 1-based starting position (should be 1 for a fresh module)
--
-- Returns: new_pos (= 9, after the 8-byte header)
-- Errors: if magic or version bytes are wrong
-- ============================================================================
function M.parse_header(bytes, pos)
    pos = pos or 1

    -- Check magic number bytes: 0x00, 0x61 ('a'), 0x73 ('s'), 0x6D ('m')
    local expected_magic = {0x00, 0x61, 0x73, 0x6D}
    for i = 1, 4 do
        local b = bytes[pos + i - 1]
        if b ~= expected_magic[i] then
            error(string.format(
                "wasm_module_parser: invalid magic byte at pos %d: expected 0x%02x, got 0x%02x",
                pos + i - 1, expected_magic[i], b or 0))
        end
    end
    pos = pos + 4

    -- Check version bytes: 0x01, 0x00, 0x00, 0x00 (version 1, little-endian)
    local expected_version = {0x01, 0x00, 0x00, 0x00}
    for i = 1, 4 do
        local b = bytes[pos + i - 1]
        if b ~= expected_version[i] then
            error(string.format(
                "wasm_module_parser: invalid version byte at pos %d: expected 0x%02x, got 0x%02x",
                pos + i - 1, expected_version[i], b or 0))
        end
    end
    pos = pos + 4

    return pos
end

-- ============================================================================
-- M.parse_section(bytes, pos) — parse one section header
--
-- Reads the section ID (1 byte) and section content length (unsigned LEB128).
-- Returns a table describing the section and the position of the first content
-- byte.
--
-- This function does NOT parse the section content — that's done by the
-- section-specific parsers above. This function just gives you the envelope.
--
-- Returns: section_info, content_start_pos
--   section_info = {
--     id      = <section ID integer>,
--     name    = <human-readable name>,
--     size    = <content length in bytes>,
--     content_start = <pos of first content byte>,
--     content_end   = <pos of last content byte>,
--   }
-- ============================================================================
function M.parse_section(bytes, pos)
    if pos > #bytes then
        return nil, pos  -- no more sections
    end

    local id = bytes[pos]
    pos = pos + 1

    local size, sc = leb128.decode_unsigned(bytes, pos)
    pos = pos + sc

    local content_start = pos
    local content_end   = pos + size - 1

    local info = {
        id            = id,
        name          = SECTION_NAMES[id] or string.format("unknown_%d", id),
        size          = size,
        content_start = content_start,
        content_end   = content_end,
    }

    return info, pos
end

-- ============================================================================
-- M.get_section(module, section_id) — find a parsed section by ID
--
-- Convenience function for finding a specific section in a parsed module.
-- For sections that can appear only once (all except Custom), returns the
-- first match. For Custom sections (id=0), returns all of them as an array.
--
-- Usage:
--   local type_section = M.get_section(module, M.SECTION_TYPE)
--   -- returns module.types (or nil if no type section was present)
--
-- Returns: the section data (or nil if not present)
-- ============================================================================
function M.get_section(module, section_id)
    if section_id == M.SECTION_TYPE     then return module.types     end
    if section_id == M.SECTION_IMPORT   then return module.imports   end
    if section_id == M.SECTION_FUNCTION then return module.functions end
    if section_id == M.SECTION_TABLE    then return module.tables    end
    if section_id == M.SECTION_MEMORY   then return module.memories  end
    if section_id == M.SECTION_GLOBAL   then return module.globals   end
    if section_id == M.SECTION_EXPORT   then return module.exports   end
    if section_id == M.SECTION_START    then return module.start     end
    if section_id == M.SECTION_ELEMENT  then return module.elements  end
    if section_id == M.SECTION_CODE     then return module.codes     end
    if section_id == M.SECTION_DATA     then return module.data      end
    if section_id == M.SECTION_CUSTOM   then return module.custom    end
    return nil
end

-- ============================================================================
-- M.parse(bytes) — parse a complete WebAssembly binary module
--
-- This is the main entry point. It accepts a binary string (the raw bytes
-- of a .wasm file) and returns a structured Lua table describing the module.
--
-- Arguments:
--   bytes — a Lua string containing the binary Wasm module data
--           (use io.open(path, "rb") to read a .wasm file)
--
-- Returns: a module table with these fields:
--   magic     — always "\0asm"
--   version   — always 1
--   types     — array of {params=[...], results=[...]}
--   imports   — array of {mod=S, name=S, desc={kind=K, ...}}
--   functions — array of type_index integers
--   tables    — array of {ref_type=T, limits={min=N, max=M}}
--   memories  — array of {limits={min=N, max=M}}
--   globals   — array of {val_type=T, mutable=bool, init_expr=[...]}
--   exports   — array of {name=S, desc={kind=K, idx=I}}
--   start     — nil or function_index integer
--   elements  — array of raw byte arrays (one per element segment)
--   codes     — array of {locals=[...], body=[...]}
--   data      — array of raw byte arrays (one per data segment)
--   custom    — array of {name=S, data=[...]} (all custom sections)
--
-- Errors if the magic number or version are wrong.
-- ============================================================================
function M.parse(bytes_str)
    -- Convert binary string to integer array for easier processing.
    -- We do this once upfront. All sub-parsers work with this integer array.
    local bytes = bytes_from_string(bytes_str)

    -- Initialize the module structure with empty defaults.
    -- Fields that are nil were absent from the binary.
    local module = {
        magic     = M.MODULE_MAGIC,
        version   = M.MODULE_VERSION,
        types     = {},
        imports   = {},
        functions = {},
        tables    = {},
        memories  = {},
        globals   = {},
        exports   = {},
        start     = nil,
        elements  = {},
        codes     = {},
        data      = {},
        custom    = {},
    }

    -- Parse and validate the 8-byte header
    local pos = M.parse_header(bytes, 1)

    -- Parse sections one by one until we run out of bytes
    while pos <= #bytes do
        local section_info, content_start = M.parse_section(bytes, pos)
        if section_info == nil then
            break
        end

        local id      = section_info.id
        local cstart  = section_info.content_start
        local cend    = section_info.content_end
        local csize   = section_info.size

        if id == M.SECTION_TYPE then
            -- Function type signatures
            module.types = parse_type_section(bytes, cstart, cend)

        elseif id == M.SECTION_IMPORT then
            -- External symbol imports
            module.imports = parse_import_section(bytes, cstart, cend)

        elseif id == M.SECTION_FUNCTION then
            -- Maps local functions to type indices
            module.functions = parse_function_section(bytes, cstart, cend)

        elseif id == M.SECTION_TABLE then
            -- Table definitions
            module.tables = parse_table_section(bytes, cstart, cend)

        elseif id == M.SECTION_MEMORY then
            -- Memory limit definitions
            module.memories = parse_memory_section(bytes, cstart, cend)

        elseif id == M.SECTION_GLOBAL then
            -- Global variable definitions
            module.globals = parse_global_section(bytes, cstart, cend)

        elseif id == M.SECTION_EXPORT then
            -- Exported symbols
            module.exports = parse_export_section(bytes, cstart, cend)

        elseif id == M.SECTION_START then
            -- Start function index
            module.start = parse_start_section(bytes, cstart, cend)

        elseif id == M.SECTION_ELEMENT then
            -- Element segments (raw bytes — complex enough to warrant a
            -- dedicated parser in wasm_element_parser)
            local raw, _ = read_bytes(bytes, cstart, csize)
            module.elements[#module.elements + 1] = raw

        elseif id == M.SECTION_CODE then
            -- Function bodies
            module.codes = parse_code_section(bytes, cstart, cend)

        elseif id == M.SECTION_DATA then
            -- Data segments (raw bytes)
            local raw, _ = read_bytes(bytes, cstart, csize)
            module.data[#module.data + 1] = raw

        elseif id == M.SECTION_CUSTOM then
            -- Custom section (name + arbitrary data)
            local custom = parse_custom_section(bytes, cstart, cend)
            module.custom[#module.custom + 1] = custom

        else
            -- Unknown section IDs are preserved as raw bytes.
            -- The spec says to ignore unknown sections for forward compatibility.
            local raw, _ = read_bytes(bytes, cstart, csize)
            -- We could store these somewhere; for now we skip them.
        end

        -- Advance past this section's content to the next section
        pos = cend + 1
    end

    return module
end

return M

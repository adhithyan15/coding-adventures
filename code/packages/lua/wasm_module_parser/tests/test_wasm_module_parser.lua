-- Tests for coding_adventures.wasm_module_parser
--
-- WebAssembly binary modules follow a precise structure. These tests construct
-- minimal valid .wasm binaries by hand (as binary strings) and verify that the
-- parser correctly decodes each part.
--
-- ANATOMY OF A TEST BINARY
-- ────────────────────────
-- Each test builds a binary string using string.char() to create raw bytes.
-- For example, the minimal valid Wasm module (magic + version, no sections):
--
--   \x00 \x61 \x73 \x6D   -- magic: "\0asm"
--   \x01 \x00 \x00 \x00   -- version: 1 (little-endian uint32)
--
-- Sections are appended as:
--   [id byte] [LEB128 length] [content bytes...]
--
-- LEB128 note: for lengths <= 127, one byte suffices (the byte IS the length).

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local leb128 = require("coding_adventures.wasm_leb128")
local wt     = require("coding_adventures.wasm_types")
local parser = require("coding_adventures.wasm_module_parser")

-- ---------------------------------------------------------------------------
-- Byte-building helpers
--
-- These helpers make it readable to construct binary Wasm data in tests.
-- ---------------------------------------------------------------------------

-- b(...) — convert a list of byte integers to a binary string
local function b(...)
    local chars = {}
    local args = {...}
    for i = 1, #args do
        chars[i] = string.char(args[i])
    end
    return table.concat(chars)
end

-- leb_u(n) — encode n as unsigned LEB128 string (for embedding in binaries)
local function leb_u(n)
    local arr = leb128.encode_unsigned(n)
    local chars = {}
    for i, byte in ipairs(arr) do
        chars[i] = string.char(byte)
    end
    return table.concat(chars)
end

-- str_field(s) — encode a string as length-prefixed UTF-8 (for names in Wasm)
local function str_field(s)
    return leb_u(#s) .. s
end

-- WASM_HEADER: the 8-byte prefix shared by all valid Wasm modules
local WASM_HEADER = b(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00)

-- section(id, content) — wrap content in a section envelope
local function section(id, content)
    return b(id) .. leb_u(#content) .. content
end

-- ---------------------------------------------------------------------------
-- Test suite
-- ---------------------------------------------------------------------------

describe("wasm_module_parser", function()

    -- =========================================================================
    -- Meta / version
    -- =========================================================================

    describe("module metadata", function()
        it("has VERSION 0.1.0", function()
            assert.equals("0.1.0", parser.VERSION)
        end)

        it("exposes MODULE_MAGIC constant", function()
            assert.equals("\0asm", parser.MODULE_MAGIC)
        end)

        it("exposes MODULE_VERSION constant", function()
            assert.equals(1, parser.MODULE_VERSION)
        end)

        it("exposes section ID constants", function()
            assert.equals(0,  parser.SECTION_CUSTOM)
            assert.equals(1,  parser.SECTION_TYPE)
            assert.equals(2,  parser.SECTION_IMPORT)
            assert.equals(3,  parser.SECTION_FUNCTION)
            assert.equals(4,  parser.SECTION_TABLE)
            assert.equals(5,  parser.SECTION_MEMORY)
            assert.equals(6,  parser.SECTION_GLOBAL)
            assert.equals(7,  parser.SECTION_EXPORT)
            assert.equals(8,  parser.SECTION_START)
            assert.equals(9,  parser.SECTION_ELEMENT)
            assert.equals(10, parser.SECTION_CODE)
            assert.equals(11, parser.SECTION_DATA)
        end)

        it("exposes parse function", function()
            assert.is_function(parser.parse)
        end)

        it("exposes parse_header function", function()
            assert.is_function(parser.parse_header)
        end)

        it("exposes parse_section function", function()
            assert.is_function(parser.parse_section)
        end)

        it("exposes get_section function", function()
            assert.is_function(parser.get_section)
        end)
    end)

    -- =========================================================================
    -- Minimal module: just header, no sections
    --
    -- The simplest valid Wasm module is 8 bytes: magic + version.
    -- This tests that the parser handles the empty case correctly.
    -- =========================================================================

    describe("minimal module (header only)", function()
        local wasm = WASM_HEADER
        local mod

        before_each(function()
            mod = parser.parse(wasm)
        end)

        it("returns a module table", function()
            assert.is_table(mod)
        end)

        it("has correct magic", function()
            assert.equals("\0asm", mod.magic)
        end)

        it("has correct version", function()
            assert.equals(1, mod.version)
        end)

        it("has empty types array", function()
            assert.is_table(mod.types)
            assert.equals(0, #mod.types)
        end)

        it("has empty imports array", function()
            assert.is_table(mod.imports)
            assert.equals(0, #mod.imports)
        end)

        it("has empty functions array", function()
            assert.is_table(mod.functions)
            assert.equals(0, #mod.functions)
        end)

        it("has empty tables array", function()
            assert.is_table(mod.tables)
            assert.equals(0, #mod.tables)
        end)

        it("has empty memories array", function()
            assert.is_table(mod.memories)
            assert.equals(0, #mod.memories)
        end)

        it("has empty globals array", function()
            assert.is_table(mod.globals)
            assert.equals(0, #mod.globals)
        end)

        it("has empty exports array", function()
            assert.is_table(mod.exports)
            assert.equals(0, #mod.exports)
        end)

        it("has nil start", function()
            assert.is_nil(mod.start)
        end)

        it("has empty codes array", function()
            assert.is_table(mod.codes)
            assert.equals(0, #mod.codes)
        end)

        it("has empty custom array", function()
            assert.is_table(mod.custom)
            assert.equals(0, #mod.custom)
        end)
    end)

    -- =========================================================================
    -- parse_header — explicit tests for header validation
    -- =========================================================================

    describe("parse_header", function()
        it("accepts valid header and returns pos=9", function()
            local bytes = {}
            for i = 1, #WASM_HEADER do
                bytes[i] = string.byte(WASM_HEADER, i)
            end
            local new_pos = parser.parse_header(bytes, 1)
            assert.equals(9, new_pos)
        end)

        it("errors on wrong magic byte", function()
            -- Change first byte from 0x00 to 0xFF
            local bad = "\xFF" .. string.sub(WASM_HEADER, 2)
            local bytes = {}
            for i = 1, #bad do bytes[i] = string.byte(bad, i) end
            assert.has_error(function()
                parser.parse_header(bytes, 1)
            end)
        end)

        it("errors on wrong version", function()
            -- Change version to 0x02
            local bad = string.sub(WASM_HEADER, 1, 4) .. b(0x02, 0x00, 0x00, 0x00)
            local bytes = {}
            for i = 1, #bad do bytes[i] = string.byte(bad, i) end
            assert.has_error(function()
                parser.parse_header(bytes, 1)
            end)
        end)
    end)

    -- =========================================================================
    -- Type section
    --
    -- The Type section lists function signatures. We build a module with:
    --   - One type entry: () → i32
    --     Encoded as: 0x60, param_count=0, result_count=1, result=0x7F
    -- =========================================================================

    describe("type section", function()
        --
        -- Type entry for () → i32:
        --   0x60      function type magic
        --   0x00      param_count = 0 (LEB128)
        --   0x01      result_count = 1 (LEB128)
        --   0x7F      i32 (the result type)
        --
        -- Section content: count=1, then the type entry
        --   0x01      count = 1 (LEB128)
        --   0x60 0x00 0x01 0x7F   the type entry
        --
        local type_content = leb_u(1)    -- count = 1
            .. b(0x60)                   -- func type marker
            .. leb_u(0)                  -- 0 params
            .. leb_u(1) .. b(0x7F)       -- 1 result: i32

        local wasm = WASM_HEADER .. section(1, type_content)
        local mod

        before_each(function()
            mod = parser.parse(wasm)
        end)

        it("parses one type entry", function()
            assert.equals(1, #mod.types)
        end)

        it("entry has empty params", function()
            assert.equals(0, #mod.types[1].params)
        end)

        it("entry has one result", function()
            assert.equals(1, #mod.types[1].results)
        end)

        it("result is i32 (0x7F)", function()
            assert.equals(0x7F, mod.types[1].results[1])
        end)
    end)

    describe("type section with multiple entries", function()
        -- Type 0: () → ()
        -- Type 1: (i32, i32) → i64
        local type_content = leb_u(2)   -- count = 2
            -- Entry 0: () → ()
            .. b(0x60) .. leb_u(0) .. leb_u(0)
            -- Entry 1: (i32, i32) → i64
            .. b(0x60) .. leb_u(2) .. b(0x7F, 0x7F) .. leb_u(1) .. b(0x7E)

        local wasm = WASM_HEADER .. section(1, type_content)
        local mod

        before_each(function()
            mod = parser.parse(wasm)
        end)

        it("parses two type entries", function()
            assert.equals(2, #mod.types)
        end)

        it("type 0 is () → ()", function()
            assert.equals(0, #mod.types[1].params)
            assert.equals(0, #mod.types[1].results)
        end)

        it("type 1 has two i32 params", function()
            assert.equals(2, #mod.types[2].params)
            assert.equals(0x7F, mod.types[2].params[1])
            assert.equals(0x7F, mod.types[2].params[2])
        end)

        it("type 1 has one i64 result", function()
            assert.equals(1, #mod.types[2].results)
            assert.equals(0x7E, mod.types[2].results[1])
        end)
    end)

    -- =========================================================================
    -- Export section
    --
    -- We build a module that exports one function named "add":
    --   - The export name is "add" (3 bytes)
    --   - Export descriptor: tag=0x00 (function), index=0
    --
    -- Binary:
    --   count=1
    --   name: 0x03 "add"    (length=3, then 'a'=0x61, 'd'=0x64, 'd'=0x64)
    --   tag:  0x00           (function)
    --   idx:  0x00           (function index 0)
    -- =========================================================================

    describe("export section", function()
        local export_content = leb_u(1)             -- count = 1
            .. str_field("add")                     -- name = "add"
            .. b(0x00) .. leb_u(0)                  -- func export, idx=0

        local wasm = WASM_HEADER .. section(7, export_content)
        local mod

        before_each(function()
            mod = parser.parse(wasm)
        end)

        it("parses one export", function()
            assert.equals(1, #mod.exports)
        end)

        it("export name is 'add'", function()
            assert.equals("add", mod.exports[1].name)
        end)

        it("export kind is 'func'", function()
            assert.equals("func", mod.exports[1].desc.kind)
        end)

        it("export index is 0", function()
            assert.equals(0, mod.exports[1].desc.idx)
        end)
    end)

    describe("export section with multiple exports", function()
        -- Export function 0 as "main" and memory 0 as "mem"
        local export_content = leb_u(2)
            .. str_field("main") .. b(0x00) .. leb_u(0)   -- func export
            .. str_field("mem")  .. b(0x02) .. leb_u(0)   -- mem export

        local wasm = WASM_HEADER .. section(7, export_content)
        local mod

        before_each(function()
            mod = parser.parse(wasm)
        end)

        it("parses two exports", function()
            assert.equals(2, #mod.exports)
        end)

        it("first export is 'main' func", function()
            assert.equals("main", mod.exports[1].name)
            assert.equals("func", mod.exports[1].desc.kind)
            assert.equals(0, mod.exports[1].desc.idx)
        end)

        it("second export is 'mem' memory", function()
            assert.equals("mem", mod.exports[2].name)
            assert.equals("mem", mod.exports[2].desc.kind)
            assert.equals(0, mod.exports[2].desc.idx)
        end)
    end)

    -- =========================================================================
    -- Import section
    --
    -- We build a module that imports a function "env"."log" with type 0.
    --
    -- Binary:
    --   count=1
    --   module: 0x03 "env"
    --   field:  0x03 "log"
    --   tag: 0x00 (function), type_idx: 0x00
    -- =========================================================================

    describe("import section (function import)", function()
        local import_content = leb_u(1)
            .. str_field("env")
            .. str_field("log")
            .. b(0x00) .. leb_u(0)   -- func import, type_idx=0

        local wasm = WASM_HEADER .. section(2, import_content)
        local mod

        before_each(function()
            mod = parser.parse(wasm)
        end)

        it("parses one import", function()
            assert.equals(1, #mod.imports)
        end)

        it("import module is 'env'", function()
            assert.equals("env", mod.imports[1].mod)
        end)

        it("import name is 'log'", function()
            assert.equals("log", mod.imports[1].name)
        end)

        it("import kind is 'func'", function()
            assert.equals("func", mod.imports[1].desc.kind)
        end)

        it("import type_idx is 0", function()
            assert.equals(0, mod.imports[1].desc.type_idx)
        end)
    end)

    describe("import section (memory import)", function()
        -- Import memory with limits: min=1 page, no max
        -- limits: flag=0x00, min=1 (LEB128)
        local import_content = leb_u(1)
            .. str_field("env")
            .. str_field("memory")
            .. b(0x02)               -- mem import tag
            .. b(0x00) .. leb_u(1)   -- limits: no max, min=1

        local wasm = WASM_HEADER .. section(2, import_content)
        local mod = parser.parse(wasm)

        it("parses one import", function()
            assert.equals(1, #mod.imports)
        end)

        it("import kind is 'mem'", function()
            assert.equals("mem", mod.imports[1].desc.kind)
        end)

        it("memory min is 1", function()
            assert.equals(1, mod.imports[1].desc.limits.min)
        end)

        it("memory max is nil (unbounded)", function()
            assert.is_nil(mod.imports[1].desc.limits.max)
        end)
    end)

    describe("import section (global import)", function()
        -- Import a const i32 global named "stackPtr" from "env"
        local import_content = leb_u(1)
            .. str_field("env")
            .. str_field("stackPtr")
            .. b(0x03)        -- global import tag
            .. b(0x7F)        -- val_type = i32
            .. b(0x00)        -- mutability = 0 (const)

        local wasm = WASM_HEADER .. section(2, import_content)
        local mod = parser.parse(wasm)

        it("parses global import", function()
            assert.equals(1, #mod.imports)
            assert.equals("global", mod.imports[1].desc.kind)
            assert.equals(0x7F, mod.imports[1].desc.val_type)
            assert.is_false(mod.imports[1].desc.mutable)
        end)
    end)

    -- =========================================================================
    -- Function section
    --
    -- A module with two functions, both using type index 0.
    -- =========================================================================

    describe("function section", function()
        local func_content = leb_u(2) .. leb_u(0) .. leb_u(0)   -- 2 funcs, both type 0

        local wasm = WASM_HEADER .. section(3, func_content)
        local mod = parser.parse(wasm)

        it("parses two function type indices", function()
            assert.equals(2, #mod.functions)
        end)

        it("both reference type 0", function()
            assert.equals(0, mod.functions[1])
            assert.equals(0, mod.functions[2])
        end)
    end)

    -- =========================================================================
    -- Memory section
    --
    -- A single memory with min=1 page, no maximum.
    -- =========================================================================

    describe("memory section", function()
        local mem_content = leb_u(1)    -- count = 1
            .. b(0x00) .. leb_u(1)      -- limits: no max, min=1

        local wasm = WASM_HEADER .. section(5, mem_content)
        local mod = parser.parse(wasm)

        it("parses one memory", function()
            assert.equals(1, #mod.memories)
        end)

        it("memory min is 1 page", function()
            assert.equals(1, mod.memories[1].limits.min)
        end)

        it("memory max is nil", function()
            assert.is_nil(mod.memories[1].limits.max)
        end)
    end)

    describe("bounded memory section", function()
        local mem_content = leb_u(1)            -- count = 1
            .. b(0x01) .. leb_u(1) .. leb_u(4)  -- limits: min=1, max=4

        local wasm = WASM_HEADER .. section(5, mem_content)
        local mod = parser.parse(wasm)

        it("memory min is 1", function()
            assert.equals(1, mod.memories[1].limits.min)
        end)

        it("memory max is 4", function()
            assert.equals(4, mod.memories[1].limits.max)
        end)
    end)

    -- =========================================================================
    -- Table section
    --
    -- A single funcref table with min=1, no max.
    -- =========================================================================

    describe("table section", function()
        local tbl_content = leb_u(1)         -- count = 1
            .. b(0x70)                        -- ref_type = funcref
            .. b(0x00) .. leb_u(1)            -- limits: no max, min=1

        local wasm = WASM_HEADER .. section(4, tbl_content)
        local mod = parser.parse(wasm)

        it("parses one table", function()
            assert.equals(1, #mod.tables)
        end)

        it("table ref_type is funcref (0x70)", function()
            assert.equals(0x70, mod.tables[1].ref_type)
        end)

        it("table min is 1", function()
            assert.equals(1, mod.tables[1].limits.min)
        end)
    end)

    -- =========================================================================
    -- Global section
    --
    -- A single const i32 global initialized to 42.
    --
    -- init_expr for i32.const 42:
    --   0x41  (i32.const opcode)
    --   0x2A  (42 as signed LEB128 — single byte since 42 < 64)
    --   0x0B  (end opcode)
    -- =========================================================================

    describe("global section", function()
        local global_content = leb_u(1)     -- count = 1
            .. b(0x7F)                       -- val_type = i32
            .. b(0x00)                       -- mutability = const
            .. b(0x41, 0x2A, 0x0B)           -- i32.const 42; end

        local wasm = WASM_HEADER .. section(6, global_content)
        local mod = parser.parse(wasm)

        it("parses one global", function()
            assert.equals(1, #mod.globals)
        end)

        it("global val_type is i32", function()
            assert.equals(0x7F, mod.globals[1].val_type)
        end)

        it("global is not mutable", function()
            assert.is_false(mod.globals[1].mutable)
        end)

        it("global has init_expr bytes", function()
            assert.is_table(mod.globals[1].init_expr)
            assert.equals(0x41, mod.globals[1].init_expr[1])  -- i32.const
            assert.equals(0x2A, mod.globals[1].init_expr[2])  -- 42
            assert.equals(0x0B, mod.globals[1].init_expr[3])  -- end
        end)
    end)

    -- =========================================================================
    -- Start section
    --
    -- Points to function index 0.
    -- =========================================================================

    describe("start section", function()
        local start_content = leb_u(0)  -- function index 0

        local wasm = WASM_HEADER .. section(8, start_content)
        local mod = parser.parse(wasm)

        it("parses start function index", function()
            assert.equals(0, mod.start)
        end)
    end)

    -- =========================================================================
    -- Code section
    --
    -- A single function body with no locals, containing just the "end" byte.
    -- This is the minimal valid function body.
    --
    -- Function body layout:
    --   body_size  (LEB128)     — total bytes below this length prefix
    --   local_count (LEB128)    — number of local variable groups = 0
    --   [instructions]          — function body bytecode
    --   0x0B                    — "end" opcode
    --
    -- For a body with no locals and just "end":
    --   body_size = 1 + 1 = 2 bytes:
    --     0x00  (local_count = 0)
    --     0x0B  (end opcode)
    -- =========================================================================

    describe("code section (minimal function body)", function()
        -- One function body: no locals, just "end"
        local body = b(0x00, 0x0B)    -- local_count=0, end opcode
        local code_content = leb_u(1)  -- count = 1
            .. leb_u(#body)             -- body_size
            .. body

        local wasm = WASM_HEADER .. section(10, code_content)
        local mod = parser.parse(wasm)

        it("parses one code entry", function()
            assert.equals(1, #mod.codes)
        end)

        it("code entry has no locals", function()
            assert.equals(0, #mod.codes[1].locals)
        end)

        it("code entry has body bytes", function()
            assert.is_table(mod.codes[1].body)
            -- The body contains the "end" opcode (0x0B)
            assert.equals(0x0B, mod.codes[1].body[1])
        end)
    end)

    describe("code section with locals", function()
        -- Function body with two local groups: 2x i32, 1x f64
        -- locals:  count=2, {n=2, type=i32}, {n=1, type=f64}
        -- The group encoding: local_decls_count=2,
        --   group 0: count=2, type=0x7F (i32)
        --   group 1: count=1, type=0x7C (f64)
        -- Then end opcode
        local body = leb_u(2)                      -- 2 local groups
            .. leb_u(2) .. b(0x7F)                 -- group 0: 2x i32
            .. leb_u(1) .. b(0x7C)                 -- group 1: 1x f64
            .. b(0x0B)                             -- end opcode
        local code_content = leb_u(1) .. leb_u(#body) .. body

        local wasm = WASM_HEADER .. section(10, code_content)
        local mod = parser.parse(wasm)

        it("parses code with two local groups", function()
            assert.equals(2, #mod.codes[1].locals)
        end)

        it("first local group: 2x i32", function()
            assert.equals(2,    mod.codes[1].locals[1].count)
            assert.equals(0x7F, mod.codes[1].locals[1].type)
        end)

        it("second local group: 1x f64", function()
            assert.equals(1,    mod.codes[1].locals[2].count)
            assert.equals(0x7C, mod.codes[1].locals[2].type)
        end)
    end)

    -- =========================================================================
    -- Custom section
    --
    -- Custom sections have a name and arbitrary data.
    -- We create one named "hello" with data bytes [1, 2, 3].
    -- =========================================================================

    describe("custom section", function()
        local custom_content = str_field("hello")    -- name = "hello"
            .. b(0x01, 0x02, 0x03)                   -- data = [1, 2, 3]

        local wasm = WASM_HEADER .. section(0, custom_content)
        local mod = parser.parse(wasm)

        it("parses one custom section", function()
            assert.equals(1, #mod.custom)
        end)

        it("custom section name is 'hello'", function()
            assert.equals("hello", mod.custom[1].name)
        end)

        it("custom section data is [1, 2, 3]", function()
            assert.equals(3, #mod.custom[1].data)
            assert.equals(0x01, mod.custom[1].data[1])
            assert.equals(0x02, mod.custom[1].data[2])
            assert.equals(0x03, mod.custom[1].data[3])
        end)
    end)

    describe("multiple custom sections", function()
        local c1 = str_field("name")    .. b(0xAA)
        local c2 = str_field("source")  .. b(0xBB, 0xCC)

        local wasm = WASM_HEADER
            .. section(0, c1)
            .. section(0, c2)
        local mod = parser.parse(wasm)

        it("parses two custom sections", function()
            assert.equals(2, #mod.custom)
        end)

        it("first is 'name'", function()
            assert.equals("name", mod.custom[1].name)
        end)

        it("second is 'source'", function()
            assert.equals("source", mod.custom[2].name)
        end)
    end)

    -- =========================================================================
    -- get_section helper
    -- =========================================================================

    describe("get_section", function()
        local wasm = WASM_HEADER .. section(1,
            leb_u(1) .. b(0x60) .. leb_u(0) .. leb_u(0)  -- one empty type
        )
        local mod = parser.parse(wasm)

        it("returns types for SECTION_TYPE", function()
            local t = parser.get_section(mod, parser.SECTION_TYPE)
            assert.is_table(t)
            assert.equals(1, #t)
        end)

        it("returns empty array for absent sections", function()
            local e = parser.get_section(mod, parser.SECTION_EXPORT)
            assert.is_table(e)
            assert.equals(0, #e)
        end)

        it("returns nil for unknown section id", function()
            local x = parser.get_section(mod, 99)
            assert.is_nil(x)
        end)
    end)

    -- =========================================================================
    -- Error handling
    -- =========================================================================

    describe("error handling", function()
        it("errors on wrong magic bytes", function()
            assert.has_error(function()
                parser.parse("NOPE\1\0\0\0")
            end)
        end)

        it("errors on wrong version", function()
            assert.has_error(function()
                parser.parse("\0asm\2\0\0\0")
            end)
        end)

        it("errors on truncated input", function()
            assert.has_error(function()
                parser.parse("\0asm")  -- only 4 bytes, version missing
            end)
        end)
    end)

    -- =========================================================================
    -- Combined: module with type + function + export sections
    --
    -- This approximates a real minimal "add" function module:
    --   - Type section: one type () → i32
    --   - Function section: one function using type 0
    --   - Export section: exports function 0 as "answer"
    --   - Code section: function body returning i32.const 42
    --
    -- The code: i32.const 42 (0x41 0x2A) end (0x0B)
    -- =========================================================================

    describe("combined module (type + function + export + code)", function()
        -- Type section: () → i32
        local type_sec = section(1,
            leb_u(1) .. b(0x60) .. leb_u(0) .. leb_u(1) .. b(0x7F)
        )

        -- Function section: one function, type index 0
        local func_sec = section(3, leb_u(1) .. leb_u(0))

        -- Export section: "answer" → func 0
        local exp_sec = section(7,
            leb_u(1) .. str_field("answer") .. b(0x00) .. leb_u(0)
        )

        -- Code section: one body with no locals, i32.const 42, end
        local body = leb_u(0) .. b(0x41, 0x2A, 0x0B)
        local code_sec = section(10, leb_u(1) .. leb_u(#body) .. body)

        local wasm = WASM_HEADER .. type_sec .. func_sec .. exp_sec .. code_sec
        local mod = parser.parse(wasm)

        it("has one type", function()
            assert.equals(1, #mod.types)
        end)

        it("type is () → i32", function()
            assert.equals(0, #mod.types[1].params)
            assert.equals(1, #mod.types[1].results)
            assert.equals(0x7F, mod.types[1].results[1])
        end)

        it("has one function", function()
            assert.equals(1, #mod.functions)
            assert.equals(0, mod.functions[1])  -- type index 0
        end)

        it("has one export named 'answer'", function()
            assert.equals(1, #mod.exports)
            assert.equals("answer", mod.exports[1].name)
            assert.equals("func", mod.exports[1].desc.kind)
            assert.equals(0, mod.exports[1].desc.idx)
        end)

        it("has one code entry", function()
            assert.equals(1, #mod.codes)
        end)

        it("code body contains i32.const opcode", function()
            -- body[1] = 0x41 (i32.const), body[2] = 0x2A (42), body[3] = 0x0B (end)
            assert.equals(0x41, mod.codes[1].body[1])
            assert.equals(0x2A, mod.codes[1].body[2])
            assert.equals(0x0B, mod.codes[1].body[3])
        end)
    end)

end)

-- Tests for coding_adventures.wasm_simulator
--
-- We build minimal Wasm binaries by hand as binary strings, then run them
-- through the module parser and the simulator. This end-to-end approach
-- verifies the full pipeline from bytes to execution results.
--
-- BUILDING WASM BINARIES BY HAND
-- ───────────────────────────────
-- A valid Wasm module is just a sequence of bytes. We construct them using
-- string.char() so each test fully controls the binary without needing a
-- compiler. This makes the tests self-contained and easy to understand.
--
-- For example, a function that adds two i32s:
--
--   Type section:     [(i32, i32) → i32]
--   Function section: [type_idx = 0]
--   Export section:   ["add" → func 0]
--   Code section:     [locals=[], body=[local.get 0, local.get 1, i32.add, end]]
--
-- Each section is:   [id byte] [LEB128 length] [content bytes]
--
-- READING THE TESTS
-- ─────────────────
-- Each test follows the same pattern:
--   1. Build a binary Wasm module using the helpers below
--   2. Parse it with wasm_module_parser
--   3. Instantiate it with wasm_simulator.Instance.new()
--   4. Call functions and check results

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local leb128    = require("coding_adventures.wasm_leb128")
local parser    = require("coding_adventures.wasm_module_parser")
local simulator = require("coding_adventures.wasm_simulator")

local Instance = simulator.Instance

-- ===========================================================================
-- Binary building helpers
-- ===========================================================================

-- b(...) — convert byte integers to a binary string
local function b(...)
    local chars = {}
    local args = {...}
    for i = 1, #args do
        chars[i] = string.char(args[i])
    end
    return table.concat(chars)
end

-- leb_u(n) — unsigned LEB128 binary string
local function leb_u(n)
    local arr = leb128.encode_unsigned(n)
    local chars = {}
    for i, byte in ipairs(arr) do chars[i] = string.char(byte) end
    return table.concat(chars)
end

-- leb_s(n) — signed LEB128 binary string (for i32.const immediates)
local function leb_s(n)
    local arr = leb128.encode_signed(n)
    local chars = {}
    for i, byte in ipairs(arr) do chars[i] = string.char(byte) end
    return table.concat(chars)
end

-- str_field(s) — length-prefixed UTF-8 string (for export/import names)
local function str_field(s)
    return leb_u(#s) .. s
end

-- section(id, content) — wrap content in a Wasm section envelope
local function section(id, content)
    return b(id) .. leb_u(#content) .. content
end

-- WASM_HEADER: the 8-byte prefix required by all valid Wasm modules
-- Magic: "\0asm", Version: 1 (little-endian uint32)
local WASM_HEADER = b(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00)

-- ===========================================================================
-- Wasm module component builders
-- ===========================================================================
-- These helpers build individual sections so tests can compose modules easily.

-- type_sec(types) — build a type section from a list of {params, results}
-- Each params/results entry is an array of ValType bytes:
--   0x7F = i32, 0x7E = i64, 0x7D = f32, 0x7C = f64
local function type_sec(types)
    local content = leb_u(#types)
    for _, t in ipairs(types) do
        content = content .. b(0x60)  -- func type marker
        content = content .. leb_u(#t.params)
        for _, vt in ipairs(t.params) do content = content .. b(vt) end
        content = content .. leb_u(#t.results)
        for _, vt in ipairs(t.results) do content = content .. b(vt) end
    end
    return section(1, content)
end

-- func_sec(type_indices) — build a function section (maps funcs to types)
local function func_sec(type_indices)
    local content = leb_u(#type_indices)
    for _, ti in ipairs(type_indices) do content = content .. leb_u(ti) end
    return section(3, content)
end

-- export_func(name, func_idx) — one function export entry
local function export_func(name, func_idx)
    return str_field(name) .. b(0x00) .. leb_u(func_idx)
end

-- export_global(name, global_idx) — one global export entry
local function export_global(name, global_idx)
    return str_field(name) .. b(0x03) .. leb_u(global_idx)
end

-- export_mem(name, mem_idx) — one memory export entry
local function export_mem(name, mem_idx)
    return str_field(name) .. b(0x02) .. leb_u(mem_idx)
end

-- export_sec(entries) — build an export section from a list of entries
local function export_sec(entries)
    local content = leb_u(#entries)
    for _, e in ipairs(entries) do content = content .. e end
    return section(7, content)
end

-- code_entry(locals, body) — one function code entry
--   locals: list of {count, type} local groups
--   body:   binary string of instruction bytes
local function code_entry(local_groups, body)
    local locals_bin = leb_u(#local_groups)
    for _, lg in ipairs(local_groups) do
        locals_bin = locals_bin .. leb_u(lg.count) .. b(lg.type)
    end
    local entry = locals_bin .. body
    return leb_u(#entry) .. entry
end

-- code_sec(entries) — build a code section from code entry binary strings
local function code_sec(entries)
    local content = leb_u(#entries)
    for _, e in ipairs(entries) do content = content .. e end
    return section(10, content)
end

-- global_sec(globals) — build a global section
--   globals: list of {val_type, mutable, init_bytes}
local function global_sec(globals)
    local content = leb_u(#globals)
    for _, g in ipairs(globals) do
        content = content .. b(g.val_type) .. b(g.mutable and 1 or 0) .. g.init
    end
    return section(6, content)
end

-- mem_sec(memories) — build a memory section
--   memories: list of {min, max?}
local function mem_sec(memories)
    local content = leb_u(#memories)
    for _, m in ipairs(memories) do
        if m.max then
            content = content .. b(0x01) .. leb_u(m.min) .. leb_u(m.max)
        else
            content = content .. b(0x00) .. leb_u(m.min)
        end
    end
    return section(5, content)
end

-- ===========================================================================
-- Helper: parse_and_instantiate(wasm_binary_string)
-- ===========================================================================
-- Parse a Wasm binary and create an Instance. Used in nearly every test.
local function new_instance(wasm_str)
    local mod = parser.parse(wasm_str)
    return Instance.new(mod)
end

-- ===========================================================================
-- Test suite
-- ===========================================================================

describe("wasm_simulator", function()

    -- =========================================================================
    -- Metadata
    -- =========================================================================

    describe("module metadata", function()
        it("has VERSION 0.1.0", function()
            assert.equals("0.1.0", simulator.VERSION)
        end)

        it("exposes Instance class", function()
            assert.is_not_nil(simulator.Instance)
            assert.is_not_nil(simulator.Instance.new)
        end)

        it("exposes to_i32 helper", function()
            assert.is_not_nil(simulator.to_i32)
        end)
    end)

    -- =========================================================================
    -- to_i32 wrapping arithmetic
    -- =========================================================================

    describe("to_i32 wrapping", function()
        it("returns small values unchanged", function()
            assert.equals(0,  simulator.to_i32(0))
            assert.equals(42, simulator.to_i32(42))
            assert.equals(-1, simulator.to_i32(-1))
        end)

        it("wraps INT_MAX + 1 to INT_MIN", function()
            assert.equals(-2147483648, simulator.to_i32(2147483648))
        end)

        it("wraps UINT32_MAX to -1", function()
            assert.equals(-1, simulator.to_i32(4294967295))
        end)

        it("wraps -2^32 back to 0", function()
            assert.equals(0, simulator.to_i32(4294967296))
        end)
    end)

    -- =========================================================================
    -- Minimal module instantiation
    -- =========================================================================

    describe("Instance.new", function()
        it("instantiates a minimal empty module", function()
            local mod = parser.parse(WASM_HEADER)
            local inst = Instance.new(mod)
            assert.is_not_nil(inst)
        end)

        it("instance has memory even with no memory section", function()
            local mod = parser.parse(WASM_HEADER)
            local inst = Instance.new(mod)
            assert.is_not_nil(inst.memory)
            assert.equals(0, inst.memory.size_pages)
        end)
    end)

    -- =========================================================================
    -- i32.const: constant push
    -- =========================================================================
    --
    -- This is the simplest possible function: it ignores its arguments and
    -- returns a constant value. The body is just:
    --   i32.const 42  (opcode 0x41, LEB128 value 0x2A)
    --   end           (opcode 0x0B)

    describe("i32.const", function()
        it("returns a constant i32 value", function()
            -- Type: () → i32
            -- Body: i32.const 42; end
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("answer", 0)})
                .. code_sec({code_entry({}, b(0x41, 0x2A, 0x0B))})

            local inst = new_instance(wasm)
            local result = inst:call("answer", {})
            assert.equals(1, #result)
            assert.equals(42, result[1])
        end)

        it("returns a negative i32.const", function()
            -- i32.const -1 encodes as LEB128 signed: 0x7F (single byte)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("neg_one", 0)})
                .. code_sec({code_entry({}, b(0x41, 0x7F, 0x0B))})

            local inst = new_instance(wasm)
            local result = inst:call("neg_one", {})
            assert.equals(-1, result[1])
        end)
    end)

    -- =========================================================================
    -- local.get / local.set / local.tee
    -- =========================================================================
    --
    -- local.get 0: push the first argument (which is a function parameter)
    -- local.get 1: push the second argument

    describe("local.get", function()
        it("returns first argument (identity function)", function()
            -- Type: (i32) → i32
            -- Body: local.get 0; end
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("id", 0)})
                .. code_sec({code_entry({}, b(0x20, 0x00, 0x0B))})

            local inst = new_instance(wasm)
            assert.equals(99, inst:call("id", {99})[1])
            assert.equals(0,  inst:call("id", {0})[1])
            assert.equals(-7, inst:call("id", {-7})[1])
        end)

        it("local.set and local.get roundtrip", function()
            -- Type: (i32) → i32
            -- Body: local.get 0; local.set 1; local.get 1; end
            -- (local 1 is a declared i32 local, initialized to 0)
            local body = b(
                0x20, 0x00,   -- local.get 0
                0x21, 0x01,   -- local.set 1
                0x20, 0x01,   -- local.get 1
                0x0B          -- end
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("roundtrip", 0)})
                .. code_sec({code_entry({{count=1, type=0x7F}}, body)})

            local inst = new_instance(wasm)
            assert.equals(55, inst:call("roundtrip", {55})[1])
        end)

        it("local.tee stores AND leaves value on stack", function()
            -- local.tee 1 is like: val = peek; local[1] = val; (val stays)
            -- Type: (i32) → i32
            -- Body: local.get 0; local.tee 1; drop; local.get 1; end
            local body = b(
                0x20, 0x00,   -- local.get 0 → push arg
                0x22, 0x01,   -- local.tee 1 → store in local[1], keep on stack
                0x1A,         -- drop         → discard the tee'd value
                0x20, 0x01,   -- local.get 1  → push local[1]
                0x0B          -- end
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("tee_test", 0)})
                .. code_sec({code_entry({{count=1, type=0x7F}}, body)})

            local inst = new_instance(wasm)
            assert.equals(77, inst:call("tee_test", {77})[1])
        end)
    end)

    -- =========================================================================
    -- Integer arithmetic: add, sub, mul, div_s, rem_s
    -- =========================================================================

    describe("i32 arithmetic", function()
        -- Helper: build a module with one binary i32 op function
        -- Type: (i32, i32) → i32
        -- Body: local.get 0; local.get 1; <opcode>; end
        local function binop_module(opcode)
            local body = b(0x20, 0x00, 0x20, 0x01, opcode, 0x0B)
            return WASM_HEADER
                .. type_sec({{params={0x7F, 0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("op", 0)})
                .. code_sec({code_entry({}, body)})
        end

        it("i32.add: 3 + 4 = 7", function()
            local inst = new_instance(binop_module(0x6A))
            assert.equals(7, inst:call("op", {3, 4})[1])
        end)

        it("i32.add: wraps on overflow", function()
            local inst = new_instance(binop_module(0x6A))
            -- 2147483647 + 1 = -2147483648 (INT_MIN) in 32-bit wrap
            assert.equals(-2147483648, inst:call("op", {2147483647, 1})[1])
        end)

        it("i32.sub: 10 - 3 = 7", function()
            local inst = new_instance(binop_module(0x6B))
            assert.equals(7, inst:call("op", {10, 3})[1])
        end)

        it("i32.sub: wraps on underflow", function()
            local inst = new_instance(binop_module(0x6B))
            -- -2147483648 - 1 = 2147483647 (INT_MAX)
            assert.equals(2147483647, inst:call("op", {-2147483648, 1})[1])
        end)

        it("i32.mul: 6 * 7 = 42", function()
            local inst = new_instance(binop_module(0x6C))
            assert.equals(42, inst:call("op", {6, 7})[1])
        end)

        it("i32.mul: negative × positive", function()
            local inst = new_instance(binop_module(0x6C))
            assert.equals(-20, inst:call("op", {-4, 5})[1])
        end)

        it("i32.div_s: 20 / 4 = 5", function()
            local inst = new_instance(binop_module(0x6D))
            assert.equals(5, inst:call("op", {20, 4})[1])
        end)

        it("i32.div_s: truncates toward zero (-7 / 2 = -3, not -4)", function()
            local inst = new_instance(binop_module(0x6D))
            assert.equals(-3, inst:call("op", {-7, 2})[1])
        end)

        it("i32.div_s: traps on divide-by-zero", function()
            local inst = new_instance(binop_module(0x6D))
            assert.has_error(function()
                inst:call("op", {5, 0})
            end)
        end)

        it("i32.rem_s: 10 rem 3 = 1", function()
            local inst = new_instance(binop_module(0x6F))
            assert.equals(1, inst:call("op", {10, 3})[1])
        end)

        it("i32.rem_s: -10 rem 3 = -1 (truncated remainder)", function()
            local inst = new_instance(binop_module(0x6F))
            assert.equals(-1, inst:call("op", {-10, 3})[1])
        end)
    end)

    -- =========================================================================
    -- Bitwise operations: and, or, xor, shl, shr_s
    -- =========================================================================

    describe("i32 bitwise operations", function()
        local function binop_module(opcode)
            local body = b(0x20, 0x00, 0x20, 0x01, opcode, 0x0B)
            return WASM_HEADER
                .. type_sec({{params={0x7F, 0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("op", 0)})
                .. code_sec({code_entry({}, body)})
        end

        it("i32.and: 0xFF00 & 0x0FF0 = 0x0F00", function()
            local inst = new_instance(binop_module(0x71))
            assert.equals(0x0F00, inst:call("op", {0xFF00, 0x0FF0})[1])
        end)

        it("i32.and: 5 & 3 = 1", function()
            -- 5 = 0b101, 3 = 0b011, AND = 0b001 = 1
            local inst = new_instance(binop_module(0x71))
            assert.equals(1, inst:call("op", {5, 3})[1])
        end)

        it("i32.or: 5 | 3 = 7", function()
            -- 5 = 0b101, 3 = 0b011, OR = 0b111 = 7
            local inst = new_instance(binop_module(0x72))
            assert.equals(7, inst:call("op", {5, 3})[1])
        end)

        it("i32.xor: 5 ^ 3 = 6", function()
            -- 5 = 0b101, 3 = 0b011, XOR = 0b110 = 6
            local inst = new_instance(binop_module(0x73))
            assert.equals(6, inst:call("op", {5, 3})[1])
        end)

        it("i32.xor: n ^ n = 0 (identity trick)", function()
            local inst = new_instance(binop_module(0x73))
            assert.equals(0, inst:call("op", {42, 42})[1])
        end)

        it("i32.shl: 1 << 3 = 8", function()
            local inst = new_instance(binop_module(0x74))
            assert.equals(8, inst:call("op", {1, 3})[1])
        end)

        it("i32.shl: 1 << 31 = INT_MIN (sign bit)", function()
            local inst = new_instance(binop_module(0x74))
            assert.equals(-2147483648, inst:call("op", {1, 31})[1])
        end)

        it("i32.shr_s: 8 >> 1 = 4 (arithmetic)", function()
            local inst = new_instance(binop_module(0x75))
            assert.equals(4, inst:call("op", {8, 1})[1])
        end)

        it("i32.shr_s: -8 >> 1 = -4 (arithmetic, sign extends)", function()
            local inst = new_instance(binop_module(0x75))
            assert.equals(-4, inst:call("op", {-8, 1})[1])
        end)
    end)

    -- =========================================================================
    -- Comparisons: eq, ne, lt_s, le_s, gt_s, ge_s
    -- =========================================================================

    describe("i32 comparisons", function()
        local function cmp_module(opcode)
            local body = b(0x20, 0x00, 0x20, 0x01, opcode, 0x0B)
            return WASM_HEADER
                .. type_sec({{params={0x7F, 0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("cmp", 0)})
                .. code_sec({code_entry({}, body)})
        end

        it("i32.eq: 5 == 5 → 1", function()
            local inst = new_instance(cmp_module(0x46))
            assert.equals(1, inst:call("cmp", {5, 5})[1])
        end)

        it("i32.eq: 5 == 6 → 0", function()
            local inst = new_instance(cmp_module(0x46))
            assert.equals(0, inst:call("cmp", {5, 6})[1])
        end)

        it("i32.ne: 5 != 6 → 1", function()
            local inst = new_instance(cmp_module(0x47))
            assert.equals(1, inst:call("cmp", {5, 6})[1])
        end)

        it("i32.ne: 5 != 5 → 0", function()
            local inst = new_instance(cmp_module(0x47))
            assert.equals(0, inst:call("cmp", {5, 5})[1])
        end)

        it("i32.lt_s: 3 < 5 → 1", function()
            local inst = new_instance(cmp_module(0x48))
            assert.equals(1, inst:call("cmp", {3, 5})[1])
        end)

        it("i32.lt_s: -1 < 0 → 1 (signed comparison)", function()
            local inst = new_instance(cmp_module(0x48))
            assert.equals(1, inst:call("cmp", {-1, 0})[1])
        end)

        it("i32.lt_s: 5 < 3 → 0", function()
            local inst = new_instance(cmp_module(0x48))
            assert.equals(0, inst:call("cmp", {5, 3})[1])
        end)

        it("i32.le_s: 5 <= 5 → 1", function()
            local inst = new_instance(cmp_module(0x4C))
            assert.equals(1, inst:call("cmp", {5, 5})[1])
        end)

        it("i32.le_s: 5 <= 4 → 0", function()
            local inst = new_instance(cmp_module(0x4C))
            assert.equals(0, inst:call("cmp", {5, 4})[1])
        end)

        it("i32.gt_s: 5 > 3 → 1", function()
            local inst = new_instance(cmp_module(0x4A))
            assert.equals(1, inst:call("cmp", {5, 3})[1])
        end)

        it("i32.ge_s: 5 >= 5 → 1", function()
            local inst = new_instance(cmp_module(0x4E))
            assert.equals(1, inst:call("cmp", {5, 5})[1])
        end)

        it("i32.ge_s: 4 >= 5 → 0", function()
            local inst = new_instance(cmp_module(0x4E))
            assert.equals(0, inst:call("cmp", {4, 5})[1])
        end)
    end)

    -- =========================================================================
    -- nop and drop
    -- =========================================================================

    describe("stack manipulation", function()
        it("nop does nothing", function()
            -- Body: nop; i32.const 7; end
            local body = b(0x01, 0x41, 0x07, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})
            local inst = new_instance(wasm)
            assert.equals(7, inst:call("f", {})[1])
        end)

        it("drop discards top of stack", function()
            -- Body: i32.const 99; i32.const 42; drop; end  → result is 99
            -- Note: 99 in signed LEB128 requires two bytes: 0xE3, 0x00
            local body = b(0x41, 0xE3, 0x00, 0x41, 0x2A, 0x1A, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})
            local inst = new_instance(wasm)
            assert.equals(99, inst:call("f", {})[1])
        end)

        it("select picks first value when condition is nonzero", function()
            -- Body: i32.const 10; i32.const 20; i32.const 1; select; end
            -- Should push 10 (condition=1 → pick val1=10)
            local body = b(0x41, 0x0A, 0x41, 0x14, 0x41, 0x01, 0x1B, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})
            local inst = new_instance(wasm)
            assert.equals(10, inst:call("f", {})[1])
        end)

        it("select picks second value when condition is zero", function()
            -- Body: i32.const 10; i32.const 20; i32.const 0; select; end
            -- Should push 20 (condition=0 → pick val2=20)
            local body = b(0x41, 0x0A, 0x41, 0x14, 0x41, 0x00, 0x1B, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})
            local inst = new_instance(wasm)
            assert.equals(20, inst:call("f", {})[1])
        end)
    end)

    -- =========================================================================
    -- return instruction (explicit early return)
    -- =========================================================================

    describe("return", function()
        it("explicit return exits function early", function()
            -- Body: i32.const 1; return; i32.const 2; end
            -- Should return 1, never reaching i32.const 2
            local body = b(0x41, 0x01, 0x0F, 0x41, 0x02, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})
            local inst = new_instance(wasm)
            assert.equals(1, inst:call("f", {})[1])
        end)
    end)

    -- =========================================================================
    -- Global variables
    -- =========================================================================
    --
    -- Globals persist their value across function calls and can be read and
    -- written with global.get (0x23) and global.set (0x24).

    describe("global variables", function()
        it("global.get reads initial global value", function()
            -- Global 0: const i32 = 42
            -- Function: () → i32 = { global.get 0; end }
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. global_sec({{val_type=0x7F, mutable=false, init=b(0x41, 0x2A, 0x0B)}})
                .. export_sec({export_func("get_g", 0), export_global("g", 0)})
                .. code_sec({code_entry({}, b(0x23, 0x00, 0x0B))})

            local inst = new_instance(wasm)
            assert.equals(42, inst:call("get_g", {})[1])
            assert.equals(42, inst:get_global("g"))
        end)

        it("global.set updates a mutable global", function()
            -- Global 0: mutable i32 = 0
            -- Function set: (i32) → void = { global.set 0; end }
            -- Function get: () → i32    = { global.get 0; end }
            local wasm = WASM_HEADER
                .. type_sec({
                    {params={0x7F}, results={}},    -- type 0: (i32) → ()
                    {params={},     results={0x7F}}, -- type 1: () → i32
                })
                .. func_sec({0, 1})
                .. global_sec({{val_type=0x7F, mutable=true, init=b(0x41, 0x00, 0x0B)}})
                .. export_sec({
                    export_func("set_g", 0),
                    export_func("get_g", 1),
                    export_global("g", 0),
                })
                .. code_sec({
                    code_entry({}, b(0x20, 0x00, 0x24, 0x00, 0x0B)),  -- set func
                    code_entry({}, b(0x23, 0x00, 0x0B)),               -- get func
                })

            local inst = new_instance(wasm)
            assert.equals(0, inst:get_global("g"))
            inst:call("set_g", {99})
            assert.equals(99, inst:get_global("g"))
            assert.equals(99, inst:call("get_g", {})[1])
        end)

        it("set_global API sets a mutable global by export name", function()
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. global_sec({{val_type=0x7F, mutable=true, init=b(0x41, 0x00, 0x0B)}})
                .. export_sec({export_func("get_g", 0), export_global("counter", 0)})
                .. code_sec({code_entry({}, b(0x23, 0x00, 0x0B))})

            local inst = new_instance(wasm)
            inst:set_global("counter", 777)
            assert.equals(777, inst:call("get_g", {})[1])
        end)

        it("set_global errors on immutable global", function()
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. global_sec({{val_type=0x7F, mutable=false, init=b(0x41, 0x01, 0x0B)}})
                .. export_sec({export_func("get_g", 0), export_global("g", 0)})
                .. code_sec({code_entry({}, b(0x23, 0x00, 0x0B))})

            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:set_global("g", 5)
            end)
        end)
    end)

    -- =========================================================================
    -- Memory operations
    -- =========================================================================
    --
    -- WebAssembly linear memory: a flat byte array. We test the store/load
    -- roundtrip and the memory_read/memory_write host API.

    describe("memory operations", function()
        it("memory_read and memory_write roundtrip", function()
            -- Module with 1 page of memory
            local wasm = WASM_HEADER
                .. mem_sec({{min=1}})
                .. export_sec({export_mem("mem", 0)})

            local inst = new_instance(wasm)
            inst:memory_write(0, {0xDE, 0xAD, 0xBE, 0xEF})
            local bytes = inst:memory_read(0, 4)
            assert.equals(0xDE, bytes[1])
            assert.equals(0xAD, bytes[2])
            assert.equals(0xBE, bytes[3])
            assert.equals(0xEF, bytes[4])
        end)

        it("i32.store and i32.load roundtrip", function()
            -- Module with 1 page of memory
            -- store func: (i32 addr, i32 val) → void
            --   local.get 0; local.get 1; i32.store align=0 offset=0; end
            -- load func: (i32 addr) → i32
            --   local.get 0; i32.load align=0 offset=0; end
            local store_body = b(0x20, 0x00, 0x20, 0x01, 0x36, 0x00, 0x00, 0x0B)
            local load_body  = b(0x20, 0x00, 0x28, 0x00, 0x00, 0x0B)

            local wasm = WASM_HEADER
                .. type_sec({
                    {params={0x7F, 0x7F}, results={}},   -- type 0: store
                    {params={0x7F},       results={0x7F}},-- type 1: load
                })
                .. func_sec({0, 1})
                .. mem_sec({{min=1}})
                .. export_sec({
                    export_func("store", 0),
                    export_func("load",  1),
                })
                .. code_sec({
                    code_entry({}, store_body),
                    code_entry({}, load_body),
                })

            local inst = new_instance(wasm)

            -- Store 12345 at address 0
            inst:call("store", {0, 12345})
            assert.equals(12345, inst:call("load", {0})[1])

            -- Store at offset 4
            inst:call("store", {4, -99})
            assert.equals(-99, inst:call("load", {4})[1])
        end)

        it("memory.size returns page count", function()
            -- memory.size pushes the current page count
            local body = b(0x3F, 0x00, 0x0B)  -- memory.size reserved=0; end
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. mem_sec({{min=2}})
                .. export_sec({export_func("size", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(2, inst:call("size", {})[1])
        end)

        it("memory.grow increases size", function()
            -- memory.grow pops delta, pushes old size (or -1)
            -- Body: i32.const 1; memory.grow 0; end  → pushes old size (=1)
            local body = b(0x41, 0x01, 0x40, 0x00, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. mem_sec({{min=1}})
                .. export_sec({export_func("grow", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            local old_size = inst:call("grow", {})[1]
            assert.equals(1, old_size)         -- old size was 1 page
            assert.equals(2, inst.memory.size_pages)  -- now 2 pages
        end)

        it("memory_read traps on out-of-bounds access", function()
            local wasm = WASM_HEADER .. mem_sec({{min=1}})
            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:memory_read(65536, 1)  -- 1 page = 65536 bytes; offset 65536 is OOB
            end)
        end)
    end)

    -- =========================================================================
    -- Control flow: block, loop, br, br_if
    -- =========================================================================
    --
    -- Wasm's structured control flow is one of its key safety properties.
    -- There are no arbitrary jumps; only structured branches.

    describe("control flow", function()
        it("block: falls through without branching", function()
            -- block void
            --   i32.const 42
            --   drop
            -- end
            -- i32.const 7
            -- end
            local body = b(
                0x02, 0x40,   -- block (void)
                0x41, 0x2A,   --   i32.const 42
                0x1A,         --   drop
                0x0B,         -- end (block)
                0x41, 0x07,   -- i32.const 7
                0x0B          -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(7, inst:call("f", {})[1])
        end)

        it("br breaks out of a block (forward jump)", function()
            -- block (void)
            --   i32.const 1
            --   br 0        ← jump to end of block
            --   drop        ← never executed
            -- end
            -- i32.const 99
            -- end
            -- The br 0 should skip the drop and continue after the block.
            -- But wait: the 1 is dropped by... let's think again.
            -- Actually: we need a void block to avoid stack mismatch.
            -- After br 0 jumps past the end of block, stack should have 1 on it
            -- then we push 99... that's two values. Let's do it properly:
            -- Drop the 1 inside the block before br, then push 99 after.
            local body = b(
                0x02, 0x40,   -- block (void)
                0x41, 0x01,   --   i32.const 1
                0x1A,         --   drop           (clear stack before br)
                0x0C, 0x00,   --   br 0            (jump to end of block)
                0x41, 0xFF, 0x00,   --   i32.const 127 (skipped)
                0x0B,         -- end (block)
                0x41, 0xE3, 0x00,   -- i32.const 99 (two-byte signed LEB128)
                0x0B          -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(99, inst:call("f", {})[1])
        end)

        it("br_if branches when condition is nonzero", function()
            -- block (void)
            --   i32.const 1
            --   br_if 0      ← branch since 1 != 0
            --   drop         ← never
            -- end
            -- i32.const 100
            -- end
            local body = b(
                0x02, 0x40,       -- block (void)
                0x41, 0x01,       --   i32.const 1
                0x0D, 0x00,       --   br_if 0
                0x41, 0x07, 0x1A, --   i32.const 7; drop (skipped)
                0x0B,             -- end (block)
                0x41, 0x64,       -- i32.const 100
                0x0B              -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(100, inst:call("f", {})[1])
        end)

        it("br_if falls through when condition is zero", function()
            -- block (void)
            --   i32.const 0
            --   br_if 0      ← does NOT branch (condition = 0)
            --   i32.const 5  ← IS executed
            --   drop
            -- end
            -- i32.const 200
            -- end
            local body = b(
                0x02, 0x40,           -- block (void)
                0x41, 0x00,           --   i32.const 0
                0x0D, 0x00,           --   br_if 0 (not taken)
                0x41, 0x05, 0x1A,     --   i32.const 5; drop
                0x0B,                 -- end (block)
                0x41, 0xC8, 0x01,     -- i32.const 200 (LEB: 0xC8 0x01)
                0x0B                  -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(200, inst:call("f", {})[1])
        end)

        it("loop with br: simple countdown", function()
            -- This function counts down from N to 0 using a loop.
            -- Type: (i32) → i32  — takes N, returns 0
            --
            -- (func (param $n i32) (result i32)
            --   (loop $loop
            --     (local.get 0)     ;; push n
            --     (i32.eqz)         ;; n == 0?
            --     (br_if 1)         ;; if n==0, exit function (br depth 1 from inside loop)
            --     (local.get 0)     ;; push n
            --     (i32.const -1)    ;; push -1
            --     (i32.add)         ;; n - 1
            --     (local.set 0)     ;; n = n - 1
            --     (br 0)            ;; loop again
            --   )
            --   (local.get 0)       ;; push 0 as result
            -- )
            --
            -- The br_if 1 at depth 1 jumps PAST the function (exits loop + function).
            -- Wait, that exits the function via the outer implicit function block.
            -- Let's simplify: wrap in a block to exit cleanly.
            --
            -- (func (param $n i32) (result i32)
            --   (block $exit             ;; depth 0 from inside loop (depth 1 from func)
            --     (loop $loop            ;; depth 0 inside loop
            --       (local.get 0)
            --       (i32.eqz)
            --       (br_if 1)            ;; exit block if n==0
            --       (local.get 0)
            --       (i32.const -1)       ;; -1
            --       (i32.add)
            --       (local.set 0)
            --       (br 0)               ;; restart loop
            --     )
            --   )
            --   (local.get 0)            ;; return 0 (since loop exits when n==0)
            -- )
            local body = b(
                0x02, 0x40,       -- block void ($exit)
                0x03, 0x40,       --   loop void ($loop)
                0x20, 0x00,       --     local.get 0
                0x45,             --     i32.eqz
                0x0D, 0x01,       --     br_if 1   (exit block if n==0)
                0x20, 0x00,       --     local.get 0
                0x41, 0x7F,       --     i32.const -1  (LEB128 signed: 0x7F = -1)
                0x6A,             --     i32.add
                0x21, 0x00,       --     local.set 0
                0x0C, 0x00,       --     br 0       (restart loop)
                0x0B,             --   end (loop)
                0x0B,             -- end (block)
                0x20, 0x00,       -- local.get 0
                0x0B              -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("countdown", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(0, inst:call("countdown", {5})[1])
            assert.equals(0, inst:call("countdown", {0})[1])
            assert.equals(0, inst:call("countdown", {3})[1])
        end)
    end)

    -- =========================================================================
    -- if / else
    -- =========================================================================

    describe("if / else", function()
        it("if: executes then-arm when condition is nonzero", function()
            -- Type: (i32) → i32
            -- if (local.get 0)
            --   i32.const 1
            -- else
            --   i32.const 0
            -- end
            local body = b(
                0x20, 0x00,   -- local.get 0
                0x04, 0x7F,   -- if i32
                0x41, 0x01,   --   i32.const 1
                0x05,         -- else
                0x41, 0x00,   --   i32.const 0
                0x0B,         -- end (if)
                0x0B          -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("bool_id", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.equals(1, inst:call("bool_id", {1})[1])
            assert.equals(1, inst:call("bool_id", {99})[1])
            assert.equals(0, inst:call("bool_id", {0})[1])
        end)

        it("if without else: executes body when condition nonzero", function()
            -- Type: (i32) → i32
            -- local 1 = 0 (extra local)
            -- if (local.get 0)
            --   i32.const 42; local.set 1
            -- end
            -- local.get 1
            local body = b(
                0x20, 0x00,   -- local.get 0
                0x04, 0x40,   -- if void
                0x41, 0x2A,   --   i32.const 42
                0x21, 0x01,   --   local.set 1
                0x0B,         -- end (if)
                0x20, 0x01,   -- local.get 1
                0x0B          -- end (function)
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("maybe42", 0)})
                .. code_sec({code_entry({{count=1, type=0x7F}}, body)})

            local inst = new_instance(wasm)
            assert.equals(42, inst:call("maybe42", {1})[1])
            assert.equals(0,  inst:call("maybe42", {0})[1])
        end)
    end)

    -- =========================================================================
    -- Function calls
    -- =========================================================================
    --
    -- A function can call other functions with the `call` instruction.
    -- This tests the full activation frame mechanism: argument passing,
    -- call stack, and result returning.

    describe("function calls", function()
        it("call: one function calls another", function()
            -- Function 0: double(x: i32) → i32 = x + x
            -- Function 1: quad(x: i32) → i32 = double(double(x))
            -- Only quad is exported.
            local double_body = b(
                0x20, 0x00,   -- local.get 0
                0x20, 0x00,   -- local.get 0
                0x6A,         -- i32.add
                0x0B          -- end
            )
            local quad_body = b(
                0x20, 0x00,       -- local.get 0
                0x10, 0x00,       -- call 0 (double)
                0x10, 0x00,       -- call 0 (double again)
                0x0B              -- end
            )
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0, 0})  -- both use type 0
                .. export_sec({export_func("quad", 1)})
                .. code_sec({
                    code_entry({}, double_body),
                    code_entry({}, quad_body),
                })

            local inst = new_instance(wasm)
            assert.equals(4,   inst:call("quad", {1})[1])
            assert.equals(8,   inst:call("quad", {2})[1])
            assert.equals(20,  inst:call("quad", {5})[1])
        end)

        it("call_by_index works for direct invocation", function()
            -- Simple function at index 0: returns 42
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("f", 0)})
                .. code_sec({code_entry({}, b(0x41, 0x2A, 0x0B))})

            local inst = new_instance(wasm)
            local result = inst:call_by_index(0, {})
            assert.equals(42, result[1])
        end)
    end)

    -- =========================================================================
    -- Error cases
    -- =========================================================================

    describe("error handling", function()
        it("call to nonexistent export raises error", function()
            local wasm = WASM_HEADER
            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:call("no_such_func", {})
            end)
        end)

        it("get_global for nonexistent export raises error", function()
            local wasm = WASM_HEADER
            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:get_global("nonexistent")
            end)
        end)

        it("unreachable instruction raises error", function()
            local body = b(0x00, 0x0B)  -- unreachable; end
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={}}})
                .. func_sec({0})
                .. export_sec({export_func("trap", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:call("trap", {})
            end)
        end)

        it("memory access out of bounds raises error", function()
            local load_body = b(0x20, 0x00, 0x28, 0x00, 0x00, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. mem_sec({{min=1}})
                .. export_sec({export_func("load", 0)})
                .. code_sec({code_entry({}, load_body)})

            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:call("load", {70000})  -- beyond 1 page (65536 bytes)
            end)
        end)

        it("divide by zero raises error", function()
            local body = b(0x41, 0x05, 0x41, 0x00, 0x6D, 0x0B)
            local wasm = WASM_HEADER
                .. type_sec({{params={}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("div0", 0)})
                .. code_sec({code_entry({}, body)})

            local inst = new_instance(wasm)
            assert.has_error(function()
                inst:call("div0", {})
            end)
        end)
    end)

    -- =========================================================================
    -- Multi-function module (fibonacci)
    -- =========================================================================
    --
    -- Fibonacci is a classic recursive algorithm. It's a good stress test for
    -- the call stack because each call to fib(n) calls fib(n-1) and fib(n-2).
    --
    -- fib(0) = 0
    -- fib(1) = 1
    -- fib(n) = fib(n-1) + fib(n-2)  for n >= 2

    describe("fibonacci (recursive)", function()
        it("computes small fibonacci numbers correctly", function()
            -- func fib(n: i32) → i32:
            --   if n <= 1: return n
            --   return fib(n-1) + fib(n-2)
            --
            -- Encoding:
            --   local.get 0          ; push n
            --   i32.const 1          ; push 1
            --   i32.le_s             ; n <= 1?
            --   if i32               ; if true:
            --     local.get 0        ;   push n (return value)
            --   else                 ; else:
            --     local.get 0        ;   push n
            --     i32.const 1        ;   push 1
            --     i32.sub            ;   n - 1
            --     call 0             ;   fib(n-1)
            --     local.get 0        ;   push n
            --     i32.const 2        ;   push 2
            --     i32.sub            ;   n - 2
            --     call 0             ;   fib(n-2)
            --     i32.add            ;   fib(n-1) + fib(n-2)
            --   end
            local fib_body = b(
                0x20, 0x00,       -- local.get 0 (n)
                0x41, 0x01,       -- i32.const 1
                0x4C,             -- i32.le_s (n <= 1)
                0x04, 0x7F,       -- if i32
                0x20, 0x00,       --   local.get 0 (return n)
                0x05,             -- else
                0x20, 0x00,       --   local.get 0
                0x41, 0x01,       --   i32.const 1
                0x6B,             --   i32.sub (n-1)
                0x10, 0x00,       --   call 0 (fib(n-1))
                0x20, 0x00,       --   local.get 0
                0x41, 0x02,       --   i32.const 2
                0x6B,             --   i32.sub (n-2)
                0x10, 0x00,       --   call 0 (fib(n-2))
                0x6A,             --   i32.add
                0x0B,             -- end (if)
                0x0B              -- end (function)
            )

            local wasm = WASM_HEADER
                .. type_sec({{params={0x7F}, results={0x7F}}})
                .. func_sec({0})
                .. export_sec({export_func("fib", 0)})
                .. code_sec({code_entry({}, fib_body)})

            local inst = new_instance(wasm)

            assert.equals(0,  inst:call("fib", {0})[1])
            assert.equals(1,  inst:call("fib", {1})[1])
            assert.equals(1,  inst:call("fib", {2})[1])
            assert.equals(2,  inst:call("fib", {3})[1])
            assert.equals(3,  inst:call("fib", {4})[1])
            assert.equals(5,  inst:call("fib", {5})[1])
            assert.equals(8,  inst:call("fib", {6})[1])
            assert.equals(55, inst:call("fib", {10})[1])
        end)
    end)

end)

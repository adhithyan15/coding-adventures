-- Tests for wasm_execution
--
-- Comprehensive test suite covering:
--   - Value constructors (i32, i64, f32, f64)
--   - Type-safe extraction helpers (as_i32, etc.)
--   - LinearMemory (load/store, bounds checks, grow)
--   - Table (get/set, bounds checks, grow)
--   - Decoder (decode_function_body, build_control_flow_map)
--   - Constant expression evaluation
--   - WasmExecutionEngine (call_function with various instructions)

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

-- The wasm_execution module depends on packages from sibling directories.
-- We add all of them to the path so require() can find them.
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../virtual_machine/src/?.lua;" .. "../../virtual_machine/src/?/init.lua;" .. package.path

local m = require("coding_adventures.wasm_execution")


-- ============================================================================
-- VALUE CONSTRUCTORS
-- ============================================================================

describe("wasm_execution", function()

    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    describe("i32 constructor", function()
        it("wraps positive values", function()
            local v = m.i32(42)
            assert.equals(0x7F, v.type)
            assert.equals(42, v.value)
        end)

        it("wraps negative values", function()
            local v = m.i32(-1)
            assert.equals(0x7F, v.type)
            assert.equals(-1, v.value)
        end)

        it("wraps values exceeding 32 bits", function()
            local v = m.i32(0x100000000)  -- 2^32
            assert.equals(0, v.value)
        end)

        it("wraps to negative for high bit set", function()
            local v = m.i32(0x80000000)
            assert.equals(-2147483648, v.value)
        end)

        it("handles zero", function()
            local v = m.i32(0)
            assert.equals(0, v.value)
        end)
    end)

    describe("i64 constructor", function()
        it("creates an i64 value", function()
            local v = m.i64(123456789)
            assert.equals(0x7E, v.type)
            assert.equals(123456789, v.value)
        end)

        it("handles negative values", function()
            local v = m.i64(-42)
            assert.equals(-42, v.value)
        end)
    end)

    describe("f32 constructor", function()
        it("rounds to single precision", function()
            local v = m.f32(3.14)
            assert.equals(0x7D, v.type)
            -- f32 rounds, so it won't be exactly 3.14
            assert.is_near(3.14, v.value, 0.01)
        end)

        it("handles zero", function()
            local v = m.f32(0.0)
            assert.equals(0.0, v.value)
        end)
    end)

    describe("f64 constructor", function()
        it("preserves double precision", function()
            local v = m.f64(3.141592653589793)
            assert.equals(0x7C, v.type)
            assert.equals(3.141592653589793, v.value)
        end)
    end)

    describe("default_value", function()
        it("returns zero i32 for I32 type", function()
            local v = m.default_value(0x7F)
            assert.equals(0x7F, v.type)
            assert.equals(0, v.value)
        end)

        it("returns zero i64 for I64 type", function()
            local v = m.default_value(0x7E)
            assert.equals(0x7E, v.type)
            assert.equals(0, v.value)
        end)

        it("returns zero f32 for F32 type", function()
            local v = m.default_value(0x7D)
            assert.equals(0x7D, v.type)
            assert.equals(0.0, v.value)
        end)

        it("returns zero f64 for F64 type", function()
            local v = m.default_value(0x7C)
            assert.equals(0x7C, v.type)
            assert.equals(0.0, v.value)
        end)
    end)


    -- ========================================================================
    -- TYPE-SAFE EXTRACTION
    -- ========================================================================

    describe("as_i32", function()
        it("extracts i32 values", function()
            local v = m.i32(99)
            assert.equals(99, m.as_i32(v))
        end)

        it("traps on type mismatch", function()
            local v = m.i64(99)
            assert.has_error(function() m.as_i32(v) end, "TrapError: type mismatch: expected i32, got i64")
        end)
    end)

    describe("as_i64", function()
        it("extracts i64 values", function()
            local v = m.i64(99)
            assert.equals(99, m.as_i64(v))
        end)
    end)

    describe("as_f32", function()
        it("extracts f32 values", function()
            local v = m.f32(1.5)
            assert.equals(1.5, m.as_f32(v))
        end)
    end)

    describe("as_f64", function()
        it("extracts f64 values", function()
            local v = m.f64(2.5)
            assert.equals(2.5, m.as_f64(v))
        end)
    end)


    -- ========================================================================
    -- LINEAR MEMORY
    -- ========================================================================

    describe("LinearMemory", function()
        it("creates memory with initial pages", function()
            local mem = m.LinearMemory.new(1, nil)
            assert.equals(1, mem:size())
            assert.equals(65536, mem:byte_length())
        end)

        it("stores and loads i32", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i32(0, 0x12345678)
            assert.equals(0x12345678, mem:load_i32(0))
        end)

        it("stores and loads i32 negative", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i32(0, -1)
            assert.equals(-1, mem:load_i32(0))
        end)

        it("stores and loads i64", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i64(0, 0x123456789ABCDEF0)
            assert.equals(0x123456789ABCDEF0, mem:load_i64(0))
        end)

        it("stores and loads f32", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_f32(0, 3.14)
            assert.is_near(3.14, mem:load_f32(0), 0.001)
        end)

        it("stores and loads f64", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_f64(0, 3.141592653589793)
            assert.equals(3.141592653589793, mem:load_f64(0))
        end)

        it("stores and loads 8-bit values", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i32_8(0, 0xAB)
            assert.equals(0xAB, mem:load_i32_8u(0))
        end)

        it("sign-extends 8-bit loads", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i32_8(0, 0xFF)
            assert.equals(-1, mem:load_i32_8s(0))
        end)

        it("stores and loads 16-bit values", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i32_16(0, 0xBEEF)
            assert.equals(0xBEEF, mem:load_i32_16u(0))
        end)

        it("sign-extends 16-bit loads", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:store_i32_16(0, 0xFFFF)
            assert.equals(-1, mem:load_i32_16s(0))
        end)

        it("traps on out-of-bounds access", function()
            local mem = m.LinearMemory.new(1, nil)
            assert.has_error(function() mem:load_i32(65536) end)
        end)

        it("grows memory", function()
            local mem = m.LinearMemory.new(1, 10)
            local old = mem:grow(2)
            assert.equals(1, old)
            assert.equals(3, mem:size())
        end)

        it("fails to grow beyond max", function()
            local mem = m.LinearMemory.new(1, 2)
            local result = mem:grow(5)
            assert.equals(-1, result)
            assert.equals(1, mem:size())
        end)

        it("writes raw bytes", function()
            local mem = m.LinearMemory.new(1, nil)
            mem:write_bytes(0, "hello")
            assert.equals(string.byte("h"), mem.data[1])
            assert.equals(string.byte("e"), mem.data[2])
            assert.equals(string.byte("l"), mem.data[3])
        end)
    end)


    -- ========================================================================
    -- TABLE
    -- ========================================================================

    describe("Table", function()
        it("creates a table with initial size", function()
            local tbl = m.Table.new(10, nil)
            assert.equals(10, tbl:size())
        end)

        it("gets nil for uninitialized entries", function()
            local tbl = m.Table.new(10, nil)
            assert.is_nil(tbl:get(0))
        end)

        it("sets and gets entries", function()
            local tbl = m.Table.new(10, nil)
            tbl:set(3, 42)
            assert.equals(42, tbl:get(3))
        end)

        it("traps on out-of-bounds get", function()
            local tbl = m.Table.new(5, nil)
            assert.has_error(function() tbl:get(5) end)
        end)

        it("traps on out-of-bounds set", function()
            local tbl = m.Table.new(5, nil)
            assert.has_error(function() tbl:set(5, 0) end)
        end)

        it("grows the table", function()
            local tbl = m.Table.new(5, 20)
            local old = tbl:grow(3)
            assert.equals(5, old)
            assert.equals(8, tbl:size())
        end)

        it("fails to grow beyond max", function()
            local tbl = m.Table.new(5, 6)
            local result = tbl:grow(5)
            assert.equals(-1, result)
            assert.equals(5, tbl:size())
        end)
    end)


    -- ========================================================================
    -- DECODER
    -- ========================================================================

    describe("decode_function_body", function()
        it("decodes simple bytecodes", function()
            -- local.get 0, local.get 0, i32.mul, end
            local body = { body = { 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B } }
            local instrs = m.decode_function_body(body)
            assert.equals(4, #instrs)
            assert.equals(0x20, instrs[1].opcode)
            assert.equals(0, instrs[1].operand)
            assert.equals(0x6C, instrs[3].opcode)
            assert.equals(0x0B, instrs[4].opcode)
        end)

        it("decodes i32.const with LEB128", function()
            -- i32.const 5
            local body = { body = { 0x41, 0x05, 0x0B } }
            local instrs = m.decode_function_body(body)
            assert.equals(2, #instrs)
            assert.equals(0x41, instrs[1].opcode)
            assert.equals(5, instrs[1].operand)
        end)
    end)


    describe("build_control_flow_map", function()
        it("maps block to end", function()
            -- block, nop, end
            local instrs = {
                { opcode = 0x02, operand = 0x40 },  -- block (empty)
                { opcode = 0x01 },                    -- nop
                { opcode = 0x0B },                    -- end
            }
            local cfmap = m.build_control_flow_map(instrs)
            assert.is_not_nil(cfmap[1])
            assert.equals(3, cfmap[1].end_pc)
        end)

        it("maps if/else to end", function()
            -- if, nop, else, nop, end
            local instrs = {
                { opcode = 0x04, operand = 0x40 },  -- if
                { opcode = 0x01 },                    -- nop
                { opcode = 0x05 },                    -- else
                { opcode = 0x01 },                    -- nop
                { opcode = 0x0B },                    -- end
            }
            local cfmap = m.build_control_flow_map(instrs)
            assert.is_not_nil(cfmap[1])
            assert.equals(5, cfmap[1].end_pc)
            assert.equals(3, cfmap[1].else_pc)
        end)
    end)


    -- ========================================================================
    -- CONSTANT EXPRESSION EVALUATOR
    -- ========================================================================

    describe("evaluate_const_expr", function()
        it("evaluates i32.const", function()
            local result = m.evaluate_const_expr({ 0x41, 0x05, 0x0B })
            assert.equals(0x7F, result.type)
            assert.equals(5, result.value)
        end)

        it("evaluates i64.const", function()
            local result = m.evaluate_const_expr({ 0x42, 0x0A, 0x0B })
            assert.equals(0x7E, result.type)
            assert.equals(10, result.value)
        end)

        it("returns i32(0) for empty expr", function()
            local result = m.evaluate_const_expr({})
            assert.equals(0x7F, result.type)
            assert.equals(0, result.value)
        end)
    end)


    -- ========================================================================
    -- WASM EXECUTION ENGINE
    -- ========================================================================

    describe("WasmExecutionEngine", function()

        it("computes square(5) = 25", function()
            -- Function: local.get 0, local.get 0, i32.mul, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, { m.i32(5) })
            assert.equals(1, #results)
            assert.equals(25, results[1].value)
        end)

        it("computes square(0) = 0", function()
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, { m.i32(0) })
            assert.equals(0, results[1].value)
        end)

        it("computes square(-3) = 9", function()
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, { m.i32(-3) })
            assert.equals(9, results[1].value)
        end)

        it("executes i32.add", function()
            -- i32.const 3, i32.const 4, i32.add, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x41, 0x03, 0x41, 0x04, 0x6A, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(7, results[1].value)
        end)

        it("executes i32.sub", function()
            -- i32.const 10, i32.const 3, i32.sub, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x41, 0x0A, 0x41, 0x03, 0x6B, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(7, results[1].value)
        end)

        it("executes i32.div_s with trap on zero", function()
            -- i32.const 10, i32.const 0, i32.div_s, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x41, 0x0A, 0x41, 0x00, 0x6D, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            assert.has_error(function() engine:call_function(0, {}) end)
        end)

        it("executes i32 comparison (eqz)", function()
            -- i32.const 0, i32.eqz, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x41, 0x00, 0x45, 0x0B },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(1, results[1].value)
        end)

        it("executes local.set and local.get", function()
            -- param i32, local i32
            -- local.get 0, i32.const 10, i32.add, local.set 1, local.get 1, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = { { count = 1, type = 0x7F } },
                        body = {
                            0x20, 0x00,  -- local.get 0
                            0x41, 0x0A,  -- i32.const 10
                            0x6A,        -- i32.add
                            0x21, 0x01,  -- local.set 1
                            0x20, 0x01,  -- local.get 1
                            0x0B,        -- end
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, { m.i32(5) })
            assert.equals(15, results[1].value)
        end)

        it("executes block and br", function()
            -- block, i32.const 42, br 0, i32.const 99, end, end
            -- The br 0 should skip i32.const 99 and jump to end of block.
            -- But only the last value before the block end matters.
            -- Since block arity is 0 (empty block type), the 42 gets dropped by br.
            -- After the block, we push a result.
            --
            -- Actually, let's do a simpler test: block with result type.
            -- block (result i32), i32.const 42, br 0, end, end_func
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x02, 0x7F,  -- block (result i32)
                            0x41, 0x2A,  -- i32.const 42
                            0x0C, 0x00,  -- br 0
                            0x0B,        -- end (block)
                            0x0B,        -- end (func)
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(42, results[1].value)
        end)

        it("executes if/else", function()
            -- param i32, result i32
            -- local.get 0, if (result i32), i32.const 1, else, i32.const 0, end, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x20, 0x00,  -- local.get 0
                            0x04, 0x7F,  -- if (result i32)
                            0x41, 0x01,  -- i32.const 1
                            0x05,        -- else
                            0x41, 0x00,  -- i32.const 0
                            0x0B,        -- end (if)
                            0x0B,        -- end (func)
                        },
                    },
                },
                host_functions = { nil },
            })

            -- truthy case
            local results = engine:call_function(0, { m.i32(5) })
            assert.equals(1, results[1].value)
        end)

        it("executes if/else false branch", function()
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x20, 0x00,  -- local.get 0
                            0x04, 0x7F,  -- if (result i32)
                            0x41, 0x01,  -- i32.const 1
                            0x05,        -- else
                            0x41, 0x00,  -- i32.const 0
                            0x0B,        -- end (if)
                            0x0B,        -- end (func)
                        },
                    },
                },
                host_functions = { nil },
            })

            -- falsy case
            local results = engine:call_function(0, { m.i32(0) })
            assert.equals(0, results[1].value)
        end)

        it("executes loop with br_if", function()
            -- Sum 1..n using a loop.
            -- param i32 (n), result i32 (sum)
            -- locals: i32 (sum=0), i32 (i=1)
            --
            -- local.get 0 ; n
            -- local.set 2 ; i = n  (actually let's keep it simpler)
            --
            -- Simpler: count down from n to 0, summing.
            -- param: n (local 0)
            -- local 1: sum (i32)
            --
            -- loop
            --   local.get 0   ; n
            --   local.get 1   ; sum
            --   local.get 0   ; n
            --   i32.add       ; sum + n
            --   local.set 1   ; sum = sum + n
            --   local.get 0   ; n
            --   i32.const 1
            --   i32.sub       ; n - 1
            --   local.tee 0   ; n = n - 1 (and leave on stack)
            --   br_if 0       ; if n != 0, loop again
            -- end
            -- local.get 1    ; push sum
            -- end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = { { count = 1, type = 0x7F } },
                        body = {
                            0x03, 0x40,  -- loop (empty)
                            0x20, 0x01,  -- local.get 1 (sum)
                            0x20, 0x00,  -- local.get 0 (n)
                            0x6A,        -- i32.add
                            0x21, 0x01,  -- local.set 1 (sum = sum + n)
                            0x20, 0x00,  -- local.get 0 (n)
                            0x41, 0x01,  -- i32.const 1
                            0x6B,        -- i32.sub
                            0x22, 0x00,  -- local.tee 0 (n = n - 1)
                            0x0D, 0x00,  -- br_if 0 (loop if n != 0)
                            0x0B,        -- end (loop)
                            0x20, 0x01,  -- local.get 1 (sum)
                            0x0B,        -- end (func)
                        },
                    },
                },
                host_functions = { nil },
            })

            -- sum(5) = 1+2+3+4+5 = 15
            local results = engine:call_function(0, { m.i32(5) })
            assert.equals(15, results[1].value)
        end)

        it("executes drop", function()
            -- i32.const 50, i32.const 42, drop, end
            -- Note: signed LEB128 single-byte range is -64..63.
            -- 50 = 0x32, 42 = 0x2A. Both fit in one byte (positive).
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x41, 0x32,  -- i32.const 50
                            0x41, 0x2A,  -- i32.const 42
                            0x1A,        -- drop
                            0x0B,        -- end
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(50, results[1].value)
        end)

        it("executes select", function()
            -- i32.const 10, i32.const 20, i32.const 1, select, end
            -- select picks val1 when condition is nonzero
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x41, 0x0A,  -- i32.const 10
                            0x41, 0x14,  -- i32.const 20
                            0x41, 0x01,  -- i32.const 1 (true)
                            0x1B,        -- select
                            0x0B,        -- end
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(10, results[1].value)
        end)

        it("calls host functions", function()
            local called_with = nil
            local host_func = {
                call = function(args)
                    called_with = args[1].value
                    return { m.i32(args[1].value * 3) }
                end,
            }

            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = {
                    { params = { 0x7F }, results = { 0x7F } },  -- host func type
                    { params = { 0x7F }, results = { 0x7F } },  -- caller type
                },
                func_bodies = {
                    nil,  -- host func has no body
                    {
                        locals = {},
                        body = {
                            0x20, 0x00,  -- local.get 0
                            0x10, 0x00,  -- call 0 (host function)
                            0x0B,        -- end
                        },
                    },
                },
                host_functions = {
                    host_func,  -- index 0
                    nil,        -- index 1
                },
            })

            local results = engine:call_function(1, { m.i32(7) })
            assert.equals(7, called_with)
            assert.equals(21, results[1].value)
        end)

        it("traps on unreachable", function()
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = {} } },
                func_bodies = {
                    {
                        locals = {},
                        body = { 0x00, 0x0B },  -- unreachable, end
                    },
                },
                host_functions = { nil },
            })

            assert.has_error(function() engine:call_function(0, {}) end)
        end)

        it("traps on wrong argument count", function()
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    { locals = {}, body = { 0x20, 0x00, 0x0B } },
                },
                host_functions = { nil },
            })

            assert.has_error(function() engine:call_function(0, {}) end)
        end)

        it("executes memory operations", function()
            local mem = m.LinearMemory.new(1, nil)
            local engine = m.WasmExecutionEngine.new({
                memory = mem,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            -- i32.const 0 (address), i32.const 42, i32.store, i32.const 0, i32.load, end
                            0x41, 0x00,        -- i32.const 0
                            0x41, 0x2A,        -- i32.const 42
                            0x36, 0x02, 0x00,  -- i32.store align=2 offset=0
                            0x41, 0x00,        -- i32.const 0
                            0x28, 0x02, 0x00,  -- i32.load align=2 offset=0
                            0x0B,              -- end
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(42, results[1].value)
        end)

        it("executes i32 bitwise operations", function()
            -- Use params to avoid LEB128 encoding complexity.
            -- param i32, param i32 -> result i32
            -- local.get 0, local.get 1, i32.and, end
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = {},
                global_types = {},
                func_types = { { params = { 0x7F, 0x7F }, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x20, 0x00,  -- local.get 0
                            0x20, 0x01,  -- local.get 1
                            0x71,        -- i32.and
                            0x0B,        -- end
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, { m.i32(0xFF00), m.i32(0x0FF0) })
            assert.equals(0x0F00, results[1].value)
        end)

        it("executes global.get and global.set", function()
            local globals = { m.i32(100) }
            local engine = m.WasmExecutionEngine.new({
                memory = nil,
                tables = {},
                globals = globals,
                global_types = { { type = 0x7F, mutable = true } },
                func_types = { { params = {}, results = { 0x7F } } },
                func_bodies = {
                    {
                        locals = {},
                        body = {
                            0x23, 0x00,  -- global.get 0
                            0x41, 0x05,  -- i32.const 5
                            0x6A,        -- i32.add
                            0x24, 0x00,  -- global.set 0
                            0x23, 0x00,  -- global.get 0
                            0x0B,        -- end
                        },
                    },
                },
                host_functions = { nil },
            })

            local results = engine:call_function(0, {})
            assert.equals(105, results[1].value)
        end)
    end)
end)

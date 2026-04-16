-- Tests for wasm_runtime
--
-- Comprehensive test suite covering:
--   - WasmRuntime creation
--   - Module loading (parsing)
--   - Module validation
--   - Module instantiation
--   - Function calling
--   - load_and_run convenience method
--   - End-to-end: square(5) = 25

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../wasm_execution/src/?.lua;" .. "../../wasm_execution/src/?/init.lua;" .. package.path
package.path = "../../wasm_validator/src/?.lua;" .. "../../wasm_validator/src/?/init.lua;" .. package.path
package.path = "../../wasm_module_parser/src/?.lua;" .. "../../wasm_module_parser/src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../virtual_machine/src/?.lua;" .. "../../virtual_machine/src/?/init.lua;" .. package.path

local m = require("coding_adventures.wasm_runtime")


-- ============================================================================
-- HELPER: Build a WASM binary by hand
-- ============================================================================
--
-- These helpers hand-assemble a WASM binary module. This mirrors what a
-- compiler would produce, but we construct it byte-by-byte so we have
-- full control and don't depend on an external .wasm file.

--- Encode an unsigned integer as LEB128.
local function leb128(n)
    local result = {}
    while true do
        local byte = n & 0x7F
        n = n >> 7
        if n > 0 then
            byte = byte | 0x80
        end
        result[#result + 1] = byte
        if n == 0 then break end
    end
    return result
end

--- Build a WASM section: id + LEB128(size) + payload bytes.
local function build_section(section_id, payload)
    local size = leb128(#payload)
    local result = { section_id }
    for _, b in ipairs(size) do result[#result + 1] = b end
    for _, b in ipairs(payload) do result[#result + 1] = b end
    return result
end

--- Concatenate multiple byte arrays into one.
local function concat_bytes(...)
    local result = {}
    for _, arr in ipairs({...}) do
        for _, b in ipairs(arr) do
            result[#result + 1] = b
        end
    end
    return result
end

--- Convert a byte array to a binary string.
local function bytes_to_string(bytes)
    local chars = {}
    for _, b in ipairs(bytes) do
        chars[#chars + 1] = string.char(b)
    end
    return table.concat(chars)
end


--- Build the square.wasm binary.
--
-- Implements:
--   (module
--     (type (func (param i32) (result i32)))
--     (func (type 0) (param i32) (result i32)
--       local.get 0
--       local.get 0
--       i32.mul)
--     (export "square" (func 0)))
--
local function build_square_wasm()
    local header = { 0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00 }

    -- Type section: 1 type entry (i32) -> (i32)
    local type_payload = concat_bytes(
        leb128(1),                          -- 1 type
        { 0x60, 0x01, 0x7F, 0x01, 0x7F }   -- func: 1 param i32, 1 result i32
    )
    local type_section = build_section(1, type_payload)

    -- Function section: 1 function using type index 0
    local func_payload = concat_bytes(leb128(1), leb128(0))
    local func_section = build_section(3, func_payload)

    -- Export section: "square" -> func 0
    local export_name = { 0x73, 0x71, 0x75, 0x61, 0x72, 0x65 }  -- "square"
    local export_payload = concat_bytes(
        leb128(1),                 -- 1 export
        leb128(#export_name),      -- name length
        export_name,               -- "square"
        { 0x00 },                  -- kind = function
        leb128(0)                  -- index = 0
    )
    local export_section = build_section(7, export_payload)

    -- Code section: 1 function body
    local body_code = {
        0x20, 0x00,  -- local.get 0
        0x20, 0x00,  -- local.get 0
        0x6C,        -- i32.mul
        0x0B,        -- end
    }
    local body = concat_bytes(leb128(0), body_code)  -- 0 local groups + code
    local code_payload = concat_bytes(
        leb128(1),            -- 1 body
        leb128(#body),        -- body size
        body                  -- the body bytes
    )
    local code_section = build_section(10, code_payload)

    local all_bytes = concat_bytes(
        header, type_section, func_section, export_section, code_section
    )
    return bytes_to_string(all_bytes)
end


--- Build an "add" function WASM binary.
-- Exports "add" which takes two i32 params and returns their sum.
local function build_add_wasm()
    local header = { 0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00 }

    -- Type section: (i32, i32) -> (i32)
    local type_payload = concat_bytes(
        leb128(1),
        { 0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F }  -- 2 params i32, 1 result i32
    )
    local type_section = build_section(1, type_payload)

    -- Function section
    local func_payload = concat_bytes(leb128(1), leb128(0))
    local func_section = build_section(3, func_payload)

    -- Export section: "add" -> func 0
    local export_name = { 0x61, 0x64, 0x64 }  -- "add"
    local export_payload = concat_bytes(
        leb128(1), leb128(#export_name), export_name, { 0x00 }, leb128(0)
    )
    local export_section = build_section(7, export_payload)

    -- Code section: local.get 0, local.get 1, i32.add, end
    local body_code = {
        0x20, 0x00,  -- local.get 0
        0x20, 0x01,  -- local.get 1
        0x6A,        -- i32.add
        0x0B,        -- end
    }
    local body = concat_bytes(leb128(0), body_code)
    local code_payload = concat_bytes(leb128(1), leb128(#body), body)
    local code_section = build_section(10, code_payload)

    return bytes_to_string(concat_bytes(
        header, type_section, func_section, export_section, code_section
    ))
end

--- Build a WASM binary with one imported function and one local export.
--
-- Implements:
--   (module
--     (type (func (param i32) (result i32)))
--     (type (func (result i32)))
--     (import "env" "host" (func (type 0)))
--     (func (type 1)
--       i32.const 42)
--     (export "local" (func 1)))
local function build_import_plus_local_wasm()
    local header = { 0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00 }

    local type_payload = concat_bytes(
        leb128(2),
        { 0x60, 0x01, 0x7F, 0x01, 0x7F },
        { 0x60, 0x00, 0x01, 0x7F }
    )
    local type_section = build_section(1, type_payload)

    local import_mod = { 0x65, 0x6E, 0x76 }   -- "env"
    local import_name = { 0x68, 0x6F, 0x73, 0x74 }  -- "host"
    local import_payload = concat_bytes(
        leb128(1),
        leb128(#import_mod), import_mod,
        leb128(#import_name), import_name,
        { 0x00 },
        leb128(0)
    )
    local import_section = build_section(2, import_payload)

    local function_section = build_section(3, concat_bytes(leb128(1), leb128(1)))

    local export_name = { 0x6C, 0x6F, 0x63, 0x61, 0x6C }  -- "local"
    local export_payload = concat_bytes(
        leb128(1),
        leb128(#export_name), export_name,
        { 0x00 },
        leb128(1)
    )
    local export_section = build_section(7, export_payload)

    local body_code = {
        0x41, 0x2A,  -- i32.const 42
        0x0B,        -- end
    }
    local body = concat_bytes(leb128(0), body_code)
    local code_payload = concat_bytes(leb128(1), leb128(#body), body)
    local code_section = build_section(10, code_payload)

    return bytes_to_string(concat_bytes(
        header, type_section, import_section, function_section, export_section, code_section
    ))
end

--- Build a WASM binary that breaks out of a loop and continues afterward.
--
-- Implements:
--   (module
--     (type (func (result i32)))
--     (func (type 0)
--       block
--         loop
--           i32.const 1
--           br_if 1
--           br 0
--         end
--       end
--       i32.const 7)
--     (export "after_break" (func 0)))
local function build_break_then_return_wasm()
    local header = { 0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00 }

    local type_payload = concat_bytes(
        leb128(1),
        { 0x60, 0x00, 0x01, 0x7F }
    )
    local type_section = build_section(1, type_payload)

    local function_section = build_section(3, concat_bytes(leb128(1), leb128(0)))

    local export_name = { 0x61, 0x66, 0x74, 0x65, 0x72, 0x5F, 0x62, 0x72, 0x65, 0x61, 0x6B }  -- "after_break"
    local export_payload = concat_bytes(
        leb128(1),
        leb128(#export_name), export_name,
        { 0x00 },
        leb128(0)
    )
    local export_section = build_section(7, export_payload)

    local body_code = {
        0x02, 0x40,  -- block
        0x03, 0x40,  -- loop
        0x41, 0x01,  -- i32.const 1
        0x0D, 0x01,  -- br_if 1
        0x0C, 0x00,  -- br 0
        0x0B,        -- end loop
        0x0B,        -- end block
        0x41, 0x07,  -- i32.const 7
        0x0B,        -- end function
    }
    local body = concat_bytes(leb128(0), body_code)
    local code_payload = concat_bytes(leb128(1), leb128(#body), body)
    local code_section = build_section(10, code_payload)

    return bytes_to_string(concat_bytes(
        header, type_section, function_section, export_section, code_section
    ))
end


-- ============================================================================
-- TESTS
-- ============================================================================

describe("wasm_runtime", function()

    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    describe("WasmRuntime", function()

        it("creates a runtime", function()
            local runtime = m.WasmRuntime.new()
            assert.is_not_nil(runtime)
        end)

        it("loads a WASM binary", function()
            local runtime = m.WasmRuntime.new()
            local wasm_bytes = build_square_wasm()
            local module = runtime:load(wasm_bytes)
            assert.is_not_nil(module)
            assert.equals(1, #module.types)
            assert.equals(1, #module.functions)
            assert.equals(1, #module.exports)
            assert.equals(1, #module.codes)
        end)

        it("validates a valid module", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_square_wasm())
            local ok, validated = runtime:validate(module)
            assert.is_true(ok)
            assert.is_not_nil(validated)
        end)

        it("instantiates a module", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_square_wasm())
            local instance = runtime:instantiate(module)
            assert.is_not_nil(instance)
            assert.is_not_nil(instance.exports)
            assert.is_not_nil(instance.exports["square"])
        end)

        it("calls an exported function", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_square_wasm())
            local instance = runtime:instantiate(module)
            local results = runtime:call(instance, "square", { 5 })
            assert.equals(1, #results)
            assert.equals(25, results[1])
        end)

        it("keeps local function indices stable after imported functions", function()
            local runtime = m.WasmRuntime.new({
                resolve_function = function(_self, module_name, name)
                    assert.equals("env", module_name)
                    assert.equals("host", name)
                    return {
                        call = function(_args)
                            return { m.i32(7) }
                        end,
                    }
                end,
            })
            local module = runtime:load(build_import_plus_local_wasm())
            local instance = runtime:instantiate(module)
            local results = runtime:call(instance, "local", {})
            assert.equals(1, #results)
            assert.equals(42, results[1])
        end)

        it("continues past branch-target end opcodes inside structured control", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_break_then_return_wasm())
            local instance = runtime:instantiate(module)
            local results = runtime:call(instance, "after_break", {})
            assert.equals(1, #results)
            assert.equals(7, results[1])
        end)

        it("errors on unknown export name", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_square_wasm())
            local instance = runtime:instantiate(module)
            assert.has_error(function()
                runtime:call(instance, "nonexistent", {})
            end)
        end)
    end)


    -- ========================================================================
    -- END-TO-END: square function
    -- ========================================================================

    describe("end-to-end: square function", function()

        it("square(5) = 25", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 5 })
            assert.equals(1, #results)
            assert.equals(25, results[1])
        end)

        it("square(0) = 0", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 0 })
            assert.equals(0, results[1])
        end)

        it("square(-3) = 9", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { -3 })
            assert.equals(9, results[1])
        end)

        it("square(1) = 1", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 1 })
            assert.equals(1, results[1])
        end)

        it("square(100) = 10000", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 100 })
            assert.equals(10000, results[1])
        end)

        it("square(2147483647) wraps in i32 to 1", function()
            -- 2147483647^2 mod 2^32 = 1
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_square_wasm(), "square", { 2147483647 })
            assert.equals(1, results[1])
        end)
    end)


    -- ========================================================================
    -- END-TO-END: add function
    -- ========================================================================

    describe("end-to-end: add function", function()

        it("add(3, 4) = 7", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_add_wasm(), "add", { 3, 4 })
            assert.equals(7, results[1])
        end)

        it("add(0, 0) = 0", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_add_wasm(), "add", { 0, 0 })
            assert.equals(0, results[1])
        end)

        it("add(-5, 5) = 0", function()
            local runtime = m.WasmRuntime.new()
            local results = runtime:load_and_run(build_add_wasm(), "add", { -5, 5 })
            assert.equals(0, results[1])
        end)
    end)


    -- ========================================================================
    -- WASM INSTANCE
    -- ========================================================================

    describe("WasmInstance", function()

        it("stores export map", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_square_wasm())
            local instance = runtime:instantiate(module)
            assert.equals(0, instance.exports["square"].kind)
            assert.equals(0, instance.exports["square"].index)
        end)

        it("has correct func_types", function()
            local runtime = m.WasmRuntime.new()
            local module = runtime:load(build_square_wasm())
            local instance = runtime:instantiate(module)
            assert.equals(1, #instance.func_types)
            assert.equals(1, #instance.func_types[1].params)
            assert.equals(0x7F, instance.func_types[1].params[1])
            assert.equals(1, #instance.func_types[1].results)
        end)
    end)


    -- ========================================================================
    -- RE-EXPORTS
    -- ========================================================================

    describe("re-exports", function()
        it("exports LinearMemory", function()
            assert.is_not_nil(m.LinearMemory)
        end)

        it("exports Table", function()
            assert.is_not_nil(m.Table)
        end)

        it("exports value constructors", function()
            assert.is_not_nil(m.i32)
            assert.is_not_nil(m.i64)
            assert.is_not_nil(m.f32)
            assert.is_not_nil(m.f64)
        end)

        it("exports trap", function()
            assert.is_not_nil(m.trap)
        end)
    end)
end)

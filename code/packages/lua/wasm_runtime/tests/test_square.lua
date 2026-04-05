-- End-to-end test: square function
--
-- Hand-assembles a WASM module that exports a square(x: i32) -> i32
-- function, then runs it through the full pipeline:
--
--   parse -> validate -> instantiate -> call
--
-- The WASM bytecode implements:
--   (module
--     (type (func (param i32) (result i32)))
--     (func (type 0) (param i32) (result i32)
--       local.get 0
--       local.get 0
--       i32.mul)
--     (export "square" (func 0)))

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../wasm_execution/src/?.lua;" .. "../../wasm_execution/src/?/init.lua;" .. package.path
package.path = "../../wasm_validator/src/?.lua;" .. "../../wasm_validator/src/?/init.lua;" .. package.path
package.path = "../../wasm_module_parser/src/?.lua;" .. "../../wasm_module_parser/src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_opcodes/src/?.lua;" .. "../../wasm_opcodes/src/?/init.lua;" .. package.path
package.path = "../../virtual_machine/src/?.lua;" .. "../../virtual_machine/src/?/init.lua;" .. package.path

local wasm_runtime = require("coding_adventures.wasm_runtime")


-- ============================================================================
-- WASM BINARY ASSEMBLY HELPERS
-- ============================================================================

--- Encode an unsigned integer as LEB128 (byte array).
local function leb128(n)
    local result = {}
    while true do
        local byte = n & 0x7F
        n = n >> 7
        if n > 0 then byte = byte | 0x80 end
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
        for _, b in ipairs(arr) do result[#result + 1] = b end
    end
    return result
end

--- Convert a byte array to a binary string.
local function bytes_to_string(bytes)
    local chars = {}
    for _, b in ipairs(bytes) do chars[#chars + 1] = string.char(b) end
    return table.concat(chars)
end


--- Build the square.wasm binary.
local function build_square_wasm()
    local header = { 0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00 }

    -- Type section: 1 type entry (i32) -> (i32)
    local type_payload = concat_bytes(
        leb128(1),
        { 0x60, 0x01, 0x7F, 0x01, 0x7F }
    )
    local type_section = build_section(1, type_payload)

    -- Function section: 1 function using type index 0
    local func_payload = concat_bytes(leb128(1), leb128(0))
    local func_section = build_section(3, func_payload)

    -- Export section: "square" -> func 0
    local export_name = { 0x73, 0x71, 0x75, 0x61, 0x72, 0x65 }  -- "square"
    local export_payload = concat_bytes(
        leb128(1), leb128(#export_name), export_name, { 0x00 }, leb128(0)
    )
    local export_section = build_section(7, export_payload)

    -- Code section: 1 body
    local body_code = {
        0x20, 0x00,  -- local.get 0
        0x20, 0x00,  -- local.get 0
        0x6C,        -- i32.mul
        0x0B,        -- end
    }
    local body = concat_bytes(leb128(0), body_code)
    local code_payload = concat_bytes(leb128(1), leb128(#body), body)
    local code_section = build_section(10, code_payload)

    return bytes_to_string(concat_bytes(
        header, type_section, func_section, export_section, code_section
    ))
end


-- ============================================================================
-- TESTS
-- ============================================================================

describe("end-to-end square", function()

    it("square(5) = 25", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local results = runtime:load_and_run(build_square_wasm(), "square", { 5 })
        assert.equals(1, #results)
        assert.equals(25, results[1])
    end)

    it("square(0) = 0", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local results = runtime:load_and_run(build_square_wasm(), "square", { 0 })
        assert.equals(0, results[1])
    end)

    it("square(-3) = 9", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local results = runtime:load_and_run(build_square_wasm(), "square", { -3 })
        assert.equals(9, results[1])
    end)

    it("square(1) = 1", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local results = runtime:load_and_run(build_square_wasm(), "square", { 1 })
        assert.equals(1, results[1])
    end)

    it("square(100) = 10000", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local results = runtime:load_and_run(build_square_wasm(), "square", { 100 })
        assert.equals(10000, results[1])
    end)

    it("square(2147483647) wraps in i32 to 1", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local results = runtime:load_and_run(build_square_wasm(), "square", { 2147483647 })
        assert.equals(1, results[1])
    end)

    it("goes through parse -> validate -> instantiate -> call", function()
        local runtime = wasm_runtime.WasmRuntime.new()
        local wasm_bytes = build_square_wasm()

        -- Step 1: Parse
        local module = runtime:load(wasm_bytes)
        assert.is_not_nil(module)
        assert.equals(1, #module.types)

        -- Step 2: Validate
        local ok, validated = runtime:validate(module)
        assert.is_true(ok)
        assert.is_not_nil(validated)

        -- Step 3: Instantiate
        local instance = runtime:instantiate(module)
        assert.is_not_nil(instance)
        assert.is_not_nil(instance.exports["square"])

        -- Step 4: Call
        local results = runtime:call(instance, "square", { 5 })
        assert.equals(25, results[1])
    end)
end)

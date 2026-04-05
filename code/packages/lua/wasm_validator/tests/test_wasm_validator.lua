-- Tests for wasm_validator
--
-- Comprehensive test suite covering all validation checks:
--   - Type index validation
--   - Import type index validation
--   - Memory limits validation
--   - Export name uniqueness
--   - Export index validation
--   - Function/code count match
--   - Validated output structure

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path

local m = require("coding_adventures.wasm_validator")


-- Helper: build a minimal valid module
local function minimal_module()
    return {
        types = {
            { params = { 0x7F }, results = { 0x7F } },  -- (i32) -> (i32)
        },
        imports = {},
        functions = { 0 },  -- func 0 uses type 0
        tables = {},
        memories = {},
        globals = {},
        exports = {
            { name = "square", kind = 0, index = 0 },
        },
        codes = {
            { locals = {}, body = { 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B } },
        },
    }
end


describe("wasm_validator", function()

    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    describe("validate", function()

        it("accepts a minimal valid module", function()
            local module = minimal_module()
            local ok, validated = m.validate(module)
            assert.is_true(ok)
            assert.is_not_nil(validated)
            assert.is_not_nil(validated.func_types)
            assert.equals(1, #validated.func_types)
        end)

        it("accepts module with no functions", function()
            local module = {
                types = {},
                imports = {},
                functions = {},
                tables = {},
                memories = {},
                globals = {},
                exports = {},
                codes = {},
            }
            local ok, _ = m.validate(module)
            assert.is_true(ok)
        end)

        it("rejects invalid function type index", function()
            local module = minimal_module()
            module.functions = { 5 }  -- type index 5 does not exist
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "type index"))
        end)

        it("rejects invalid import type index", function()
            local module = minimal_module()
            module.imports = {
                { kind = 0, type_index = 99, module_name = "env", name = "foo" },
            }
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "type index"))
        end)

        it("rejects memory min exceeding max spec", function()
            local module = minimal_module()
            module.memories = {
                { limits = { min = 100000, max = nil } },
            }
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "min pages"))
        end)

        it("rejects memory max exceeding spec", function()
            local module = minimal_module()
            module.memories = {
                { limits = { min = 1, max = 100000 } },
            }
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "max pages"))
        end)

        it("rejects memory min > max", function()
            local module = minimal_module()
            module.memories = {
                { limits = { min = 10, max = 5 } },
            }
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "min pages.*exceeds max"))
        end)

        it("accepts valid memory limits", function()
            local module = minimal_module()
            module.memories = {
                { limits = { min = 1, max = 100 } },
            }
            local ok, _ = m.validate(module)
            assert.is_true(ok)
        end)

        it("rejects duplicate export names", function()
            local module = minimal_module()
            module.exports = {
                { name = "foo", kind = 0, index = 0 },
                { name = "foo", kind = 0, index = 0 },
            }
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "duplicate export"))
        end)

        it("rejects export with out-of-range function index", function()
            local module = minimal_module()
            module.exports = {
                { name = "bad", kind = 0, index = 99 },
            }
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "function index.*out of range"))
        end)

        it("rejects export with out-of-range memory index", function()
            local module = minimal_module()
            module.exports = {
                { name = "mem", kind = 2, index = 0 },
            }
            -- No memories defined
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "memory index.*out of range"))
        end)

        it("rejects function/code count mismatch", function()
            local module = minimal_module()
            module.codes = {}  -- no code entries but 1 function
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "function count.*does not match code count"))
        end)

        it("builds correct func_types with imports", function()
            local module = {
                types = {
                    { params = {}, results = { 0x7F } },         -- type 0
                    { params = { 0x7F }, results = { 0x7F } },   -- type 1
                },
                imports = {
                    { kind = 0, type_index = 0, module_name = "env", name = "log" },
                },
                functions = { 1 },  -- local func uses type 1
                tables = {},
                memories = {},
                globals = {},
                exports = {},
                codes = {
                    { locals = {}, body = { 0x20, 0x00, 0x0B } },
                },
            }
            local ok, validated = m.validate(module)
            assert.is_true(ok)
            -- func_types should have 2 entries: import + local
            assert.equals(2, #validated.func_types)
            -- First is the import's type (0 params)
            assert.equals(0, #validated.func_types[1].params)
            -- Second is the local function's type (1 param)
            assert.equals(1, #validated.func_types[2].params)
        end)

        it("expands local declarations", function()
            local module = minimal_module()
            module.codes = {
                {
                    locals = {
                        { count = 3, type = 0x7F },
                        { count = 2, type = 0x7E },
                    },
                    body = { 0x20, 0x00, 0x0B },
                },
            }
            local ok, validated = m.validate(module)
            assert.is_true(ok)
            -- 3 i32 + 2 i64 = 5 locals
            assert.equals(5, #validated.func_locals[1])
            assert.equals(0x7F, validated.func_locals[1][1])
            assert.equals(0x7F, validated.func_locals[1][3])
            assert.equals(0x7E, validated.func_locals[1][4])
            assert.equals(0x7E, validated.func_locals[1][5])
        end)

        it("rejects export with out-of-range table index", function()
            local module = minimal_module()
            module.exports = {
                { name = "tbl", kind = 1, index = 0 },
            }
            -- No tables defined
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "table index.*out of range"))
        end)

        it("rejects export with out-of-range global index", function()
            local module = minimal_module()
            module.exports = {
                { name = "g", kind = 3, index = 0 },
            }
            -- No globals defined
            local ok, err = m.validate(module)
            assert.is_false(ok)
            assert.truthy(string.find(err, "global index.*out of range"))
        end)
    end)
end)

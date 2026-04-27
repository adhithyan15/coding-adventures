package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
package.path = "../../wasm_leb128/src/?.lua;" .. "../../wasm_leb128/src/?/init.lua;" .. package.path
package.path = "../../wasm_types/src/?.lua;" .. "../../wasm_types/src/?/init.lua;" .. package.path
package.path = "../../wasm_module_parser/src/?.lua;" .. "../../wasm_module_parser/src/?/init.lua;" .. package.path

local encoder = require("coding_adventures.wasm_module_encoder")
local parser = require("coding_adventures.wasm_module_parser")

local function byte_array(text)
    local bytes = {}
    for index = 1, #text do
        bytes[#bytes + 1] = string.byte(text, index)
    end
    return bytes
end

describe("wasm_module_encoder", function()
    it("encodes a minimal function module that round-trips through the parser", function()
        local module = {
            types = {
                { params = { 0x7F }, results = { 0x7F } },
            },
            functions = { 0 },
            exports = {
                { name = "identity", kind = 0, index = 0 },
            },
            codes = {
                {
                    locals = {},
                    body = { 0x20, 0x00, 0x0B },
                },
            },
        }

        local encoded = encoder.encode_module(module)
        local parsed = parser.parse(encoded)

        assert.is_true(encoded:sub(1, 8) == encoder.WASM_MAGIC .. encoder.WASM_VERSION)
        assert.are.same(module.types, parsed.types)
        assert.are.same(module.functions, parsed.functions)
        assert.are.equal("identity", parsed.exports[1].name)
        assert.are.equal("func", parsed.exports[1].desc.kind)
        assert.are.equal(0, parsed.exports[1].desc.idx)
        assert.are.same(module.codes, parsed.codes)
    end)

    it("encodes memory, globals, start, and data segments", function()
        local module = {
            types = {
                { params = {}, results = { 0x7F } },
            },
            functions = { 0 },
            memories = {
                { limits = { min = 1, max = 2 } },
            },
            globals = {
                {
                    global_type = { value_type = 0x7F, mutable = false },
                    init_expr = { 0x41, 0x2A, 0x0B },
                },
            },
            exports = {
                { name = "main", kind = 0, index = 0 },
                { name = "memory", kind = 2, index = 0 },
            },
            start = 0,
            codes = {
                {
                    locals = {
                        { count = 1, type = 0x7F },
                    },
                    body = { 0x41, 0x07, 0x0B },
                },
            },
            data = {
                {
                    memory_index = 0,
                    offset_expr = { 0x41, 0x00, 0x0B },
                    data = byte_array("Nib"),
                },
            },
        }

        local parsed = parser.parse(encoder.encode_module(module))

        assert.are.same(module.memories, parsed.memories)
        assert.are.equal(module.globals[1].global_type.value_type, parsed.globals[1].val_type)
        assert.are.equal(module.globals[1].global_type.mutable, parsed.globals[1].mutable)
        assert.are.same(module.globals[1].init_expr, parsed.globals[1].init_expr)
        assert.are.equal(module.start, parsed.start)
        assert.is_true(#parsed.data == 1)
    end)

    it("encodes imports, tables, and custom sections", function()
        local module = {
            types = {
                { params = {}, results = {} },
            },
            imports = {
                { module_name = "env", name = "f", kind = 0, type_index = 0 },
                {
                    module_name = "env",
                    name = "table",
                    kind = 1,
                    type_info = { ref_type = 0x70, limits = { min = 1, max = 4 } },
                },
                {
                    module_name = "env",
                    name = "memory",
                    kind = 2,
                    type_info = { limits = { min = 1 } },
                },
                {
                    module_name = "env",
                    name = "glob",
                    kind = 3,
                    type_info = { value_type = 0x7F, mutable = true },
                },
            },
            custom = {
                { name = "name", data = { 0x01, 0x02 } },
            },
        }

        local parsed = parser.parse(encoder.encode_module(module))

        assert.are.equal(4, #parsed.imports)
        assert.are.equal("env", parsed.imports[1].mod)
        assert.are.equal("func", parsed.imports[1].desc.kind)
        assert.are.equal("table", parsed.imports[2].desc.kind)
        assert.are.equal("mem", parsed.imports[3].desc.kind)
        assert.are.equal("global", parsed.imports[4].desc.kind)
        assert.are.equal("name", parsed.custom[1].name)
        assert.are.same({ 0x01, 0x02 }, parsed.custom[1].data)
    end)

    it("raises on a function import missing a type index", function()
        local module = {
            types = {},
            imports = {
                { module_name = "env", name = "f", kind = 0 },
            },
        }

        assert.has_error(function()
            encoder.encode_module(module)
        end, "wasm_module_encoder.encode_module: function imports require type_index")
    end)

    it("accepts parser-style import, export, and global shapes", function()
        local module = {
            types = {
                { params = {}, results = {} },
            },
            imports = {
                {
                    mod = "env",
                    name = "f",
                    desc = { kind = "func", type_idx = 0 },
                },
            },
            globals = {
                {
                    val_type = 0x7F,
                    mutable = false,
                    init_expr = { 0x41, 0x01, 0x0B },
                },
            },
            exports = {
                {
                    name = "f",
                    desc = { kind = "func", idx = 0 },
                },
            },
        }

        local parsed = parser.parse(encoder.encode_module(module))

        assert.are.equal("env", parsed.imports[1].mod)
        assert.are.equal("func", parsed.imports[1].desc.kind)
        assert.are.equal("f", parsed.exports[1].name)
        assert.are.equal("func", parsed.exports[1].desc.kind)
        assert.are.equal(0x7F, parsed.globals[1].val_type)
        assert.is_false(parsed.globals[1].mutable)
    end)
end)

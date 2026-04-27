-- ============================================================================
-- wasm_module_encoder — Encode WebAssembly module tables into raw bytes
-- ============================================================================

local leb128 = require("coding_adventures.wasm_leb128")

local M = {}

M.VERSION = "0.1.0"

M.WASM_MAGIC = "\0asm"
M.WASM_VERSION = string.char(0x01, 0x00, 0x00, 0x00)

local EXTERNAL_FUNCTION = 0
local EXTERNAL_TABLE = 1
local EXTERNAL_MEMORY = 2
local EXTERNAL_GLOBAL = 3
local IMPORT_KIND_BY_NAME = {
    func = EXTERNAL_FUNCTION,
    table = EXTERNAL_TABLE,
    mem = EXTERNAL_MEMORY,
    global = EXTERNAL_GLOBAL,
}

local function append_bytes(buffer, bytes)
    for _, byte in ipairs(bytes) do
        buffer[#buffer + 1] = byte
    end
end

local function bytes_from_string(text)
    local bytes = {}
    for index = 1, #text do
        bytes[#bytes + 1] = string.byte(text, index)
    end
    return bytes
end

local function bytes_from_value(value)
    if type(value) == "string" then
        return bytes_from_string(value)
    end
    return value or {}
end

local function u32(value)
    return leb128.encode_unsigned(value)
end

local function section(section_id, payload)
    local bytes = { section_id }
    append_bytes(bytes, u32(#payload))
    append_bytes(bytes, payload)
    return bytes
end

local function encode_name(text)
    local utf8 = bytes_from_string(text)
    local bytes = {}
    append_bytes(bytes, u32(#utf8))
    append_bytes(bytes, utf8)
    return bytes
end

local function encode_vector(values, encoder)
    local bytes = {}
    append_bytes(bytes, u32(#values))
    for _, value in ipairs(values) do
        append_bytes(bytes, encoder(value))
    end
    return bytes
end

local function encode_value_types(types)
    local bytes = {}
    append_bytes(bytes, u32(#types))
    append_bytes(bytes, types)
    return bytes
end

local function encode_func_type(func_type)
    local bytes = { 0x60 }
    append_bytes(bytes, encode_value_types(func_type.params or {}))
    append_bytes(bytes, encode_value_types(func_type.results or {}))
    return bytes
end

local function encode_limits(limits)
    local bytes = {}
    if limits.max == nil then
        bytes[#bytes + 1] = 0x00
        append_bytes(bytes, u32(limits.min))
    else
        bytes[#bytes + 1] = 0x01
        append_bytes(bytes, u32(limits.min))
        append_bytes(bytes, u32(limits.max))
    end
    return bytes
end

local function encode_memory_type(memory_type)
    return encode_limits(memory_type.limits or memory_type)
end

local function encode_table_type(table_type)
    local bytes = { table_type.ref_type or table_type.element_type }
    append_bytes(bytes, encode_limits(table_type.limits))
    return bytes
end

local function encode_global_type(global_type)
    return {
        global_type.value_type or global_type.val_type,
        global_type.mutable and 0x01 or 0x00,
    }
end

local function encode_import(import_value)
    local kind = import_value.kind
    if kind == nil and import_value.desc then
        kind = IMPORT_KIND_BY_NAME[import_value.desc.kind]
    end
    local bytes = {}

    append_bytes(bytes, encode_name(import_value.module_name or import_value.mod))
    append_bytes(bytes, encode_name(import_value.name))
    bytes[#bytes + 1] = kind

    if kind == EXTERNAL_FUNCTION then
        local type_index = import_value.type_index or import_value.typeInfo or import_value.type_info
        if type_index == nil and import_value.desc then
            type_index = import_value.desc.type_idx
        end
        if type_index == nil then
            error("wasm_module_encoder.encode_module: function imports require type_index")
        end
        append_bytes(bytes, u32(type_index))
    elseif kind == EXTERNAL_TABLE then
        local table_type = import_value.type_info or import_value.typeInfo
        if table_type == nil and import_value.desc then
            table_type = {
                ref_type = import_value.desc.ref_type,
                limits = import_value.desc.limits,
            }
        end
        if table_type == nil then
            error("wasm_module_encoder.encode_module: table imports require table metadata")
        end
        append_bytes(bytes, encode_table_type(table_type))
    elseif kind == EXTERNAL_MEMORY then
        local memory_type = import_value.type_info or import_value.typeInfo
        if memory_type == nil and import_value.desc then
            memory_type = { limits = import_value.desc.limits }
        end
        if memory_type == nil then
            error("wasm_module_encoder.encode_module: memory imports require memory metadata")
        end
        append_bytes(bytes, encode_memory_type(memory_type))
    elseif kind == EXTERNAL_GLOBAL then
        local global_type = import_value.type_info or import_value.typeInfo
        if global_type == nil and import_value.desc then
            global_type = {
                value_type = import_value.desc.val_type,
                mutable = import_value.desc.mutable,
            }
        end
        if global_type == nil then
            error("wasm_module_encoder.encode_module: global imports require global metadata")
        end
        append_bytes(bytes, encode_global_type(global_type))
    else
        error(string.format("wasm_module_encoder.encode_module: unsupported import kind %s", tostring(kind)))
    end

    return bytes
end

local function encode_export(export_value)
    local bytes = {}
    append_bytes(bytes, encode_name(export_value.name))
    local kind = export_value.kind
    local index = export_value.index
    if export_value.desc then
        kind = IMPORT_KIND_BY_NAME[export_value.desc.kind]
        index = export_value.desc.idx
    end
    bytes[#bytes + 1] = kind
    append_bytes(bytes, u32(index))
    return bytes
end

local function encode_global(global_value)
    local bytes = {}
    append_bytes(bytes, encode_global_type(global_value.global_type or global_value.type_info or global_value))
    append_bytes(bytes, bytes_from_value(global_value.init_expr or global_value.init))
    return bytes
end

local function encode_element(element)
    local bytes = {}
    append_bytes(bytes, u32(element.table_index))
    append_bytes(bytes, bytes_from_value(element.offset_expr))
    append_bytes(bytes, u32(#(element.function_indices or {})))
    for _, func_index in ipairs(element.function_indices or {}) do
        append_bytes(bytes, u32(func_index))
    end
    return bytes
end

local function encode_data_segment(segment)
    local bytes = {}
    append_bytes(bytes, u32(segment.memory_index))
    append_bytes(bytes, bytes_from_value(segment.offset_expr))
    local data = bytes_from_value(segment.data)
    append_bytes(bytes, u32(#data))
    append_bytes(bytes, data)
    return bytes
end

local function encode_function_body(body)
    local payload = {}
    local locals_ = body.locals or {}
    append_bytes(payload, u32(#locals_))
    for _, local_group in ipairs(locals_) do
        append_bytes(payload, u32(local_group.count))
        payload[#payload + 1] = local_group.type
    end
    append_bytes(payload, bytes_from_value(body.body or body.code))

    local bytes = {}
    append_bytes(bytes, u32(#payload))
    append_bytes(bytes, payload)
    return bytes
end

local function encode_custom(custom)
    local bytes = {}
    append_bytes(bytes, encode_name(custom.name))
    append_bytes(bytes, bytes_from_value(custom.data))
    return bytes
end

local function bytes_to_string(bytes)
    local chars = {}
    for _, byte in ipairs(bytes) do
        chars[#chars + 1] = string.char(byte)
    end
    return table.concat(chars)
end

function M.encode_module(module)
    local sections = {}
    local customs = module.customs or module.custom or {}
    local codes = module.codes or module.code or {}

    for _, custom in ipairs(customs) do
        sections[#sections + 1] = section(0, encode_custom(custom))
    end
    if #(module.types or {}) > 0 then
        sections[#sections + 1] = section(1, encode_vector(module.types, encode_func_type))
    end
    if #(module.imports or {}) > 0 then
        sections[#sections + 1] = section(2, encode_vector(module.imports, encode_import))
    end
    if #(module.functions or {}) > 0 then
        sections[#sections + 1] = section(3, encode_vector(module.functions, u32))
    end
    if #(module.tables or {}) > 0 then
        sections[#sections + 1] = section(4, encode_vector(module.tables, encode_table_type))
    end
    if #(module.memories or {}) > 0 then
        sections[#sections + 1] = section(5, encode_vector(module.memories, encode_memory_type))
    end
    if #(module.globals or {}) > 0 then
        sections[#sections + 1] = section(6, encode_vector(module.globals, encode_global))
    end
    if #(module.exports or {}) > 0 then
        sections[#sections + 1] = section(7, encode_vector(module.exports, encode_export))
    end
    if module.start ~= nil then
        sections[#sections + 1] = section(8, u32(module.start))
    end
    if #(module.elements or {}) > 0 then
        sections[#sections + 1] = section(9, encode_vector(module.elements, encode_element))
    end
    if #codes > 0 then
        sections[#sections + 1] = section(10, encode_vector(codes, encode_function_body))
    end
    if #(module.data or {}) > 0 then
        sections[#sections + 1] = section(11, encode_vector(module.data, encode_data_segment))
    end

    local bytes = bytes_from_string(M.WASM_MAGIC .. M.WASM_VERSION)
    for _, current in ipairs(sections) do
        append_bytes(bytes, current)
    end
    return bytes_to_string(bytes)
end

return M

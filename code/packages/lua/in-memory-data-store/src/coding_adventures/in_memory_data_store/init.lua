local engine_module = require("coding_adventures.in_memory_data_store_engine")
local protocol = require("coding_adventures.in_memory_data_store_protocol")
local resp = require("coding_adventures.resp_protocol")

local M = { VERSION = "0.1.0" }
local Value = resp.Value

local function response_to_resp_value(response)
    if response.kind == "simple_string" then
        return Value.simple_string(response.value)
    elseif response.kind == "error" then
        return Value.error(response.value)
    elseif response.kind == "integer" then
        return Value.integer(response.value)
    elseif response.kind == "bulk_string" then
        return response.value == nil and Value.null_bulk_string() or Value.bulk_string(response.value)
    elseif response.kind == "array" then
        if response.value == nil then
            return Value.null_array()
        end
        local values = {}
        for index, item in ipairs(response.value) do
            values[index] = response_to_resp_value(item)
        end
        return Value.array(values)
    end
    error("unknown engine response kind: " .. tostring(response.kind), 2)
end

local function command_from_resp(frame)
    if not resp.is_value(frame) or frame.kind ~= "array" or frame.is_null then
        return nil
    end

    local parts = {}
    for index, item in ipairs(frame.value) do
        if item.kind == "bulk_string" and not item.is_null then
            parts[index] = item.value
        elseif item.kind == "simple_string" then
            parts[index] = item.value
        elseif item.kind == "integer" then
            parts[index] = tostring(item.value)
        else
            return nil
        end
    end
    return protocol.CommandFrame.from_parts(parts)
end

local function encode_resp_stream(values)
    local encoded = {}
    for index, value in ipairs(values) do
        encoded[index] = resp.encode(value)
    end
    return table.concat(encoded)
end

local function command_to_frame(command)
    local values = { Value.bulk_string(command.command) }
    for _, argument in ipairs(command.args) do
        values[#values + 1] = Value.bulk_string(argument)
    end
    return Value.array(values)
end

local function frame_to_response_text(frame)
    if frame.kind == "simple_string" or frame.kind == "error" then
        return frame.value
    elseif frame.kind == "integer" then
        return tostring(frame.value)
    elseif frame.kind == "bulk_string" then
        return frame.is_null and "(nil)" or frame.value
    end
    return frame.is_null and "(nil)" or string.format("[array:%d]", #frame.value)
end

local DataStore = {}
DataStore.__index = DataStore

function DataStore.new(options)
    options = options or {}
    if type(options) ~= "table" then
        error("options must be a table", 2)
    end
    if options.engine ~= nil and options.store ~= nil then
        error("engine and store are mutually exclusive", 2)
    end

    local data_engine = options.engine
    if data_engine == nil then
        data_engine = engine_module.DataStoreEngine.new({
            store = options.store,
            database_count = options.database_count,
            time_provider = options.time_provider,
        })
    end
    return setmetatable({ engine = data_engine, decoder = resp.Decoder.new() }, DataStore)
end

function DataStore:get_store()
    return self.engine.store
end

function DataStore:reset(store)
    self.engine = engine_module.DataStoreEngine.new({ store = store })
    self.decoder = resp.Decoder.new()
    return self
end

function DataStore:execute_command(command)
    return response_to_resp_value(self.engine:execute_frame(command))
end

function DataStore:execute_parts(parts)
    return response_to_resp_value(self.engine:execute_parts(parts))
end

function DataStore:execute_frame(frame)
    if not resp.is_value(frame) or frame.kind ~= "array" or frame.is_null then
        return Value.error("ERR expected RESP array command")
    end
    if #frame.value == 0 then
        return nil
    end
    local command = command_from_resp(frame)
    if command == nil then
        return Value.error("ERR expected RESP command array")
    end
    return self:execute_command(command)
end

function DataStore:process(input)
    self.decoder:feed(input)
    local responses = {}
    while self.decoder:has_message() do
        local response = self:execute_frame(self.decoder:get_message())
        if response ~= nil then
            responses[#responses + 1] = response
        end
    end
    return responses
end

function DataStore:handle(input)
    return encode_resp_stream(self:process(input))
end

M.DataStore = DataStore
M.InMemoryDataStore = DataStore
M.new = DataStore.new
M.response_to_resp_value = response_to_resp_value
M.command_from_resp = command_from_resp
M.encode_resp_stream = encode_resp_stream
M.command_to_frame = command_to_frame
M.frame_to_response_text = frame_to_response_text
M.ok = function() return Value.simple_string("OK") end

return M

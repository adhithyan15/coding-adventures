local M = {VERSION = "0.1.0"}

local function ascii_upper(data)
    if type(data) ~= "string" then
        error("data must be a string", 2)
    end

    local bytes = {}
    for index = 1, #data do
        local byte = string.byte(data, index)
        if byte > 127 then
            error("data must contain only ASCII bytes", 2)
        end
        if byte >= string.byte("a") and byte <= string.byte("z") then
            byte = byte - 32
        end
        bytes[index] = string.char(byte)
    end
    return table.concat(bytes)
end

local function copy_string_array(values, name)
    if values == nil then
        return {}
    end
    if type(values) ~= "table" then
        error(name .. " must be a table", 3)
    end
    local copy = {}
    for index, value in ipairs(values) do
        if type(value) ~= "string" then
            error(string.format("%s[%d] must be a string", name, index), 3)
        end
        copy[index] = value
    end
    return copy
end

local CommandFrame = {}
CommandFrame.__index = CommandFrame

function CommandFrame.new(command, args)
    if type(command) ~= "string" then
        error("command must be a string", 2)
    end
    return setmetatable({command = command, args = copy_string_array(args, "args")}, CommandFrame)
end

function CommandFrame.from_parts(parts)
    if type(parts) ~= "table" then
        error("parts must be a table", 2)
    end
    if #parts == 0 then
        return nil
    end
    local args = {}
    for index = 2, #parts do
        local value = parts[index]
        if type(value) ~= "string" then
            error(string.format("parts[%d] must be a string", index), 2)
        end
        args[#args + 1] = value
    end
    return CommandFrame.new(ascii_upper(parts[1]), args)
end

function CommandFrame:to_parts()
    local parts = {self.command}
    for _, arg in ipairs(self.args) do
        parts[#parts + 1] = arg
    end
    return parts
end

local EngineResponse = {}
EngineResponse.__index = EngineResponse

local valid_kinds = {
    simple_string = true,
    error = true,
    integer = true,
    bulk_string = true,
    array = true,
}

local function is_response(value)
    return type(value) == "table" and getmetatable(value) == EngineResponse
end

local function copy_response_array(values)
    if values == nil then
        return nil
    end
    if type(values) ~= "table" then
        error("array response value must be a table or nil", 3)
    end
    local copy = {}
    for index, value in ipairs(values) do
        if not is_response(value) then
            error(string.format("array response value[%d] must be an EngineResponse", index), 3)
        end
        copy[index] = value
    end
    return copy
end

function EngineResponse.new(kind, value)
    if not valid_kinds[kind] then
        error("invalid response kind: " .. tostring(kind), 2)
    end
    if (kind == "simple_string" or kind == "error") and type(value) ~= "string" then
        error(kind .. " value must be a string", 2)
    end
    if kind == "integer" and (type(value) ~= "number" or value % 1 ~= 0) then
        error("integer value must be an integer", 2)
    end
    if kind == "bulk_string" and value ~= nil and type(value) ~= "string" then
        error("bulk_string value must be a string or nil", 2)
    end
    if kind == "array" then
        value = copy_response_array(value)
    end
    return setmetatable({kind = kind, value = value}, EngineResponse)
end

function EngineResponse.simple_string(value)
    return EngineResponse.new("simple_string", value)
end

function EngineResponse.error(value)
    return EngineResponse.new("error", value)
end

function EngineResponse.integer(value)
    return EngineResponse.new("integer", value)
end

function EngineResponse.bulk_string(value)
    return EngineResponse.new("bulk_string", value)
end

function EngineResponse.array(value)
    return EngineResponse.new("array", value)
end

function EngineResponse.ok()
    return EngineResponse.simple_string("OK")
end

function EngineResponse.null()
    return EngineResponse.bulk_string(nil)
end

function EngineResponse.zero()
    return EngineResponse.integer(0)
end

function EngineResponse.one()
    return EngineResponse.integer(1)
end

M.ascii_upper = ascii_upper
M.CommandFrame = CommandFrame
M.EngineResponse = EngineResponse

return M

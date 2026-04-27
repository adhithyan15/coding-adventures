-- spec_loader.lua -- JSON Spec Loading and Validation
-- =================================================

local Errors = require("coding_adventures.cli_builder.errors")

local SpecLoader = {}

--- load reads a JSON spec from a file.
--
-- NOTE: This implementation uses a simple regex-based JSON parser
-- because the standard Lua environment in this repo does not have a 
-- dedicated JSON library and cowsay's spec is relatively simple.
--
-- @param path string Path to the JSON file.
-- @return table The parsed spec, or nil + error.
function SpecLoader.load(path)
    local f = io.open(path, "r")
    if not f then
        return nil, Errors.SpecError("could not open spec file: " .. path)
    end
    local content = f:read("*a")
    f:close()

    return SpecLoader.parse_string(content)
end

--- parse_string parses a JSON string into a Lua table.
--
-- @param json_str string The JSON content.
-- @return table The parsed spec, or nil + error.
function SpecLoader.parse_string(json_str)
    local pos = 1
    local function skip_ws()
        pos = json_str:find("[^%s]", pos) or #json_str + 1
    end

    local parse_val -- forward decl

    local function parse_string_val()
        local start = pos + 1
        local stop = json_str:find('"', start)
        while stop and json_str:sub(stop - 1, stop - 1) == "\\" do
            stop = json_str:find('"', stop + 1)
        end
        if not stop then error("Unterminated string at " .. pos) end
        pos = stop + 1
        return json_str:sub(start, stop - 1):gsub("\\(.)", "%1")
    end

    local function parse_array()
        local arr = {}
        pos = pos + 1 -- skip [
        skip_ws()
        if json_str:sub(pos, pos) == "]" then
            pos = pos + 1
            return arr
        end
        while true do
            arr[#arr + 1] = parse_val()
            skip_ws()
            local char = json_str:sub(pos, pos)
            if char == "]" then
                pos = pos + 1
                return arr
            end
            if char == "," then
                pos = pos + 1
                skip_ws()
            end
        end
    end

    local function parse_object()
        local obj = {}
        pos = pos + 1 -- skip {
        skip_ws()
        if json_str:sub(pos, pos) == "}" then
            pos = pos + 1
            return obj
        end
        while true do
            local key = parse_string_val()
            skip_ws()
            if json_str:sub(pos, pos) ~= ":" then
                error("Expected : at " .. pos)
            end
            pos = pos + 1 -- skip :
            obj[key] = parse_val()
            skip_ws()
            local char = json_str:sub(pos, pos)
            if char == "}" then
                pos = pos + 1
                return obj
            end
            if char == "," then
                pos = pos + 1
                skip_ws()
            end
        end
    end

    parse_val = function()
        skip_ws()
        local char = json_str:sub(pos, pos)
        if char == "{" then return parse_object() end
        if char == "[" then return parse_array() end
        if char == '"' then return parse_string_val() end
        if char:match("[%d%-]") then
            local start = pos
            pos = json_str:find("[^%d%.%e%E%+%-]", pos) or #json_str + 1
            return tonumber(json_str:sub(start, pos - 1))
        end
        if json_str:sub(pos, pos + 3) == "true" then pos = pos + 4; return true end
        if json_str:sub(pos, pos + 4) == "false" then pos = pos + 5; return false end
        if json_str:sub(pos, pos + 3) == "null" then pos = pos + 4; return nil end
        error("Unexpected character at " .. pos .. ": " .. char)
    end

    local ok, result = pcall(parse_val)
    if not ok then
        return nil, Errors.SpecError(result)
    end
    return result
end

-- I'll put the real_parse logic into parse_string.
return SpecLoader

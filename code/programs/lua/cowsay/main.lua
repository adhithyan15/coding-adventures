#!/usr/bin/env lua
-- main.lua -- Cowsay implementation in Lua
-- =========================================

local clibuilder = require("coding_adventures.cli_builder")
local Parser = clibuilder.Parser

-- Find repo root
local function find_repo_root()
    local path = debug.getinfo(1).source:sub(2)
    local dir = path:match("(.*/)") or "./"
    -- Walk up to find 'code' directory
    for _ = 1, 10 do
        local f = io.open(dir .. "code", "r")
        if f then
            f:close()
            -- Normalizing to absolute path if possible
            local handle = io.popen("cd " .. dir .. " && pwd")
            if handle then
                local res = handle:read("*a"):gsub("\n", "")
                handle:close()
                return res
            end
            return dir
        end
        dir = dir .. "../"
    end
    return "."
end

local ROOT = find_repo_root()

--- Simple word wrap function.
local function wrap_text(text, width)
    local lines = {}
    for line in text:gmatch("[^\r\n]+") do
        local current_line = ""
        for word in line:gmatch("%S+") do
            if #current_line + #word + 1 > width then
                table.insert(lines, current_line)
                current_line = word
            else
                if #current_line > 0 then
                    current_line = current_line .. " " .. word
                else
                    current_line = word
                end
            end
        end
        table.insert(lines, current_line)
    end
    return lines
end

local function format_bubble(lines, is_think)
    if #lines == 0 then return "" end
    
    local max_len = 0
    for _, line in ipairs(lines) do
        if #line > max_len then max_len = #line end
    end
    
    local top = " " .. string.rep("_", max_len + 2)
    local bottom = " " .. string.rep("-", max_len + 2)
    
    local result = { top }
    if #lines == 1 then
        local start, stop = "<", ">"
        if is_think then start, stop = "(", ")" end
        table.insert(result, string.format("%s %s %s", start, lines[1] .. string.rep(" ", max_len - #lines[1]), stop))
    else
        for i, line in ipairs(lines) do
            local start, stop = "|", "|"
            if not is_think then
                if i == 1 then start, stop = "/", "\\"
                elseif i == #lines then start, stop = "\\", "/"
                end
            else
                start, stop = "(", ")"
            end
            table.insert(result, string.format("%s %s %s", start, line .. string.rep(" ", max_len - #line), stop))
        end
    end
    table.insert(result, bottom)
    return table.concat(result, "\n")
end

local function load_cow(name)
    local path = ROOT .. "/code/specs/cows/" .. name .. ".cow"
    local f = io.open(path, "r")
    if not f then
        f = io.open(ROOT .. "/code/specs/cows/default.cow", "r")
    end
    if not f then return "$thoughts   ^__^\n $thoughts  ($eyes)\\_______\n" end
    local content = f:read("*a")
    f:close()
    
    -- Strip through the first EOC marker and its trailing semicolon/newline
    local body = content:match("<<EOC;%s*\r?\n?(.-)EOC")
    return body or content
end

local function main()
    local spec_path = ROOT .. "/code/specs/cowsay.json"
    local parser, err = Parser.new(spec_path, arg)
    if not parser then
        local msg = (type(err) == "table" and err.message) or tostring(err)
        io.stderr:write("Error: " .. msg .. "\n")
        os.exit(1)
    end
    
    local result, perrs = parser:parse()
    if not result then
        io.stderr:write("Parse error:\n")
        for _, e in ipairs(perrs) do
            io.stderr:write("  - " .. e.message .. "\n")
        end
        os.exit(1)
    end
    
    if result.type == "help_result" then
        print(result.text)
        return
    elseif result.type == "version_result" then
        print(result.version)
        return
    end
    
    local flags = result.flags
    local args = result.arguments
    
    local message = args.message or ""
    if message == "" then
        -- Read from stdin if not tty
        local handle = io.popen("test -t 0")
        local is_tty = handle:close()
        if not is_tty then
            message = io.read("*a"):gsub("%s+$", "")
        end
    end
    
    if message == "" then return end
    
    local eyes = flags.eyes or "oo"
    local tongue = flags.tongue or "  "
    
    if flags.borg then eyes = "==" end
    if flags.dead then eyes = "XX"; tongue = "U " end
    if flags.greedy then eyes = "$$" end
    if flags.paranoid then eyes = "@@" end
    if flags.stoned then eyes = "xx"; tongue = "U " end
    if flags.tired then eyes = "--" end
    if flags.wired then eyes = "OO" end
    if flags.youthful then eyes = ".." end
    
    eyes = (eyes .. "  "):sub(1, 2)
    tongue = (tongue .. "  "):sub(1, 2)
    
    local width = flags.width or 40
    local lines
    if flags.nowrap then
        lines = {}
        for line in message:gmatch("[^\r\n]+") do table.insert(lines, line) end
    else
        lines = wrap_text(message, width)
    end
    
    local is_think = flags.think or false
    -- cowthink if invoked as such
    local progname = arg[0]:match("([^/\\]+)$") or ""
    if progname == "cowthink" then is_think = true end
    
    local thoughts = is_think and "o" or "\\"
    
    local bubble = format_bubble(lines, is_think)
    local cow_template = load_cow(flags.cowfile or "default")
    
    local function replace_all(str, t)
        for k, v in pairs(t) do
            local val = tostring(v):gsub("%%", "%%%%")
            local patt = k:gsub("%$", "%%$")
            str = str:gsub(patt, val)
        end
        return str
    end
    
    local cow = replace_all(cow_template, {
        ["$eyes"] = eyes,
        ["$tongue"] = tongue,
        ["$thoughts"] = thoughts
    })
    
    -- Final unescape for backslashes
    cow = cow:gsub("\\\\", "\\")
    
    print(bubble)
    print(cow)
end

main()

local grammar_tools = require("coding_adventures.grammar_tools")
local nib_lexer = require("coding_adventures.nib_lexer")
local parser_pkg = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    src = src:gsub("\\", "/")
    return src:match("(.+)/[^/]+$") or "."
end

local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = result .. "/.."
    end
    return result
end

local grammar_cache = nil

local function get_grammar()
    if grammar_cache then
        return grammar_cache
    end

    local grammar_path = up(get_script_dir(), 6) .. "/grammars/nib.grammar"
    local file, open_err = io.open(grammar_path, "r")
    if not file then
        error("nib_parser: cannot open grammar file: " .. grammar_path .. " (" .. (open_err or "unknown error") .. ")")
    end

    local content = file:read("*all")
    file:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error("nib_parser: failed to parse nib.grammar: " .. (parse_err or "unknown error"))
    end

    grammar_cache = grammar
    return grammar_cache
end

function M.parse(source)
    local grammar = get_grammar()
    local tokens = nib_lexer.tokenize(source)
    local parser = parser_pkg.GrammarParser.new(tokens, grammar)
    return parser:parse()
end

function M.get_grammar()
    return get_grammar()
end

return M

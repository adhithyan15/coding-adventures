local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg = require("coding_adventures.lexer")

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

    local tokens_path = up(get_script_dir(), 6) .. "/grammars/nib.tokens"
    local file, open_err = io.open(tokens_path, "r")
    if not file then
        error("nib_lexer: cannot open grammar file: " .. tokens_path .. " (" .. (open_err or "unknown error") .. ")")
    end

    local content = file:read("*all")
    file:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("nib_lexer: failed to parse nib.tokens: " .. (parse_err or "unknown error"))
    end

    grammar_cache = grammar
    return grammar_cache
end

function M.tokenize(source)
    local grammar = get_grammar()
    local raw = lexer_pkg.GrammarLexer.new(source, grammar):tokenize()
    local tokens = {}

    for _, tok in ipairs(raw) do
        tokens[#tokens + 1] = {
            type = tok.type_name,
            value = tok.value,
            line = tok.line,
            col = tok.column,
        }
    end

    return tokens
end

function M.get_grammar()
    return get_grammar()
end

return M

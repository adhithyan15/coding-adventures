local lexer_pkg = require("coding_adventures.lexer")

local compiled_grammars = {
    ["1987"] = require("coding_adventures.vhdl_lexer._grammar_1987"),
    ["1993"] = require("coding_adventures.vhdl_lexer._grammar_1993"),
    ["2002"] = require("coding_adventures.vhdl_lexer._grammar_2002"),
    ["2008"] = require("coding_adventures.vhdl_lexer._grammar_2008"),
    ["2019"] = require("coding_adventures.vhdl_lexer._grammar_2019"),
}

local M = {}
M.VERSION = "0.1.0"
M.DEFAULT_VERSION = "2008"
M.SUPPORTED_VERSIONS = { "1987", "1993", "2002", "2008", "2019" }

local grammar_cache = {}
local keyword_cache = {}

local function resolve_version(version)
    local resolved = (version == nil or version == "") and M.DEFAULT_VERSION or version
    if compiled_grammars[resolved] then
        return resolved
    end
    error(
        "vhdl_lexer: unknown VHDL version '" .. tostring(resolved) .. "'. " ..
        "Valid values are: 1987, 1993, 2002, 2008, 2019."
    )
end

function M.get_grammar(version)
    local resolved = resolve_version(version)
    if grammar_cache[resolved] == nil then
        grammar_cache[resolved] = compiled_grammars[resolved].token_grammar()
    end
    return grammar_cache[resolved]
end

local function keyword_set(version)
    local resolved = resolve_version(version)
    if keyword_cache[resolved] == nil then
        local set = {}
        for _, keyword in ipairs(M.get_grammar(resolved).keywords or {}) do
            set[keyword] = true
        end
        keyword_cache[resolved] = set
    end
    return keyword_cache[resolved]
end

function M.tokenize(source, version)
    local resolved = resolve_version(version)
    local grammar = M.get_grammar(resolved)
    local keywords = keyword_set(resolved)
    local gl = lexer_pkg.GrammarLexer.new(source, grammar)
    local raw = gl:tokenize()
    local tokens = {}

    for _, tok in ipairs(raw) do
        local type_name = tok.type_name
        local value = tok.value
        local lowered = type(value) == "string" and string.lower(value) or value

        if keywords[lowered] then
            value = lowered
        elseif type_name == "NAME" or type_name == "KEYWORD" then
            value = lowered
            if keywords[lowered] then
                type_name = "KEYWORD"
            end
        elseif type_name == "BIT_STRING" then
            value = lowered
        end

        tokens[#tokens + 1] = {
            type = type_name,
            value = value,
            line = tok.line,
            col = tok.column,
        }
    end

    return tokens
end

function M.resolve_version(version)
    return resolve_version(version)
end

return M

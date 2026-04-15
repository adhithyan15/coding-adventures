local lexer_pkg = require("coding_adventures.lexer")

local compiled_grammars = {
    ["1995"] = require("coding_adventures.verilog_lexer._grammar_1995"),
    ["2001"] = require("coding_adventures.verilog_lexer._grammar_2001"),
    ["2005"] = require("coding_adventures.verilog_lexer._grammar_2005"),
}

local M = {}
M.VERSION = "0.1.0"
M.DEFAULT_VERSION = "2005"
M.SUPPORTED_VERSIONS = { "1995", "2001", "2005" }

local grammar_cache = {}

local function resolve_version(version)
    local resolved = (version == nil or version == "") and M.DEFAULT_VERSION or version
    if compiled_grammars[resolved] then
        return resolved
    end
    error(
        "verilog_lexer: unknown Verilog version '" .. tostring(resolved) .. "'. " ..
        "Valid values are: 1995, 2001, 2005."
    )
end

local function get_compiled_module(version)
    return compiled_grammars[resolve_version(version)]
end

function M.get_grammar(version)
    local resolved = resolve_version(version)
    if grammar_cache[resolved] == nil then
        grammar_cache[resolved] = get_compiled_module(resolved).token_grammar()
    end
    return grammar_cache[resolved]
end

function M.tokenize(source, version)
    local grammar = M.get_grammar(version)
    local gl = lexer_pkg.GrammarLexer.new(source, grammar)
    local raw = gl:tokenize()
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

function M.resolve_version(version)
    return resolve_version(version)
end

return M

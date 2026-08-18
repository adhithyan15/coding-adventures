local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

local grammar_cache = nil

local function get_grammar()
    if not grammar_cache then
        grammar_cache = require("coding_adventures.nib_lexer._grammar").token_grammar()
    end
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

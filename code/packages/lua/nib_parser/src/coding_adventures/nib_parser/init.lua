local nib_lexer = require("coding_adventures.nib_lexer")
local parser_pkg = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

local grammar_cache = nil

local function get_grammar()
    if not grammar_cache then
        grammar_cache = require("coding_adventures.nib_parser._grammar").parser_grammar()
    end
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

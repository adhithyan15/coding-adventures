local vhdl_lexer = require("coding_adventures.vhdl_lexer")
local parser_pkg = require("coding_adventures.parser")

local compiled_grammars = {
    ["1987"] = require("coding_adventures.vhdl_parser._grammar_1987"),
    ["1993"] = require("coding_adventures.vhdl_parser._grammar_1993"),
    ["2002"] = require("coding_adventures.vhdl_parser._grammar_2002"),
    ["2008"] = require("coding_adventures.vhdl_parser._grammar_2008"),
    ["2019"] = require("coding_adventures.vhdl_parser._grammar_2019"),
}

local M = {}
M.VERSION = "0.1.0"
M.DEFAULT_VERSION = vhdl_lexer.DEFAULT_VERSION
M.SUPPORTED_VERSIONS = vhdl_lexer.SUPPORTED_VERSIONS

local grammar_cache = {}

local function resolve_version(version)
    return vhdl_lexer.resolve_version(version)
end

function M.get_grammar(version)
    local resolved = resolve_version(version)
    if grammar_cache[resolved] == nil then
        grammar_cache[resolved] = compiled_grammars[resolved].parser_grammar()
    end
    return grammar_cache[resolved]
end

function M.create_parser(source, version)
    local tokens = vhdl_lexer.tokenize(source, version)
    local grammar = M.get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

function M.parse(source, version)
    local parser = M.create_parser(source, version)
    local ast, err = parser:parse()
    if not ast then
        error("vhdl_parser: " .. (err or "parse failed"))
    end
    return ast
end

function M.resolve_version(version)
    return resolve_version(version)
end

return M

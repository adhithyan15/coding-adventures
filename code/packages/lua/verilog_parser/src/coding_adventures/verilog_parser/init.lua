local verilog_lexer = require("coding_adventures.verilog_lexer")
local parser_pkg = require("coding_adventures.parser")

local compiled_grammars = {
    ["1995"] = require("coding_adventures.verilog_parser._grammar_1995"),
    ["2001"] = require("coding_adventures.verilog_parser._grammar_2001"),
    ["2005"] = require("coding_adventures.verilog_parser._grammar_2005"),
}

local M = {}
M.VERSION = "0.1.0"
M.DEFAULT_VERSION = verilog_lexer.DEFAULT_VERSION
M.SUPPORTED_VERSIONS = verilog_lexer.SUPPORTED_VERSIONS

local grammar_cache = {}

local function resolve_version(version)
    return verilog_lexer.resolve_version(version)
end

function M.get_grammar(version)
    local resolved = resolve_version(version)
    if grammar_cache[resolved] == nil then
        grammar_cache[resolved] = compiled_grammars[resolved].parser_grammar()
    end
    return grammar_cache[resolved]
end

function M.create_parser(source, version)
    local tokens = verilog_lexer.tokenize(source, version)
    local grammar = M.get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

function M.parse(source, version)
    local parser = M.create_parser(source, version)
    local ast, err = parser:parse()
    if not ast then
        error("verilog_parser: " .. (err or "parse failed"))
    end
    return ast
end

function M.resolve_version(version)
    return resolve_version(version)
end

return M

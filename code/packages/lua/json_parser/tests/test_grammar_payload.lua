package.path = (
    "../src/?.lua;../src/?/init.lua;" ..
    "../../json_lexer/src/?.lua;../../json_lexer/src/?/init.lua;" ..
    package.path
)

local parser_grammar = require("coding_adventures.json_parser.grammar_data")
local lexer_grammar = require("coding_adventures.json_lexer.grammar_data")

local function read(path)
    local file = assert(io.open(path, "rb"))
    local content = file:read("*all")
    file:close()
    return content
end

describe("bundled JSON grammars", function()
    it("keeps the parser payload aligned with the canonical fixture", function()
        assert.equal(
            read("../../../../grammars/json/json.grammar"),
            parser_grammar
        )
    end)

    it("keeps the lexer payload aligned with the canonical fixture", function()
        assert.equal(
            read("../../../../grammars/json/json.tokens"),
            lexer_grammar
        )
    end)
end)

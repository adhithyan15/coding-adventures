package.path = (
    "../src/?.lua;../src/?/init.lua;" ..
    "../../lattice_lexer/src/?.lua;../../lattice_lexer/src/?/init.lua;" ..
    package.path
)

local parser_grammar = require("coding_adventures.lattice_parser.grammar_data")
local lexer_grammar = require("coding_adventures.lattice_lexer.grammar_data")

local function read(path)
    local file = assert(io.open(path, "rb"))
    local content = file:read("*all")
    file:close()
    return content
end

describe("bundled Lattice grammars", function()
    it("keeps the parser payload aligned with the canonical fixture", function()
        assert.equal(
            read("../../../../grammars/lattice/lattice.grammar"),
            parser_grammar
        )
    end)

    it("keeps the lexer payload aligned with the canonical fixture", function()
        assert.equal(
            read("../../../../grammars/lattice/lattice.tokens"),
            lexer_grammar
        )
    end)
end)

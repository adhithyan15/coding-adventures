package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../grammar_tools/src/?.lua;" ..
    "../../grammar_tools/src/?/init.lua;" ..
    "../../lexer/src/?.lua;" ..
    "../../lexer/src/?/init.lua;" ..
    "../../state_machine/src/?.lua;" ..
    "../../state_machine/src/?/init.lua;" ..
    "../../directed_graph/src/?.lua;" ..
    "../../directed_graph/src/?/init.lua;" ..
    package.path
)

local haskell_lexer = require("coding_adventures.haskell_lexer")

describe("haskell_lexer", function()
    it("tokenizes with the default 2010 grammar", function()
        local tokens = haskell_lexer.tokenize("x")
        assert.are.equal("NAME", tokens[1].type)
    end)

    it("emits virtual layout tokens", function()
        local tokens = haskell_lexer.tokenize("let\n  x = y\nin x")
        local types = {}
        for _, token in ipairs(tokens) do
            types[token.type] = true
        end
        assert.is_true(types["VIRTUAL_LBRACE"])
        assert.is_true(types["VIRTUAL_RBRACE"])
    end)

    it("supports historical versions", function()
        local tokens = haskell_lexer.tokenize("x", "98")
        assert.are.equal("NAME", tokens[1].type)
    end)
end)

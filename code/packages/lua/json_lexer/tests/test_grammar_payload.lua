package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local grammar_data = require("coding_adventures.json_lexer.grammar_data")

describe("bundled JSON token grammar", function()
    it("matches the canonical language-neutral grammar byte for byte", function()
        local file = assert(io.open("../../../../grammars/json/json.tokens", "rb"))
        local canonical = file:read("*all")
        file:close()
        assert.equal(canonical, grammar_data)
    end)
end)

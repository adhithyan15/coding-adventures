package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local nib_lexer = require("coding_adventures.nib_lexer")

describe("nib_lexer", function()
    it("tokenizes a function declaration", function()
        local tokens = nib_lexer.tokenize("fn main() { return 0; }")
        assert.are.equal("FN", tokens[1].type)
        assert.are.equal("fn", tokens[1].value)
        assert.are.equal("NAME", tokens[2].type)
        assert.are.equal("main", tokens[2].value)
    end)

    it("prefers multicharacter operators", function()
        local tokens = nib_lexer.tokenize("1 +% 2 +? 3")
        assert.are.equal("+%", tokens[2].value)
        assert.are.equal("+?", tokens[4].value)
    end)
end)

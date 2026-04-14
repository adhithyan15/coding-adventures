package.path = "../../nib_lexer/src/?.lua;" ..
    "../../nib_lexer/src/?/init.lua;" ..
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    package.path

local nib_parser = require("coding_adventures.nib_parser")

describe("nib_parser", function()
    it("parses a simple program", function()
        local ast, err = nib_parser.parse("fn main() { return 0; }")
        assert.is_nil(err)
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)
end)

local haskell_parser = require("coding_adventures.haskell_parser")

describe("haskell_parser", function()
    it("uses file as the root rule", function()
        local ast = haskell_parser.parse("x")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses explicit-brace let expressions", function()
        local ast = haskell_parser.parse("let { x = y } in x", "2010")
        assert.are.equal("file", ast.rule_name)
    end)
end)

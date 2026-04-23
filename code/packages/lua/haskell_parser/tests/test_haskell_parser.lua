package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../grammar_tools/src/?.lua;" ..
    "../../grammar_tools/src/?/init.lua;" ..
    "../../lexer/src/?.lua;" ..
    "../../lexer/src/?/init.lua;" ..
    "../../parser/src/?.lua;" ..
    "../../parser/src/?/init.lua;" ..
    "../../state_machine/src/?.lua;" ..
    "../../state_machine/src/?/init.lua;" ..
    "../../directed_graph/src/?.lua;" ..
    "../../directed_graph/src/?/init.lua;" ..
    "../../haskell_lexer/src/?.lua;" ..
    "../../haskell_lexer/src/?/init.lua;" ..
    package.path
)

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

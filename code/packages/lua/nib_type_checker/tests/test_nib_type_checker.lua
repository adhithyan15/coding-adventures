package.path = "../../nib_lexer/src/?.lua;" ..
    "../../nib_lexer/src/?/init.lua;" ..
    "../../nib_parser/src/?.lua;" ..
    "../../nib_parser/src/?/init.lua;" ..
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    package.path

local nib_parser = require("coding_adventures.nib_parser")
local checker = require("coding_adventures.nib_type_checker")

describe("nib_type_checker", function()
    it("accepts function calls and returns", function()
        local ast = nib_parser.parse("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(3, 4); }")
        local result = checker.check(ast)
        assert.is_true(result.ok)
    end)

    it("accepts the loop subset", function()
        local ast = nib_parser.parse([[
            fn count_to(n: u4) -> u4 {
                let acc: u4 = 0;
                for i: u4 in 0..n {
                    acc = acc +% 1;
                }
                return acc;
            }
        ]])
        local result = checker.check(ast)
        assert.is_true(result.ok)
    end)

    it("reports assignment mismatches", function()
        local ast = nib_parser.parse("fn main() { let flag: bool = true; flag = 1; }")
        local result = checker.check(ast)
        assert.is_false(result.ok)
    end)

    it("reports arity mismatches", function()
        local ast = nib_parser.parse("fn add(a: u4, b: u4) -> u4 { return a +% b; } fn main() -> u4 { return add(1); }")
        local result = checker.check(ast)
        assert.is_false(result.ok)
    end)
end)

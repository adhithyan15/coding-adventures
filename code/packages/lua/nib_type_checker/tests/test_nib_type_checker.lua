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

    it("reports type mismatches in a plain two-operand add_expr", function()
        -- Regression test: add_expr = shift_expr { (PLUS|MINUS|...) shift_expr }
        -- (#11257). Nib's lexer does not tokenize SHL/SHR,
        -- so every operand of a plain `a + b` is parsed as a shift_expr node
        -- that transparently wraps a single mul_expr child. Before shift_expr
        -- was added to this checker's expression_rules allowlist,
        -- expression_children(add_expr) filtered out both shift_expr operands,
        -- so check_expr's add_expr branch never saw >= 2 operands and silently
        -- fell through to the single-child passthrough, inferring only the
        -- left operand's type and never comparing it against the right
        -- operand -- so a u4 + u8 mismatch went unreported.
        local ast = nib_parser.parse("fn main() -> u4 { let a: u4 = 1; let b: u8 = 2; let c: u4 = a + b; return c; }")
        local result = checker.check(ast)
        assert.is_false(result.ok)
    end)
end)

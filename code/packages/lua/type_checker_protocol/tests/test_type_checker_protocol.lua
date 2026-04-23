package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local protocol = require("coding_adventures.type_checker_protocol")

describe("type_checker_protocol", function()
    it("normalizes hook kinds during dispatch", function()
        local checker = protocol.new_generic_type_checker(function(node)
            return node.kind
        end)

        checker:register_hook("enter", "fn decl", function()
            return "exact"
        end)

        assert.equals("exact", checker:dispatch("enter", { kind = "fn decl" }))
    end)

    it("falls through when a hook returns not_handled", function()
        local checker = protocol.new_generic_type_checker(function(node)
            return node.kind
        end)

        checker:register_hook("enter", "expr:add", function()
            return checker:not_handled()
        end)
        checker:register_hook("enter", "*", function()
            return "fallback"
        end)

        assert.equals("fallback", checker:dispatch("enter", { kind = "expr:add" }))
    end)

    it("collects diagnostics into a type-check result", function()
        local checker = protocol.new_generic_type_checker(
            function(node)
                return node.kind
            end,
            function(node)
                return node.line, node.column
            end
        )

        function checker:run(ast)
            self:error("bad node", ast)
        end

        local result = checker:check({ kind = "expr", line = 4, column = 2 })

        assert.is_false(result.ok)
        assert.equals(1, #result.errors)
        assert.equals("bad node", result.errors[1].message)
        assert.equals(4, result.errors[1].line)
        assert.equals(2, result.errors[1].column)
    end)
end)

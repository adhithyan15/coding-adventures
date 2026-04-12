-- Tests for csharp_parser
-- =======================
--
-- These assertions follow the grammar-driven parser's current behavior:
-- the root rule is `compilation_unit`, and class declarations are valid
-- across all supported versions. The shared Lua parser runtime now guards
-- against left recursion, but this package still doesn't reliably accept
-- C# top-level statements, so we avoid asserting that capability here.

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
    "../../csharp_lexer/src/?.lua;" ..
    "../../csharp_lexer/src/?/init.lua;" ..
    "../../parser/src/?.lua;" ..
    "../../parser/src/?/init.lua;" ..
    package.path
)

local csharp_parser = require("coding_adventures.csharp_parser")

local function assert_parses_compilation_unit(source, version)
    local ast = csharp_parser.parse_csharp(source, version)
    assert.are.equal("compilation_unit", ast.rule_name)
    return ast
end

describe("csharp_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(csharp_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(csharp_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", csharp_parser.VERSION)
    end)

    it("exposes parse_csharp as a function", function()
        assert.is_function(csharp_parser.parse_csharp)
    end)

    it("exposes create_csharp_parser as a function", function()
        assert.is_function(csharp_parser.create_csharp_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(csharp_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = csharp_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        assert.is_true(#g.rules > 0)
    end)

    it("grammar first rule is compilation_unit", function()
        local g = csharp_parser.get_grammar()
        assert.are.equal("compilation_unit", g.rules[1].name)
    end)
end)

describe("class declarations", function()
    it("parses a simple class", function()
        assert_parses_compilation_unit("class Hello {}")
    end)

    it("parses a public class", function()
        assert_parses_compilation_unit("public class Main {}")
    end)

    it("parses a namespaced class", function()
        assert_parses_compilation_unit("namespace MyApp { public class Greeter {} }")
    end)

    it("parses a method inside a class", function()
        assert_parses_compilation_unit("class Program { void Main() {} }")
    end)
end)

describe("create_csharp_parser", function()
    it("returns a parser object", function()
        local parser = csharp_parser.create_csharp_parser("public class Foo {}")
        assert.is_not_nil(parser)
        assert.is_function(parser.parse)
    end)

    it("produces the same root rule as parse_csharp", function()
        local source = "public class Foo {}"
        local ast_direct = csharp_parser.parse_csharp(source)
        local parser = csharp_parser.create_csharp_parser(source)
        local ast_factory = parser:parse()

        assert.are.equal(ast_direct.rule_name, ast_factory.rule_name)
        assert.are.equal(#ast_direct.children, #ast_factory.children)
    end)

    it("accepts a version string", function()
        local parser = csharp_parser.create_csharp_parser("public class Foo {}", "8.0")
        local ast = parser:parse()
        assert.are.equal("compilation_unit", ast.rule_name)
    end)
end)

describe("version-aware parsing", function()
    local class_versions = {
        "1.0", "2.0", "3.0", "4.0", "5.0", "6.0",
        "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
    }

    for _, version in ipairs(class_versions) do
        it("parses a class declaration in C# " .. version, function()
            assert_parses_compilation_unit("public class Foo {}", version)
        end)
    end

    it("uses the default grammar when version is omitted", function()
        assert_parses_compilation_unit("public class Foo {}")
    end)

    it("uses the default grammar when version is empty", function()
        assert_parses_compilation_unit("public class Foo {}", "")
    end)
end)

describe("error handling", function()
    it("raises on unknown version string", function()
        assert.has_error(function()
            csharp_parser.parse_csharp("public class Foo {}", "99.0")
        end)
    end)

    it("raises on invalid version format", function()
        assert.has_error(function()
            csharp_parser.parse_csharp("public class Foo {}", "csharp12")
        end)
    end)
end)

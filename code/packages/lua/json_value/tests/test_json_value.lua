-- Tests for coding_adventures.json_value
-- ========================================
--
-- Comprehensive busted test suite for the json_value package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - M.null sentinel: identity, is_null, tostring
--   - evaluate: all scalar types (string, number, boolean, null)
--   - evaluate: simple object, nested object
--   - evaluate: simple array, nested array
--   - evaluate: mixed nested structure
--   - String escape sequences: \" \\ \/ \n \t \r \f \b \uXXXX
--   - from_string: one-step parse + evaluate
--   - to_json: all scalar types
--   - to_json: arrays (compact and pretty)
--   - to_json: objects (compact and pretty, key sorting)
--   - to_json: nested structures
--   - to_json: special number cases (integer vs float, NaN, Infinity)
--   - Round-trip: from_string → to_json → from_string

-- ---------------------------------------------------------------------------
-- Package path setup
--
-- When running via `busted . --pattern=test_` from the `tests/` directory,
-- relative paths must reach sibling Lua packages in the monorepo.
-- ---------------------------------------------------------------------------
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../json_parser/src/?.lua;"                            ..
    "../../json_parser/src/?/init.lua;"                       ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    "../../json_lexer/src/?.lua;"                             ..
    "../../json_lexer/src/?/init.lua;"                        ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    package.path
)

local jv = require("coding_adventures.json_value")

-- =========================================================================
-- Module surface
-- =========================================================================

describe("json_value module", function()
    it("loads successfully", function()
        assert.is_not_nil(jv)
    end)

    it("exposes VERSION string", function()
        assert.is_string(jv.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", jv.VERSION)
    end)

    it("exposes null sentinel", function()
        assert.is_not_nil(jv.null)
        assert.is_table(jv.null)
    end)

    it("exposes is_null function", function()
        assert.is_function(jv.is_null)
    end)

    it("exposes evaluate function", function()
        assert.is_function(jv.evaluate)
    end)

    it("exposes from_string function", function()
        assert.is_function(jv.from_string)
    end)

    it("exposes to_json function", function()
        assert.is_function(jv.to_json)
    end)
end)

-- =========================================================================
-- Null sentinel
-- =========================================================================

describe("null sentinel", function()
    it("null is its own identity (singleton)", function()
        assert.are.equal(jv.null, jv.null)
    end)

    it("null has a __tostring of 'null'", function()
        assert.are.equal("null", tostring(jv.null))
    end)

    it("is_null(jv.null) returns true", function()
        assert.is_true(jv.is_null(jv.null))
    end)

    it("is_null(nil) returns false", function()
        assert.is_false(jv.is_null(nil))
    end)

    it("is_null(false) returns false", function()
        assert.is_false(jv.is_null(false))
    end)

    it("is_null({}) returns false for an arbitrary table", function()
        assert.is_false(jv.is_null({}))
    end)

    it("is_null('null') returns false for the string 'null'", function()
        assert.is_false(jv.is_null("null"))
    end)
end)

-- =========================================================================
-- evaluate: scalar values
-- =========================================================================

describe("evaluate scalar values", function()
    it("evaluates a JSON string", function()
        local v = jv.from_string('"hello"')
        assert.are.equal("hello", v)
    end)

    it("evaluates an empty string", function()
        local v = jv.from_string('""')
        assert.are.equal("", v)
    end)

    it("evaluates an integer number", function()
        local v = jv.from_string("42")
        assert.are.equal(42, v)
    end)

    it("evaluates a negative number", function()
        local v = jv.from_string("-7")
        assert.are.equal(-7, v)
    end)

    it("evaluates a float number", function()
        local v = jv.from_string("3.14")
        -- Allow small floating-point imprecision
        assert.is_true(math.abs(v - 3.14) < 1e-10)
    end)

    it("evaluates true", function()
        local v = jv.from_string("true")
        assert.is_true(v)
    end)

    it("evaluates false", function()
        local v = jv.from_string("false")
        assert.is_false(v)
    end)

    it("evaluates null to the sentinel", function()
        local v = jv.from_string("null")
        assert.is_true(jv.is_null(v))
        assert.are.equal(jv.null, v)
    end)
end)

-- =========================================================================
-- evaluate: string escapes
-- =========================================================================

describe("evaluate string escape sequences", function()
    it("unescapes double quote: \\\"", function()
        local v = jv.from_string('"say \\"hi\\""')
        assert.are.equal('say "hi"', v)
    end)

    it("unescapes backslash: \\\\", function()
        local v = jv.from_string('"a\\\\b"')
        assert.are.equal("a\\b", v)
    end)

    it("unescapes forward slash: \\/", function()
        local v = jv.from_string('"a\\/b"')
        assert.are.equal("a/b", v)
    end)

    it("unescapes newline: \\n", function()
        local v = jv.from_string('"line1\\nline2"')
        assert.are.equal("line1\nline2", v)
    end)

    it("unescapes tab: \\t", function()
        local v = jv.from_string('"a\\tb"')
        assert.are.equal("a\tb", v)
    end)

    it("unescapes carriage return: \\r", function()
        local v = jv.from_string('"a\\rb"')
        assert.are.equal("a\rb", v)
    end)

    it("unescapes form feed: \\f", function()
        local v = jv.from_string('"a\\fb"')
        assert.are.equal("a\fb", v)
    end)

    it("unescapes backspace: \\b", function()
        local v = jv.from_string('"a\\bb"')
        assert.are.equal("a\bb", v)
    end)

    it("unescapes \\u0041 to 'A' (ASCII via Unicode escape)", function()
        local v = jv.from_string('"\\u0041"')
        assert.are.equal("A", v)
    end)

    it("unescapes \\u00e9 to é (Latin-1 via Unicode escape)", function()
        local v = jv.from_string('"\\u00e9"')
        -- U+00E9 = é, UTF-8 encoding = 0xC3 0xA9
        assert.are.equal("\xc3\xa9", v)
    end)

    it("unescapes \\u4e2d (CJK character) to UTF-8", function()
        local v = jv.from_string('"\\u4e2d"')
        -- U+4E2D = 中, UTF-8 = 0xE4 0xB8 0xAD
        assert.are.equal("\xe4\xb8\xad", v)
    end)
end)

-- =========================================================================
-- evaluate: objects
-- =========================================================================

describe("evaluate objects", function()
    it("evaluates an empty object to an empty table", function()
        local v = jv.from_string("{}")
        assert.is_table(v)
        assert.are.equal(0, #v)
        -- No keys
        local count = 0
        for _ in pairs(v) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("evaluates a simple key-value object", function()
        local v = jv.from_string('{"key": "value"}')
        assert.is_table(v)
        assert.are.equal("value", v["key"])
    end)

    it("evaluates object with integer value", function()
        local v = jv.from_string('{"n": 42}')
        assert.are.equal(42, v["n"])
    end)

    it("evaluates object with boolean values", function()
        local v = jv.from_string('{"a": true, "b": false}')
        assert.is_true(v["a"])
        assert.is_false(v["b"])
    end)

    it("evaluates object with null value", function()
        local v = jv.from_string('{"x": null}')
        assert.is_true(jv.is_null(v["x"]))
    end)

    it("evaluates object with multiple pairs", function()
        local v = jv.from_string('{"a": 1, "b": 2, "c": 3}')
        assert.are.equal(1, v["a"])
        assert.are.equal(2, v["b"])
        assert.are.equal(3, v["c"])
    end)

    it("evaluates nested objects", function()
        local v = jv.from_string('{"outer": {"inner": 99}}')
        assert.is_table(v["outer"])
        assert.are.equal(99, v["outer"]["inner"])
    end)
end)

-- =========================================================================
-- evaluate: arrays
-- =========================================================================

describe("evaluate arrays", function()
    it("evaluates an empty array to an empty table", function()
        local v = jv.from_string("[]")
        assert.is_table(v)
        assert.are.equal(0, #v)
    end)

    it("evaluates an array of numbers", function()
        local v = jv.from_string("[1, 2, 3]")
        assert.are.equal(1, v[1])
        assert.are.equal(2, v[2])
        assert.are.equal(3, v[3])
    end)

    it("evaluates an array of strings", function()
        local v = jv.from_string('["a", "b", "c"]')
        assert.are.equal("a", v[1])
        assert.are.equal("b", v[2])
        assert.are.equal("c", v[3])
    end)

    it("evaluates a mixed-type array", function()
        local v = jv.from_string('[1, "two", true, false, null]')
        assert.are.equal(1,    v[1])
        assert.are.equal("two", v[2])
        assert.is_true(v[3])
        assert.is_false(v[4])
        assert.is_true(jv.is_null(v[5]))
    end)

    it("evaluates a nested array", function()
        local v = jv.from_string("[[1, 2], [3, 4]]")
        assert.are.equal(1, v[1][1])
        assert.are.equal(2, v[1][2])
        assert.are.equal(3, v[2][1])
        assert.are.equal(4, v[2][2])
    end)

    it("evaluates array of objects", function()
        local v = jv.from_string('[{"id": 1}, {"id": 2}]')
        assert.are.equal(1, v[1]["id"])
        assert.are.equal(2, v[2]["id"])
    end)
end)

-- =========================================================================
-- evaluate: complex mixed structure
-- =========================================================================

describe("evaluate complex mixed structure", function()
    it("evaluates a realistic JSON document", function()
        local src = [[
{
  "name": "Alice",
  "age": 30,
  "active": true,
  "score": -1.5,
  "tags": ["lua", "json"],
  "address": {
    "city": "Metropolis",
    "zip": null
  }
}]]
        local v = jv.from_string(src)
        assert.are.equal("Alice",       v["name"])
        assert.are.equal(30,            v["age"])
        assert.is_true(v["active"])
        assert.is_true(math.abs(v["score"] - (-1.5)) < 1e-10)
        assert.are.equal("lua",         v["tags"][1])
        assert.are.equal("json",        v["tags"][2])
        assert.are.equal("Metropolis",  v["address"]["city"])
        assert.is_true(jv.is_null(v["address"]["zip"]))
    end)
end)

-- =========================================================================
-- to_json: scalar values
-- =========================================================================

describe("to_json scalar values", function()
    it("serializes nil to null", function()
        assert.are.equal("null", jv.to_json(nil))
    end)

    it("serializes M.null to null", function()
        assert.are.equal("null", jv.to_json(jv.null))
    end)

    it("serializes true to 'true'", function()
        assert.are.equal("true", jv.to_json(true))
    end)

    it("serializes false to 'false'", function()
        assert.are.equal("false", jv.to_json(false))
    end)

    it("serializes an integer without decimal point", function()
        assert.are.equal("42", jv.to_json(42))
    end)

    it("serializes a negative integer", function()
        assert.are.equal("-7", jv.to_json(-7))
    end)

    it("serializes zero as 0", function()
        assert.are.equal("0", jv.to_json(0))
    end)

    it("serializes a float with decimal", function()
        local s = jv.to_json(3.14)
        -- Should contain a decimal point
        assert.matches("%.", s)
    end)

    it("serializes a plain string", function()
        assert.are.equal('"hello"', jv.to_json("hello"))
    end)

    it("serializes an empty string", function()
        assert.are.equal('""', jv.to_json(""))
    end)

    it("serializes NaN to null", function()
        assert.are.equal("null", jv.to_json(0/0))
    end)

    it("serializes Infinity to null", function()
        assert.are.equal("null", jv.to_json(math.huge))
    end)

    it("serializes -Infinity to null", function()
        assert.are.equal("null", jv.to_json(-math.huge))
    end)
end)

-- =========================================================================
-- to_json: string escaping
-- =========================================================================

describe("to_json string escaping", function()
    it("escapes double quotes", function()
        assert.are.equal('"say \\"hi\\""', jv.to_json('say "hi"'))
    end)

    it("escapes backslashes", function()
        assert.are.equal('"a\\\\b"', jv.to_json("a\\b"))
    end)

    it("escapes newlines", function()
        assert.are.equal('"a\\nb"', jv.to_json("a\nb"))
    end)

    it("escapes tabs", function()
        assert.are.equal('"a\\tb"', jv.to_json("a\tb"))
    end)

    it("escapes carriage return", function()
        assert.are.equal('"a\\rb"', jv.to_json("a\rb"))
    end)

    it("escapes form feed", function()
        assert.are.equal('"a\\fb"', jv.to_json("a\fb"))
    end)

    it("escapes backspace", function()
        assert.are.equal('"a\\bb"', jv.to_json("a\bb"))
    end)

    it("escapes control chars with \\uXXXX", function()
        -- U+0001 (SOH) has no short form
        local s = jv.to_json("\x01")
        assert.matches("\\u0001", s)
    end)
end)

-- =========================================================================
-- to_json: arrays
-- =========================================================================

describe("to_json arrays", function()
    it("serializes an empty array as []", function()
        -- Note: {} in Lua becomes {} (object), not []; use a non-empty array
        -- to test array detection.  Empty Lua table → "{}".
        assert.are.equal("{}", jv.to_json({}))
    end)

    it("serializes a sequence table as a JSON array", function()
        assert.are.equal("[1,2,3]", jv.to_json({1, 2, 3}))
    end)

    it("serializes an array of strings", function()
        assert.are.equal('["a","b"]', jv.to_json({"a", "b"}))
    end)

    it("serializes an array with null sentinel", function()
        assert.are.equal("[1,null,3]", jv.to_json({1, jv.null, 3}))
    end)

    it("serializes nested array", function()
        assert.are.equal("[[1,2],[3,4]]", jv.to_json({{1,2},{3,4}}))
    end)

    it("pretty-prints an array with indent=2", function()
        local s = jv.to_json({1, 2, 3}, 2)
        -- Should contain newlines and spaces
        assert.matches("\n", s)
        assert.matches("  1", s)
    end)
end)

-- =========================================================================
-- to_json: objects
-- =========================================================================

describe("to_json objects", function()
    it("serializes an empty table as {}", function()
        assert.are.equal("{}", jv.to_json({}))
    end)

    it("serializes a simple object", function()
        local s = jv.to_json({key = "value"})
        assert.are.equal('{"key":"value"}', s)
    end)

    it("serializes object with multiple keys sorted alphabetically", function()
        local s = jv.to_json({b = 2, a = 1, c = 3})
        -- Keys must be sorted: a, b, c
        assert.are.equal('{"a":1,"b":2,"c":3}', s)
    end)

    it("pretty-prints an object with indent=2", function()
        local s = jv.to_json({key = "value"}, 2)
        assert.matches("\n", s)
        assert.matches("  ", s)
    end)

    it("serializes nested object", function()
        local s = jv.to_json({a = {b = 1}})
        assert.are.equal('{"a":{"b":1}}', s)
    end)
end)

-- =========================================================================
-- to_json: mixed nested
-- =========================================================================

describe("to_json mixed nested structures", function()
    it("serializes object containing array", function()
        local s = jv.to_json({tags = {"lua", "json"}})
        assert.are.equal('{"tags":["lua","json"]}', s)
    end)

    it("serializes array of objects", function()
        local s = jv.to_json({{id = 1}, {id = 2}})
        assert.are.equal('[{"id":1},{"id":2}]', s)
    end)
end)

-- =========================================================================
-- Round-trip: from_string → to_json → from_string
-- =========================================================================

describe("round-trip", function()
    local function round_trip(json_str)
        local v1 = jv.from_string(json_str)
        local s  = jv.to_json(v1)
        local v2 = jv.from_string(s)
        return v1, v2
    end

    it("round-trips a number", function()
        local v1, v2 = round_trip("42")
        assert.are.equal(v1, v2)
    end)

    it("round-trips a string", function()
        local v1, v2 = round_trip('"hello world"')
        assert.are.equal(v1, v2)
    end)

    it("round-trips true", function()
        local v1, v2 = round_trip("true")
        assert.are.equal(v1, v2)
    end)

    it("round-trips false", function()
        local v1, v2 = round_trip("false")
        assert.are.equal(v1, v2)
    end)

    it("round-trips null", function()
        local v1, v2 = round_trip("null")
        assert.is_true(jv.is_null(v1))
        assert.is_true(jv.is_null(v2))
    end)

    it("round-trips an array of numbers", function()
        local v1, v2 = round_trip("[1,2,3]")
        assert.are.same(v1, v2)
    end)

    it("round-trips a simple object", function()
        local v1, v2 = round_trip('{"x":1,"y":2}')
        assert.are.equal(v1["x"], v2["x"])
        assert.are.equal(v1["y"], v2["y"])
    end)

    it("round-trips a string with escape sequences", function()
        local v1, v2 = round_trip('"hello\\nworld"')
        assert.are.equal(v1, v2)
        assert.matches("\n", v1)
    end)

    it("round-trips a complex nested structure", function()
        local src = '{"name":"Bob","scores":[10,20,30],"active":true}'
        local v1, v2 = round_trip(src)
        assert.are.equal(v1["name"],         v2["name"])
        assert.are.equal(v1["scores"][2],    v2["scores"][2])
        assert.are.equal(v1["active"],       v2["active"])
    end)
end)

-- =========================================================================
-- Pretty-print: indent produces valid JSON
-- =========================================================================

describe("pretty-printing produces valid JSON", function()
    it("pretty-printed output can be parsed back", function()
        local data = {name = "Alice", age = 30, tags = {"lua", "json"}}
        local pretty = jv.to_json(data, 2)
        -- Must be parseable
        local v = jv.from_string(pretty)
        assert.are.equal("Alice", v["name"])
        assert.are.equal(30,      v["age"])
        assert.are.equal("lua",   v["tags"][1])
    end)
end)

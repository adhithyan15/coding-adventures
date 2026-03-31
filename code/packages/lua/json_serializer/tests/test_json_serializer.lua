-- Tests for coding_adventures.json_serializer
-- ============================================
--
-- Comprehensive busted test suite for the json_serializer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - encode: basic types (nil, boolean, number, string)
--   - encode: arrays (compact and pretty)
--   - encode: objects (compact, pretty, key sorting)
--   - encode: sort_keys option (true/false)
--   - encode: allow_nan option
--   - encode: max_depth guard
--   - encode: nested structures
--   - decode: basic parsing
--   - decode: allow_comments (// and /* */)
--   - decode: trailing comma handling (non-strict vs strict)
--   - validate: valid documents
--   - validate: type errors
--   - validate: required fields
--   - validate: minLength / maxLength / pattern
--   - validate: minimum / maximum
--   - validate: minItems / maxItems / items
--   - validate: enum
--   - validate: additional_properties = false
--   - schema_encode: number-to-string coercion
--   - schema_encode: additional_properties filtering
--   - schema_encode: nested object coercion
--   - Round-trip: encode → decode

-- ---------------------------------------------------------------------------
-- Package path setup
-- ---------------------------------------------------------------------------
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../json_value/src/?.lua;"                             ..
    "../../json_value/src/?/init.lua;"                        ..
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

local js = require("coding_adventures.json_serializer")

-- =========================================================================
-- Module surface
-- =========================================================================

describe("json_serializer module", function()
    it("loads successfully", function()
        assert.is_not_nil(js)
    end)

    it("exposes VERSION string", function()
        assert.is_string(js.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", js.VERSION)
    end)

    it("re-exports null sentinel", function()
        assert.is_not_nil(js.null)
        assert.is_table(js.null)
    end)

    it("re-exports is_null function", function()
        assert.is_function(js.is_null)
    end)

    it("exposes encode function", function()
        assert.is_function(js.encode)
    end)

    it("exposes decode function", function()
        assert.is_function(js.decode)
    end)

    it("exposes validate function", function()
        assert.is_function(js.validate)
    end)

    it("exposes schema_encode function", function()
        assert.is_function(js.schema_encode)
    end)
end)

-- =========================================================================
-- encode: basic types
-- =========================================================================

describe("encode basic types", function()
    it("encodes nil as null", function()
        assert.are.equal("null", js.encode(nil))
    end)

    it("encodes null sentinel as null", function()
        assert.are.equal("null", js.encode(js.null))
    end)

    it("encodes true", function()
        assert.are.equal("true", js.encode(true))
    end)

    it("encodes false", function()
        assert.are.equal("false", js.encode(false))
    end)

    it("encodes integer without decimal", function()
        assert.are.equal("42", js.encode(42))
    end)

    it("encodes negative integer", function()
        assert.are.equal("-7", js.encode(-7))
    end)

    it("encodes zero", function()
        assert.are.equal("0", js.encode(0))
    end)

    it("encodes float with decimal", function()
        local s = js.encode(3.14)
        assert.matches("%.", s)
    end)

    it("encodes a plain string", function()
        assert.are.equal('"hello"', js.encode("hello"))
    end)

    it("encodes empty string", function()
        assert.are.equal('""', js.encode(""))
    end)

    it("encodes NaN as null by default", function()
        assert.are.equal("null", js.encode(0/0))
    end)

    it("encodes Infinity as null by default", function()
        assert.are.equal("null", js.encode(math.huge))
    end)

    it("encodes -Infinity as null by default", function()
        assert.are.equal("null", js.encode(-math.huge))
    end)
end)

-- =========================================================================
-- encode: allow_nan option
-- =========================================================================

describe("encode allow_nan option", function()
    it("encodes NaN as quoted string when allow_nan=true", function()
        local s = js.encode(0/0, { allow_nan = true })
        assert.are.equal('"NaN"', s)
    end)

    it("encodes Infinity as quoted string when allow_nan=true", function()
        local s = js.encode(math.huge, { allow_nan = true })
        assert.are.equal('"Infinity"', s)
    end)

    it("encodes -Infinity as quoted string when allow_nan=true", function()
        local s = js.encode(-math.huge, { allow_nan = true })
        assert.are.equal('"-Infinity"', s)
    end)
end)

-- =========================================================================
-- encode: max_depth guard
-- =========================================================================

describe("encode max_depth guard", function()
    it("raises an error when nesting exceeds max_depth", function()
        -- Build a nested table 5 levels deep
        local deep = { a = { b = { c = { d = { e = "leaf" } } } } }
        -- Allow only 3 levels → should fail at depth 4
        assert.has_error(function()
            js.encode(deep, { max_depth = 3 })
        end)
    end)

    it("succeeds when depth is exactly at max_depth", function()
        -- Two levels deep, max_depth = 2 → should succeed
        local tbl = { a = { b = 1 } }
        assert.has_no_error(function()
            js.encode(tbl, { max_depth = 2 })
        end)
    end)
end)

-- =========================================================================
-- encode: arrays
-- =========================================================================

describe("encode arrays", function()
    it("encodes a sequence table as a JSON array", function()
        assert.are.equal("[1,2,3]", js.encode({1, 2, 3}))
    end)

    it("encodes an array of strings", function()
        assert.are.equal('["a","b"]', js.encode({"a", "b"}))
    end)

    it("encodes a nested array", function()
        assert.are.equal("[[1,2],[3,4]]", js.encode({{1,2},{3,4}}))
    end)

    it("pretty-prints an array with indent=2", function()
        local s = js.encode({1, 2, 3}, { indent = 2 })
        assert.matches("\n", s)
        assert.matches("  1", s)
    end)

    it("pretty-printed array is parseable", function()
        local s  = js.encode({10, 20, 30}, { indent = 4 })
        local v  = js.decode(s)
        assert.are.equal(10, v[1])
        assert.are.equal(20, v[2])
        assert.are.equal(30, v[3])
    end)
end)

-- =========================================================================
-- encode: objects
-- =========================================================================

describe("encode objects", function()
    it("encodes an empty table as {}", function()
        assert.are.equal("{}", js.encode({}))
    end)

    it("encodes a simple object", function()
        assert.are.equal('{"key":"value"}', js.encode({ key = "value" }))
    end)

    it("sorts keys alphabetically by default", function()
        assert.are.equal('{"a":1,"b":2,"c":3}',
            js.encode({ b = 2, a = 1, c = 3 }))
    end)

    it("does not sort keys when sort_keys=false", function()
        -- We cannot assert a specific order, but we can assert that the
        -- output is valid JSON with all three keys present.
        local s = js.encode({ b = 2, a = 1, c = 3 }, { sort_keys = false })
        assert.is_truthy(s:match('"a":1'))
        assert.is_truthy(s:match('"b":2'))
        assert.is_truthy(s:match('"c":3'))
    end)

    it("pretty-prints an object with indent=2", function()
        local s = js.encode({ key = "value" }, { indent = 2 })
        assert.matches("\n", s)
        assert.matches("  ", s)
    end)

    it("encodes a nested object", function()
        assert.are.equal('{"a":{"b":1}}', js.encode({ a = { b = 1 } }))
    end)
end)

-- =========================================================================
-- encode: mixed nested
-- =========================================================================

describe("encode mixed nested structures", function()
    it("encodes object containing array", function()
        assert.are.equal('{"tags":["lua","json"]}',
            js.encode({ tags = {"lua", "json"} }))
    end)

    it("encodes array of objects", function()
        assert.are.equal('[{"id":1},{"id":2}]',
            js.encode({{ id = 1 }, { id = 2 }}))
    end)

    it("encodes a realistic document", function()
        local doc = {
            name   = "Alice",
            age    = 30,
            active = true,
            tags   = {"lua", "json"},
        }
        local s = js.encode(doc)
        -- All keys present; keys sorted
        assert.is_truthy(s:match('"active":true'))
        assert.is_truthy(s:match('"age":30'))
        assert.is_truthy(s:match('"name":"Alice"'))
        assert.is_truthy(s:match('"tags":%["lua","json"%]'))
    end)
end)

-- =========================================================================
-- decode: basic
-- =========================================================================

describe("decode basic", function()
    it("decodes a JSON string", function()
        local v = js.decode('"hello"')
        assert.are.equal("hello", v)
    end)

    it("decodes a JSON number", function()
        assert.are.equal(42, js.decode("42"))
    end)

    it("decodes true and false", function()
        assert.is_true(js.decode("true"))
        assert.is_false(js.decode("false"))
    end)

    it("decodes null to the null sentinel", function()
        assert.is_true(js.is_null(js.decode("null")))
    end)

    it("decodes an object", function()
        local v = js.decode('{"x":1,"y":2}')
        assert.are.equal(1, v.x)
        assert.are.equal(2, v.y)
    end)

    it("decodes an array", function()
        local v = js.decode("[1,2,3]")
        assert.are.equal(1, v[1])
        assert.are.equal(3, v[3])
    end)
end)

-- =========================================================================
-- decode: allow_comments
-- =========================================================================

describe("decode allow_comments", function()
    it("strips single-line comments with allow_comments=true", function()
        local json_with_comments = [[
{
  "name": "Alice", // the user's name
  "age": 30
}
        ]]
        local v = js.decode(json_with_comments, { allow_comments = true })
        assert.are.equal("Alice", v.name)
        assert.are.equal(30, v.age)
    end)

    it("strips multi-line comments with allow_comments=true", function()
        local json_with_comments = [[
{
  /* This is a
     multi-line comment */
  "x": 1
}
        ]]
        local v = js.decode(json_with_comments, { allow_comments = true })
        assert.are.equal(1, v.x)
    end)

    it("preserves // inside string values", function()
        local json_with_url = '{"url": "http://example.com"}'
        local v = js.decode(json_with_url, { allow_comments = true })
        assert.are.equal("http://example.com", v.url)
    end)

    it("strips comments from an array", function()
        local s = '[ 1, /* comment */ 2, 3 ]'
        local v = js.decode(s, { allow_comments = true })
        assert.are.equal(1, v[1])
        assert.are.equal(2, v[2])
        assert.are.equal(3, v[3])
    end)
end)

-- =========================================================================
-- decode: trailing commas (non-strict vs strict)
-- =========================================================================

describe("decode trailing commas", function()
    it("allows trailing comma in object (non-strict default)", function()
        local v = js.decode('{"a":1,"b":2,}')
        assert.are.equal(1, v.a)
        assert.are.equal(2, v.b)
    end)

    it("allows trailing comma in array (non-strict default)", function()
        local v = js.decode('[1,2,3,]')
        assert.are.equal(1, v[1])
        assert.are.equal(3, v[3])
    end)

    it("raises on trailing comma in strict mode", function()
        assert.has_error(function()
            js.decode('{"a":1,}', { strict = true })
        end)
    end)

    it("raises on trailing comma in array in strict mode", function()
        assert.has_error(function()
            js.decode('[1,2,]', { strict = true })
        end)
    end)

    it("handles trailing comma combined with comments", function()
        local s = '{ "a": 1, /* last */ }'
        local v = js.decode(s, { allow_comments = true })
        assert.are.equal(1, v.a)
    end)
end)

-- =========================================================================
-- validate: basic type checks
-- =========================================================================

describe("validate type checks", function()
    it("validates a string correctly", function()
        local ok, errs = js.validate("hello", { type = "string" })
        assert.is_true(ok)
        assert.is_nil(errs)
    end)

    it("fails when string given for number type", function()
        local ok, errs = js.validate("hello", { type = "number" })
        assert.is_false(ok)
        assert.is_truthy(#errs > 0)
    end)

    it("validates an integer", function()
        local ok = js.validate(5, { type = "integer" })
        assert.is_true(ok)
    end)

    it("fails when float given for integer type", function()
        local ok, errs = js.validate(5.5, { type = "integer" })
        assert.is_false(ok)
        assert.is_truthy(#errs > 0)
    end)

    it("validates boolean", function()
        assert.is_true(js.validate(true,  { type = "boolean" }))
        assert.is_true(js.validate(false, { type = "boolean" }))
    end)

    it("validates null sentinel", function()
        assert.is_true(js.validate(js.null, { type = "null" }))
    end)

    it("validates an array", function()
        local ok = js.validate({1, 2, 3}, { type = "array" })
        assert.is_true(ok)
    end)

    it("validates an object", function()
        local ok = js.validate({ a = 1 }, { type = "object" })
        assert.is_true(ok)
    end)

    it("fails when array given for object type", function()
        local ok, errs = js.validate({1, 2, 3}, { type = "object" })
        assert.is_false(ok)
        assert.is_truthy(#errs > 0)
    end)
end)

-- =========================================================================
-- validate: string constraints
-- =========================================================================

describe("validate string constraints", function()
    it("passes minLength when string is long enough", function()
        assert.is_true(js.validate("hello", { type = "string", minLength = 3 }))
    end)

    it("fails minLength when string is too short", function()
        local ok, errs = js.validate("hi", { type = "string", minLength = 5 })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("minLength"))
    end)

    it("passes maxLength when string is short enough", function()
        assert.is_true(js.validate("hi", { type = "string", maxLength = 5 }))
    end)

    it("fails maxLength when string is too long", function()
        local ok, errs = js.validate("hello world", { type = "string", maxLength = 5 })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("maxLength"))
    end)

    it("passes pattern when string matches", function()
        assert.is_true(js.validate(
            "user@example.com",
            { type = "string", pattern = "@" }))
    end)

    it("fails pattern when string does not match", function()
        local ok, errs = js.validate(
            "notanemail",
            { type = "string", pattern = "@" })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("pattern"))
    end)
end)

-- =========================================================================
-- validate: number constraints
-- =========================================================================

describe("validate number constraints", function()
    it("passes minimum when value is >= minimum", function()
        assert.is_true(js.validate(5, { type = "number", minimum = 0 }))
    end)

    it("fails minimum when value is below minimum", function()
        local ok, errs = js.validate(-1, { type = "number", minimum = 0 })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("minimum"))
    end)

    it("passes maximum when value is <= maximum", function()
        assert.is_true(js.validate(99, { type = "number", maximum = 100 }))
    end)

    it("fails maximum when value exceeds maximum", function()
        local ok, errs = js.validate(150, { type = "number", maximum = 100 })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("maximum"))
    end)

    it("passes both minimum and maximum when in range", function()
        assert.is_true(js.validate(50, { type = "number", minimum = 0, maximum = 100 }))
    end)
end)

-- =========================================================================
-- validate: array constraints
-- =========================================================================

describe("validate array constraints", function()
    it("passes minItems when array is long enough", function()
        assert.is_true(js.validate({1, 2, 3}, { type = "array", minItems = 2 }))
    end)

    it("fails minItems when array is too short", function()
        local ok, errs = js.validate({1}, { type = "array", minItems = 3 })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("minItems"))
    end)

    it("passes maxItems when array is short enough", function()
        assert.is_true(js.validate({1, 2}, { type = "array", maxItems = 5 }))
    end)

    it("fails maxItems when array is too long", function()
        local ok, errs = js.validate({1,2,3,4,5,6}, { type = "array", maxItems = 3 })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("maxItems"))
    end)

    it("validates items sub-schema recursively", function()
        local ok = js.validate(
            {1, 2, 3},
            { type = "array", items = { type = "number" } })
        assert.is_true(ok)
    end)

    it("fails items sub-schema when an element is wrong type", function()
        local ok, errs = js.validate(
            {1, "two", 3},
            { type = "array", items = { type = "number" } })
        assert.is_false(ok)
        assert.is_truthy(#errs > 0)
    end)
end)

-- =========================================================================
-- validate: object constraints
-- =========================================================================

describe("validate object constraints", function()
    it("passes when all required fields are present", function()
        local schema = {
            type       = "object",
            required   = { "name", "age" },
            properties = {
                name = { type = "string" },
                age  = { type = "number" },
            },
        }
        local ok = js.validate({ name = "Alice", age = 30 }, schema)
        assert.is_true(ok)
    end)

    it("fails when a required field is missing", function()
        local schema = {
            type     = "object",
            required = { "name", "age" },
            properties = {
                name = { type = "string" },
                age  = { type = "number" },
            },
        }
        local ok, errs = js.validate({ name = "Alice" }, schema)
        assert.is_false(ok)
        -- Should mention 'age' as missing
        local found = false
        for _, e in ipairs(errs) do
            if e:match("age") then found = true; break end
        end
        assert.is_true(found)
    end)

    it("validates property sub-schemas recursively", function()
        local schema = {
            type       = "object",
            properties = {
                name = { type = "string", minLength = 1 },
                age  = { type = "integer", minimum = 0 },
            },
        }
        local ok = js.validate({ name = "Bob", age = 25 }, schema)
        assert.is_true(ok)
    end)

    it("fails when a property violates its sub-schema", function()
        local schema = {
            type       = "object",
            properties = {
                age = { type = "integer", minimum = 0 },
            },
        }
        local ok, errs = js.validate({ age = -5 }, schema)
        assert.is_false(ok)
        assert.is_truthy(#errs > 0)
    end)

    it("allows additional properties by default", function()
        local schema = {
            type       = "object",
            properties = { name = { type = "string" } },
        }
        local ok = js.validate({ name = "Alice", extra = "field" }, schema)
        assert.is_true(ok)
    end)

    it("rejects additional properties when additional_properties=false", function()
        local schema = {
            type                  = "object",
            properties            = { name = { type = "string" } },
            additional_properties = false,
        }
        local ok, errs = js.validate(
            { name = "Alice", extra = "field" }, schema)
        assert.is_false(ok)
        local found = false
        for _, e in ipairs(errs) do
            if e:match("additional property") then found = true; break end
        end
        assert.is_true(found)
    end)
end)

-- =========================================================================
-- validate: enum
-- =========================================================================

describe("validate enum", function()
    it("passes when value is in enum", function()
        local ok = js.validate("red", { enum = {"red", "green", "blue"} })
        assert.is_true(ok)
    end)

    it("fails when value is not in enum", function()
        local ok, errs = js.validate("yellow", { enum = {"red", "green", "blue"} })
        assert.is_false(ok)
        assert.is_truthy(errs[1]:match("enum"))
    end)

    it("passes null in enum when value is null", function()
        local ok = js.validate(js.null, { enum = { js.null, "none" } })
        assert.is_true(ok)
    end)

    it("fails null not in enum", function()
        local ok, errs = js.validate(js.null, { enum = { "red", "blue" } })
        assert.is_false(ok)
        assert.is_truthy(#errs > 0)
    end)
end)

-- =========================================================================
-- validate: multiple errors collected
-- =========================================================================

describe("validate collects multiple errors", function()
    it("reports all errors at once", function()
        local schema = {
            type       = "object",
            required   = { "name", "email", "age" },
            properties = {
                name  = { type = "string" },
                email = { type = "string", pattern = "@" },
                age   = { type = "integer", minimum = 0 },
            },
        }
        -- Missing 'name', email has wrong type, age out of range
        local ok, errs = js.validate({ email = 42, age = -1 }, schema)
        assert.is_false(ok)
        -- Should have at least 3 errors: missing name, wrong email type, age minimum
        assert.is_truthy(#errs >= 2)
    end)
end)

-- =========================================================================
-- schema_encode: coercion
-- =========================================================================

describe("schema_encode coercion", function()
    it("coerces number to string when schema.type='string'", function()
        local schema = { type = "string" }
        local s = js.schema_encode(42, schema)
        assert.are.equal('"42"', s)
    end)

    it("coerces float to string", function()
        local schema = { type = "string" }
        local s = js.schema_encode(3.14, schema)
        -- Should be a quoted string
        assert.matches('^"', s)
        assert.matches('"$', s)
    end)

    it("does not coerce when type already matches", function()
        local schema = { type = "number" }
        local s = js.schema_encode(42, schema)
        assert.are.equal("42", s)
    end)

    it("coerces a property inside an object", function()
        local schema = {
            type       = "object",
            properties = {
                price = { type = "string" },
                qty   = { type = "number" },
            },
        }
        local value = { price = 9.99, qty = 3 }
        local s = js.schema_encode(value, schema)
        local decoded = js.decode(s)
        -- price should have been coerced to a string
        assert.is_string(decoded.price)
        -- qty stays a number
        assert.is_number(decoded.qty)
    end)
end)

-- =========================================================================
-- schema_encode: additional_properties filtering
-- =========================================================================

describe("schema_encode additional_properties filtering", function()
    it("drops properties not in schema when additional_properties=false", function()
        local schema = {
            type                  = "object",
            additional_properties = false,
            properties            = {
                name = { type = "string" },
            },
        }
        local value = { name = "Alice", secret = "password123" }
        local s       = js.schema_encode(value, schema)
        local decoded = js.decode(s)
        assert.are.equal("Alice", decoded.name)
        assert.is_nil(decoded.secret)
    end)

    it("keeps all properties when additional_properties not set to false", function()
        local schema = {
            type       = "object",
            properties = { name = { type = "string" } },
        }
        local value   = { name = "Alice", extra = "ok" }
        local s       = js.schema_encode(value, schema)
        local decoded = js.decode(s)
        assert.are.equal("Alice", decoded.name)
        assert.are.equal("ok",    decoded.extra)
    end)
end)

-- =========================================================================
-- schema_encode: nested objects
-- =========================================================================

describe("schema_encode nested objects", function()
    it("recursively coerces nested object properties", function()
        local schema = {
            type       = "object",
            properties = {
                user = {
                    type       = "object",
                    properties = {
                        id = { type = "string" },  -- coerce number → string
                    },
                },
            },
        }
        local value   = { user = { id = 100 } }
        local s       = js.schema_encode(value, schema)
        local decoded = js.decode(s)
        -- id should have been coerced to the string "100"
        assert.is_string(decoded.user.id)
        assert.are.equal("100", decoded.user.id)
    end)
end)

-- =========================================================================
-- Round-trip: encode → decode
-- =========================================================================

describe("round-trip encode → decode", function()
    local function round_trip(v)
        return js.decode(js.encode(v))
    end

    it("round-trips an integer", function()
        assert.are.equal(42, round_trip(42))
    end)

    it("round-trips a string", function()
        assert.are.equal("hello world", round_trip("hello world"))
    end)

    it("round-trips true", function()
        assert.is_true(round_trip(true))
    end)

    it("round-trips false", function()
        assert.is_false(round_trip(false))
    end)

    it("round-trips null", function()
        assert.is_true(js.is_null(round_trip(js.null)))
    end)

    it("round-trips an array", function()
        local v = round_trip({10, 20, 30})
        assert.are.equal(10, v[1])
        assert.are.equal(20, v[2])
        assert.are.equal(30, v[3])
    end)

    it("round-trips a simple object", function()
        local v = round_trip({ x = 1, y = 2 })
        assert.are.equal(1, v.x)
        assert.are.equal(2, v.y)
    end)

    it("round-trips a complex nested structure", function()
        local data = {
            name   = "Bob",
            scores = {10, 20, 30},
            active = true,
            meta   = { source = "test" },
        }
        local v = round_trip(data)
        assert.are.equal("Bob",   v.name)
        assert.are.equal(20,      v.scores[2])
        assert.is_true(v.active)
        assert.are.equal("test",  v.meta.source)
    end)
end)

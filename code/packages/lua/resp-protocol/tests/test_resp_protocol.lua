package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    package.path,
}, ";")

local resp = require("coding_adventures.resp_protocol")
local Value = resp.Value

local function assert_round_trip(value)
    local wire = resp.encode(value)
    local decoded, next_offset = resp.decode(wire)
    assert.equals(#wire + 1, next_offset)
    assert.is_true(resp.equal(value, decoded))
end

describe("RESP values", function()
    it("constructs and validates every wire kind", function()
        assert.equals("OK", Value.simple_string("OK").value)
        local err = Value.error("WRONGTYPE bad value")
        assert.equals("WRONGTYPE", err.error_type)
        assert.equals("bad value", err.detail)
        assert.equals(42, Value.integer(42).value)
        assert.is_true(Value.null_bulk_string().is_null)
        assert.is_true(Value.null_array().is_null)
        assert.is_truthy(tostring(Value.array({})):match("0 values"))

        assert.has_error(function() Value.new("unknown") end, "invalid RESP kind: unknown")
        assert.has_error(function() Value.simple_string("bad\rline") end, "simple_string value must not contain CR or LF")
        assert.has_error(function() Value.integer(1.5) end, "integer value must be an integer")
        assert.has_error(function() Value.array({ "not-a-value" }) end, "array value[1] must be a RESP Value")
    end)
end)

describe("RESP encoding", function()
    it("encodes scalar and null values exactly", function()
        assert.equals("+OK\r\n", resp.encode(Value.simple_string("OK")))
        assert.equals("-ERR boom\r\n", resp.encode(Value.error("ERR boom")))
        assert.equals(":-42\r\n", resp.encode(Value.integer(-42)))
        assert.equals("$5\r\nhello\r\n", resp.encode(Value.bulk_string("hello")))
        assert.equals("$0\r\n\r\n", resp.encode(Value.bulk_string("")))
        assert.equals("$-1\r\n", resp.encode(Value.null_bulk_string()))
        assert.equals("*-1\r\n", resp.encode(Value.null_array()))
    end)

    it("encodes binary and nested arrays", function()
        local binary = "\0foo\r\nbar\255"
        assert.equals("$10\r\n" .. binary .. "\r\n", resp.encode_bulk_string(binary))
        local command = Value.array({
            Value.bulk_string("SET"),
            Value.bulk_string("key"),
            Value.array({ Value.integer(1), Value.null_bulk_string() }),
        })
        assert.equals(
            "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n*2\r\n:1\r\n$-1\r\n",
            resp.encode(command)
        )
        assert.equals(":1\r\n:0\r\n$1\r\nx\r\n", resp.encode_many({ true, false, "x" }))
    end)
end)

describe("RESP decoding", function()
    it("decodes all typed values and offsets", function()
        local wire = "+OK\r\n-ERR boom\r\n:42\r\n$3\r\nfoo\r\n$-1\r\n*-1\r\n"
        local values, next_offset = resp.decode_all(wire)
        assert.equals(6, #values)
        assert.equals(#wire + 1, next_offset)
        assert.equals("simple_string", values[1].kind)
        assert.equals("error", values[2].kind)
        assert.equals(42, values[3].value)
        assert.equals("foo", values[4].value)
        assert.is_true(values[5].is_null)
        assert.is_true(values[6].is_null)
        assert.equals("array", values[6].kind)
    end)

    it("returns no value and consumes nothing for every incomplete prefix", function()
        local full = "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
        for length = 0, #full - 1 do
            local value, next_offset = resp.decode(full:sub(1, length))
            assert.is_nil(value)
            assert.equals(1, next_offset)
        end
        local value, next_offset = resp.decode(full)
        assert.equals(#full + 1, next_offset)
        assert.equals(3, #value.value)
    end)

    it("rejects malformed integers, lengths, and terminators", function()
        assert.has_error(function() resp.decode(":abc\r\n") end, "invalid RESP integer: abc")
        assert.has_error(function() resp.decode("$-2\r\n") end, "invalid negative bulk length: -2")
        assert.has_error(function() resp.decode("*-2\r\n") end, "invalid negative array length: -2")
        assert.has_error(function() resp.decode("$3\r\nfooXX") end, "bulk string missing trailing CRLF")
        assert.has_error(function() resp.decode("+OK\r\n", 0) end, "offset is outside data")
    end)

    it("decodes inline commands and preserves incomplete tails", function()
        local inline, next_offset = resp.decode("SET key value\r\n")
        assert.equals(3, #inline.value)
        assert.equals("SET", inline.value[1].value)
        assert.equals(16, next_offset)

        local values, tail_offset = resp.decode_all("+OK\r\n:1\r\n$5\r\nhel")
        assert.equals(2, #values)
        assert.equals(10, tail_offset)
    end)
end)

describe("round trips and streaming", function()
    it("round-trips nested and all-byte bulk values", function()
        local bytes = {}
        for value = 0, 255 do
            bytes[#bytes + 1] = string.char(value)
        end
        assert_round_trip(Value.bulk_string(table.concat(bytes)))
        assert_round_trip(Value.array({
            Value.simple_string("OK"),
            Value.error("ERR boom"),
            Value.integer(-1),
            Value.bulk_string("payload\0"),
            Value.null_bulk_string(),
            Value.array({ Value.integer(2) }),
            Value.null_array(),
        }))
    end)

    it("handles byte-by-byte fragmentation and multiple messages", function()
        local expected = {
            Value.array({ Value.bulk_string("PING") }),
            Value.integer(42),
            Value.error("ERR bad"),
        }
        local wire = resp.encode_many(expected)
        local decoder = resp.Decoder.new()
        for index = 1, #wire do
            decoder:feed(wire:sub(index, index))
        end
        assert.equals(0, decoder:pending_bytes())
        for _, value in ipairs(expected) do
            assert.is_true(decoder:has_message())
            assert.is_true(resp.equal(value, decoder:get_message()))
        end
        assert.is_false(decoder:has_message())
        assert.has_error(function() decoder:get_message() end, "no decoded message is available")
    end)
end)

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local protocol = require("coding_adventures.in_memory_data_store_protocol")
local CommandFrame = protocol.CommandFrame
local EngineResponse = protocol.EngineResponse

describe("ASCII command normalization", function()
    it("uppercases ASCII bytes without changing punctuation", function()
        assert.equals("GET", protocol.ascii_upper("get"))
        assert.equals("MSET", protocol.ascii_upper("mSeT"))
        assert.equals("PING-2", protocol.ascii_upper("ping-2"))
    end)

    it("rejects non-string and non-ASCII input", function()
        assert.has_error(function() protocol.ascii_upper(1) end, "data must be a string")
        assert.has_error(function() protocol.ascii_upper("\255") end, "data must contain only ASCII bytes")
    end)
end)

describe("CommandFrame", function()
    it("builds normalized frames from byte-string parts", function()
        local frame = CommandFrame.from_parts({"set", "key", "value"})
        assert.equals("SET", frame.command)
        assert.are.same({"key", "value"}, frame.args)
        assert.are.same({"SET", "key", "value"}, frame:to_parts())
        assert.is_nil(CommandFrame.from_parts({}))
    end)

    it("defensively copies argument and part arrays", function()
        local args = {"key"}
        local frame = CommandFrame.new("GET", args)
        args[1] = "changed"
        assert.are.same({"key"}, frame.args)

        local parts = frame:to_parts()
        parts[2] = "changed"
        assert.are.same({"GET", "key"}, frame:to_parts())
    end)

    it("validates commands, args, and parts", function()
        assert.has_error(function() CommandFrame.new(1) end, "command must be a string")
        assert.has_error(function() CommandFrame.new("GET", "key") end, "args must be a table")
        assert.has_error(function() CommandFrame.new("GET", {1}) end, "args[1] must be a string")
        assert.has_error(function() CommandFrame.from_parts("GET") end, "parts must be a table")
        assert.has_error(function() CommandFrame.from_parts({"GET", 1}) end, "parts[2] must be a string")
    end)
end)

describe("EngineResponse", function()
    it("constructs scalar and convenience responses", function()
        assert.are.same({kind = "simple_string", value = "PONG"}, EngineResponse.simple_string("PONG"))
        assert.are.same({kind = "error", value = "ERR"}, EngineResponse.error("ERR"))
        assert.are.same({kind = "integer", value = 42}, EngineResponse.integer(42))
        assert.are.same({kind = "bulk_string", value = "value"}, EngineResponse.bulk_string("value"))
        assert.are.same({kind = "simple_string", value = "OK"}, EngineResponse.ok())
        assert.equals("bulk_string", EngineResponse.null().kind)
        assert.is_nil(EngineResponse.null().value)
        assert.are.same({kind = "integer", value = 0}, EngineResponse.zero())
        assert.are.same({kind = "integer", value = 1}, EngineResponse.one())
    end)

    it("constructs defensive response arrays and null arrays", function()
        local values = {EngineResponse.ok(), EngineResponse.integer(3)}
        local response = EngineResponse.array(values)
        values[1] = EngineResponse.error("changed")
        assert.equals("array", response.kind)
        assert.equals("simple_string", response.value[1].kind)
        assert.equals(3, response.value[2].value)
        assert.equals("array", EngineResponse.array(nil).kind)
        assert.is_nil(EngineResponse.array(nil).value)
    end)

    it("validates response kinds and payloads", function()
        assert.has_error(function() EngineResponse.new("unknown", nil) end, "invalid response kind: unknown")
        assert.has_error(function() EngineResponse.simple_string(1) end, "simple_string value must be a string")
        assert.has_error(function() EngineResponse.integer(1.5) end, "integer value must be an integer")
        assert.has_error(function() EngineResponse.bulk_string(1) end, "bulk_string value must be a string or nil")
        assert.has_error(function() EngineResponse.array({"not a response"}) end, "array response value[1] must be an EngineResponse")
    end)
end)

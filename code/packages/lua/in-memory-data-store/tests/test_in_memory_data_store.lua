local data_store = require("coding_adventures.in_memory_data_store")
local engine = require("coding_adventures.in_memory_data_store_engine")
local protocol = require("coding_adventures.in_memory_data_store_protocol")
local resp = require("coding_adventures.resp_protocol")

local Value = resp.Value

local function command(...)
    local values = {}
    for index, part in ipairs({ ... }) do
        values[index] = Value.bulk_string(part)
    end
    return Value.array(values)
end

describe("in-memory data store", function()
    it("executes RESP frames end to end", function()
        local store = data_store.new()
        local response = store:execute_frame(command("PING"))
        assert.equals("simple_string", response.kind)
        assert.equals("PONG", response.value)
    end)

    it("handles incremental and pipelined RESP input", function()
        local store = data_store.new()
        local set_wire = resp.encode(command("SET", "counter", "1"))
        local get_wire = resp.encode(command("GET", "counter"))

        assert.same({}, store:process(set_wire:sub(1, 5)))
        local output = store:handle(set_wire:sub(6) .. get_wire)
        assert.equals("+OK\r\n$1\r\n1\r\n", output)
    end)

    it("preserves binary-safe values", function()
        local store = data_store.new()
        local binary = "a\0b\255"
        assert.equals("OK", store:execute_frame(command("SET", "binary", binary)).value)
        assert.equals(binary, store:execute_frame(command("GET", "binary")).value)
    end)

    it("rejects invalid frames and ignores blank arrays", function()
        local store = data_store.new()
        assert.equals("error", store:execute_frame(Value.simple_string("PING")).kind)
        assert.is_nil(store:execute_frame(Value.array({})))
        assert.equals("error", store:execute_frame(Value.array({ Value.null_bulk_string() })).kind)
    end)

    it("converts command and response IR values", function()
        local frame = protocol.CommandFrame.new("ECHO", { "hello" })
        assert.equals("ECHO", data_store.command_from_resp(data_store.command_to_frame(frame)).command)

        local nested = protocol.EngineResponse.array({
            protocol.EngineResponse.integer(2),
            protocol.EngineResponse.bulk_string(nil),
        })
        local converted = data_store.response_to_resp_value(nested)
        assert.equals("integer", converted.value[1].kind)
        assert.is_true(converted.value[2].is_null)
        assert.equals("(nil)", data_store.frame_to_response_text(converted.value[2]))
    end)

    it("accepts injected engines and resets to a fresh store", function()
        local injected = engine.DataStoreEngine.new()
        local store = data_store.new({ engine = injected })
        store:execute_parts({ "SET", "name", "Ada" })
        assert.equals("Ada", store:execute_parts({ "GET", "name" }).value)
        store:reset()
        assert.is_true(store:execute_parts({ "GET", "name" }).is_null)
    end)
end)

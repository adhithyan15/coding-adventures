-- Tests for coding_adventures.json_rpc
-- =====================================
--
-- Busted test suite for the JSON-RPC 2.0 transport library.
--
-- Test coverage:
--   1. Module loads and exposes public API
--   2. JSON encoder: primitives, objects, arrays, null, escaping
--   3. JSON decoder: primitives, objects, arrays, null, unicode escapes
--   4. MessageWriter: Content-Length header format, payload, \r\n separator
--   5. MessageReader: single message, back-to-back messages, EOF, bad JSON,
--      valid JSON that is not a JSON-RPC message
--   6. Message constructors: Request, Response, ErrorResponse, Notification
--   7. classify_message: all four types + invalid cases
--   8. Server dispatch: request → response, notification → no response,
--      unknown method → -32601, handler error → -32603,
--      handler returning ResponseError → error response
--   9. Round-trip: encode → frame → parse → decode
--  10. Error constants have correct values

-- ---------------------------------------------------------------------------
-- Package path — must come before any require()
-- Per lessons.md: Lua test files MUST set package.path before require.
-- ---------------------------------------------------------------------------
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local jrpc = require("coding_adventures.json_rpc")

-- =========================================================================
-- Helper: build an in-memory readable stream from a string buffer.
--
-- The MessageReader calls stream:read("l") to read header lines, and
-- stream:read(n) to read the payload bytes.  We implement both modes.
-- =========================================================================

--- Create a mock readable stream backed by a string buffer.
-- @param data  string  The bytes the stream should return.
-- @return      table   Mock stream with :read() method.
local function make_reader_stream(data)
    local pos = 1
    return {
        read = function(self, mode)
            if pos > #data then return nil end  -- EOF
            if type(mode) == "number" then
                -- Read exactly `mode` bytes.
                local chunk = data:sub(pos, pos + mode - 1)
                if #chunk == 0 then return nil end
                pos = pos + #chunk
                return chunk
            elseif mode == "l" then
                -- Read one line (up to and including \n, returns without \n).
                local start = pos
                while pos <= #data and data:sub(pos, pos) ~= '\n' do
                    pos = pos + 1
                end
                if pos > #data and start == pos then return nil end
                local line = data:sub(start, pos - 1)
                if pos <= #data then pos = pos + 1 end  -- consume \n
                return line
            end
            return nil
        end
    }
end

--- Create a mock writable stream that accumulates written bytes.
-- @return  table  Mock stream with :write() and .buffer field.
local function make_writer_stream()
    local buf = {}
    return {
        write = function(self, s) buf[#buf + 1] = s end,
        flush = function(self) end,  -- no-op
        get   = function(self) return table.concat(buf) end,
    }
end

--- Frame a JSON string the same way the MessageWriter does.
-- Used to build test input for the MessageReader.
local function frame(json_str)
    return string.format("Content-Length: %d\r\n\r\n%s", #json_str, json_str)
end

-- =========================================================================
-- 1. Module surface
-- =========================================================================

describe("json_rpc module", function()
    it("loads successfully", function()
        assert.is_not_nil(jrpc)
    end)

    it("exposes VERSION string", function()
        assert.is_string(jrpc.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", jrpc.VERSION)
    end)

    it("exposes errors table", function()
        assert.is_table(jrpc.errors)
    end)

    it("exposes MessageReader, MessageWriter, Server", function()
        assert.is_table(jrpc.MessageReader)
        assert.is_table(jrpc.MessageWriter)
        assert.is_table(jrpc.Server)
    end)

    it("exposes message constructors", function()
        assert.is_function(jrpc.Request)
        assert.is_function(jrpc.Response)
        assert.is_function(jrpc.ErrorResponse)
        assert.is_function(jrpc.Notification)
    end)

    it("exposes null sentinel and is_null", function()
        assert.is_not_nil(jrpc.null)
        assert.is_function(jrpc.is_null)
        assert.is_true(jrpc.is_null(jrpc.null))
        assert.is_false(jrpc.is_null(nil))
        assert.is_false(jrpc.is_null(false))
    end)
end)

-- =========================================================================
-- 2. Error constants
-- =========================================================================

describe("error constants", function()
    it("PARSE_ERROR is -32700", function()
        assert.are.equal(-32700, jrpc.errors.PARSE_ERROR)
    end)

    it("INVALID_REQUEST is -32600", function()
        assert.are.equal(-32600, jrpc.errors.INVALID_REQUEST)
    end)

    it("METHOD_NOT_FOUND is -32601", function()
        assert.are.equal(-32601, jrpc.errors.METHOD_NOT_FOUND)
    end)

    it("INVALID_PARAMS is -32602", function()
        assert.are.equal(-32602, jrpc.errors.INVALID_PARAMS)
    end)

    it("INTERNAL_ERROR is -32603", function()
        assert.are.equal(-32603, jrpc.errors.INTERNAL_ERROR)
    end)
end)

-- =========================================================================
-- 3. JSON encoder / decoder (inline codec)
-- =========================================================================

describe("json_encode", function()
    it("encodes null sentinel", function()
        assert.are.equal("null", jrpc.json_encode(jrpc.null))
    end)

    it("encodes nil as null", function()
        assert.are.equal("null", jrpc.json_encode(nil))
    end)

    it("encodes booleans", function()
        assert.are.equal("true",  jrpc.json_encode(true))
        assert.are.equal("false", jrpc.json_encode(false))
    end)

    it("encodes integers without decimal point", function()
        assert.are.equal("42",  jrpc.json_encode(42))
        assert.are.equal("-7",  jrpc.json_encode(-7))
        assert.are.equal("0",   jrpc.json_encode(0))
    end)

    it("encodes a plain string", function()
        assert.are.equal('"hello"', jrpc.json_encode("hello"))
    end)

    it("encodes special characters in strings", function()
        assert.are.equal('"a\\nb"',   jrpc.json_encode("a\nb"))
        assert.are.equal('"a\\tb"',   jrpc.json_encode("a\tb"))
        assert.are.equal('"a\\"b"',   jrpc.json_encode('a"b'))
        assert.are.equal('"a\\\\b"',  jrpc.json_encode("a\\b"))
    end)

    it("encodes an array", function()
        assert.are.equal("[1,2,3]", jrpc.json_encode({1, 2, 3}))
    end)

    it("encodes an object (sorted keys)", function()
        assert.are.equal('{"a":1,"b":2}', jrpc.json_encode({b=2, a=1}))
    end)

    it("encodes nested object", function()
        assert.are.equal('{"x":{"y":1}}', jrpc.json_encode({x={y=1}}))
    end)
end)

describe("json_decode", function()
    it("decodes null", function()
        assert.is_true(jrpc.is_null(jrpc.json_decode("null")))
    end)

    it("decodes true and false", function()
        assert.is_true(jrpc.json_decode("true"))
        assert.is_false(jrpc.json_decode("false"))
    end)

    it("decodes integers", function()
        assert.are.equal(42, jrpc.json_decode("42"))
        assert.are.equal(-7, jrpc.json_decode("-7"))
    end)

    it("decodes a string", function()
        assert.are.equal("hello", jrpc.json_decode('"hello"'))
    end)

    it("decodes escape sequences", function()
        assert.are.equal("a\nb",  jrpc.json_decode('"a\\nb"'))
        assert.are.equal('a"b',   jrpc.json_decode('"a\\"b"'))
        assert.are.equal("a\\b",  jrpc.json_decode('"a\\\\b"'))
    end)

    it("decodes a JSON object", function()
        local v = jrpc.json_decode('{"id":1,"method":"ping"}')
        assert.are.equal(1,      v.id)
        assert.are.equal("ping", v.method)
    end)

    it("decodes a JSON array", function()
        local v = jrpc.json_decode("[1,2,3]")
        assert.are.equal(1, v[1])
        assert.are.equal(3, v[3])
    end)

    it("raises on malformed JSON", function()
        assert.has_error(function() jrpc.json_decode("{bad}") end)
    end)
end)

-- =========================================================================
-- 4. Message constructors
-- =========================================================================

describe("message constructors", function()
    it("Request has jsonrpc, id, method, params", function()
        local r = jrpc.Request(1, "ping", {x=1})
        assert.are.equal("2.0",  r.jsonrpc)
        assert.are.equal(1,      r.id)
        assert.are.equal("ping", r.method)
        assert.are.equal(1,      r.params.x)
    end)

    it("Request without params omits params key", function()
        local r = jrpc.Request(2, "initialize")
        assert.is_nil(r.params)
    end)

    it("Response has jsonrpc, id, result", function()
        local r = jrpc.Response(1, {ok=true})
        assert.are.equal("2.0", r.jsonrpc)
        assert.are.equal(1,     r.id)
        assert.is_true(r.result.ok)
    end)

    it("ErrorResponse has jsonrpc, id, error", function()
        local e = { code = -32601, message = "Method not found" }
        local r = jrpc.ErrorResponse(1, e)
        assert.are.equal("2.0",           r.jsonrpc)
        assert.are.equal(1,               r.id)
        assert.are.equal(-32601,          r.error.code)
        assert.are.equal("Method not found", r.error.message)
    end)

    it("Notification has jsonrpc, method, no id", function()
        local n = jrpc.Notification("textDocument/didOpen", {uri="file:///x"})
        assert.are.equal("2.0",                   n.jsonrpc)
        assert.are.equal("textDocument/didOpen",  n.method)
        assert.are.equal("file:///x",             n.params.uri)
        assert.is_nil(n.id)
    end)
end)

-- =========================================================================
-- 5. classify_message
-- =========================================================================

describe("classify_message", function()
    it("classifies a Request", function()
        local msg = {jsonrpc="2.0", id=1, method="ping"}
        assert.are.equal("request", jrpc.classify_message(msg))
    end)

    it("classifies a Notification", function()
        local msg = {jsonrpc="2.0", method="$/status"}
        assert.are.equal("notification", jrpc.classify_message(msg))
    end)

    it("classifies a success Response", function()
        local msg = {jsonrpc="2.0", id=1, result={}}
        assert.are.equal("response", jrpc.classify_message(msg))
    end)

    it("classifies an error Response", function()
        local msg = {jsonrpc="2.0", id=1, error={code=-32601, message="x"}}
        assert.are.equal("response", jrpc.classify_message(msg))
    end)

    it("returns nil for non-table", function()
        assert.is_nil(jrpc.classify_message("string"))
        assert.is_nil(jrpc.classify_message(nil))
    end)

    it("returns nil when jsonrpc field is missing or wrong", function()
        assert.is_nil(jrpc.classify_message({id=1, method="foo"}))
        assert.is_nil(jrpc.classify_message({jsonrpc="1.0", id=1, method="foo"}))
    end)
end)

-- =========================================================================
-- 6. MessageWriter
-- =========================================================================

describe("MessageWriter", function()
    it("write_raw produces correct Content-Length header", function()
        local ws = make_writer_stream()
        local writer = jrpc.MessageWriter:new(ws)
        local payload = '{"jsonrpc":"2.0","id":1,"result":null}'
        writer:write_raw(payload)
        local out = ws:get()
        -- Header must be "Content-Length: N\r\n\r\n"
        assert.is_truthy(out:match("^Content%-Length: %d+\r\n\r\n"))
        -- The number must equal the payload length
        local n = tonumber(out:match("Content%-Length: (%d+)"))
        assert.are.equal(#payload, n)
    end)

    it("write_raw includes the payload after the header", function()
        local ws = make_writer_stream()
        local writer = jrpc.MessageWriter:new(ws)
        local payload = '{"jsonrpc":"2.0"}'
        writer:write_raw(payload)
        local out = ws:get()
        -- Payload appears after the blank line
        assert.is_truthy(out:find(payload, 1, true))
    end)

    it("write_message encodes the message and frames it", function()
        local ws = make_writer_stream()
        local writer = jrpc.MessageWriter:new(ws)
        writer:write_message(jrpc.Response(1, {pong=true}))
        local out = ws:get()
        assert.is_truthy(out:match("Content%-Length:"))
        assert.is_truthy(out:find('"result"', 1, true))
    end)

    it("Content-Length matches UTF-8 byte length of payload", function()
        -- Use a payload with a multi-byte UTF-8 character.
        local ws = make_writer_stream()
        local writer = jrpc.MessageWriter:new(ws)
        -- "café" has a 2-byte é in UTF-8; total bytes = 6 not 5 chars.
        local payload = '{"k":"caf\xc3\xa9"}'   -- raw UTF-8
        writer:write_raw(payload)
        local out = ws:get()
        local n = tonumber(out:match("Content%-Length: (%d+)"))
        assert.are.equal(#payload, n)
    end)
end)

-- =========================================================================
-- 7. MessageReader
-- =========================================================================

describe("MessageReader", function()
    it("reads a single framed message", function()
        local json = '{"jsonrpc":"2.0","id":1,"method":"ping"}'
        local rs = make_reader_stream(frame(json))
        local reader = jrpc.MessageReader:new(rs)
        local msg, err = reader:read_message()
        assert.is_nil(err)
        assert.is_not_nil(msg)
        assert.are.equal(1,      msg.id)
        assert.are.equal("ping", msg.method)
    end)

    it("reads two back-to-back messages", function()
        local j1 = '{"jsonrpc":"2.0","id":1,"method":"a"}'
        local j2 = '{"jsonrpc":"2.0","method":"b"}'
        local rs = make_reader_stream(frame(j1) .. frame(j2))
        local reader = jrpc.MessageReader:new(rs)

        local m1, e1 = reader:read_message()
        assert.is_nil(e1)
        assert.are.equal("a", m1.method)

        local m2, e2 = reader:read_message()
        assert.is_nil(e2)
        assert.are.equal("b", m2.method)
    end)

    it("returns nil, nil on clean EOF", function()
        local rs = make_reader_stream("")
        local reader = jrpc.MessageReader:new(rs)
        local msg, err = reader:read_message()
        assert.is_nil(msg)
        assert.is_nil(err)
    end)

    it("returns nil, error on malformed JSON", function()
        local framed = "Content-Length: 5\r\n\r\n{bad}"
        local rs = make_reader_stream(framed)
        local reader = jrpc.MessageReader:new(rs)
        local msg, err = reader:read_message()
        assert.is_nil(msg)
        assert.is_not_nil(err)
        -- Error should mention parse error
        assert.is_truthy(err:find("parse error") or err:find("invalid"))
    end)

    it("returns nil, error for valid JSON that is not a JSON-RPC message", function()
        local json = '{"foo":"bar"}'   -- valid JSON, no jsonrpc field
        local framed = frame(json)
        local rs = make_reader_stream(framed)
        local reader = jrpc.MessageReader:new(rs)
        local msg, err = reader:read_message()
        assert.is_nil(msg)
        assert.is_not_nil(err)
        assert.is_truthy(err:find("invalid request") or err:find("JSON-RPC"))
    end)

    it("read_raw returns the raw JSON string", function()
        local json = '{"jsonrpc":"2.0","id":5,"method":"test"}'
        local rs = make_reader_stream(frame(json))
        local reader = jrpc.MessageReader:new(rs)
        local raw, err = reader:read_raw()
        assert.is_nil(err)
        assert.are.equal(json, raw)
    end)
end)

-- =========================================================================
-- 8. Server dispatch
-- =========================================================================

describe("Server dispatch", function()

    --- Run the server on a single input message and return the output buffer.
    local function run_server_once(input_json, setup_fn)
        local rs = make_reader_stream(frame(input_json))
        local ws = make_writer_stream()
        local server = jrpc.Server:new(rs, ws)
        if setup_fn then setup_fn(server) end
        server:serve()
        return ws:get()
    end

    it("dispatches a request to its handler and writes a response", function()
        local input = '{"jsonrpc":"2.0","id":1,"method":"greet","params":{"name":"World"}}'
        local out = run_server_once(input, function(srv)
            srv:on_request("greet", function(id, params)
                return { greeting = "Hello, " .. params.name }
            end)
        end)
        -- The output must be a framed message with "result"
        assert.is_truthy(out:find('"result"', 1, true))
        assert.is_truthy(out:find("Hello, World", 1, true))
    end)

    it("dispatches a notification to its handler without writing a response", function()
        local notif = '{"jsonrpc":"2.0","method":"didChange","params":{"x":1}}'
        local notif_called = false
        local out = run_server_once(notif, function(srv)
            srv:on_notification("didChange", function(params)
                notif_called = true
            end)
        end)
        assert.is_true(notif_called)
        -- No response should be written for a notification
        assert.are.equal("", out)
    end)

    it("sends -32601 for unknown request method", function()
        local input = '{"jsonrpc":"2.0","id":1,"method":"unknown"}'
        local out = run_server_once(input)
        assert.is_truthy(out:find("-32601", 1, true))
        assert.is_truthy(out:find('"error"', 1, true))
    end)

    it("sends -32603 when a request handler raises an error", function()
        local input = '{"jsonrpc":"2.0","id":2,"method":"boom"}'
        local out = run_server_once(input, function(srv)
            srv:on_request("boom", function(id, params)
                error("intentional test error")
            end)
        end)
        assert.is_truthy(out:find("-32603", 1, true))
    end)

    it("sends an error response when handler returns a ResponseError", function()
        local input = '{"jsonrpc":"2.0","id":3,"method":"validate","params":{}}'
        local out = run_server_once(input, function(srv)
            srv:on_request("validate", function(id, params)
                return { code = jrpc.errors.INVALID_PARAMS, message = "bad params" }
            end)
        end)
        assert.is_truthy(out:find("-32602", 1, true))
        assert.is_truthy(out:find('"error"', 1, true))
    end)

    it("ignores notifications for unregistered methods (no error response)", function()
        local notif = '{"jsonrpc":"2.0","method":"unregistered"}'
        local out = run_server_once(notif)
        -- No output at all for an unregistered notification
        assert.are.equal("", out)
    end)

    it("on_request is chainable", function()
        local rs = make_reader_stream("")
        local ws = make_writer_stream()
        local server = jrpc.Server:new(rs, ws)
        local chain = server
            :on_request("a", function() end)
            :on_request("b", function() end)
        assert.are.equal(server, chain)
    end)

    it("on_notification is chainable", function()
        local rs = make_reader_stream("")
        local ws = make_writer_stream()
        local server = jrpc.Server:new(rs, ws)
        local chain = server
            :on_notification("x", function() end)
            :on_notification("y", function() end)
        assert.are.equal(server, chain)
    end)
end)

-- =========================================================================
-- 9. Round-trip tests
-- =========================================================================

describe("round-trip", function()
    it("Request round-trips through encode/frame/parse/decode", function()
        local req = jrpc.Request(42, "textDocument/hover",
            {textDocument = {uri = "file:///main.bf"}, position = {line=0, character=3}})

        local ws = make_writer_stream()
        local writer = jrpc.MessageWriter:new(ws)
        writer:write_message(req)

        local rs = make_reader_stream(ws:get())
        local reader = jrpc.MessageReader:new(rs)
        local msg, err = reader:read_message()

        assert.is_nil(err)
        assert.are.equal(42,                    msg.id)
        assert.are.equal("textDocument/hover",  msg.method)
        assert.are.equal("file:///main.bf",     msg.params.textDocument.uri)
    end)

    it("Notification round-trips", function()
        local notif = jrpc.Notification("textDocument/didOpen",
            {textDocument = {uri = "file:///a.bf", text = "+[>+<-]."}})

        local ws = make_writer_stream()
        jrpc.MessageWriter:new(ws):write_message(notif)

        local rs = make_reader_stream(ws:get())
        local msg = jrpc.MessageReader:new(rs):read_message()

        assert.are.equal("textDocument/didOpen", msg.method)
        assert.are.equal("+[>+<-].",             msg.params.textDocument.text)
    end)

    it("Error response round-trips", function()
        local err_resp = jrpc.ErrorResponse(1, {
            code    = jrpc.errors.METHOD_NOT_FOUND,
            message = "Method not found",
            data    = "no handler for foo",
        })

        local ws = make_writer_stream()
        jrpc.MessageWriter:new(ws):write_message(err_resp)

        local rs = make_reader_stream(ws:get())
        local msg = jrpc.MessageReader:new(rs):read_message()

        assert.are.equal(1,                       msg.id)
        assert.are.equal(jrpc.errors.METHOD_NOT_FOUND, msg.error.code)
        assert.are.equal("Method not found",      msg.error.message)
    end)
end)

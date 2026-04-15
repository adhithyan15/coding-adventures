-- Tests for coding_adventures.rpc
-- =================================
--
-- Busted test suite for the abstract RPC primitive library.
--
-- Test coverage:
--   1.  Module loads and exposes public API
--   2.  Error code constants have correct values
--   3.  Message constructors produce correct tables
--   4.  MockCodec and MockFramer helpers work correctly
--   5.  Server: dispatches request → success response
--   6.  Server: dispatches request → handler error response
--   7.  Server: dispatches request → unknown method → -32601
--   8.  Server: handler throws Lua error → -32603
--   9.  Server: notification → handler called, no response sent
--   10. Server: unknown notification → silently dropped, no response
--   11. Server: notification handler throws → no crash, no response
--   12. Server: codec decode failure → error response with null id
--   13. Server: response/error messages → silently ignored
--   14. Server: serve() loop stops cleanly on EOF
--   15. Server: on_request and on_notification are chainable
--   16. Client: request() sends correct message, returns result
--   17. Client: request() returns error on server error response
--   18. Client: request() returns error on EOF before response
--   19. Client: notify() sends message without reading response
--   20. Client: on_notification() handler called for server-push during request()
--   21. Client: request ids auto-increment starting at 1
--   22. Client: on_notification is chainable
--   23. Multiple back-to-back requests
--   24. Server: serve() handles multiple messages before EOF

-- ---------------------------------------------------------------------------
-- Package path — must come before any require()
-- Per lessons.md: Lua test files MUST set package.path before require.
-- ---------------------------------------------------------------------------
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local rpc = require("coding_adventures.rpc")

-- =========================================================================
-- Mock codec and framer
-- =========================================================================
--
-- To test the RPC layer in isolation (without a real JSON codec or real I/O)
-- we need stub implementations of the RpcCodec and RpcFramer interfaces.
--
-- MockCodec
-- ---------
-- Uses a simple but unique encoding: a Lua table serialised with string.format
-- using only the fields the RPC layer cares about. The encoding is not JSON —
-- it is a custom format only used in these tests. The point is that the rpc
-- module never inspects the bytes — it only calls codec:encode(msg) and
-- codec:decode(bytes).
--
-- We implement MockCodec using Lua's built-in string serialisation (no deps).
-- The format chosen is a simple record with "|" separators:
--
--   "request|<id>|<method>|<params_as_string>"
--   "response|<id>|<result_as_string>"
--   "error|<id>|<code>|<message>|<data_or_nil>"
--   "notification|<method>|<params_as_string>"
--
-- We use a minimal value-to-string helper and a matching parser.
--
-- MockFramer
-- ----------
-- Pre-loads a list of byte strings to return in order from read_frame().
-- Accumulates all bytes passed to write_frame() in an output list.

-- -------------------------------------------------------------------------
-- Minimal value serialiser for MockCodec
-- -------------------------------------------------------------------------

--- Serialise a Lua value to a compact string for use in MockCodec encoding.
-- We only need to handle the types that appear in test parameters/results:
-- nil, booleans, numbers, strings. Nested tables are printed with tostring()
-- which is enough for round-trip equality in our tests (tables are by reference).
local function val_to_str(v)
    if v == nil then
        return "NIL"
    elseif type(v) == "boolean" then
        return v and "TRUE" or "FALSE"
    elseif type(v) == "number" then
        return "N:" .. tostring(v)
    elseif type(v) == "string" then
        -- Escape pipes so they don't break the field separator.
        return "S:" .. v:gsub("|", "\\|"):gsub("\n", "\\n")
    elseif type(v) == "table" then
        -- For simple {key=val} tables used in params, build a sorted repr.
        local parts = {}
        local keys = {}
        for k in pairs(v) do keys[#keys+1] = k end
        table.sort(keys, function(a,b) return tostring(a) < tostring(b) end)
        for _, k in ipairs(keys) do
            parts[#parts+1] = tostring(k) .. "=" .. val_to_str(v[k])
        end
        return "T:{" .. table.concat(parts, ",") .. "}"
    else
        return "O:" .. tostring(v)
    end
end

--- Parse a string produced by val_to_str back to the original value.
-- Returns the Lua value, or raises an error for unknown formats.
local function str_to_val(s)
    if s == "NIL" then
        return nil
    elseif s == "TRUE" then
        return true
    elseif s == "FALSE" then
        return false
    elseif s:sub(1, 2) == "N:" then
        return tonumber(s:sub(3))
    elseif s:sub(1, 2) == "S:" then
        return s:sub(3):gsub("\\|", "|"):gsub("\\n", "\n")
    elseif s:sub(1, 2) == "T:" then
        -- Parse "T:{key=val,key=val,...}"
        local inner = s:sub(4, #s - 1)   -- strip "T:{" and "}"
        local t = {}
        -- Simple comma-split (keys/vals have no commas in our tests)
        for pair in (inner .. ","):gmatch("([^,]+),") do
            local k, v_str = pair:match("^(.-)=(.+)$")
            if k then
                t[k] = str_to_val(v_str)
            end
        end
        return t
    else
        return s   -- fallback: return as string
    end
end

-- -------------------------------------------------------------------------
-- MockCodec
-- -------------------------------------------------------------------------

local MockCodec = {}
MockCodec.__index = MockCodec

--- Create a MockCodec.
-- If `fail_decode` is true, every decode() call returns a parse error.
-- @param fail_decode  boolean  (optional) Force all decodes to fail.
function MockCodec.new(fail_decode)
    return setmetatable({ fail_decode = fail_decode }, MockCodec)
end

--- Encode an RpcMessage table to a bytes string.
-- The format is deliberately simple — just enough for round-trip tests.
-- Real codecs (JsonCodec, etc.) replace this with their own encoding.
function MockCodec:encode(msg)
    local kind = msg.kind
    if kind == "request" then
        return table.concat({
            "request",
            val_to_str(msg.id),
            val_to_str(msg.method),
            val_to_str(msg.params),
        }, "|")
    elseif kind == "response" then
        return table.concat({
            "response",
            val_to_str(msg.id),
            val_to_str(msg.result),
        }, "|")
    elseif kind == "error" then
        return table.concat({
            "error",
            val_to_str(msg.id),
            val_to_str(msg.code),
            val_to_str(msg.message),
            val_to_str(msg.data),
        }, "|")
    elseif kind == "notification" then
        return table.concat({
            "notification",
            val_to_str(msg.method),
            val_to_str(msg.params),
        }, "|")
    else
        error("MockCodec: unknown message kind: " .. tostring(kind))
    end
end

--- Decode a bytes string back to an RpcMessage table.
-- Returns msg, nil on success or nil, err_table on failure.
function MockCodec:decode(bytes)
    if self.fail_decode then
        return nil, {
            id      = nil,
            code    = rpc.errors.PARSE_ERROR,
            message = "MockCodec: forced decode failure",
            data    = bytes,
        }
    end

    -- Split on unescaped "|". Our val_to_str escapes "|" inside strings,
    -- so a simple split on "|" is safe here.
    local parts = {}
    for part in (bytes .. "|"):gmatch("([^|]*)|") do
        parts[#parts+1] = part
    end

    local kind = parts[1]
    if kind == "request" then
        return {
            kind   = "request",
            id     = str_to_val(parts[2]),
            method = str_to_val(parts[3]),
            params = str_to_val(parts[4]),
        }, nil
    elseif kind == "response" then
        return {
            kind   = "response",
            id     = str_to_val(parts[2]),
            result = str_to_val(parts[3]),
        }, nil
    elseif kind == "error" then
        return {
            kind    = "error",
            id      = str_to_val(parts[2]),
            code    = str_to_val(parts[3]),
            message = str_to_val(parts[4]),
            data    = str_to_val(parts[5]),
        }, nil
    elseif kind == "notification" then
        return {
            kind   = "notification",
            method = str_to_val(parts[2]),
            params = str_to_val(parts[3]),
        }, nil
    else
        return nil, {
            id      = nil,
            code    = rpc.errors.INVALID_REQUEST,
            message = "MockCodec: unknown kind: " .. tostring(kind),
            data    = bytes,
        }
    end
end

-- -------------------------------------------------------------------------
-- MockFramer
-- -------------------------------------------------------------------------

local MockFramer = {}
MockFramer.__index = MockFramer

--- Create a MockFramer.
-- @param incoming  table  List of byte strings to return from read_frame(), in order.
--                         When the list is exhausted, read_frame() returns nil (EOF).
-- @return          MockFramer
function MockFramer.new(incoming)
    return setmetatable({
        -- Queue of frames to serve from read_frame().
        incoming = incoming or {},
        -- Index of the next frame to return.
        pos      = 1,
        -- All bytes passed to write_frame(), in order.
        outgoing = {},
    }, MockFramer)
end

--- Return the next pre-loaded frame, or nil on EOF.
function MockFramer:read_frame()
    if self.pos > #self.incoming then
        return nil   -- clean EOF
    end
    local frame = self.incoming[self.pos]
    self.pos = self.pos + 1
    return frame
end

--- Accumulate a frame in the outgoing list.
-- Always succeeds (returns true, nil).
function MockFramer:write_frame(bytes)
    self.outgoing[#self.outgoing + 1] = bytes
    return true, nil
end

--- Return the number of frames written so far.
function MockFramer:written_count()
    return #self.outgoing
end

--- Return the i-th written frame (1-indexed), decoded via the given codec.
-- Convenience helper for assertions.
function MockFramer:written_msg(i, codec)
    local bytes = self.outgoing[i]
    if not bytes then return nil end
    local msg, _ = codec:decode(bytes)
    return msg
end

-- =========================================================================
-- Helper: build a server with a codec+framer pair that has some pre-loaded
-- incoming frames and captures outgoing frames for assertion.
-- =========================================================================

--- Create a (server, codec, framer) triple ready for testing.
-- @param frames  table  List of byte strings the server will read.
local function make_server(frames, fail_decode)
    local codec  = MockCodec.new(fail_decode)
    local framer = MockFramer.new(frames)
    local server = rpc.Server.new(codec, framer)
    return server, codec, framer
end

--- Create a (client, codec, framer) triple ready for testing.
-- @param frames  table  List of byte strings the client will read.
local function make_client(frames)
    local codec  = MockCodec.new()
    local framer = MockFramer.new(frames)
    local client = rpc.Client.new(codec, framer)
    return client, codec, framer
end

--- Encode a message using a fresh MockCodec (for building test input frames).
local function encode_msg(msg)
    return MockCodec.new():encode(msg)
end

-- =========================================================================
-- 1. Module surface
-- =========================================================================

describe("rpc module", function()
    it("loads successfully", function()
        assert.is_not_nil(rpc)
        assert.is_table(rpc)
    end)

    it("exposes VERSION string matching semver", function()
        assert.is_string(rpc.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", rpc.VERSION)
    end)

    it("exposes errors table", function()
        assert.is_table(rpc.errors)
    end)

    it("exposes message constructors", function()
        assert.is_function(rpc.request_msg)
        assert.is_function(rpc.response_msg)
        assert.is_function(rpc.error_msg)
        assert.is_function(rpc.notification_msg)
    end)

    it("exposes Server class", function()
        assert.is_table(rpc.Server)
        assert.is_function(rpc.Server.new)
    end)

    it("exposes Client class", function()
        assert.is_table(rpc.Client)
        assert.is_function(rpc.Client.new)
    end)
end)

-- =========================================================================
-- 2. Error code constants
-- =========================================================================

describe("rpc error codes", function()
    it("PARSE_ERROR is -32700", function()
        assert.equals(-32700, rpc.errors.PARSE_ERROR)
    end)

    it("INVALID_REQUEST is -32600", function()
        assert.equals(-32600, rpc.errors.INVALID_REQUEST)
    end)

    it("METHOD_NOT_FOUND is -32601", function()
        assert.equals(-32601, rpc.errors.METHOD_NOT_FOUND)
    end)

    it("INVALID_PARAMS is -32602", function()
        assert.equals(-32602, rpc.errors.INVALID_PARAMS)
    end)

    it("INTERNAL_ERROR is -32603", function()
        assert.equals(-32603, rpc.errors.INTERNAL_ERROR)
    end)
end)

-- =========================================================================
-- 3. Message constructors
-- =========================================================================

describe("message constructors", function()
    it("request_msg produces correct table", function()
        local m = rpc.request_msg(1, "add", {a=1, b=2})
        assert.equals("request", m.kind)
        assert.equals(1, m.id)
        assert.equals("add", m.method)
        assert.same({a=1, b=2}, m.params)
    end)

    it("request_msg with nil params", function()
        local m = rpc.request_msg(42, "ping", nil)
        assert.equals("request", m.kind)
        assert.equals(42, m.id)
        assert.equals("ping", m.method)
        assert.is_nil(m.params)
    end)

    it("response_msg produces correct table", function()
        local m = rpc.response_msg(7, "hello")
        assert.equals("response", m.kind)
        assert.equals(7, m.id)
        assert.equals("hello", m.result)
    end)

    it("response_msg with nil result", function()
        local m = rpc.response_msg(3, nil)
        assert.equals("response", m.kind)
        assert.equals(3, m.id)
        assert.is_nil(m.result)
    end)

    it("error_msg produces correct table", function()
        local m = rpc.error_msg(5, -32601, "Method not found", "extra")
        assert.equals("error", m.kind)
        assert.equals(5, m.id)
        assert.equals(-32601, m.code)
        assert.equals("Method not found", m.message)
        assert.equals("extra", m.data)
    end)

    it("error_msg with nil id (decode-time error)", function()
        local m = rpc.error_msg(nil, -32700, "Parse error", nil)
        assert.equals("error", m.kind)
        assert.is_nil(m.id)
        assert.equals(-32700, m.code)
    end)

    it("notification_msg produces correct table", function()
        local m = rpc.notification_msg("log", "hello world")
        assert.equals("notification", m.kind)
        assert.equals("log", m.method)
        assert.equals("hello world", m.params)
    end)

    it("notification_msg with nil params", function()
        local m = rpc.notification_msg("ping", nil)
        assert.equals("notification", m.kind)
        assert.equals("ping", m.method)
        assert.is_nil(m.params)
    end)
end)

-- =========================================================================
-- 4. MockCodec and MockFramer helpers
-- =========================================================================

describe("mock helpers", function()
    it("MockCodec round-trips a request", function()
        local codec = MockCodec.new()
        local original = rpc.request_msg(1, "greet", "world")
        local bytes = codec:encode(original)
        local decoded, err = codec:decode(bytes)
        assert.is_nil(err)
        assert.equals("request", decoded.kind)
        assert.equals(1, decoded.id)
        assert.equals("greet", decoded.method)
        assert.equals("world", decoded.params)
    end)

    it("MockCodec round-trips a response", function()
        local codec = MockCodec.new()
        local original = rpc.response_msg(3, 42)
        local bytes = codec:encode(original)
        local decoded, err = codec:decode(bytes)
        assert.is_nil(err)
        assert.equals("response", decoded.kind)
        assert.equals(3, decoded.id)
        assert.equals(42, decoded.result)
    end)

    it("MockCodec round-trips an error message", function()
        local codec = MockCodec.new()
        local original = rpc.error_msg(2, -32601, "not found", nil)
        local bytes = codec:encode(original)
        local decoded, err = codec:decode(bytes)
        assert.is_nil(err)
        assert.equals("error", decoded.kind)
        assert.equals(2, decoded.id)
        assert.equals(-32601, decoded.code)
        assert.equals("not found", decoded.message)
    end)

    it("MockCodec round-trips a notification", function()
        local codec = MockCodec.new()
        local original = rpc.notification_msg("update", "data")
        local bytes = codec:encode(original)
        local decoded, err = codec:decode(bytes)
        assert.is_nil(err)
        assert.equals("notification", decoded.kind)
        assert.equals("update", decoded.method)
        assert.equals("data", decoded.params)
    end)

    it("MockCodec with fail_decode returns parse error", function()
        local codec = MockCodec.new(true)
        local msg, err = codec:decode("anything")
        assert.is_nil(msg)
        assert.is_not_nil(err)
        assert.equals(rpc.errors.PARSE_ERROR, err.code)
    end)

    it("MockFramer returns frames in order then nil", function()
        local frames = {"frame1", "frame2", "frame3"}
        local framer = MockFramer.new(frames)
        assert.equals("frame1", framer:read_frame())
        assert.equals("frame2", framer:read_frame())
        assert.equals("frame3", framer:read_frame())
        assert.is_nil(framer:read_frame())   -- EOF
        assert.is_nil(framer:read_frame())   -- still EOF
    end)

    it("MockFramer accumulates written frames", function()
        local framer = MockFramer.new({})
        framer:write_frame("aaa")
        framer:write_frame("bbb")
        assert.equals(2, framer:written_count())
        assert.equals("aaa", framer.outgoing[1])
        assert.equals("bbb", framer.outgoing[2])
    end)
end)

-- =========================================================================
-- 5. Server: dispatches request → success response
-- =========================================================================

describe("Server request dispatch", function()
    it("calls handler and sends success response", function()
        local req_frame = encode_msg(rpc.request_msg(1, "echo", "hello"))
        local server, codec, framer = make_server({req_frame})

        server:on_request("echo", function(id, params)
            return params, nil  -- echo the params back as the result
        end)
        server:serve()

        -- One response frame should have been written.
        assert.equals(1, framer:written_count())
        local resp = framer:written_msg(1, codec)
        assert.is_not_nil(resp)
        assert.equals("response", resp.kind)
        assert.equals(1, resp.id)
        assert.equals("hello", resp.result)
    end)

    it("handler receives correct id and params", function()
        local received_id, received_params
        local req_frame = encode_msg(rpc.request_msg(99, "capture", "my_param"))
        local server, codec, framer = make_server({req_frame})

        server:on_request("capture", function(id, params)
            received_id     = id
            received_params = params
            return "ok", nil
        end)
        server:serve()

        assert.equals(99, received_id)
        assert.equals("my_param", received_params)
    end)
end)

-- =========================================================================
-- 6. Server: handler returns error
-- =========================================================================

describe("Server request handler error", function()
    it("sends error response when handler returns err", function()
        local req_frame = encode_msg(rpc.request_msg(2, "fail", nil))
        local server, codec, framer = make_server({req_frame})

        server:on_request("fail", function(id, params)
            return nil, {
                code    = rpc.errors.INVALID_PARAMS,
                message = "bad params",
                data    = "details",
            }
        end)
        server:serve()

        assert.equals(1, framer:written_count())
        local resp = framer:written_msg(1, codec)
        assert.equals("error", resp.kind)
        assert.equals(2, resp.id)
        assert.equals(rpc.errors.INVALID_PARAMS, resp.code)
        assert.equals("bad params", resp.message)
        assert.equals("details", resp.data)
    end)
end)

-- =========================================================================
-- 7. Server: unknown method → -32601
-- =========================================================================

describe("Server method not found", function()
    it("sends -32601 for unregistered method", function()
        local req_frame = encode_msg(rpc.request_msg(3, "unknown_method", nil))
        local server, codec, framer = make_server({req_frame})
        -- No handlers registered.
        server:serve()

        assert.equals(1, framer:written_count())
        local resp = framer:written_msg(1, codec)
        assert.equals("error", resp.kind)
        assert.equals(3, resp.id)
        assert.equals(rpc.errors.METHOD_NOT_FOUND, resp.code)
    end)

    it("method not found message includes method name", function()
        local req_frame = encode_msg(rpc.request_msg(1, "missing", nil))
        local server, codec, framer = make_server({req_frame})
        server:serve()

        local resp = framer:written_msg(1, codec)
        assert.matches("missing", resp.message)
    end)
end)

-- =========================================================================
-- 8. Server: handler panics → -32603
-- =========================================================================

describe("Server handler panic recovery", function()
    it("pcall catches handler error and sends -32603", function()
        local req_frame = encode_msg(rpc.request_msg(4, "bomb", nil))
        local server, codec, framer = make_server({req_frame})

        server:on_request("bomb", function(id, params)
            error("explosion!")  -- Lua error — should be caught by pcall
        end)
        server:serve()

        assert.equals(1, framer:written_count())
        local resp = framer:written_msg(1, codec)
        assert.equals("error", resp.kind)
        assert.equals(4, resp.id)
        assert.equals(rpc.errors.INTERNAL_ERROR, resp.code)
        assert.equals("Internal error", resp.message)
        -- The error message should be in data.
        assert.is_not_nil(resp.data)
        assert.matches("explosion", resp.data)
    end)

    it("server loop continues after a panicking handler", function()
        local req1 = encode_msg(rpc.request_msg(1, "bomb", nil))
        local req2 = encode_msg(rpc.request_msg(2, "ok_method", nil))
        local server, codec, framer = make_server({req1, req2})

        server:on_request("bomb", function()
            error("boom")
        end)
        server:on_request("ok_method", function(id, params)
            return "survived", nil
        end)
        server:serve()

        -- Two responses: one error, one success.
        assert.equals(2, framer:written_count())
        local resp1 = framer:written_msg(1, codec)
        local resp2 = framer:written_msg(2, codec)
        assert.equals("error",    resp1.kind)
        assert.equals("response", resp2.kind)
        assert.equals("survived", resp2.result)
    end)
end)

-- =========================================================================
-- 9. Server: notification → handler called, no response
-- =========================================================================

describe("Server notification dispatch", function()
    it("calls notification handler and sends no response", function()
        local called_with
        local notif_frame = encode_msg(rpc.notification_msg("log", "hello"))
        local server, codec, framer = make_server({notif_frame})

        server:on_notification("log", function(params)
            called_with = params
        end)
        server:serve()

        -- Handler was called.
        assert.equals("hello", called_with)
        -- No response written for a notification.
        assert.equals(0, framer:written_count())
    end)
end)

-- =========================================================================
-- 10. Server: unknown notification → silently dropped
-- =========================================================================

describe("Server unknown notification", function()
    it("silently drops notification with no handler registered", function()
        local notif_frame = encode_msg(rpc.notification_msg("unknown_event", nil))
        local server, codec, framer = make_server({notif_frame})
        -- No notification handlers registered.
        server:serve()

        -- No response, no crash.
        assert.equals(0, framer:written_count())
    end)
end)

-- =========================================================================
-- 11. Server: notification handler throws → no crash, no response
-- =========================================================================

describe("Server notification handler error", function()
    it("swallows handler error and sends no response", function()
        local notif_frame = encode_msg(rpc.notification_msg("bad_notif", nil))
        local server, codec, framer = make_server({notif_frame})

        server:on_notification("bad_notif", function(params)
            error("notification handler crashed")
        end)
        -- Should not raise; should complete normally.
        assert.has_no.errors(function()
            server:serve()
        end)

        -- No response.
        assert.equals(0, framer:written_count())
    end)
end)

-- =========================================================================
-- 12. Server: codec decode failure → error response with nil id
-- =========================================================================

describe("Server codec decode failure", function()
    it("sends parse error response with nil id when codec fails", function()
        -- The framer returns one frame, but the codec is configured to fail.
        local framer_with_bad_frame = MockFramer.new({"garbage_bytes"})
        local fail_codec = MockCodec.new(true)   -- always fails to decode
        local server = rpc.Server.new(fail_codec, framer_with_bad_frame)
        server:serve()

        -- One error response should have been written.
        assert.equals(1, framer_with_bad_frame:written_count())
        -- Decode the written response using a working codec.
        local ok_codec = MockCodec.new()
        local resp, err = ok_codec:decode(framer_with_bad_frame.outgoing[1])
        assert.is_nil(err)
        assert.equals("error", resp.kind)
        assert.is_nil(resp.id)
        assert.equals(rpc.errors.PARSE_ERROR, resp.code)
    end)

    it("server loop continues after a decode error", function()
        -- First frame: undecipherable. Second frame: a valid request.
        local good_req = encode_msg(rpc.request_msg(1, "greet", nil))
        local framer = MockFramer.new({"garbage", good_req})

        -- We need a codec that fails on "garbage" but succeeds on the request.
        -- Use a custom codec for this case.
        local selective_codec = {
            fail_next = true,
            encode = function(self, msg)
                return MockCodec.new():encode(msg)
            end,
            decode = function(self, bytes)
                if bytes == "garbage" then
                    return nil, {
                        id      = nil,
                        code    = rpc.errors.PARSE_ERROR,
                        message = "Parse error",
                        data    = nil,
                    }
                end
                return MockCodec.new():decode(bytes)
            end,
        }
        local server = rpc.Server.new(selective_codec, framer)
        server:on_request("greet", function(id, params)
            return "hi", nil
        end)
        server:serve()

        -- Two write calls: one error, one success.
        assert.equals(2, framer:written_count())
        local ok_codec = MockCodec.new()
        local resp1 = framer:written_msg(1, ok_codec)
        local resp2 = framer:written_msg(2, ok_codec)
        assert.equals("error",    resp1.kind)
        assert.equals("response", resp2.kind)
        assert.equals("hi",       resp2.result)
    end)
end)

-- =========================================================================
-- 13. Server: response/error messages → silently ignored
-- =========================================================================

describe("Server ignores incoming responses", function()
    it("silently drops a response message (server never requests)", function()
        local resp_frame = encode_msg(rpc.response_msg(1, "some_result"))
        local server, codec, framer = make_server({resp_frame})
        server:serve()
        -- No crash, no outgoing frames.
        assert.equals(0, framer:written_count())
    end)

    it("silently drops an incoming error message", function()
        local err_frame = encode_msg(rpc.error_msg(2, -32601, "not found", nil))
        local server, codec, framer = make_server({err_frame})
        server:serve()
        assert.equals(0, framer:written_count())
    end)
end)

-- =========================================================================
-- 14. Server: serve() stops cleanly on EOF
-- =========================================================================

describe("Server EOF handling", function()
    it("serve() returns cleanly when framer returns nil immediately", function()
        -- No incoming frames → immediate EOF.
        local server, _, framer = make_server({})
        assert.has_no.errors(function()
            server:serve()
        end)
        assert.equals(0, framer:written_count())
    end)

    it("serve() stops after processing all frames", function()
        local req = encode_msg(rpc.request_msg(1, "ping", nil))
        local server, codec, framer = make_server({req})
        server:on_request("ping", function(id, _)
            return "pong", nil
        end)
        server:serve()
        -- Exactly one response, then serve() returned.
        assert.equals(1, framer:written_count())
    end)
end)

-- =========================================================================
-- 15. Server: method chaining
-- =========================================================================

describe("Server method chaining", function()
    it("on_request returns self for chaining", function()
        local server, _, framer = make_server({})
        local returned = server:on_request("a", function() end)
        assert.equals(server, returned)
    end)

    it("on_notification returns self for chaining", function()
        local server, _, framer = make_server({})
        local returned = server:on_notification("b", function() end)
        assert.equals(server, returned)
    end)

    it("chained registration works correctly", function()
        local req1 = encode_msg(rpc.request_msg(1, "add", nil))
        local req2 = encode_msg(rpc.request_msg(2, "sub", nil))
        local server, codec, framer = make_server({req1, req2})

        server
            :on_request("add", function(id, _) return "added", nil end)
            :on_request("sub", function(id, _) return "subbed", nil end)
        server:serve()

        assert.equals(2, framer:written_count())
        assert.equals("added",  framer:written_msg(1, codec).result)
        assert.equals("subbed", framer:written_msg(2, codec).result)
    end)
end)

-- =========================================================================
-- 16. Client: request() sends correct message, returns result
-- =========================================================================

describe("Client request", function()
    it("sends a request frame and returns the decoded result", function()
        -- Pre-load the response the server would send.
        local response_frame = encode_msg(rpc.response_msg(1, "pong"))
        local client, codec, framer = make_client({response_frame})

        local result, err = client:request("ping", nil)

        assert.is_nil(err)
        assert.equals("pong", result)

        -- Verify the outgoing request frame.
        assert.equals(1, framer:written_count())
        local sent = framer:written_msg(1, codec)
        assert.equals("request", sent.kind)
        assert.equals(1, sent.id)
        assert.equals("ping", sent.method)
    end)

    it("passes params to the server correctly", function()
        local response_frame = encode_msg(rpc.response_msg(1, "done"))
        local client, codec, framer = make_client({response_frame})

        client:request("process", "my_data")

        local sent = framer:written_msg(1, codec)
        assert.equals("my_data", sent.params)
    end)
end)

-- =========================================================================
-- 17. Client: request() returns error on server error response
-- =========================================================================

describe("Client request error", function()
    it("returns nil, err_table on server error response", function()
        local error_frame = encode_msg(rpc.error_msg(1, -32601, "not found", nil))
        local client, codec, framer = make_client({error_frame})

        local result, err = client:request("missing", nil)

        assert.is_nil(result)
        assert.is_not_nil(err)
        assert.equals(-32601, err.code)
        assert.equals("not found", err.message)
    end)
end)

-- =========================================================================
-- 18. Client: request() returns error on EOF before response
-- =========================================================================

describe("Client EOF before response", function()
    it("returns internal error when connection closes before response", function()
        -- No frames pre-loaded → immediate EOF.
        local client, _, _ = make_client({})

        local result, err = client:request("never_answered", nil)

        assert.is_nil(result)
        assert.is_not_nil(err)
        assert.equals(rpc.errors.INTERNAL_ERROR, err.code)
        assert.matches("closed", err.message)
    end)
end)

-- =========================================================================
-- 19. Client: notify() sends message without reading response
-- =========================================================================

describe("Client notify", function()
    it("sends a notification frame and does not read any response", function()
        -- If notify() tried to read, it would block on an empty framer.
        -- An empty framer (no incoming frames) is sufficient to test this.
        local client, codec, framer = make_client({})

        client:notify("log", "event happened")

        assert.equals(1, framer:written_count())
        local sent = framer:written_msg(1, codec)
        assert.equals("notification", sent.kind)
        assert.equals("log", sent.method)
        assert.equals("event happened", sent.params)
    end)

    it("notify sends no response and framer has no reads", function()
        -- Even if there are frames available, notify() should not consume them.
        local extra_frame = encode_msg(rpc.response_msg(999, "surprise"))
        local client, codec, framer = make_client({extra_frame})

        client:notify("event", nil)

        -- Still at position 1 (the extra_frame was not read).
        assert.equals(1, framer.pos)
    end)
end)

-- =========================================================================
-- 20. Client: on_notification() handler called for server-push
-- =========================================================================

describe("Client server-push notification during request", function()
    it("dispatches server-push notification while waiting for response", function()
        local push_data
        -- Two incoming frames: a server-push notification, then the real response.
        local push_frame     = encode_msg(rpc.notification_msg("progress", "50%"))
        local response_frame = encode_msg(rpc.response_msg(1, "final_result"))
        local client, codec, framer = make_client({push_frame, response_frame})

        client:on_notification("progress", function(params)
            push_data = params
        end)

        local result, err = client:request("long_operation", nil)

        assert.is_nil(err)
        assert.equals("final_result", result)
        assert.equals("50%", push_data)
    end)

    it("ignores server-push notification with no registered handler", function()
        local push_frame     = encode_msg(rpc.notification_msg("unknown_push", nil))
        local response_frame = encode_msg(rpc.response_msg(1, "ok"))
        local client, _, framer = make_client({push_frame, response_frame})

        -- No handler for "unknown_push" — should not crash; should still get response.
        local result, err
        assert.has_no.errors(function()
            result, err = client:request("anything", nil)
        end)
        -- The unknown notification is skipped; the real response is received.
        assert.is_nil(err)
        assert.equals("ok", result)
    end)
end)

-- =========================================================================
-- 21. Client: request ids auto-increment from 1
-- =========================================================================

describe("Client auto-increment ids", function()
    it("first request uses id 1", function()
        local response_frame = encode_msg(rpc.response_msg(1, "r1"))
        local client, codec, framer = make_client({response_frame})
        client:request("m1", nil)

        local sent = framer:written_msg(1, codec)
        assert.equals(1, sent.id)
    end)

    it("second request uses id 2", function()
        local r1 = encode_msg(rpc.response_msg(1, "r1"))
        local r2 = encode_msg(rpc.response_msg(2, "r2"))
        local client, codec, framer = make_client({r1, r2})

        client:request("m1", nil)
        client:request("m2", nil)

        local sent1 = framer:written_msg(1, codec)
        local sent2 = framer:written_msg(2, codec)
        assert.equals(1, sent1.id)
        assert.equals(2, sent2.id)
    end)

    it("ids keep incrementing across multiple requests", function()
        local frames = {}
        for i = 1, 5 do
            frames[i] = encode_msg(rpc.response_msg(i, "r" .. i))
        end
        local client, codec, framer = make_client(frames)

        for i = 1, 5 do
            client:request("method", nil)
        end

        for i = 1, 5 do
            local sent = framer:written_msg(i, codec)
            assert.equals(i, sent.id)
        end
    end)
end)

-- =========================================================================
-- 22. Client: on_notification is chainable
-- =========================================================================

describe("Client method chaining", function()
    it("on_notification returns self for chaining", function()
        local client, _, _ = make_client({})
        local returned = client:on_notification("x", function() end)
        assert.equals(client, returned)
    end)

    it("chained on_notification works for multiple handlers", function()
        local calls = {}
        local push1 = encode_msg(rpc.notification_msg("a", "v1"))
        local push2 = encode_msg(rpc.notification_msg("b", "v2"))
        local resp  = encode_msg(rpc.response_msg(1, "done"))
        local client, _, framer = make_client({push1, push2, resp})

        client
            :on_notification("a", function(p) calls[#calls+1] = "a:" .. p end)
            :on_notification("b", function(p) calls[#calls+1] = "b:" .. p end)

        client:request("do_it", nil)

        assert.equals(2, #calls)
        assert.equals("a:v1", calls[1])
        assert.equals("b:v2", calls[2])
    end)
end)

-- =========================================================================
-- 23. Multiple back-to-back requests (integration)
-- =========================================================================

describe("integration: multiple requests and notifications", function()
    it("handles mixed request/notification sequence", function()
        local req1   = encode_msg(rpc.request_msg(1, "greet", "Alice"))
        local notif  = encode_msg(rpc.notification_msg("log", "processed"))
        local req2   = encode_msg(rpc.request_msg(2, "greet", "Bob"))

        local server, codec, framer = make_server({req1, notif, req2})
        local log_received

        server
            :on_request("greet", function(id, name)
                return "Hello, " .. name, nil
            end)
            :on_notification("log", function(params)
                log_received = params
            end)
        server:serve()

        -- Two responses (for req1 and req2), no response for the notification.
        assert.equals(2, framer:written_count())
        assert.equals("Hello, Alice", framer:written_msg(1, codec).result)
        assert.equals("Hello, Bob",   framer:written_msg(2, codec).result)
        assert.equals("processed", log_received)
    end)
end)

-- =========================================================================
-- 24. Server: serve() handles multiple messages before EOF
-- =========================================================================

describe("Server multi-message serve loop", function()
    it("processes 10 requests in order", function()
        local frames = {}
        for i = 1, 10 do
            frames[i] = encode_msg(rpc.request_msg(i, "square", i))
        end

        local server, codec, framer = make_server(frames)
        server:on_request("square", function(id, n)
            return n * n, nil
        end)
        server:serve()

        assert.equals(10, framer:written_count())
        for i = 1, 10 do
            local resp = framer:written_msg(i, codec)
            assert.equals("response", resp.kind)
            assert.equals(i, resp.id)
            assert.equals(i * i, resp.result)
        end
    end)
end)

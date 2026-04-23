-- coding_adventures.rpc — Codec-agnostic RPC primitive
-- =====================================================
--
-- This module provides the abstract RPC layer that sits between an application
-- and the wire. It does NOT know about JSON, Content-Length headers, or any
-- particular serialisation format. Those concerns belong to codec and framer
-- implementations that are passed in at construction time.
--
-- # Architecture
--
-- Think of this layer like the rules of a phone call:
--   - You dial a number (request id)
--   - You say what you want (method + params)
--   - The other side answers (response with result or error)
--   - Or you leave a voicemail (notification — no reply expected)
--
-- The *language* you speak is the codec (JSON, MessagePack, Protobuf …).
-- The *phone network* is the framer + transport (Content-Length stdio, TCP, …).
-- This module defines the *rules of the conversation* — the same in any language.
--
-- # Layered design
--
--   ┌────────────────────────────────────────────────────────┐
--   │  Application                                           │
--   │  (handlers, business logic)                            │
--   ├────────────────────────────────────────────────────────┤
--   │  RpcServer / RpcClient  ← THIS MODULE                  │
--   │  (method dispatch, id correlation, error codes,        │
--   │   handler registry, panic recovery)                    │
--   ├────────────────────────────────────────────────────────┤
--   │  RpcCodec (pluggable)                                  │
--   │  RpcMessage  ↔  bytes                                  │
--   ├────────────────────────────────────────────────────────┤
--   │  RpcFramer (pluggable)                                 │
--   │  byte stream ↔ discrete byte chunks                    │
--   ├────────────────────────────────────────────────────────┤
--   │  Transport (stdin/stdout, TCP socket, …)               │
--   └────────────────────────────────────────────────────────┘
--
-- # Interface contracts (Lua duck-typing)
--
-- Lua has no formal interface keyword. We use documented contracts: any table
-- that provides the required methods satisfies the interface.
--
-- RpcCodec contract:
--   codec:encode(msg)         → bytes (string)
--   codec:decode(bytes)       → msg, err
--       On success: msg is a table {kind=...}, err is nil.
--       On failure: msg is nil, err is {id=nil, code=..., message=..., data=...}
--
-- RpcFramer contract:
--   framer:read_frame()       → bytes|nil   (nil = clean EOF)
--   framer:write_frame(bytes) → ok, err     (ok=true on success; ok=false, err=string on failure)
--
-- # Message kinds
--
-- All decoded messages are plain Lua tables with a string `kind` field:
--
--   { kind="request",      id=..., method=..., params=... }
--   { kind="response",     id=..., result=... }
--   { kind="error",        id=..., code=..., message=..., data=... }   (id may be nil)
--   { kind="notification", method=..., params=... }
--
-- # Error codes
--
-- These integer codes come from the JSON-RPC 2.0 spec §5.1 and are
-- codec-agnostic — the same values are used regardless of the wire format.
--
--   Code    | Name              | Meaning
--   --------|-------------------|--------------------------------------------
--   -32700  | PARSE_ERROR       | Codec could not decode the frame bytes
--   -32600  | INVALID_REQUEST   | Bytes decoded but not a valid RPC message
--   -32601  | METHOD_NOT_FOUND  | No handler registered for the method
--   -32602  | INVALID_PARAMS    | Handler rejected params as malformed
--   -32603  | INTERNAL_ERROR    | Unexpected error thrown by the handler

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Error codes
-- =========================================================================
--
-- Stored in a sub-table so callers can write rpc.errors.METHOD_NOT_FOUND
-- rather than sprinkling magic numbers through their code.

M.errors = {
    PARSE_ERROR      = -32700,
    INVALID_REQUEST  = -32600,
    METHOD_NOT_FOUND = -32601,
    INVALID_PARAMS   = -32602,
    INTERNAL_ERROR   = -32603,
}

-- =========================================================================
-- Message constructors
-- =========================================================================
--
-- These produce plain Lua tables with a `kind` discriminator field.
-- The codec is responsible for translating these tables to and from bytes.
-- The RPC layer only ever works with these plain-table representations.

--- Build a request message table.
-- A request expects a response from the server. The `id` is used to correlate
-- the eventual response back to this call.
-- @param id      string|number  Unique identifier for this request.
-- @param method  string         The procedure name to invoke.
-- @param params  any            Optional parameters (any Lua value).
-- @return        table          { kind="request", id=id, method=method, params=params }
function M.request_msg(id, method, params)
    return { kind = "request", id = id, method = method, params = params }
end

--- Build a response message table (success case).
-- Sent by the server in reply to a request that succeeded.
-- @param id      string|number  Must match the id from the originating request.
-- @param result  any            The return value of the handler.
-- @return        table          { kind="response", id=id, result=result }
function M.response_msg(id, result)
    return { kind = "response", id = id, result = result }
end

--- Build an error response message table.
-- Sent by the server when a request fails for any reason.
-- The `id` may be nil when the error occurred before the id could be extracted
-- (e.g., the frame bytes were completely malformed).
-- @param id       string|number|nil  The request id, or nil if unknown.
-- @param code     number             One of M.errors.* (or a server-defined code).
-- @param message  string             Human-readable description of the error.
-- @param data     any                Optional additional context (stack trace, etc.).
-- @return         table              { kind="error", id=id, code=code, ... }
function M.error_msg(id, code, message, data)
    return { kind = "error", id = id, code = code, message = message, data = data }
end

--- Build a notification message table.
-- Notifications are fire-and-forget: no response is expected or sent.
-- They have no `id` field because correlation is not needed.
-- @param method  string  The notification method name.
-- @param params  any     Optional parameters.
-- @return        table   { kind="notification", method=method, params=params }
function M.notification_msg(method, params)
    return { kind = "notification", method = method, params = params }
end

-- =========================================================================
-- RpcServer
-- =========================================================================
--
-- The server owns a codec and a framer. It maintains two dispatch tables —
-- one for request handlers and one for notification handlers — and drives a
-- blocking read-dispatch-write loop.
--
-- Handler contracts:
--   Request handler:      function(id, params) → result, err
--       result: any Lua value that becomes the `result` field of the response.
--       err:    nil (success) or a table {code, message, data?} (error response).
--               If `err` is not nil, result is ignored and an error response is sent.
--
--   Notification handler: function(params) → (return value ignored)
--       The server MUST NOT send any response to a notification, even on error.
--
-- Handler errors:
--   If the handler itself throws a Lua error (via `error()`), pcall() catches it
--   and the server sends a -32603 Internal error response with the error message
--   in the `data` field. This prevents one bad handler from killing the server.

local Server = {}
Server.__index = Server

--- Create a new RpcServer.
--
-- Example:
--   local server = rpc.Server.new(my_codec, my_framer)
--   server:on_request("add", function(id, params)
--     return params.a + params.b, nil
--   end)
--   server:serve()
--
-- @param codec   table  An RpcCodec (implements :encode(msg) and :decode(bytes)).
-- @param framer  table  An RpcFramer (implements :read_frame() and :write_frame(bytes)).
-- @return        Server
function Server.new(codec, framer)
    -- We use Server.new() rather than Server:new() so that the method table
    -- itself is not accidentally passed as `self`. This is a common Lua idiom
    -- for factory functions that return new instances.
    return setmetatable({
        codec         = codec,
        framer        = framer,
        -- request_handlers: maps method name (string) → function(id, params)
        request_handlers      = {},
        -- notification_handlers: maps method name (string) → function(params)
        notification_handlers = {},
    }, Server)
end

--- Register a handler for a named request method.
--
-- Calling on_request with the same method name twice replaces the first handler.
-- Returns `self` so calls can be chained:
--
--   server:on_request("ping", ping_handler)
--         :on_request("echo", echo_handler)
--
-- @param method   string    The RPC method name (e.g., "textDocument/hover").
-- @param handler  function  function(id, params) → result, err
-- @return         Server    self (enables method chaining)
function Server:on_request(method, handler)
    self.request_handlers[method] = handler
    return self   -- chainable
end

--- Register a handler for a named notification method.
--
-- Unknown notifications (no registered handler) are silently dropped per spec.
-- Returns `self` for chaining.
--
-- @param method   string    The notification method name.
-- @param handler  function  function(params) → (return value ignored)
-- @return         Server    self (enables method chaining)
function Server:on_notification(method, handler)
    self.notification_handlers[method] = handler
    return self   -- chainable
end

--- Send an encoded message using the framer.
-- Internal helper: encodes `msg` via the codec then writes via the framer.
-- @param msg  table  An RpcMessage table (produced by M.response_msg, etc.).
function Server:_send(msg)
    local bytes = self.codec:encode(msg)
    self.framer:write_frame(bytes)
end

--- Dispatch one decoded message and send any required response.
--
-- This is the heart of the server. For each message type:
--
--   request:
--     1. Look up the handler.
--     2. If not found: send METHOD_NOT_FOUND (-32601).
--     3. If found: pcall the handler.
--        a. pcall failed (Lua error): send INTERNAL_ERROR (-32603).
--        b. handler returned err != nil: send error response.
--        c. handler returned result, nil: send success response.
--
--   notification:
--     1. Look up the handler.
--     2. If found: pcall it (errors silently swallowed, no response).
--     3. If not found: silently drop. Never send any response.
--
--   response / error:
--     A pure server that never sends requests of its own will receive no
--     responses. Silently ignore (bidirectional extensions can override this).
--
-- @param msg  table  A decoded RpcMessage table with a `kind` field.
function Server:dispatch(msg)
    local kind = msg.kind

    if kind == "request" then
        local id     = msg.id
        local method = msg.method
        local params = msg.params

        local handler = self.request_handlers[method]
        if not handler then
            -- Spec §5.1: method not found → send -32601 error response.
            self:_send(M.error_msg(id,
                M.errors.METHOD_NOT_FOUND,
                "Method not found: " .. tostring(method)))
            return
        end

        -- Use pcall so a handler that throws does not kill the serve() loop.
        -- pcall returns: true + handler_returns  OR  false + error_message
        local ok, result, err = pcall(handler, id, params)

        if not ok then
            -- `result` here is actually the error message from the failed pcall.
            -- Send INTERNAL_ERROR with the message in `data` for debuggability.
            self:_send(M.error_msg(id,
                M.errors.INTERNAL_ERROR,
                "Internal error",
                tostring(result)))
            return
        end

        -- Handler ran successfully. Check whether it signalled an error.
        if err ~= nil then
            -- Convention: handler returns (nil_or_anything, err_table)
            -- where err_table has {code, message, data?}
            self:_send(M.error_msg(id,
                err.code    or M.errors.INTERNAL_ERROR,
                err.message or "Internal error",
                err.data))
        else
            -- Success: send the result back.
            self:_send(M.response_msg(id, result))
        end

    elseif kind == "notification" then
        -- Per spec: the server MUST NOT send any response to a notification,
        -- even if the handler errors. Notifications are fire-and-forget.
        local handler = self.notification_handlers[msg.method]
        if handler then
            -- pcall so errors don't crash the loop; return value is ignored.
            pcall(handler, msg.params)
        end
        -- Unknown notification method: silently drop (no response).

    else
        -- "response" or "error" kinds, or anything unknown.
        -- A pure server ignores incoming responses.
        -- Silently drop.
        _ = msg  -- avoid unused-variable lint warnings
    end
end

--- Run the blocking serve loop.
--
-- Reads frames one at a time from the framer. For each frame:
--   1. Asks the codec to decode it into an RpcMessage.
--   2. If decoding fails: sends an error response with null id and continues.
--   3. If decoding succeeds: dispatches the message (may send a response).
--   4. Continues until the framer returns nil (clean EOF).
--
-- The loop never raises an error to the caller — framing errors are reported
-- as RPC error responses, handler errors are caught by pcall in dispatch().
function Server:serve()
    while true do
        -- Read one frame from the transport.
        local bytes = self.framer:read_frame()

        -- nil = clean EOF (client disconnected). Shut down gracefully.
        if bytes == nil then
            break
        end

        -- Ask the codec to interpret the bytes as an RpcMessage.
        local msg, decode_err = self.codec:decode(bytes)

        if msg == nil then
            -- Codec couldn't decode the frame. decode_err is an error table
            -- with {id, code, message, data}. Send it as an error response.
            -- Use nil id if the codec couldn't extract one.
            local err_id   = (decode_err and decode_err.id) or nil
            local err_code = (decode_err and decode_err.code) or M.errors.PARSE_ERROR
            local err_msg  = (decode_err and decode_err.message) or "Parse error"
            local err_data = (decode_err and decode_err.data) or nil
            self:_send(M.error_msg(err_id, err_code, err_msg, err_data))
            -- Continue the loop — don't abort on a single bad message.
        else
            self:dispatch(msg)
        end
    end
end

-- Attach Server to the module.
M.Server = Server

-- =========================================================================
-- RpcClient
-- =========================================================================
--
-- The client sends requests and notifications to a remote server. It manages
-- request ids internally (auto-incrementing counter starting at 1) and
-- implements a blocking request/response cycle: after sending a request, it
-- reads frames until it sees a response with the matching id.
--
-- While waiting for a response the client may receive server-push notifications.
-- These are delivered to handlers registered via on_notification().
--
-- # Blocking model
--
-- The client here is intentionally simple and synchronous. It does not support
-- concurrent requests (only one request in flight at a time). For a concurrent
-- model you would need a background reader goroutine/thread and a pending-request
-- map — that is a future extension.
--
-- # Id correlation
--
-- Each request gets a unique integer id. The client keeps a `next_id` counter:
--
--   request 1 → id=1
--   request 2 → id=2
--   ...
--
-- The server echoes the id back in the response. The client checks that the
-- response id matches what it sent, ignoring any responses for other ids
-- (which can happen if a server-push notification contains a response-shaped
-- message from a buggy server).

local Client = {}
Client.__index = Client

--- Create a new RpcClient.
--
-- Example:
--   local client = rpc.Client.new(my_codec, my_framer)
--   local result, err = client:request("add", {a=1, b=2})
--   if err then print("error:", err.message)
--   else   print("result:", result) end
--
-- @param codec   table  An RpcCodec (implements :encode(msg) and :decode(bytes)).
-- @param framer  table  An RpcFramer (implements :read_frame() and :write_frame(bytes)).
-- @return        Client
function Client.new(codec, framer)
    return setmetatable({
        codec                  = codec,
        framer                 = framer,
        -- next_id: the id for the next outgoing request. Starts at 1.
        next_id                = 1,
        -- notification_handlers: maps method name → function(params)
        notification_handlers  = {},
    }, Client)
end

--- Register a handler for server-push notifications.
--
-- While the client is blocked waiting for a response to a `request()` call,
-- the server may send notifications (e.g., "textDocument/publishDiagnostics"
-- in the LSP protocol). This method registers a handler for those.
--
-- Returns `self` for chaining.
--
-- @param method   string    The notification method name.
-- @param handler  function  function(params) → (return value ignored)
-- @return         Client    self (enables method chaining)
function Client:on_notification(method, handler)
    self.notification_handlers[method] = handler
    return self
end

--- Send a request and wait for the matching response.
--
-- This is a blocking call. It:
--   1. Assigns the next auto-incremented id.
--   2. Encodes the request via the codec.
--   3. Sends the encoded bytes via the framer.
--   4. Reads frames in a loop until a response with the matching id arrives.
--      During the wait, any server-push notifications are dispatched to their handlers.
--   5. Returns (result, nil) on success or (nil, err_table) on error.
--
-- @param method  string  The procedure name to call.
-- @param params  any     Optional parameters (any Lua value, including nil).
-- @return        any, nil        On success: (result_value, nil)
-- @return        nil, table      On error:   (nil, {code, message, data?})
function Client:request(method, params)
    -- Claim the next request id and advance the counter.
    local id = self.next_id
    self.next_id = self.next_id + 1

    -- Encode and send the request.
    local req_bytes = self.codec:encode(M.request_msg(id, method, params))
    self.framer:write_frame(req_bytes)

    -- Wait for the matching response. The loop handles interleaved server
    -- notifications that arrive while we are waiting.
    while true do
        local bytes = self.framer:read_frame()

        if bytes == nil then
            -- EOF before we received a response. Report as an error.
            return nil, {
                id      = id,
                code    = M.errors.INTERNAL_ERROR,
                message = "Connection closed before response",
            }
        end

        local msg, decode_err = self.codec:decode(bytes)

        if msg == nil then
            -- The server sent something we couldn't decode. Report as error.
            -- In practice this is rare but we must handle it gracefully.
            return nil, {
                id      = id,
                code    = (decode_err and decode_err.code) or M.errors.PARSE_ERROR,
                message = (decode_err and decode_err.message) or "Parse error in response",
                data    = decode_err and decode_err.data,
            }
        end

        if msg.kind == "response" and msg.id == id then
            -- This is our response.
            return msg.result, nil

        elseif msg.kind == "error" and msg.id == id then
            -- The server sent an error response for our request.
            return nil, {
                id      = msg.id,
                code    = msg.code,
                message = msg.message,
                data    = msg.data,
            }

        elseif msg.kind == "notification" then
            -- Server-push notification while we wait. Dispatch to handler if any.
            local handler = self.notification_handlers[msg.method]
            if handler then
                pcall(handler, msg.params)
            end
            -- Continue the loop — we are still waiting for our response.

        else
            -- Response for a different id, or an unexpected message kind.
            -- Silently skip and keep waiting.
        end
    end
end

--- Send a notification (fire-and-forget).
--
-- Unlike request(), this does not wait for any response. Notifications are
-- used for one-way events where the caller does not need confirmation.
--
-- @param method  string  The notification method name.
-- @param params  any     Optional parameters (any Lua value, including nil).
function Client:notify(method, params)
    local bytes = self.codec:encode(M.notification_msg(method, params))
    self.framer:write_frame(bytes)
    -- No read: notifications get no response by definition.
end

-- Attach Client to the module.
M.Client = Client

-- =========================================================================
-- Module exports
-- =========================================================================
--
-- The public API surface of this module:
--
--   rpc.VERSION              — "0.1.0"
--   rpc.errors               — table of error code constants
--   rpc.request_msg(...)     — message constructor
--   rpc.response_msg(...)    — message constructor
--   rpc.error_msg(...)       — message constructor
--   rpc.notification_msg(...)— message constructor
--   rpc.Server               — server class (Server.new, :on_request, :serve, ...)
--   rpc.Client               — client class (Client.new, :request, :notify, ...)

return M

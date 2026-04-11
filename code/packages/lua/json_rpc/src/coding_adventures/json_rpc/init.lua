-- coding_adventures.json_rpc — JSON-RPC 2.0 over stdin/stdout
-- ============================================================
--
-- JSON-RPC 2.0 is the wire protocol underneath the Language Server Protocol
-- (LSP).  This module implements the transport layer:
--
--   1. Content-Length framing — read/write messages with an HTTP-style header
--   2. JSON encoding/decoding — a minimal inline codec (no external deps)
--   3. Message discrimination — tell Requests from Notifications from Responses
--   4. Server dispatch loop — route incoming messages to registered handlers
--
-- # How Content-Length framing works
--
-- Raw TCP/stdin is a byte stream with no message boundaries.  JSON has no
-- self-delimiting structure at the byte level — you cannot tell where one JSON
-- object ends without parsing it.  The LSP/JSON-RPC solution is to prefix each
-- message with a small HTTP-like header:
--
--     Content-Length: 97\r\n
--     \r\n
--     {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
--
-- The receiver reads the header, extracts the byte count (97), then reads
-- exactly that many bytes.  No guessing, no scanning, no partial reads.
--
-- # Message discrimination
--
-- All four message types share the `"jsonrpc": "2.0"` field, so the
-- discriminant is:
--
--   has `id`  AND `method`  → Request
--   has `id`  AND `result`  → Response (success)
--   has `id`  AND `error`   → Response (error)
--   has `method`, no `id`   → Notification
--
-- # Why no external JSON library?
--
-- The spec requires **no** dependency on other coding-adventures packages.
-- Lua's standard library has no JSON module, so we ship a minimal inline
-- encoder/decoder that handles exactly the shapes JSON-RPC messages use:
-- objects, arrays, strings, numbers, booleans, and null.
--
-- For general-purpose JSON work, prefer coding-adventures-json-serializer.

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Error codes (JSON-RPC 2.0 spec § 5.1)
-- =========================================================================
--
-- These integer codes are reserved by the spec.  Every error response sent
-- by the server must carry one of these codes (or a server-defined code in
-- the range -32099 to -32000).
--
-- | Code    | Name              | Meaning                                   |
-- |---------|-------------------|-------------------------------------------|
-- | -32700  | Parse error       | Message body is not valid JSON            |
-- | -32600  | Invalid Request   | JSON is valid but not a Request object    |
-- | -32601  | Method not found  | Method not registered                     |
-- | -32602  | Invalid params    | Invalid method parameters                 |
-- | -32603  | Internal error    | Internal server error                     |

M.errors = {
    PARSE_ERROR      = -32700,
    INVALID_REQUEST  = -32600,
    METHOD_NOT_FOUND = -32601,
    INVALID_PARAMS   = -32602,
    INTERNAL_ERROR   = -32603,
}

-- =========================================================================
-- Minimal JSON encoder
-- =========================================================================
--
-- Only handles values that appear in JSON-RPC messages:
--   - objects (Lua non-sequence tables)
--   - arrays  (Lua sequence tables)
--   - strings
--   - numbers
--   - booleans
--   - null (represented as the special sentinel M.null)
--
-- Design choice: we use a table-accumulator pattern (t[#t+1] = ...) rather
-- than string concatenation to avoid O(n²) behaviour when building long strings.

-- The null sentinel.  Since Lua's nil cannot be stored in tables, we need a
-- distinct value that encodes to JSON `null`.  Any table with a specific
-- marker will do; we check for it with is_null().
M.null = setmetatable({}, { __tostring = function() return "null" end })

--- Return true when v is the JSON null sentinel.
-- @param v  any
-- @return   boolean
local function is_null(v)
    return v == M.null
end
M.is_null = is_null

--- Count all entries in a table — works for both arrays and objects.
-- Unlike #t, this counts every key regardless of type.
local function table_count(t)
    local n = 0
    for _ in pairs(t) do n = n + 1 end
    return n
end

--- Return true when t is a Lua sequence (keys 1..#t, no gaps, non-empty).
local function is_sequence(t)
    local n = #t
    if n == 0 then return false end
    return table_count(t) == n
end

--- Escape a Lua string for use inside a JSON string literal.
-- Handles the full set of characters that must be escaped in JSON:
--   " → \"       \ → \\
--   control characters U+0000–U+001F → \uXXXX (or short form)
local function encode_string(s)
    local escaped = s
        :gsub("\\", "\\\\")   -- backslash FIRST (otherwise we'd double-escape)
        :gsub('"',  '\\"')
        :gsub("\b", "\\b")
        :gsub("\f", "\\f")
        :gsub("\n", "\\n")
        :gsub("\r", "\\r")
        :gsub("\t", "\\t")
    -- Any remaining control characters (U+0000–U+001F)
    escaped = escaped:gsub("[\x00-\x1f]", function(c)
        return string.format("\\u%04x", c:byte(1))
    end)
    return '"' .. escaped .. '"'
end

--- Recursively encode a Lua value to a JSON string.
-- @param v    any   The value to encode.
-- @param buf  table Accumulator (array of string fragments).
local function encode_value(v, buf)
    local t = type(v)

    if v == nil or is_null(v) then
        buf[#buf + 1] = "null"

    elseif t == "boolean" then
        buf[#buf + 1] = v and "true" or "false"

    elseif t == "number" then
        -- NaN and Infinity are not valid JSON; map them to null.
        if v ~= v then
            buf[#buf + 1] = "null"
        elseif v == math.huge or v == -math.huge then
            buf[#buf + 1] = "null"
        elseif math.floor(v) == v and math.abs(v) < 2^53 then
            -- Integer — no decimal point, e.g. 42 not 42.0
            buf[#buf + 1] = string.format("%d", v)
        else
            buf[#buf + 1] = string.format("%.14g", v)
        end

    elseif t == "string" then
        buf[#buf + 1] = encode_string(v)

    elseif t == "table" then
        if is_sequence(v) then
            -- JSON array: [elem, elem, ...]
            buf[#buf + 1] = "["
            for i = 1, #v do
                if i > 1 then buf[#buf + 1] = "," end
                encode_value(v[i], buf)
            end
            buf[#buf + 1] = "]"
        else
            -- JSON object: {"key": value, ...}
            -- Collect and sort string keys for deterministic output.
            local keys = {}
            for k in pairs(v) do
                if type(k) == "string" then keys[#keys + 1] = k end
            end
            table.sort(keys)

            buf[#buf + 1] = "{"
            for i, k in ipairs(keys) do
                if i > 1 then buf[#buf + 1] = "," end
                buf[#buf + 1] = encode_string(k)
                buf[#buf + 1] = ":"
                encode_value(v[k], buf)
            end
            buf[#buf + 1] = "}"
        end

    else
        -- Functions, userdata, threads: not representable in JSON → null.
        buf[#buf + 1] = "null"
    end
end

--- Encode a Lua value to a compact JSON string.
-- @param v  any   Value to encode.
-- @return   string  Compact JSON string.
local function json_encode(v)
    local buf = {}
    encode_value(v, buf)
    return table.concat(buf)
end

-- =========================================================================
-- Minimal JSON decoder
-- =========================================================================
--
-- Implements a recursive-descent parser for the JSON grammar subset used
-- by JSON-RPC messages.  The parser is intentionally simple: it does not
-- attempt to recover from errors, and it does not support trailing commas
-- or comments.
--
-- Parsing state is kept in a single `ctx` table with two fields:
--   ctx.s   — the input string
--   ctx.pos — the current byte position (1-indexed)
--
-- Each parse_* function advances ctx.pos and returns the parsed value.
-- On error it calls `error()` with a descriptive message.

--- Skip whitespace characters (space, tab, \r, \n).
local function skip_ws(ctx)
    while ctx.pos <= #ctx.s do
        local c = ctx.s:sub(ctx.pos, ctx.pos)
        if c == ' ' or c == '\t' or c == '\r' or c == '\n' then
            ctx.pos = ctx.pos + 1
        else
            break
        end
    end
end

--- Peek at the current character without consuming it.
local function peek(ctx)
    return ctx.s:sub(ctx.pos, ctx.pos)
end

--- Consume and return the current character; error if `expected` is given
--- and the current character does not match.
local function consume(ctx, expected)
    local c = ctx.s:sub(ctx.pos, ctx.pos)
    if expected and c ~= expected then
        error(string.format(
            "JSON parse error at pos %d: expected %q, got %q",
            ctx.pos, expected, c))
    end
    ctx.pos = ctx.pos + 1
    return c
end

-- Forward declaration so parse_value can call parse_object/parse_array,
-- and parse_object/parse_array can call parse_value recursively.
local parse_value

--- Parse a JSON string literal.  Returns the unescaped Lua string.
-- Handles all escape sequences defined by JSON: \" \\ \/ \b \f \n \r \t \uXXXX.
local function parse_string(ctx)
    consume(ctx, '"')
    local chunks = {}
    while ctx.pos <= #ctx.s do
        local c = ctx.s:sub(ctx.pos, ctx.pos)
        if c == '"' then
            ctx.pos = ctx.pos + 1
            return table.concat(chunks)
        elseif c == '\\' then
            ctx.pos = ctx.pos + 1
            local esc = ctx.s:sub(ctx.pos, ctx.pos)
            ctx.pos = ctx.pos + 1
            if     esc == '"'  then chunks[#chunks+1] = '"'
            elseif esc == '\\' then chunks[#chunks+1] = '\\'
            elseif esc == '/'  then chunks[#chunks+1] = '/'
            elseif esc == 'b'  then chunks[#chunks+1] = '\b'
            elseif esc == 'f'  then chunks[#chunks+1] = '\f'
            elseif esc == 'n'  then chunks[#chunks+1] = '\n'
            elseif esc == 'r'  then chunks[#chunks+1] = '\r'
            elseif esc == 't'  then chunks[#chunks+1] = '\t'
            elseif esc == 'u'  then
                -- \uXXXX — decode the 4-hex-digit code point.
                -- We only handle the Basic Multilingual Plane here (code points < 0x10000).
                local hex = ctx.s:sub(ctx.pos, ctx.pos + 3)
                ctx.pos = ctx.pos + 4
                local cp = tonumber(hex, 16)
                if not cp then
                    error("JSON parse error: invalid \\u escape: " .. hex)
                end
                -- Encode code point to UTF-8.
                if cp < 0x80 then
                    chunks[#chunks+1] = string.char(cp)
                elseif cp < 0x800 then
                    chunks[#chunks+1] = string.char(
                        0xC0 + math.floor(cp / 64),
                        0x80 + (cp % 64))
                else
                    chunks[#chunks+1] = string.char(
                        0xE0 + math.floor(cp / 4096),
                        0x80 + math.floor((cp % 4096) / 64),
                        0x80 + (cp % 64))
                end
            else
                error("JSON parse error: unknown escape \\" .. esc)
            end
        else
            chunks[#chunks+1] = c
            ctx.pos = ctx.pos + 1
        end
    end
    error("JSON parse error: unterminated string")
end

--- Parse a JSON number literal (integer or float).
local function parse_number(ctx)
    -- Consume the full number token with a simple pattern match.
    local num_str = ctx.s:match("^-?%d+%.?%d*[eE]?[+-]?%d*", ctx.pos)
    if not num_str then
        error("JSON parse error at pos " .. ctx.pos .. ": invalid number")
    end
    ctx.pos = ctx.pos + #num_str
    return tonumber(num_str)
end

--- Parse a JSON array: [ value, value, ... ]
local function parse_array(ctx)
    consume(ctx, '[')
    skip_ws(ctx)
    local arr = {}
    if peek(ctx) == ']' then
        ctx.pos = ctx.pos + 1
        return arr
    end
    while true do
        skip_ws(ctx)
        arr[#arr + 1] = parse_value(ctx)
        skip_ws(ctx)
        local c = peek(ctx)
        if c == ']' then
            ctx.pos = ctx.pos + 1
            return arr
        elseif c == ',' then
            ctx.pos = ctx.pos + 1
        else
            error("JSON parse error at pos " .. ctx.pos
                .. ": expected ',' or ']', got " .. c)
        end
    end
end

--- Parse a JSON object: { "key": value, ... }
local function parse_object(ctx)
    consume(ctx, '{')
    skip_ws(ctx)
    local obj = {}
    if peek(ctx) == '}' then
        ctx.pos = ctx.pos + 1
        return obj
    end
    while true do
        skip_ws(ctx)
        if peek(ctx) ~= '"' then
            error("JSON parse error at pos " .. ctx.pos
                .. ": expected string key, got " .. peek(ctx))
        end
        local key = parse_string(ctx)
        skip_ws(ctx)
        consume(ctx, ':')
        skip_ws(ctx)
        local val = parse_value(ctx)
        obj[key] = val
        skip_ws(ctx)
        local c = peek(ctx)
        if c == '}' then
            ctx.pos = ctx.pos + 1
            return obj
        elseif c == ',' then
            ctx.pos = ctx.pos + 1
        else
            error("JSON parse error at pos " .. ctx.pos
                .. ": expected ',' or '}', got " .. c)
        end
    end
end

--- Parse any JSON value at the current position.
-- This is the entry point for recursive descent.
parse_value = function(ctx)
    skip_ws(ctx)
    local c = peek(ctx)
    if c == '"' then
        return parse_string(ctx)
    elseif c == '{' then
        return parse_object(ctx)
    elseif c == '[' then
        return parse_array(ctx)
    elseif c == 't' then
        -- true
        if ctx.s:sub(ctx.pos, ctx.pos + 3) == "true" then
            ctx.pos = ctx.pos + 4
            return true
        end
        error("JSON parse error at pos " .. ctx.pos .. ": invalid token")
    elseif c == 'f' then
        -- false
        if ctx.s:sub(ctx.pos, ctx.pos + 4) == "false" then
            ctx.pos = ctx.pos + 5
            return false
        end
        error("JSON parse error at pos " .. ctx.pos .. ": invalid token")
    elseif c == 'n' then
        -- null
        if ctx.s:sub(ctx.pos, ctx.pos + 3) == "null" then
            ctx.pos = ctx.pos + 4
            return M.null
        end
        error("JSON parse error at pos " .. ctx.pos .. ": invalid token")
    elseif c == '-' or (c >= '0' and c <= '9') then
        return parse_number(ctx)
    else
        error("JSON parse error at pos " .. ctx.pos
            .. ": unexpected character " .. c)
    end
end

--- Decode a JSON string to a Lua value.
-- Raises an error on malformed JSON.
-- @param s  string  UTF-8 encoded JSON.
-- @return   any     Lua value (table for objects/arrays, string, number, boolean, M.null).
local function json_decode(s)
    local ctx = { s = s, pos = 1 }
    local value = parse_value(ctx)
    -- Ensure there is nothing after the top-level value (except whitespace).
    skip_ws(ctx)
    if ctx.pos <= #ctx.s then
        error(string.format(
            "JSON parse error: trailing garbage at pos %d: %q",
            ctx.pos, ctx.s:sub(ctx.pos, ctx.pos + 10)))
    end
    return value
end

-- =========================================================================
-- Message constructors
-- =========================================================================
--
-- All JSON-RPC 2.0 messages carry `"jsonrpc": "2.0"`.  These constructors
-- produce plain Lua tables that can be passed directly to MessageWriter.
--
-- Example Request:
--   {
--     "jsonrpc": "2.0",
--     "id": 1,
--     "method": "textDocument/hover",
--     "params": { "textDocument": {"uri": "file:///main.bf"}, "position": {...} }
--   }
--
-- Example Notification (no id):
--   {
--     "jsonrpc": "2.0",
--     "method": "textDocument/didOpen",
--     "params": { "textDocument": {"uri": "...", "text": "++[>+<-]."} }
--   }

--- Build a Request message table.
-- @param id      string|number  Unique request identifier.
-- @param method  string         The RPC method name.
-- @param params  any            Optional parameters (table or nil).
-- @return        table          Message table ready for encoding.
function M.Request(id, method, params)
    local msg = { jsonrpc = "2.0", id = id, method = method }
    if params ~= nil then msg.params = params end
    return msg
end

--- Build a success Response message table.
-- @param id      string|number|nil  The id from the originating Request.
-- @param result  any                The result value.
-- @return        table
function M.Response(id, result)
    return { jsonrpc = "2.0", id = id, result = result }
end

--- Build an error Response message table.
-- @param id         string|number|nil  The id from the originating Request (or nil).
-- @param error_obj  table              { code, message, data? }
-- @return           table
function M.ErrorResponse(id, error_obj)
    return { jsonrpc = "2.0", id = id, error = error_obj }
end

--- Build a Notification message table.  Notifications have no `id`.
-- @param method  string  The notification method name.
-- @param params  any     Optional parameters.
-- @return        table
function M.Notification(method, params)
    local msg = { jsonrpc = "2.0", method = method }
    if params ~= nil then msg.params = params end
    return msg
end

-- =========================================================================
-- Message discrimination
-- =========================================================================
--
-- Given a decoded Lua table, determine which of the four JSON-RPC message
-- types it represents.  The discriminant:
--
--   has id AND method                 → "request"
--   has method AND no id              → "notification"
--   has id AND (result or error key)  → "response"
--
-- We use explicit key presence tests rather than truthiness checks because
-- `id` can legitimately be 0 (falsy in some languages; not in Lua, but
-- being explicit is clearer).

--- Return the message type string: "request", "notification", "response",
--- or nil if the table does not look like any valid JSON-RPC message.
-- @param msg  table  Decoded Lua table.
-- @return     string|nil
local function classify_message(msg)
    if type(msg) ~= "table" then return nil end
    if msg.jsonrpc ~= "2.0" then return nil end

    local has_id     = msg.id ~= nil        -- id present (including 0, "")
    local has_method = msg.method ~= nil
    local has_result = msg.result ~= nil
    -- error key may be present even when value is falsy; use rawget to be safe
    local has_error  = rawget(msg, "error") ~= nil

    if has_id and has_method then
        return "request"
    elseif has_method and not has_id then
        return "notification"
    elseif has_id and (has_result or has_error) then
        return "response"
    else
        return nil
    end
end

M.classify_message = classify_message

-- =========================================================================
-- MessageReader
-- =========================================================================
--
-- Reads one Content-Length-framed JSON-RPC message from a byte stream.
--
-- The stream must expose:
--   stream:read(n)    → string|nil  read exactly n bytes; nil = EOF
--   stream:read("l")  → string|nil  read one line (used to read header lines)
--
-- Any object (table) with a `read` method works — this makes testing easy:
-- you can pass an in-memory mock instead of io.stdin.
--
-- read_message() return convention:
--   message, nil   — success; message is the decoded Lua table
--   nil, nil       — clean EOF (no bytes read yet for this message)
--   nil, string    — error (malformed header, bad JSON, etc.)

M.MessageReader = {}
M.MessageReader.__index = M.MessageReader

--- Create a new MessageReader.
-- @param stream  table  Anything with a :read() method.
-- @return        MessageReader
function M.MessageReader:new(stream)
    return setmetatable({ stream = stream }, self)
end

--- Read the Content-Length header block from the stream.
-- Headers end at an empty \r\n line.  Returns the content length (number)
-- or nil, error_string.
local function read_content_length(stream)
    local content_length = nil

    while true do
        -- Read one header line.  The LSP spec says headers end with \r\n
        -- but we accept \n alone as well for robustness.
        local line = stream:read("l")
        if line == nil then
            -- EOF while reading headers
            if content_length == nil then
                return nil, nil   -- clean EOF (no message started)
            else
                return nil, "unexpected EOF in headers"
            end
        end

        -- Strip the trailing \r if present (read("l") strips \n but not \r)
        line = line:gsub("\r$", "")

        -- An empty line marks the end of the header block.
        if line == "" then
            if content_length then
                return content_length, nil
            else
                return nil, "header block ended without Content-Length"
            end
        end

        -- Parse the header field.  We only care about Content-Length.
        -- Format: "Header-Name: value"
        local name, value = line:match("^([^:]+):%s*(.+)$")
        if name and name:lower() == "content-length" then
            content_length = tonumber(value)
            if not content_length then
                return nil, "Content-Length value is not a number: " .. value
            end
        end
        -- Other headers (e.g., Content-Type) are silently ignored.
    end
end

--- Read exactly n bytes from the stream.
-- Returns the string, or nil + error on short read / EOF.
local function read_exact(stream, n)
    if n == 0 then return "", nil end
    local data = stream:read(n)
    if data == nil then
        return nil, "unexpected EOF reading payload"
    end
    if #data < n then
        return nil, string.format(
            "short read: expected %d bytes, got %d", n, #data)
    end
    return data, nil
end

--- Read the raw JSON payload as a string (without parsing).
-- Returns: raw_string, nil  OR  nil, error_string  OR  nil, nil (EOF).
function M.MessageReader:read_raw()
    local length, err = read_content_length(self.stream)
    if length == nil then
        return nil, err   -- nil, nil = EOF; nil, string = error
    end
    local payload, perr = read_exact(self.stream, length)
    if payload == nil then
        return nil, perr
    end
    return payload, nil
end

--- Read one framed message, decode the JSON, and return a typed Lua table.
-- Returns: message, nil  OR  nil, error_string  OR  nil, nil (EOF).
function M.MessageReader:read_message()
    local raw, err = self:read_raw()
    if raw == nil then
        return nil, err
    end

    -- Decode JSON.  Map JSON parse failures to the PARSE_ERROR code.
    local ok, decoded = pcall(json_decode, raw)
    if not ok then
        return nil, string.format(
            "parse error (%d): %s", M.errors.PARSE_ERROR, decoded)
    end

    -- Validate the decoded value is a JSON-RPC message.
    local msg_type = classify_message(decoded)
    if msg_type == nil then
        return nil, string.format(
            "invalid request (%d): not a JSON-RPC 2.0 message",
            M.errors.INVALID_REQUEST)
    end

    -- Annotate with the classified type for the caller's convenience.
    decoded._type = msg_type
    return decoded, nil
end

-- =========================================================================
-- MessageWriter
-- =========================================================================
--
-- Writes one Content-Length-framed JSON-RPC message to a byte stream.
--
-- The stream must expose:
--   stream:write(str)  write a string to the stream
--   stream:flush()     optional; called after each message if present
--
-- The framing format:
--   "Content-Length: <n>\r\n\r\n<payload>"
-- where <n> is the byte length of the UTF-8-encoded JSON payload.

M.MessageWriter = {}
M.MessageWriter.__index = M.MessageWriter

--- Create a new MessageWriter.
-- @param stream  table  Anything with a :write() method.
-- @return        MessageWriter
function M.MessageWriter:new(stream)
    return setmetatable({ stream = stream }, self)
end

--- Frame a JSON string and write it to the stream.
-- @param json_str  string  The JSON payload (already encoded).
function M.MessageWriter:write_raw(json_str)
    -- The header uses byte length, not character count.
    -- In Lua, #str gives the byte length, which is correct for UTF-8.
    local header = string.format(
        "Content-Length: %d\r\n\r\n", #json_str)
    self.stream:write(header)
    self.stream:write(json_str)
    -- Flush if the stream supports it (e.g., io.stdout in line-buffered mode).
    if self.stream.flush then
        self.stream:flush()
    end
end

--- Encode a message table to JSON and write it with Content-Length framing.
-- @param message  table  A message table (Request, Response, Notification, etc.).
function M.MessageWriter:write_message(message)
    -- Strip the internal _type annotation before encoding.
    local copy = {}
    for k, v in pairs(message) do
        if k ~= "_type" then copy[k] = v end
    end
    local json_str = json_encode(copy)
    self:write_raw(json_str)
end

-- =========================================================================
-- Server
-- =========================================================================
--
-- The Server combines a MessageReader and MessageWriter with a handler table.
-- It drives the read-dispatch-write loop.
--
-- Handler contract:
--   request handler:      function(id, params) → result | error_table
--   notification handler: function(params)     → ignored
--
-- If a request handler returns a table with `code` and `message` keys
-- (a ResponseError shape), the server sends it as an error response.
-- Otherwise, the return value is used as the result field.
--
-- Example:
--   local server = JsonRpc.Server:new(io.stdin, io.stdout)
--   server:on_request("initialize", function(id, params)
--     return { capabilities = {} }
--   end)
--   server:on_notification("textDocument/didChange", function(params)
--     -- process change
--   end)
--   server:serve()

M.Server = {}
M.Server.__index = M.Server

--- Create a new Server.
-- @param in_stream   table  Readable stream (MessageReader source).
-- @param out_stream  table  Writable stream (MessageWriter sink).
-- @return            Server
function M.Server:new(in_stream, out_stream)
    return setmetatable({
        reader   = M.MessageReader:new(in_stream),
        writer   = M.MessageWriter:new(out_stream),
        requests = {},       -- method → handler function
        notifications = {},  -- method → handler function
    }, self)
end

--- Register a request handler.  Returns the server for method chaining.
-- @param method   string    JSON-RPC method name.
-- @param handler  function  function(id, params) → result or error_table
-- @return         Server    self (for chaining)
function M.Server:on_request(method, handler)
    self.requests[method] = handler
    return self
end

--- Register a notification handler.  Returns the server for method chaining.
-- @param method   string    JSON-RPC method name.
-- @param handler  function  function(params) → nil
-- @return         Server    self (for chaining)
function M.Server:on_notification(method, handler)
    self.notifications[method] = handler
    return self
end

--- Return true when a table looks like a ResponseError (has code + message).
-- We use this to distinguish handler return values from plain result tables.
-- @param v  any
-- @return   boolean
local function is_response_error(v)
    return type(v) == "table"
        and type(v.code) == "number"
        and type(v.message) == "string"
end

--- Dispatch one decoded message and write any necessary response.
-- @param msg  table  Decoded message (has _type annotation).
function M.Server:dispatch(msg)
    local msg_type = msg._type

    if msg_type == "request" then
        local id     = msg.id
        local method = msg.method
        local params = msg.params

        local handler = self.requests[method]
        if not handler then
            -- Method not registered → -32601 Method not found
            local err_resp = M.ErrorResponse(id, {
                code    = M.errors.METHOD_NOT_FOUND,
                message = "Method not found: " .. tostring(method),
            })
            self.writer:write_message(err_resp)
            return
        end

        -- Call the handler, catching errors.
        local ok, result = pcall(handler, id, params)
        if not ok then
            -- Unhandled Lua error in handler → -32603 Internal error
            local err_resp = M.ErrorResponse(id, {
                code    = M.errors.INTERNAL_ERROR,
                message = "Internal error",
                data    = tostring(result),
            })
            self.writer:write_message(err_resp)
            return
        end

        -- If the handler returned a ResponseError shape, send it as an error.
        if is_response_error(result) then
            local err_resp = M.ErrorResponse(id, result)
            self.writer:write_message(err_resp)
        else
            -- Otherwise send the return value as the result.
            local resp = M.Response(id, result)
            self.writer:write_message(resp)
        end

    elseif msg_type == "notification" then
        -- Notifications: call handler if registered, silently ignore if not.
        -- Per spec: the server MUST NOT send a response to a Notification.
        local handler = self.notifications[msg.method]
        if handler then
            -- Ignore handler errors for notifications (spec says no response).
            pcall(handler, msg.params)
        end

    elseif msg_type == "response" then
        -- Response to a request we sent (client-side).  In a pure server
        -- implementation there are no pending requests, so we silently drop.
        -- A future bidirectional extension can handle these.
        _ = msg  -- suppress unused-variable lint

    end
end

--- Run the server loop, blocking until EOF or an unrecoverable error.
-- Reads messages one at a time, dispatches them, writes responses.
function M.Server:serve()
    while true do
        local msg, err = self.reader:read_message()

        if msg == nil and err == nil then
            -- Clean EOF: client closed stdin.  Shut down gracefully.
            break
        end

        if err then
            -- Message-level parse/framing error.  Send an error response
            -- with a null id (we cannot know the request id).
            -- Then continue reading — do not abort the loop.
            local err_resp
            if err:find("parse error") then
                err_resp = M.ErrorResponse(M.null, {
                    code    = M.errors.PARSE_ERROR,
                    message = "Parse error",
                    data    = err,
                })
            else
                err_resp = M.ErrorResponse(M.null, {
                    code    = M.errors.INVALID_REQUEST,
                    message = "Invalid Request",
                    data    = err,
                })
            end
            self.writer:write_message(err_resp)
            -- If the stream is broken, the next read will return EOF and
            -- we will exit cleanly on the next iteration.
            goto continue
        end

        self:dispatch(msg)

        ::continue::
    end
end

-- =========================================================================
-- Module exports
-- =========================================================================
--
-- We expose the public API through the top-level table M.
-- json_encode and json_decode are also exported for testing convenience.

M.json_encode = json_encode
M.json_decode = json_decode

return M

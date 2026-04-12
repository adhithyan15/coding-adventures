-- tcp-client -- TCP client with buffered I/O and configurable timeouts
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
--
-- A TCP client is the fundamental building block for any networked application.
-- It opens a connection to a remote machine, sends bytes, receives bytes, and
-- closes the connection. Every web browser, email client, chat program, and
-- database driver starts with a TCP client.
--
-- This package wraps luasocket's TCP primitives with ergonomic defaults:
-- timeouts, buffered reading, and clean error handling. It is
-- protocol-agnostic -- it knows nothing about HTTP, SMTP, or Redis. It just
-- moves bytes reliably between two machines.
--
-- Analogy: A telephone call.
--
--   Making a TCP connection is like making a phone call:
--
--   1. DIAL (DNS + connect)
--      Look up "Grandma" in contacts -> 555-0123   (DNS resolution)
--      Dial the number and wait for it to ring      (TCP three-way handshake)
--      If nobody picks up after 30 seconds, hang up (connect timeout)
--
--   2. TALK (read/write)
--      Say "Hello, Grandma!"                        (write_all)
--      Listen for her response                      (read_line)
--      If silence for 30s -> "Still there?"         (read timeout)
--
--   3. HANG UP (shutdown/close)
--      Say "Goodbye" and hang up                    (shutdown_write + close)
--
-- Where it fits:
--
--   url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
--                              |
--                         raw byte stream
--
-- Usage:
--   local tcp_client = require("coding_adventures.tcp_client")
--   local conn = tcp_client.connect("info.cern.ch", 80)
--   conn:write_all("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")
--   conn:flush()
--   local status_line = conn:read_line()
--   print(status_line)
--   conn:close()
--
-- ============================================================================

local socket = require("socket")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- TcpError -- structured error type
-- ============================================================================
-- Every error from this library is a TcpError table with a `type` field that
-- identifies what went wrong and a `message` field with human-readable details.
--
-- Error types and their meanings:
--
--   type                  | meaning
--   ----------------------|---------------------------------------------------
--   dns_resolution_failed | hostname could not be resolved to an IP address
--   connection_refused    | server is reachable but nothing listening on port
--   timeout               | operation took too long (connect, read, or write)
--   connection_reset      | remote side crashed or closed unexpectedly
--   broken_pipe           | tried to write after remote closed
--   unexpected_eof        | connection closed before expected data arrived
--   io_error              | catch-all for other OS-level errors
--
-- The `phase` field on timeout errors distinguishes where the timeout occurred:
-- "connect", "read", or "write".

M.TcpError = {}
M.TcpError.__index = M.TcpError

--- Create a new TcpError.
--
-- @param error_type  string  One of the error type constants above.
-- @param message     string  Human-readable description of what went wrong.
-- @param fields      table   Optional extra fields (host, addr, phase, etc.)
-- @return TcpError
function M.TcpError.new(error_type, message, fields)
    local self = setmetatable({}, M.TcpError)
    self.type = error_type
    self.message = message
    -- Merge any additional fields into the error object.
    -- For example, dns_resolution_failed errors include `host`,
    -- timeout errors include `phase` and `duration`.
    if fields then
        for k, v in pairs(fields) do
            self[k] = v
        end
    end
    return self
end

--- String representation of this error, for display and debugging.
function M.TcpError:__tostring()
    return string.format("TcpError(%s): %s", self.type, self.message)
end

-- ============================================================================
-- Helper: map a luasocket error string to a TcpError
-- ============================================================================
-- Luasocket returns nil + error_string on failure. This function maps those
-- error strings to our structured TcpError types.
--
-- Common luasocket error strings:
--   "timeout"            -> read/write timed out
--   "closed"             -> remote side closed the connection
--   "refused"            -> nothing listening on the port (connection refused)
--   "connection refused" -> same, on some platforms
--   "host not found"     -> DNS resolution failed

local function map_socket_error(err, phase)
    if not err then
        return M.TcpError.new("io_error", "unknown error")
    end

    -- Normalize to lowercase for matching
    local lower = string.lower(err)

    if lower == "timeout" then
        return M.TcpError.new("timeout", phase .. " timed out", {
            phase = phase,
        })
    elseif lower == "closed" then
        -- "closed" can mean different things depending on context:
        -- during read -> connection reset or clean close
        -- during write -> broken pipe
        if phase == "write" then
            return M.TcpError.new("broken_pipe",
                "broken pipe (remote closed)")
        else
            return M.TcpError.new("connection_reset",
                "connection reset by peer")
        end
    elseif string.find(lower, "refused") then
        return M.TcpError.new("connection_refused",
            "connection refused", { addr = "" })
    elseif string.find(lower, "host not found")
        or string.find(lower, "getaddrinfo")
        or string.find(lower, "no address")
        or string.find(lower, "name or service not known")
        or string.find(lower, "no such host") then
        return M.TcpError.new("dns_resolution_failed",
            "DNS resolution failed: " .. err, { host = "" })
    else
        return M.TcpError.new("io_error", err)
    end
end

-- ============================================================================
-- ConnectOptions -- configuration for establishing a connection
-- ============================================================================
-- All timeouts default to 30 seconds. The buffer size defaults to 8192 bytes
-- (8 KiB), which is a good balance between memory usage and system call
-- reduction.
--
-- Why separate timeouts?
--
--   connect_timeout (30s) -- how long to wait for the TCP handshake
--     If a server is down or firewalled, the OS might wait minutes.
--
--   read_timeout (30s) -- how long to wait for data after calling read
--     Without this, a stalled server hangs your program forever.
--
--   write_timeout (30s) -- how long to wait for the OS send buffer
--     Usually instant, but blocks if the remote side isn't reading.

M.ConnectOptions = {}
M.ConnectOptions.__index = M.ConnectOptions

--- Create a new ConnectOptions with the given overrides.
--
-- Any field not provided uses the default value. This mirrors the builder
-- pattern used in the Rust version.
--
-- @param opts  table  Optional table of overrides:
--   - connect_timeout: number (seconds), default 30
--   - read_timeout:    number (seconds) or nil for no timeout, default 30
--   - write_timeout:   number (seconds) or nil for no timeout, default 30
--   - buffer_size:     number (bytes), default 8192
-- @return ConnectOptions
--
-- Example:
--   local opts = tcp_client.ConnectOptions.new({ connect_timeout = 10 })
function M.ConnectOptions.new(opts)
    opts = opts or {}
    local self = setmetatable({}, M.ConnectOptions)

    -- connect_timeout: how long to wait for the TCP handshake to complete.
    -- Default: 30 seconds. This prevents hanging forever when a server is
    -- down or behind a firewall that silently drops SYN packets.
    self.connect_timeout = opts.connect_timeout or 30

    -- read_timeout: how long to wait for data after calling a read function.
    -- Default: 30 seconds. Without this, a stalled or slow server will cause
    -- your program to hang indefinitely.
    self.read_timeout = opts.read_timeout or 30

    -- write_timeout: how long to wait for the OS to accept outgoing data.
    -- Default: 30 seconds. Writes usually complete instantly (data goes to
    -- the OS send buffer), but can block if the buffer is full.
    self.write_timeout = opts.write_timeout or 30

    -- buffer_size: size of the internal read buffer in bytes.
    -- Default: 8192 (8 KiB). Larger buffers mean fewer system calls but
    -- more memory usage. 8 KiB is the standard compromise.
    self.buffer_size = opts.buffer_size or 8192

    return self
end

-- ============================================================================
-- TcpConnection -- buffered I/O over a TCP stream
-- ============================================================================
-- A TcpConnection wraps a luasocket TCP object and adds an internal read
-- buffer for efficient, line-oriented or chunk-oriented communication.
--
-- Why buffered I/O?
--
--   Without buffering:
--     read() returns arbitrary chunks: "HT", "TP/", "1.0 2", "00 OK\r\n"
--     100 read() calls = 100 syscalls (expensive!)
--
--   With an internal buffer (8 KiB):
--     First read() pulls up to 8 KiB from the OS into memory.
--     Subsequent read_line() / read_exact() calls serve data from the buffer.
--     100 lines might need only 1-2 syscalls.
--
-- Luasocket's receive("*l") handles line buffering internally, but for
-- read_until with arbitrary delimiters and read_exact we maintain our own
-- buffer to avoid byte-at-a-time system calls.

local TcpConnection = {}
TcpConnection.__index = TcpConnection

--- (Internal) Create a new TcpConnection wrapping a connected socket.
--
-- @param sock     userdata  A connected luasocket TCP object.
-- @param options  ConnectOptions  The options used to establish this connection.
-- @return TcpConnection
local function new_connection(sock, options)
    local self = setmetatable({}, TcpConnection)

    -- The underlying luasocket TCP object. All I/O ultimately goes through
    -- this object's :send() and :receive() methods.
    self._sock = sock

    -- Internal read buffer: a string that holds data we have received from
    -- the OS but not yet returned to the caller. This enables efficient
    -- read_until and read_exact without byte-at-a-time system calls.
    self._buffer = ""

    -- How many bytes to request from the OS at a time when refilling the
    -- internal buffer. Larger values mean fewer system calls.
    self._buffer_size = options.buffer_size

    -- Store timeouts so we can switch between read and write timeouts
    -- as needed. Luasocket only has a single timeout setting per socket,
    -- so we swap it before each read or write operation.
    self._read_timeout = options.read_timeout
    self._write_timeout = options.write_timeout

    return self
end

-- ============================================================================
-- Internal: fill the read buffer
-- ============================================================================
-- Attempts to read up to buffer_size bytes from the socket into the internal
-- buffer. Returns true if any data was added, false on EOF.
--
-- Luasocket's receive(n) returns partial data on timeout, which we capture
-- and add to the buffer. This is important: even if a timeout fires, we
-- keep whatever bytes arrived before the timeout.

local function fill_buffer(conn)
    -- Set the socket timeout to the read timeout before reading.
    conn._sock:settimeout(conn._read_timeout)

    local data, err, partial = conn._sock:receive(conn._buffer_size)
    if data then
        conn._buffer = conn._buffer .. data
        return true, nil
    elseif partial and #partial > 0 then
        -- Partial data arrived before timeout or close. Keep it.
        conn._buffer = conn._buffer .. partial
        if err == "timeout" then
            return true, nil
        end
        return true, err
    else
        -- No data at all.
        return false, err
    end
end

-- ============================================================================
-- read_line -- read until newline
-- ============================================================================
-- Reads bytes until a newline (\n) is found. Returns the line INCLUDING the
-- trailing \n (and \r\n if present). Returns an empty string at EOF (remote
-- closed cleanly).
--
-- This is the workhorse for line-oriented protocols like HTTP/1.0, SMTP,
-- and RESP (Redis protocol).
--
-- Algorithm:
--   1. Search the internal buffer for \n.
--   2. If found, extract and return everything up to and including \n.
--   3. If not found, read more data from the socket and repeat.
--   4. If EOF (socket closed), return whatever is in the buffer (may be
--      empty string if nothing left).
--
-- @return string  The line including \n, or "" at EOF.
-- @error  TcpError on timeout or connection error.

function TcpConnection:read_line()
    while true do
        -- Search for newline in the buffered data.
        local newline_pos = string.find(self._buffer, "\n", 1, true)
        if newline_pos then
            -- Found it! Extract the line (including \n) and leave the
            -- rest in the buffer for subsequent reads.
            local line = string.sub(self._buffer, 1, newline_pos)
            self._buffer = string.sub(self._buffer, newline_pos + 1)
            return line, nil
        end

        -- No newline yet. Try to read more data from the socket.
        local ok, err = fill_buffer(self)
        if not ok then
            if err == "closed" or err == nil then
                -- EOF: return whatever remains in the buffer.
                -- If the buffer is empty, this returns "" signaling EOF.
                local remaining = self._buffer
                self._buffer = ""
                return remaining, nil
            elseif err == "timeout" then
                return nil, map_socket_error(err, "read")
            else
                return nil, map_socket_error(err, "read")
            end
        end
        -- If fill_buffer returned true but added no data (shouldn't happen
        -- normally), the next iteration will check again.
    end
end

-- ============================================================================
-- read_exact -- read exactly n bytes
-- ============================================================================
-- Blocks until all n bytes have been received. Useful for protocols that
-- specify an exact content length (e.g., HTTP Content-Length header).
--
-- Algorithm:
--   1. If the buffer already has >= n bytes, extract and return them.
--   2. Otherwise, read more data from the socket and repeat.
--   3. If EOF before n bytes, return TcpError with type "unexpected_eof".
--
-- @param n  number  The exact number of bytes to read.
-- @return string  Exactly n bytes of data.
-- @error  TcpError on timeout, EOF, or connection error.

function TcpConnection:read_exact(n)
    -- Keep reading until we have enough bytes in the buffer.
    while #self._buffer < n do
        local ok, err = fill_buffer(self)
        if not ok then
            if err == "closed" or err == nil then
                -- EOF before we got enough bytes.
                local received = #self._buffer
                self._buffer = ""
                return nil, M.TcpError.new("unexpected_eof",
                    string.format(
                        "unexpected EOF: expected %d bytes, got %d",
                        n, received),
                    { expected = n, received = received })
            elseif err == "timeout" then
                return nil, map_socket_error(err, "read")
            else
                return nil, map_socket_error(err, "read")
            end
        end
    end

    -- We have enough data. Extract exactly n bytes.
    local data = string.sub(self._buffer, 1, n)
    self._buffer = string.sub(self._buffer, n + 1)
    return data, nil
end

-- ============================================================================
-- read_until -- read until a delimiter is found
-- ============================================================================
-- Reads bytes until the given delimiter string (or single byte) is found.
-- Returns all bytes up to AND including the delimiter. This is useful for
-- protocols with custom delimiters:
--   - RESP (Redis) uses \r\n
--   - null-terminated strings use \0
--   - HTTP chunk encoding uses \r\n
--
-- @param delimiter  string  The delimiter to search for (e.g., "\0", "\r\n").
-- @return string  All bytes up to and including the delimiter.
-- @error  TcpError on timeout, EOF, or connection error.

function TcpConnection:read_until(delimiter)
    -- Convert a number (byte value) to a single-character string.
    -- This lets callers pass either a string or a byte: read_until("\0")
    -- or read_until(0) both work.
    if type(delimiter) == "number" then
        delimiter = string.char(delimiter)
    end

    local delim_len = #delimiter

    while true do
        -- Search for the delimiter in the buffered data.
        local found_pos = string.find(self._buffer, delimiter, 1, true)
        if found_pos then
            -- Extract everything up to and including the delimiter.
            local end_pos = found_pos + delim_len - 1
            local data = string.sub(self._buffer, 1, end_pos)
            self._buffer = string.sub(self._buffer, end_pos + 1)
            return data, nil
        end

        -- Delimiter not found yet. Read more data.
        local ok, err = fill_buffer(self)
        if not ok then
            if err == "closed" or err == nil then
                -- EOF without finding delimiter. Return what we have.
                local remaining = self._buffer
                self._buffer = ""
                if #remaining > 0 then
                    return remaining, nil
                end
                return "", nil
            elseif err == "timeout" then
                return nil, map_socket_error(err, "read")
            else
                return nil, map_socket_error(err, "read")
            end
        end
    end
end

-- ============================================================================
-- write_all -- write all bytes to the connection
-- ============================================================================
-- Sends all bytes in the given string to the remote side. Luasocket's send()
-- may send partial data; this function loops until everything is sent.
--
-- Unlike the Rust version which uses BufWriter, luasocket sends data
-- immediately (there is no userspace write buffer). This means each write_all
-- call results in at least one system call. For protocols that build up
-- multi-line requests, consider concatenating the request first, then calling
-- write_all once.
--
-- @param data  string  The bytes to send.
-- @return true on success.
-- @error  TcpError on broken pipe, timeout, or connection error.

function TcpConnection:write_all(data)
    if #data == 0 then
        return true, nil
    end

    -- Set write timeout before sending.
    self._sock:settimeout(self._write_timeout)

    -- Luasocket's send() returns: bytes_sent, err, last_byte_sent
    -- On partial send, last_byte_sent tells us where to resume.
    local total_sent = 0
    while total_sent < #data do
        local sent, err, last_sent = self._sock:send(data, total_sent + 1)
        if sent then
            total_sent = sent
        elseif last_sent and last_sent > total_sent then
            total_sent = last_sent
        else
            return nil, map_socket_error(err, "write")
        end
    end

    return true, nil
end

-- ============================================================================
-- flush -- flush the write buffer
-- ============================================================================
-- Luasocket sends data immediately via send(), so there is no userspace
-- write buffer to flush. This method exists for API compatibility with the
-- Rust version, where BufWriter requires explicit flushing.
--
-- In the Rust version, you MUST call flush() after writing a complete
-- request. In Lua, it's a no-op but calling it is still good practice
-- for code that may be ported to other languages.
--
-- @return true (always succeeds).

function TcpConnection:flush()
    -- No-op: luasocket sends immediately.
    return true, nil
end

-- ============================================================================
-- shutdown_write -- half-close the connection
-- ============================================================================
-- Shuts down the write half of the connection. This signals to the remote
-- side that no more data will be sent. The read half remains open -- you
-- can still receive data.
--
-- This is called "half-close" because TCP connections are full-duplex
-- (bidirectional). Shutting down one direction is like saying "I'm done
-- talking, but I'm still listening."
--
-- Half-close is important for protocols like HTTP/1.0:
--   1. Client sends the complete request
--   2. Client calls shutdown_write() -> "I'm done sending"
--   3. Server reads the complete request (sees EOF on its end)
--   4. Server sends the response
--   5. Client reads the response
--   6. Connection fully closed
--
-- @return true on success.
-- @error  TcpError on connection error.

function TcpConnection:shutdown_write()
    local ok, err = self._sock:shutdown("send")
    if ok == 1 or ok == true or ok == nil then
        -- luasocket:shutdown returns 1 on success
        -- But on error it returns nil, err
        if err then
            return nil, map_socket_error(err, "write")
        end
        return true, nil
    end
    return true, nil
end

-- ============================================================================
-- peer_addr -- get the remote address
-- ============================================================================
-- Returns a string "ip:port" representing the remote side of this connection.
-- This is useful for logging and debugging.
--
-- @return string  The remote address in "ip:port" format.
-- @error  TcpError if the socket is not connected.

function TcpConnection:peer_addr()
    local ip, port = self._sock:getpeername()
    if not ip then
        return nil, M.TcpError.new("io_error",
            "failed to get peer address: " .. tostring(port))
    end
    return string.format("%s:%d", ip, port), nil
end

-- ============================================================================
-- local_addr -- get the local address
-- ============================================================================
-- Returns a string "ip:port" representing the local side of this connection.
-- The port is assigned by the OS when the connection is established.
--
-- @return string  The local address in "ip:port" format.
-- @error  TcpError if the socket is not connected.

function TcpConnection:local_addr()
    local ip, port = self._sock:getsockname()
    if not ip then
        return nil, M.TcpError.new("io_error",
            "failed to get local address: " .. tostring(port))
    end
    return string.format("%s:%d", ip, port), nil
end

-- ============================================================================
-- close -- close the connection
-- ============================================================================
-- Fully closes the TCP connection, releasing the socket. After calling close,
-- no further reads or writes are possible.
--
-- In Lua, sockets are also closed when garbage collected, but explicitly
-- closing is good practice to release resources promptly. A closed socket
-- sitting in a GC queue holds onto the OS file descriptor until the
-- garbage collector runs.
--
-- @return true (always succeeds, errors are silently ignored).

function TcpConnection:close()
    self._sock:close()
    return true
end

-- ============================================================================
-- connect -- establish a TCP connection
-- ============================================================================
-- Establishes a TCP connection to the given host and port.
--
-- Algorithm:
--
--   1. DNS resolution: The host string is resolved to an IP address by
--      luasocket's connect() function, which calls the OS resolver
--      (respects /etc/hosts, system DNS).
--
--   2. TCP handshake: A three-way handshake (SYN, SYN-ACK, ACK) establishes
--      the connection. The connect_timeout limits how long this can take.
--
--   3. Configure timeouts: Read and write timeouts are stored for later use.
--      Luasocket only supports one timeout at a time, so we swap between
--      read_timeout and write_timeout before each operation.
--
--   4. Return a TcpConnection wrapping the connected socket.
--
-- @param host     string          Hostname ("example.com") or IP ("127.0.0.1")
-- @param port     number          TCP port number (e.g., 80 for HTTP)
-- @param options  ConnectOptions  Optional connection configuration.
--                                 If nil, defaults are used (30s timeouts,
--                                 8 KiB buffer).
-- @return TcpConnection on success.
-- @error  TcpError on DNS failure, connection refused, timeout, etc.
--
-- Example:
--   local conn, err = tcp_client.connect("example.com", 80)
--   if not conn then
--     print("Failed to connect: " .. err.message)
--     return
--   end

function M.connect(host, port, options)
    -- Use default options if none provided.
    if not options then
        options = M.ConnectOptions.new()
    end

    -- Create a new TCP socket object.
    local sock, sock_err = socket.tcp()
    if not sock then
        return nil, M.TcpError.new("io_error",
            "failed to create socket: " .. tostring(sock_err))
    end

    -- Set the connect timeout. This limits how long the TCP handshake can
    -- take. Without this, connecting to a firewalled host could block for
    -- minutes (the OS default TCP timeout is often 75-120 seconds).
    sock:settimeout(options.connect_timeout)

    -- Attempt to connect. Luasocket's connect() handles DNS resolution
    -- internally -- if `host` is a hostname, it calls the OS resolver
    -- to translate it to an IP address before attempting the TCP handshake.
    local ok, err = sock:connect(host, port)
    if not ok then
        sock:close()

        -- Classify the error. Luasocket returns different error strings
        -- for different failure modes:
        local lower_err = string.lower(tostring(err))

        if string.find(lower_err, "refused") then
            return nil, M.TcpError.new("connection_refused",
                string.format("connection refused by %s:%d", host, port),
                { addr = string.format("%s:%d", host, port) })
        elseif lower_err == "timeout" then
            return nil, M.TcpError.new("timeout",
                string.format("connect timed out after %ds",
                    options.connect_timeout),
                { phase = "connect", duration = options.connect_timeout })
        elseif string.find(lower_err, "host not found")
            or string.find(lower_err, "getaddrinfo")
            or string.find(lower_err, "no address")
            or string.find(lower_err, "name or service not known")
            or string.find(lower_err, "no such host") then
            return nil, M.TcpError.new("dns_resolution_failed",
                string.format("DNS resolution failed for '%s': %s",
                    host, err),
                { host = host })
        else
            return nil, M.TcpError.new("io_error",
                string.format("failed to connect to %s:%d: %s",
                    host, port, err))
        end
    end

    -- Connection established! Wrap the socket in a TcpConnection with
    -- buffered reading and configured timeouts.
    return new_connection(sock, options), nil
end

return M

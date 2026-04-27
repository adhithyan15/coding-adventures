-- test_tcp_client.lua -- tests for coding_adventures.tcp_client
--
-- Run via:
--   cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
--
-- These tests use real TCP sockets on localhost. Each test that needs a server
-- binds to port 0 (OS-assigned) to avoid conflicts when tests run in parallel.
-- Servers run in coroutines alongside the test using luasocket's non-blocking
-- select() for coordination.
--
-- Test groups:
--   1. Unit tests (ConnectOptions, TcpError)
--   2. Echo server tests (connect, read_line, read_exact, read_until)
--   3. Timeout tests (connect timeout, read timeout)
--   4. Error tests (connection refused, DNS failure)
--   5. Half-close tests
--   6. Edge case tests
-- ============================================================================

local socket = require("socket")
local tcp_client = require("coding_adventures.tcp_client")

-- ============================================================================
-- Helper: create a TCP listener on an OS-assigned port
-- ============================================================================
-- Binds to 127.0.0.1:0, letting the OS pick an available port. Returns the
-- listener socket and the assigned port number.
--
-- We use 127.0.0.1 (loopback) instead of 0.0.0.0 to avoid firewall prompts
-- and ensure tests work even without network access.

local function create_listener()
    local server = assert(socket.bind("127.0.0.1", 0))
    local _, port = server:getsockname()
    server:settimeout(5) -- 5 second accept timeout for safety
    return server, port
end

-- ============================================================================
-- Helper: accept one client and echo everything back
-- ============================================================================
-- Accepts exactly one connection, reads all data, echoes it back, then closes.
-- This simulates a simple echo server for testing read/write round-trips.

local function accept_and_echo(server)
    local client = server:accept()
    if not client then return end
    client:settimeout(5)
    while true do
        local data, err, partial = client:receive(8192)
        local chunk = data or partial
        if chunk and #chunk > 0 then
            client:send(chunk)
        end
        if err == "closed" then break end
        if err == "timeout" and (not chunk or #chunk == 0) then break end
    end
    client:close()
end

-- ============================================================================
-- Helper: accept one client and send specific data, then close
-- ============================================================================

local function accept_and_send(server, data_to_send)
    local client = server:accept()
    if not client then return end
    client:settimeout(2)
    if data_to_send and #data_to_send > 0 then
        client:send(data_to_send)
    end
    -- Small delay so client can read before we close
    socket.sleep(0.05)
    client:close()
end

-- ============================================================================
-- Helper: accept one client, read a request, then send a response
-- ============================================================================

local function accept_read_then_respond(server, response)
    local client = server:accept()
    if not client then return end
    client:settimeout(5)
    -- Read some data (the "request")
    client:receive(4096)
    -- Send the response
    if response and #response > 0 then
        client:send(response)
    end
    socket.sleep(0.05)
    client:close()
end

-- ============================================================================
-- Helper: accept one client, hold connection open, never send
-- ============================================================================

local function accept_and_hold_silent(server, hold_time)
    local client = server:accept()
    if not client then return end
    -- Just sit here doing nothing for hold_time seconds
    socket.sleep(hold_time or 3)
    client:close()
end

-- ============================================================================
-- Group 1: Unit tests -- ConnectOptions and TcpError
-- ============================================================================

describe("ConnectOptions", function()
    it("has sensible defaults", function()
        local opts = tcp_client.ConnectOptions.new()
        assert.equals(30, opts.connect_timeout)
        assert.equals(30, opts.read_timeout)
        assert.equals(30, opts.write_timeout)
        assert.equals(8192, opts.buffer_size)
    end)

    it("accepts custom values", function()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 10,
            read_timeout = 60,
            write_timeout = 5,
            buffer_size = 4096,
        })
        assert.equals(10, opts.connect_timeout)
        assert.equals(60, opts.read_timeout)
        assert.equals(5, opts.write_timeout)
        assert.equals(4096, opts.buffer_size)
    end)

    it("allows partial overrides", function()
        local opts = tcp_client.ConnectOptions.new({ read_timeout = 120 })
        assert.equals(30, opts.connect_timeout) -- default
        assert.equals(120, opts.read_timeout)   -- overridden
        assert.equals(30, opts.write_timeout)   -- default
        assert.equals(8192, opts.buffer_size)   -- default
    end)
end)

describe("TcpError", function()
    it("creates an error with type and message", function()
        local err = tcp_client.TcpError.new("timeout", "read timed out")
        assert.equals("timeout", err.type)
        assert.equals("read timed out", err.message)
    end)

    it("includes extra fields", function()
        local err = tcp_client.TcpError.new("dns_resolution_failed",
            "no such host", { host = "bad.example" })
        assert.equals("dns_resolution_failed", err.type)
        assert.equals("bad.example", err.host)
    end)

    it("has a string representation", function()
        local err = tcp_client.TcpError.new("broken_pipe",
            "broken pipe (remote closed)")
        local str = tostring(err)
        assert.truthy(string.find(str, "broken_pipe"))
        assert.truthy(string.find(str, "broken pipe"))
    end)
end)

-- ============================================================================
-- Group 2: Echo server tests
-- ============================================================================

describe("echo server", function()
    it("connect and disconnect", function()
        local server, port = create_listener()

        -- Run echo server in a coroutine-like pattern:
        -- we launch the server accept in background via a short timeout
        -- and connect from the test thread.
        -- Since luasocket is blocking, we connect first (the OS buffers
        -- the SYN), then accept.
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn, err = tcp_client.connect("127.0.0.1", port, opts)
        assert.is_nil(err)
        assert.is_not_nil(conn)

        -- Accept on the server side to complete the handshake
        local client = server:accept()
        assert.is_not_nil(client)

        conn:close()
        client:close()
        server:close()
    end)

    it("write and read back", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Write from our connection, echo from server side
        conn:write_all("Hello, TCP!")
        conn:flush()

        -- Server reads and echoes
        local data = client:receive(11)
        assert.equals("Hello, TCP!", data)
        client:send(data)

        -- Client reads the echo
        local result, read_err = conn:read_exact(11)
        assert.is_nil(read_err)
        assert.equals("Hello, TCP!", result)

        conn:close()
        client:close()
        server:close()
    end)

    it("read_line from echo", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Send two lines through the echo server
        conn:write_all("Hello\r\nWorld\r\n")
        conn:flush()

        -- Server reads and echoes
        local raw = client:receive(14)
        client:send(raw)

        -- Client reads line by line
        local line1 = assert(conn:read_line())
        assert.equals("Hello\r\n", line1)

        local line2 = assert(conn:read_line())
        assert.equals("World\r\n", line2)

        conn:close()
        client:close()
        server:close()
    end)

    it("read_exact from echo", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Build 100 bytes of patterned data
        local parts = {}
        for i = 0, 99 do
            parts[#parts + 1] = string.char(i % 256)
        end
        local data = table.concat(parts)

        conn:write_all(data)
        conn:flush()

        -- Server echoes
        local raw = client:receive(100)
        client:send(raw)

        -- Client reads exactly 100 bytes
        local result = assert(conn:read_exact(100))
        assert.equals(100, #result)
        assert.equals(data, result)

        conn:close()
        client:close()
        server:close()
    end)

    it("read_until with null delimiter", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Send data with a null delimiter
        local payload = "key:value\0next"
        conn:write_all(payload)
        conn:flush()

        -- Server echoes
        local raw = client:receive(#payload)
        client:send(raw)

        -- Client reads until null byte
        local result = assert(conn:read_until("\0"))
        assert.equals("key:value\0", result)

        conn:close()
        client:close()
        server:close()
    end)

    it("read_until with numeric delimiter", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Send data; delimiter is byte value 0 (null)
        local payload = "hello\0world"
        conn:write_all(payload)
        conn:flush()

        local raw = client:receive(#payload)
        client:send(raw)

        -- Pass delimiter as a number (byte value)
        local result = assert(conn:read_until(0))
        assert.equals("hello\0", result)

        conn:close()
        client:close()
        server:close()
    end)

    it("large data transfer", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
            buffer_size = 4096,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(10)

        -- Build 32 KiB of patterned data
        local parts = {}
        for i = 1, 32768 do
            parts[#parts + 1] = string.char((i - 1) % 256)
        end
        local data = table.concat(parts)

        -- Send in the background-ish: write from conn, then echo from server
        conn:write_all(data)
        conn:flush()

        -- Server echoes in chunks (it may not get all 32K at once)
        local total_echoed = 0
        while total_echoed < #data do
            local chunk, err, partial = client:receive(8192)
            local got = chunk or partial
            if got and #got > 0 then
                client:send(got)
                total_echoed = total_echoed + #got
            end
            if err == "closed" then break end
        end

        -- Client reads all 32K back
        local result = assert(conn:read_exact(32768))
        assert.equals(32768, #result)
        assert.equals(data, result)

        conn:close()
        client:close()
        server:close()
    end)

    it("multiple exchanges", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Exchange 1: ping
        conn:write_all("ping\n")
        conn:flush()
        local s1 = client:receive("*l")
        client:send(s1 .. "\n")
        local line1 = assert(conn:read_line())
        assert.equals("ping\n", line1)

        -- Exchange 2: pong
        conn:write_all("pong\n")
        conn:flush()
        local s2 = client:receive("*l")
        client:send(s2 .. "\n")
        local line2 = assert(conn:read_line())
        assert.equals("pong\n", line2)

        conn:close()
        client:close()
        server:close()
    end)
end)

-- ============================================================================
-- Group 3: Timeout tests
-- ============================================================================

describe("timeouts", function()
    it("connect timeout on non-routable address", function()
        -- 10.255.255.1 is a non-routable address. The TCP SYN will be sent
        -- but no SYN-ACK will ever come back, causing a timeout.
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 1,
            read_timeout = 1,
            write_timeout = 1,
        })
        local conn, err = tcp_client.connect("10.255.255.1", 1, opts)
        assert.is_nil(conn)
        assert.is_not_nil(err)
        -- Could be timeout or io_error depending on the platform
        assert.truthy(
            err.type == "timeout" or err.type == "io_error",
            "expected timeout or io_error, got: " .. err.type
        )
    end)

    it("read timeout on silent server", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5,
            read_timeout = 0.5, -- very short timeout
            write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())

        -- Server accepts but never sends. Client should time out on read.
        local data, err = conn:read_line()
        assert.is_nil(data)
        assert.is_not_nil(err)
        assert.equals("timeout", err.type)

        conn:close()
        client:close()
        server:close()
    end)

    it("read_exact timeout on silent server", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5,
            read_timeout = 0.5,
            write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())

        local data, err = conn:read_exact(100)
        assert.is_nil(data)
        assert.is_not_nil(err)
        assert.equals("timeout", err.type)

        conn:close()
        client:close()
        server:close()
    end)
end)

-- ============================================================================
-- Group 4: Error tests
-- ============================================================================

describe("errors", function()
    it("connection refused on closed port", function()
        -- Bind a port, then close the listener immediately so nothing is
        -- listening when we try to connect.
        local server, port = create_listener()
        server:close()

        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn, err = tcp_client.connect("127.0.0.1", port, opts)
        assert.is_nil(conn)
        assert.is_not_nil(err)
        -- Could be connection_refused or io_error depending on platform
        assert.truthy(
            err.type == "connection_refused" or err.type == "io_error",
            "expected connection_refused or io_error, got: " .. err.type
        )
    end)

    it("DNS failure on non-existent hostname", function()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn, err = tcp_client.connect(
            "this.host.does.not.exist.example", 80, opts)
        assert.is_nil(conn)
        assert.is_not_nil(err)
        -- Some ISPs hijack NXDOMAIN, so we accept several error types
        assert.truthy(
            err.type == "dns_resolution_failed"
            or err.type == "connection_refused"
            or err.type == "timeout"
            or err.type == "io_error",
            "expected DNS-related error, got: " .. err.type
        )
    end)

    it("unexpected EOF when server sends less than requested", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(2)

        -- Server sends 50 bytes, then closes
        local parts = {}
        for i = 0, 49 do parts[#parts + 1] = string.char(i) end
        client:send(table.concat(parts))
        client:close()

        -- Small delay to let the FIN arrive
        socket.sleep(0.1)

        -- Client tries to read 100 bytes but only 50 are available
        local data, err = conn:read_exact(100)
        assert.is_nil(data)
        assert.is_not_nil(err)
        assert.equals("unexpected_eof", err.type)
        assert.equals(50, err.received)
        assert.equals(100, err.expected)

        conn:close()
        server:close()
    end)

    it("broken pipe when writing to closed connection", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())

        -- Server immediately closes
        client:close()
        socket.sleep(0.2)

        -- Client tries to write. May need multiple writes because the
        -- first might succeed (data goes to OS send buffer) before the
        -- RST arrives from the server.
        local got_error = false
        for _ = 1, 20 do
            local big_data = string.rep("x", 65536)
            local ok, err = conn:write_all(big_data)
            if not ok then
                got_error = true
                assert.truthy(
                    err.type == "broken_pipe"
                    or err.type == "connection_reset"
                    or err.type == "io_error",
                    "expected write error, got: " .. err.type
                )
                break
            end
            socket.sleep(0.02)
        end
        assert.is_true(got_error, "expected write error after server closed")

        conn:close()
        server:close()
    end)
end)

-- ============================================================================
-- Group 5: Half-close tests
-- ============================================================================

describe("half-close", function()
    it("client shutdown_write then read response", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Client sends data, then shuts down write
        conn:write_all("request data")
        conn:flush()
        conn:shutdown_write()

        -- Server reads until EOF (client shut down write -> server sees EOF)
        local received_parts = {}
        while true do
            local data, err, partial = client:receive(4096)
            local chunk = data or partial
            if chunk and #chunk > 0 then
                received_parts[#received_parts + 1] = chunk
            end
            if err == "closed" then break end
        end
        local received = table.concat(received_parts)
        assert.equals("request data", received)

        -- Server sends final response
        client:send("DONE\n")
        client:close()

        -- Client reads the response (read half is still open!)
        local response = assert(conn:read_line())
        assert.equals("DONE\n", response)

        conn:close()
        server:close()
    end)
end)

-- ============================================================================
-- Group 6: Edge cases
-- ============================================================================

describe("edge cases", function()
    it("empty read_line at EOF", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(2)

        -- Server sends one line then closes
        client:send("hello\n")
        client:close()
        socket.sleep(0.1)

        -- First read_line returns the line
        local line = assert(conn:read_line())
        assert.equals("hello\n", line)

        -- Second read_line returns empty string (EOF)
        local eof = conn:read_line()
        assert.equals("", eof)

        conn:close()
        server:close()
    end)

    it("zero byte write succeeds", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local _ = assert(server:accept())

        -- Writing zero bytes should succeed without error
        local ok, err = conn:write_all("")
        assert.is_true(ok)
        assert.is_nil(err)

        conn:close()
        server:close()
    end)

    it("peer_addr returns correct address", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local _ = assert(server:accept())

        local peer = assert(conn:peer_addr())
        assert.truthy(string.find(peer, "127.0.0.1"))
        assert.truthy(string.find(peer, tostring(port)))

        conn:close()
        server:close()
    end)

    it("local_addr returns valid address", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local _ = assert(server:accept())

        local addr = assert(conn:local_addr())
        assert.truthy(string.find(addr, "127.0.0.1"))
        -- Local port should be a positive number
        local local_port = tonumber(string.match(addr, ":(%d+)$"))
        assert.truthy(local_port and local_port > 0)

        conn:close()
        server:close()
    end)

    it("connect with default options (nil)", function()
        local server, port = create_listener()
        -- Pass nil for options -- should use defaults
        local conn, err = tcp_client.connect("127.0.0.1", port, nil)
        assert.is_nil(err)
        assert.is_not_nil(conn)
        local _ = assert(server:accept())

        conn:close()
        server:close()
    end)

    it("connect with no options argument", function()
        local server, port = create_listener()
        -- Omit options entirely
        local conn, err = tcp_client.connect("127.0.0.1", port)
        assert.is_nil(err)
        assert.is_not_nil(conn)
        local _ = assert(server:accept())

        conn:close()
        server:close()
    end)

    it("read_until at EOF returns remaining data", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(2)

        -- Server sends data WITHOUT the delimiter, then closes
        client:send("no delimiter here")
        client:close()
        socket.sleep(0.1)

        -- read_until should return whatever is available
        local result = assert(conn:read_until("\0"))
        assert.equals("no delimiter here", result)

        conn:close()
        server:close()
    end)

    it("flush is a no-op that succeeds", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local _ = assert(server:accept())

        local ok, err = conn:flush()
        assert.is_true(ok)
        assert.is_nil(err)

        conn:close()
        server:close()
    end)

    it("request-response pattern like HTTP", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local client = assert(server:accept())
        client:settimeout(5)

        -- Client sends HTTP-like request
        conn:write_all("GET / HTTP/1.0\r\n\r\n")
        conn:flush()

        -- Server reads request and sends response
        client:receive("*l") -- read request line
        client:receive("*l") -- read blank line
        client:send("HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello")
        client:close()

        -- Client reads response line by line
        local status = assert(conn:read_line())
        assert.truthy(string.find(status, "HTTP/1.0 200"))

        local header = assert(conn:read_line())
        assert.truthy(string.find(header, "Content%-Length"))

        local blank = assert(conn:read_line())
        assert.equals("\r\n", blank)

        local body = assert(conn:read_exact(5))
        assert.equals("hello", body)

        conn:close()
        server:close()
    end)

    it("VERSION constant exists", function()
        assert.is_string(tcp_client.VERSION)
        assert.equals("0.1.0", tcp_client.VERSION)
    end)

    it("close is idempotent", function()
        local server, port = create_listener()
        local opts = tcp_client.ConnectOptions.new({
            connect_timeout = 5, read_timeout = 5, write_timeout = 5,
        })
        local conn = assert(tcp_client.connect("127.0.0.1", port, opts))
        local _ = assert(server:accept())

        -- Closing twice should not error
        conn:close()
        -- Second close may error but should not crash
        pcall(function() conn:close() end)

        server:close()
    end)
end)

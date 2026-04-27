# tcp-client

TCP client with buffered I/O and configurable timeouts — NET01

A Lua TCP client library that wraps luasocket with ergonomic defaults for building network clients. It is **protocol-agnostic** — it knows nothing about HTTP, SMTP, or Redis. It just moves bytes reliably between two machines. Higher-level packages build application protocols on top.

## Where It Fits

```
url-parser (NET00) → tcp-client (NET01, THIS) → frame-extractor (NET02)
                          ↓
                     raw byte stream
```

## Dependencies

- [luasocket](https://luarocks.org/modules/luasocket/luasocket) — TCP socket primitives

## Usage

```lua
local tcp_client = require("coding_adventures.tcp_client")

-- Connect with default options (30s timeouts, 8 KiB buffer)
local conn, err = tcp_client.connect("info.cern.ch", 80)
if not conn then
    print("Failed: " .. err.message)
    return
end

-- Send an HTTP request
conn:write_all("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")
conn:flush()

-- Read the response line by line
local status_line = conn:read_line()
print(status_line)  -- "HTTP/1.0 200 OK\r\n"

-- Close when done
conn:close()
```

## API

### `tcp_client.ConnectOptions.new(opts)`

Create connection options with custom timeouts and buffer size.

```lua
local opts = tcp_client.ConnectOptions.new({
    connect_timeout = 10,   -- seconds (default: 30)
    read_timeout    = 60,   -- seconds (default: 30)
    write_timeout   = 5,    -- seconds (default: 30)
    buffer_size     = 4096, -- bytes   (default: 8192)
})
```

### `tcp_client.connect(host, port, options)`

Establish a TCP connection. Returns `TcpConnection, nil` on success or `nil, TcpError` on failure.

```lua
local conn, err = tcp_client.connect("example.com", 80, opts)
```

### `TcpConnection:read_line()`

Read until `\n`. Returns the line including the trailing newline. Returns `""` at EOF.

### `TcpConnection:read_exact(n)`

Read exactly `n` bytes. Returns `nil, TcpError(unexpected_eof)` if the connection closes before `n` bytes arrive.

### `TcpConnection:read_until(delimiter)`

Read until `delimiter` (string or byte value). Returns data including the delimiter.

```lua
local data = conn:read_until("\0")      -- string delimiter
local data = conn:read_until(0)         -- byte value (same effect)
local data = conn:read_until("\r\n")    -- multi-char delimiter
```

### `TcpConnection:write_all(data)`

Write all bytes. Handles partial sends internally.

### `TcpConnection:flush()`

No-op (luasocket sends immediately). Exists for API compatibility with the Rust version.

### `TcpConnection:shutdown_write()`

Half-close: signal "I'm done sending" while keeping the read half open. Used in HTTP/1.0 to signal end of request.

### `TcpConnection:peer_addr()` / `TcpConnection:local_addr()`

Returns the remote/local address as an `"ip:port"` string.

### `TcpConnection:close()`

Close the connection and release the socket.

## Error Types

All errors are `TcpError` tables with a `type` field and a `message` field:

| Type | Meaning |
|------|---------|
| `dns_resolution_failed` | Hostname could not be resolved |
| `connection_refused` | Server reachable but nothing listening |
| `timeout` | Connect, read, or write took too long |
| `connection_reset` | Remote side crashed (TCP RST) |
| `broken_pipe` | Wrote after remote closed |
| `unexpected_eof` | Connection closed before expected data arrived |
| `io_error` | Catch-all for other OS errors |

```lua
local conn, err = tcp_client.connect("bad.host.example", 80)
if not conn then
    if err.type == "dns_resolution_failed" then
        print("Check the hostname: " .. err.host)
    elseif err.type == "timeout" then
        print("Timed out during: " .. err.phase)
    end
end
```

## Development

```bash
# Install dependencies
luarocks install luasocket
luarocks install busted

# Run tests
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_

# Or use the BUILD file
bash BUILD
```

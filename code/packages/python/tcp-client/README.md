# tcp-client

TCP client with buffered I/O and configurable timeouts.

This package wraps Python's `socket` module with ergonomic defaults for
building network clients. It is **protocol-agnostic** -- it knows nothing
about HTTP, SMTP, or Redis. It just moves bytes reliably between two
machines. Higher-level packages build application protocols on top.

## Where it fits

```
url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
                          |
                     raw byte stream
```

## Installation

```bash
pip install -e .[dev]
```

## Usage

```python
from tcp_client import connect, ConnectOptions

# Connect with default options (30s timeouts, 8 KiB buffer)
conn = connect("info.cern.ch", 80)

# Send an HTTP request
conn.write_all(b"GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")
conn.flush()

# Read the response line by line
status_line = conn.read_line()
print(status_line)  # "HTTP/1.0 200 OK\r\n"

# Read headers until blank line
while True:
    line = conn.read_line()
    if line == "\r\n":
        break
    print(line, end="")

# Read the body
body = conn.read_exact(content_length)
conn.close()
```

## Custom timeouts

```python
opts = ConnectOptions(
    connect_timeout=5.0,    # 5 seconds to establish connection
    read_timeout=10.0,      # 10 seconds to wait for data
    write_timeout=10.0,     # 10 seconds to wait for send buffer
    buffer_size=16384,      # 16 KiB read buffer
)
conn = connect("example.com", 80, opts)
```

## Context manager

```python
with connect("example.com", 80) as conn:
    conn.write_all(b"PING\n")
    conn.flush()
    response = conn.read_line()
# Connection is automatically closed
```

## Error handling

```python
from tcp_client import (
    connect, TcpError, DnsResolutionFailed,
    ConnectionRefused, Timeout, ConnectionReset,
    BrokenPipe, UnexpectedEof,
)

try:
    conn = connect("example.com", 80)
    conn.write_all(b"request")
    data = conn.read_exact(100)
except DnsResolutionFailed as e:
    print(f"Bad hostname: {e.host}")
except ConnectionRefused as e:
    print(f"Nothing listening: {e.addr}")
except Timeout as e:
    print(f"{e.phase} timed out after {e.duration}s")
except UnexpectedEof as e:
    print(f"Expected {e.expected} bytes, got {e.received}")
except TcpError as e:
    print(f"Network error: {e}")
```

## API

### `connect(host, port, options=None) -> TcpConnection`

Establish a TCP connection. Uses `socket.create_connection` internally.

### `TcpConnection`

| Method | Description |
|---|---|
| `read_line()` | Read until `\n`, returns `""` at EOF |
| `read_exact(n)` | Read exactly `n` bytes |
| `read_until(delimiter)` | Read until delimiter byte found |
| `write_all(data)` | Send all bytes (uses `sendall`) |
| `flush()` | No-op (sendall is immediate) |
| `shutdown_write()` | Half-close the write direction |
| `peer_addr()` | Remote `(host, port)` tuple |
| `local_addr()` | Local `(host, port)` tuple |
| `close()` | Close the connection |

### `ConnectOptions`

| Field | Default | Description |
|---|---|---|
| `connect_timeout` | `30.0` | Seconds to wait for TCP handshake |
| `read_timeout` | `30.0` | Seconds to wait for read data (`None` = forever) |
| `write_timeout` | `30.0` | Seconds to wait for write buffer (`None` = forever) |
| `buffer_size` | `8192` | Internal read buffer size in bytes |

## Development

```bash
# Run tests
bash BUILD

# Run linter
ruff check src/ tests/

# Run type checker
mypy --strict src/tcp_client/
```

# tcp-client

TCP client with buffered I/O and configurable timeouts.

This is an Elixir port of the Rust `tcp-client` crate. It wraps Erlang's
`:gen_tcp` with an ergonomic API for line-oriented and chunk-oriented network
communication. It is protocol-agnostic — it knows nothing about HTTP, SMTP,
or Redis. Higher-level packages build application protocols on top.

## Where it fits

```
url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
                         |
                    raw byte stream
```

## Usage

```elixir
alias CodingAdventures.TcpClient

# Connect with default options (30s timeouts, 8 KiB buffer)
{:ok, conn} = TcpClient.connect("info.cern.ch", 80)

# Write a request
:ok = TcpClient.write_all(conn, "GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")

# Read response line by line (returns updated conn for buffer threading)
{:ok, {status, conn}} = TcpClient.read_line(conn)
IO.puts(status)  # "HTTP/1.0 200 OK\r\n"

# Read exact number of bytes
{:ok, {body, conn}} = TcpClient.read_exact(conn, 1270)

# Read until a delimiter
{:ok, {data, conn}} = TcpClient.read_until(conn, 0)  # read until null byte

# Close when done
:ok = TcpClient.close(conn)
```

## Configuration

```elixir
{:ok, conn} = TcpClient.connect("example.com", 80,
  connect_timeout: 5_000,   # 5 seconds to establish connection
  read_timeout: 10_000,     # 10 seconds per read
  write_timeout: 10_000,    # 10 seconds per write
  buffer_size: 16_384       # 16 KiB read buffer
)
```

## API

| Function          | Description                                        |
|-------------------|----------------------------------------------------|
| `connect/3`       | Establish a TCP connection                         |
| `read_line/1`     | Read until `\n` (includes trailing newline)        |
| `read_exact/2`    | Read exactly N bytes                               |
| `read_until/2`    | Read until a delimiter byte                        |
| `write_all/2`     | Send all bytes to the connection                   |
| `flush/1`         | No-op (gen_tcp sends immediately)                  |
| `shutdown_write/1`| Half-close the write direction                     |
| `peer_addr/1`     | Remote IP and port                                 |
| `local_addr/1`    | Local IP and port                                  |
| `close/1`         | Close the connection                               |

## Error atoms

| Atom                      | Meaning                          |
|---------------------------|----------------------------------|
| `:dns_resolution_failed`  | Hostname not found               |
| `:connection_refused`     | Nobody listening on that port    |
| `:timeout`                | Operation took too long          |
| `:connection_reset`       | Remote closed unexpectedly       |
| `:broken_pipe`            | Write after remote close         |
| `:unexpected_eof`         | Connection closed mid-read       |

## Development

```bash
# Run tests
bash BUILD
```

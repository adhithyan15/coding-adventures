# tcp-client

A TCP client with buffered I/O and configurable timeouts.

This gem wraps Ruby's `TCPSocket` with ergonomic defaults for building network clients. It is protocol-agnostic -- it knows nothing about HTTP, SMTP, or Redis. It just moves bytes reliably between two machines. Higher-level packages build application protocols on top.

## Where it fits

```
url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
                         |
                    raw byte stream
```

## Installation

```ruby
gem "coding_adventures_tcp_client", path: "code/packages/ruby/tcp_client"
```

## Usage

```ruby
require "coding_adventures_tcp_client"

# Connect with defaults (30s timeouts, 8 KiB buffer)
conn = CodingAdventures::TcpClient.connect("info.cern.ch", 80)

# Send an HTTP request
conn.write_all("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")
conn.flush

# Read the response
status_line = conn.read_line
puts status_line  # => "HTTP/1.0 200 OK\r\n"

conn.close
```

### Custom options

```ruby
opts = CodingAdventures::TcpClient::ConnectOptions.new(
  connect_timeout: 10,
  read_timeout: 5,
  write_timeout: 5,
  buffer_size: 16_384
)
conn = CodingAdventures::TcpClient.connect("example.com", 80, opts)
```

### Error handling

```ruby
begin
  conn = CodingAdventures::TcpClient.connect("example.com", 80)
rescue CodingAdventures::TcpClient::DnsResolutionFailed
  puts "Check your hostname"
rescue CodingAdventures::TcpClient::ConnectionRefused
  puts "Server is not accepting connections"
rescue CodingAdventures::TcpClient::Timeout
  puts "Connection timed out"
rescue CodingAdventures::TcpClient::TcpError => e
  puts "TCP error: #{e.message}"
end
```

## API

### `TcpClient.connect(host, port, options = nil) -> TcpConnection`

Establish a TCP connection. Returns a `TcpConnection` for reading and writing.

### `TcpConnection#read_line -> String`

Read until `\n`. Returns the line including the newline. Returns `""` at EOF.

### `TcpConnection#read_exact(n) -> String`

Read exactly `n` bytes. Raises `UnexpectedEof` if the connection closes early.

### `TcpConnection#read_until(delimiter) -> String`

Read until the delimiter is found. Returns data including the delimiter.

### `TcpConnection#write_all(data)`

Write all bytes. Call `flush` after to ensure delivery.

### `TcpConnection#flush`

Flush any buffered writes to the network.

### `TcpConnection#shutdown_write`

Half-close: signal that no more data will be sent. Read half stays open.

### `TcpConnection#peer_addr -> [host, port]`

Returns the remote address.

### `TcpConnection#local_addr -> [host, port]`

Returns the local address.

### `TcpConnection#close`

Close the connection. Safe to call multiple times.

## Development

```bash
# Run tests
bash BUILD
```

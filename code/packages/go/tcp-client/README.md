# tcp-client

A TCP client with buffered I/O and configurable timeouts for Go.

This package wraps `net.TCPConn` with ergonomic defaults for building network
clients. It is **protocol-agnostic** -- it knows nothing about HTTP, SMTP, or
Redis. It just moves bytes reliably between two machines. Higher-level packages
build application protocols on top.

## Where it fits

```text
url-parser (NET00) --> tcp-client (NET01, THIS) --> frame-extractor (NET02)
                           |
                      raw byte stream
```

## Usage

```go
import tcpclient "github.com/adhithyan15/coding-adventures/code/packages/go/tcp-client"

// Connect with default options (30s timeouts, 8 KiB buffers).
conn, err := tcpclient.Connect("info.cern.ch", 80, tcpclient.DefaultOptions())
if err != nil {
    log.Fatal(err)
}
defer conn.Close()

// Send an HTTP/1.0 request.
conn.WriteAll([]byte("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n"))
conn.Flush()

// Read the response line by line.
statusLine, _ := conn.ReadLine()
fmt.Println(statusLine) // "HTTP/1.0 200 OK\r\n"
```

## API

| Function / Method         | Description                                      |
|---------------------------|--------------------------------------------------|
| `DefaultOptions()`        | Returns `ConnectOptions` with 30s timeouts, 8 KiB buffers |
| `Connect(host, port, opts)` | Establish a TCP connection                      |
| `ReadLine()`              | Read until `\n` (line-oriented protocols)         |
| `ReadExact(n)`            | Read exactly `n` bytes                           |
| `ReadUntil(delimiter)`    | Read until a specific byte is found              |
| `WriteAll(data)`          | Buffer data for sending                          |
| `Flush()`                 | Send all buffered data to the network            |
| `ShutdownWrite()`         | Half-close the write side (TCP FIN)              |
| `PeerAddr()`              | Remote address of the connection                 |
| `LocalAddr()`             | Local address of the connection                  |
| `Close()`                 | Close the connection                             |

## Error handling

All errors are returned as `*TcpError` with a `Kind` field:

- `DnsResolutionFailed` -- hostname could not be resolved
- `ConnectionRefused` -- nothing listening on the port
- `Timeout` -- connect, read, or write took too long
- `ConnectionReset` -- remote side sent TCP RST
- `BrokenPipe` -- write after remote closed
- `UnexpectedEof` -- connection closed before expected data arrived
- `IoError` -- catch-all for other OS-level errors

## Development

```bash
# Run tests
bash BUILD
```

# tcp-client

TCP client with buffered I/O and configurable timeouts — NET01

A protocol-agnostic TCP client that wraps POSIX sockets with ergonomic defaults
for building network clients. It knows nothing about HTTP, SMTP, or Redis — it
just moves bytes reliably between two machines. Higher-level packages build
application protocols on top.

## Where it fits

```text
url-parser (NET00) → tcp-client (NET01, THIS) → frame-extractor (NET02)
                        ↓
                   raw byte stream
```

## API

### Connect

```swift
let conn = try tcpConnect(host: "info.cern.ch", port: 80)
```

With custom options:

```swift
let opts = ConnectOptions(connectTimeout: 10, readTimeout: 5, writeTimeout: 5)
let conn = try tcpConnect(host: "example.com", port: 80, options: opts)
```

### Read

```swift
// Line-oriented (HTTP, SMTP, Redis)
let line = try conn.readLine()

// Exact byte count (Content-Length)
let body = try conn.readExact(1024)

// Until delimiter (null-terminated strings)
let chunk = try conn.readUntil(0)
```

### Write

```swift
try conn.writeAll(Data("GET / HTTP/1.0\r\n\r\n".utf8))
try conn.flush()
```

### Connection management

```swift
try conn.shutdownWrite()          // half-close
let (ip, port) = try conn.peerAddr()   // remote address
let (lip, lport) = try conn.localAddr() // local address
conn.close()                      // full close
```

### Error handling

```swift
do {
    let conn = try tcpConnect(host: "example.com", port: 80)
} catch TcpError.dnsResolutionFailed(let host, let msg) {
    print("DNS failed for \(host): \(msg)")
} catch TcpError.connectionRefused(let addr) {
    print("Nothing listening at \(addr)")
} catch TcpError.timeout(let phase, let duration) {
    print("\(phase) timed out after \(duration)s")
}
```

## Implementation

Uses POSIX sockets directly (`socket`, `connect`, `recv`, `send`, `select`,
`getaddrinfo`, `setsockopt`). No Foundation networking — just raw syscalls
with an internal read buffer for efficient line-oriented I/O.

Key techniques:
- Non-blocking `connect()` + `select()` for connect timeout control
- `SO_RCVTIMEO` / `SO_SNDTIMEO` for read/write timeouts
- Internal `Data` buffer to reduce syscall overhead
- `errno` mapping to structured `TcpError` enum

## Development

```bash
bash BUILD
```

Note: Swift cannot be tested on Windows. Tests run on macOS/Linux CI.

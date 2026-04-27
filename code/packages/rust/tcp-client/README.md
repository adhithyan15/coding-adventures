# tcp-client

TCP client with buffered I/O and configurable timeouts — NET01.

Part of the **Venture** browser networking pipeline. Wraps `std::net::TcpStream` with ergonomic defaults: configurable connect/read/write timeouts, buffered I/O, and structured error types.

## Connection lifecycle

```text
1. DIAL:   connect("host", 80, opts)  → DNS resolve + TCP handshake
2. TALK:   write_all(b"GET /") + flush() → read_line()
3. HANGUP: shutdown_write() → drop
```

## Usage

```rust
use tcp_client::{connect, ConnectOptions};

let mut conn = connect("info.cern.ch", 80, ConnectOptions::default())?;
conn.write_all(b"GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")?;
conn.flush()?;
let status = conn.read_line()?;
println!("{}", status); // "HTTP/1.0 200 OK\r\n"
```

## API

- `connect(host, port, options)` — establish TCP connection with timeout
- `read_line()` — read until `\n`
- `read_exact(n)` — read exactly n bytes
- `read_until(delimiter)` — read until delimiter byte
- `write_all(data)` — buffered write
- `flush()` — send buffered data
- `shutdown_write()` — half-close (signal "done writing")
- `peer_addr()` / `local_addr()` — connection endpoints

## Spec

See `code/specs/NET01-tcp-client.md` for the full specification.

## Development

```bash
cargo test -p tcp-client -- --nocapture
```
